//! Original guest probes: no OEM firmware, programmes or reference implementation.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_m6809_core::{Bus, C, Cpu, InterruptLines, Registers};
const ROM: &[u8; 16_384] = include_bytes!("../../../firmware/basic/basic.rom");
const SYMBOLS: &str = include_str!("../../../firmware/basic/basic.sym");
struct Memory {
    probe_address: u16,
    bytes: Vec<u8>,
    cycles: u64,
    output: Vec<u8>,
    signals: Vec<(u64, u16, u8)>,
    signal_limit: usize,
    input_signal: Vec<(u64, bool)>,
    input_cursor: usize,
    rising_edges: usize,
    irq_at_rising: Option<usize>,
    irq_ack_edges: Option<usize>,
    key_matrix: Option<[u8; 8]>,
    next_frame: Option<u64>,
    frame_pending: bool,
}
impl Memory {
    fn new() -> Self {
        let mut bytes = vec![0; 65536];
        // Unpressed keyboard rows and an idle-high printer strobe. Matrix
        // selection is independent of the printer's falling-edge data latch.
        bytes[0xff00] = 0x7f;
        bytes[0xff20] = 2;
        bytes[0x8000..0xc000].copy_from_slice(ROM);
        bytes[0xffe0..].copy_from_slice(&ROM[16352..]);
        Self {
            probe_address: 0x2000,
            bytes,
            cycles: 0,
            output: Vec::new(),
            signals: Vec::new(),
            signal_limit: 4096,
            input_signal: Vec::new(),
            input_cursor: 0,
            rising_edges: 0,
            irq_at_rising: None,
            irq_ack_edges: None,
            key_matrix: None,
            next_frame: None,
            frame_pending: false,
        }
    }
    fn execute(&mut self, cpu: &mut Cpu, code: &[u8]) {
        self.execute_with_budget(cpu, code, 16_384);
    }
    fn execute_with_budget(&mut self, cpu: &mut Cpu, code: &[u8], budget: usize) {
        let start = usize::from(self.probe_address);
        self.bytes[start..start + code.len()].copy_from_slice(code);
        cpu.registers.pc = self.probe_address;
        cpu.registers.s = 0x7000;
        for _ in 0..budget {
            if usize::from(cpu.registers.pc) == start + code.len() {
                return;
            }
            cpu.step(self).unwrap();
        }
        panic!(
            "guest ABI probe exceeded budget: PC {:04X}, X {:04X}, U {:04X}, S {:04X}",
            cpu.registers.pc, cpu.registers.x, cpu.registers.u, cpu.registers.s
        );
    }
}
impl Bus for Memory {
    fn read(&mut self, a: u16) -> u8 {
        if a == 0xff02 {
            self.frame_pending = false;
        }
        if a == 0xff00
            && let Some(columns) = self.key_matrix
        {
            let mut rows = 0x7f;
            for (column, keys) in columns.iter().enumerate() {
                if self.bytes[0xff02] & (1 << column) == 0 {
                    rows &= !keys;
                }
            }
            return rows;
        }
        if a == 0xff20 && !self.input_signal.is_empty() {
            while self.input_cursor + 1 < self.input_signal.len()
                && self.input_signal[self.input_cursor + 1].0 <= self.cycles
            {
                self.input_cursor += 1;
            }
            return (self.bytes[usize::from(a)] & 0xfe)
                | u8::from(self.input_signal[self.input_cursor].1);
        }
        self.bytes[usize::from(a)]
    }
    fn write(&mut self, a: u16, v: u8) {
        if a == 0xff20 && self.bytes[0xff20] & 2 != 0 && v & 2 == 0 {
            self.output.push(self.bytes[0xff02]);
        }
        if a == 0xff20 && v >> 2 >= 32 && self.bytes[0xff20] >> 2 < 32 {
            self.rising_edges += 1;
        }
        if a == 0x6300 && v == 1 && self.irq_at_rising.is_some() {
            self.irq_ack_edges = Some(self.rising_edges);
        }
        if (matches!(a, 0xff20..=0xff22) || (0xffc0..=0xffd4).contains(&a))
            && self.signals.len() < self.signal_limit
        {
            self.signals.push((self.cycles, a, v));
        }
        if a < 0x8000 || (0xff00..=0xff3f).contains(&a) {
            self.bytes[usize::from(a)] = v;
        }
    }
    fn tick(&mut self, n: u32) {
        self.cycles += u64::from(n);
        if let Some(next) = self.next_frame
            && self.cycles >= next
        {
            self.frame_pending = true;
            self.next_frame = Some(next + 17745);
        }
    }
    fn interrupts(&self) -> InterruptLines {
        InterruptLines {
            irq: self.frame_pending
                || self
                    .irq_at_rising
                    .is_some_and(|edge| self.rising_edges >= edge)
                    && self.bytes[0x6300] == 0,
            ..InterruptLines::default()
        }
    }
}
fn symbol(name: &str) -> u16 {
    let line = SYMBOLS
        .lines()
        .find(|line| line.starts_with(&format!("{name} EQU $")))
        .unwrap();
    u16::from_str_radix(line.split('$').nth(1).unwrap(), 16).unwrap()
}
fn call(code: &mut Vec<u8>, address: u16) {
    code.push(0xbd);
    code.extend(address.to_be_bytes());
}
fn packed_integer(value: i16) -> [u8; 5] {
    if value == 0 {
        return [0; 5];
    }
    let magnitude = u32::from(value.unsigned_abs());
    let exponent = 31 - magnitude.leading_zeros();
    let mantissa = (magnitude << (31 - exponent)).to_be_bytes();
    [
        u8::try_from(exponent + 129).unwrap(),
        (mantissa[0] & 127) | if value < 0 { 128 } else { 0 },
        mantissa[1],
        mantissa[2],
        mantissa[3],
    ]
}
#[test]
fn basic_rom_all_signed_integers_round_trip_through_legacy_calls() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    for raw in 0..=u16::MAX {
        let value = raw.cast_signed();
        cpu.registers = Registers::default();
        let mut code = vec![0xcc];
        code.extend(raw.to_be_bytes());
        call(&mut code, 0x8c37);
        code.extend([0x8e, 0x60, 0]);
        call(&mut code, symbol("FAC_STORE"));
        call(&mut code, 0x8b2d);
        memory.execute(&mut cpu, &code);
        assert_eq!(cpu.registers.d(), raw, "integer {value}");
        assert_eq!(cpu.registers.cc & C, 0);
        assert_eq!(
            &memory.bytes[0x6000..0x6005],
            &packed_integer(value),
            "packed integer {value}"
        );
    }
}
#[test]
fn basic_rom_numeric_zero_range_and_packed_load() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    for (packed, expected, overflow) in [
        ([0, 255, 255, 255, 255], 0, false),
        ([129, 0, 0, 0, 0], 1, false),
        ([144, 128, 0, 0, 0], 0x8000, false),
        ([144, 0, 0, 0, 0], 0, true),
        ([145, 128, 0, 0, 0], 0, true),
    ] {
        memory.bytes[0x6000..0x6005].copy_from_slice(&packed);
        let mut code = vec![0x8e, 0x60, 0];
        call(&mut code, symbol("FAC_LOAD"));
        call(&mut code, 0x8b2d);
        memory.execute(&mut cpu, &code);
        assert_eq!(cpu.registers.cc & C != 0, overflow);
        if !overflow {
            assert_eq!(cpu.registers.d(), expected);
        }
    }
}
#[test]
fn basic_rom_fixed_published_vectors_are_original_entry_stubs() {
    for address in [
        0x8006, 0x8009, 0x800c, 0x800f, 0x8012, 0x8015, 0x8018, 0x801b, 0x801e, 0x8021, 0x8024,
        0x8027, 0x8b2d, 0x8c37, 0xb3b4, 0xb54a, 0xb93e, 0xb999, 0xbb40, 0xbbb5, 0xbbbb, 0xbbe5,
        0xbd52, 0xbda5, 0xbdad, 0xbe12, 0xbe6a,
    ] {
        assert_eq!(ROM[address - 0x8000], 0x7e);
    }
    assert_eq!(
        &ROM[0x3ff2..],
        &[1, 0, 1, 3, 1, 15, 1, 12, 1, 6, 1, 9, 0xb3, 0xb4]
    );
    // Stubs alone are not ABI qualification: the corresponding guest behaviour must pass separately.
}

#[test]
fn basic_rom_ram_scanner_keeps_legacy_pointer_and_character_flags() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    let template = usize::from(symbol("SCANNER_TEMPLATE") - 0x8000);
    memory.bytes[0x9f..0xab].copy_from_slice(&ROM[template..template + 12]);
    memory.bytes[0xa6..0xa8].copy_from_slice(&0x01ff_u16.to_be_bytes());
    memory.bytes[0x200..0x206].copy_from_slice(b"  42:\0");
    cpu.registers.b = 0x7b;
    for (entry, expected, pointer, flags) in [
        (0x9f, b'4', 0x202, 0),
        (0xa5, b'4', 0x202, 0),
        (0x9f, b'2', 0x203, 0),
        (0x9f, b':', 0x204, C | eightbit_m6809_core::Z),
        (0x9f, 0, 0x205, C | eightbit_m6809_core::Z),
    ] {
        let mut code = Vec::new();
        call(&mut code, entry);
        memory.execute(&mut cpu, &code);
        assert_eq!(cpu.registers.a, expected);
        assert_eq!(cpu.registers.b, 0x7b);
        assert_eq!(
            u16::from_be_bytes([memory.bytes[0xa6], memory.bytes[0xa7]]),
            pointer
        );
        assert_eq!(cpu.registers.cc & (C | eightbit_m6809_core::Z), flags);
    }
}

// Fixed finite reference vectors fit these explicitly rounded conversions.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn packed_number(value: f64) -> [u8; 5] {
    if value == 0.0 {
        return [0; 5];
    }
    assert!(value.is_finite());
    let mut power = value.abs().log2().floor() as i32;
    if power < -128 {
        return [0; 5];
    }
    let mut mantissa = (value.abs() / 2_f64.powi(power) * 2_f64.powi(31)).round() as u64;
    if mantissa == 1_u64 << 32 {
        mantissa >>= 1;
        power += 1;
    }
    let bytes = u32::try_from(mantissa).unwrap().to_be_bytes();
    [
        u8::try_from(power + 129).unwrap(),
        (bytes[0] & 127) | if value.is_sign_negative() { 128 } else { 0 },
        bytes[1],
        bytes[2],
        bytes[3],
    ]
}
fn unpack_to(memory: &mut Memory, address: usize, value: [u8; 5]) {
    memory.bytes[address..address + 6].copy_from_slice(&[
        value[0],
        if value[0] == 0 { 0 } else { value[1] | 128 },
        value[2],
        value[3],
        value[4],
        value[1],
    ]);
}
#[test]
#[allow(clippy::cast_possible_truncation)]
fn basic_rom_original_floating_arithmetic_and_rounding() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    let mut pairs = vec![
        (0.0, 1.0),
        (1.0, 3.0),
        (-1.0, 3.0),
        (32767.0, -32768.0),
        (0.25, 0.75),
        (2_f64.powi(-40), 2_f64.powi(-39)),
        (2_f64.powi(40), 2_f64.powi(39)),
        (1.0, 2_f64.powi(-32)),
        (1.0 + 2_f64.powi(-31), 2_f64.powi(-32)),
    ];
    for n in 1..=400_u32 {
        pairs.push((
            f64::from(n * 137 % 65536) / 16.0 - 2048.0,
            f64::from(n * 353 % 65536) / 16.0 - 2048.0,
        ));
    }
    for (left, right) in pairs {
        for (routine, result) in [
            ("F_ADD", left + right),
            ("F_SUB", left - right),
            ("F_MUL", left * right),
            ("F_DIV", left / right),
        ] {
            cpu.registers = Registers::default();
            unpack_to(&mut memory, 0x4f, packed_number(left));
            unpack_to(&mut memory, 0x5c, packed_number(right));
            let mut code = Vec::new();
            call(&mut code, symbol(routine));
            code.extend([0x8e, 0x60, 0]);
            call(&mut code, symbol("FAC_STORE"));
            memory.execute(&mut cpu, &code);
            assert_eq!(cpu.registers.cc & C, 0, "{routine} {left} {right}");
            assert_eq!(
                &memory.bytes[0x6000..0x6005],
                &packed_number(result),
                "{routine} {left} {right}"
            );
        }
    }
}

#[test]
fn basic_rom_native_add_subtract_share_a_complete_carry_chain() {
    // Align exact integers in a wider, independent oracle, then round once to
    // the public 32-bit significand. No host floating-point intermediate is used.
    fn result(left: [u8; 5], right: [u8; 5], subtract: bool) -> Option<[u8; 5]> {
        let base = left[0].min(right[0]);
        let integer = |value: [u8; 5]| {
            if value[0] == 0 {
                return 0_i128;
            }
            let magnitude = i128::from(u32::from_be_bytes([
                value[1] | 128,
                value[2],
                value[3],
                value[4],
            ])) << (value[0] - base);
            if value[1] & 128 == 0 {
                magnitude
            } else {
                -magnitude
            }
        };
        let sum = integer(left)
            + if subtract {
                -integer(right)
            } else {
                integer(right)
            };
        if sum == 0 {
            return Some([0; 5]);
        }
        let magnitude = sum.unsigned_abs();
        let bits = 128 - magnitude.leading_zeros();
        let mut exponent = i32::from(base) + i32::try_from(bits).unwrap() - 32;
        let mut mantissa = if bits > 32 {
            let shift = bits - 32;
            (magnitude >> shift)
                + u128::from(magnitude & ((1_u128 << shift) - 1) >= 1_u128 << (shift - 1))
        } else {
            magnitude << (32 - bits)
        };
        if mantissa == 1_u128 << 32 {
            mantissa >>= 1;
            exponent += 1;
        }
        if exponent <= 0 {
            return Some([0; 5]);
        }
        let exponent = u8::try_from(exponent).ok()?;
        let bytes = u32::try_from(mantissa).unwrap().to_be_bytes();
        Some([
            exponent,
            (bytes[0] & 127) | if sum < 0 { 128 } else { 0 },
            bytes[1],
            bytes[2],
            bytes[3],
        ])
    }
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    let mut state = 0xb56c_092f_u32;
    for index in 0..4096_u32 {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let mut left = state.to_be_bytes();
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let mut right = state.to_be_bytes();
        // Deliberate cancellation, complete carries, low-byte borrows and ties.
        match index % 8 {
            0 => right = left,
            1 => {
                left = [255; 4];
                right = [128, 0, 0, 1];
            }
            2 => {
                left = [128, 0, 0, 0];
                right = [255; 4];
            }
            3 => {
                left = [128, 0, 1, 0];
                right = [128, 0, 0, 255];
            }
            _ => {}
        }
        let distance = u8::try_from((index / 8) % 65).unwrap();
        let exponent = if index % 11 == 0 { 255 } else { 128 };
        let left = [exponent, left[0], left[1], left[2], left[3]];
        let right = [exponent - distance, right[0], right[1], right[2], right[3]];
        for (name, subtract) in [("F_ADD", false), ("F_SUB", true)] {
            cpu.registers = Registers::default();
            memory.bytes[0xd8..0xea].fill(0xa5);
            memory.bytes[0x3d9..0x400].fill(0x5a);
            unpack_to(&mut memory, 0x4f, left);
            unpack_to(&mut memory, 0x5c, right);
            let mut code = Vec::new();
            call(&mut code, symbol(name));
            memory.execute(&mut cpu, &code);
            match result(left, right, subtract) {
                Some(expected) => {
                    assert_eq!(cpu.registers.cc & C, 0, "{name} {left:?} {right:?}");
                    let mut packed = Vec::from([0x8e, 0x60, 0]);
                    call(&mut packed, symbol("FAC_STORE"));
                    memory.execute(&mut cpu, &packed);
                    assert_eq!(
                        &memory.bytes[0x6000..0x6005],
                        &expected,
                        "{name} {left:?} {right:?}"
                    );
                }
                None => assert_ne!(cpu.registers.cc & C, 0, "{name} overflow"),
            }
            assert_eq!(&memory.bytes[0xd8..0xea], &[0xa5; 18]);
            assert_eq!(&memory.bytes[0x3d9..0x400], &[0x5a; 39]);
            assert_eq!(cpu.registers.s, 0x7000);
        }
    }
}

#[test]
fn basic_rom_arithmetic_halfway_magnitudes_round_away_from_zero_like_lawful_profile() {
    // Independent packed results observed using original synthetic BASIC inputs.
    for (source, expected) in [
        ("A=4294967296:B=A+1", [161, 0, 0, 0, 1]),
        ("A=4294967296:B=-A-1", [161, 128, 0, 0, 1]),
        ("A=4294967298:B=A+1", [161, 0, 0, 0, 2]),
        ("A=4294967298:B=-A-1", [161, 128, 0, 0, 2]),
        ("A=4294967296:B=A+.5", [161, 0, 0, 0, 0]),
        ("A=4294967296:B=A+1.5", [161, 0, 0, 0, 1]),
        ("A=1:B=1E38", [255, 22, 118, 153, 83]),
        ("A=1:M=VARPTR(A):POKE M+4,3:B=A*1.5", [129, 64, 0, 0, 5]),
        ("A=1:M=VARPTR(A):POKE M+4,3:B=-A*1.5", [129, 192, 0, 0, 5]),
    ] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        assert!(execute_direct(&mut memory, &mut cpu, source), "{source}");
        // VARPTR fixtures create M before B; locate the named scalar explicitly.
        let table = usize::from(word(&memory, 0x1b));
        let end = usize::from(word(&memory, 0x1d));
        let record = (table..end)
            .step_by(7)
            .find(|p| &memory.bytes[*p..*p + 2] == b"B\0")
            .unwrap();
        assert_eq!(&memory.bytes[record + 2..record + 7], &expected, "{source}");
        assert_eq!(word(&memory, usize::from(symbol("STR_ROOTS"))), 0);
        assert_eq!(memory.bytes[usize::from(symbol("EX_DEPTH"))], 0);
    }
}

#[test]
fn basic_rom_native_mul_keeps_the_full_product_for_ties_and_sticky_rounding() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    let mut state = 0x529a_07d3_u32;
    for index in 0..4096 {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let left = state | 0x8000_0000;
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let right = state | 0x8000_0000;
        let (left, right) = match index {
            0 => (0x8000_0001, 0xc000_0000),
            1 => (0x8000_0003, 0xc000_0000),
            _ => (left, right),
        };
        let negative = state & 0x4000_0000 != 0;
        for (address, mantissa, sign) in [
            (0x4f, left, 0),
            (0x5c, right, if negative { 128 } else { 0 }),
        ] {
            memory.bytes[address] = 129;
            memory.bytes[address + 1..address + 5].copy_from_slice(&mantissa.to_be_bytes());
            memory.bytes[address + 5] = sign;
        }
        let product = u64::from(left) * u64::from(right);
        let shift = if product & (1_u64 << 63) == 0 { 31 } else { 32 };
        let mut exponent = if shift == 31 { 129 } else { 130 };
        let mut rounded = product >> shift;
        let remainder = product & ((1_u64 << shift) - 1);
        let half = 1_u64 << (shift - 1);
        if remainder >= half {
            rounded += 1;
        }
        if rounded == 1_u64 << 32 {
            rounded >>= 1;
            exponent += 1;
        }
        let bytes = u32::try_from(rounded).unwrap().to_be_bytes();
        let expected = [
            exponent,
            (bytes[0] & 127) | if negative { 128 } else { 0 },
            bytes[1],
            bytes[2],
            bytes[3],
        ];
        let mut code = Vec::new();
        call(&mut code, symbol("F_MUL"));
        code.extend([0x8e, 0x60, 0]);
        call(&mut code, symbol("FAC_STORE"));
        memory.execute(&mut cpu, &code);
        assert_eq!(memory.bytes[0x6000..0x6005], expected);
        assert_eq!(cpu.registers.cc & C, 0);
    }
}
#[test]
fn basic_rom_floating_range_failures_and_underflow() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    for (routine, left, right, overflow) in [
        ("F_DIV", 1.0, 0.0, true),
        ("F_MUL", 2_f64.powi(126), 2.0, true),
        ("F_MUL", 2_f64.powi(-128), 0.5, false),
    ] {
        unpack_to(&mut memory, 0x4f, packed_number(left));
        unpack_to(&mut memory, 0x5c, packed_number(right));
        let mut code = Vec::new();
        call(&mut code, symbol(routine));
        memory.execute(&mut cpu, &code);
        assert_eq!(cpu.registers.cc & C != 0, overflow);
        if !overflow {
            assert_eq!(memory.bytes[0x4f], 0);
        }
    }
}

#[test]
fn basic_rom_decimal_literals_advance_pointer_and_reject_invalid_exponents() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    for (text, value, consumed) in [
        ("0!", 0.0, 1),
        ("12345:", 12345.0, 5),
        (".5,", 0.5, 2),
        ("1.25+", 1.25, 4),
        ("10E2;", 1000.0, 4),
        ("125E-2)", 1.25, 6),
        ("0E255:", 0.0, 5),
        ("1E-255:", 0.0, 6),
    ] {
        memory.bytes[0x6000..0x6000 + text.len()].copy_from_slice(text.as_bytes());
        let mut code = vec![0x8e, 0x60, 0];
        call(&mut code, symbol("NUMBER_PARSE"));
        memory.execute_with_budget(&mut cpu, &code, 1_048_576);
        assert_eq!(cpu.registers.cc & C, 0, "{text}");
        assert_eq!(cpu.registers.x, 0x6000 + consumed, "{text}");
        code = vec![0x8e, 0x61, 0];
        call(&mut code, symbol("FAC_STORE"));
        memory.execute(&mut cpu, &code);
        assert_eq!(
            &memory.bytes[0x6100..0x6105],
            &packed_number(value),
            "{text}"
        );
    }
    for text in [".", "E1", "1E", "1E+", "1E-", "1E256", "1E39"] {
        memory.bytes[0x6000..0x6000 + text.len()].copy_from_slice(text.as_bytes());
        memory.bytes[0x6000 + text.len()] = 0;
        let mut code = vec![0x8e, 0x60, 0];
        call(&mut code, symbol("NUMBER_PARSE"));
        memory.execute_with_budget(&mut cpu, &code, 1_048_576);
        assert_ne!(cpu.registers.cc & C, 0, "invalid literal {text}");
        assert_eq!(cpu.registers.s, 0x7000, "stack restored after {text}");
    }
}

#[test]
fn basic_rom_decimal_exponents_share_word_scanning_without_losing_caller_workspace() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    for text in [
        "1E+2!", "1 E - 2!", "0E255!", "1E-255!", "1E256!", "1E63999!", "1E64000!", "1E+!", "1E-!",
    ] {
        memory.bytes[0x6000..0x6100].fill(0);
        memory.bytes[0x6000..0x6000 + text.len()].copy_from_slice(text.as_bytes());
        set_word(&mut memory, 0x2b, 0xa55a);
        memory.bytes[0xd8..=0xe9].fill(0xa5);
        memory.bytes[0x3d9..=0x3ff].fill(0x5a);
        cpu.registers.y = 0x1234;
        cpu.registers.u = 0x5678;
        let mut code = vec![0x8e, 0x60, 0];
        call(&mut code, symbol("NUMBER_PARSE"));
        memory.execute_with_budget(&mut cpu, &code, 1_048_576);
        assert_eq!(word(&memory, 0x2b), 0xa55a, "{text}");
        assert_eq!(cpu.registers.y, 0x1234, "{text}");
        assert_eq!(cpu.registers.u, 0x5678, "{text}");
        assert_eq!(cpu.registers.s, 0x7000, "{text}");
        assert!(memory.bytes[0xd8..=0xe9].iter().all(|&byte| byte == 0xa5));
        assert!(memory.bytes[0x3d9..=0x3ff].iter().all(|&byte| byte == 0x5a));
        assert_eq!(
            cpu.registers.cc & C == 0,
            !text.contains("256")
                && !text.contains("63999")
                && !text.contains("64000")
                && !text.contains("+!")
                && !text.contains("-!"),
            "{text}"
        );
    }
}

#[test]
fn basic_rom_typed_errors_report_selected_legacy_codes_and_recover() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    for (source, expected) in [
        ("A=1+\"X\"", "TM"),
        ("A$=1", "TM"),
        ("A=\"X\"-\"Y\"", "TM"),
        ("A=ABS(\"X\")", "TM"),
        ("A=1/0", "/0"),
        ("A=0/0", "/0"),
        ("A=1E38*10", "OV"),
        ("A=PEEK(-1)", "FC"),
        ("A=PEEK(65536)", "FC"),
        ("A=SQR(-1)", "FC"),
        ("A$=HEX$(-1)", "FC"),
        ("POKE25344,256", "FC"),
        ("A=1+", "SN"),
    ] {
        assert!(execute_direct(&mut memory, &mut cpu, "Z=7"));
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(numeric_scalar(&memory, [b'Z', 0]), packed_integer(7));
        assert_eq!(word(&memory, usize::from(symbol("STR_ROOTS"))), 0);
        assert_eq!(memory.bytes[usize::from(symbol("EX_DEPTH"))], 0);
        memory.output.clear();
        let mut code = Vec::new();
        call(&mut code, symbol("BASIC_ERROR"));
        memory.execute(&mut cpu, &code);
        assert_eq!(
            memory.output,
            format!("?{expected} ERROR\r").as_bytes(),
            "{source}"
        );
        assert!(execute_direct(&mut memory, &mut cpu, "Z=85"));
        assert_eq!(numeric_scalar(&memory, [b'Z', 0]), packed_integer(85));
    }
    store_line(&mut memory, &mut cpu, 1000, "A=1/0");
    assert!(!execute_direct(&mut memory, &mut cpu, "RUN"));
    memory.output.clear();
    let mut code = Vec::new();
    call(&mut code, symbol("BASIC_ERROR"));
    memory.execute(&mut cpu, &code);
    assert_eq!(memory.output, b"?/0 ERROR IN 1000\r");
    memory.bytes[usize::from(symbol("ERROR_CODE"))] = 255;
    set_word(&mut memory, 0x68, 65535);
    memory.output.clear();
    memory.execute(&mut cpu, &code);
    assert_eq!(memory.output, b"?SN ERROR\r");
}

#[test]
fn basic_rom_address_conversion_returns_carry_without_using_a_parser_frame() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    for (value, valid) in [
        (-1.0, false),
        (0.0, true),
        (32768.0, true),
        (65535.0, true),
        (65536.0, false),
    ] {
        unpack_to(&mut memory, 0x4f, packed_number(value));
        memory.bytes[0x3d9..=0x3ff].fill(0xa5);
        let mut code = Vec::new();
        call(&mut code, symbol("FAC_TO_ADDRESS"));
        memory.execute(&mut cpu, &code);
        assert_eq!(cpu.registers.cc & C == 0, valid, "{value}");
        assert_eq!(cpu.registers.s, 0x7000);
        assert!(memory.bytes[0x3d9..=0x3ff].iter().all(|&byte| byte == 0xa5));
    }
}

#[test]
fn basic_rom_prompt_reports_programme_line_then_clears_it_before_direct_errors() {
    let memory = cold_boot_to_probe("1000 A=1/0\rRUN\rA=1+\rPOKE25345,85:EXEC25000\r");
    assert_eq!(memory.bytes[0x6301], 85);
    let screen: String = memory.bytes[0x400..0x600]
        .iter()
        .map(|&byte| {
            let value = byte & 63;
            char::from(if value < 32 { value + 64 } else { value })
        })
        .collect();
    assert!(screen.contains("?/0 ERROR IN 1000"));
    assert!(screen.contains("?SN ERROR"));
    assert!(!screen.contains("?SN ERROR IN"));
}

#[test]
fn basic_rom_numeric_comparison_handles_sign_zero_and_magnitude() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    for left in [-32768.0, -2.5, -1.0, 0.0, 0.5, 1.0, 32767.0] {
        for right in [-32768.0, -2.5, -1.0, 0.0, 0.5, 1.0, 32767.0] {
            unpack_to(&mut memory, 0x4f, packed_number(left));
            unpack_to(&mut memory, 0x5c, packed_number(right));
            let mut code = Vec::new();
            call(&mut code, symbol("F_COMPARE"));
            memory.execute(&mut cpu, &code);
            assert_eq!(
                cpu.registers.a,
                if left < right {
                    255
                } else {
                    u8::from(left > right)
                }
            );
            assert_eq!(
                cpu.registers.cc & eightbit_m6809_core::Z != 0,
                left.partial_cmp(&right) == Some(std::cmp::Ordering::Equal)
            );
        }
    }
}

#[test]
fn basic_rom_guest_expression_precedence_functions_and_failure_recovery() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    for (source, expected) in [
        ("1+2*3", 7.0),
        ("(1+2)*3", 9.0),
        ("-2^2", -4.0),
        ("2^3^2", 512.0),
        ("2^-3", 0.125),
        ("10/4-1", 1.5),
        ("1<2", -1.0),
        ("2>1", -1.0),
        ("1=2", 0.0),
        ("1<=1", -1.0),
        ("1<>2", -1.0),
        ("1>=2", 0.0),
        ("10 AND 3 OR 8", 10.0),
        ("NOT 1=2", -1.0),
        ("NOT -1", 0.0),
        ("ABS(-1.25)", 1.25),
        ("SGN(-3)", -1.0),
        ("SGN(0)", 0.0),
        ("FIX(-1.75)", -1.0),
        ("INT(-1.75)", -2.0),
        ("INT(1.75)", 1.0),
        ("INT(-2)", -2.0),
        ("A+1", 1.0),
        ("A1+AB+1", 1.0),
    ] {
        guest_tokens(&mut memory, &mut cpu, source);
        let mut code = vec![0x8e, 2, 0xdd];
        call(&mut code, symbol("EVAL"));
        memory.execute_with_budget(&mut cpu, &code, 1_048_576);
        assert_eq!(cpu.registers.cc & C, 0, "expression {source}");
        assert_eq!(
            memory.bytes[usize::from(cpu.registers.x)],
            0,
            "expression cursor {source}"
        );
        let mut code = vec![0x8e, 0x61, 0];
        call(&mut code, symbol("FAC_STORE"));
        memory.execute(&mut cpu, &code);
        assert_eq!(
            &memory.bytes[0x6100..0x6105],
            &packed_number(expected),
            "expression {source}"
        );
    }
    for source in [
        "1/0",
        "(1+2",
        "1+",
        "(-2)^.5",
        "NOT 40000",
        "1+ABS(",
        &"(".repeat(40),
    ] {
        guest_tokens(&mut memory, &mut cpu, source);
        let mut code = vec![0x8e, 2, 0xdd];
        call(&mut code, symbol("EVAL"));
        memory.execute_with_budget(&mut cpu, &code, 1_048_576);
        assert_ne!(cpu.registers.cc & C, 0, "invalid expression {source}");
        assert_eq!(cpu.registers.s, 0x7000, "recovered stack after {source}");
        assert_eq!(
            memory.bytes[usize::from(symbol("EX_DEPTH"))],
            0,
            "recovered depth after {source}"
        );
    }
    assert_eq!(&memory.bytes[0x1e03..0x1e05], b"A\0");
    assert_eq!(&memory.bytes[0x1e0a..0x1e0c], b"A1");
    assert_eq!(&memory.bytes[0x1e11..0x1e13], b"AB");
    assert_eq!(word(&memory, 0x1d), 0x1e18);
}

fn enable_guest_printer(memory: &mut Memory) {
    memory.bytes[0x6f] = 0xfe;
    memory.bytes[0xff22] = 0;
    memory.bytes[0xff20] = 2;
    memory.bytes[0x16a] = 0x39;
    memory.bytes[0x1a6] = 0x39;
    set_word(memory, 0x27, 0x7ffe);
    set_word(memory, 0x23, 0x7ffe);
}

#[test]
fn basic_rom_packed_operand_frames_leave_space_for_deep_numeric_expressions() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    // A legitimate narrow gap above resident data, with no variable allocation.
    // The parser must retain its collision margin rather than borrowing pages.
    set_word(&mut memory, 0x1f, 0x6d00);
    memory.bytes[0xd8..0xea].fill(0xa5);
    let source = format!("{}1", "1^".repeat(30));
    guest_tokens(&mut memory, &mut cpu, &source);
    let mut code = vec![0x8e, 2, 0xdd];
    call(&mut code, symbol("EVAL"));
    memory.execute_with_budget(&mut cpu, &code, 4_194_304);
    assert_eq!(cpu.registers.cc & C, 0);
    assert_eq!(cpu.registers.s, 0x7000);
    assert_eq!(word(&memory, usize::from(symbol("STR_ROOTS"))), 0);
    assert_eq!(&memory.bytes[0xd8..0xea], &[0xa5; 18]);
    assert_eq!(memory.bytes[0x4f], 129);
    assert_eq!(memory.bytes[0x50], 128);

    for source in [format!("{}(", "1^".repeat(30)), "1+\"X\"".into()] {
        guest_tokens(&mut memory, &mut cpu, &source);
        memory.execute_with_budget(&mut cpu, &code, 4_194_304);
        assert_ne!(cpu.registers.cc & C, 0);
        assert_eq!(cpu.registers.s, 0x7000);
        assert_eq!(word(&memory, usize::from(symbol("STR_ROOTS"))), 0);
        assert_eq!(memory.bytes[usize::from(symbol("EX_DEPTH"))], 0);
    }
}

#[test]
fn basic_rom_only_live_string_operands_are_registered_as_collector_roots() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    // Original USR observer records a root address, never ROM instructions.
    // It returns ASCII A through the published integer-to-FAC entry.
    let mut observer = vec![0xfc, 3, 0xea, 0xfd, 0x63, 0, 0xcc, 0, 65];
    call(&mut observer, 0x8c37);
    observer.push(0x39);
    memory.bytes[0x6400..0x6400 + observer.len()].copy_from_slice(&observer);
    set_word(&mut memory, 0xb0, 0x134);
    set_word(&mut memory, 0x134, 0x6400);
    assert!(execute_direct(&mut memory, &mut cpu, "Z=1+USR0(0)"));
    assert_eq!(word(&memory, 0x6300), 0);
    assert_eq!(numeric_scalar(&memory, [b'Z', 0]), packed_integer(66));
    assert!(execute_direct(&mut memory, &mut cpu, "A$=\"HELLO\""));
    assert!(execute_direct(&mut memory, &mut cpu, "B$=CHR$(USR0(0))"));
    assert!(execute_direct(&mut memory, &mut cpu, "B$=A$+CHR$(USR0(0))"));
    assert_ne!(word(&memory, 0x6300), 0);
    assert_eq!(word(&memory, usize::from(symbol("STR_ROOTS"))), 0);
    assert!(execute_direct(&mut memory, &mut cpu, "Z=B$=\"HELLOA\""));
    assert_eq!(numeric_scalar(&memory, [b'Z', 0]), packed_integer(-1));
}
fn execute_direct(memory: &mut Memory, cpu: &mut Cpu, source: &str) -> bool {
    execute_direct_with_budget(memory, cpu, source, 4_194_304)
}
fn execute_direct_with_budget(
    memory: &mut Memory,
    cpu: &mut Cpu,
    source: &str,
    budget: usize,
) -> bool {
    guest_tokens(memory, cpu, source);
    let mut code = vec![0x8e, 2, 0xdd];
    call(&mut code, symbol("EXECUTE"));
    memory.execute_with_budget(cpu, &code, budget);
    assert_eq!(cpu.registers.s, 0x7000, "statement stack after {source}");
    cpu.registers.cc & C == 0
}
fn numeric_scalar(memory: &Memory, name: [u8; 2]) -> [u8; 5] {
    memory.bytes[usize::from(word(memory, 0x1b))..usize::from(word(memory, 0x1d))]
        .chunks_exact(7)
        .find(|record| record[..2] == name)
        .expect("synthetic scalar must exist")[2..]
        .try_into()
        .unwrap()
}

