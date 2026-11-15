//! Original test vectors calculated from M6809PM arithmetic and addressing rules.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_m6809_core::{Bus, C, Cpu, E, F, H, I, InterruptLines, N, Registers, V, Z};

struct Memory {
    data: Vec<u8>,
    ticks: u32,
    lines: InterruptLines,
    accesses: Vec<(u32, u16, bool)>,
    nmi_pulse: Option<(u32, u32)>,
    halt: bool,
}
impl Memory {
    fn new(code: &[u8]) -> Self {
        let mut data = vec![0; 65536];
        data[..code.len()].copy_from_slice(code);
        Self {
            data,
            ticks: 0,
            lines: InterruptLines::default(),
            accesses: Vec::new(),
            nmi_pulse: None,
            halt: false,
        }
    }
    fn vector(&mut self, address: u16, target: u16) {
        let a = usize::from(address);
        self.data[a..a + 2].copy_from_slice(&target.to_be_bytes());
    }
}
impl Bus for Memory {
    fn halted(&self) -> bool {
        self.halt
    }
    fn read(&mut self, a: u16) -> u8 {
        self.accesses.push((self.ticks, a, false));
        self.data[usize::from(a)]
    }
    fn write(&mut self, a: u16, v: u8) {
        self.accesses.push((self.ticks, a, true));
        self.data[usize::from(a)] = v;
    }
    fn tick(&mut self, n: u32) {
        self.ticks += n;
        if let Some((start, end)) = self.nmi_pulse {
            self.lines.nmi = self.ticks >= start && self.ticks < end;
        }
    }
    fn interrupts(&self) -> InterruptLines {
        self.lines
    }
}

#[test]
fn exhaustive_eight_bit_arithmetic() {
    let mut bus = Memory::new(&[0x8b, 0]);
    let mut cpu = Cpu::default();
    for (opcode, subtract, carry_in) in [
        (0x8b, false, false),
        (0x89, false, true),
        (0x80, true, false),
        (0x82, true, true),
        (0x81, true, false),
    ] {
        bus.data[0] = opcode;
        for left in 0..=255_u8 {
            for right in 0..=255_u8 {
                bus.data[1] = right;
                cpu.registers = Registers {
                    a: left,
                    cc: I | F | H | if carry_in { C } else { 0 },
                    ..Registers::default()
                };
                bus.accesses.clear();
                assert_eq!(cpu.step(&mut bus).unwrap(), 2);
                let mathematical = if subtract {
                    i32::from(left) - i32::from(right) - i32::from(carry_in)
                } else {
                    i32::from(left) + i32::from(right) + i32::from(carry_in)
                };
                let signed = if subtract {
                    i32::from(left.cast_signed())
                        - i32::from(right.cast_signed())
                        - i32::from(carry_in)
                } else {
                    i32::from(left.cast_signed())
                        + i32::from(right.cast_signed())
                        + i32::from(carry_in)
                };
                let result = u8::try_from(mathematical.rem_euclid(256)).unwrap();
                let expected = if (0..=255).contains(&mathematical) {
                    0
                } else {
                    C
                } | if (-128..=127).contains(&signed) { 0 } else { V }
                    | if result == 0 { Z } else { 0 }
                    | if result & 128 != 0 { N } else { 0 };
                assert_eq!(
                    cpu.registers.cc & (C | V | Z | N),
                    expected,
                    "opcode {opcode:02x}, {left}, {right}"
                );
                assert_eq!(cpu.registers.a, if opcode == 0x81 { left } else { result });
                assert_eq!(cpu.registers.cc & (I | F), I | F);
                assert_eq!(
                    cpu.registers.cc & H != 0,
                    subtract
                        || u16::from(left & 15) + u16::from(right & 15) + u16::from(carry_in) > 15
                );
            }
        }
    }
}

