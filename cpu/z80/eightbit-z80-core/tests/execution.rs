//! Original vectors; instruction/cycle contracts are Zilog UM008011-0816 facts.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
use eightbit_z80_core::{Access, Bus, C, Cpu, H, N, PV, S, X, Y, Z};

struct Memory {
    bytes: Box<[u8]>,
    clocks: u64,
    irq: bool,
    nmi: bool,
    vector: u8,
    input: u8,
    output: Vec<(u64, u16, u8)>,
    reti: usize,
    waits: u8,
    refreshes: Vec<u16>,
}
impl Memory {
    fn new(program: &[u8]) -> Self {
        let mut bytes = vec![0; 65536].into_boxed_slice();
        bytes[..program.len()].copy_from_slice(program);
        Self {
            bytes,
            clocks: 0,
            irq: false,
            nmi: false,
            vector: 255,
            input: 0,
            output: vec![],
            reti: 0,
            waits: 0,
            refreshes: vec![],
        }
    }
}
impl Bus for Memory {
    fn read(&mut self, address: u16) -> u8 {
        self.bytes[usize::from(address)]
    }
    fn write(&mut self, address: u16, value: u8) {
        self.bytes[usize::from(address)] = value;
    }
    fn input(&mut self, _port: u16) -> u8 {
        self.input
    }
    fn output(&mut self, port: u16, value: u8) {
        self.output.push((self.clocks, port, value));
    }
    fn tick(&mut self, t_states: u32) {
        self.clocks += u64::from(t_states);
    }
    fn refresh(&mut self, address: u16) {
        self.refreshes.push(address);
    }
    fn irq(&self) -> bool {
        self.irq
    }
    fn nmi(&self) -> bool {
        self.nmi
    }
    fn interrupt_acknowledge(&mut self) -> u8 {
        self.vector
    }
    fn reti(&mut self) {
        self.reti += 1;
    }
    fn wait_states(&self, _: u16, _: Access) -> u8 {
        self.waits
    }
}
fn execute(cpu: &mut Cpu, memory: &mut Memory, bytes: &[u8]) -> u32 {
    memory.bytes[..bytes.len()].copy_from_slice(bytes);
    cpu.registers.pc = 0;
    let before = memory.clocks;
    let elapsed = cpu.step(memory).unwrap();
    assert_eq!(u64::from(elapsed), memory.clocks - before);
    elapsed
}
fn basic_flags(result: u8) -> u8 {
    let mut f = result & (S | X | Y);
    if result == 0 {
        f |= Z;
    }
    f
}

#[test]
fn all_byte_add_subtract_and_compare_inputs_match_integer_oracle() {
    let mut cpu = Cpu::default();
    let mut memory = Memory::new(&[0, 0]);
    for operation in [0xc6, 0xce, 0xd6, 0xde, 0xfe] {
        let subtract = matches!(operation, 0xd6 | 0xde | 0xfe);
        for a in 0..=255_u8 {
            for b in 0..=255_u8 {
                for carry in 0..=1 {
                    cpu.registers.a = a;
                    cpu.registers.f = carry;
                    let c = if matches!(operation, 0xce | 0xde) {
                        carry
                    } else {
                        0
                    };
                    let wide = if subtract {
                        i32::from(a) - i32::from(b) - i32::from(c)
                    } else {
                        i32::from(a) + i32::from(b) + i32::from(c)
                    };
                    let signed = if subtract {
                        i32::from(a.cast_signed()) - i32::from(b.cast_signed()) - i32::from(c)
                    } else {
                        i32::from(a.cast_signed()) + i32::from(b.cast_signed()) + i32::from(c)
                    };
                    let low = if subtract {
                        i32::from(a % 16) - i32::from(b % 16) - i32::from(c)
                    } else {
                        i32::from(a % 16) + i32::from(b % 16) + i32::from(c)
                    };
                    let result = wide as u8;
                    let mut flags = basic_flags(result);
                    if subtract {
                        flags |= N;
                    }
                    if !(0..=255).contains(&wide) {
                        flags |= C;
                    }
                    if !(0..=15).contains(&low) {
                        flags |= H;
                    }
                    if !(-128..=127).contains(&signed) {
                        flags |= PV;
                    }
                    if operation == 0xfe {
                        flags = (flags & !(X | Y)) | (b & (X | Y));
                    }
                    assert_eq!(execute(&mut cpu, &mut memory, &[operation, b]), 7);
                    assert_eq!(
                        cpu.registers.f, flags,
                        "op {operation:02x}, {a:02x}, {b:02x}, carry {c}"
                    );
                    assert_eq!(cpu.registers.a, if operation == 0xfe { a } else { result });
                }
            }
        }
    }
}