#[test]
fn basic_rom_timer_assignment_validates_before_resetting_the_public_counter() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    for (source, expected) in [("TIMER=0", 0), ("TIMER=65535", 65535), ("TIMER=12.75", 12)] {
        memory.bytes[0x112] = 0xa5;
        assert!(execute_direct(&mut memory, &mut cpu, source));
        assert_eq!(memory.bytes[0x112], 0);
        assert_eq!(word(&memory, 0x113), expected);
        assert!(execute_direct(&mut memory, &mut cpu, "A=TIMER"));
        assert_eq!(
            numeric_scalar(&memory, [b'A', 0]),
            packed_number(f64::from(expected))
        );
    }
    for source in [
        "TIMER=-1",
        "TIMER=65536",
        "TIMER=\"X\"",
        "TIMER=",
        "TIMER 2",
        "TIMER=12 XYZ",
    ] {
        memory.bytes[0x112..0x115].copy_from_slice(&[85, 0x12, 0x34]);
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(&memory.bytes[0x112..0x115], &[85, 0x12, 0x34]);
        assert!(execute_direct(&mut memory, &mut cpu, "TIMER=9"));
        assert_eq!(word(&memory, 0x113), 9);
    }
}

#[test]
fn basic_rom_line_zero_runs_lists_branches_and_deletes_without_changing_ide_schema() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    store_line(&mut memory, &mut cpu, 0, "A=A+1:IF A<2 THEN 0");
    store_line(&mut memory, &mut cpu, 10, "END");
    assert!(execute_direct(&mut memory, &mut cpu, "RUN 0"));
    assert_eq!(numeric_scalar(&memory, [b'A', 0]), packed_integer(2));
    assert!(execute_direct(&mut memory, &mut cpu, "LIST 0"));
    assert_eq!(memory.output, b"0 A=A+1:IF A<2 THEN 0\r");
    memory.output.clear();
    assert!(execute_direct(&mut memory, &mut cpu, "DEL 0"));
    assert!(execute_direct(&mut memory, &mut cpu, "LIST"));
    assert_eq!(memory.output, b"10 END\r");
    for source in ["", "X", "64000", "999999"] {
        memory.bytes[0x6300..0x6300 + source.len()].copy_from_slice(source.as_bytes());
        memory.bytes[0x6300 + source.len()] = 0;
        let mut code = vec![0x8e, 0x63, 0];
        call(&mut code, symbol("PARSE_LINE_NUMBER"));
        memory.execute(&mut cpu, &code);
        assert_ne!(cpu.registers.cc & C, 0, "invalid line {source:?}");
    }
}

#[test]
fn basic_rom_native_name_parser_owns_validation_and_preserves_its_callers_frame() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    for (name, valid) in [
        (" a1tail", true),
        ("B$", true),
        ("", false),
        ("1A", false),
        ("[A", false),
        ("+A", false),
    ] {
        memory.bytes[0x6300..0x6300 + name.len()].copy_from_slice(name.as_bytes());
        memory.bytes[0x6300 + name.len()] = 0;
        let before = memory.bytes[0x1e03..0x1f00].to_vec();
        let mut code = vec![0xce, 0x55, 0xaa, 0x8e, 0x63, 0];
        call(&mut code, symbol("VAR_FIND"));
        memory.execute(&mut cpu, &code);
        assert_eq!(cpu.registers.cc & C == 0, valid, "{name:?}");
        assert_eq!(cpu.registers.u, 0x55aa);
        if !valid {
            assert_eq!(&memory.bytes[0x1e03..0x1f00], before.as_slice());
        }
    }
}

#[test]
fn basic_rom_native_unsigned_print_handles_decimal_boundaries_and_preserves_registers() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    enable_guest_printer(&mut memory);
    for number in [
        0_u16, 1, 9, 10, 99, 100, 999, 1000, 9999, 10000, 32768, 65535,
    ] {
        memory.output.clear();
        let mut code = vec![0xcc];
        code.extend(number.to_be_bytes());
        code.extend([0x8e, 0x12, 0x34, 0x10, 0x8e, 0x23, 0x45, 0xce, 0x34, 0x56]);
        call(&mut code, symbol("PRINT_UNSIGNED"));
        let before = memory.cycles;
        memory.execute(&mut cpu, &code);
        assert!(
            memory.cycles - before < 32_000,
            "unsigned print {number}: {} E cycles",
            memory.cycles - before
        );
        assert_eq!(memory.output, number.to_string().as_bytes());
        assert_eq!(cpu.registers.d(), number);
        assert_eq!(cpu.registers.x, 0x1234);
        assert_eq!(cpu.registers.y, 0x2345);
        assert_eq!(cpu.registers.u, 0x3456);
    }
}

#[test]
fn basic_rom_native_signed_difference_uses_borrow_across_the_complete_mantissa() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    let mut seed = 0x5296_381d_u32;
    for _ in 0..2048 {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let left = f64::from(seed) / 65536.0;
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let right = f64::from(seed) / 65536.0;
        for (left, right) in [
            (left, right),
            (-left, right),
            (left, -right),
            (-left, -right),
            (left, left),
        ] {
            unpack_to(&mut memory, 0x4f, packed_number(left));
            unpack_to(&mut memory, 0x5c, packed_number(right));
            let mut code = vec![0xce, 0x55, 0xaa];
            call(&mut code, symbol("F_SUB"));
            code.extend([0x8e, 0x63, 0]);
            call(&mut code, symbol("FAC_STORE"));
            memory.execute(&mut cpu, &code);
            assert_eq!(cpu.registers.cc & C, 0);
            assert_eq!(cpu.registers.u, 0x55aa);
            assert_eq!(
                &memory.bytes[0x6300..0x6305],
                &packed_number(left - right),
                "{left} - {right}"
            );
        }
    }
}
#[test]
fn basic_rom_list_and_delete_inclusive_ranges_preserve_survivors() {
    for (range, expected) in [
        ("", "10 END\r20 END\r30 END\r"),
        ("-", "10 END\r20 END\r30 END\r"),
        ("20", "20 END\r"),
        ("10-20", "10 END\r20 END\r"),
        ("-20", "10 END\r20 END\r"),
        ("20-", "20 END\r30 END\r"),
        ("11-29", "20 END\r"),
        ("0-9", ""),
        ("31-63999", ""),
    ] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        for line in [10, 20, 30] {
            store_line(&mut memory, &mut cpu, line, "END");
        }
        assert!(execute_direct(
            &mut memory,
            &mut cpu,
            &format!("LIST {range}")
        ));
        assert_eq!(memory.output, expected.as_bytes(), "LIST {range}");
        memory.output.clear();
        if range.is_empty() {
            assert!(!execute_direct(&mut memory, &mut cpu, "DEL"));
            continue;
        }
        assert!(execute_direct(
            &mut memory,
            &mut cpu,
            &format!("DEL {range}")
        ));
        assert!(execute_direct(&mut memory, &mut cpu, "LIST"));
        let survivors: String = "10 END\r20 END\r30 END\r"
            .split_inclusive('\r')
            .filter(|line| !expected.contains(line))
            .collect();
        assert_eq!(memory.output, survivors.as_bytes(), "DEL {range}");
        assert_eq!(word(&memory, 0x1b), word(&memory, 0x1d));
        assert_eq!(word(&memory, 0x1d), word(&memory, 0x1f));
    }
}

#[test]
fn basic_rom_bad_ranges_fail_before_listing_or_deleting() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    store_line(&mut memory, &mut cpu, 10, "END");
    assert!(execute_direct(&mut memory, &mut cpu, "A=7"));
    let programme = memory.bytes[0x1e01..usize::from(word(&memory, 0x1f))].to_vec();
    for range in ["20-10", "64000", "A", "10-20,30", "1--2", "10 X"] {
        for command in ["LIST", "LLIST", "DEL"] {
            memory.output.clear();
            assert!(!execute_direct(
                &mut memory,
                &mut cpu,
                &format!("{command} {range}")
            ));
            assert!(memory.output.is_empty());
            assert_eq!(
                &memory.bytes[0x1e01..usize::from(word(&memory, 0x1f))],
                programme.as_slice()
            );
            assert_eq!(memory.bytes[0x6f], 0xfe);
        }
    }
}

#[test]
fn basic_rom_delete_during_execution_is_rejected_without_editing() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    store_line(&mut memory, &mut cpu, 10, "DEL10-20");
    store_line(&mut memory, &mut cpu, 20, "END");
    let programme = memory.bytes[0x1e01..usize::from(word(&memory, 0x1b))].to_vec();
    assert!(!execute_direct(&mut memory, &mut cpu, "RUN"));
    assert_eq!(
        &memory.bytes[0x1e01..usize::from(word(&memory, 0x1b))],
        programme
    );
    assert!(execute_direct(&mut memory, &mut cpu, "LIST"));
}

#[test]
fn basic_rom_circle_execution_cost_is_bounded_on_the_guest() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    let mut initialise = Vec::new();
    call(&mut initialise, symbol("GFX_INIT"));
    memory.execute(&mut cpu, &initialise);
    assert!(execute_direct(&mut memory, &mut cpu, "PMODE4,1:PCLS0"));
    let start = memory.cycles;
    assert!(execute_direct_with_budget(
        &mut memory,
        &mut cpu,
        "CIRCLE(128,96),60,1,.6",
        33_554_432
    ));
    let cycles = memory.cycles - start;
    eprintln!("CIRCLE guest cycles: {cycles}");
    assert!(cycles <= 1_500_000, "CIRCLE used {cycles} guest cycles");
    // Exact legacy pixels, including its truncation, are checked by the
    // independent canvas fixtures rather than an ideal mathematical ellipse.
    assert_eq!(
        memory.bytes[0x600..0x1e00]
            .iter()
            .map(|v| v.count_ones())
            .sum::<u32>(),
        272
    );
}

#[test]
fn basic_rom_unsigned_product_retains_all_bits_and_registers() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    for left in [0_u16, 1, 127, 128, 255, 256, 32767, 32768, 65534, 65535] {
        for right in (0..=65535_u16).step_by(257).chain([1, 32767, 32768, 65534]) {
            let mut code = vec![0xcc];
            code.extend(left.to_be_bytes());
            code.push(0x8e);
            code.extend(right.to_be_bytes());
            call(&mut code, symbol("PRODUCT_16"));
            cpu.registers.u = 0x6789;
            let before = memory.cycles;
            memory.execute(&mut cpu, &code);
            assert_eq!(
                (u32::from(cpu.registers.y) << 16) | u32::from(cpu.registers.d()),
                u32::from(left) * u32::from(right),
                "{left} * {right}"
            );
            assert_eq!((cpu.registers.x, cpu.registers.u), (right, 0x6789));
            assert!(memory.cycles - before < 1000);
        }
    }
}

#[test]
fn basic_rom_circle_raster_matches_original_modes_arcs_clipping_and_bounds() {
    use sha2::{Digest, Sha256};
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/circle-raster.json")).unwrap();
    for expected in fixtures["cases"].as_array().unwrap() {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        let mut init = Vec::new();
        call(&mut init, symbol("GFX_INIT"));
        memory.execute(&mut cpu, &init);
        let source = expected["source"].as_str().unwrap();
        let success = execute_direct_with_budget(&mut memory, &mut cpu, source, 4_194_304);
        assert_eq!(success, expected["success"].as_bool().unwrap(), "{source}");
        let length = usize::try_from(expected["graphics"]["bytes"].as_u64().unwrap()).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&memory.bytes[0x600..0x600 + length])),
            expected["graphics"]["sha256"].as_str().unwrap(),
            "{source}"
        );
        assert_eq!(cpu.registers.s, 0x7000, "{source}");
    }
}

#[test]
fn basic_rom_circle_chords_clip_before_addressing_and_keep_public_workspace() {
    for (source, point) in [
        ("CIRCLE(0,0),60,1,.6", (60, 0)),
        ("CIRCLE(255,191),60,1,.6", (195, 191)),
        ("CIRCLE(128,96),300,1,.2,0,.25", (128, 155)),
        ("CIRCLE(128,96),0,1,1", (128, 96)),
    ] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        let mut initialise = Vec::new();
        call(&mut initialise, symbol("GFX_INIT"));
        memory.execute(&mut cpu, &initialise);
        assert!(execute_direct(&mut memory, &mut cpu, "PMODE4,1:PCLS0"));
        memory.bytes[0xd8..0xea].fill(0xa7);
        let programme = memory.bytes[0x1e00..0x2000].to_vec();
        assert!(
            execute_direct_with_budget(&mut memory, &mut cpu, source, 8_388_608),
            "{source}"
        );
        assert_eq!(memory.bytes[0xd8..0xea], [0xa7; 18], "{source}");
        assert_eq!(memory.bytes[0x1e00..0x2000], programme, "{source}");
        let (x, y) = point;
        assert_ne!(
            memory.bytes[0x600 + y * 32 + x / 8] & (0x80 >> (x % 8)),
            0,
            "{source}"
        );
    }
}

#[test]
fn basic_rom_circle_radius_and_aspect_quantisation_match_lawful_pixels() {
    for (radius, ratio, width, height) in [
        (10.9, 1.0, 10, 10),
        (10.9, 1.5, 10, 15),
        (60.0, 0.6, 60, 35),
        (10.0, 0.6, 10, 5),
        (100.0, 0.101, 100, 9),
        (10.0, 0.1001, 10, 0),
    ] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        let mut initialise = Vec::new();
        call(&mut initialise, symbol("GFX_INIT"));
        memory.execute(&mut cpu, &initialise);
        assert!(execute_direct(&mut memory, &mut cpu, "PMODE4,1:PCLS0"));
        assert!(execute_direct(
            &mut memory,
            &mut cpu,
            &format!("CIRCLE(100,90),{radius},1,{ratio}")
        ));
        for (x, y) in [
            (100 + width, 90),
            (100 - width, 90),
            (100, 90 + height),
            (100, 90 - height),
        ] {
            assert!(execute_direct(
                &mut memory,
                &mut cpu,
                &format!("A=PPOINT({x},{y})")
            ));
            assert_eq!(numeric_scalar(&memory, [b'A', 0]), packed_integer(1));
        }
        assert!(execute_direct(
            &mut memory,
            &mut cpu,
            &format!("A=PPOINT(100,{})", 91 + height)
        ));
        assert_eq!(numeric_scalar(&memory, [b'A', 0]), packed_integer(0));
    }
}

#[test]
fn basic_rom_compact_fn_and_else_terminated_commands_recover_cleanly() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    store_line(&mut memory, &mut cpu, 10, "DEF FNF(X)=X*X+1");
    store_line(&mut memory, &mut cpu, 20, "Z=FNF(3):PRINT Z");
    assert!(execute_direct(&mut memory, &mut cpu, "RUN"));
    assert_eq!(memory.output, b" 10 \r");
    let mut initialise = Vec::new();
    call(&mut initialise, symbol("GFX_INIT"));
    memory.execute(&mut cpu, &initialise);
    assert!(execute_direct(&mut memory, &mut cpu, "PMODE4,1:PCLS1:A=0"));
    for source in [
        "IF 1 THEN PCLS ELSE A=99",
        "IF 1 THEN LIST ELSE A=99",
        "IF 1 THEN LIST10- ELSE A=99",
        "IF 1 THEN CLS ELSE A=99",
        "IF 1 THEN CIRCLE(100,90),10,1,.5 ELSE A=99",
    ] {
        assert!(execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(numeric_scalar(&memory, [b'A', 0]), packed_integer(0));
    }
}

#[test]
fn basic_rom_circle_arcs_clip_and_validate_before_plotting() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    let mut initialise = Vec::new();
    call(&mut initialise, symbol("GFX_INIT"));
    memory.execute(&mut cpu, &initialise);
    assert!(execute_direct(&mut memory, &mut cpu, "PMODE 4,1:PCLS"));
    for source in [
        "CIRCLE(10,10),-1",
        "CIRCLE(10,256),5",
        "CIRCLE(10,10),5,,256",
        "CIRCLE(10,10),5,,1,-.1,1",
        "CIRCLE(10,10),5,,1,0,1.1",
        "CIRCLE(10,10),5,,1,0,1,2",
    ] {
        let before = memory.bytes[0x600..0x1e00].to_vec();
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(memory.bytes[0x600..0x1e00], before, "{source}");
    }
    // Equal quantised endpoints select the observed complete turn.
    guest_tokens(&mut memory, &mut cpu, "CIRCLE(100,90),10,,1,0,.001");
    let mut code = vec![0x8e, 2, 0xdd];
    call(&mut code, symbol("EXECUTE"));
    memory.execute_with_budget(&mut cpu, &code, 8_388_608);
    assert_eq!(cpu.registers.cc & C, 0);
    assert_eq!(cpu.registers.s, 0x7000);
    assert!(execute_direct(&mut memory, &mut cpu, "A=PPOINT(110,90)"));
    assert_eq!(numeric_scalar(&memory, [b'A', 0]), packed_integer(1));
    assert!(execute_direct(&mut memory, &mut cpu, "PCLS0"));
    assert!(execute_direct_with_budget(
        &mut memory,
        &mut cpu,
        "CIRCLE(0,0),10,1,1,.5,.75",
        16_777_216
    ));
    assert_eq!(memory.bytes[0x600], 0x80);
    assert!(memory.bytes[0x601..0x1e00].iter().all(|&byte| byte == 0));
}

#[test]
fn basic_rom_equal_circle_endpoints_select_a_full_turn() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    let mut initialise = Vec::new();
    call(&mut initialise, symbol("GFX_INIT"));
    memory.execute(&mut cpu, &initialise);
    assert!(execute_direct(&mut memory, &mut cpu, "PMODE4,1:PCLS0"));
    assert!(execute_direct_with_budget(
        &mut memory,
        &mut cpu,
        "CIRCLE(100,90),10,1,1,.25,.25",
        33_554_432
    ));
    for (x, y) in [(110, 90), (100, 100), (90, 90), (100, 80)] {
        assert!(execute_direct(
            &mut memory,
            &mut cpu,
            &format!("A=PPOINT({x},{y})")
        ));
        assert_eq!(numeric_scalar(&memory, [b'A', 0]), packed_integer(1));
    }
}

#[test]
fn basic_rom_circles_and_ellipses_cover_cardinals_in_every_mode() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    let mut initialise = Vec::new();
    call(&mut initialise, symbol("GFX_INIT"));
    memory.execute(&mut cpu, &initialise);
    for mode in 0..=4 {
        let colour = if mode & 1 == 0 { 1 } else { 2 };
        assert!(execute_direct(
            &mut memory,
            &mut cpu,
            &format!("PMODE {mode},1:PCLS{}", u8::from(mode & 1 != 0))
        ));
        assert!(execute_direct_with_budget(
            &mut memory,
            &mut cpu,
            &format!("CIRCLE(100,90),10,{colour},.5"),
            33_554_432
        ));
        // Lower modes truncate the physical radius before applying aspect.
        let height = if mode < 2 { 4 } else { 5 };
        for (x, y) in [(110, 90), (100, 90 + height), (90, 90), (100, 90 - height)] {
            assert!(execute_direct(
                &mut memory,
                &mut cpu,
                &format!("A=PPOINT({x},{y})")
            ));
            assert_eq!(
                numeric_scalar(&memory, [b'A', 0]),
                packed_integer(colour),
                "PMODE {mode}: {x},{y}"
            );
        }
        assert!(execute_direct(&mut memory, &mut cpu, "A=PPOINT(100,90)"));
        assert_eq!(
            numeric_scalar(&memory, [b'A', 0]),
            packed_integer(i16::from(mode & 1 != 0))
        );
    }
}
#[test]
fn basic_rom_arrays_match_observed_legacy_descriptors_and_element_order() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    assert!(execute_direct(&mut memory, &mut cpu, "DIM A(1,2)"));
    assert_eq!(
        &memory.bytes[0x1e03..0x1e0c],
        &[65, 0, 0, 39, 2, 0, 3, 0, 2]
    );
    for (source, value) in [
        ("A(0,0)=1", 1),
        ("A(1,0)=2", 2),
        ("A(0,1)=3", 3),
        ("A(1,1)=4", 4),
        ("A(0,2)=5", 5),
        ("A(1,2)=6", 6),
    ] {
        assert!(execute_direct(&mut memory, &mut cpu, source), "{source}");
        let offset = 0x1e0c + usize::try_from(value - 1).unwrap() * 5;
        assert_eq!(&memory.bytes[offset..offset + 5], &packed_integer(value));
    }
    assert!(execute_direct(&mut memory, &mut cpu, "Z=7:PRINT A(1,Z-5)"));
    assert_eq!(memory.output, b" 6 \r");
    assert_eq!(
        &memory.bytes[0x1e0a..0x1e13],
        &[65, 0, 0, 39, 2, 0, 3, 0, 2]
    );
    assert_eq!(word(&memory, 0x1f), 0x1e31);
    for source in [
        "DIM A(1,2)",
        "A(2,0)=9",
        "A(0,3)=9",
        "A(-1,0)=9",
        "A(1)=9",
        "A(1,2,0)=9",
        "DIM B(32767,32767)",
        "DIM B()",
    ] {
        let end = word(&memory, 0x1f);
        let old = memory.bytes[0x1e03..usize::from(end)].to_vec();
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(word(&memory, 0x1f), end, "{source}");
        assert_eq!(&memory.bytes[0x1e03..usize::from(end)], old, "{source}");
    }
    assert!(execute_direct(&mut memory, &mut cpu, "B(10)=9:PRINT B(10)"));
    let base = 0x1e31;
    assert_eq!(&memory.bytes[base..base + 7], &[66, 0, 0, 62, 1, 0, 11]);
    assert_eq!(&memory.bytes[base + 57..base + 62], &packed_integer(9));
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "C(1)=2:A(B(0),C(1))=11"
    ));
    assert!(execute_direct(&mut memory, &mut cpu, "PRINT A(0,2)"));
    assert!(memory.output.ends_with(b" 11 \r"));
}
#[test]
fn basic_rom_string_scalars_arrays_and_heap_match_original_synthetic_observations() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "A$=\"HELLO\":B$=A$:C$=A$+\"!\":Z=7"
    ));
    for (address, descriptor) in [
        (0x1e03, [65, 128, 5, 0, 127, 250, 0]),
        (0x1e0a, [66, 128, 5, 0, 127, 245, 0]),
        (0x1e11, [67, 128, 6, 0, 127, 238, 0]),
    ] {
        assert_eq!(&memory.bytes[address..address + 7], &descriptor);
    }
    assert_eq!(word(&memory, 0x23), 32749);
    assert!(execute_direct(&mut memory, &mut cpu, "PRINT A$;B$;C$"));
    assert_eq!(memory.output, b"HELLOHELLOHELLO!\r");
    for (source, expected) in [
        ("PRINT LEN(A$)", " 5 \r"),
        ("PRINT ASC(B$)", " 72 \r"),
        ("PRINT CHR$(65)", "A\r"),
        ("PRINT VAL(\" -12.5X\")", "-12.5 \r"),
        ("PRINT VAL(\"NO\")", " 0 \r"),
        ("PRINT A$=B$;A$<C$;A$>C$", "-1 -1  0 \r"),
        ("PRINT \"A\"+\"B\"+\"C\"", "ABC\r"),
    ] {
        memory.output.clear();
        assert!(execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(memory.output, expected.as_bytes(), "{source}");
    }
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "DIM D$(2):D$(1)=C$:PRINT D$(1)"
    ));
    assert!(memory.output.ends_with(b"HELLO!\r"));
    for source in [
        "A$=1",
        "Z=A$",
        "PRINT A$-B$",
        "PRINT 1+A$",
        "PRINT A$+1",
        "PRINT LEN(7)",
        "PRINT ASC(\"\")",
        "PRINT CHR$(256)",
        "PRINT -A$",
    ] {
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
    }
    // Scalar creation during the RHS relocates an existing array LHS safely.
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "DIM N(1):N(1)=UN:PRINT N(1)"
    ));
    assert!(memory.output.ends_with(b" 0 \r"));
}
#[test]
fn basic_rom_for_next_and_gosub_execute_nested_original_programmes() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    for (line, body) in [
        (10, "FOR I=2 TO 5 STEP 2"),
        (20, "GOSUB 100"),
        (30, "NEXT I"),
        (40, "FOR J=3 TO 1 STEP -1:PRINT J;:NEXT J"),
        (50, "END"),
        (100, "FOR K=1 TO 2:PRINT I+K;:NEXT K"),
        (110, "RETURN"),
    ] {
        store_line(&mut memory, &mut cpu, line, body);
    }
    assert!(execute_direct(&mut memory, &mut cpu, "RUN"));
    assert_eq!(memory.output, b" 3  4  5  6  3  2  1 ");
    memory.output.clear();
    for source in [
        "FOR I=1 TO 2:PRINT I;:NEXT",
        "FOR I=1 TO 2:FOR J=1 TO 2:PRINT I*10+J;:NEXT J,I",
    ] {
        assert!(execute_direct(&mut memory, &mut cpu, source), "{source}");
    }
    assert_eq!(memory.output, b" 1  2  11  12  21  22 ");
    memory.output.clear();
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "FOR I=.125 TO 5 STEP 2:PRINT I;:NEXT"
    ));
    assert_eq!(memory.output, b" .125  2.125  4.125 ");
    for source in [
        "NEXT",
        "RETURN",
        "GOSUB 999",
        "FOR I=1",
        "FOR I=1 TO \"X\"",
        "FOR I$=1 TO 2",
        "FOR I(0)=1 TO 2",
        "FOR I=1 TO 2 STEP",
        "FOR I=1 TO 2:NEXT J",
    ] {
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
    }
}
#[test]
fn basic_rom_guest_exec_observes_legacy_for_and_gosub_stack_records() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    // Our original guest probe records S and copies the stack after its return slot.
    let probe = [
        0x1f, 0x40, 0xfd, 0x63, 0x00, 0x1f, 0x41, 0x30, 0x02, 0x10, 0x8e, 0x64, 0x00, 0xc6, 0x40,
        0xa6, 0x80, 0xa7, 0xa0, 0x5a, 0x26, 0xf9, 0x39,
    ];
    memory.bytes[0x6000..0x6000 + probe.len()].copy_from_slice(&probe);
    for (line, body) in [
        (10, "FOR I=2 TO 5 STEP 2"),
        (20, "GOSUB 100"),
        (30, "NEXT I"),
        (40, "END"),
        (100, "EXEC 24576"),
        (110, "RETURN"),
    ] {
        store_line(&mut memory, &mut cpu, line, body);
    }
    assert!(execute_direct(&mut memory, &mut cpu, "RUN"));
    let frame = &memory.bytes[0x6400..0x6419];
    assert_eq!(frame[0], 0xbd);
    assert_eq!(&frame[1..3], &20_u16.to_be_bytes());
    assert_eq!(&frame[5..7], &symbol("EXEC_AFTER_STATEMENT").to_be_bytes());
    assert_eq!(frame[7], 0x80);
    assert_eq!(&frame[8..10], &(word(&memory, 0x1b) + 2).to_be_bytes());
    assert_eq!(&frame[10..15], &[130, 128, 0, 0, 0]);
    assert_eq!(frame[15], 1);
    assert_eq!(&frame[16..21], &packed_integer(5));
    assert_eq!(&frame[21..23], &10_u16.to_be_bytes());
    let s = word(&memory, 0x6300);
    assert_eq!(word(&memory, usize::from(symbol("CTL_BASE"))) - s, 27); // EXEC return + GOSUB7 + FOR18.
}
fn cold_boot_to_probe(source: &str) -> Memory {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    // Original RAM keyboard supplier replaces our own KEYIN entry for this
    // ABI-only test. It does not claim to verify physical matrix scanning.
    let key = usize::from(symbol("KEYIN"));
    memory.bytes[key..key + 3].copy_from_slice(&[0x7e, 0x65, 0x00]);
    memory.bytes[0x6500..0x6510].copy_from_slice(&[
        0x34, 0x10, 0xbe, 0x66, 0x00, 0xa6, 0x80, 0xbf, 0x66, 0x00, 0x4d, 0x35, 0x90, 0x12, 0x12,
        0x12,
    ]);
    set_word(&mut memory, 0x6600, 0x6800);
    let prefix = "POKE 25000,31:POKE 25001,64:POKE 25002,253:POKE 25003,98:POKE 25004,0:POKE 25005,126:POKE 25006,98:POKE 25007,14\r";
    let text = format!("{prefix}{source}");
    assert!(text.len() < 1536);
    memory.bytes[0x6800..0x6800 + text.len()].copy_from_slice(text.as_bytes());
    cpu.reset(&mut memory);
    let mut reached = false;
    for _ in 0..8_388_608 {
        if cpu.registers.pc == 0x620e {
            reached = true;
            break;
        }
        cpu.step(&mut memory).unwrap();
    }
    assert!(
        reached,
        "cold-boot ABI probe did not reach its original RAM trap"
    );
    memory
}
#[test]
fn basic_rom_cold_boot_control_stack_matches_lawful_reference_observations() {
    let memory = cold_boot_to_probe(
        "10 FOR I=2 TO 5 STEP 2\r20 GOSUB 100\r30 NEXT I\r40 END\r100 EXEC 25000\r110 RETURN\rRUN\r",
    );
    assert_eq!(word(&memory, 0x27), 32766);
    assert_eq!(word(&memory, 0x21), 32566);
    assert_eq!(word(&memory, 0x6200), 32538);
    assert_eq!(word(&memory, 0x68), 100);
    let stack = &memory.bytes[32540..32567];
    assert_eq!(&stack[..5], &[0xbd, 0, 20, 0x1e, 0x1a]);
    assert_eq!(
        &stack[7..],
        &[
            0x80, 0x1e, 0x42, 130, 128, 0, 0, 0, 1, 131, 32, 0, 0, 0, 0, 10, 0x1e, 0x12, 0, 0
        ]
    );
}
#[test]
fn basic_rom_cold_boot_continue_preserves_control_frames_and_clear_relocates_stack() {
    let memory = cold_boot_to_probe(
        "10 FOR I=1 TO 2\r20 STOP\r30 I=I+5\r40 NEXT I\r50 EXEC 25000\rRUN\rA=9\rCONT\r",
    );
    assert_eq!(word(&memory, 0x29), 20);
    assert_eq!(word(&memory, 0x68), 50);
    assert_eq!(word(&memory, 0x6200), 32563);
    let scalar = usize::from(word(&memory, 0x1b));
    assert_eq!(&memory.bytes[scalar + 2..scalar + 7], &packed_integer(7));
    let memory = cold_boot_to_probe("CLEAR 201\rCLEAR 50\rCLEAR 100,20000\rEXEC 25000\r");
    assert_eq!(word(&memory, 0x27), 19999);
    assert_eq!(word(&memory, 0x21), 19899);
    assert_eq!(word(&memory, 0x6200), 19896);
}
#[test]
fn basic_rom_string_collection_preserves_scalars_arrays_and_pending_expression_roots() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "DIM B$(2):A$=\"HELLO\":B$(1)=\"WORLD\""
    ));
    for iteration in 0..100 {
        assert!(
            execute_direct(&mut memory, &mut cpu, "C$=\"ABC\"+A$+(B$(1)+\"!\")"),
            "iteration {iteration}, free {:04x}, head {:04x}, mode {}, roots {:04x}, GC top {:04x}",
            word(&memory, 0x23),
            word(&memory, 0x47),
            memory.bytes[0x43],
            word(&memory, 0xea),
            word(&memory, 0x49)
        );
        assert_eq!(word(&memory, 0xea), 0, "temporary root chain must unwind");
    }
    memory.output.clear();
    assert!(execute_direct(&mut memory, &mut cpu, "PRINT A$;B$(1);C$"));
    assert_eq!(memory.output, b"HELLOWORLDABCHELLOWORLD!\r");
    let large = format!("C$=\"{}\"+\"{}\"", "X".repeat(95), "Y".repeat(95));
    assert!(!execute_direct(&mut memory, &mut cpu, &large));
    assert_eq!(word(&memory, 0xea), 0);
    memory.output.clear();
    assert!(execute_direct(&mut memory, &mut cpu, "PRINT C$"));
    assert_eq!(memory.output, b"ABCHELLOWORLD!\r");
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "C$=\"RECOVERED\":PRINT C$"
    ));
    assert!(memory.output.ends_with(b"RECOVERED\r"));
}
#[test]
fn basic_rom_clear_reserve_and_exclusive_memory_limit_match_reference() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    assert!(execute_direct(&mut memory, &mut cpu, "CLEAR 50"));
    assert_eq!(word(&memory, 0x21), 32716);
    assert!(execute_direct(&mut memory, &mut cpu, "CLEAR"));
    assert_eq!(word(&memory, 0x21), 32716);
    assert!(execute_direct(&mut memory, &mut cpu, "CLEAR 100,20000"));
    assert_eq!(word(&memory, 0x27), 19999);
    assert_eq!(word(&memory, 0x23), 19999);
    assert_eq!(word(&memory, 0x21), 19899);
    assert!(execute_direct(&mut memory, &mut cpu, "CLEAR 1000,32767"));
    let source = format!("A$=\"{}\":B$=A$:PRINT LEN(B$)", "Z".repeat(100));
    assert!(execute_direct(&mut memory, &mut cpu, &source));
    assert_eq!(memory.output, b" 100 \r");
    for source in [
        "CLEAR -1",
        "CLEAR 1,0",
        "CLEAR 65535",
        "CLEAR 1,32768",
        "CLEAR 1,500",
        "CLEAR 10,20000 GARBAGE",
        "NEW 1",
    ] {
        let before = [
            word(&memory, 0x21),
            word(&memory, 0x23),
            word(&memory, 0x27),
        ];
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(
            [
                word(&memory, 0x21),
                word(&memory, 0x23),
                word(&memory, 0x27)
            ],
            before,
            "{source}"
        );
    }
}
#[test]
fn basic_rom_read_restore_and_data_workspace_match_reference() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    for (line, body) in [
        (10, "DATA 1,\"HELLO\",2:DATA 3"),
        (20, "READ X,Y$"),
        (30, "END"),
    ] {
        store_line(&mut memory, &mut cpu, line, body);
    }
    assert!(execute_direct(&mut memory, &mut cpu, "RUN"));
    assert_eq!(word(&memory, 0x31), 10);
    assert_eq!(word(&memory, 0x33), 7696);
    let first = usize::from(word(&memory, 0x1b));
    assert_eq!(&memory.bytes[first..first + 7], &[88, 0, 129, 0, 0, 0, 0]);
    assert_eq!(
        &memory.bytes[first + 7..first + 14],
        &[89, 128, 5, 0, 30, 10, 0]
    );
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "READ Z,W:PRINT X;Y$;Z;W"
    ));
    assert_eq!(memory.output, b" 1 HELLO 2  3 \r");
    assert!(!execute_direct(&mut memory, &mut cpu, "READ Z"));
    assert!(execute_direct(&mut memory, &mut cpu, "RESTORE"));
    assert_eq!(word(&memory, 0x31), 10);
    assert_eq!(word(&memory, 0x33), 7680);
    assert!(execute_direct(&mut memory, &mut cpu, "READ Z:PRINT Z"));
    assert!(memory.output.ends_with(b" 1 \r"));
    assert!(!execute_direct(&mut memory, &mut cpu, "RESTORE 10"));
    assert!(execute_direct(&mut memory, &mut cpu, "NEW"));
    for (line, body) in [
        (10, "A=LOG(1):REM DATA 999"),
        (20, "A=ABS(-1):DATA -12.5, UNQUOTED  ,\"A:B,C\""),
        (30, "READ A,B$,C$:PRINT A;B$;C$:END"),
    ] {
        store_line(&mut memory, &mut cpu, line, body);
    }
    memory.output.clear();
    assert!(execute_direct(&mut memory, &mut cpu, "RUN"));
    assert_eq!(memory.output, b"-12.5 UNQUOTEDA:B,C\r");
}
#[test]
fn basic_rom_type_errors_and_skipped_functions_do_not_mutate_or_execute() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    memory.bytes[0] = 0xa5;
    memory.bytes[25000] = 0x5a;
    memory.bytes[0x6000..0x6006].copy_from_slice(&[0x86, 0x77, 0xb7, 0x61, 0xa8, 0x39]);
    for source in [
        "POKE \"X\",7",
        "POKE 25000,\"X\"",
        "POKE 25000,7 GARBAGE",
        "EXEC \"X\"",
        "EXEC 24576 GARBAGE",
        "IF \"X\" THEN POKE 25000,9",
        "END 5",
    ] {
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(memory.bytes[0], 0xa5, "{source}");
        assert_eq!(memory.bytes[25000], 0x5a, "{source}");
    }
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "IF 0 THEN PRINT SQR(4) ELSE PRINT 7"
    ));
    assert_eq!(memory.output, b" 7 \r");
}
#[test]
fn basic_rom_guest_commands_scalar_layout_and_programme_execution() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    assert!(execute_direct(&mut memory, &mut cpu, "A=2+3*4:PRINT A"));
    assert_eq!(memory.output, b" 14 \r");
    assert_eq!(&memory.bytes[0x1e03..0x1e05], b"A\0");
    assert_eq!(&memory.bytes[0x1e05..0x1e0a], &packed_integer(14));
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "LET A=A/8:PRINT \"RESULT=\";A;"
    ));
    assert_eq!(memory.output, b" 14 \rRESULT= 1.75 ");
    for (line, body) in [
        (10, "A=5:PRINT A;"),
        (20, "GO TO 40"),
        (30, "PRINT 999"),
        (40, "A=A+1:PRINT A:END"),
        (50, "PRINT 999"),
    ] {
        store_line(&mut memory, &mut cpu, line, body);
    }
    memory.output.clear();
    assert!(execute_direct(&mut memory, &mut cpu, "RUN"));
    assert_eq!(memory.output, b" 5  6 \r");
    assert_eq!(word(&memory, 0x68), 0xffff);
    assert_eq!(
        &memory.bytes[usize::from(word(&memory, 0x1b)) + 2..usize::from(word(&memory, 0x1b)) + 7],
        &packed_integer(6)
    );
    memory.output.clear();
    assert!(execute_direct(&mut memory, &mut cpu, "LIST"));
    assert_eq!(
        memory.output,
        b"10 A=5:PRINT A;\r20 GO TO 40\r30 PRINT 999\r40 A=A+1:PRINT A:END\r50 PRINT 999\r"
    );
    assert!(execute_direct(&mut memory, &mut cpu, "CLEAR"));
    assert_eq!(word(&memory, 0x1b), word(&memory, 0x1d));
    assert!(execute_direct(&mut memory, &mut cpu, "NEW"));
    assert_eq!(word(&memory, 0x1b), 0x1e03);
    assert_eq!(&memory.bytes[0x1e01..0x1e03], &[0, 0]);
}
#[test]
fn basic_rom_guest_number_presentation_and_failed_statement_recovery() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    for (source, expected) in [
        ("PRINT 0", " 0 \r"),
        ("PRINT -32768", "-32768 \r"),
        ("PRINT .125", " .125 \r"),
        ("PRINT .01", " .01 \r"),
        ("PRINT 1E-3", " 1E-03 \r"),
        ("PRINT 1E9", " 1E+09 \r"),
        ("PRINT 1/3", " .333333333 \r"),
        ("PRINT 100.5", " 100.5 \r"),
    ] {
        memory.output.clear();
        assert!(execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(memory.output, expected.as_bytes(), "{source}");
    }
    assert!(execute_direct(&mut memory, &mut cpu, "A=7"));
    for source in ["A=1/0", "LET A=", "PRINT 1+", "GOTO999", "SIN(0)"] {
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(&memory.bytes[0x1e05..0x1e0a], &packed_integer(7));
    }
    memory.output.clear();
    assert!(execute_direct(&mut memory, &mut cpu, "PRINT A"));
    assert_eq!(memory.output, b" 7 \r");
}
#[test]
fn basic_rom_guest_conditionals_poke_peek_and_machine_code_call() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    for (source, expected) in [
        ("IF 1 THEN PRINT 7 ELSE PRINT 8", " 7 \r"),
        ("IF 0 THEN PRINT 7 ELSE PRINT 8", " 8 \r"),
        ("IF 0 THEN PRINT 7", ""),
        ("IF 0 THEN PRINT \"ELSE:IF\" ELSE PRINT 9", " 9 \r"),
        ("POKE 25000,171:PRINT PEEK(25000)", " 171 \r"),
    ] {
        memory.output.clear();
        assert!(execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(memory.output, expected.as_bytes(), "{source}");
    }
    memory.bytes[0x6200..0x6206].copy_from_slice(&[0x86, 0x5a, 0xb7, 0x63, 0, 0x39]);
    assert!(execute_direct(&mut memory, &mut cpu, "EXEC 25088"));
    assert_eq!(memory.bytes[0x6300], 0x5a);
    for (line, source) in [(10, "IF 1 THEN 30"), (20, "PRINT 999"), (30, "PRINT 1:END")] {
        store_line(&mut memory, &mut cpu, line, source);
    }
    memory.output.clear();
    assert!(execute_direct(&mut memory, &mut cpu, "RUN"));
    assert_eq!(memory.output, b" 1 \r");
    for source in [
        "POKE 25000,256",
        "POKE -1,7",
        "POKE 25000,1/0",
        "IF 1 PRINT 1",
        "IF 1 THEN 999",
    ] {
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(
            memory.bytes[25000], 171,
            "failed POKE preserves destination"
        );
    }
}
fn unpacked_number(memory: &Memory) -> f64 {
    let exponent = memory.bytes[0x4f];
    if exponent == 0 {
        return 0.0;
    }
    let mantissa = u32::from_be_bytes(memory.bytes[0x50..0x54].try_into().unwrap());
    let magnitude = f64::from(mantissa) * 2_f64.powi(i32::from(exponent) - 160);
    if memory.bytes[0x54] & 128 != 0 {
        -magnitude
    } else {
        magnitude
    }
}
#[test]
fn basic_rom_original_real_functions_have_bounded_error_and_domain_recovery() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    for (routine, argument) in [
        ("F_SQR", 0.0_f64),
        ("F_SQR", 2.0),
        ("F_SQR", 9.0),
        ("F_SQR", 2_f64.powi(-128)),
        ("F_SQR", 2_f64.powi(126)),
        ("F_LOG", 1.0),
        ("F_LOG", 1.5),
        ("F_LOG", 2.0),
        ("F_LOG", 0.01),
        ("F_LOG", 2_f64.powi(-128)),
        ("F_LOG", 2_f64.powi(126)),
        ("F_EXP", 0.0),
        ("F_EXP", 1.0),
        ("F_EXP", -1.0),
        ("F_EXP", 10.0),
        ("F_EXP", -10.0),
        ("F_EXP", 80.0),
        ("F_EXP", -80.0),
    ] {
        unpack_to(&mut memory, 0x4f, packed_number(argument));
        let mut code = Vec::new();
        call(&mut code, symbol(routine));
        memory.execute_with_budget(&mut cpu, &code, 1_048_576);
        assert_eq!(cpu.registers.cc & C, 0, "{routine}({argument})");
        let expected = match routine {
            "F_SQR" => argument.sqrt(),
            "F_LOG" => argument.ln(),
            _ => argument.exp(),
        };
        let result = unpacked_number(&memory);
        let tolerance = 2e-8 * expected.abs().max(1e-38);
        assert!(
            (result - expected).abs() <= tolerance,
            "{routine}({argument}): {result} versus {expected}"
        );
        assert_eq!(cpu.registers.s, 0x7000);
    }
    for (routine, argument) in [
        ("F_SQR", -1.0),
        ("F_LOG", 0.0),
        ("F_LOG", -1.0),
        ("F_EXP", 90.0),
    ] {
        unpack_to(&mut memory, 0x4f, packed_number(argument));
        let mut code = Vec::new();
        call(&mut code, symbol(routine));
        memory.execute_with_budget(&mut cpu, &code, 1_048_576);
        assert_ne!(
            cpu.registers.cc & C,
            0,
            "domain failure {routine}({argument})"
        );
        assert_eq!(cpu.registers.s, 0x7000);
    }
}
#[test]
fn basic_rom_real_rounding_trigonometry_and_general_powers() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    for argument in [
        -1e20_f64, -32768.75, -1.75, -0.125, 0.0, 0.125, 32768.75, 1e20,
    ] {
        for (routine, expected) in [
            ("F_FIX_VALUE", argument.trunc()),
            ("F_INT_VALUE", argument.floor()),
        ] {
            unpack_to(&mut memory, 0x4f, packed_number(argument));
            let mut code = Vec::new();
            call(&mut code, symbol(routine));
            code.extend([0x8e, 0x61, 0]);
            call(&mut code, symbol("FAC_STORE"));
            memory.execute(&mut cpu, &code);
            assert_eq!(
                &memory.bytes[0x6100..0x6105],
                &packed_number(expected),
                "{routine}({argument})"
            );
        }
    }
    for index in -60..=60 {
        let argument = f64::from(index) / 10.0;
        for (routine, expected) in [
            ("F_SIN", argument.sin()),
            ("F_COS", argument.cos()),
            ("F_ATN", argument.atan()),
        ] {
            unpack_to(&mut memory, 0x4f, packed_number(argument));
            let mut code = Vec::new();
            call(&mut code, symbol(routine));
            memory.execute_with_budget(&mut cpu, &code, 1_048_576);
            assert_eq!(cpu.registers.cc & C, 0, "{routine}({argument})");
            let result = unpacked_number(&memory);
            assert!(
                (result - expected).abs() <= 2e-8,
                "{routine}({argument}): {result} versus {expected}"
            );
            assert_eq!(cpu.registers.s, 0x7000);
        }
    }
    empty_programme(&mut memory);
    for (source, expected) in [
        ("2^.5", 2.0_f64.sqrt()),
        ("1.5^3.25", 1.5_f64.powf(3.25)),
        ("1^32767", 1.0),
        ("0^.5", 0.0),
        ("SIN(1)+COS(1)", 1.0_f64.sin() + 1.0_f64.cos()),
        ("ATN(1)", std::f64::consts::FRAC_PI_4),
        ("TAN(.5)", 0.5_f64.tan()),
        ("SQR(4)+LOG(EXP(1))", 3.0),
    ] {
        guest_tokens(&mut memory, &mut cpu, source);
        let mut code = vec![0x8e, 2, 0xdd];
        call(&mut code, symbol("EVAL"));
        memory.execute_with_budget(&mut cpu, &code, 1_048_576);
        assert_eq!(cpu.registers.cc & C, 0, "{source}");
        let result = unpacked_number(&memory);
        assert!(
            (result - expected).abs() <= 2e-8 * expected.abs().max(1.0),
            "{source}: {result} versus {expected}"
        );
    }
}
#[test]
fn basic_rom_line_input_stays_inside_the_legacy_buffer_and_rejects_overflow() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    // An original guest keyboard fixture supplies characters while exercising
    // the actual line editor; ordinary hardware scanning has separate board tests.
    memory.bytes[0x6300..0x630c].copy_from_slice(&[
        0x34, 0x10, 0xbe, 0x64, 0, 0xa6, 0x80, 0xbf, 0x64, 0, 0x35, 0x90,
    ]);
    let key = usize::from(symbol("KEYIN"));
    memory.bytes[key..key + 3].copy_from_slice(&[0x7e, 0x63, 0]);
    memory.bytes[0x16a] = 0x39;
    memory.bytes[0x182] = 0x39;
    for length in [250_usize, 251, 252, 255, 1, 0] {
        set_word(&mut memory, 0x6400, 0x6402);
        memory.bytes[0x6402..0x6402 + length].fill(b'A');
        memory.bytes[0x6402 + length] = 13;
        memory.bytes[0x3d9..0x3eb].fill(0xa5);
        memory.bytes[0xd8..0xea].fill(0xa5);
        set_word(&mut memory, 0x88, 0x400);
        let mut code = Vec::new();
        call(&mut code, symbol("READLINE"));
        memory.execute_with_budget(&mut cpu, &code, 1_048_576);
        assert_eq!(
            cpu.registers.cc & C != 0,
            length > 251,
            "line length {length}"
        );
        assert_eq!(cpu.registers.x, 0x2dd);
        assert_eq!(memory.bytes[0x2dd + length.min(251)], 0);
        assert!(memory.bytes[0x3d9..0x3eb].iter().all(|&byte| byte == 0xa5));
        assert_eq!(&memory.bytes[0xd8..0xea], &[0xa5; 18]);
    }
    set_word(&mut memory, 0x6400, 0x6402);
    memory.bytes[0x6402] = 3;
    let mut code = Vec::new();
    call(&mut code, symbol("READLINE"));
    memory.execute_with_budget(&mut cpu, &code, 1_048_576);
    assert_ne!(cpu.registers.cc & C, 0);
    assert_eq!(cpu.registers.s, 0x7000);
    assert_eq!(memory.bytes[0x2dd], 0);
    assert_eq!(&memory.bytes[0xd8..0xea], &[0xa5; 18]);
}