#[test]
fn decimal_adjust_all_valid_bcd_additions() {
    let mut bus = Memory::new(&[0x8b, 0, 0x19]);
    let mut cpu = Cpu::default();
    for left in 0..100_u8 {
        for right in 0..100_u8 {
            cpu.registers = Registers {
                a: (left / 10) * 16 + left % 10,
                ..Registers::default()
            };
            bus.data[1] = (right / 10) * 16 + right % 10;
            bus.accesses.clear();
            cpu.step(&mut bus).unwrap();
            assert_eq!(cpu.step(&mut bus).unwrap(), 2);
            let total = u16::from(left) + u16::from(right);
            let decimal = u8::try_from(total % 100).unwrap();
            assert_eq!(cpu.registers.a, (decimal / 10) * 16 + decimal % 10);
            assert_eq!(cpu.registers.cc & C != 0, total >= 100);
        }
    }
}

#[test]
fn every_indexed_postbyte_has_documented_acceptance_and_cycles() {
    for post in 0..=255_u8 {
        let mut bus = Memory::new(&[0xa6, post, 0, 0]);
        let mut cpu = Cpu::default();
        cpu.registers.x = 0x2000;
        cpu.registers.y = 0x3000;
        cpu.registers.u = 0x4000;
        cpu.registers.s = 0x5000;
        let mode = post & 15;
        let valid = post < 128
            || if post & 16 != 0 {
                matches!(mode, 1 | 3..=6 | 8 | 9 | 11..=13) || post == 0x9f
            } else {
                matches!(mode, 0..=6 | 8 | 9 | 11..=13)
            };
        let result = cpu.step(&mut bus);
        if !valid {
            assert!(result.is_err(), "undocumented indexed {post:02x}");
            assert_eq!(bus.accesses.len(), 2, "no operand read for {post:02x}");
            continue;
        }
        let extra = if post < 128 {
            1
        } else {
            (match mode {
                0 | 2 | 15 => 2,
                1 | 3 => 3,
                4 => 0,
                5 | 6 | 8 | 12 => 1,
                9 | 11 => 4,
                13 => 5,
                _ => unreachable!(),
            }) + if post & 16 != 0 { 3 } else { 0 }
        };
        assert_eq!(result.unwrap(), 4 + extra, "indexed {post:02x}");
        assert_eq!(bus.ticks, 4 + extra);
    }
}

#[test]
fn indexed_offsets_indirection_wrap_and_lea() {
    for (post, extra, expected, final_x) in [
        (0x1f, vec![], 0x1fff_u16, 0x2000),
        (0x88, vec![0x80], 0x1f80, 0x2000),
        (0x89, vec![0xff, 0xff], 0x1fff, 0x2000),
        (0x80, vec![], 0x2000, 0x2001),
        (0x83, vec![], 0x1ffe, 0x1ffe),
        (0x85, vec![], 0x1fff, 0x2000),
        (0x86, vec![], 0x1ffe, 0x2000),
        (0x8b, vec![], 0x1eff, 0x2000),
    ] {
        let mut code = vec![0xa6, post];
        code.extend(extra);
        let mut bus = Memory::new(&code);
        bus.data[usize::from(expected)] = 0x42;
        let mut cpu = Cpu::default();
        cpu.registers.x = 0x2000;
        cpu.registers.a = 0xfe;
        cpu.registers.b = 0xff;
        cpu.step(&mut bus).unwrap();
        assert_eq!(cpu.registers.a, 0x42);
        assert_eq!(cpu.registers.x, final_x);
    }
    let mut bus = Memory::new(&[0x30, 0x81, 0xa6, 0x94]);
    bus.data[0xffff] = 0x12;
    bus.data[0] = 0x34;
    let mut cpu = Cpu::default();
    cpu.registers.pc = 0x100;
    bus.data[0x100..0x104].copy_from_slice(&[0x30, 0x81, 0xa6, 0x94]);
    cpu.registers.x = 0xffff;
    cpu.step(&mut bus).unwrap();
    assert_eq!(cpu.registers.x, 0xffff);
    bus.data[0x1234] = 0x7b;
    cpu.step(&mut bus).unwrap();
    assert_eq!(cpu.registers.a, 0x7b);
}