#[test]
fn decimal_adjust_matches_every_valid_decimal_addition_and_subtraction() {
    let mut cpu = Cpu::default();
    let mut memory = Memory::new(&[0, 0]);
    for a in 0..100_u8 {
        for b in 0..100_u8 {
            for carry in 0..=1 {
                for subtract in [false, true] {
                    cpu.registers.a = ((a / 10) << 4) | (a % 10);
                    cpu.registers.f = carry;
                    let bcd = ((b / 10) << 4) | (b % 10);
                    execute(
                        &mut cpu,
                        &mut memory,
                        &[if subtract { 0xde } else { 0xce }, bcd],
                    );
                    assert_eq!(execute(&mut cpu, &mut memory, &[0x27]), 4);
                    let wide = if subtract {
                        i16::from(a) - i16::from(b) - i16::from(carry)
                    } else {
                        i16::from(a) + i16::from(b) + i16::from(carry)
                    };
                    let decimal = wide.rem_euclid(100) as u8;
                    assert_eq!(cpu.registers.a, ((decimal / 10) << 4) | (decimal % 10));
                    assert_eq!(cpu.registers.f & C != 0, !(0..100).contains(&wide));
                    assert_eq!(cpu.registers.f & N != 0, subtract);
                }
            }
        }
    }
}

#[test]
fn indexed_displacement_real_hl_and_register_halves_have_distinct_semantics() {
    let mut cpu = Cpu::default();
    let mut memory = Memory::new(&[]);
    cpu.registers.ix = 0x8000;
    cpu.registers.h = 0x27;
    memory.bytes[0x7ffe] = 0xa9;
    assert_eq!(execute(&mut cpu, &mut memory, &[0xdd, 0x66, 0xfe]), 19);
    assert_eq!(cpu.registers.h, 0xa9);
    assert_eq!(cpu.registers.ix, 0x8000);
    assert_eq!(execute(&mut cpu, &mut memory, &[0xdd, 0x74, 3]), 19);
    assert_eq!(memory.bytes[0x8003], 0xa9);
    assert_eq!(execute(&mut cpu, &mut memory, &[0xdd, 0x26, 0x72]), 11);
    assert_eq!(cpu.registers.ix, 0x7200);
    assert_eq!(cpu.registers.h, 0xa9);
    assert_eq!(execute(&mut cpu, &mut memory, &[0xfd, 0x21, 1, 0]), 14);
    memory.bytes[0xffff] = 0x7f;
    assert_eq!(execute(&mut cpu, &mut memory, &[0xfd, 0x34, 0xfe]), 23);
    assert_eq!(memory.bytes[0xffff], 0x80);
    assert_eq!(cpu.registers.f & (S | H | PV | N), S | H | PV);
}

#[test]
fn cb_bit_shift_and_indexed_copy_cycles_and_flags() {
    let mut cpu = Cpu::default();
    let mut memory = Memory::new(&[]);
    cpu.registers.b = 0x81;
    assert_eq!(execute(&mut cpu, &mut memory, &[0xcb, 0]), 8);
    assert_eq!(cpu.registers.b, 3);
    assert_eq!(cpu.registers.f, C | PV);
    cpu.registers.ix = 0x3000;
    memory.bytes[0x3001] = 0x81;
    assert_eq!(execute(&mut cpu, &mut memory, &[0xdd, 0xcb, 1, 0]), 23);
    assert_eq!(cpu.registers.b, 3);
    assert_eq!(memory.bytes[0x3001], 3);
    assert_eq!(execute(&mut cpu, &mut memory, &[0xdd, 0xcb, 1, 0x7e]), 20);
    assert_eq!(cpu.registers.f, C | H | Z | PV | Y);
    assert_eq!(memory.bytes[0x3001], 3);
    assert_eq!(execute(&mut cpu, &mut memory, &[0xcb, 0x80]), 8);
    assert_eq!(cpu.registers.b, 2);
    assert_eq!(cpu.registers.f, C | H | Z | PV | Y);
}