fn guest_tokens(memory: &mut Memory, cpu: &mut Cpu, text: &str) -> Vec<u8> {
    memory.bytes[0x2dd..0x2dd + text.len()].copy_from_slice(text.as_bytes());
    memory.bytes[0x2dd + text.len()] = 0;
    memory.bytes[0x1a3] = 0x39;
    let mut code = vec![0x8e, 2, 0xdd];
    call(&mut code, symbol("TOKENISE"));
    // 256 input bytes and the fixed vocabulary bound the guest lexical search.
    memory.execute_with_budget(cpu, &code, 4_194_304);
    assert_eq!(cpu.registers.cc & C, 0, "tokenisation failed: {text}");
    memory.bytes[0x2dd..0x3d9]
        .iter()
        .take_while(|&&b| b != 0)
        .copied()
        .collect()
}
#[test]
fn basic_rom_machine_code_suffix_tokens_match_common_ide_forms() {
    use eightbit_dragon_basic_core::{Dialect, analyse, tokenise};
    let dialect = Dialect::dragon32().unwrap();
    for source in [
        "CLOADM\"FILE\"",
        "CSAVEM\"FILE\",24576,24577,24576",
        "PRINT\"CLOADM\":REM CSAVEM",
    ] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        let actual = guest_tokens(&mut memory, &mut cpu, source);
        let analysis = analyse(&format!("10 {source}\n"), &dialect);
        assert!(
            !analysis.has_errors(),
            "{source}: {:?}",
            analysis.diagnostics
        );
        let expected = tokenise(&analysis.program, &dialect).unwrap();
        assert_eq!(actual, expected.lines[0].code, "{source}");
    }
}

#[test]
fn basic_rom_guest_tokeniser_matches_vocabulary_and_common_ide_forms() {
    use eightbit_dragon_basic_core::{Dialect, analyse, tokenise};
    let dialect = Dialect::dragon32().unwrap();
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    for token in &dialect.statement_tokens {
        assert_eq!(
            guest_tokens(&mut memory, &mut cpu, &token.text),
            if matches!(token.code, 0x83 | 0x84) {
                vec![b':', token.code]
            } else {
                vec![token.code]
            }
        );
    }
    for token in &dialect.function_tokens {
        assert_eq!(
            guest_tokens(&mut memory, &mut cpu, &token.text),
            [255, token.code]
        );
    }
    for body in [
        "FOR I=1 TO 10",
        "PRINT \"GOTO DATA SIN\":DATA GOTO,SIN:REM PRINT GOTO",
        "DATA \"PRINT:GOTO\",SIN:GOTO20",
        "A=SGN(-10)+USR0(1)",
        "PRINT LEFT$(A$,1);TAB(2)",
        "SIN=1:PRINT SIN(1)",
        "A=10 AND 3 OR 8",
        "DEF FNF(X)=X*X+1",
        "Z=FNF(3)",
    ] {
        let analysis = analyse(&format!("10 {body}\n"), &dialect);
        let expected = tokenise(&analysis.program, &dialect).unwrap();
        assert_eq!(
            guest_tokens(&mut memory, &mut cpu, body),
            expected.lines[0].code,
            "{body}"
        );
    }
}
#[test]
fn basic_rom_tokeniser_rejects_overflow_without_touching_programme_memory() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    memory.bytes[0x1a3] = 0x39;
    memory.bytes[0x4000..0x4101].fill(b'A');
    memory.bytes[0x4101] = 0;
    memory.bytes[0x1e01..0x1e11].fill(0x51);
    let mut code = vec![0x8e, 0x40, 0];
    call(&mut code, symbol("TOKENISE"));
    memory.execute_with_budget(&mut cpu, &code, 4_194_304);
    assert_ne!(cpu.registers.cc & C, 0);
    assert!(memory.bytes[0x1e01..0x1e11].iter().all(|&v| v == 0x51));
}

#[test]
fn basic_rom_in_place_tokens_preserve_cassette_preamble_and_private_state() {
    use eightbit_dragon_basic_core::{Dialect, analyse, tokenise};
    let dialect = Dialect::dragon32().unwrap();
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    assert_eq!(symbol("TOKENBUF"), 0x2dd);
    for source in [
        "PRINT\"IF:DATA\":DATA \"GO:TO\",ABC:REM SIN".to_owned(),
        "A".repeat(251),
        "PRINT1:".repeat(35),
    ] {
        memory.bytes[0x1da..0x2dd].fill(0xa5);
        memory.bytes[0x3d9..0x400].fill(0x5a);
        let expected = tokenise(
            &analyse(&format!("10 {source}\n"), &dialect).program,
            &dialect,
        )
        .unwrap();
        let actual = guest_tokens(&mut memory, &mut cpu, &source);
        assert_eq!(actual, expected.lines[0].code);
        assert!(actual.len() <= source.len());
        assert_eq!(memory.bytes[0x2dd + actual.len()], 0);
        assert!(memory.bytes[0x1da..0x2dd].iter().all(|&b| b == 0xa5));
        assert!(memory.bytes[0x3d9..0x400].iter().all(|&b| b == 0x5a));
    }
    memory.bytes[0x4000..0x40fc].fill(b'A');
    memory.bytes[0x40fc] = 0;
    let mut code = vec![0x8e, 0x40, 0];
    call(&mut code, symbol("TOKENISE"));
    memory.execute_with_budget(&mut cpu, &code, 4_194_304);
    assert_ne!(cpu.registers.cc & C, 0);
    assert!(memory.bytes[0x1da..0x2dd].iter().all(|&b| b == 0xa5));
    assert!(memory.bytes[0x3d9..0x400].iter().all(|&b| b == 0x5a));
}

fn set_word(memory: &mut Memory, address: usize, value: u16) {
    memory.bytes[address..address + 2].copy_from_slice(&value.to_be_bytes());
}
fn word(memory: &Memory, address: usize) -> u16 {
    u16::from_be_bytes([memory.bytes[address], memory.bytes[address + 1]])
}
fn empty_programme(memory: &mut Memory) {
    set_word(memory, 0x19, 0x1e01);
    set_word(memory, 0x1b, 0x1e03);
    set_word(memory, 0x1d, 0x1e03);
    set_word(memory, 0x1f, 0x1e03);
    set_word(memory, 0x21, 0x7f36);
    memory.bytes[0x1e01..0x1e03].fill(0);
}
fn store_line(memory: &mut Memory, cpu: &mut Cpu, number: u16, text: &str) {
    let tokens = guest_tokens(memory, cpu, text);
    let end = 0x2dd + tokens.len() + 1;
    let mut code = vec![0xcc];
    code.extend(number.to_be_bytes());
    code.push(0x10);
    code.push(0x8e);
    code.extend(u16::try_from(end).unwrap().to_be_bytes());
    call(&mut code, symbol("STORE_LINE"));
    memory.execute_with_budget(cpu, &code, 4_194_304);
    assert_eq!(cpu.registers.cc & C, 0, "store line {number}");
}
#[test]
fn basic_rom_programme_insert_replace_delete_links_match_ide_files() {
    use eightbit_dragon_basic_core::{Dialect, analyse, tokenise};
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    let dialect = Dialect::dragon32().unwrap();
    for (number, body, expected_source) in [
        (30, "END", "30 END\n"),
        (10, "PRINT \"HELLO\"", "10 PRINT \"HELLO\"\n30 END\n"),
        (20, "GOTO30", "10 PRINT \"HELLO\"\n20 GOTO30\n30 END\n"),
        (
            10,
            "REM REPLACED WITH LONGER CONTENT",
            "10 REM REPLACED WITH LONGER CONTENT\n20 GOTO30\n30 END\n",
        ),
        (10, "CLS", "10 CLS\n20 GOTO30\n30 END\n"),
        (20, "", "10 CLS\n30 END\n"),
        (5, "", "10 CLS\n30 END\n"),
        (30, "", "10 CLS\n"),
        (10, "", ""),
    ] {
        store_line(&mut memory, &mut cpu, number, body);
        let expected = tokenise(&analyse(expected_source, &dialect).program, &dialect).unwrap();
        assert_eq!(
            &memory.bytes[0x1e01..usize::from(word(&memory, 0x1b))],
            &expected.bytes,
            "after editing {number}"
        );
        assert_eq!(word(&memory, 0x1b), word(&memory, 0x1d));
        assert_eq!(word(&memory, 0x1b), word(&memory, 0x1f));
    }
}
#[test]
fn basic_rom_failed_storage_preserves_existing_programme() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    store_line(&mut memory, &mut cpu, 10, "END");
    for (number, corrupt) in [(64000_u16, false), (65535, false), (20, true)] {
        let before = memory.bytes[0x1e01..0x1e20].to_vec();
        if corrupt {
            set_word(&mut memory, 0x1e01, 0x1e01);
        }
        let protected = memory.bytes[0x1e01..0x1e20].to_vec();
        let tokens = guest_tokens(&mut memory, &mut cpu, "PRINT 1");
        let mut code = vec![0xcc];
        code.extend(number.to_be_bytes());
        code.extend([0x10, 0x8e]);
        code.extend(
            u16::try_from(0x2dd + tokens.len() + 1)
                .unwrap()
                .to_be_bytes(),
        );
        call(&mut code, symbol("STORE_LINE"));
        memory.execute(&mut cpu, &code);
        assert_ne!(cpu.registers.cc & C, 0);
        assert_eq!(&memory.bytes[0x1e01..0x1e20], &protected);
        memory.bytes[0x1e01..0x1e20].copy_from_slice(&before);
    }
}

#[test]
fn basic_rom_shared_programme_walker_checks_without_writing_and_repairs_only_links() {
    let mut valid = synthetic_tokenised_lines(&[(0, b"FIRST"), (63999, b"LAST")]);
    valid[..2].copy_from_slice(&0x1e0b_u16.to_be_bytes());
    valid[10..12].copy_from_slice(&0x1e14_u16.to_be_bytes());
    for repair in [false, true] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        let mut input = valid.clone();
        let second = 5 + b"FIRST".len();
        // The cassette supplies stale links. No input link is followed as code or data.
        input[..2].copy_from_slice(&[0xff, 0xff]);
        input[second..second + 2].copy_from_slice(&[0, 1]);
        memory.bytes[0x1e01..0x1e01 + input.len()].copy_from_slice(&input);
        let terminal = 0x1e01 + u16::try_from(input.len()).unwrap() - 2;
        set_word(&mut memory, usize::from(symbol("REC_LIMIT")), terminal);
        cpu.registers.u = 0x6789;
        let mut code = vec![0xc6, u8::from(repair), 0x8e, 0x1e, 1];
        call(&mut code, symbol("PROGRAM_WALK"));
        memory.execute(&mut cpu, &code);
        assert_eq!(cpu.registers.u, 0x6789);
        assert_eq!(cpu.registers.s, 0x7000);
        assert_eq!(cpu.registers.cc & C == 0, repair);
        let expected = if repair { &valid } else { &input };
        assert_eq!(&memory.bytes[0x1e01..0x1e01 + input.len()], expected);
    }
    for body in [
        synthetic_tokenised_lines(&[(10, b"A"), (10, b"B")]),
        synthetic_tokenised_lines(&[(20, b"A"), (10, b"B")]),
        synthetic_tokenised_lines(&[(64000, b"A")]),
        vec![0, 0, 0, 0],
    ] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        memory.bytes[0x1e01..0x1e01 + body.len()].copy_from_slice(&body);
        set_word(
            &mut memory,
            0x1b,
            0x1e01 + u16::try_from(body.len()).unwrap(),
        );
        let mut code = Vec::new();
        call(&mut code, symbol("VALIDATE_PROGRAM"));
        memory.execute(&mut cpu, &code);
        assert_ne!(cpu.registers.cc & C, 0);
        assert_eq!(&memory.bytes[0x1e01..0x1e01 + body.len()], &body);
    }
}

#[test]
fn basic_rom_multi_argument_strings_and_nested_collection() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    for (source, expected) in [
        (
            "PRINT LEFT$(\"HELLO\",2);RIGHT$(\"HELLO\",2);MID$(\"HELLO\",2,3)",
            "HELOELL\r",
        ),
        (
            "PRINT LEFT$(\"HELLO\",0);RIGHT$(\"HELLO\",50);MID$(\"HELLO\",8)",
            "HELLO\r",
        ),
        (
            "PRINT MID$(\"HELLO\",2);STRING$(3,65);STRING$(2,\"XY\")",
            "ELLOAAAXX\r",
        ),
        (
            "PRINT INSTR(\"HELLO\",\"LL\");INSTR(3,\"HELLO\",\"L\");INSTR(8,\"HELLO\",\"\")",
            " 3  3  0 \r",
        ),
        (
            "PRINT INSTR(\"HELLO\",\"\");INSTR(\"\",\"\");INSTR(\"ABC\",\"BCD\")",
            " 1  1  0 \r",
        ),
    ] {
        memory.output.clear();
        assert!(execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(memory.output, expected.as_bytes(), "{source}");
    }
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "A$=\"HELLO\":B$=\"WORLD\""
    ));
    for i in 0..80 {
        assert!(
            execute_direct(
                &mut memory,
                &mut cpu,
                "C$=MID$(A$+B$,2,LEN(LEFT$(A$,3)))+RIGHT$(B$,3)"
            ),
            "allocation {i}"
        );
    }
    memory.output.clear();
    assert!(execute_direct(&mut memory, &mut cpu, "PRINT A$;B$;C$"));
    assert_eq!(memory.output, b"HELLOWORLDELLRLD\r");
    for source in [
        "PRINT MID$(A$,0)",
        "PRINT LEFT$(A$,-1)",
        "PRINT STRING$(256,65)",
        "PRINT INSTR(0,A$,B$)",
        "PRINT INSTR(A$,2)",
        "PRINT STRING$(3,\"\")",
        "PRINT RIGHT$(A$,1,2)",
    ] {
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(word(&memory, usize::from(symbol("STR_ROOTS"))), 0);
    }
}

#[test]
fn basic_rom_mid_assignment_matches_selected_lawful_string_observations() {
    for (assignment, expected) in [
        ("MID$(A$,2,2)=\"ABC\"", "HABLOHELLO\r"),
        ("MID$(A$,2)=\"ABC\"", "HABCOHELLO\r"),
        ("LET MID$(A$,5)=\"ABC\"", "HELLAHELLO\r"),
        ("MID$(A$,2)=\"\"", "HELLOHELLO\r"),
        ("MID$(A$,2)=A$", "HHHHHHELLO\r"),
        ("MID$(A$,2,0)=\"ABC\"", "HABCHHELLO\r"),
    ] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        assert!(execute_direct(&mut memory, &mut cpu, "A$=\"HELLO\":B$=A$"));
        assert!(
            execute_direct(&mut memory, &mut cpu, assignment),
            "{assignment}"
        );
        assert_eq!(&memory.bytes[0x8000..0xc000], ROM.as_slice());
        assert!(execute_direct(&mut memory, &mut cpu, "PRINT A$;B$"));
        assert_eq!(memory.output, expected.as_bytes(), "{assignment}");
    }
}

#[test]
fn basic_rom_mid_assignment_tracks_array_relocation_and_collects_live_strings() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "DIM A$(1):A$(0)=\"HELLO\""
    ));
    for _ in 0..100 {
        assert!(execute_direct(
            &mut memory,
            &mut cpu,
            "MID$(A$(0),1+UN)=LEFT$(\"XYZ\",1+QX)"
        ));
    }
    assert!(execute_direct(&mut memory, &mut cpu, "PRINT A$(0)"));
    assert_eq!(memory.output, b"XELLO\r");
    assert_eq!(word(&memory, usize::from(symbol("STR_ROOTS"))), 0);
    assert_eq!(numeric_scalar(&memory, [b'U', b'N']), packed_integer(0));
    assert_eq!(numeric_scalar(&memory, [b'Q', b'X']), packed_integer(0));
}

#[test]
fn basic_rom_mid_zero_length_keeps_the_observed_bounded_256_byte_copy() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "CLEAR300,16000:A$=\"HELLO\""
    ));
    let descriptor = numeric_scalar(&memory, [b'A', 128]);
    let pointer = usize::from(u16::from_be_bytes([descriptor[2], descriptor[3]]));
    memory.bytes[pointer + 5..pointer + 271].fill(85);
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "MID$(A$,2,0)=\"ABC\""
    ));
    for offset in 1..=256 {
        assert_eq!(memory.bytes[pointer + offset], b"HABC"[offset % 4]);
    }
    assert_eq!(memory.bytes[pointer + 257], 85);
    assert_eq!(memory.bytes[pointer], b'H');
}

#[test]
fn basic_rom_mid_assignment_keeps_programme_literals_and_recovers_after_errors() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    store_line(
        &mut memory,
        &mut cpu,
        10,
        "A$=\"HELLO\":B$=A$:MID$(A$,2,2)=\"AB\":PRINT A$;B$",
    );
    let programme = memory.bytes[0x1e01..usize::from(word(&memory, 0x1b))].to_vec();
    for _ in 0..2 {
        memory.output.clear();
        assert!(execute_direct(&mut memory, &mut cpu, "RUN"));
        assert_eq!(memory.output, b"HABLOHELLO\r");
        assert_eq!(&memory.bytes[0x1e01..0x1e01 + programme.len()], &programme);
    }
    for source in [
        "MID$(A$,0)=\"X\"",
        "MID$(A$,6)=\"X\"",
        "MID$(A$,2,256)=\"X\"",
        "MID$(A$,2) = 1",
        "MID$(A$,2,)=\"X\"",
        "MID$(A$,2)=\"X\" 9",
        "MID$(1,2)=\"X\"",
        "MID$(A$,2)\"X\"",
        "MID$(A,1)=\"X\"",
        "LEFT$(A$,1)=\"X\"",
    ] {
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(word(&memory, usize::from(symbol("STR_ROOTS"))), 0);
        assert!(execute_direct(&mut memory, &mut cpu, "Z=7"));
        assert_eq!(numeric_scalar(&memory, [b'Z', 0]), packed_integer(7));
    }
}

#[test]
fn basic_rom_mid_assignment_allocation_failure_preserves_the_programme_value() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    store_line(&mut memory, &mut cpu, 10, "A$=\"HELLO\":END");
    assert!(execute_direct(&mut memory, &mut cpu, "RUN"));
    let table = usize::from(word(&memory, 0x1b));
    let original = memory.bytes[table..table + 7].to_vec();
    let limit = word(&memory, 0x21);
    set_word(&mut memory, 0x27, limit);
    set_word(&mut memory, 0x23, limit);
    assert!(!execute_direct(&mut memory, &mut cpu, "MID$(A$,2)=\"X\""));
    assert_eq!(&memory.bytes[table..table + 7], &original);
    assert_eq!(word(&memory, usize::from(symbol("STR_ROOTS"))), 0);
    set_word(&mut memory, 0x27, 0x7ffe);
    set_word(&mut memory, 0x23, 0x7ffe);
    assert!(execute_direct(&mut memory, &mut cpu, "PRINT A$"));
    assert_eq!(memory.output, b"HELLO\r");
}

#[test]
fn basic_rom_compact_arithmetic_skips_remaining_operations_after_a_failure() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    unpack_to(&mut memory, 0x4f, packed_number(1.0));
    unpack_to(&mut memory, 0x5c, packed_number(0.0));
    cpu.registers.x = 0x3456;
    cpu.registers.u = 0x5678;
    let mut code = vec![0x32, 0x78]; // LEAS -8,S: the original numeric frame.
    call(&mut code, symbol("FP_RUN"));
    code.extend([
        u8::try_from(symbol("FP_DIV")).unwrap(),
        u8::try_from(symbol("FP_KLOAD")).unwrap() + 2,
        u8::try_from(symbol("FP_EXP")).unwrap(),
        u8::try_from(symbol("FP_COMPARE")).unwrap(),
        u8::try_from(symbol("FP_STORE")).unwrap(),
        0,
    ]);
    code.extend([0x32, 0x68]); // LEAS 8,S preserves failure C.
    memory.execute(&mut cpu, &code);
    assert_ne!(cpu.registers.cc & C, 0);
    assert_eq!(cpu.registers.s, 0x7000);
    assert_eq!(cpu.registers.x, 0x3456);
    assert_eq!(cpu.registers.u, 0x5678);
    assert_eq!(unpacked_number(&memory).to_bits(), 1.0_f64.to_bits());
}