#[test]
fn interrupt_priority_frames_masks_and_rti() {
    for (lines, vector, cycles, size, masks) in [
        (
            InterruptLines {
                irq: true,
                ..InterruptLines::default()
            },
            0xfff8,
            19,
            12,
            I,
        ),
        (
            InterruptLines {
                irq: true,
                firq: true,
                nmi: false,
            },
            0xfff6,
            10,
            3,
            I | F,
        ),
        (
            InterruptLines {
                irq: true,
                firq: true,
                nmi: true,
            },
            0xfffc,
            19,
            12,
            I | F,
        ),
    ] {
        let mut bus = Memory::new(&[0x10, 0xce, 0x70, 0]);
        let mut cpu = Cpu::default();
        cpu.step(&mut bus).unwrap();
        let before = Registers {
            a: 0x12,
            b: 0x34,
            dp: 0x56,
            x: 0x1234,
            y: 0x2345,
            u: 0x3456,
            s: 0x7000,
            pc: 0x4567,
            cc: C | H,
        };
        cpu.registers = before;
        bus.lines = lines;
        bus.vector(vector, 0x6000);
        bus.data[0x6000] = 0x3b;
        assert_eq!(cpu.step(&mut bus).unwrap(), cycles);
        assert_eq!(cpu.registers.pc, 0x6000);
        assert_eq!(cpu.registers.s, 0x7000 - size);
        assert_eq!(cpu.registers.cc & (I | F), masks);
        assert_eq!(&bus.data[0x6ffe..0x7000], &[0x45, 0x67]);
        assert_eq!(
            bus.data[usize::from(cpu.registers.s)],
            before.cc | if size == 12 { E } else { 0 }
        );
        bus.lines = InterruptLines::default();
        assert_eq!(cpu.step(&mut bus).unwrap(), if size == 12 { 15 } else { 6 });
        assert_eq!(
            cpu.registers,
            Registers {
                cc: before.cc | if size == 12 { E } else { 0 },
                ..before
            }
        );
    }
}

#[test]
fn software_interrupt_vectors_and_mask_preservation() {
    for (code, vector, cycles, masks) in [
        (vec![0x3f], 0xfffa, 19, I | F),
        (vec![0x10, 0x3f], 0xfff4, 20, 0),
        (vec![0x11, 0x3f], 0xfff2, 20, 0),
    ] {
        let mut bus = Memory::new(&code);
        bus.vector(vector, 0x4000);
        let mut cpu = Cpu::default();
        cpu.registers.s = 0x7000;
        assert_eq!(cpu.step(&mut bus).unwrap(), cycles);
        assert_eq!(cpu.registers.pc, 0x4000);
        assert_eq!(cpu.registers.cc, E | masks);
        assert_eq!(cpu.registers.s, 0x6ff4);
        assert_eq!(bus.data[0x6ff4], E);
    }
}

#[test]
fn cwai_stacks_once_and_sync_wakes_on_masked_interrupt() {
    let mut bus = Memory::new(&[0x3c, 0xaf]);
    bus.vector(0xfff6, 0x4000);
    bus.data[0x4000] = 0x3b;
    let mut cpu = Cpu::default();
    cpu.registers.s = 0x7000;
    cpu.registers.cc = I | F;
    assert_eq!(cpu.step(&mut bus).unwrap(), 20);
    assert_eq!(cpu.registers.s, 0x6ff4);
    assert_eq!(cpu.step(&mut bus).unwrap(), 1);
    bus.lines.firq = true;
    assert_eq!(cpu.step(&mut bus).unwrap(), 7);
    assert_eq!(cpu.registers.s, 0x6ff4);
    bus.lines.firq = false;
    assert_eq!(cpu.step(&mut bus).unwrap(), 15);
    assert_eq!(cpu.registers.pc, 2);
    assert_eq!(cpu.registers.s, 0x7000);
    bus.data[2..4].copy_from_slice(&[0x13, 0x12]);
    cpu.registers.cc |= I;
    assert_eq!(cpu.step(&mut bus).unwrap(), 4);
    assert_eq!(cpu.step(&mut bus).unwrap(), 1);
    bus.lines.irq = true;
    assert_eq!(cpu.step(&mut bus).unwrap(), 2);
    assert_eq!(cpu.registers.pc, 4);
}