#[test]
fn block_transfer_can_pause_between_repetitions_and_preserves_carry() {
    let mut cpu = Cpu::default();
    let mut memory = Memory::new(&[0xed, 0xb0]);
    cpu.registers.set_hl(0x8100);
    cpu.registers.set_de(0x8200);
    cpu.registers.set_bc(3);
    cpu.registers.f = S | Z | C;
    memory.bytes[0x8100..0x8103].copy_from_slice(&[0x12, 0x34, 0x56]);
    assert_eq!(cpu.step(&mut memory).unwrap(), 21);
    assert_eq!(cpu.registers.pc, 0);
    assert_eq!(cpu.registers.bc(), 2);
    assert_eq!(cpu.registers.f & (S | Z | C | PV), S | Z | C | PV);
    assert_eq!(cpu.step(&mut memory).unwrap(), 21);
    assert_eq!(cpu.step(&mut memory).unwrap(), 16);
    assert_eq!(cpu.registers.pc, 2);
    assert_eq!(cpu.registers.bc(), 0);
    assert_eq!(&memory.bytes[0x8200..0x8203], &[0x12, 0x34, 0x56]);
    assert_eq!(cpu.registers.f & (S | Z | C | PV), S | Z | C);
}

#[test]
fn ei_halt_irq_and_reti_observe_delayed_enable_and_return_address() {
    let mut cpu = Cpu::default();
    let mut memory = Memory::new(&[0xfb, 0x76]);
    cpu.registers.sp = 0x9000;
    cpu.registers.interrupt_mode = 1;
    memory.bytes[0x38..0x3b].copy_from_slice(&[0xfb, 0xed, 0x4d]);
    memory.irq = true;
    assert_eq!(cpu.step(&mut memory).unwrap(), 4);
    assert_eq!(cpu.step(&mut memory).unwrap(), 4);
    assert!(cpu.halted);
    assert_eq!(cpu.step(&mut memory).unwrap(), 13);
    assert!(!cpu.halted);
    assert_eq!(cpu.registers.pc, 0x38);
    assert_eq!(&memory.bytes[0x8ffe..0x9000], &[2, 0]);
    assert_eq!(cpu.step(&mut memory).unwrap(), 4);
    assert_eq!(cpu.step(&mut memory).unwrap(), 14);
    assert_eq!(cpu.registers.pc, 2);
    assert_eq!(memory.reti, 1);
}

#[test]
fn im2_wraps_vector_reads_and_nmi_is_edge_triggered_without_clobbering_iff2() {
    let mut cpu = Cpu::default();
    let mut memory = Memory::new(&[0]);
    cpu.registers.pc = 0x4567;
    cpu.registers.sp = 0x9000;
    cpu.registers.i = 255;
    cpu.registers.iff1 = true;
    cpu.registers.iff2 = true;
    cpu.registers.interrupt_mode = 2;
    memory.bytes[0xffff] = 0x34;
    memory.bytes[0] = 0x12;
    memory.irq = true;
    assert_eq!(cpu.step(&mut memory).unwrap(), 19);
    assert_eq!(cpu.registers.pc, 0x1234);
    assert!(!cpu.registers.iff1 && !cpu.registers.iff2);
    memory.irq = false;
    cpu.registers.iff2 = true;
    memory.nmi = true;
    assert_eq!(cpu.step(&mut memory).unwrap(), 11);
    assert_eq!(cpu.registers.pc, 0x66);
    assert!(cpu.registers.iff2);
    assert_eq!(cpu.step(&mut memory).unwrap(), 4);
    assert_eq!(cpu.registers.pc, 0x67);
    memory.nmi = false;
    cpu.step(&mut memory).unwrap();
    memory.nmi = true;
    assert_eq!(cpu.step(&mut memory).unwrap(), 11);
    assert!(cpu.registers.iff2);
}