#[test]
fn basic_rom_native_collector_visits_register_roots_without_descriptor_copies() {
    for (active, argument, overlap) in [
        (false, true, false),
        (true, false, false),
        (true, true, false),
        (true, true, true),
    ] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        set_word(&mut memory, 0x27, 0x7ffe);
        set_word(&mut memory, 0x23, 0x7f50);
        memory.bytes[0x06] = 0xff;
        memory.bytes[0x4f] = 4;
        set_word(&mut memory, 0x52, 0x7f60);
        memory.bytes[0x5c] = 3;
        set_word(&mut memory, 0x5f, if overlap { 0x7f61 } else { 0x7f70 });
        memory.bytes[usize::from(symbol("STR_BOTH"))] = u8::from(argument);
        memory.bytes[0x7f60..0x7f64].copy_from_slice(b"ABCD");
        memory.bytes[0x7f70..0x7f73].copy_from_slice(b"XYZ");
        memory.bytes[0xd8..0xea].fill(0xa5);
        cpu.registers.set_d(0x1a2b);
        cpu.registers.x = 0x2345;
        cpu.registers.y = 0x3456;
        cpu.registers.u = 0x4567;
        let mut code = Vec::new();
        call(
            &mut code,
            symbol(if active {
                "STRING_COLLECT_ACTIVE"
            } else {
                "STRING_COLLECT"
            }),
        );
        memory.execute(&mut cpu, &code);
        assert_eq!(cpu.registers.cc & C, 0);
        assert_eq!(cpu.registers.d(), 0x1a2b);
        assert_eq!(
            (cpu.registers.x, cpu.registers.y, cpu.registers.u),
            (0x2345, 0x3456, 0x4567)
        );
        assert_eq!(cpu.registers.s, 0x7000);
        assert_eq!(word(&memory, usize::from(symbol("STR_ROOTS"))), 0);
        assert_eq!(&memory.bytes[0xd8..0xea], &[0xa5; 18]);
        if active {
            let length = if argument && !overlap { 7 } else { 4 };
            assert_eq!(word(&memory, 0x23), 0x7ffe - length);
            let start = usize::from(word(&memory, 0x52));
            assert_eq!(&memory.bytes[start..start + 4], b"ABCD");
            if argument {
                let start = usize::from(word(&memory, 0x5f));
                assert_eq!(
                    &memory.bytes[start..start + 3],
                    if overlap { b"BCD" } else { b"XYZ" }
                );
            } else {
                assert_eq!(word(&memory, 0x5f), 0x7f70);
            }
        } else {
            assert_eq!(word(&memory, 0x23), 0x7ffe);
            assert_eq!(word(&memory, 0x52), 0x7f60);
            assert_eq!(word(&memory, 0x5f), 0x7f70);
        }
    }
}

#[test]
fn basic_rom_native_collector_validates_active_registers_before_any_move() {
    for corrupt_argument in [false, true] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        set_word(&mut memory, 0x27, 0x7ffe);
        set_word(&mut memory, 0x23, 0x7f50);
        memory.bytes[0x06] = 0xff;
        memory.bytes[0x4f] = 4;
        memory.bytes[0x5c] = 3;
        set_word(
            &mut memory,
            0x52,
            if corrupt_argument { 0x7f60 } else { 0xffff },
        );
        set_word(
            &mut memory,
            0x5f,
            if corrupt_argument { 0x7ffe } else { 0x7f70 },
        );
        memory.bytes[usize::from(symbol("STR_BOTH"))] = 1;
        memory.bytes[0x7f50..0x7fff].fill(0xa5);
        let before = memory.bytes[0x7f50..0x7fff].to_vec();
        let pointers = (word(&memory, 0x52), word(&memory, 0x5f));
        let mut code = Vec::new();
        call(&mut code, symbol("STRING_COLLECT_ACTIVE"));
        memory.execute(&mut cpu, &code);
        assert_ne!(cpu.registers.cc & C, 0);
        assert_eq!(cpu.registers.s, 0x7000);
        assert_eq!(word(&memory, 0x23), 0x7f50);
        assert_eq!((word(&memory, 0x52), word(&memory, 0x5f)), pointers);
        assert_eq!(word(&memory, usize::from(symbol("STR_ROOTS"))), 0);
        assert_eq!(memory.bytes[0x7f50..0x7fff], before);
    }
}

#[test]
fn basic_rom_collector_moves_overlapping_roots_as_one_interval() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "A$=\"HELLOWORLD\":B$=\"!\":C$=\"?\""
    ));
    let table = usize::from(word(&memory, 0x1b));
    let original = word(&memory, table + 4);
    // Original RAM-only aliases model three partially overlapping live substrings.
    memory.bytes[table + 9] = 6;
    set_word(&mut memory, table + 11, original + 2);
    memory.bytes[table + 16] = 7;
    set_word(&mut memory, table + 18, original + 1);
    let mut code = Vec::new();
    call(&mut code, symbol("STRING_COLLECT"));
    memory.execute(&mut cpu, &code);
    assert_eq!(cpu.registers.cc & C, 0);
    assert_eq!(word(&memory, 0x23), word(&memory, 0x27) - 10);
    assert!(execute_direct(&mut memory, &mut cpu, "PRINT A$;B$;C$"));
    assert_eq!(memory.output, b"HELLOWORLDLLOWORELLOWOR\r");
}

#[test]
fn basic_rom_value_functions_and_radix_literals_preserve_guest_cursor() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    // This flat-memory probe supplies a no-key result; matrix input has separate board tests.
    let key = usize::from(symbol("KEYIN"));
    memory.bytes[key..key + 2].copy_from_slice(&[0x4f, 0x39]);
    set_word(&mut memory, 0xb0, 0x134);
    set_word(&mut memory, 0x134, symbol("CMD_UNSUPPORTED"));
    for (source, expected) in [
        (
            "PRINT HEX$(0);HEX$(15);HEX$(4660);HEX$(65535)",
            "0F1234FFFF\r",
        ),
        ("PRINT HEX$(32768);HEX$(65535.9)", "8000FFFF\r"),
        ("PRINT STR$(12);STR$(-1.5);STR$(0)", " 12-1.5 0\r"),
        (
            "PRINT &H7FFF;&H8000;&HFFFF;&O177777",
            " 32767  32768  65535  65535 \r",
        ),
        (
            "A=1:PRINT VARPTR(A)=7685;LEN(INKEY$);EOF(-1)",
            "-1  0  0 \r",
        ),
    ] {
        memory.output.clear();
        assert!(execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(memory.output, expected.as_bytes(), "{source}");
        assert_eq!(memory.bytes[usize::from(symbol("FORMAT_BUFFER"))], 0);
    }
    for source in [
        "PRINT HEX$(-1)",
        "PRINT HEX$(-32768)",
        "PRINT HEX$(-32769)",
        "PRINT HEX$(65536)",
        "PRINT &H10000",
        "PRINT &O200000",
        "PRINT VARPTR(1)",
        "PRINT USR0(1)",
    ] {
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
    }
    // Original guest routine receives X=$4F and uses the public conversions.
    memory.bytes[0x6000..0x600b]
        .copy_from_slice(&[0xbd, 0x8b, 0x2d, 0xc3, 0, 1, 0xbd, 0x8c, 0x37, 0x39, 0x12]);
    set_word(&mut memory, 0xb0, 0x134);
    set_word(&mut memory, 0x134, 0x6000);
    memory.output.clear();
    assert!(execute_direct(&mut memory, &mut cpu, "PRINT USR(41)+1"));
    assert_eq!(memory.output, b" 43 \r");
}

#[test]
fn basic_rom_radix_literals_and_val_match_lawful_unsigned_spacing_and_error_cases() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    for (source, expected) in [
        ("PRINT &177777;-&H1;&H 10;&H1 0", " 65535 -1  16  16 \r"),
        ("PRINT &H;&O;&;&8", " 0  0  0  8 \r"),
        (
            "PRINT VAL(\"&HFFFF\");VAL(\"&177777\");VAL(\"&O177777\")",
            " 65535  65535  65535 \r",
        ),
        (
            "PRINT VAL(\"-&H1\");VAL(\"  +&H10\");VAL(\"&H10REST\")",
            " 0  0  16 \r",
        ),
        (
            "PRINT VAL(\"&H\");VAL(\"&O\");VAL(\"&\");VAL(\"&Q1\");VAL(\"&8\")",
            " 0  0  0  0  8 \r",
        ),
        (
            "PRINT VAL(\"& 177\");VAL(\"&H 10\");VAL(\"&H1 0\");VAL(\"&HFG\")",
            " 127  16  16  15 \r",
        ),
    ] {
        memory.output.clear();
        assert!(execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(memory.output, expected.as_bytes(), "{source}");
    }
    for expression in [
        "&H10000",
        "&O200000",
        "&19",
        "&O9",
        "&09",
        "&89",
        "VAL(\"&H10000\")",
        "VAL(\"&19\")",
        "VAL(\"&O9\")",
        "VAL(\"&09\")",
        "VAL(\"&89\")",
    ] {
        assert!(
            !execute_direct(&mut memory, &mut cpu, &format!("A={expression}")),
            "{expression}"
        );
        assert_eq!(word(&memory, usize::from(symbol("STR_ROOTS"))), 0);
        assert_eq!(memory.bytes[usize::from(symbol("EX_DEPTH"))], 0);
        assert!(execute_direct(&mut memory, &mut cpu, "A=VAL(\"&HFFFF\")"));
        assert_eq!(numeric_scalar(&memory, [b'A', 0]), packed_number(65535.0));
    }
}

#[test]
fn basic_rom_shared_decimal_digit_scanner_keeps_legacy_internal_spaces() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "A=1 2:B=1 . 5:C=1 E 2:D=1 E - 2:E=VAL(\"1 2\"):F=VAL(\"1 E - 2\")"
    ));
    for (name, expected) in [
        (b'A', [132, 64, 0, 0, 0]),
        (b'B', [129, 64, 0, 0, 0]),
        (b'C', [135, 72, 0, 0, 0]),
        (b'D', [122, 35, 215, 10, 62]),
        (b'E', [132, 64, 0, 0, 0]),
        (b'F', [122, 35, 215, 10, 62]),
    ] {
        assert_eq!(numeric_scalar(&memory, [name, 0]), expected);
    }
    store_line(&mut memory, &mut cpu, 10, "A=A+1:END");
    assert!(execute_direct(&mut memory, &mut cpu, "RUN 1 0"));
    assert_eq!(numeric_scalar(&memory, [b'A', 0]), packed_integer(1));
    for source in ["A=1 E +", "A=1 E -", "A=1 E 256"] {
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert!(execute_direct(&mut memory, &mut cpu, "A=1 2"));
    }
}

#[test]
fn basic_rom_graphics_modes_pixels_colours_and_workspace() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    let mut code = Vec::new();
    call(&mut code, symbol("GFX_INIT"));
    memory.execute(&mut cpu, &code);
    assert_eq!(&memory.bytes[0xb2..0xb4], &[3, 0]);
    assert_eq!(&memory.bytes[0xde..0xe3], &[2, 0xba, 0x42, 4, 2]);
    for (mode, stride, end, x, y, colour) in [
        (0, 16, 3072, 127, 95, 1),
        (1, 32, 4608, 127, 95, 4),
        (2, 16, 4608, 127, 191, 1),
        (3, 32, 7680, 127, 191, 4),
        (4, 32, 7680, 255, 191, 1),
    ] {
        let source = format!("PMODE {mode},1:PCLS 0:PSET(255,191):PRINT PPOINT(255,191)");
        memory.output.clear();
        assert!(execute_direct(&mut memory, &mut cpu, &source), "{source}");
        assert_eq!(
            memory.output,
            format!(" {colour} \r").as_bytes(),
            "{source}"
        );
        assert_eq!(word(&memory, 0xba), 1536);
        assert_eq!(word(&memory, 0xb7), end);
        assert_eq!(memory.bytes[0xb9], stride);
        assert_eq!(word(&memory, 0xbd), x);
        assert_eq!(word(&memory, 0xbf), y);
        assert_eq!(
            memory.bytes[usize::from(end - 1)],
            if colour == 4 { 255 } else { 1 }
        );
    }
    memory.output.clear();
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "PMODE 4,1:PCLS:PSET(0,0,1):PRINT PPOINT(0,0);:PSET(1,0,2):PRINT PPOINT(1,0);:COLOR 2,3:PSET(2,0):PRINT PPOINT(2,0)"
    ));
    assert_eq!(memory.output, b" 1  0  0 \r");
    assert_eq!(&memory.bytes[0xb2..0xb4], &[0, 3]);
    assert!(execute_direct(&mut memory, &mut cpu, "PMODE 0,2"));
    assert_eq!(word(&memory, 0xba), 3072);
    assert_eq!(word(&memory, 0xb7), 4608);
    for source in [
        "PMODE 5,1",
        "PMODE 4,8",
        "PSET(0,256)",
        "PSET(65536,1)",
        "COLOR 0",
        "PSET(0,0,9)",
    ] {
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
    }
}

#[test]
fn basic_rom_pclear_relocates_and_relinks_programmes_with_atomic_bounds() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    let mut code = Vec::new();
    call(&mut code, symbol("GFX_INIT"));
    memory.execute(&mut cpu, &code);
    store_line(&mut memory, &mut cpu, 10, "PRINT 17");
    store_line(&mut memory, &mut cpu, 20, "END");
    let length = usize::from(word(&memory, 0x1b) - word(&memory, 0x19));
    assert!(execute_direct(&mut memory, &mut cpu, "PCLEAR 2"));
    assert_eq!(word(&memory, 0x19), 4609);
    assert_eq!(usize::from(word(&memory, 0x1b)), 4609 + length);
    assert!(execute_direct(&mut memory, &mut cpu, "RUN"));
    assert_eq!(memory.output, b" 17 \r");
    assert!(execute_direct(&mut memory, &mut cpu, "PCLEAR 8"));
    assert_eq!(word(&memory, 0x19), 13825);
    memory.output.clear();
    assert!(execute_direct(&mut memory, &mut cpu, "RUN"));
    assert_eq!(memory.output, b" 17 \r");
    assert!(execute_direct(&mut memory, &mut cpu, "PMODE 4,1"));
    assert!(!execute_direct(&mut memory, &mut cpu, "PCLEAR 1"));
    assert_eq!(word(&memory, 0x19), 13825);
    let programme = memory.bytes[13825..13825 + length].to_vec();
    for source in ["PCLEAR 0", "PCLEAR 9", "PCLEAR -1", "PCLEAR 1,2"] {
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(word(&memory, 0x19), 13825);
        assert_eq!(&memory.bytes[13825..13825 + length], programme);
    }
}

#[test]
fn basic_rom_semigraphics_cells_and_colour_clearing() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    for (source, expected) in [
        ("CLS:PRINT POINT(0,0)", "-1 \r"),
        (
            "CLS 0:SET(0,0,5):SET(1,1,5):PRINT POINT(0,0);POINT(1,1);POINT(1,0)",
            " 5  5  0 \r",
        ),
        ("RESET(0,0):PRINT POINT(0,0);POINT(1,1)", " 0  5 \r"),
        ("CLS 8:PRINT POINT(63,31);POINT(0,0)", " 8  8 \r"),
    ] {
        memory.output.clear();
        assert!(execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(memory.output, expected.as_bytes(), "{source}");
    }
    for source in [
        "SET(64,0,1)",
        "RESET(0,32)",
        "PRINT POINT(0,32)",
        "CLS 9",
        "SET(0,0,0)",
    ] {
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
    }
}

#[test]
fn basic_rom_computed_branches_usr_definitions_and_trace() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    for (line, source) in [
        (10, "ON 2 GOTO 100,200"),
        (100, "PRINT 1:END"),
        (200, "ON 2 GOSUB 300,400:PRINT 3:END"),
        (300, "PRINT 8:RETURN"),
        (400, "PRINT 2:RETURN"),
    ] {
        store_line(&mut memory, &mut cpu, line, source);
    }
    assert!(execute_direct(&mut memory, &mut cpu, "RUN"));
    assert_eq!(memory.output, b" 2 \r 3 \r");
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "ON 0 GOTO 100:ON 5 GOSUB 100,200:PRINT 7"
    ));
    assert!(memory.output.ends_with(b" 7 \r"));
    for source in [
        "ON -1 GOTO 100",
        "ON 256 GOTO 100",
        "ON 1 GOTO 999",
        "ON 1 PRINT 7",
    ] {
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
    }
    set_word(&mut memory, 0xb0, 0x134);
    assert!(execute_direct(&mut memory, &mut cpu, "DEF USR3=24576"));
    assert_eq!(word(&memory, 0x13a), 0x6000);
    memory.bytes[0x6000..0x600a]
        .copy_from_slice(&[0xbd, 0x8b, 0x2d, 0xc3, 0, 1, 0xbd, 0x8c, 0x37, 0x39]);
    memory.output.clear();
    assert!(execute_direct(&mut memory, &mut cpu, "PRINT USR3(41)"));
    assert_eq!(memory.output, b" 42 \r");
    assert!(execute_direct(&mut memory, &mut cpu, "TRON"));
    assert_eq!(memory.bytes[0xaf], 1);
    assert!(execute_direct(&mut memory, &mut cpu, "TROFF"));
    assert_eq!(memory.bytes[0xaf], 0);
}

#[test]
fn basic_rom_original_random_generator_range_seed_and_recovery() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "A=RND(-123):B=RND(0):C=RND(-123):PRINT A=C"
    ));
    assert_eq!(memory.output, b"-1 \r");
    for _ in 0..100 {
        assert!(execute_direct(
            &mut memory,
            &mut cpu,
            "A=RND(0):B=RND(6):C=RND(1)"
        ));
        let table = usize::from(word(&memory, 0x1b));
        let decode = |at: usize| {
            let p = &memory.bytes[at..at + 5];
            if p[0] == 0 {
                return 0.0;
            }
            let m = u32::from_be_bytes([p[1] | 0x80, p[2], p[3], p[4]]);
            f64::from(m)
                * 2.0_f64.powi(i32::from(p[0]) - 160)
                * if p[1] & 128 == 0 { 1.0 } else { -1.0 }
        };
        let a = decode(table + 2);
        let b = decode(table + 9);
        let c = decode(table + 16);
        assert!((0.0..1.0).contains(&a));
        assert!((1.0..=6.0).contains(&b) && b.fract() == 0.0);
        assert_eq!(c.to_bits(), 1.0_f64.to_bits());
    }
    assert!(!execute_direct(&mut memory, &mut cpu, "A=RND(\"X\")"));
}

#[test]
fn basic_rom_random_truncates_positive_fractional_bounds_before_scaling() {
    // Lawful black-box observations: this seed advances to $C4CA17E0. Scaling
    // its fraction by 1.5 before truncation would incorrectly return two.
    for (bound, expected) in [(".5", 1), ("1.5", 1), ("1.9", 1), ("2.5", 2)] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        memory.bytes[0x116..0x11a].fill(0xff);
        assert!(execute_direct(
            &mut memory,
            &mut cpu,
            &format!("B=RND({bound}):Z=85")
        ));
        let table = usize::from(word(&memory, 0x1b));
        assert_eq!(
            &memory.bytes[table + 2..table + 7],
            &packed_integer(expected),
            "bound {bound}"
        );
        assert_eq!(&memory.bytes[table + 9..table + 14], &packed_integer(85));
        assert_eq!(&memory.bytes[0x116..0x11a], &0xc4ca_17e0_u32.to_be_bytes());
        assert_eq!(word(&memory, usize::from(symbol("STR_ROOTS"))), 0);
    }
}

#[test]
fn basic_rom_sound_writes_dac_tones_at_observed_pitch_law() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    for pitch in [1_u16, 89, 255] {
        memory.signals.clear();
        assert!(execute_direct(
            &mut memory,
            &mut cpu,
            &format!("SOUND {pitch},1")
        ));
        let samples: Vec<_> = memory.signals.iter().filter(|s| s.1 == 0xff20).collect();
        assert!(samples.len() >= 40);
        assert!(
            samples.iter().all(|s| s.2 & 2 == 2),
            "tone must preserve inactive printer strobe"
        );
        let mut intervals: Vec<_> = samples.windows(2).map(|p| p[1].0 - p[0].0).collect();
        intervals.sort_unstable();
        let quarter = intervals[intervals.len() / 2];
        assert_eq!(quarter, 1303 - u64::from(pitch) * 5, "pitch {pitch}");
        let elapsed = samples.last().unwrap().0 - samples.first().unwrap().0;
        assert!(
            (50_000..90_000).contains(&elapsed),
            "duration cycles {elapsed}"
        );
        assert_eq!(
            memory.bytes[0xff23] & 8,
            0,
            "tone disables the DAC gate on return"
        );
    }
    for source in [
        "SOUND 0,1",
        "SOUND 1,0",
        "SOUND 256,1",
        "SOUND 1,256",
        "SOUND 89,1,2",
    ] {
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
    }
}

#[test]
fn basic_rom_cassette_byte_output_periods_and_lsb_order() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    for byte in [0_u8, 255, 85, 129] {
        memory.signals.clear();
        let mut code = vec![0x86, byte];
        call(&mut code, 0x801e);
        memory.execute(&mut cpu, &code);
        let rising: Vec<_> = memory
            .signals
            .iter()
            .filter(|s| s.1 == 0xff20 && s.2 >> 2 >= 32)
            .map(|s| s.0)
            .collect();
        assert_eq!(rising.len(), 8);
        let edges: Vec<_> = memory.signals.iter().filter(|s| s.1 == 0xff20).collect();
        assert_eq!(edges.len(), 16);
        for (bit, edge) in edges.chunks_exact(2).enumerate() {
            let expected_half = if byte & (1 << bit) != 0 { 218_u64 } else { 407 };
            assert!(
                (edge[1].0 - edge[0].0).abs_diff(expected_half) <= 8,
                "bit {bit} byte {byte}: unbalanced high half"
            );
        }

        let periods: Vec<_> = rising.windows(2).map(|p| p[1] - p[0]).collect();
        for (bit, period) in periods.iter().enumerate() {
            let expected = if byte & (1 << bit) != 0 { 435_u64 } else { 813 };
            assert!(
                period.abs_diff(expected) <= 4,
                "bit {bit} byte {byte}: {period} E cycles, expected {expected}"
            );
        }
    }
}

fn synthetic_tape(memory: &mut Memory, bytes: &[u8]) {
    memory.input_signal = vec![(memory.cycles, false)];
    memory.input_cursor = 0;
    let mut time = memory.cycles + 512;
    for &byte in bytes {
        for bit in 0..8 {
            let period = if byte & (1 << bit) == 0 { 813 } else { 435 };
            memory.input_signal.push((time, true));
            memory.input_signal.push((time + period / 2, false));
            time += period;
        }
    }
}
#[test]
fn basic_rom_cassette_byte_input_samples_edges_in_lsb_order_and_times_out() {
    for byte in [0_u8, 255, 85, 129] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        synthetic_tape(&mut memory, &[byte, 0x55]);
        let mut code = Vec::new();
        call(&mut code, 0x8021);
        call(&mut code, 0x8024);
        code.extend([0xb7, 0x63, 0]);
        memory.execute_with_budget(&mut cpu, &code, 1_048_576);
        assert_eq!(cpu.registers.cc & C, 0, "byte {byte}");
        assert_eq!(memory.bytes[0x81], 0);
        assert_eq!(memory.bytes[0x6300], byte);
    }
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    let mut code = Vec::new();
    call(&mut code, 0x8021);
    memory.execute_with_budget(&mut cpu, &code, 1_048_576);
    assert_ne!(cpu.registers.cc & C, 0);
    assert_eq!(memory.bytes[0x81], 3);
}

#[test]
fn basic_rom_cassette_blocks_check_checksum_length_and_ram_boundary() {
    for (checksum, destination, status) in
        [(17_u8, 0x6000_u16, 0), (18, 0x6000, 1), (17, 0x7fff, 2)]
    {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        let mut tape = vec![0x55; 128];
        tape.extend([0x55, 0x3c, 1, 3, 2, 4, 7, checksum, 0x55, 0x55]);
        synthetic_tape(&mut memory, &tape);
        set_word(&mut memory, 0x7e, destination);
        memory.bytes[0x6000..0x6003].fill(0x77);
        let mut code = Vec::new();
        call(&mut code, 0x8021);
        call(&mut code, 0xb93e);
        memory.execute_with_budget(&mut cpu, &code, 2_097_152);
        assert_eq!(memory.bytes[0x81], status);
        assert_eq!(cpu.registers.cc & C == 0, status == 0);
        assert_eq!(cpu.registers.cc & eightbit_m6809_core::Z != 0, status == 0);
        if status == 2 {
            assert_eq!(&memory.bytes[0x6000..0x6003], &[0x77; 3]);
        } else {
            assert_eq!(&memory.bytes[0x6000..0x6003], &[2, 4, 7]);
            assert_eq!(memory.bytes[0x7c], 1);
            assert_eq!(memory.bytes[0x7d], 3);
        }
    }
}

#[test]
fn basic_rom_lines_cover_all_directions_and_preserve_endpoint_cursor() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    let mut init = Vec::new();
    call(&mut init, symbol("GFX_INIT"));
    memory.execute(&mut cpu, &init);
    for (x, y) in [
        (0_u16, 0_u16),
        (128, 0),
        (255, 0),
        (0, 96),
        (255, 96),
        (0, 191),
        (128, 191),
        (255, 191),
        (128, 96),
    ] {
        let source = format!("PMODE 4,1:PCLS 0:LINE(128,96)-({x},{y}),PSET");
        assert!(execute_direct(&mut memory, &mut cpu, &source), "{source}");
        assert_eq!(word(&memory, 0xc7), x);
        assert_eq!(word(&memory, 0xc9), y);
        let count: u32 = memory.bytes[0x600..0x1e00]
            .iter()
            .map(|v| v.count_ones())
            .sum();
        assert_eq!(
            count,
            u32::from(x.abs_diff(128).max(y.abs_diff(96))) + 1,
            "{source}"
        );
        for (px, py) in [(128, 96), (x, y)] {
            assert_ne!(
                memory.bytes[0x600 + usize::from(py) * 32 + usize::from(px) / 8]
                    & (0x80 >> (px % 8)),
                0
            );
        }
    }
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "PCLS 0:LINE(16,16)-(24,20),PSET:LINE-(32,20),PSET"
    ));
    for (x, y) in [
        (16, 16),
        (17, 17),
        (18, 17),
        (19, 18),
        (20, 18),
        (21, 19),
        (22, 19),
        (23, 20),
        (24, 20),
        (32, 20),
    ] {
        assert_ne!(
            memory.bytes[0x600 + y * 32 + x / 8] & (0x80 >> (x % 8)),
            0,
            "pixel {x},{y}"
        );
    }
    let before = memory.bytes[0x600..0x1e00].to_vec();
    for source in [
        "LINE(0,0)-(1,256),PSET",
        "LINE(0,0)-(1,1),XYZ",
        "LINE(0,0)-(1,1),PSET,UNKNOWN",
    ] {
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(memory.bytes[0x600..0x1e00], before);
    }
}

#[test]
fn basic_rom_rectangle_outlines_and_fills_cover_reversed_and_degenerate_corners() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    let mut init = Vec::new();
    call(&mut init, symbol("GFX_INIT"));
    memory.execute(&mut cpu, &init);
    for (x1, y1, x2, y2) in [
        (16_u16, 20_u16, 32_u16, 30_u16),
        (32, 30, 16, 20),
        (32, 20, 16, 30),
        (16, 30, 32, 20),
        (16, 20, 16, 30),
        (16, 20, 32, 20),
        (16, 20, 16, 20),
    ] {
        for fill in [false, true] {
            let suffix = if fill { "BF" } else { "B" };
            let source = format!("PMODE 4,1:PCLS 0:LINE({x1},{y1})-({x2},{y2}),PSET,{suffix}");
            assert!(execute_direct(&mut memory, &mut cpu, &source), "{source}");
            let width = x1.abs_diff(x2) + 1;
            let height = y1.abs_diff(y2) + 1;
            let expected = if fill || width == 1 || height == 1 {
                u32::from(width) * u32::from(height)
            } else {
                u32::from(width) * 2 + u32::from(height) * 2 - 4
            };
            let count: u32 = memory.bytes[0x600..0x1e00]
                .iter()
                .map(|v| v.count_ones())
                .sum();
            assert_eq!(count, expected, "{source}");
            assert_eq!(word(&memory, 0xc7), x2);
            assert_eq!(word(&memory, 0xc9), y2);
        }
    }
}

#[test]
fn basic_rom_fn_records_and_lexical_parameter_restoration_match_observations() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    for (line, source) in [
        (10, "DEF FN F(X)=X*X+1"),
        (20, "Z=FN F(3)"),
        (30, "PRINT Z"),
    ] {
        store_line(&mut memory, &mut cpu, line, source);
    }
    assert!(execute_direct(&mut memory, &mut cpu, "RUN"));
    assert_eq!(memory.output, b" 10 \r");
    let table = usize::from(word(&memory, 0x1b));
    assert_eq!(&memory.bytes[table..table + 2], &[b'F' | 128, 0]);
    assert_eq!(word(&memory, table + 2), 0x1e0e);
    assert_eq!(word(&memory, table + 4), u16::try_from(table + 9).unwrap());
    assert_eq!(memory.bytes[table + 6], 0);
    assert_eq!(
        &memory.bytes[table + 7..table + 14],
        &[b'X', 0, 0, 0, 0, 0, 0]
    );
    assert!(execute_direct(&mut memory, &mut cpu, "NEW"));
    for (line, source) in [
        (10, "DEF FN F(X)=X*X+1"),
        (20, "DEF FN G(Y)=FN F(Y)+FN F(Y+1)"),
        (30, "X=7:Y=8:Z=FN G(3):PRINT Z;X;Y"),
    ] {
        store_line(&mut memory, &mut cpu, line, source);
    }
    memory.output.clear();
    assert!(execute_direct(&mut memory, &mut cpu, "RUN"));
    assert_eq!(memory.output, b" 27  7  8 \r");
    assert!(!execute_direct(&mut memory, &mut cpu, "DEF FN Q(X)=X"));
    assert!(!execute_direct(&mut memory, &mut cpu, "PRINT FN Q(1)"));
}

#[test]
fn basic_rom_fn_failure_and_recursion_restore_parameters_stack_and_roots() {
    for expression in ["1/(X-1)", "FN F(X)", "\"BAD\""] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        store_line(
            &mut memory,
            &mut cpu,
            10,
            &format!("DEF FN F(X)={expression}"),
        );
        store_line(&mut memory, &mut cpu, 20, "X=9:Z=FN F(1)");
        assert!(
            !execute_direct(&mut memory, &mut cpu, "RUN"),
            "{expression}"
        );
        assert_eq!(word(&memory, usize::from(symbol("STR_ROOTS"))), 0);
        assert_eq!(memory.bytes[usize::from(symbol("EX_DEPTH"))], 0);
        assert!(execute_direct(&mut memory, &mut cpu, "PRINT X"));
        assert_eq!(memory.output, b" 9 \r", "{expression}");
    }
}

#[test]
fn basic_rom_random_seed_layout_and_recurrence_match_lawful_observations() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    let mut init = Vec::new();
    call(&mut init, symbol("RND_INIT"));
    memory.execute(&mut cpu, &init);
    assert_eq!(&memory.bytes[0x115..0x11a], &[128, 79, 199, 82, 89]);
    for (state, product) in [
        (0x7f8f_2bfe_u32, 0x79de_c673_u32),
        (0x50c2_2e35, 0x4b11_c8aa),
        (0x22e9_33f2, 0x1d38_ce67),
        (0xa781_e231, 0xa1d1_7ca6),
        (0x7281_3946, 0x6cd0_d3bb),
        (0x551f_b54d, 0x4f6f_4fc2),
    ] {
        assert!(execute_direct(&mut memory, &mut cpu, "A=RND(0)"));
        assert_eq!(memory.bytes[0x115], 128);
        assert_eq!(&memory.bytes[0x116..0x11a], &state.to_be_bytes());
        assert_eq!(&memory.bytes[0xab..0xaf], &product.to_be_bytes());
    }
    assert!(execute_direct(&mut memory, &mut cpu, "A=RND(-1)"));
    assert_eq!(&memory.bytes[0x116..0x11a], &0x85b0_658b_u32.to_be_bytes());
    let table = usize::from(word(&memory, 0x1b));
    assert_eq!(
        &memory.bytes[table + 2..table + 7],
        &[128, 5, 176, 101, 139]
    );
}

#[test]
fn basic_rom_random_fraction_guard_and_halfway_rounding_match_lawful_observations() {
    // Original synthetic POKEs/RND programmes on the approved exact-profile ROM.
    // These are numeric interface facts; no reference instructions were read.
    for (seed, expected) in [
        (0x85b0_658b_u32, [127, 55, 226, 16, 201]),
        (0x4fc7_5259_u32, [127, 127, 30, 87, 253]),
        (0x50c2_2e35_u32, [126, 11, 164, 207, 201]),
        (0x22e9_33f2_u32, [128, 39, 129, 226, 50]),
        (0xc9a1_42db_u32, [123, 72, 189, 154, 151]),
        (0x8bd3_78c6_u32, [124, 18, 153, 252, 219]),
        (0x72cc_784b_u32, [123, 122, 169, 212, 141]),
        (0x6c72_585f_u32, [94, 20, 0, 0, 0]),
        (0x418c_9d62_u32, [97, 15, 128, 0, 0]),
        (0x16a6_e265_u32, [98, 6, 64, 0, 0]),
        (0xebc1_2768_u32, [98, 88, 0, 0, 0]),
        (0x402a_3b74_u32, [99, 105, 0, 0, 0]),
        (0xb19d_165c_u32, [104, 127, 14, 0, 0]),
        (0x86b7_5b5f_u32, [105, 0, 4, 0, 0]),
        (0xdc5b_135c_u32, [112, 127, 255, 1, 0]),
        (0xb175_585f_u32, [113, 0, 0, 125, 128]),
    ] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        memory.bytes[0x116..0x11a].copy_from_slice(&seed.to_be_bytes());
        let mut code = Vec::new();
        call(&mut code, symbol("FAC_ZERO"));
        call(&mut code, symbol("VALUE_RND"));
        code.extend([0x8e, 0x60, 0]);
        call(&mut code, symbol("FAC_STORE"));
        memory.execute(&mut cpu, &code);
        assert_eq!(&memory.bytes[0x6000..0x6005], &expected, "seed {seed:08x}");
        assert_eq!(cpu.registers.cc & C, 0);
    }
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "A=1E20:B=RND(-1):C=RND(A):Z=85"
    ));
    let table = usize::from(word(&memory, 0x1b));
    assert_eq!(
        &memory.bytes[table + 16..table + 21],
        &[193, 121, 53, 65, 85]
    );
    assert_eq!(word(&memory, usize::from(symbol("STR_ROOTS"))), 0);
    assert_eq!(memory.bytes[usize::from(symbol("EX_DEPTH"))], 0);
}