#[test]
fn nmi_requires_initialised_stack_and_latches_short_pulse() {
    let mut bus = Memory::new(&[0x12, 0x10, 0xce, 0x70, 0, 0x3d, 0x12]);
    bus.vector(0xfffc, 0x6000);
    let mut cpu = Cpu::default();
    bus.lines.nmi = true;
    assert_eq!(cpu.step(&mut bus).unwrap(), 2); // No stack yet.
    cpu.step(&mut bus).unwrap();
    bus.lines.nmi = false;
    let start = bus.ticks;
    bus.nmi_pulse = Some((start + 2, start + 4));
    assert_eq!(cpu.step(&mut bus).unwrap(), 11);
    assert!(!bus.lines.nmi);
    assert_eq!(cpu.step(&mut bus).unwrap(), 19);
    assert_eq!(cpu.registers.pc, 0x6000);
    assert_eq!(&bus.data[0x6ffe..0x7000], &[0, 6]);
}

#[test]
fn memory_data_access_cycles_and_preserved_flags() {
    let mut bus = Memory::new(&[0x96, 0x20, 0x0c, 0x20, 0xb7, 0x30, 0]);
    bus.data[0x1220] = 0x7f;
    let mut cpu = Cpu::default();
    cpu.registers.dp = 0x12;
    cpu.registers.cc = C | H;
    assert_eq!(cpu.step(&mut bus).unwrap(), 4);
    assert_eq!(
        bus.accesses,
        [(0, 0, false), (1, 1, false), (3, 0x1220, false)]
    );
    bus.accesses.clear();
    assert_eq!(cpu.step(&mut bus).unwrap(), 6);
    assert_eq!(
        bus.accesses,
        [
            (4, 2, false),
            (5, 3, false),
            (7, 0x1220, false),
            (9, 0x1220, true)
        ]
    );
    assert_eq!(cpu.registers.cc & (C | H | V | N), C | H | V | N);
    bus.accesses.clear();
    assert_eq!(cpu.step(&mut bus).unwrap(), 5);
    assert_eq!(bus.accesses.last(), Some(&(14, 0x3000, true)));
}