#[test]
fn io_preserves_full_port_address_flags_and_transfer_time() {
    let mut cpu = Cpu::default();
    let mut memory = Memory::new(&[]);
    cpu.registers.a = 0x12;
    cpu.registers.f = 0xa5;
    assert_eq!(execute(&mut cpu, &mut memory, &[0xd3, 0xfe]), 11);
    assert_eq!(memory.output, vec![(10, 0x12fe, 0x12)]);
    assert_eq!(cpu.registers.f, 0xa5);
    cpu.registers.set_bc(0xfefe);
    memory.input = 0x80;
    assert_eq!(execute(&mut cpu, &mut memory, &[0xed, 0x78]), 12);
    assert_eq!(cpu.registers.a, 0x80);
    assert_eq!(cpu.registers.f, S | C);
}

#[test]
fn prefixes_refresh_wrap_and_contention_are_bounded() {
    let mut cpu = Cpu::default();
    let mut memory = Memory::new(&[0xdd; 64]);
    cpu.registers.r = 0xff;
    let error = cpu.step(&mut memory).unwrap_err();
    assert_eq!(error.address, 0);
    assert_eq!(cpu.registers.pc, 64);
    assert_eq!(cpu.registers.r, 0xbf);
    cpu.registers.iy = 0x8000;
    assert_eq!(
        execute(&mut cpu, &mut memory, &[0xdd, 0xfd, 0x21, 0x34, 0x12]),
        18
    );
    assert_eq!(cpu.registers.iy, 0x1234);
    memory.waits = 2;
    assert_eq!(execute(&mut cpu, &mut memory, &[0x3e, 0x42]), 11);
    assert_eq!(cpu.registers.a, 0x42);
}

#[test]
fn every_base_and_prefixed_encoding_executes_within_a_small_instruction_bound() {
    let mut memory = Memory::new(&[]);
    for prefix in [
        &[][..],
        &[0xcb][..],
        &[0xed][..],
        &[0xdd][..],
        &[0xfd][..],
        &[0xdd, 0xcb, 0][..],
        &[0xfd, 0xcb, 0][..],
    ] {
        for opcode in 0..=255 {
            let mut cpu = Cpu::default();
            cpu.registers.sp = 0x9000;
            memory.bytes.fill(0);
            let mut bytes = prefix.to_vec();
            bytes.extend([opcode, 0, 0, 0]);
            let cycles = execute(&mut cpu, &mut memory, &bytes);
            assert!((4..=32).contains(&cycles), "{bytes:02x?}: {cycles}");
        }
    }
}

#[test]
fn refresh_observes_only_m1_halt_and_interrupt_cycles_with_i_and_r_wrap() {
    let mut cpu = Cpu::default();
    let mut memory = Memory::new(&[0xdd, 0xcb, 0, 0x46, 0x76]);
    cpu.registers.i = 0x1e;
    cpu.registers.r = 0xfe;
    cpu.registers.sp = 0x9000;
    cpu.registers.interrupt_mode = 1;
    // The indexed CB displacement and operation byte are ordinary reads.
    assert_eq!(cpu.step(&mut memory).unwrap(), 20);
    assert_eq!(memory.refreshes, [0x1eff, 0x1e80]);
    assert_eq!(cpu.step(&mut memory).unwrap(), 4);
    assert_eq!(cpu.step(&mut memory).unwrap(), 4);
    assert_eq!(memory.refreshes, [0x1eff, 0x1e80, 0x1e81, 0x1e82]);
    cpu.registers.iff1 = true;
    memory.irq = true;
    assert_eq!(cpu.step(&mut memory).unwrap(), 13);
    assert_eq!(memory.refreshes.last(), Some(&0x1e83));
    memory.irq = false;
    memory.nmi = true;
    assert_eq!(cpu.step(&mut memory).unwrap(), 11);
    assert_eq!(memory.refreshes.last(), Some(&0x1e84));
}