#[test]
fn basic_rom_random_string_type_errors_advance_once_abort_and_recover() {
    for previous in ["X=1", "X=-1", "X=RND(-1)"] {
        for value in ["\"\"", "\"X\"", "\"HELLO\""] {
            let mut memory = Memory::new();
            let mut cpu = Cpu::default();
            empty_programme(&mut memory);
            enable_guest_printer(&mut memory);
            let mut code = Vec::new();
            call(&mut code, symbol("RND_INIT"));
            memory.execute(&mut cpu, &code);
            assert!(execute_direct(&mut memory, &mut cpu, previous));
            assert!(execute_direct(&mut memory, &mut cpu, "Q=0"));
            let initial = u32::from_be_bytes(memory.bytes[0x116..0x11a].try_into().unwrap());
            assert!(!execute_direct(
                &mut memory,
                &mut cpu,
                &format!("A=RND({value}):Q=99")
            ));
            let next = initial.wrapping_mul(0x40e6_4dab).wrapping_add(0x05b0_658b);
            assert_eq!(&memory.bytes[0x116..0x11a], &next.to_be_bytes());
            assert_eq!(word(&memory, usize::from(symbol("STR_ROOTS"))), 0);
            assert_eq!(memory.bytes[usize::from(symbol("EX_DEPTH"))], 0);
            assert!(execute_direct(&mut memory, &mut cpu, "B=RND(0):Z=85"));
            let table = usize::from(word(&memory, 0x1b));
            let end = usize::from(word(&memory, 0x1d));
            let value_of = |name| {
                let at = (table..end)
                    .step_by(7)
                    .find(|p| memory.bytes[*p] == name)
                    .unwrap();
                &memory.bytes[at + 2..at + 7]
            };
            assert_eq!(value_of(b'Q'), &[0; 5]);
            let expected = if previous == "X=RND(-1)" {
                [127, 114, 30, 40, 176]
            } else {
                [127, 33, 132, 92, 107]
            };
            assert_eq!(value_of(b'B'), &expected, "{previous} RND({value})");
            assert_eq!(value_of(b'Z'), &packed_integer(85));
        }
    }
}

#[test]
fn basic_rom_native_product_retains_all_64_bits() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    let mut pairs = vec![
        (0, 0),
        (1, u32::MAX),
        (u32::MAX, u32::MAX),
        (0x8000_0000, 0x8000_0000),
        (0x00ff_ffff, 0xff00_0001),
    ];
    let mut seed = 0x1234_5678_u32;
    for _ in 0..512 {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        pairs.push((seed, seed.rotate_left(13)));
    }
    for (left, right) in pairs {
        memory.bytes[0x5fff..0x6009].fill(0xa5);
        memory.bytes[0x50..0x54].copy_from_slice(&left.to_be_bytes());
        memory.bytes[0x5d..0x61].copy_from_slice(&right.to_be_bytes());
        cpu.registers.x = 0x1234;
        cpu.registers.y = 0x5678;
        cpu.registers.u = 0x6000;
        let mut code = Vec::new();
        call(&mut code, symbol("F_PRODUCT"));
        memory.execute(&mut cpu, &code);
        assert_eq!(
            &memory.bytes[0x6000..0x6008],
            &(u64::from(left) * u64::from(right)).to_be_bytes()
        );
        assert_eq!(&memory.bytes[0x50..0x54], &left.to_be_bytes());
        assert_eq!(&memory.bytes[0x5d..0x61], &right.to_be_bytes());
        assert_eq!(memory.bytes[0x5fff], 0xa5);
        assert_eq!(memory.bytes[0x6008], 0xa5);
        assert_eq!(cpu.registers.u, 0x6000);
        assert_eq!(cpu.registers.s, 0x7000);
    }
}

#[test]
fn basic_rom_native_random_product_preserves_legacy_scratch_across_seed_boundaries() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    let mut seeds = vec![0, 1, 255, 256, 65535, 65536, 0x8000_0000, u32::MAX];
    let mut seed = 0x1234_5678_u32;
    for _ in 0..512 {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        seeds.push(seed);
    }
    for seed in seeds {
        memory.bytes[0x116..0x11a].copy_from_slice(&seed.to_be_bytes());
        let mut code = Vec::new();
        call(&mut code, symbol("FAC_ZERO"));
        call(&mut code, symbol("VALUE_RND"));
        let before = memory.cycles;
        memory.execute(&mut cpu, &code);
        let product = seed.wrapping_mul(0x40e6_4dab);
        assert_eq!(&memory.bytes[0xab..0xaf], &product.to_be_bytes());
        assert_eq!(
            &memory.bytes[0x116..0x11a],
            &product.wrapping_add(0x05b0_658b).to_be_bytes()
        );
        assert_eq!(cpu.registers.cc & C, 0);
        assert!(memory.cycles - before < 2500, "seed {seed:08x}");
    }
}

#[test]
fn basic_rom_screen_preserves_the_token_cursor_and_writes_only_display_controls() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    let mut init = Vec::new();
    call(&mut init, symbol("GFX_INIT"));
    memory.execute(&mut cpu, &init);
    assert!(execute_direct(&mut memory, &mut cpu, "PMODE 4,1"));
    for (source, vdg, registers) in [
        (
            "SCREEN 1,1",
            0xf8,
            vec![
                0xffc0, 0xffc3, 0xffc5, 0xffc7, 0xffc9, 0xffca, 0xffcc, 0xffce, 0xffd0, 0xffd2,
            ],
        ),
        (
            "SCREEN 0,0",
            0,
            vec![
                0xffc0, 0xffc2, 0xffc4, 0xffc6, 0xffc9, 0xffca, 0xffcc, 0xffce, 0xffd0, 0xffd2,
            ],
        ),
    ] {
        memory.signals.clear();
        assert!(execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(memory.bytes[0xff22] & 0xf8, vdg);
        let selected: Vec<_> = memory
            .signals
            .iter()
            .filter(|s| s.1 >= 0xffc0)
            .map(|s| s.1)
            .collect();
        assert_eq!(selected, registers);
    }
    for source in ["SCREEN 2,0", "SCREEN 1,2", "SCREEN 1,0,2", "SCREEN \"BAD\""] {
        memory.signals.clear();
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert!(
            memory.signals.is_empty(),
            "invalid selection must not write device pins"
        );
    }
}

fn supply_original_keyboard(memory: &mut Memory, text: &[u8]) {
    // An original RAM supplier replaces only our own public keyboard entry.
    memory.bytes[0xbbe5..0xbbe8].copy_from_slice(&[0x7e, 0x65, 0]);
    memory.bytes[0x6500..0x650d].copy_from_slice(&[
        0x34, 0x10, 0xbe, 0x66, 0, 0xa6, 0x80, 0xbf, 0x66, 0, 0x35, 0x90, 0x12,
    ]);
    set_word(memory, 0x6600, 0x6700);
    memory.bytes[0x6700..0x6700 + text.len()].copy_from_slice(text);
    memory.bytes[0x182] = 0x39;
    set_word(memory, 0x88, 0x400);
}
#[test]
fn basic_rom_interactive_input_preserves_data_cursor_case_and_command_tokens() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    set_word(&mut memory, 0x33, 0x1e00);
    let active = usize::from(symbol("DATA_ACTIVE"));
    memory.bytes[active] = 0;
    supply_original_keyboard(&mut memory, b"-12.5,\"Mixed, !?\"\r");
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "INPUT A,B$:PRINT A;B$"
    ));
    assert!(memory.output.ends_with(b"-12.5 Mixed, !?\r"));
    assert_eq!(word(&memory, 0x33), 0x1e00);
    assert_eq!(memory.bytes[active], 0);
    memory.output.clear();
    supply_original_keyboard(&mut memory, b"  full, \"line\" !?\r");
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "LINE INPUT \"NAME\";C$:PRINT C$"
    ));
    assert_eq!(memory.output, b"NAME  full, \"line\" !?\r");
    assert_eq!(word(&memory, 0x33), 0x1e00);
    assert_eq!(memory.bytes[active], 0);
    assert!(execute_direct(&mut memory, &mut cpu, "CLEAR 500"));
    supply_original_keyboard(&mut memory, b"17\r");
    let command = format!("INPUT \"{}\";Q:PRINT Q", "P".repeat(220));
    assert!(execute_direct(&mut memory, &mut cpu, &command));
    assert!(memory.output.ends_with(b" 17 \r"));
}
#[test]
fn basic_rom_input_recovers_from_bad_replies_and_break_with_balanced_stack() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    supply_original_keyboard(&mut memory, b"oops\r42\r");
    assert!(execute_direct(&mut memory, &mut cpu, "INPUT A:PRINT A"));
    assert!(memory.output.ends_with(b" 42 \r"));
    assert!(memory.output.windows(5).any(|part| part == b"?REDO"));
    // A rejected string reply must restore the destination's type from the
    // current input frame before retrying; saved return bytes are not a type.
    supply_original_keyboard(&mut memory, b"\"Rejected\":tail\rCorrect\r");
    assert!(execute_direct(&mut memory, &mut cpu, "INPUT R$:PRINT R$"));
    assert!(memory.output.ends_with(b"Correct\r"));
    supply_original_keyboard(&mut memory, b"1\r2\r");
    assert!(execute_direct(&mut memory, &mut cpu, "INPUT A,B:PRINT A+B"));
    assert!(memory.output.ends_with(b" 3 \r"));
    supply_original_keyboard(&mut memory, b"3,Hello\r");
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "DIM C(1),D$(1):INPUT C(1), D$(1):PRINT C(1);D$(1)"
    ));
    assert!(memory.output.ends_with(b" 3 Hello\r"));
    supply_original_keyboard(&mut memory, &[3]);
    assert!(!execute_direct(&mut memory, &mut cpu, "INPUT A"));
    assert!(execute_direct(&mut memory, &mut cpu, "PRINT 7"));
    assert!(memory.output.ends_with(b" 7 \r"));
    for source in [
        "LINE INPUT A",
        "LINE INPUT A$,B$",
        "INPUT 12",
        "INPUT A,12",
        "INPUT \"BAD\" A",
    ] {
        supply_original_keyboard(&mut memory, b"1,2\r");
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
    }
}
#[test]
fn basic_rom_page_copy_validates_both_extents_before_writing() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    memory.bytes[0xbc] = 6;
    for (i, byte) in memory.bytes[0x600..0xc00].iter_mut().enumerate() {
        *byte = u8::try_from(i % 251).unwrap();
    }
    let source = memory.bytes[0x600..0xc00].to_vec();
    assert!(execute_direct(&mut memory, &mut cpu, "PCOPY 1,4:PRINT 7"));
    assert_eq!(&memory.bytes[0x1800..0x1e00], &source);
    assert!(memory.output.ends_with(b" 7 \r"));
    assert!(execute_direct(&mut memory, &mut cpu, "PCOPY 1,1"));
    assert_eq!(&memory.bytes[0x600..0xc00], &source);
    for source in [
        "PCOPY 0,1",
        "PCOPY 1,0",
        "PCOPY 7,1",
        "PCOPY 1,7",
        "PCOPY 1,2,3",
    ] {
        let before = memory.bytes[0x600..0x1e00].to_vec();
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(&memory.bytes[0x600..0x1e00], &before);
    }
}
#[test]
fn basic_rom_print_at_and_tab_match_observed_screen_cursor_positions() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    memory.bytes[0x6f] = 0;
    for (column, expected) in [(1, 0x404), (8, 0x409), (40, 0x429), (255, 0x500)] {
        assert!(execute_direct(
            &mut memory,
            &mut cpu,
            &format!("CLS:PRINT\"ABC\";TAB({column});\"X\";")
        ));
        assert_eq!(word(&memory, 0x88), expected);
    }
    for (position, expected) in [(0, 0x401), (511, 0x5e0)] {
        assert!(execute_direct(
            &mut memory,
            &mut cpu,
            &format!("CLS:PRINT@{position},\"X\";")
        ));
        assert_eq!(word(&memory, 0x88), expected);
    }
    for source in [
        "PRINT@512,\"BAD\"",
        "PRINT@-1,\"BAD\"",
        "PRINT@\"BAD\",1",
        "PRINT@3 1",
        "PRINT TAB(256)",
        "PRINT TAB(-1)",
        "PRINT TAB(\"BAD\")",
    ] {
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
    }
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "PRINT@5,\"OK\";:POKE 25344,165"
    ));
    assert_eq!(&memory.bytes[0x405..0x407], &[0x4f, 0x4b]);
    assert_eq!(memory.bytes[0x6300], 165);
}
#[test]
fn basic_rom_screen_output_retains_every_semigraphics_byte() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    memory.bytes[0x16a] = 0x39;
    for byte in 128..=255 {
        set_word(&mut memory, 0x88, 0x400);
        let mut code = vec![0x86, byte];
        call(&mut code, 0x800c);
        memory.execute(&mut cpu, &code);
        assert_eq!(memory.bytes[0x400], byte);
        assert_eq!(word(&memory, 0x88), 0x401);
    }
}
#[test]
fn basic_rom_initialisation_entries_return_without_entering_the_command_loop() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    memory.bytes[0x6000] = 0xa5;
    let mut code = Vec::new();
    call(&mut code, 0x8000);
    memory.execute(&mut cpu, &code);
    assert_eq!(memory.bytes[0xff21], 0x34);
    assert_eq!(memory.bytes[0x6000], 0xa5);
    code.clear();
    call(&mut code, 0x8003);
    memory.execute(&mut cpu, &code);
    assert_eq!(word(&memory, 0x19), 0x1e01);
    assert_eq!(word(&memory, 0x27), 0x7ffe);
    assert_eq!(memory.bytes[0x6000], 0xa5);
    assert_eq!(memory.bytes[0x100], 0x7e);
    assert_eq!(memory.bytes[0x16a], 0x39);
}
#[test]
fn basic_rom_run_line_clears_values_and_llist_restores_the_selected_device() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    store_line(&mut memory, &mut cpu, 10, "PRINT 10");
    store_line(&mut memory, &mut cpu, 20, "PRINT A");
    store_line(&mut memory, &mut cpu, 30, "END");
    assert!(execute_direct(&mut memory, &mut cpu, "A=9"));
    assert!(!execute_direct(&mut memory, &mut cpu, "RUN 15"));
    assert!(execute_direct(&mut memory, &mut cpu, "PRINT A"));
    assert!(memory.output.ends_with(b" 9 \r"));
    memory.output.clear();
    assert!(execute_direct(&mut memory, &mut cpu, "RUN 20"));
    assert_eq!(memory.output, b" 0 \r");
    assert!(!execute_direct(&mut memory, &mut cpu, "RUN 20,30"));
    memory.output.clear();
    memory.bytes[0x6f] = 0;
    assert!(execute_direct(&mut memory, &mut cpu, "LLIST"));
    assert_eq!(memory.output, b"10 PRINT 10\r20 PRINT A\r30 END\r");
    assert_eq!(memory.bytes[0x6f], 0);
    set_word(&mut memory, 0x1e01, 0xffff);
    assert!(!execute_direct(&mut memory, &mut cpu, "LLIST"));
    assert_eq!(memory.bytes[0x6f], 0);
}

#[test]
fn basic_rom_explicit_print_devices_keep_columns_and_restore_after_failure() {
    for (source, expected, column) in [
        ("PRINT#-2,\"ABC\";", b"ABC".as_slice(), 3),
        ("PRINT#-2,\"ABC\"", b"ABC\r".as_slice(), 0),
        (
            "PRINT#-2,\"ABC\",\"X\";",
            b"ABC             X".as_slice(),
            17,
        ),
        ("PRINT#-2,\"ABC\";TAB(8);\"X\";", b"ABC     X".as_slice(), 9),
        ("PRINT#-2,\"\"", b" \r".as_slice(), 0),
        ("PRINT#-2,CHR$(13);", b" \r".as_slice(), 0),
    ] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        memory.bytes[0x6f] = 0;
        memory.bytes[0x6c] = 7;
        assert!(execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(memory.output, expected, "{source}");
        assert_eq!(memory.bytes[0x9c], column, "{source}");
        assert_eq!(memory.bytes[0x6c], 7, "{source}");
        assert_eq!(memory.bytes[0x6f], 0, "{source}");
    }
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    set_word(&mut memory, 0x88, 0x400);
    assert!(execute_direct(&mut memory, &mut cpu, "PRINT#0,\"ABC\";"));
    assert!(memory.output.is_empty());
    assert_eq!(&memory.bytes[0x400..0x403], &[0x41, 0x42, 0x43]);
    assert_eq!(memory.bytes[0x6c], 3);
    assert_eq!(memory.bytes[0x6f], 0xfe);
    for source in [
        "PRINT#-2,",
        "PRINT#0,",
        "PRINT#-1,\"X\"",
        "PRINT#1,\"X\"",
        "PRINT#-3,\"X\"",
        "PRINT#32768,\"X\"",
        "PRINT#-32769,\"X\"",
        "PRINT#\"X\",1",
        "PRINT#-2 \"X\"",
    ] {
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(memory.bytes[0x6f], 0xfe);
        assert!(memory.output.is_empty());
    }
    memory.bytes[0x6f] = 0;
    assert!(!execute_direct(&mut memory, &mut cpu, "PRINT#-2,1/0"));
    assert_eq!(memory.bytes[0x6f], 0);
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "PRINT#-2,\"RECOVERED\""
    ));
    assert_eq!(memory.output, b"RECOVERED\r");
    assert_eq!(memory.bytes[0x6f], 0);
}

#[test]
fn basic_rom_collector_closes_many_adjacent_roots_without_counter_wrap() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    set_word(&mut memory, 0x1b, 0x3000);
    let table = usize::from(word(&memory, 0x1b));
    let count = 256;
    set_word(&mut memory, 0x1d, u16::try_from(table + count * 7).unwrap());
    let end = word(&memory, 0x1d);
    set_word(&mut memory, 0x1f, end);
    set_word(&mut memory, 0x21, 0x6000);
    set_word(&mut memory, 0x23, 0x6000);
    for i in 0..count {
        let address = table + i * 7;
        memory.bytes[address..address + 7].copy_from_slice(&[b'A', 128, 2, 0, 0, 0, 0]);
        set_word(
            &mut memory,
            address + 4,
            u16::try_from(0x6100 + i * 2).unwrap(),
        );
        memory.bytes[0x6100 + i * 2] = u8::try_from(i).unwrap();
        memory.bytes[0x6101 + i * 2] = u8::try_from(i).unwrap() ^ 0xa5;
    }
    let expected = memory.bytes[0x6100..0x6300].to_vec();
    let mut code = Vec::new();
    call(&mut code, symbol("STRING_COLLECT"));
    memory.execute_with_budget(&mut cpu, &code, 4_194_304);
    assert_eq!(cpu.registers.cc & C, 0);
    assert_eq!(word(&memory, 0x23), 0x7dfe);
    assert_eq!(&memory.bytes[0x7dff..0x7fff], expected);
    for i in 0..count {
        assert_eq!(
            word(&memory, table + i * 7 + 4),
            0x7dff + u16::try_from(i * 2).unwrap()
        );
        assert_eq!(memory.bytes[table + i * 7 + 3], 0);
        assert_eq!(memory.bytes[table + i * 7 + 6], 0);
    }
}

fn graphics_plane(memory: &Memory, mode: u8) -> Vec<u8> {
    let width = if mode == 4 { 256 } else { 128 };
    let height = if mode < 2 { 96 } else { 192 };
    let bits = if mode & 1 == 0 { 1 } else { 2 };
    (0..width * height)
        .map(|i| {
            let shift = 8 - bits - (i % (8 / bits)) * bits;
            let colour = (memory.bytes[0x600 + i / (8 / bits)] >> shift) & ((1 << bits) - 1);
            if bits == 1 { colour * 3 } else { colour }
        })
        .collect()
}

fn set_graphics_pixel(memory: &mut Memory, mode: u8, x: usize, y: usize, colour: u8) {
    let width = if mode == 4 { 256 } else { 128 };
    let bits = if mode & 1 == 0 { 1 } else { 2 };
    let i = y * width + x;
    let shift = 8 - bits - (i % (8 / bits)) * bits;
    let mask = ((1 << bits) - 1) << shift;
    let packed = if bits == 1 {
        u8::from(colour != 0)
    } else {
        colour
    };
    let cell = &mut memory.bytes[0x600 + i / (8 / bits)];
    *cell = (*cell & !mask) | (packed << shift);
}

fn model_boundary_fill(
    plane: &mut [u8],
    width: usize,
    seed: (usize, usize),
    paint: u8,
    border: u8,
) {
    let mut queue = std::collections::VecDeque::from([seed]);
    let mut visited = vec![false; plane.len()];
    while let Some((x, y)) = queue.pop_front() {
        let i = y * width + x;
        let cell = &mut plane[i];
        if visited[i] || *cell == border {
            continue;
        }
        visited[i] = true;
        *cell = paint;
        if x > 0 {
            queue.push_back((x - 1, y));
        }
        if x + 1 < width {
            queue.push_back((x + 1, y));
        }
        if y > 0 {
            queue.push_back((x, y - 1));
        }
        if (y + 1) * width < plane.len() {
            queue.push_back((x, y + 1));
        }
    }
}

#[test]
fn basic_rom_paint_matches_independent_boundary_fill_in_every_mode() {
    for mode in 0..=4 {
        for seed in [(3, 3), (0, 0), (12, 12), (2, 2)] {
            let mut memory = Memory::new();
            let mut cpu = Cpu::default();
            empty_programme(&mut memory);
            let mut init = Vec::new();
            call(&mut init, symbol("GFX_INIT"));
            memory.execute(&mut cpu, &init);
            assert!(execute_direct(
                &mut memory,
                &mut cpu,
                &format!("PMODE{mode},1:PCLS{}", u8::from(mode & 1 != 0))
            ));
            let width = if mode == 4 { 256 } else { 128 };
            let sx = if mode == 4 { 1 } else { 2 };
            let sy = if mode < 2 { 2 } else { 1 };
            let paint = if mode & 1 == 0 { 3 } else { 1 };
            for y in 2..=24 {
                for x in 2..=30 {
                    if x == 2
                        || x == 30
                        || y == 2
                        || y == 24
                        || (x == 10 && y < 20)
                        || (x == 20 && y > 6)
                    {
                        set_graphics_pixel(&mut memory, mode, x, y, 3);
                    }
                }
            }
            // Existing paint colours must remain traversable when they differ from the border.
            for x in 12..=16 {
                for y in 10..=14 {
                    if x == 12 || x == 16 || y == 10 || y == 14 {
                        set_graphics_pixel(&mut memory, mode, x, y, paint);
                    }
                }
            }
            let mut expected = graphics_plane(&memory, mode);
            model_boundary_fill(&mut expected, width, seed, paint, 3);
            memory.bytes[0xd8..0xea].fill(0xa5);
            let programme = memory.bytes[0x1e00..0x6000].to_vec();
            let colour = if mode & 1 == 0 { 1 } else { 2 };
            let border = if mode & 1 == 0 { 1 } else { 4 };
            let source = format!(
                "PAINT({},{}),{colour},{border}",
                seed.0 * sx + sx - 1,
                seed.1 * sy + sy - 1
            );
            assert!(
                execute_direct_with_budget(&mut memory, &mut cpu, &source, 33_554_432),
                "mode {mode} seed {seed:?}"
            );
            let actual = graphics_plane(&memory, mode);
            let mismatch = actual
                .iter()
                .zip(&expected)
                .enumerate()
                .find(|(_, (a, e))| a != e);
            assert!(
                mismatch.is_none(),
                "mode {mode} seed {seed:?} first mismatch {mismatch:?}"
            );
            assert_eq!(&memory.bytes[0xd8..0xea], &[0xa5; 18]);
            assert_eq!(&memory.bytes[0x1e00..0x6000], programme);
        }
    }
}

#[test]
fn basic_rom_paint_validates_before_drawing_and_recovers_after_errors() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    let mut init = Vec::new();
    call(&mut init, symbol("GFX_INIT"));
    memory.execute(&mut cpu, &init);
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "PMODE4,1:PCLS0:LINE(2,2)-(10,10),PSET,B"
    ));
    let before = graphics_plane(&memory, 4);
    for source in [
        "PAINT(0,256)",
        "PAINT(-1,0)",
        "PAINT(65536,0)",
        "PAINT(5,5),9",
        "PAINT(5,5),1,9",
        "PAINT(5,5),1,1,1",
        "PAINT(5,5),\"X\"",
        "PAINT(5,5) GARBAGE",
        "PAINT(5,5),",
    ] {
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(graphics_plane(&memory, 4), before, "{source}");
    }
    // A nested graphics function must not replace the seed coordinates.
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "PAINT(5,5),1+PPOINT(0,0),1"
    ));
    let mut expected = before;
    model_boundary_fill(&mut expected, 256, (5, 5), 3, 3);
    assert_eq!(graphics_plane(&memory, 4), expected);
    assert!(execute_direct_with_budget(
        &mut memory,
        &mut cpu,
        "PCLS0:PAINT(255,191)",
        16_777_216
    ));
    assert!(graphics_plane(&memory, 4).iter().all(|&pixel| pixel == 3));
}

#[test]
fn basic_rom_paint_stack_exhaustion_keeps_the_programme_guard_and_recovers() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    let mut init = Vec::new();
    call(&mut init, symbol("GFX_INIT"));
    memory.execute(&mut cpu, &init);
    assert!(execute_direct(&mut memory, &mut cpu, "PMODE4,1:PCLS0"));
    // A comb above a free scanline queues many separate adjacent spans.
    for x in (0..256).step_by(2) {
        for y in 0..96 {
            set_graphics_pixel(&mut memory, 4, x, y, 3);
        }
    }
    memory.bytes[0x6e00..0x6e80].fill(0xa5);
    set_word(&mut memory, 0x1f, 0x6e80);
    assert!(!execute_direct_with_budget(
        &mut memory,
        &mut cpu,
        "PAINT(0,96),1,1",
        16_777_216
    ));
    assert_eq!(&memory.bytes[0x6e00..0x6e80], &[0xa5; 128]);
    set_word(&mut memory, 0x1f, 0x1e03);
    assert!(execute_direct_with_budget(
        &mut memory,
        &mut cpu,
        "PCLS0:PAINT(255,191)",
        16_777_216
    ));
    assert!(graphics_plane(&memory, 4).iter().all(|&pixel| pixel == 3));
}

#[test]
fn basic_rom_paint_selected_lawful_colour_and_seed_observations() {
    for (source, expected) in [
        (
            "PMODE3,1:PCLS1:LINE(2,2)-(30,24),PSET,B:COLOR2:LINE(10,2)-(10,24),PSET:PAINT(5,5),2,4",
            [2, 2, 4, 1],
        ),
        (
            "PMODE3,1:PCLS1:LINE(2,2)-(30,24),PSET,B:COLOR2:LINE(2,10)-(30,10),PSET:PAINT(5,5),2,4",
            [2, 2, 4, 2],
        ),
        (
            "PMODE3,1:PCLS1:LINE(2,2)-(30,24),PSET,B:COLOR2:LINE(4,10)-(28,10),PSET:PAINT(5,5),2,4",
            [2, 1, 4, 1],
        ),
        (
            "PMODE3,1:PCLS1:LINE(2,2)-(30,24),PSET,B:PSET(10,2,2):PAINT(5,5),2,4",
            [2, 2, 4, 1],
        ),
        (
            "PMODE3,1:PCLS1:LINE(2,2)-(30,24),PSET,B:PSET(5,5,2):PAINT(5,5),2,4",
            [2, 2, 4, 1],
        ),
        (
            "PMODE3,1:PCLS1:LINE(2,2)-(30,24),PSET,B:PAINT(5,5),0,4",
            [4, 4, 4, 1],
        ),
        (
            "PMODE3,1:PCLS1:LINE(2,2)-(30,24),PSET,B:PAINT(5,5),2,0",
            [2, 2, 4, 1],
        ),
    ] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        let mut init = Vec::new();
        call(&mut init, symbol("GFX_INIT"));
        memory.execute(&mut cpu, &init);
        assert!(execute_direct_with_budget(
            &mut memory,
            &mut cpu,
            source,
            16_777_216
        ));
        for ((x, y), colour) in [(5, 5), (20, 20), (2, 2), (0, 0)].into_iter().zip(expected) {
            assert!(execute_direct(
                &mut memory,
                &mut cpu,
                &format!("Z=PPOINT({x},{y})")
            ));
            assert_eq!(
                numeric_scalar(&memory, [b'Z', 0]),
                packed_integer(colour),
                "{source} at {x},{y}"
            );
        }
    }
}

#[test]
fn basic_rom_paint_complete_planes_match_lawful_synthetic_observations() {
    use sha2::{Digest, Sha256};
    #[derive(serde::Deserialize)]
    struct Fixture {
        #[serde(rename = "schemaVersion")]
        schema_version: u32,
        cases: Vec<Case>,
    }
    #[derive(serde::Deserialize)]
    struct Case {
        name: String,
        commands: Vec<String>,
        mode: u8,
        bytes: usize,
        sha256: String,
    }
    let fixture: Fixture =
        serde_json::from_str(include_str!("fixtures/paint-layouts.json")).unwrap();
    assert_eq!(fixture.schema_version, 1);
    assert_eq!(fixture.cases.len(), 28);
    for case in fixture.cases {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        let mut init = Vec::new();
        call(&mut init, symbol("GFX_INIT"));
        memory.execute(&mut cpu, &init);
        memory.bytes[0xd8..0xea].fill(0xa5);
        let programme = memory.bytes[0x1e00..0x6000].to_vec();
        for source in case.commands {
            assert!(
                execute_direct_with_budget(&mut memory, &mut cpu, &source, 33_554_432),
                "{}: {source}",
                case.name
            );
        }
        assert_eq!(memory.bytes[0xb6], case.mode, "{}", case.name);
        assert_eq!(word(&memory, 0xba), 0x600, "{}", case.name);
        assert_eq!(usize::from(word(&memory, 0xb7)), 0x600 + case.bytes);
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(&memory.bytes[0x600..0x600 + case.bytes])
            ),
            case.sha256,
            "{}",
            case.name
        );
        assert_eq!(&memory.bytes[0xd8..0xea], &[0xa5; 18]);
        // The probe uses direct scalar result records, which begin at $1E03.
        assert_eq!(&memory.bytes[0x1e00..0x1e03], &programme[..3]);
        assert_eq!(&memory.bytes[0x3000..0x6000], &programme[0x1200..]);
    }
}

#[test]
fn basic_rom_collector_rejects_malformed_late_roots_before_moving_strings() {
    for corrupt in 0..4 {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        assert!(execute_direct(
            &mut memory,
            &mut cpu,
            "DIM B$(1):A$=\"HELLO\":B$(0)=\"WORLD\""
        ));
        let array = usize::from(word(&memory, 0x1d));
        match corrupt {
            0 => set_word(&mut memory, array + 2, 6),
            1 => memory.bytes[array + 4] = 0,
            2 => set_word(&mut memory, array + 9, 0x7ffe),
            _ => {
                set_word(&mut memory, usize::from(symbol("STR_ROOTS")), 0x7100);
                set_word(&mut memory, 0x7100, 0x7100);
            }
        }
        let variables = memory.bytes[0x1e03..usize::from(word(&memory, 0x1f))].to_vec();
        let heap = memory.bytes[0x7f00..0x7fff].to_vec();
        let free = word(&memory, 0x23);
        let mut code = Vec::new();
        call(&mut code, symbol("STRING_COLLECT"));
        memory.execute_with_budget(&mut cpu, &code, 1_048_576);
        assert_ne!(cpu.registers.cc & C, 0, "corruption {corrupt}");
        assert_eq!(
            memory.bytes[0x1e03..usize::from(word(&memory, 0x1f))],
            variables
        );
        assert_eq!(memory.bytes[0x7f00..0x7fff], heap);
        assert_eq!(word(&memory, 0x23), free);
    }
}

#[test]
fn basic_rom_shared_array_validation_checks_every_extent_before_collection() {
    for dimensions in [1, 8, 125] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        let header = 5 + dimensions * 2;
        set_word(&mut memory, 0x1f, 0x3000 + header + 5);
        set_word(&mut memory, 0x3002, header + 5);
        memory.bytes[0x3004] = u8::try_from(dimensions).unwrap();
        for dimension in 0..dimensions {
            set_word(&mut memory, usize::from(0x3005 + dimension * 2), 1);
        }
        cpu.registers.y = 0x3000;
        cpu.registers.u = 0x6789;
        let mut code = Vec::new();
        call(&mut code, symbol("ARRAY_RECORD"));
        memory.execute(&mut cpu, &code);
        assert_eq!(cpu.registers.cc & C, 0);
        assert_eq!(cpu.registers.d(), header);
        assert_eq!(cpu.registers.x, 0x3000 + header + 5);
        assert_eq!((cpu.registers.y, cpu.registers.u), (0x3000, 0x6789));
        set_word(&mut memory, usize::from(0x3003 + dimensions * 2), 0);
        memory.execute(&mut cpu, &code);
        assert_eq!(
            cpu.registers.cc & C,
            0,
            "numeric G marker, {dimensions} dimensions"
        );
        assert_eq!(cpu.registers.d(), header);
        assert_eq!(cpu.registers.x, 0x3000 + header + 5);
    }
    for corruption in 0..10 {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        assert!(execute_direct(
            &mut memory,
            &mut cpu,
            "DIM B(1):A$=\"HELLO\""
        ));
        let array = usize::from(word(&memory, 0x1d));
        match corruption {
            0 => set_word(&mut memory, 0x1f, u16::try_from(array + 6).unwrap()),
            1 => set_word(&mut memory, array + 2, 6),
            2 => set_word(&mut memory, array + 2, 0xffff),
            3 => set_word(&mut memory, array + 2, 18),
            4 => memory.bytes[array + 4] = 0,
            5 => memory.bytes[array + 4] = 126,
            6 => memory.bytes[array + 4] = 8,
            7 => {
                // A final zero is a valid GET G marker; its retained payload
                // must still contain an integral number of numeric elements.
                set_word(&mut memory, array + 5, 0);
                set_word(&mut memory, array + 2, 16);
            }
            8 => set_word(&mut memory, array + 5, 0xffff),
            _ => set_word(&mut memory, array + 5, 1),
        }
        let records = memory.bytes[0x1e03..0x1f00].to_vec();
        let heap = memory.bytes[0x7f00..0x7fff].to_vec();
        let free = word(&memory, 0x23);
        let mut code = Vec::new();
        call(&mut code, symbol("STRING_COLLECT"));
        memory.execute(&mut cpu, &code);
        assert_ne!(cpu.registers.cc & C, 0, "corruption {corruption}");
        assert_eq!(memory.bytes[0x1e03..0x1f00], records);
        assert_eq!(memory.bytes[0x7f00..0x7fff], heap);
        assert_eq!(word(&memory, 0x23), free);
    }
}