#[test]
fn documented_opcode_matrix_and_instruction_counts() {
    // Independent opcode/cycle tables transcribed as facts from M6809PM Appendix F.
    let mut expected = [[0_u32; 256]; 3];
    for (opcode, cycles) in [
        (0x12, 2),
        (0x13, 4),
        (0x16, 5),
        (0x17, 9),
        (0x19, 2),
        (0x1a, 3),
        (0x1c, 3),
        (0x1d, 2),
        (0x1e, 8),
        (0x1f, 6),
        (0x39, 5),
        (0x3a, 3),
        (0x3b, 6),
        (0x3c, 20),
        (0x3d, 11),
        (0x3f, 19),
    ] {
        expected[0][opcode] = cycles;
    }
    expected[0][0x20..=0x2f].fill(3);
    expected[0][0x30..=0x33].fill(4);
    expected[0][0x34..=0x37].fill(8);
    for low in [0, 3, 4, 6, 7, 8, 9, 10, 12, 13, 15] {
        for (base, cycles) in [(0, 6), (0x40, 2), (0x50, 2), (0x60, 6), (0x70, 7)] {
            expected[0][base + low] = cycles;
        }
    }
    for (op, cycles) in [(0x0e, 3), (0x6e, 3), (0x7e, 4)] {
        expected[0][op] = cycles;
    }
    let a = [2, 2, 2, 4, 2, 2, 2, 0, 2, 2, 2, 2, 4, 7, 3, 0];
    let b = [2, 2, 2, 4, 2, 2, 2, 0, 2, 2, 2, 2, 3, 0, 3, 0];
    for (base, group) in [(0x80, a), (0xc0, b)] {
        for (low, immediate) in group.into_iter().enumerate() {
            expected[0][base + low] = immediate;
            for (mode, delta) in [(1, 2), (2, 2), (3, 3)] {
                let basic = match low {
                    7 => 2,
                    15 => 3,
                    13 if base == 0xc0 => 3,
                    13 => 5,
                    _ => immediate,
                };
                expected[0][base + mode * 16 + low] = basic + delta;
            }
        }
    }
    for (op, cycles) in expected[1].iter_mut().enumerate().take(0x30).skip(0x21) {
        *cycles = if matches!(op, 0x22 | 0x24 | 0x26 | 0x28 | 0x2a | 0x2c | 0x2e) {
            6
        } else {
            5
        };
    }
    expected[1][0x3f] = 20;
    expected[2][0x3f] = 20;
    for page in [1, 2] {
        for base in [0x83, 0x8c] {
            for (mode, cycles) in [(0, 5), (1, 7), (2, 7), (3, 8)] {
                expected[page][base + mode * 16] = cycles;
            }
        }
    }
    for base in [0x8e, 0xce] {
        for (mode, cycles) in [(0, 4), (1, 6), (2, 6), (3, 7)] {
            expected[1][base + mode * 16] = cycles;
            if mode > 0 {
                expected[1][base + mode * 16 + 1] = cycles;
            }
        }
    }
    let mut executed = 0;
    for (page, table) in expected.into_iter().enumerate() {
        for (op, cycles) in table.into_iter().enumerate() {
            if page == 0 && matches!(op, 0x10 | 0x11) {
                continue;
            }
            let mut code = Vec::new();
            if page > 0 {
                code.push(if page == 1 { 0x10 } else { 0x11 });
            }
            code.push(u8::try_from(op).unwrap());
            let indexed =
                matches!(op >> 4, 6 | 10 | 14) || (page == 0 && (0x30..=0x33).contains(&op));
            code.extend(if indexed {
                [0x84, 0]
            } else if page == 0 && op == 0x3c {
                [0xff, 0]
            } else {
                [0x12, 0]
            });
            let mut bus = Memory::new(&code);
            let mut cpu = Cpu::default();
            cpu.registers.x = 0x2000;
            cpu.registers.s = 0x7000;
            cpu.registers.u = 0x6000;
            let result = cpu.step(&mut bus);
            if cycles == 0 {
                assert!(result.is_err(), "undocumented page {page}, opcode {op:02x}");
            } else {
                assert_eq!(result.unwrap(), cycles, "page {page}, opcode {op:02x}");
                assert_eq!(bus.ticks, cycles);
                executed += 1;
            }
        }
    }
    assert_eq!(executed, 268);
}

#[test]
fn halt_keeps_registers_and_latches_nmi_until_bus_release() {
    let mut bus = Memory::new(&[0x10, 0xce, 0x70, 0, 0x12]);
    let mut cpu = Cpu::default();
    cpu.step(&mut bus).unwrap();
    let before = cpu.registers;
    bus.vector(0xfffc, 0x6000);
    bus.halt = true;
    let start = bus.ticks;
    bus.nmi_pulse = Some((start + 1, start + 2));
    for _ in 0..3 {
        assert_eq!(cpu.step(&mut bus).unwrap(), 1);
        assert_eq!(cpu.registers, before);
    }
    bus.halt = false;
    assert_eq!(cpu.step(&mut bus).unwrap(), 19);
    assert_eq!(cpu.registers.pc, 0x6000);
}