#[test]
fn basic_rom_native_failure_scopes_restore_registers_depth_roots_and_parent_boundary() {
    for (entry, source) in [
        ("EVAL", "1+\"X\""),
        ("EVAL", "1/(2-2)"),
        ("VAR_FIND", "A(-1)"),
    ] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        guest_tokens(&mut memory, &mut cpu, source);
        set_word(&mut memory, usize::from(symbol("STATEMENT_SP")), 0xa55a);
        memory.bytes[0x6f] = 0xfe;
        cpu.registers.x = 0x02dd;
        cpu.registers.y = 0x1234;
        cpu.registers.u = 0x2345;
        let mut code = Vec::new();
        call(&mut code, symbol(entry));
        memory.execute_with_budget(&mut cpu, &code, 4_194_304);
        assert_ne!(cpu.registers.cc & C, 0, "{entry}: {source}");
        assert_eq!(cpu.registers.s, 0x7000);
        assert_eq!(cpu.registers.y, 0x1234);
        assert_eq!(cpu.registers.u, 0x2345);
        assert_eq!(word(&memory, usize::from(symbol("STATEMENT_SP"))), 0xa55a);
        assert_eq!(word(&memory, usize::from(symbol("STR_ROOTS"))), 0);
        assert_eq!(memory.bytes[usize::from(symbol("EX_DEPTH"))], 0);
        assert_eq!(memory.bytes[0x6f], 0xfe);
    }
}

#[test]
fn basic_rom_nested_statement_call_from_usr_restores_the_enclosing_parser() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    guest_tokens(&mut memory, &mut cpu, "POKE25345,85");
    let tokens = memory.bytes[0x02dd..=0x03d8].to_vec();
    memory.bytes[0x6200..0x62fc].copy_from_slice(&tokens);
    let mut observer = vec![0x8e, 0x62, 0];
    call(&mut observer, 0x84d6);
    observer.extend([0xcc, 0, 14]);
    call(&mut observer, 0x8c37);
    observer.push(0x39);
    memory.bytes[0x6400..0x6400 + observer.len()].copy_from_slice(&observer);
    set_word(&mut memory, 0xb0, 0x134);
    set_word(&mut memory, 0x134, 0x6400);
    set_word(&mut memory, usize::from(symbol("STATEMENT_SP")), 0xa55a);
    assert!(execute_direct(&mut memory, &mut cpu, "A=USR0(0)+1:PRINT A"));
    assert_eq!(memory.output, b" 15 \r");
    assert_eq!(memory.bytes[0x6301], 85);
    assert_eq!(word(&memory, usize::from(symbol("STATEMENT_SP"))), 0xa55a);
    assert_eq!(word(&memory, usize::from(symbol("STR_ROOTS"))), 0);
}

fn append_synthetic_cassette_block(tape: &mut Vec<u8>, kind: u8, payload: &[u8]) {
    let length = u8::try_from(payload.len()).unwrap();
    tape.extend([0x55; 128]);
    tape.extend([0x55, 0x3c, kind, length]);
    tape.extend(payload);
    let checksum = payload
        .iter()
        .fold(kind.wrapping_add(length), |sum, value| {
            sum.wrapping_add(*value)
        });
    tape.extend([checksum, 0x55]);
}
fn append_synthetic_cassette_file(tape: &mut Vec<u8>, name: [u8; 8]) {
    let mut header = name.to_vec();
    header.extend([2, 0, 0, 0x60, 0, 0x60, 0]);
    append_synthetic_cassette_block(tape, 0, &header);
    append_synthetic_cassette_block(tape, 1, &[0x12, 0x34, 0x56]);
    append_synthetic_cassette_block(tape, 1, &[0x78]);
    append_synthetic_cassette_block(tape, 255, &[]);
}

#[test]
fn basic_rom_skipf_finds_named_files_and_leaves_the_following_file_available() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    assert!(execute_direct(&mut memory, &mut cpu, "REM"));
    let programme = memory.bytes[0x1e00..0x6000].to_vec();
    let mut tape = Vec::new();
    for name in [b"FIRST   ", b"WANTED  ", b"NEXT    "] {
        append_synthetic_cassette_file(&mut tape, *name);
    }
    tape.extend([0x55; 16]);
    synthetic_tape(&mut memory, &tape);
    assert!(execute_direct_with_budget(
        &mut memory,
        &mut cpu,
        "SKIPF\"WANTED\":POKE25345,85",
        16_777_216
    ));
    assert_eq!(memory.bytes[0x6301], 85);
    assert_eq!(memory.bytes[0x81], 0);
    assert_eq!(memory.bytes[0xff21] & 8, 0);
    assert!(memory.input_cursor > 0 && memory.input_cursor + 1 < memory.input_signal.len());
    assert!(execute_direct_with_budget(
        &mut memory,
        &mut cpu,
        "SKIPF\"NEXT\"",
        16_777_216
    ));
    assert_eq!(memory.bytes[0x81], 0);
    assert_eq!(memory.bytes[0xff21] & 8, 0);
    assert_eq!(memory.bytes[0x7c], 255);
    assert_eq!(memory.bytes[0x7d], 0);
    assert_eq!(&memory.bytes[0x1e00..0x6000], programme);
    assert_eq!(word(&memory, 0x7e), 0x01da);
}

#[test]
fn basic_rom_skipf_wildcards_accept_any_cassette_file_type() {
    for (source, file_type) in [("SKIPF", 0_u8), ("SKIPF\"\"", 1), ("SKIPF\"FILE\"", 2)] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        let mut tape = Vec::new();
        let mut header = b"FILE    ".to_vec();
        header.extend([file_type, 0, 0, 0, 0, 0, 0]);
        append_synthetic_cassette_block(&mut tape, 0, &header);
        append_synthetic_cassette_block(&mut tape, 1, &[1, 2, 3]);
        append_synthetic_cassette_block(&mut tape, 255, &[]);
        tape.extend([0x55; 16]);
        synthetic_tape(&mut memory, &tape);
        assert!(
            execute_direct_with_budget(&mut memory, &mut cpu, source, 16_777_216),
            "{source}: status {}, FAC {:?}, type {}, CPU {:?}",
            memory.bytes[0x81],
            &memory.bytes[0x4f..0x57],
            memory.bytes[6],
            cpu.registers
        );
        assert_eq!(memory.bytes[0xff21] & 8, 0);
    }
}

#[test]
fn basic_rom_skipf_rejects_bad_syntax_before_motor_or_input_activity() {
    for source in ["SKIPF 1", "SKIPF\"TOO LONG!\"", "SKIPF\"FILE\",1", "SKIPF("] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        let mut tape = Vec::new();
        append_synthetic_cassette_file(&mut tape, *b"FILE    ");
        synthetic_tape(&mut memory, &tape);
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(memory.input_cursor, 0);
        assert_eq!(memory.bytes[0xff21], 0);
        assert!(execute_direct(&mut memory, &mut cpu, "A=17"));
        assert_eq!(word(&memory, usize::from(symbol("STR_ROOTS"))), 0);
    }
}

#[test]
fn basic_rom_skipf_stops_motor_and_recovers_after_corrupt_or_missing_tape() {
    for variant in 0..5 {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        let mut tape = Vec::new();
        let mut header = b"FILE    ".to_vec();
        header.extend([2, 0, 0, 0, 0, 0, 0]);
        if variant == 1 {
            header.pop();
        }
        append_synthetic_cassette_block(&mut tape, 0, &header);
        if variant == 0 {
            let checksum = tape.len() - 2;
            tape[checksum] ^= 1;
        }
        match variant {
            2 => append_synthetic_cassette_block(&mut tape, 2, &[]),
            3 => append_synthetic_cassette_block(&mut tape, 255, &[1]),
            _ => {}
        }
        if variant < 4 {
            tape.extend([0x55; 16]);
        }
        synthetic_tape(&mut memory, &tape);
        assert!(
            !execute_direct_with_budget(&mut memory, &mut cpu, "SKIPF\"FILE\"", 16_777_216),
            "variant {variant}"
        );
        assert_eq!(
            memory.bytes[0x81],
            if variant == 0 {
                1
            } else if variant == 4 {
                3
            } else {
                2
            }
        );
        assert_eq!(memory.bytes[0xff21] & 8, 0);
        assert!(execute_direct(&mut memory, &mut cpu, "A=17"));
        assert_eq!(word(&memory, usize::from(symbol("STR_ROOTS"))), 0);
        assert_eq!(memory.bytes[usize::from(symbol("EX_DEPTH"))], 0);
    }
}

#[test]
fn basic_rom_cloadm_consumes_authoritative_media_blocks_with_unsigned_wrapping_offsets() {
    let payload: Vec<u8> = (0..268_u16)
        .map(|i| i.wrapping_mul(17).wrapping_add(3).to_le_bytes()[0])
        .collect();
    for (offset, target) in [
        ("0", 0x6000_usize),
        ("256", 0x6100),
        ("65280", 0x5f00),
        ("65535", 0x5fff),
    ] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        let mut tape =
            eightbit_dragon_media::create_machine_code("NATIVE", &payload, 0x6000, 0x6002).unwrap();
        tape.extend([0x55; 16]);
        synthetic_tape(&mut memory, &tape);
        memory.bytes[target - 1..=target + payload.len()].fill(0xa5);
        assert!(
            execute_direct_with_budget(
                &mut memory,
                &mut cpu,
                &format!("CLOADM\"NATIVE\",{offset}"),
                16_777_216
            ),
            "offset {offset}, status {}",
            memory.bytes[0x81]
        );
        assert_eq!(
            &memory.bytes[target..target + payload.len()],
            payload,
            "offset {offset}"
        );
        assert_eq!(memory.bytes[target - 1], 0xa5);
        assert_eq!(memory.bytes[target + payload.len()], 0xa5);
        assert_eq!(word(&memory, 0x9d), u16::try_from(target + 2).unwrap());
        assert_eq!(memory.bytes[0xff21] & 8, 0);
        assert_eq!(memory.bytes[0x81], 0);
    }
}

#[test]
fn basic_rom_cloadm_then_exec_uses_the_loaded_original_entry_and_legacy_stack() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    // Original guest code: NOP; LDD #165; STD $6300; RTS.
    let payload = [0x12, 0xcc, 0, 165, 0xfd, 0x63, 0, 0x39];
    let mut tape =
        eightbit_dragon_media::create_machine_code("NATIVE", &payload, 0x6100, 0x6101).unwrap();
    tape.extend([0x55; 16]);
    synthetic_tape(&mut memory, &tape);
    assert!(execute_direct_with_budget(
        &mut memory,
        &mut cpu,
        "CLOADM\"NATIVE\",65280:EXEC:POKE25346,85",
        16_777_216
    ));
    assert_eq!(word(&memory, 0x6300), 165);
    assert_eq!(memory.bytes[0x6302], 85);
    assert_eq!(word(&memory, 0x9d), 0x6001);
    assert_eq!(memory.bytes[0xff21] & 8, 0);
}

#[test]
fn basic_rom_cloadm_checks_each_complete_block_before_writing_past_ram() {
    for (load, payload, succeeds) in [
        (0x7fff_u16, vec![77_u8], true),
        (0x7fff, vec![77, 88], false),
        (0x8000, vec![77], false),
        (0xffff, vec![77, 88], false),
    ] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        memory.bytes[0x7fff] = 0xa5;
        set_word(&mut memory, 0x9d, 0x1234);
        let mut tape = Vec::new();
        let mut header = b"NATIVE  ".to_vec();
        header.extend([2, 0, 0]);
        header.extend(load.to_be_bytes());
        header.extend(load.to_be_bytes());
        append_synthetic_cassette_block(&mut tape, 0, &header);
        append_synthetic_cassette_block(&mut tape, 1, &payload);
        append_synthetic_cassette_block(&mut tape, 255, &[]);
        tape.extend([0x55; 16]);
        synthetic_tape(&mut memory, &tape);
        assert_eq!(
            execute_direct_with_budget(&mut memory, &mut cpu, "CLOADM\"NATIVE\"", 16_777_216),
            succeeds,
            "load {load:#06x}"
        );
        assert_eq!(memory.bytes[0xff21] & 8, 0);
        if succeeds {
            assert_eq!(memory.bytes[0x7fff], 77);
            assert_eq!(word(&memory, 0x9d), load);
        } else {
            assert_eq!(memory.bytes[0x7fff], 0xa5);
            assert_eq!(word(&memory, 0x9d), load);
            assert_eq!(memory.bytes[0x81], 2);
        }
    }
}

#[test]
fn basic_rom_cloadm_rejects_wrong_headers_bad_checksums_and_truncation() {
    for variant in 0..4 {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        memory.bytes[0x6000..0x6003].fill(0xa5);
        set_word(&mut memory, 0x9d, 0x1234);
        let mut tape = Vec::new();
        let mut header = b"NATIVE  ".to_vec();
        header.extend([
            if variant == 0 { 0 } else { 2 },
            if variant == 1 { 255 } else { 0 },
            0,
            0x60,
            0,
            0x60,
            0,
        ]);
        append_synthetic_cassette_block(&mut tape, 0, &header);
        append_synthetic_cassette_block(&mut tape, 1, &[2, 4, 7]);
        if variant == 2 {
            let checksum = tape.len() - 2;
            tape[checksum] ^= 1;
        }
        if variant != 3 {
            append_synthetic_cassette_block(&mut tape, 255, &[]);
        }
        tape.extend([0x55; 16]);
        synthetic_tape(&mut memory, &tape);
        assert!(
            !execute_direct_with_budget(&mut memory, &mut cpu, "CLOADM\"NATIVE\"", 16_777_216),
            "variant {variant}"
        );
        assert_eq!(memory.bytes[0xff21] & 8, 0);
        assert_eq!(
            word(&memory, 0x9d),
            if variant < 2 { 0x1234 } else { 0x6000 }
        );
        assert_eq!(
            memory.bytes[0x81],
            if variant == 2 {
                1
            } else if variant == 3 {
                3
            } else {
                2
            }
        );
        assert_eq!(
            &memory.bytes[0x6000..0x6003],
            if variant >= 2 { &[2, 4, 7] } else { &[0xa5; 3] }
        );
        assert!(execute_direct(&mut memory, &mut cpu, "A=17"));
    }
}

#[test]
fn basic_rom_cloadm_validates_syntax_without_starting_the_motor() {
    for source in [
        "CLOADM 1",
        "CLOADM\"NATIVE\",-256",
        "CLOADM\"NATIVE\",-32769",
        "CLOADM\"NATIVE\",65536",
        "CLOADM\"NATIVE\",\"X\"",
        "CLOADM\"NATIVE\",1,2",
    ] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(memory.bytes[0xff21], 0);
        assert_eq!(word(&memory, usize::from(symbol("STR_ROOTS"))), 0);
        assert!(execute_direct(&mut memory, &mut cpu, "A=17"));
    }
}

#[test]
fn basic_rom_default_exec_without_a_loaded_entry_fails_and_recovers() {
    let memory = cold_boot_to_probe("EXEC\rPOKE25345,85:EXEC25000\r");
    assert_eq!(memory.bytes[0x6301], 85);
    let screen: String = memory.bytes[0x400..0x600]
        .iter()
        .map(|&byte| {
            let value = byte & 63;
            char::from(if value < 32 { value + 64 } else { value })
        })
        .collect();
    assert!(screen.contains("?SN ERROR"));
}

#[test]
fn basic_rom_cloadm_allows_a_legitimate_default_execution_entry_at_zero() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    let payload = [0xcc, 0, 165, 0xfd, 0x63, 0, 0x39];
    let mut tape = eightbit_dragon_media::create_machine_code("ZERO", &payload, 0, 0).unwrap();
    tape.extend([0x55; 16]);
    synthetic_tape(&mut memory, &tape);
    assert!(execute_direct_with_budget(
        &mut memory,
        &mut cpu,
        "CLOADM\"ZERO\":EXEC",
        16_777_216
    ));
    assert_eq!(word(&memory, 0x9d), 0);
    assert_eq!(memory.bytes[0x6301], 165);
}

fn cassette_pulses(bytes: &[u8]) -> Vec<eightbit_dragon_emulator::TapePulse> {
    bytes
        .iter()
        .flat_map(|&byte| {
            (0..8).flat_map(move |bit| {
                let period = if byte & (1 << bit) == 0 { 813_u32 } else { 435 } * 16;
                [
                    eightbit_dragon_emulator::TapePulse {
                        clocks: period / 2,
                        high: true,
                    },
                    eightbit_dragon_emulator::TapePulse {
                        clocks: period - period / 2,
                        high: false,
                    },
                ]
            })
        })
        .collect()
}

#[test]
fn basic_rom_board_keyboard_pia_transport_and_loaded_code_work_together() -> Result<(), String> {
    use eightbit_dragon_emulator::{Firmware, MAX_SLICE, Machine};
    let mut machine = Machine::with_firmware(Firmware::BasicDevelopment);
    for _ in 0..8 {
        machine.advance(MAX_SLICE)?;
    }
    let payload = [0xcc, 0, 165, 0xfd, 0x63, 0, 0x39];
    let mut cassette =
        eightbit_dragon_media::create_machine_code("NATIVE", &payload, 0x6000, 0x6000).unwrap();
    cassette.extend([0x55; 16]);
    machine.mount_cassette(cassette_pulses(&cassette))?;
    machine.type_text("CLOADM\"NATIVE\":EXEC:PRINT\"CASSETTE READY\"")?;
    for _ in 0..256 {
        machine.advance(MAX_SLICE)?;
    }
    assert_eq!(machine.peek(0x6301), 165);
    assert_eq!((machine.peek(0x9d), machine.peek(0x9e)), (0x60, 0));
    assert_eq!(machine.peek(0x81), 0);
    let screen = machine.snapshot().text;
    assert!(screen.contains("CASSETTE READY"));
    assert!(!screen.contains("?SN ERROR"));
    assert!(screen.contains("DEV>"));
    Ok(())
}

#[test]
fn basic_rom_board_missing_cassette_times_out_and_accepts_new_keyboard_input() -> Result<(), String>
{
    use eightbit_dragon_emulator::{Firmware, MAX_SLICE, Machine};
    let mut machine = Machine::with_firmware(Firmware::BasicDevelopment);
    for _ in 0..8 {
        machine.advance(MAX_SLICE)?;
    }
    machine.type_text("CLOADM\"MISSING\"")?;
    for _ in 0..192 {
        machine.advance(MAX_SLICE)?;
    }
    assert_eq!(machine.peek(0x81), 3);
    assert!(machine.snapshot().text.contains("?SN ERROR"));
    machine.type_text("POKE25345,85:PRINT\"RECOVERED\"")?;
    for _ in 0..96 {
        machine.advance(MAX_SLICE)?;
    }
    assert_eq!(machine.peek(0x6301), 85);
    assert!(machine.snapshot().text.contains("RECOVERED"));
    machine.reset();
    for _ in 0..8 {
        machine.advance(MAX_SLICE)?;
    }
    assert!(machine.snapshot().text.contains("DEV>"));
    Ok(())
}

#[test]
fn basic_rom_cloadm_streams_multiple_blocks_across_the_cassette_buffer_without_copy_aliasing() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    let mut tape = Vec::new();
    let mut header = b"BUFFER  ".to_vec();
    header.extend([2, 0, 0, 0x60, 0, 1, 0xdb]);
    append_synthetic_cassette_block(&mut tape, 0, &header);
    append_synthetic_cassette_block(&mut tape, 1, &[11, 22, 33, 44, 55, 66]);
    append_synthetic_cassette_block(&mut tape, 1, &[77, 88, 99, 111]);
    append_synthetic_cassette_block(&mut tape, 255, &[]);
    tape.extend([0x55; 16]);
    synthetic_tape(&mut memory, &tape);
    assert!(execute_direct_with_budget(
        &mut memory,
        &mut cpu,
        "CLOADM\"BUFFER\"",
        16_777_216
    ));
    assert_eq!(
        &memory.bytes[0x1db..0x1e5],
        &[11, 22, 33, 44, 55, 66, 77, 88, 99, 111]
    );
    assert_eq!(word(&memory, 0x7e), 0x1e5);
    assert_eq!(word(&memory, 0x9d), 0x6000);
    assert_eq!(memory.bytes[0x81], 0);
}

#[test]
fn basic_rom_cloadm_accepts_omitted_and_empty_filename_with_optional_offset() {
    for (source, address) in [
        ("CLOADM", 0x6000_usize),
        ("CLOADM\"\"", 0x6000),
        ("CLOADM,256", 0x6100),
    ] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        let mut tape =
            eightbit_dragon_media::create_machine_code("NATIVE", &[11, 22, 33], 0x6000, 0x6000)
                .unwrap();
        tape.extend([0x55; 16]);
        synthetic_tape(&mut memory, &tape);
        assert!(
            execute_direct_with_budget(&mut memory, &mut cpu, source, 16_777_216),
            "{source}"
        );
        assert_eq!(&memory.bytes[address..address + 3], &[11, 22, 33]);
        assert_eq!(word(&memory, 0x9d), u16::try_from(address).unwrap());
    }
}

fn synthetic_tokenised_lines(lines: &[(u16, &[u8])]) -> Vec<u8> {
    let mut program = Vec::new();
    for (number, body) in lines {
        // Deliberately stale, non-zero links: CLOAD must rebuild them at TXTTAB.
        program.extend([0x12, 0x34]);
        program.extend(number.to_be_bytes());
        program.extend(*body);
        program.push(0);
    }
    program.extend([0, 0]);
    program
}

#[test]
fn basic_rom_cload_relinks_original_tokenised_lines_and_runs_the_loaded_programme() {
    for source in [
        "CLOAD\"BASIC\":POKE25345,99:RUN",
        "CLOAD:RUN",
        "CLOAD\"\":RUN",
    ] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        let program =
            synthetic_tokenised_lines(&[(0, &[b'A', 0xcb, b'7']), (10, &[0x87, b'A', 0xc3, b'2'])]);
        let mut tape = eightbit_dragon_media::create_tokenised_basic("BASIC", &program).unwrap();
        tape.extend([0x55; 16]);
        synthetic_tape(&mut memory, &tape);
        assert!(
            execute_direct_with_budget(&mut memory, &mut cpu, source, 16_777_216),
            "{source}: status {}",
            memory.bytes[0x81]
        );
        assert!(
            memory.output.is_empty(),
            "CLOAD discards its command-line tail"
        );
        assert_eq!(memory.bytes[0x6301], 0);
        assert!(execute_direct(&mut memory, &mut cpu, "RUN"));
        assert_eq!(memory.output, b" 9 \r");
        assert_eq!(word(&memory, 0x1e01), 0x1e09);
        assert_eq!(word(&memory, 0x1e09), 0x1e12);
        assert_eq!(word(&memory, 0x1e12), 0);
        assert_eq!(word(&memory, 0x1b), 0x1e14);
        assert_eq!(numeric_scalar(&memory, [b'A', 0]), packed_integer(7));
        assert_eq!(memory.bytes[0xff21] & 8, 0);
        assert_eq!(cpu.registers.s, 0x7000);
    }
}

#[test]
fn basic_rom_cload_handles_multiple_blocks_and_clears_old_variables_and_data_cursor() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    assert!(execute_direct(&mut memory, &mut cpu, "A=99:DIM B(2)"));
    let bodies = [0x82, b'O', b'R', b'I', b'G', b'I', b'N', b'A', b'L'];
    let lines: Vec<_> = (1..=40)
        .map(|line| (line * 10, bodies.as_slice()))
        .collect();
    let program = synthetic_tokenised_lines(&lines);
    let mut tape = eightbit_dragon_media::create_tokenised_basic("MULTI", &program).unwrap();
    tape.extend([0x55; 16]);
    synthetic_tape(&mut memory, &tape);
    assert!(execute_direct_with_budget(
        &mut memory,
        &mut cpu,
        "CLOAD\"MULTI\"",
        33_554_432
    ));
    let end = 0x1e01 + u16::try_from(program.len()).unwrap();
    for pointer in [0x1b, 0x1d, 0x1f] {
        assert_eq!(word(&memory, pointer), end);
    }
    let mut address = 0x1e01;
    for (number, body) in lines {
        let next = address + u16::try_from(body.len() + 5).unwrap();
        assert_eq!(word(&memory, usize::from(address)), next);
        assert_eq!(word(&memory, usize::from(address) + 2), number);
        address = next;
    }
    assert_eq!(word(&memory, usize::from(address)), 0);
    assert_eq!(word(&memory, 0x33), 0x1e00);
    assert!(execute_direct(&mut memory, &mut cpu, "RUN"));
}

#[test]
fn basic_rom_cload_rejects_malformed_linked_bodies_and_leaves_a_valid_empty_programme() {
    let mut unsorted = synthetic_tokenised_lines(&[(20, b"A"), (10, b"B")]);
    let mut variants = vec![
        Vec::new(),
        vec![0],
        vec![0, 0, 1],
        vec![1, 1, 0, 10, 0],
        vec![1, 1, 0, 10, b'A', b'A', 1, 1],
        synthetic_tokenised_lines(&[(64000, b"A")]),
    ];
    variants.push(std::mem::take(&mut unsorted));
    for program in variants {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        let mut tape = eightbit_dragon_media::create_tokenised_basic("BAD", &program).unwrap();
        tape.extend([0x55; 16]);
        synthetic_tape(&mut memory, &tape);
        assert!(
            !execute_direct_with_budget(&mut memory, &mut cpu, "CLOAD\"BAD\"", 16_777_216),
            "malformed {program:?}"
        );
        assert_eq!(memory.bytes[0x81], 2);
        assert_eq!(&memory.bytes[0x1e01..0x1e03], &[0, 0]);
        for pointer in [0x1b, 0x1d, 0x1f] {
            assert_eq!(word(&memory, pointer), 0x1e03);
        }
        assert_eq!(memory.bytes[0xff21] & 8, 0);
        assert!(execute_direct(&mut memory, &mut cpu, "A=17"));
    }
}

#[test]
fn basic_rom_cload_preflights_programme_bounds_and_discards_checksum_failure() {
    let body = [0x82; 200];
    let program = synthetic_tokenised_lines(&[(10, &body), (20, &body)]);
    for checksum_failure in [false, true] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        if !checksum_failure {
            set_word(&mut memory, 0x21, 0x1f00);
        }
        let mut tape = eightbit_dragon_media::create_tokenised_basic("LIMIT", &program).unwrap();
        if checksum_failure {
            let parsed = eightbit_dragon_media::read_tokenised_basic(&tape).unwrap();
            let data = &parsed.blocks[1];
            tape[data.offset + 3 + data.data_length] ^= 1;
        }
        tape.extend([0x55; 16]);
        synthetic_tape(&mut memory, &tape);
        assert!(!execute_direct_with_budget(
            &mut memory,
            &mut cpu,
            "CLOAD\"LIMIT\"",
            33_554_432
        ));
        assert_eq!(memory.bytes[0x81], if checksum_failure { 1 } else { 2 });
        assert_eq!(word(&memory, 0x1e01), 0);
        assert_eq!(word(&memory, 0x1b), 0x1e03);
        assert_eq!(memory.bytes[0xff21] & 8, 0);
        assert_eq!(cpu.registers.s, 0x7000);
    }
}

#[test]
fn basic_rom_cload_rejects_offsets_before_tape_and_wrong_types_before_clearing() {
    for source in ["CLOAD,1", "CLOAD\"BASIC\",1"] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        assert!(!execute_direct(&mut memory, &mut cpu, source));
        assert_eq!(memory.input_cursor, 0);
        assert_eq!(memory.bytes[0xff21], 0);
    }
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    assert!(execute_direct(&mut memory, &mut cpu, "A=99"));
    let mut tape =
        eightbit_dragon_media::create_machine_code("WRONG", &[1], 0x6000, 0x6000).unwrap();
    tape.extend([0x55; 16]);
    synthetic_tape(&mut memory, &tape);
    assert!(!execute_direct_with_budget(
        &mut memory,
        &mut cpu,
        "CLOAD\"WRONG\"",
        16_777_216
    ));
    assert_eq!(numeric_scalar(&memory, [b'A', 0]), packed_integer(99));
    assert_eq!(memory.bytes[0x81], 2);
}

#[test]
fn basic_rom_operator_records_keep_deep_expression_space_and_caller_registers() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    set_word(&mut memory, 0x1f, 0x6d80);
    let source = format!("{}1", "1^".repeat(30));
    guest_tokens(&mut memory, &mut cpu, &source);
    let mut code = vec![0x8e, 2, 0xdd];
    call(&mut code, symbol("EVAL"));
    memory.bytes[0x2000..0x2000 + code.len()].copy_from_slice(&code);
    cpu.registers = Registers {
        pc: 0x2000,
        s: 0x7000,
        y: 0x1234,
        u: 0x2345,
        ..Registers::default()
    };
    let mut minimum_stack = 0x7000;
    let end = 0x2000 + u16::try_from(code.len()).unwrap();
    for _ in 0..4_194_304 {
        if cpu.registers.pc == end {
            break;
        }
        cpu.step(&mut memory).unwrap();
        minimum_stack = minimum_stack.min(cpu.registers.s);
    }
    assert_eq!(cpu.registers.pc, end);
    assert_eq!(cpu.registers.cc & C, 0);
    assert_eq!(cpu.registers.s, 0x7000);
    assert_eq!(cpu.registers.y, 0x1234);
    assert_eq!(cpu.registers.u, 0x2345);
    assert!(minimum_stack >= 0x6e00, "lowest stack {minimum_stack:04x}");
    assert_eq!(word(&memory, usize::from(symbol("STR_ROOTS"))), 0);
    assert_eq!(memory.bytes[usize::from(symbol("EX_DEPTH"))], 0);
}

#[test]
fn basic_rom_cload_from_a_running_programme_does_not_resume_overwritten_tokens() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    store_line(&mut memory, &mut cpu, 10, "CLOAD\"NEXT\":POKE25345,99");
    store_line(&mut memory, &mut cpu, 20, "POKE25345,100");
    let program =
        synthetic_tokenised_lines(&[(10, &[0x93, b'2', b'5', b'3', b'4', b'5', b',', b'8', b'5'])]);
    let mut tape = eightbit_dragon_media::create_tokenised_basic("NEXT", &program).unwrap();
    tape.extend([0x55; 16]);
    synthetic_tape(&mut memory, &tape);
    assert!(execute_direct_with_budget(
        &mut memory,
        &mut cpu,
        "RUN",
        16_777_216
    ));
    assert_eq!(memory.bytes[0x6301], 0);
    assert!(execute_direct(&mut memory, &mut cpu, "RUN"));
    assert_eq!(memory.bytes[0x6301], 85);
}

// Decode the emitted physical periods independently of the guest block writer.
// Inter-byte call overhead is well below the separation between 1/0 periods.
fn recorded_cassette(memory: &Memory) -> Vec<u8> {
    assert!(
        memory.signals.len() < memory.signal_limit,
        "capture overflow"
    );
    let stopped = memory
        .signals
        .iter()
        .rev()
        .find(|sample| sample.1 == 0xff21 && sample.2 & 8 == 0)
        .unwrap()
        .0;
    let rising: Vec<_> = memory
        .signals
        .iter()
        .filter(|sample| sample.1 == 0xff20 && sample.2 >> 2 >= 32)
        .map(|sample| sample.0)
        .chain([stopped])
        .collect();
    assert_eq!((rising.len() - 1) % 8, 0);
    rising
        .windows(2)
        .collect::<Vec<_>>()
        .chunks_exact(8)
        .map(|bits| {
            bits.iter().enumerate().fold(0, |byte, (bit, edge)| {
                let period = edge[1] - edge[0];
                assert!((380..=1200).contains(&period), "cassette period {period}");
                byte | if period < 624 { 1 << bit } else { 0 }
            })
        })
        .collect()
}

#[test]
fn basic_rom_csave_emits_valid_empty_single_and_multiple_block_files_and_reloads() {
    for lines in [0, 1, 7] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        // Keep this large programme clear of the synthetic caller at $2000.
        set_word(&mut memory, 0x19, 0x3000);
        for pointer in [0x1b, 0x1d, 0x1f] {
            set_word(&mut memory, pointer, 0x3002);
        }
        enable_guest_printer(&mut memory);
        for number in 1..=lines {
            store_line(
                &mut memory,
                &mut cpu,
                number * 10,
                &format!("REM{}", "ORIGINAL".repeat(24)),
            );
        }
        let programme = memory.bytes
            [usize::from(word(&memory, 0x19))..usize::from(word(&memory, 0x1b))]
            .to_vec();
        memory.bytes[0xd8..0xea].fill(0xa5);
        memory.signals.clear();
        memory.signal_limit = 131_072;
        assert!(execute_direct_with_budget(
            &mut memory,
            &mut cpu,
            "CSAVE\"NATIVE\":POKE25344,85",
            16_777_216
        ));
        assert_eq!(memory.bytes[0x6300], 85, "same-line continuation");
        assert_eq!(memory.bytes[0xff21] & 8, 0, "motor off");
        assert!(memory.bytes[0xd8..0xea].iter().all(|&b| b == 0xa5));
        let tape = recorded_cassette(&memory);
        let file = eightbit_dragon_media::read_tokenised_basic(&tape).unwrap();
        assert_eq!(file.filename, "NATIVE");
        assert_eq!(file.program, programme);
        assert_eq!(file.data_blocks, programme.len().div_ceil(255));
        assert_eq!(file.metadata.address_bytes, [0; 4]);
        assert_eq!(file.metadata.ascii_flag, 0);
        let mut input = tape;
        input.extend([0x55; 16]);
        assert!(execute_direct(&mut memory, &mut cpu, "NEW"));
        synthetic_tape(&mut memory, &input);
        assert!(execute_direct_with_budget(
            &mut memory,
            &mut cpu,
            "CLOAD\"NATIVE\"",
            16_777_216
        ));
        assert_eq!(
            &memory.bytes[usize::from(word(&memory, 0x19))..usize::from(word(&memory, 0x1b))],
            programme
        );
        assert_eq!(memory.bytes[0xff21] & 8, 0);
    }
}

#[test]
fn basic_rom_csave_rejects_bad_arguments_and_corrupt_programmes_before_output() {
    for source in [
        "CSAVE123",
        "CSAVE\"TOOLONGXX\"",
        "CSAVE\"NATIVE\",B",
        "CSAVE\"NATIVE\",1",
    ] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert!(memory.signals.is_empty(), "{source}");
        assert!(execute_direct(&mut memory, &mut cpu, "A=PEEK(65535)"));
    }
    for (address, value) in [(0x19, 0x200), (0x1b, 0x7fff), (0x1e01, 0x1e00)] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        set_word(&mut memory, address, value);
        assert!(!execute_direct(&mut memory, &mut cpu, "CSAVE\"NATIVE\""));
        assert!(memory.signals.is_empty());
    }
}

#[test]
fn basic_rom_csavem_preserves_inclusive_ranges_metadata_and_multiple_blocks() {
    for length in [1_u16, 255, 256, 514] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        let payload: Vec<_> = (0..length).map(|index| index.to_le_bytes()[0]).collect();
        memory.bytes[0x6000..0x6000 + payload.len()].copy_from_slice(&payload);
        set_word(&mut memory, 0x9d, 0x1234);
        memory.signal_limit = 131_072;
        let source = format!(
            "CSAVEM\"NATIVE\",24576,{},65535:POKE25344,85",
            24576 + length - 1
        );
        assert!(execute_direct_with_budget(
            &mut memory,
            &mut cpu,
            &source,
            16_777_216
        ));
        assert_eq!(memory.bytes[0x6300], 85);
        assert_eq!(word(&memory, 0x9d), 0x1234, "saving keeps the loaded entry");
        assert_eq!(memory.bytes[0xff21] & 8, 0);
        let tape = recorded_cassette(&memory);
        let file = eightbit_dragon_media::read_machine_code(&tape).unwrap();
        assert_eq!(file.filename, "NATIVE");
        assert_eq!(file.execution_address, 65535);
        assert_eq!(file.load_address, 0x6000);
        assert_eq!(file.program, payload);
        assert_eq!(file.data_blocks, payload.len().div_ceil(255));
        assert_eq!(&memory.bytes[0x6000..0x6000 + payload.len()], payload);
        memory.bytes[0x6000..0x6000 + payload.len()].fill(0xa5);
        let mut input = tape;
        input.extend([0x55; 16]);
        synthetic_tape(&mut memory, &input);
        assert!(execute_direct_with_budget(
            &mut memory,
            &mut cpu,
            "CLOADM\"NATIVE\"",
            16_777_216
        ));
        assert_eq!(&memory.bytes[0x6000..0x6000 + payload.len()], payload);
        assert_eq!(word(&memory, 0x9d), 65535);
    }
}

#[test]
fn basic_rom_csavem_rejects_bad_ranges_and_syntax_before_enabling_the_motor() {
    for arguments in [
        ",-1,24576,24576",
        ",24577,24576,24576",
        ",0,65536,0",
        ",24576,24576,-1",
        ",24576,24576,65536",
        ",24576,24576",
        ",24576",
        ",24576,24576,24576,0",
        ",24576,24576,\"A\"",
    ] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        let source = format!("CSAVEM\"NATIVE\"{arguments}");
        assert!(!execute_direct(&mut memory, &mut cpu, &source), "{source}");
        assert!(memory.signals.is_empty(), "{source}");
        assert!(execute_direct(
            &mut memory,
            &mut cpu,
            "A=HEX$(65535)=\"FFFF\""
        ));
        assert_eq!(numeric_scalar(&memory, [b'A', 0]), packed_integer(-1));
    }
}

#[test]
fn basic_rom_csavem_reads_original_rom_and_the_final_guest_address() {
    for (start, length) in [(0x7fff_u16, 2_u16), (0xbffe, 4), (0xfffd, 3)] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        memory.bytes[0x7fff] = 0xa5;
        let expected =
            memory.bytes[usize::from(start)..usize::from(start) + usize::from(length)].to_vec();
        memory.signal_limit = 16384;
        let source = format!(
            "CSAVEM\"BUS\",{start},{},32768:POKE25344,85",
            u32::from(start) + u32::from(length) - 1
        );
        assert!(execute_direct_with_budget(
            &mut memory,
            &mut cpu,
            &source,
            4_194_304
        ));
        let file = eightbit_dragon_media::read_machine_code(&recorded_cassette(&memory)).unwrap();
        assert_eq!(file.program, expected);
        assert_eq!(file.load_address, start);
        assert_eq!(file.execution_address, 32768);
        assert_eq!(memory.bytes[0x6300], 85);
        assert_eq!(memory.bytes[0xff21] & 8, 0);
    }
}

#[test]
fn basic_rom_csavem_full_bus_count_does_not_treat_65536_as_empty() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    // Instrument only our byte writer to isolate full-range counter/framing
    // logic from 65,536 bytes of waveform delay. Short saves test the actual DAC.
    let writer = usize::from(symbol("TAPE_BYTE_OUT"));
    let spy = [
        0x34, 5, 0xb7, 0xff, 2, 0xf6, 0xff, 0x20, 0xc4, 0xfd, 0xf7, 0xff, 0x20, 0xca, 2, 0xf7,
        0xff, 0x20, 0x35, 0x85,
    ];
    memory.bytes[writer..writer + spy.len()].copy_from_slice(&spy);
    let expected_rom = memory.bytes[0x8000..0xc000].to_vec();
    assert!(
        execute_direct_with_budget(
            &mut memory,
            &mut cpu,
            "CSAVEM\"FULL\",0,65535,32768",
            4_194_304
        ),
        "status {}, output {}, pointer {}, boundary {}",
        memory.bytes[0x81],
        memory.output.len(),
        word(&memory, 0x7e),
        word(&memory, usize::from(symbol("STATEMENT_SP")))
    );
    let file = eightbit_dragon_media::read_machine_code(&memory.output).unwrap();
    assert_eq!(file.program.len(), 65536);
    assert_eq!(file.data_blocks, 258);
    assert_eq!(file.load_address, 0);
    assert_eq!(file.execution_address, 32768);
    assert_eq!(file.program[0x8000..0xc000], expected_rom);
    assert_eq!(file.program[65535], memory.bytes[65535]);
    assert_eq!(memory.bytes[0xff21] & 8, 0);
}

#[test]
fn basic_rom_shared_optional_delimiter_preserves_flags_registers_and_cursor() {
    for (text, offset, equal) in [(b" ,A".as_slice(), 2, true), (b" A".as_slice(), 1, false)] {
        for flags in 0..=u8::MAX {
            let mut memory = Memory::new();
            let mut cpu = Cpu::default();
            memory.bytes[0x6000..0x6000 + text.len()].copy_from_slice(text);
            cpu.registers.x = 0x6000;
            cpu.registers.y = 0xa55a;
            cpu.registers.u = 0x5aa5;
            cpu.registers.b = 0x42;
            cpu.registers.cc = flags;
            let mut code = Vec::new();
            call(&mut code, symbol("EX_OPTIONAL_COMMA"));
            memory.execute(&mut cpu, &code);
            assert_eq!(cpu.registers.x, 0x6000 + offset);
            assert_eq!(cpu.registers.y, 0xa55a);
            assert_eq!(cpu.registers.u, 0x5aa5);
            assert_eq!(cpu.registers.b, 0x42);
            // CMP preserves E/F/H/I; its successful repeat must restore every
            // remaining flag, including Z after LEAX changes the cursor.
            let expected = (flags & 0xf0) | if equal { eightbit_m6809_core::Z } else { 0 };
            assert_eq!(cpu.registers.cc, expected);
            assert_eq!(cpu.registers.s, 0x7000);
        }
    }
}

#[test]
fn basic_rom_cassette_block_keeps_bit_periods_across_byte_boundaries() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    let payload = [0, 255, 129, 85, 1, 128, 254, 127];
    memory.bytes[0x6000..0x6008].copy_from_slice(&payload);
    memory.bytes[0x7c] = 1;
    memory.bytes[0x7d] = 8;
    set_word(&mut memory, 0x7e, 0x6000);
    let mut code = Vec::new();
    call(&mut code, 0xb999);
    memory.execute_with_budget(&mut cpu, &code, 262_144);
    let rising: Vec<_> = memory
        .signals
        .iter()
        .filter(|s| s.1 == 0xff20 && s.2 >> 2 >= 32)
        .map(|s| s.0)
        .collect();
    let mut expected = vec![0x55, 0x3c, 1, 8];
    expected.extend(payload);
    expected.extend([
        payload
            .iter()
            .fold(9_u8, |sum, byte| sum.wrapping_add(*byte)),
        0x55,
    ]);
    assert_eq!(rising.len(), expected.len() * 8);
    for (index, edge) in rising.windows(2).enumerate() {
        let one = expected[index / 8] & (1 << (index % 8)) != 0;
        let period = edge[1] - edge[0];
        let accepted = if one { 380..=560 } else { 750..=930 };
        assert!(
            accepted.contains(&period),
            "byte {} bit {}: {period} E cycles",
            index / 8,
            index % 8
        );
    }
}

#[test]
fn basic_rom_cassette_block_defers_pending_irq_until_every_byte_is_complete() {
    use eightbit_m6809_core::{F, I};
    for masks in [0, I, F, I | F] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        let payload = [0x87, 0x8a, 0xff];
        memory.bytes[0x6100..0x6103].copy_from_slice(&payload);
        // Original delayed IRQ acknowledgement. Its delay must never lengthen
        // a payload byte's final one bit into a zero at 9.6 kHz capture.
        memory.bytes[0x6000..0x600b]
            .copy_from_slice(&[0xc6, 10, 0x5a, 0x26, 0xfd, 0x86, 1, 0xb7, 0x63, 0, 0x3b]);
        set_word(&mut memory, 0xfff8, 0x6000);
        memory.bytes[0x7c] = 1;
        memory.bytes[0x7d] = 3;
        set_word(&mut memory, 0x7e, 0x6100);
        memory.irq_at_rising = Some(40);
        cpu.registers.cc = masks;
        let mut code = Vec::new();
        call(&mut code, 0xb999);
        memory.execute_with_budget(&mut cpu, &code, 262_144);
        assert_eq!(memory.rising_edges, 72);
        assert_eq!(memory.irq_ack_edges, (masks & I == 0).then_some(72));
        assert_eq!(cpu.registers.cc & (I | F), masks);
        assert_eq!(cpu.registers.cc & C, 0);
        let rising: Vec<_> = memory
            .signals
            .iter()
            .filter(|signal| signal.1 == 0xff20 && signal.2 >> 2 >= 32)
            .map(|signal| signal.0)
            .collect();
        assert!(rising[40] - rising[39] <= 470);
    }
}

#[test]
fn basic_rom_csave_accepts_omitted_and_empty_names_and_csavem_reaches_last_ram_byte() {
    for source in ["CSAVE", "CSAVE\"\"", "CSAVE\"EIGHTBIT\""] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        memory.signal_limit = 16384;
        assert!(execute_direct_with_budget(
            &mut memory,
            &mut cpu,
            source,
            4_194_304
        ));
        let tape = recorded_cassette(&memory);
        let file = eightbit_dragon_media::read_tokenised_basic(&tape).unwrap();
        assert_eq!(
            file.filename,
            if source.contains("EIGHTBIT") {
                "EIGHTBIT"
            } else {
                ""
            }
        );
        assert_eq!(file.program, [0, 0]);
    }
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    memory.signal_limit = 16384;
    memory.bytes[0x7fff] = 0xa5;
    assert!(execute_direct_with_budget(
        &mut memory,
        &mut cpu,
        "CSAVEM\"TOP\",32767,32767,0",
        4_194_304
    ));
    let file = eightbit_dragon_media::read_machine_code(&recorded_cassette(&memory)).unwrap();
    assert_eq!(file.load_address, 0x7fff);
    assert_eq!(file.execution_address, 0);
    assert_eq!(file.program, [0xa5]);
}

fn synthetic_ascii_cassette(source: &[u8], block_size: usize) -> Vec<u8> {
    let mut tape = Vec::new();
    let mut header = b"TEXT    ".to_vec();
    header.extend([0, 255, 255, 0, 0, 0, 0]);
    append_synthetic_cassette_block(&mut tape, 0, &header);
    for block in source.chunks(block_size) {
        append_synthetic_cassette_block(&mut tape, 1, block);
    }
    append_synthetic_cassette_block(&mut tape, 255, &[]);
    tape.extend([0x55; 16]);
    tape
}

// Gapped output has arbitrary low silence between blocks. Decode each complete
// high half-wave, independently of the writer and its rising-edge period test.
fn recorded_gapped_cassette(memory: &Memory) -> Vec<u8> {
    assert!(memory.signals.len() < memory.signal_limit);
    let mut high = None;
    let mut bits = Vec::new();
    for &(cycle, address, value) in &memory.signals {
        if address != 0xff20 {
            continue;
        }
        if value >> 2 >= 32 {
            assert!(high.replace(cycle).is_none());
        } else if let Some(start) = high.take() {
            let width = cycle - start;
            assert!((180..=450).contains(&width), "half-wave {width}");
            bits.push(width < 312);
        }
    }
    assert!(high.is_none());
    assert_eq!(bits.len() % 8, 0);
    bits.chunks_exact(8)
        .map(|byte| {
            byte.iter()
                .enumerate()
                .fold(0_u8, |value, (bit, set)| value | (u8::from(*set) << bit))
        })
        .collect()
}

fn original_cassette_blocks(tape: &[u8]) -> Vec<(u8, Vec<u8>)> {
    let mut cursor = 0;
    let mut result = Vec::new();
    while cursor < tape.len() {
        while cursor < tape.len() && tape[cursor] == 0x55 {
            cursor += 1;
        }
        if cursor == tape.len() {
            break;
        }
        assert_eq!(tape[cursor], 0x3c);
        let kind = tape[cursor + 1];
        let length = usize::from(tape[cursor + 2]);
        let payload = tape[cursor + 3..cursor + 3 + length].to_vec();
        let checksum = payload.iter().fold(
            kind.wrapping_add(u8::try_from(length).unwrap()),
            |sum, byte| sum.wrapping_add(*byte),
        );
        assert_eq!(tape[cursor + 3 + length], checksum);
        assert_eq!(tape[cursor + 4 + length], 0x55);
        result.push((kind, payload));
        cursor += 5 + length;
    }
    result
}

#[test]
fn basic_rom_ascii_csave_streams_exact_listing_and_reloads_across_blocks() {
    for line_count in [0, 2, 7] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        set_word(&mut memory, 0x19, 0x3000);
        for pointer in [0x1b, 0x1d, 0x1f] {
            set_word(&mut memory, pointer, 0x3002);
        }
        let mut source = String::from("\r");
        for line in 1..=line_count {
            let body = format!("REM{}", "ORIGINAL".repeat(23));
            store_line(&mut memory, &mut cpu, line * 10, &body);
            source.push_str(&format!("{} {body}\r", line * 10));
        }
        let programme = memory.bytes[0x3000..usize::from(word(&memory, 0x1b))].to_vec();
        memory.bytes[0xd8..0xea].fill(0xa5);
        memory.signals.clear();
        memory.signal_limit = 131_072;
        assert!(execute_direct_with_budget(
            &mut memory,
            &mut cpu,
            "CSAVE\"TEXT\",A:POKE25344,85",
            32_000_000
        ));
        assert_eq!(memory.bytes[0x6300], 85);
        assert_eq!(memory.bytes[0xff21] & 8, 0);
        assert_eq!(memory.bytes[usize::from(symbol("FORMAT_BUFFER"))], 0);
        assert_eq!(&memory.bytes[0xd8..0xea], &[0xa5; 18]);
        let tape = recorded_gapped_cassette(&memory);
        let blocks = original_cassette_blocks(&tape);
        assert_eq!(
            blocks.first(),
            Some(&(0, b"TEXT    \0\xff\xff\0\0\0\0".to_vec()))
        );
        assert_eq!(blocks.last(), Some(&(255, Vec::new())));
        let payload: Vec<u8> = blocks
            .iter()
            .filter(|(kind, _)| *kind == 1)
            .flat_map(|(_, data)| data.iter().copied())
            .collect();
        assert_eq!(payload, source.as_bytes());
        assert_eq!(blocks.len() - 2, source.len().div_ceil(255));
        assert_eq!(
            &memory.bytes[0x3000..usize::from(word(&memory, 0x1b))],
            programme
        );
        assert!(execute_direct(&mut memory, &mut cpu, "NEW"));
        synthetic_tape(&mut memory, &tape);
        assert!(execute_direct_with_budget(
            &mut memory,
            &mut cpu,
            "CLOAD\"TEXT\"",
            32_000_000
        ));
        assert_eq!(
            &memory.bytes[0x3000..usize::from(word(&memory, 0x1b))],
            programme
        );
    }
}

#[test]
fn basic_rom_ascii_cload_handles_partial_lines_crlf_eof_tail_and_replacements() {
    for block_size in [1, 7, 128, 255] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        store_line(&mut memory, &mut cpu, 99, "A=99");
        let tape = synthetic_ascii_cassette(b"\r\n10 A=7\r\n20 A=A+2\r10 A=8\r30 END", block_size);
        synthetic_tape(&mut memory, &tape);
        assert!(
            execute_direct_with_budget(
                &mut memory,
                &mut cpu,
                "CLOAD\"TEXT\":POKE25344,99",
                32_000_000
            ),
            "block size {block_size}"
        );
        assert_eq!(memory.bytes[0x6300], 0, "discard overwritten direct tail");
        assert_eq!(memory.bytes[0xff21] & 8, 0);
        assert_eq!(word(&memory, 0x1b), word(&memory, 0x1d));
        assert_eq!(word(&memory, 0x1d), word(&memory, 0x1f));
        let first = usize::from(word(&memory, 0x19));
        let second = usize::from(word(&memory, first));
        let terminal = usize::from(word(&memory, second));
        assert_eq!(word(&memory, first + 2), 10);
        assert_eq!(word(&memory, second + 2), 20);
        assert_eq!(
            word(&memory, terminal),
            0,
            "unterminated line 30 is discarded"
        );
        assert!(execute_direct(&mut memory, &mut cpu, "RUN"));
        assert_eq!(numeric_scalar(&memory, [b'A', 0]), packed_integer(10));
    }
}

#[test]
fn basic_rom_ascii_cload_bad_source_and_line_overflow_leave_new_and_recover() {
    for source in [
        b"10 A=7\rBAD\r".to_vec(),
        b"10 A=7\r64000 END\r".to_vec(),
        b"10 A=7\r20 A=\0HIDDEN\r".to_vec(),
        format!("10 A=7\r20 REM{}\r", "X".repeat(251)).into_bytes(),
        b"10 A=7\r20 \x80\r".to_vec(),
    ] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        synthetic_tape(&mut memory, &synthetic_ascii_cassette(&source, 17));
        assert!(!execute_direct_with_budget(
            &mut memory,
            &mut cpu,
            "CLOAD\"TEXT\"",
            32_000_000
        ));
        assert_eq!(memory.bytes[0xff21] & 8, 0);
        assert_eq!(memory.bytes[0x81], 2);
        assert_eq!(word(&memory, 0x1b), word(&memory, 0x19) + 2);
        assert_eq!(word(&memory, 0x1b), word(&memory, 0x1f));
        assert_eq!(word(&memory, usize::from(symbol("STR_ROOTS"))), 0);
        assert!(execute_direct(&mut memory, &mut cpu, "A=7"));
        assert_eq!(numeric_scalar(&memory, [b'A', 0]), packed_integer(7));
    }
}

#[test]
fn basic_rom_ascii_cload_checksum_failure_leaves_new_and_recovers() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    let mut tape = synthetic_ascii_cassette(b"10 A=7\r20 END\r", 255);
    // Header occupies 128 leader bytes and its complete 21-byte frame.
    let checksum = 149 + 128 + 4 + b"10 A=7\r20 END\r".len();
    tape[checksum] ^= 1;
    synthetic_tape(&mut memory, &tape);
    assert!(!execute_direct_with_budget(
        &mut memory,
        &mut cpu,
        "CLOAD\"TEXT\"",
        32_000_000
    ));
    assert_eq!(memory.bytes[0x81], 1);
    assert_eq!(memory.bytes[0xff21] & 8, 0);
    assert_eq!(word(&memory, 0x1b), word(&memory, 0x19) + 2);
    assert!(execute_direct(&mut memory, &mut cpu, "A=7"));
    assert_eq!(numeric_scalar(&memory, [b'A', 0]), packed_integer(7));
}

#[test]
fn basic_rom_ascii_line_limit_keeps_private_state_and_programme_bounds() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    memory.bytes[0xd8..0xea].fill(0xa5);
    let mut source = format!("10 REM{}", "X".repeat(245));
    assert_eq!(source.len(), 251);
    source.push('\r');
    synthetic_tape(
        &mut memory,
        &synthetic_ascii_cassette(source.as_bytes(), 128),
    );
    assert!(execute_direct_with_budget(
        &mut memory,
        &mut cpu,
        "CLOAD\"TEXT\"",
        32_000_000
    ));
    assert_eq!(&memory.bytes[0xd8..0xea], &[0xa5; 18]);
    assert_eq!(memory.bytes[usize::from(symbol("EX_DEPTH"))], 0);
    assert_eq!(word(&memory, usize::from(symbol("STR_ROOTS"))), 0);
    assert!(execute_direct(&mut memory, &mut cpu, "LIST"));
    assert_eq!(memory.output, source.as_bytes());
}

#[test]
fn basic_rom_run_filename_expressions_load_before_starting_and_reject_stale_programmes() {
    for command in ["RUN\"TEXT\"", "RUN M$", "RUN LEFT$(M$,4)"] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        store_line(&mut memory, &mut cpu, 10, "POKE25344,99");
        assert!(execute_direct(&mut memory, &mut cpu, "M$=\"TEXT\""));
        synthetic_tape(
            &mut memory,
            &synthetic_ascii_cassette(b"10 POKE25344,85\r20 END\r", 7),
        );
        assert!(
            execute_direct_with_budget(&mut memory, &mut cpu, command, 32_000_000),
            "{command}"
        );
        assert_eq!(memory.bytes[0x6300], 85);
        assert_eq!(memory.bytes[0xff21] & 8, 0);
    }
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    store_line(&mut memory, &mut cpu, 10, "POKE25344,99");
    synthetic_tape(&mut memory, &synthetic_ascii_cassette(b"BAD\r", 255));
    assert!(!execute_direct_with_budget(
        &mut memory,
        &mut cpu,
        "RUN\"TEXT\"",
        32_000_000
    ));
    assert_eq!(
        memory.bytes[0x6300], 0,
        "a failed load must never run the old programme"
    );
    assert_eq!(memory.bytes[0xff21] & 8, 0);
}

#[test]
fn basic_rom_loads_host_ascii_wav_through_the_physical_cassette_entry() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    let source = "10 POKE25344,85\n20 PRINT\"ASCII READY\"\n30 END\n";
    let cas = eightbit_dragon_media::create_ascii_basic("TEXT", source).unwrap();
    let wav = eightbit_dragon_media::encode_cassette_wav(&cas).unwrap();
    // Independently sample the generated PCM signal into PIA edges. Do not use
    // the host cassette decoder or insert programme bytes directly into RAM.
    memory.input_signal.push((memory.cycles, false));
    let base = memory.cycles + 512;
    let mut previous = false;
    for (frame, bytes) in wav[44..].chunks_exact(2).enumerate() {
        let level = i16::from_le_bytes([bytes[0], bytes[1]]) > 0;
        if level != previous {
            let time = u64::try_from(frame).unwrap() * 14_318_180 / (16 * 44_100);
            memory.input_signal.push((base + time, level));
            previous = level;
        }
    }
    assert!(execute_direct_with_budget(
        &mut memory,
        &mut cpu,
        "CLOAD\"TEXT\":POKE25344,1",
        16_777_216
    ));
    assert_eq!(
        memory.bytes[0x6300], 0,
        "overwritten direct tail must not run"
    );
    assert_eq!(memory.bytes[0xff21] & 8, 0);
    assert!(execute_direct(&mut memory, &mut cpu, "RUN"));
    assert_eq!(memory.bytes[0x6300], 85);
    assert!(
        memory
            .output
            .windows(b"ASCII READY".len())
            .any(|part| part == b"ASCII READY")
    );
}

#[test]
fn basic_rom_compact_constant_pool_loads_exact_values_and_preserves_caller_frame() {
    let constants = [
        [129, 0x80, 0, 0, 0, 0],
        [130, 0x80, 0, 0, 0, 0],
        [132, 0xa0, 0, 0, 0, 0x20],
        [128, 0xb1, 0x72, 0x17, 0xf8, 0x31],
        [128, 0x80, 0, 0, 0, 0],
        [130, 0xc9, 0x0f, 0xda, 0xa2, 0x49],
        [131, 0xc9, 0x0f, 0xda, 0xa2, 0x49],
        [129, 0xc9, 0x0f, 0xda, 0xa2, 0x49],
        [128, 0xc9, 0x0f, 0xda, 0xa2, 0x49],
    ];
    for (operation, destination, untouched) in [("FP_KLOAD", 0x4f, 0x5c), ("FP_KARG", 0x5c, 0x4f)] {
        for (index, expected) in constants.iter().enumerate() {
            let mut memory = Memory::new();
            let mut cpu = Cpu::default();
            memory.bytes[0x4f..0x55].fill(0xa5);
            memory.bytes[0x5c..0x62].fill(0xa5);
            memory.bytes[0xd8..0xea].fill(0x5a);
            cpu.registers.x = 0x3456;
            cpu.registers.u = 0x5678;
            let mut code = Vec::new();
            call(&mut code, symbol("FP_RUN"));
            code.extend([
                u8::try_from(symbol(operation)).unwrap() + u8::try_from(index).unwrap(),
                0,
            ]);
            memory.execute(&mut cpu, &code);
            assert_eq!(
                &memory.bytes[destination..destination + 6],
                expected,
                "{operation} {index}"
            );
            assert_eq!(&memory.bytes[untouched..untouched + 6], &[0xa5; 6]);
            assert_eq!(&memory.bytes[0xd8..0xea], &[0x5a; 18]);
            assert_eq!(
                (cpu.registers.s, cpu.registers.x, cpu.registers.u),
                (0x7000, 0x3456, 0x5678)
            );
            assert_eq!(cpu.registers.cc & C, 0);
        }
    }
}

#[test]
fn basic_rom_compact_decimal_digits_preserve_all_fac_bits_and_native_frame() {
    for digit in 0..=9 {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        let original = [245, 0xa5, 0x12, 0x34, 0x56, 0xa5];
        memory.bytes[0x4f..0x55].copy_from_slice(&original);
        cpu.registers.b = digit;
        cpu.registers.x = 0x3456;
        cpu.registers.u = 0x5678;
        let mut code = Vec::new();
        call(&mut code, symbol("ARG_DIGIT"));
        memory.execute(&mut cpu, &code);
        assert_eq!(&memory.bytes[0x4f..0x55], &original);
        let mut expected = Memory::new();
        unpack_to(&mut expected, 0x5c, packed_integer(i16::from(digit)));
        assert_eq!(&memory.bytes[0x5c..0x62], &expected.bytes[0x5c..0x62]);
        assert_eq!(
            (cpu.registers.s, cpu.registers.x, cpu.registers.u),
            (0x7000, 0x3456, 0x5678)
        );
        assert_eq!(cpu.registers.cc & C, 0);
        assert!(
            memory.cycles <= 450,
            "digit conversion used {} cycles",
            memory.cycles
        );
    }
}

#[test]
fn basic_rom_compact_semigraphics_address_covers_every_cell_and_subpixel() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    let mut code = Vec::new();
    call(&mut code, symbol("SG_ADDRESS"));
    for y in 0_u8..32 {
        for x in 0_u8..64 {
            memory.bytes[0xc4] = x;
            memory.bytes[0xc6] = y;
            memory.execute(&mut cpu, &code);
            assert_eq!(cpu.registers.cc & C, 0);
            assert_eq!(
                cpu.registers.y,
                0x400 + u16::from(y / 2) * 32 + u16::from(x / 2)
            );
            assert_eq!(cpu.registers.b, 8 >> ((y % 2) * 2 + x % 2));
        }
    }
    for (x, y) in [(64, 0), (255, 0), (0, 32), (0, 255)] {
        memory.bytes[0xc4] = x;
        memory.bytes[0xc6] = y;
        memory.execute(&mut cpu, &code);
        assert_ne!(cpu.registers.cc & C, 0);
    }
}

#[test]
fn basic_rom_compact_decimal_layout_handles_fixed_scientific_and_rounding_boundaries() {
    for (value, expected) in [
        (0.0, " 0 "),
        (-0.125, "-.125 "),
        (0.01, " .01 "),
        (0.001, " 1E-03 "),
        (100_000_000.0, " 100000000 "),
        (1_000_000_000.0, " 1E+09 "),
        (99_999_999.5, " 99999999.5 "),
        (999_999_999.75, " 1E+09 "),
        (1.0e-38, " 1E-38 "),
        (1.0e37, " 1E+37 "),
    ] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        enable_guest_printer(&mut memory);
        unpack_to(&mut memory, 0x4f, packed_number(value));
        let mut code = Vec::new();
        call(&mut code, symbol("PRINT_NUMBER"));
        memory.execute_with_budget(&mut cpu, &code, 1_048_576);
        assert_eq!(memory.output, expected.as_bytes(), "{value}");
        assert_eq!(cpu.registers.s, 0x7000);
    }
}

#[test]
fn basic_rom_decimal_display_matches_independent_oracle_across_every_binary_exponent() {
    #[derive(serde::Deserialize)]
    struct Case {
        packed: [u8; 5],
        display: String,
    }
    #[derive(serde::Deserialize)]
    struct Fixtures {
        cases: Vec<Case>,
    }
    let fixtures: Fixtures =
        serde_json::from_str(include_str!("fixtures/decimal-display.json")).unwrap();
    assert_eq!(fixtures.cases.len(), 1530);
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    enable_guest_printer(&mut memory);
    let mut code = Vec::new();
    call(&mut code, symbol("PRINT_NUMBER"));
    for case in fixtures.cases {
        memory.output.clear();
        unpack_to(&mut memory, 0x4f, case.packed);
        memory.execute_with_budget(&mut cpu, &code, 1_048_576);
        assert_eq!(
            memory.output,
            case.display.as_bytes(),
            "packed {:?}",
            case.packed
        );
        assert_eq!(cpu.registers.s, 0x7000);
    }
}

#[test]
fn basic_rom_compact_key_mapping_preserves_every_matrix_position_shift_and_alpha_lock() {
    let base = b"0123456789:;,-./@ABCDEFGHIJKLMNOPQRSTUVWXYZ^\n\x08\x09 \r\x0c\x03\0\0\0\0\0";
    assert_eq!(base.len(), 56);
    for (index, &unshifted) in base.iter().enumerate() {
        for shift in [false, true] {
            for alpha_lock in [false, true] {
                let mut memory = Memory::new();
                let mut cpu = Cpu::default();
                let mut keys = [0; 8];
                keys[index % 8] |= 1 << (index / 8);
                if shift {
                    keys[7] |= 64;
                }
                memory.key_matrix = Some(keys);
                memory.bytes[0x167] = 0x39;
                memory.bytes[0x149] = if alpha_lock { 255 } else { 0 };
                memory.bytes[0x150..0x15a].fill(255);
                cpu.registers.b = 0xa5;
                cpu.registers.x = 0x3456;
                cpu.registers.y = 0x4567;
                cpu.registers.u = 0x5678;
                let mut code = Vec::new();
                call(&mut code, 0x8006);
                memory.execute(&mut cpu, &code);
                let expected = if unshifted.is_ascii_uppercase() {
                    if shift == alpha_lock {
                        unshifted + 32
                    } else {
                        unshifted
                    }
                } else if shift && unshifted == b'0' {
                    0
                } else if shift && unshifted == b'@' {
                    19
                } else if shift && (43..50).contains(&index) {
                    [95, 91, 21, 93, 32, 13, 92][index - 43]
                } else if shift && (b','..=b'/').contains(&unshifted) {
                    unshifted + 16
                } else if shift && (b'0'..=b';').contains(&unshifted) {
                    unshifted - 16
                } else {
                    unshifted
                };
                assert_eq!(
                    cpu.registers.a, expected,
                    "matrix {index}, shift {shift}, alpha {alpha_lock}"
                );
                assert_eq!(memory.bytes[0x87], expected);
                let expected_lock = if alpha_lock ^ (shift && unshifted == b'0') {
                    255
                } else {
                    0
                };
                assert_eq!(memory.bytes[0x149], expected_lock);
                assert_eq!(
                    cpu.registers.cc & eightbit_m6809_core::Z != 0,
                    expected == 0
                );
                assert_eq!(
                    (
                        cpu.registers.b,
                        cpu.registers.x,
                        cpu.registers.y,
                        cpu.registers.u
                    ),
                    (0xa5, 0x3456, 0x4567, 0x5678)
                );
                assert_eq!(memory.bytes[0xff02], 255);
            }
        }
    }
}

#[test]
fn basic_rom_shared_mux_selector_changes_only_the_two_selection_bits() {
    for first in [0, 4, 0x34, 0x38, 0xff] {
        for second in [0, 4, 0x34, 0x38, 0xff] {
            for selection in 0_u8..4 {
                let mut memory = Memory::new();
                let mut cpu = Cpu::default();
                memory.bytes[0xff01] = first;
                memory.bytes[0xff03] = second;
                memory.bytes[0xff23] = 0x35;
                cpu.registers.b = selection;
                let mut code = Vec::new();
                call(&mut code, symbol("SELECT_MUX"));
                memory.execute(&mut cpu, &code);
                assert_eq!(memory.bytes[0xff01], (first & !8) | ((selection & 1) << 3));
                assert_eq!(memory.bytes[0xff03], (second & !8) | ((selection & 2) << 2));
                assert_eq!(memory.bytes[0xff23], 0x35);
                assert_eq!(cpu.registers.b, selection);
            }
        }
    }
}

#[test]
fn basic_rom_decimal_scratch_rejects_stack_collision_before_output_and_recovers() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    unpack_to(&mut memory, 0x4f, packed_number(999_999_999.75));
    memory.bytes[0x6f80..0x6fa0].fill(0xa5);
    set_word(&mut memory, 0x1f, 0x6fa0);
    let mut wrapper = vec![0x34, 0x66, 0x10, 0x8e]; // Save D/Y/U, select native body.
    wrapper.extend(symbol("PRINT_NUMBER").to_be_bytes());
    wrapper.push(0x7e); // The native scope tail-enters with the saved D/Y/U on top.
    wrapper.extend(symbol("NATIVE_FRAME").to_be_bytes());
    memory.bytes[0x6000..0x6000 + wrapper.len()].copy_from_slice(&wrapper);
    let mut code = Vec::new();
    call(&mut code, 0x6000);
    memory.execute(&mut cpu, &code);
    assert_ne!(cpu.registers.cc & C, 0);
    assert_eq!(cpu.registers.s, 0x7000);
    assert!(memory.output.is_empty());
    assert_eq!(&memory.bytes[0x6f80..0x6fa0], &[0xa5; 32]);
    set_word(&mut memory, 0x1f, 0x1e03);
    assert!(execute_direct(&mut memory, &mut cpu, "PRINT STR$(1.25)"));
    assert_eq!(memory.output, b" 1.25\r");
}

#[test]
fn basic_rom_guest_tokeniser_matches_original_synthetic_lexical_facts() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/guest-lexical.json")).unwrap();
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    for case in fixture["observations"].as_array().unwrap() {
        let source = case["source"].as_str().unwrap();
        let expected: Vec<u8> = case["tokens"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| u8::try_from(v.as_u64().unwrap()).unwrap())
            .collect();
        assert_eq!(
            guest_tokens(&mut memory, &mut cpu, source),
            expected,
            "{source}"
        );
    }
}

#[test]
fn basic_rom_legacy_lexer_keeps_the_existing_compiled_identifier_contract() {
    use eightbit_dragon_basic_core::{Dialect, analyse, tokenise};
    let dialect = Dialect::dragon32().unwrap();
    let source = "10 PRINTA=1:CLOADME=2:CSAVEMX=3:Z=PRINTA+CLOADME+CSAVEMX\n";
    let analysis = analyse(source, &dialect);
    assert!(!analysis.has_errors());
    let compiled = tokenise(&analysis.program, &dialect).unwrap();
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    let payload = &compiled.lines[0].code;
    memory.bytes[0x4000..0x4000 + payload.len()].copy_from_slice(payload);
    memory.bytes[0x4000 + payload.len()] = 0;
    let mut code = vec![0x8e, 0x40, 0];
    call(&mut code, symbol("EXECUTE"));
    memory.execute_with_budget(&mut cpu, &code, 4_194_304);
    assert_eq!(cpu.registers.cc & C, 0);
    assert_eq!(numeric_scalar(&memory, [b'Z', 0]), packed_integer(6));
}

#[test]
fn basic_rom_apostrophe_lookahead_preserves_in_place_input_bounds_and_recovery() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    for length in [0, 1, 2, 127, 248, 249] {
        let source = format!("'{}", "a".repeat(length));
        memory.bytes[0x1da..0x2dd].fill(0xa5);
        memory.bytes[0x3d9..0x400].fill(0x5a);
        let actual = guest_tokens(&mut memory, &mut cpu, &source);
        let mut expected = vec![b':', 0x83];
        expected.extend(vec![b'a'; length]);
        assert_eq!(actual, expected);
        assert_eq!(memory.bytes[0x2dd + actual.len()], 0);
        assert!(memory.bytes[0x1da..0x2dd].iter().all(|&v| v == 0xa5));
        assert!(memory.bytes[0x3d9..0x400].iter().all(|&v| v == 0x5a));
    }
    for source in [
        format!("'{}", "a".repeat(250)).into_bytes(),
        vec![b'\'', 0x80],
    ] {
        memory.bytes[0x2dd..0x2dd + source.len()].copy_from_slice(&source);
        memory.bytes[0x2dd + source.len()] = 0;
        let mut code = vec![0x8e, 2, 0xdd];
        call(&mut code, symbol("TOKENISE"));
        memory.execute_with_budget(&mut cpu, &code, 4_194_304);
        assert_ne!(cpu.registers.cc & C, 0);
        assert!(memory.bytes[0x1da..0x2dd].iter().all(|&v| v == 0xa5));
        assert!(memory.bytes[0x3d9..0x400].iter().all(|&v| v == 0x5a));
        assert_eq!(guest_tokens(&mut memory, &mut cpu, "?1"), [0x87, b'1']);
    }
}

#[test]
fn basic_rom_draw_matches_original_synthetic_facts() {
    use sha2::{Digest, Sha256};
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/draw.json")).unwrap();
    for expected in fixtures["cases"].as_array().unwrap() {
        let source = expected["source"].as_str().unwrap();
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        let mut init = Vec::new();
        call(&mut init, symbol("GFX_INIT"));
        memory.execute(&mut cpu, &init);
        assert_eq!(
            execute_direct_with_budget(&mut memory, &mut cpu, source, 16_777_216),
            expected["success"].as_bool().unwrap(),
            "{source}"
        );
        let length = usize::try_from(expected["graphics"]["bytes"].as_u64().unwrap()).unwrap();
        let plane = format!("{:x}", Sha256::digest(&memory.bytes[0x600..0x600 + length]));
        assert_eq!(
            plane,
            expected["graphics"]["sha256"].as_str().unwrap(),
            "{source} pixels"
        );
        assert_eq!(
            serde_json::json!([word(&memory, 0xc7), word(&memory, 0xc9)]),
            expected["cursor"],
            "{source} cursor"
        );
        assert_eq!(
            serde_json::json!([word(&memory, 0xbd), word(&memory, 0xbf)]),
            expected["physical"],
            "{source} workspace"
        );
        assert_eq!(
            serde_json::json!(memory.bytes[0xe8]),
            expected["angle"],
            "{source} angle"
        );
        assert_eq!(
            serde_json::json!(memory.bytes[0xe9]),
            expected["scale"],
            "{source} scale"
        );
    }
}

#[test]
fn basic_rom_line_raster_matches_original_in_every_mode_and_direction() {
    use sha2::{Digest, Sha256};
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/line-raster.json")).unwrap();
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    let mut init = Vec::new();
    call(&mut init, symbol("GFX_INIT"));
    memory.execute(&mut cpu, &init);
    for expected in fixtures["cases"].as_array().unwrap() {
        let source = expected["source"].as_str().unwrap();
        assert!(
            execute_direct_with_budget(&mut memory, &mut cpu, source, 4_194_304),
            "{source}"
        );
        let length = usize::try_from(expected["graphics"]["bytes"].as_u64().unwrap()).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&memory.bytes[0x600..0x600 + length])),
            expected["graphics"]["sha256"].as_str().unwrap(),
            "{source}"
        );
        assert_eq!(
            serde_json::json!([word(&memory, 0xc7), word(&memory, 0xc9)]),
            expected["cursor"],
            "{source} cursor"
        );
    }
}

#[test]
fn basic_rom_draw_bounds_nested_strings_and_recovers_after_failures() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    let mut init = Vec::new();
    call(&mut init, symbol("GFX_INIT"));
    memory.execute(&mut cpu, &init);
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "CLEAR1024:PMODE4,1:PCLS"
    ));
    for source in [
        "DRAW 1",
        "DRAW\"R4\"GARBAGE",
        "Q=-1:DRAW\"R=Q;\"",
        "Q$=\"X\":DRAW\"R=Q$;\"",
    ] {
        let before = memory.bytes[0x600..0x1e00].to_vec();
        assert!(!execute_direct(&mut memory, &mut cpu, source), "{source}");
        assert_eq!(memory.bytes[0x600..0x1e00], before, "{source}");
        assert_eq!(cpu.registers.s, 0x7000);
        assert!(
            execute_direct(&mut memory, &mut cpu, "DRAW\"\""),
            "{source} recovery: {:?}, error {}",
            cpu.registers,
            memory.bytes[0x3fd]
        );
    }
    assert!(execute_direct_with_budget(
        &mut memory,
        &mut cpu,
        "DRAW STRING$(255,\"B\")",
        4_194_304
    ));
    assert_eq!([word(&memory, 0xc7), word(&memory, 0xc9)], [128, 96]);
    assert!(execute_direct_with_budget(
        &mut memory,
        &mut cpu,
        "DRAW STRING$(255,\"R\")",
        4_194_304
    ));
    assert_eq!([word(&memory, 0xc7), word(&memory, 0xc9)], [255, 96]);
    assert_eq!(
        memory.bytes[0x600..0x1e00]
            .iter()
            .map(|v| v.count_ones())
            .sum::<u32>(),
        128
    );
    assert!(execute_direct(&mut memory, &mut cpu, "X$=\"XX$;\""));
    let storage = usize::from(word(&memory, 0x1f));
    let variables = memory.bytes[0x1e03..storage].to_vec();
    assert!(!execute_direct_with_budget(
        &mut memory,
        &mut cpu,
        "DRAW X$",
        4_194_304
    ));
    assert_eq!(cpu.registers.s, 0x7000);
    assert_eq!(memory.bytes[0x1e03..storage], variables);
    assert!(execute_direct(&mut memory, &mut cpu, "DRAW\"BM10,20R4\""));
    assert_eq!([word(&memory, 0xc7), word(&memory, 0xc9)], [14, 20]);
}

#[test]
fn basic_rom_draw_stack_collision_cannot_write_live_programme_storage() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    let mut init = Vec::new();
    call(&mut init, symbol("GFX_INIT"));
    memory.execute(&mut cpu, &init);
    set_word(&mut memory, 0x1f, 0x6e80);
    memory.bytes[0x6e60..0x6e80].fill(0xa5);
    let source = format!("DRAW\"{}\"", "R".repeat(200));
    assert!(!execute_direct_with_budget(
        &mut memory,
        &mut cpu,
        &source,
        4_194_304
    ));
    assert_eq!(cpu.registers.s, 0x7000);
    assert_eq!(&memory.bytes[0x6e60..0x6e80], &[0xa5; 32]);
    assert_eq!([word(&memory, 0xc7), word(&memory, 0xc9)], [128, 96]);
    set_word(&mut memory, 0x1f, 0x1e03);
    assert!(execute_direct(&mut memory, &mut cpu, "DRAW\"R4\""));
    assert_eq!(word(&memory, 0xc7), 132);
}

#[test]
fn basic_rom_deep_expressions_use_available_stack_and_guard_live_storage() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    // Lawful synthetic observations accept each depth, including 120 when the
    // complete direct line stays inside the keyboard input limit.
    for depth in [32, 40, 60, 100, 120] {
        let source = format!("A={}1{}", "(".repeat(depth), ")".repeat(depth));
        assert!(
            execute_direct(&mut memory, &mut cpu, &source),
            "depth {depth}"
        );
        assert_eq!(numeric_scalar(&memory, [b'A', 0]), packed_integer(1));
    }
    let source = format!("{}1{}", "(".repeat(120), ")".repeat(120));
    guest_tokens(&mut memory, &mut cpu, &source);
    set_word(&mut memory, 0x1f, 0x6e80);
    memory.bytes[0x6e60..0x6e80].fill(0xa5);
    let mut code = vec![0x8e, 2, 0xdd];
    call(&mut code, symbol("EVAL"));
    memory.execute_with_budget(&mut cpu, &code, 1_048_576);
    assert_ne!(cpu.registers.cc & C, 0);
    assert_eq!(cpu.registers.s, 0x7000);
    assert_eq!(&memory.bytes[0x6e60..0x6e80], &[0xa5; 32]);
    let array_end = word(&memory, 0x1d);
    set_word(&mut memory, 0x1f, array_end);
    assert!(execute_direct(&mut memory, &mut cpu, "A=7"));
}

#[test]
fn basic_rom_play_parameters_match_lawful_observations() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/play-parameters.json")).unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    for expected in cases {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        let mut init = Vec::new();
        call(&mut init, symbol("SETUP"));
        memory.execute_with_budget(&mut cpu, &init, 1_000_000);
        memory.next_frame = Some(memory.cycles + 17745);
        cpu.registers.cc &= 0xaf;
        let source = expected["source"].as_str().unwrap();
        let success = execute_direct_with_budget(&mut memory, &mut cpu, source, 16_777_216);
        let state = expected["state"].as_array().unwrap();
        let wanted: Vec<_> = [6, 7, 8, 9, 10, 13]
            .iter()
            .map(|i| u8::try_from(state[*i].as_u64().unwrap()).unwrap())
            .collect();
        let got: Vec<_> = [0xde, 0xdf, 0xe0, 0xe1, 0xe2, 0xe5]
            .iter()
            .map(|i| memory.bytes[*i])
            .collect();
        if success != expected["success"].as_bool().unwrap() || got != wanted {
            failures.push(format!(
                "{source}: success {success}, state {got:?}, expected {wanted:?}"
            ));
        }
        assert_eq!(cpu.registers.s, 0x7000, "{source}");
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn basic_rom_play_dac_pitch_volume_and_frame_duration() {
    // Synthetic black-box pitch facts, in E cycles. The original integer delay
    // generator has an eight-cycle grid, so allow half a delay iteration.
    for (macro_text, expected_quarter, frames, volume) in [
        ("O1T8L4C", 3418, 8, 15),
        ("O2T8L4C", 1714, 8, 15),
        ("O3T8L4C", 855, 8, 15),
        ("O4T8L4C", 430, 8, 15),
        ("O5T8L4C", 215, 8, 15),
        ("O2T8L4D", 1522, 8, 15),
        ("O2T8L4E", 1354, 8, 15),
        ("O2T8L4F", 1282, 8, 15),
        ("O2T8L4G", 1138, 8, 15),
        ("O2T8L4A", 1018, 8, 15),
        ("O2T8L4B", 906, 8, 15),
        ("O2T8L4C#", 1618, 8, 15),
        ("O2T8L4V31C", 1714, 8, 31),
        ("O2T8L4.C", 1714, 12, 15),
        ("O2T8L4..C", 1714, 16, 15),
        ("O2T8L8C", 1714, 4, 15),
        ("O2T16L4C", 1714, 4, 15),
    ] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        let mut init = Vec::new();
        call(&mut init, symbol("SETUP"));
        memory.execute_with_budget(&mut cpu, &init, 1_000_000);
        memory.next_frame = Some(memory.cycles + 17745);
        cpu.registers.cc &= 0xaf;
        memory.signals.clear();
        assert!(execute_direct_with_budget(
            &mut memory,
            &mut cpu,
            &format!("PLAY\"{macro_text}\""),
            4_000_000
        ));
        let samples: Vec<_> = memory.signals.iter().filter(|s| s.1 == 0xff20).collect();
        assert!(samples.len() > 20, "{macro_text}");
        for (index, sample) in samples.iter().take(12).enumerate() {
            assert_eq!(
                sample.2 >> 2,
                [31, 31 + volume, 31, 31 - volume][index % 4],
                "{macro_text}"
            );
            assert_eq!(sample.2 & 3, 2, "printer strobe {macro_text}");
        }
        let mut intervals: Vec<_> = samples.windows(2).map(|p| p[1].0 - p[0].0).collect();
        intervals.sort_unstable();
        let quarter = intervals[intervals.len() / 2];
        assert!(
            quarter.abs_diff(expected_quarter) <= 4,
            "{macro_text}: {quarter} expected {expected_quarter}"
        );
        let elapsed = samples.last().unwrap().0 - samples[0].0;
        assert!(
            ((frames - 1) * 17745..=(frames + 1) * 17745).contains(&elapsed),
            "{macro_text}: duration {elapsed}"
        );
        assert_eq!(word(&memory, 0xe3), 0, "{macro_text}");
        assert_eq!(memory.bytes[0xff23] & 8, 0, "{macro_text}");
        assert_eq!(cpu.registers.s, 0x7000);
    }
}

#[test]
fn basic_rom_run_resets_music_and_draw_defaults_without_resetting_graphics() {
    for command in ["CLEAR", "NEW", "RUN", "RUN10", "RUN65000"] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        let mut init = Vec::new();
        call(&mut init, symbol("SETUP"));
        memory.execute_with_budget(&mut cpu, &init, 1_000_000);
        store_line(&mut memory, &mut cpu, 10, "END");
        assert!(execute_direct(
            &mut memory,
            &mut cpu,
            "PMODE3,1:COLOR2,3:DRAW\"S8A2BM50,60\":PLAY\"O1V31L8T9\""
        ));
        let graphics = memory.bytes[0xb2..0xc7].to_vec();
        let success = execute_direct(&mut memory, &mut cpu, command);
        assert_eq!(success, command != "RUN65000", "{command}");
        assert_eq!(&memory.bytes[0xb2..0xc7], graphics, "{command}");
        let (cursor, music): ([u16; 2], [u8; 12]) = if command.starts_with("RUN") {
            ([128, 96], [2, 186, 66, 4, 2, 0, 0, 0, 0, 0, 0, 0])
        } else {
            ([50, 60], [0, 250, 2, 8, 9, 0, 0, 0, 0, 0, 2, 8])
        };
        assert_eq!(
            [word(&memory, 0xc7), word(&memory, 0xc9)],
            cursor,
            "{command}"
        );
        assert_eq!(&memory.bytes[0xde..0xea], music, "{command}");
    }
}

#[test]
fn basic_rom_graphics_array_marker_preserves_lookup_and_collection_boundaries() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "DIM A(7,3),B$(1):A=0:B$(0)=\"HELLO\""
    ));
    let array = usize::from(word(&memory, 0x1d));
    assert_eq!(
        &memory.bytes[array..array + 9],
        &[b'A', 0, 0, 169, 2, 0, 4, 0, 8]
    );
    set_word(&mut memory, array + 7, 0);
    memory.bytes[array + 9..array + 169].fill(0x55);
    let before = memory.bytes[array..array + 169].to_vec();
    assert!(execute_direct(
        &mut memory,
        &mut cpu,
        "DIM C(1):B$(1)=\"WORLD\":PRINT FRE(0);B$(0);B$(1)"
    ));
    let array = usize::from(word(&memory, 0x1d));
    assert_eq!(&memory.bytes[array..array + 169], before);
    assert!(memory.output.windows(10).any(|w| w == b"HELLOWORLD"));
    assert!(!execute_direct(&mut memory, &mut cpu, "PRINT A(0,0)"));
    assert!(execute_direct(&mut memory, &mut cpu, "A=12:PRINT A;C(0)"));
    // A short or non-integral former extent is not a valid G marker.
    for length in [9, 10, 14, 168] {
        set_word(&mut memory, array + 2, length);
        let mut code = vec![0x10, 0x8e];
        code.extend(u16::try_from(array).unwrap().to_be_bytes());
        call(&mut code, symbol("ARRAY_RECORD"));
        memory.execute(&mut cpu, &code);
        assert_ne!(cpu.registers.cc & C, 0, "record length {length}");
    }
    set_word(&mut memory, array + 2, 169);
    memory.bytes[array + 1] = 128;
    let mut code = vec![0x10, 0x8e];
    code.extend(u16::try_from(array).unwrap().to_be_bytes());
    call(&mut code, symbol("ARRAY_RECORD"));
    memory.execute(&mut cpu, &code);
    assert_ne!(
        cpu.registers.cc & C,
        0,
        "string arrays cannot carry a numeric graphics marker"
    );
}

#[test]
fn basic_rom_pcls_uses_the_public_background_byte_directly() {
    for mode in [0, 4] {
        for colour in [0u8, 1, 2, 3, 4, 5, 255] {
            let mut memory = Memory::new();
            let mut cpu = Cpu::default();
            empty_programme(&mut memory);
            let mut setup = Vec::new();
            call(&mut setup, symbol("GFX_INIT"));
            memory.execute(&mut cpu, &setup);
            assert!(execute_direct(
                &mut memory,
                &mut cpu,
                &format!("PMODE{mode},1:POKE179,{colour}:PCLS")
            ));
            let start = usize::from(word(&memory, 0xba));
            let end = usize::from(word(&memory, 0xb7));
            assert!(
                memory.bytes[start..end]
                    .iter()
                    .all(|&byte| byte == colour.wrapping_mul(85)),
                "mode {mode}, background {colour}"
            );
        }
    }
}

#[test]
fn basic_rom_graphics_coordinates_clip_within_the_legacy_numeric_domains() {
    use sha2::{Digest, Sha256};
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/coordinate-bounds.json")).unwrap();
    for expected in fixture["cases"].as_array().unwrap() {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        let mut init = Vec::new();
        call(&mut init, symbol("SETUP"));
        memory.execute_with_budget(&mut cpu, &init, 1_000_000);
        let source = expected["source"].as_str().unwrap();
        let success = execute_direct_with_budget(&mut memory, &mut cpu, source, 16_777_216);
        assert_eq!(success, expected["success"].as_bool().unwrap(), "{source}");
        assert_eq!(
            format!("{:x}", Sha256::digest(&memory.bytes[0x600..0x1e00])),
            expected["graphics"]["sha256"].as_str().unwrap(),
            "{source}"
        );
    }
}

#[test]
fn basic_rom_get_put_matches_synthetic_arrays_pixels_and_partial_transfers() {
    use sha2::{Digest, Sha256};
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/get-put.json")).unwrap();
    for expected in fixture["cases"].as_array().unwrap() {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        let mut init = Vec::new();
        call(&mut init, symbol("SETUP"));
        memory.execute_with_budget(&mut cpu, &init, 1_000_000);
        let source = expected["source"].as_str().unwrap();
        assert_eq!(
            execute_direct_with_budget(&mut memory, &mut cpu, source, 8_000_000),
            expected["success"].as_bool().unwrap(),
            "{source}"
        );
        let start = usize::from(word(&memory, 0x1d));
        let end = usize::from(word(&memory, 0x1f));
        let arrays: Vec<_> = expected["arrays"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| u8::try_from(v.as_u64().unwrap()).unwrap())
            .collect();
        assert_eq!(&memory.bytes[start..end], arrays, "array data: {source}");
        let first = usize::from(word(&memory, 0xba));
        let length = usize::try_from(expected["graphics"]["bytes"].as_u64().unwrap()).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&memory.bytes[first..first + length])),
            expected["graphics"]["sha256"].as_str().unwrap(),
            "canvas: {source}"
        );
        assert!(
            execute_direct(&mut memory, &mut cpu, "Z=86"),
            "recovery: {source}"
        );
        assert_eq!(numeric_scalar(&memory, [b'Z', 0]), packed_integer(86));
    }
}

#[test]
fn basic_rom_get_put_round_trip_full_planes_without_touching_adjacent_arrays() {
    for mode in 0..=4 {
        for packed in [false, true] {
            let mut memory = Memory::new();
            // A complete graphics array covers the ordinary $2000 probe driver.
            // Keep synthetic executable code outside both array and video RAM.
            memory.probe_address = 0x5800;
            let mut cpu = Cpu::default();
            empty_programme(&mut memory);
            enable_guest_printer(&mut memory);
            let mut init = Vec::new();
            call(&mut init, symbol("SETUP"));
            memory.execute_with_budget(&mut cpu, &init, 1_000_000);
            assert!(execute_direct(
                &mut memory,
                &mut cpu,
                &format!("PMODE{mode},1:DIM A(1230),B(2):B(2)=123:A=0")
            ));
            let end = usize::from(word(&memory, 0xb7));
            for (index, byte) in memory.bytes[0x600..end].iter_mut().enumerate() {
                *byte = u8::try_from((index * 37 + 13) % 256).unwrap();
            }
            let plane = memory.bytes[0x600..end].to_vec();
            let array = usize::from(word(&memory, 0x1d));
            let after = array + usize::from(word(&memory, array + 2));
            let array_end = usize::from(word(&memory, 0x1f));
            let guard = memory.bytes[after..array_end].to_vec();
            let suffix = if packed { ",G" } else { "" };
            let before = memory.cycles;
            assert!(execute_direct_with_budget(
                &mut memory,
                &mut cpu,
                &format!("GET(0,0)-(255,191),A{suffix}"),
                8_000_000
            ));
            assert!(memory.cycles - before < if packed { 4_100_000 } else { 650_000 });
            assert_eq!(&memory.bytes[array + 7..array + 7 + plane.len()], plane);
            memory.bytes[0x600..end].fill(0);
            let suffix = if packed { ",PSET" } else { "" };
            let before = memory.cycles;
            assert!(execute_direct_with_budget(
                &mut memory,
                &mut cpu,
                &format!("PUT(0,0)-(255,191),A{suffix}"),
                8_000_000
            ));
            assert!(memory.cycles - before < if packed { 5_100_000 } else { 500_000 });
            assert_eq!(&memory.bytes[0x600..end], plane);
            assert_eq!(&memory.bytes[after..array_end], guard);
            assert!(execute_direct(&mut memory, &mut cpu, "PRINT FRE(0):Z=B(2)"));
            assert_eq!(numeric_scalar(&memory, [b'Z', 0]), packed_integer(123));
        }
    }
}

#[test]
fn basic_rom_print_lists_and_spaced_identifiers_match_synthetic_observations() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/print-list.json")).unwrap();
    for expected in fixture["cases"].as_array().unwrap() {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        memory.bytes[0x6f] = 0;
        let source = expected["source"].as_str().unwrap();
        assert_eq!(
            execute_direct(&mut memory, &mut cpu, source),
            expected["success"].as_bool().unwrap(),
            "{source}"
        );
        assert_eq!(
            memory.output,
            expected["output"].as_str().unwrap().as_bytes(),
            "{source}"
        );
        assert_eq!(memory.bytes[0x6f], 0, "device restored: {source}");
        assert_eq!(
            word(&memory, 0x3ea),
            0,
            "temporary roots released: {source}"
        );
    }
}

#[test]
fn basic_rom_bounded_text_sink_preserves_registers_and_guards() {
    for capacity in [0u8, 1, 32, 251, 255] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        let mut setup = Vec::new();
        call(&mut setup, symbol("SETUP"));
        memory.execute(&mut cpu, &setup);
        memory.bytes[0x5fff..0x6101].fill(0xa5);
        memory.bytes[usize::from(symbol("FORMAT_BUFFER"))] = 1;
        let target = usize::from(symbol("FORMAT_TARGET"));
        memory.bytes[target..target + 2].copy_from_slice(&0x6000u16.to_be_bytes());
        memory.bytes[usize::from(symbol("FORMAT_LENGTH"))] = capacity;
        memory.bytes[0x6f] = 0xfe;
        memory.output.clear();
        cpu.registers.b = 0x7b;
        cpu.registers.x = 0x1234;
        cpu.registers.y = 0x5678;
        cpu.registers.u = 0x2345;
        let mut code = Vec::new();
        for _ in 0..usize::from(capacity) + 4 {
            code.extend([0x86, b'Q']);
            call(&mut code, symbol("CHROUT"));
        }
        memory.execute_with_budget(&mut cpu, &code, 32768);
        assert_eq!(cpu.registers.a, b'Q');
        assert_eq!(cpu.registers.b, 0x7b);
        assert_eq!(cpu.registers.x, 0x1234);
        assert_eq!(cpu.registers.y, 0x5678);
        assert_eq!(cpu.registers.u, 0x2345);
        assert_eq!(memory.bytes[0x5fff], 0xa5);
        assert!(
            memory.bytes[0x6000..0x6000 + usize::from(capacity)]
                .iter()
                .all(|byte| *byte == b'Q')
        );
        assert!(
            memory.bytes[0x6000 + usize::from(capacity)..0x6101]
                .iter()
                .all(|byte| *byte == 0xa5)
        );
        assert_eq!(memory.bytes[usize::from(symbol("FORMAT_LENGTH"))], 0);
        assert_eq!(
            u16::from_be_bytes([memory.bytes[target], memory.bytes[target + 1]]),
            0x6000 + u16::from(capacity)
        );
        assert!(memory.output.is_empty());
        memory.bytes[usize::from(symbol("FORMAT_BUFFER"))] = 0;
        memory.execute(&mut cpu, &[0x86, b'R', 0xbd, 0xb5, 0x4a]);
        assert_eq!(memory.output, b"R");
    }
}

#[test]
fn basic_rom_list_suppresses_only_structural_comment_and_else_colons() {
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/list-text.json")).unwrap();
    for row in fixtures["observations"].as_array().unwrap() {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        for line in row["programme"].as_array().unwrap() {
            let (number, text) = line.as_str().unwrap().split_once(' ').unwrap();
            store_line(&mut memory, &mut cpu, number.parse().unwrap(), text);
        }
        enable_guest_printer(&mut memory);
        assert!(execute_direct(&mut memory, &mut cpu, "LIST"));
        assert_eq!(memory.output, row["listing"].as_str().unwrap().as_bytes());
    }
}

#[test]
fn basic_rom_shared_text_context_skips_every_function_prefix() {
    for function in [
        "SGN", "INT", "ABS", "POS", "RND", "SQR", "LOG", "EXP", "SIN", "COS", "TAN", "ATN", "PEEK",
        "LEN", "STR$", "VAL", "ASC", "CHR$", "EOF", "JOYSTK", "FIX", "HEX$", "LEFT$", "RIGHT$",
        "MID$", "POINT", "INKEY$", "MEM", "VARPTR", "INSTR", "TIMER", "PPOINT", "STRING$", "USR",
    ] {
        let mut memory = Memory::new();
        let mut cpu = Cpu::default();
        empty_programme(&mut memory);
        enable_guest_printer(&mut memory);
        // No function body or argument validation may run in a false branch.
        assert!(
            execute_direct(
                &mut memory,
                &mut cpu,
                &format!("IF 0 THEN A={function}(1) ELSE PRINT 7")
            ),
            "{function}"
        );
        assert_eq!(memory.output, b" 7 \r", "{function}");
        store_line(&mut memory, &mut cpu, 10, &format!("A={function}(1)"));
        store_line(&mut memory, &mut cpu, 20, "DATA 85");
        let mut code = Vec::new();
        call(&mut code, symbol("DATA_RESET"));
        memory.execute(&mut cpu, &code);
        assert!(
            execute_direct(&mut memory, &mut cpu, "READ B"),
            "{function}"
        );
        assert_eq!(
            numeric_scalar(&memory, [b'B', 0]),
            packed_integer(85),
            "{function}"
        );
        memory.output.clear();
        assert!(execute_direct(&mut memory, &mut cpu, "LIST"));
        assert_eq!(
            memory.output,
            format!("10 A={function}(1)\r20 DATA 85\r").as_bytes(),
            "{function}"
        );
    }
}

#[test]
fn basic_rom_native_scope_restores_temporary_output_mode_on_success_and_failure() {
    for previous in [0u8, 1, 255] {
        for fail in [false, true] {
            let mut memory = Memory::new();
            let mut cpu = Cpu::default();
            memory.bytes[usize::from(symbol("FORMAT_BUFFER"))] = previous;
            memory.bytes[0x6f] = 0xfe;
            let mut body = vec![0x34, 0x66]; // The ordinary native wrapper saves D/Y/U.
            call(&mut body, symbol("NATIVE_INLINE"));
            body.extend([0x86, 23, 0xb7]);
            body.extend(symbol("FORMAT_BUFFER").to_be_bytes());
            if fail {
                body.extend([0x0f, 0x6f, 0x7e]);
                body.extend(symbol("NATIVE_FAIL").to_be_bytes());
            } else {
                body.extend([0x4f, 0x39]);
            }
            memory.bytes[0x6400..0x6400 + body.len()].copy_from_slice(&body);
            memory.execute(&mut cpu, &[0xbd, 0x64, 0]);
            assert_eq!(cpu.registers.cc & C != 0, fail);
            assert_eq!(cpu.registers.s, 0x7000);
            assert_eq!(memory.bytes[usize::from(symbol("FORMAT_BUFFER"))], previous);
            assert_eq!(memory.bytes[0x6f], 0xfe);
            assert_eq!(word(&memory, usize::from(symbol("STATEMENT_SP"))), 0);
        }
    }
}

#[test]
fn basic_rom_shared_byte_compare_keeps_unsigned_flags_and_bounded_spans() {
    for size in [0usize, 1, 8, 255] {
        for difference in [None, Some(0), Some(size.saturating_sub(1))] {
            for greater in [false, true] {
                let mut memory = Memory::new();
                let mut cpu = Cpu::default();
                memory.bytes[0x3000..0x3100].fill(0x55);
                memory.bytes[0x3200..0x3300].fill(0x55);
                let mismatch = difference.filter(|&offset| offset < size);
                if let Some(offset) = mismatch {
                    memory.bytes[0x3000 + offset] = if greater { 255 } else { 0 };
                }
                cpu.registers.x = 0x3000;
                cpu.registers.y = 0x3200;
                cpu.registers.u = 0x4567;
                cpu.registers.b = u8::try_from(size).unwrap();
                cpu.registers.cc = C;
                let mut code = Vec::new();
                call(&mut code, symbol("COMPARE_BYTES"));
                memory.execute(&mut cpu, &code);
                assert_eq!(
                    cpu.registers.cc & eightbit_m6809_core::Z != 0,
                    mismatch.is_none()
                );
                assert_eq!(cpu.registers.cc & C != 0, mismatch.is_some() && !greater);
                let consumed = u16::try_from(mismatch.map_or(size, |offset| offset + 1)).unwrap();
                assert_eq!(cpu.registers.x, 0x3000 + consumed);
                assert_eq!(cpu.registers.y, 0x3200 + consumed);
                assert_eq!(cpu.registers.u, 0x4567);
                assert_eq!(cpu.registers.s, 0x7000);
            }
        }
    }
}

#[test]
fn basic_rom_break_preserves_nested_control_frames_and_continuation() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    // The real prompt owns the control stack throughout RUN/Break/CONT. An ABI
    // call with a separate caller stack must not replace those live frames.
    let key = usize::from(symbol("KEYIN"));
    memory.bytes[key..key + 3].copy_from_slice(&[0x7e, 0x65, 0]);
    memory.bytes[0x6500..0x650d].copy_from_slice(&[
        0x34, 0x10, 0xbe, 0x66, 0, 0xa6, 0x80, 0xbf, 0x66, 0, 0x4d, 0x35, 0x90,
    ]);
    set_word(&mut memory, 0x6600, 0x6800);
    let text = concat!(
        "10 FOR I=1 TO 2\r20 GOSUB100\r30 NEXT I\r40 POKE25002,85:END\r",
        "100 POKE25000,I:IF PEEK(25001)=0 THEN100\r110 RETURN\r",
        "RUN\rPOKE25001,1:CONT\r"
    );
    memory.bytes[0x6800..0x6800 + text.len()].copy_from_slice(text.as_bytes());
    cpu.reset(&mut memory);
    let mut interrupted = false;
    let mut stopped = false;
    let mut finished = false;
    for _ in 0..8_388_608 {
        if memory.bytes[25000] == 1 && !interrupted {
            let mut keys = [0; 8];
            keys[2] = 64;
            memory.key_matrix = Some(keys);
            interrupted = true;
        }
        if interrupted && !stopped && memory.bytes[usize::from(symbol("CONT_VALID"))] == 1 {
            stopped = true;
            assert_eq!(word(&memory, 0x29), 100);
            memory.key_matrix = Some([0; 8]);
        }
        if memory.bytes[25002] == 85 {
            finished = true;
            break;
        }
        cpu.step(&mut memory).unwrap();
    }
    assert!(interrupted && stopped && finished);
    assert_eq!(memory.bytes[25000], 2);
    assert_eq!(numeric_scalar(&memory, [b'I', 0]), packed_integer(3));
}

#[test]
fn basic_rom_break_poll_uses_only_escape_without_printer_data_or_other_key_consumption() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    for index in 0..56 {
        let mut keys = [0; 8];
        keys[index % 8] = 1 << (index / 8);
        memory.key_matrix = Some(keys);
        memory.bytes[0xff02] = 0x3a;
        memory.bytes[0x152] = 0xff;
        let mut code = vec![];
        call(&mut code, symbol("BREAK_POLL"));
        memory.execute(&mut cpu, &code);
        assert_eq!(cpu.registers.cc & 4 != 0, index == 50, "matrix {index}");
        assert_eq!(memory.bytes[0xff02], 0x3a);
        assert_eq!(memory.bytes[0x152], if index == 50 { 0xbf } else { 0xff });
        assert!(memory.output.is_empty());
    }
}

#[test]
fn basic_rom_stop_reports_line_and_continues_after_the_statement() {
    let mut memory = Memory::new();
    let mut cpu = Cpu::default();
    empty_programme(&mut memory);
    enable_guest_printer(&mut memory);
    store_line(&mut memory, &mut cpu, 10, "STOP:POKE25000,85");
    assert!(execute_direct(&mut memory, &mut cpu, "RUN"));
    assert_eq!(memory.bytes[25000], 0);
    assert!(memory.output.ends_with(b"BREAK IN 10\r"));
    assert!(execute_direct(&mut memory, &mut cpu, "CONT"));
    assert_eq!(memory.bytes[25000], 85);
}
