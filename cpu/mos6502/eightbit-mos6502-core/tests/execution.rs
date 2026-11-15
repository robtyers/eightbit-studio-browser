//! Original algebraic and bus-observation vectors; no external emulator fixtures.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_mos6502_core::{
    B, Bus, C, Cpu, D, Error, I, MAX_STEPS, N, Registers, StepKind, StopReason, U, V, Z,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Access {
    Read(u16),
    Write(u16, u8),
}
use Access::{Read as R, Write as W};
struct Ram {
    bytes: Box<[u8]>,
    trace: Vec<Access>,
    fail: Option<usize>,
}
impl Default for Ram {
    fn default() -> Self {
        Self {
            bytes: vec![0; 65536].into_boxed_slice(),
            trace: Vec::new(),
            fail: None,
        }
    }
}
impl Bus for Ram {
    type Error = &'static str;
    fn read(&mut self, address: u16) -> Result<u8, Self::Error> {
        self.trace.push(R(address));
        if self.fail == Some(self.trace.len()) {
            Err("read rejected")
        } else {
            Ok(self.bytes[usize::from(address)])
        }
    }
    fn write(&mut self, address: u16, value: u8) -> Result<(), Self::Error> {
        self.trace.push(W(address, value));
        if self.fail == Some(self.trace.len()) {
            Err("write rejected")
        } else {
            self.bytes[usize::from(address)] = value;
            Ok(())
        }
    }
}
fn setup(programme: &[u8]) -> (Cpu, Ram) {
    let mut ram = Ram::default();
    ram.bytes[0x200..0x200 + programme.len()].copy_from_slice(programme);
    (
        Cpu::from_registers(Registers {
            pc: 0x200,
            sp: 0xfd,
            ..Registers::default()
        }),
        ram,
    )
}
fn state(pc: u16, a: u8, p: u8) -> Cpu {
    Cpu::from_registers(Registers {
        pc,
        a,
        p,
        sp: 0xfd,
        ..Registers::default()
    })
}

#[test]
fn reset_is_seven_reads_and_retains_nmos_decimal_and_ram() {
    let mut ram = Ram::default();
    ram.bytes[0xfffc..0xfffe].copy_from_slice(&[0x34, 0x12]);
    let mut cpu = state(0x4567, 0xab, D | B);
    assert_eq!(cpu.reset(&mut ram), Ok(7));
    assert_eq!(
        ram.trace,
        [
            R(0x4567),
            R(0x4567),
            R(0x1fd),
            R(0x1fc),
            R(0x1fb),
            R(0xfffc),
            R(0xfffd)
        ]
    );
    assert_eq!(
        cpu.registers(),
        Registers {
            pc: 0x1234,
            sp: 0xfa,
            a: 0xab,
            p: D | I | U,
            x: 0,
            y: 0
        }
    );
    let mut cold = Cpu::new();
    ram.trace.clear();
    cold.reset(&mut ram).unwrap();
    assert_eq!(cold.registers().sp, 0xfd);
}

#[test]
fn binary_adc_sbc_exhaustive_results_and_all_arithmetic_flags() {
    let (_, mut ram) = setup(&[0x69, 0]);
    for opcode in [0x69, 0xe9] {
        ram.bytes[0x200] = opcode;
        for a in 0..=255u8 {
            for operand in 0..=255u8 {
                for carry in [false, true] {
                    ram.trace.clear();
                    ram.bytes[0x201] = operand;
                    let mut cpu = state(0x200, a, I | u8::from(carry));
                    assert_eq!(cpu.step(&mut ram).unwrap().cycles, 2);
                    let wide = if opcode == 0x69 {
                        i16::from(a) + i16::from(operand) + i16::from(carry)
                    } else {
                        i16::from(a) - i16::from(operand) - i16::from(!carry)
                    };
                    let result = wide.to_le_bytes()[0];
                    let signed_a = i16::from(i8::from_ne_bytes([a]));
                    let signed_b = i16::from(i8::from_ne_bytes([operand]));
                    let signed = if opcode == 0x69 {
                        signed_a + signed_b + i16::from(carry)
                    } else {
                        signed_a - signed_b - i16::from(!carry)
                    };
                    let expected_c = if opcode == 0x69 {
                        wide > 255
                    } else {
                        wide >= 0
                    };
                    let expected_p = I
                        | U
                        | u8::from(expected_c)
                        | if result == 0 { Z } else { 0 }
                        | (result & N)
                        | if (-128..=127).contains(&signed) { 0 } else { V };
                    assert_eq!(
                        (cpu.registers().a, cpu.registers().p),
                        (result, expected_p),
                        "{opcode:02x} {a:02x} {operand:02x} {carry}"
                    );
                }
            }
        }
    }
}

fn bcd(value: u8) -> u8 {
    (value / 10) * 16 + value % 10
}
#[test]
fn valid_decimal_digits_exhaustive_against_base_ten_arithmetic() {
    let (_, mut ram) = setup(&[0x69, 0]);
    for opcode in [0x69, 0xe9] {
        ram.bytes[0x200] = opcode;
        for a in 0..100 {
            for operand in 0..100 {
                for carry in [false, true] {
                    ram.trace.clear();
                    ram.bytes[0x201] = bcd(operand);
                    let mut cpu = state(0x200, bcd(a), D | I | u8::from(carry));
                    cpu.step(&mut ram).unwrap();
                    let decimal = if opcode == 0x69 {
                        i16::from(a) + i16::from(operand) + i16::from(carry)
                    } else {
                        i16::from(a) - i16::from(operand) - i16::from(!carry)
                    };
                    let result = u8::try_from(decimal.rem_euclid(100)).unwrap();
                    let expected_c = if opcode == 0x69 {
                        decimal >= 100
                    } else {
                        decimal >= 0
                    };
                    assert_eq!(
                        (cpu.registers().a, cpu.registers().p & C),
                        (bcd(result), u8::from(expected_c)),
                        "{opcode:02x} {a} {operand} {carry}"
                    );
                    assert_eq!(cpu.registers().p & (D | I | U | B), D | I | U);
                }
            }
        }
    }
}

#[test]
fn nmos_decimal_flags_are_not_cmos_adjusted_result_flags() {
    // Expectations specify pre-decimal flag behaviour separately from the decimal result.
    for (opcode, a, operand, carry, result, flags) in [
        (0x69, 0x50, 0x50, false, 0x00, C | N | V),
        (0x69, 0x79, 0x00, true, 0x80, N | V),
        (0x69, 0x99, 0x00, true, 0x00, C | N),
        (0xe9, 0x00, 0x01, true, 0x99, N),
        (0xe9, 0x80, 0x01, true, 0x79, C | V),
        (0xe9, 0x50, 0x50, true, 0x00, C | Z),
    ] {
        let (_, mut ram) = setup(&[opcode, operand]);
        let mut cpu = state(0x200, a, D | u8::from(carry));
        cpu.step(&mut ram).unwrap();
        assert_eq!(
            (cpu.registers().a, cpu.registers().p),
            (result, flags | D | U)
        );
    }
}

#[test]
fn indexed_read_store_and_rmw_emit_different_dummy_cycles() {
    let (cpu, mut ram) = setup(&[0xbd, 0xff, 0x12]);
    let mut regs = cpu.registers();
    regs.x = 1;
    regs.a = 0x5a;
    ram.bytes[0x1300] = 0x7f;
    for (opcode, expected) in [
        (
            0xbd,
            vec![R(0x200), R(0x201), R(0x202), R(0x1200), R(0x1300)],
        ),
        (
            0x9d,
            vec![R(0x200), R(0x201), R(0x202), R(0x1200), W(0x1300, 0x5a)],
        ),
        (
            0xfe,
            vec![
                R(0x200),
                R(0x201),
                R(0x202),
                R(0x1200),
                R(0x1300),
                W(0x1300, 0x7f),
                W(0x1300, 0x80),
            ],
        ),
    ] {
        ram.trace.clear();
        ram.bytes[0x200] = opcode;
        ram.bytes[0x1300] = 0x7f;
        let step = Cpu::from_registers(regs).step(&mut ram).unwrap();
        assert_eq!(usize::from(step.cycles), expected.len());
        assert_eq!(ram.trace, expected);
    }
    ram.bytes[0x201] = 0x10;
    for (opcode, cycles) in [(0xbd, 4), (0x9d, 5), (0xfe, 7)] {
        ram.trace.clear();
        ram.bytes[0x200] = opcode;
        assert_eq!(
            Cpu::from_registers(regs).step(&mut ram).unwrap().cycles,
            cycles
        );
    }
}

#[test]
fn zero_page_indirect_indexing_and_full_address_wrap() {
    let (cpu, mut ram) = setup(&[0xa1, 0xfe]);
    let mut regs = cpu.registers();
    regs.x = 1;
    ram.bytes[0xff] = 0xff;
    ram.bytes[0] = 0xff;
    ram.bytes[0xffff] = 0x7a;
    let mut cpu = Cpu::from_registers(regs);
    assert_eq!(cpu.step(&mut ram).unwrap().cycles, 6);
    assert_eq!(cpu.registers().a, 0x7a);
    assert_eq!(
        ram.trace,
        [R(0x200), R(0x201), R(0xfe), R(0xff), R(0), R(0xffff)]
    );
    ram.trace.clear();
    ram.bytes[0x200] = 0xb1;
    ram.bytes[0x201] = 0xff;
    ram.bytes[0] = 0xff;
    regs.y = 1;
    let mut cpu = Cpu::from_registers(regs);
    cpu.step(&mut ram).unwrap();
    assert_eq!(
        ram.trace,
        [R(0x200), R(0x201), R(0xff), R(0), R(0xff00), R(0)]
    );
    assert_eq!(cpu.registers().a, 0xff);
    ram.trace.clear();
    ram.bytes[0xffff] = 0xa9;
    ram.bytes[0] = 0x42;
    let mut cpu = state(0xffff, 0, 0);
    cpu.step(&mut ram).unwrap();
    assert_eq!((cpu.registers().pc, cpu.registers().a), (1, 0x42));
    assert_eq!(ram.trace, [R(0xffff), R(0)]);
}

#[test]
fn branches_cover_all_conditions_page_crossing_and_pc_wrap() {
    for (opcode, mask, set) in [
        (0x10, N, false),
        (0x30, N, true),
        (0x50, V, false),
        (0x70, V, true),
        (0x90, C, false),
        (0xb0, C, true),
        (0xd0, Z, false),
        (0xf0, Z, true),
    ] {
        for taken in [false, true] {
            let p = if taken == set { mask } else { 0 };
            for (pc, offset, target, cycles) in [
                (0x0200u16, 2, 0x0204, 3),
                (0x02fd, 1, 0x0300, 4),
                (0x0300, 0xfc, 0x02fe, 4),
                (0xfffd, 1, 0, 4),
            ] {
                let mut ram = Ram::default();
                ram.bytes[usize::from(pc)] = opcode;
                ram.bytes[usize::from(pc.wrapping_add(1))] = offset;
                let mut cpu = state(pc, 0, p);
                let step = cpu.step(&mut ram).unwrap();
                assert_eq!(step.cycles, if taken { cycles } else { 2 });
                assert_eq!(
                    cpu.registers().pc,
                    if taken { target } else { pc.wrapping_add(2) }
                );
            }
        }
    }
}

#[test]
fn indirect_jmp_wraps_within_page_and_jsr_rts_order_is_observable() {
    let (mut cpu, mut ram) = setup(&[0x6c, 0xff, 0x12]);
    ram.bytes[0x12ff] = 0x34;
    ram.bytes[0x1200] = 0x56;
    ram.bytes[0x1300] = 0x99;
    assert_eq!(cpu.step(&mut ram).unwrap().cycles, 5);
    assert_eq!(cpu.registers().pc, 0x5634);
    assert_eq!(
        ram.trace,
        [R(0x200), R(0x201), R(0x202), R(0x12ff), R(0x1200)]
    );
    let (mut cpu, mut ram) = setup(&[0x20, 0x00, 0x04]);
    ram.bytes[0x400] = 0x60;
    assert_eq!(cpu.step(&mut ram).unwrap().cycles, 6);
    assert_eq!(
        ram.trace,
        [
            R(0x200),
            R(0x201),
            R(0x1fd),
            W(0x1fd, 2),
            W(0x1fc, 2),
            R(0x202)
        ]
    );
    ram.trace.clear();
    assert_eq!(cpu.step(&mut ram).unwrap().cycles, 6);
    assert_eq!(
        ram.trace,
        [R(0x400), R(0x401), R(0x1fb), R(0x1fc), R(0x1fd), R(0x202)]
    );
    assert_eq!((cpu.registers().pc, cpu.registers().sp), (0x203, 0xfd));
}

#[test]
fn brk_irq_nmi_rti_status_and_stack_wrap() {
    let (mut cpu, mut ram) = setup(&[0x00, 0xaa]);
    ram.bytes[0xfffe..].copy_from_slice(&[0x00, 0x04]);
    ram.bytes[0x400] = 0x40;
    assert_eq!(cpu.step(&mut ram).unwrap().cycles, 7);
    assert_eq!(
        ram.trace,
        [
            R(0x200),
            R(0x201),
            W(0x1fd, 2),
            W(0x1fc, 2),
            W(0x1fb, I | B | U),
            R(0xfffe),
            R(0xffff)
        ]
    );
    assert_eq!(cpu.step(&mut ram).unwrap().cycles, 6);
    assert_eq!((cpu.registers().pc, cpu.registers().p), (0x202, I | U));
    let mut cpu = Cpu::from_registers(Registers {
        pc: 0x200,
        sp: 0,
        p: D | U,
        ..Registers::default()
    });
    ram.trace.clear();
    cpu.set_irq(true);
    assert_eq!(cpu.step(&mut ram).unwrap().kind, StepKind::Irq);
    assert_eq!(
        ram.trace,
        [
            R(0x200),
            R(0x200),
            W(0x100, 2),
            W(0x1ff, 0),
            W(0x1fe, D | U),
            R(0xfffe),
            R(0xffff)
        ]
    );
    assert_eq!(cpu.registers().p, D | I | U);
    ram.bytes[0xfffa..0xfffc].copy_from_slice(&[0, 5]);
    ram.bytes[0x500] = 0xea;
    cpu.set_nmi(true);
    assert_eq!(cpu.step(&mut ram).unwrap().kind, StepKind::Nmi);
    assert_eq!(
        cpu.step(&mut ram).unwrap().kind,
        StepKind::Instruction(0xea)
    );
    cpu.set_nmi(false);
    cpu.set_nmi(true);
    assert_eq!(cpu.step(&mut ram).unwrap().kind, StepKind::Nmi);
}

#[test]
fn cli_sei_plp_delay_irq_mask_but_rti_restores_it_immediately() {
    let (mut cpu, mut ram) = setup(&[0x58, 0xea, 0xea]);
    cpu.set_irq(true);
    assert_eq!(
        cpu.step(&mut ram).unwrap().kind,
        StepKind::Instruction(0x58)
    );
    assert_eq!(
        cpu.step(&mut ram).unwrap().kind,
        StepKind::Instruction(0xea)
    );
    assert_eq!(cpu.step(&mut ram).unwrap().kind, StepKind::Irq);
    let (_, mut ram) = setup(&[0x78, 0xea]);
    let mut cpu = state(0x200, 0, 0);
    cpu.step(&mut ram).unwrap();
    cpu.set_irq(true);
    assert_eq!(cpu.step(&mut ram).unwrap().kind, StepKind::Irq);
    let (mut cpu, mut ram) = setup(&[0x28, 0xea]);
    ram.bytes[0x1fe] = 0;
    cpu.set_irq(true);
    cpu.step(&mut ram).unwrap();
    assert_eq!(
        cpu.step(&mut ram).unwrap().kind,
        StepKind::Instruction(0xea)
    );
    assert_eq!(cpu.step(&mut ram).unwrap().kind, StepKind::Irq);
    let (mut cpu, mut ram) = setup(&[0x40]);
    ram.bytes[0x1fe] = 0;
    ram.bytes[0x1ff] = 0;
    ram.bytes[0x100] = 4;
    cpu.set_irq(true);
    cpu.step(&mut ram).unwrap();
    assert_eq!(cpu.registers().pc, 0x400);
    assert_eq!(cpu.step(&mut ram).unwrap().kind, StepKind::Irq);
}

#[test]
fn unsupported_opcodes_all_fault_and_require_explicit_recovery() {
    let mut documented = 0;
    for opcode in 0..=255u8 {
        let (mut cpu, mut ram) = setup(&[opcode, 0, 0]);
        match cpu.step(&mut ram) {
            Ok(step) => {
                documented += 1;
                assert!((2..=7).contains(&step.cycles));
                assert_eq!(usize::from(step.cycles), ram.trace.len());
            }
            Err(Error::UnsupportedOpcode {
                address,
                opcode: actual,
            }) => {
                assert_eq!((address, actual), (0x200, opcode));
                assert_eq!(ram.trace, [R(0x200)]);
                assert_eq!(cpu.step(&mut ram), Err(Error::NeedsReset));
                ram.bytes[0xfffc..0xfffe].copy_from_slice(&[0, 4]);
                ram.bytes[0x400] = 0xea;
                cpu.reset(&mut ram).unwrap();
                assert!(!cpu.is_faulted());
                cpu.step(&mut ram).unwrap();
            }
            Err(error) => panic!("unexpected decode result {opcode:02x}: {error:?}"),
        }
    }
    assert_eq!(documented, 151);
}

#[test]
fn bus_errors_stop_at_the_failed_cycle_and_reset_failures_remain_faulted() {
    for failure in 1..=7 {
        let (cpu, mut ram) = setup(&[0xfe, 0x00, 0x04]);
        ram.fail = Some(failure);
        let mut cpu = Cpu::from_registers(cpu.registers());
        assert!(matches!(cpu.step(&mut ram), Err(Error::Bus(_))));
        assert_eq!(ram.trace.len(), failure);
        assert!(cpu.is_faulted());
        assert_eq!(cpu.step(&mut ram), Err(Error::NeedsReset));
        ram.trace.clear();
        ram.fail = Some(7);
        assert_eq!(cpu.reset(&mut ram), Err(Error::Bus("read rejected")));
        assert!(cpu.is_faulted());
        ram.trace.clear();
        ram.fail = None;
        cpu.reset(&mut ram).unwrap();
        assert!(!cpu.is_faulted());
    }
}

#[test]
fn bounded_execution_cancels_without_access_and_resumes_deterministically() {
    let (mut cpu, mut ram) = setup(&[0x4c, 0x00, 0x02]);
    for budget in [0, MAX_STEPS + 1, u32::MAX] {
        assert_eq!(
            cpu.run(&mut ram, budget, || false),
            Err(Error::InvalidBudget)
        );
    }
    assert!(ram.trace.is_empty());
    assert_eq!(cpu.run(&mut ram, MAX_STEPS, || true).unwrap().steps, 0);
    assert_eq!(
        cpu.run(&mut ram, 10, || true).unwrap().reason,
        StopReason::Cancelled
    );
    assert!(ram.trace.is_empty());
    let mut polls = 0;
    let result = cpu
        .run(&mut ram, 10, || {
            polls += 1;
            polls == 4
        })
        .unwrap();
    assert_eq!(
        (result.steps, result.cycles, result.reason),
        (3, 9, StopReason::Cancelled)
    );
    let resumed = cpu.run(&mut ram, 4, || false).unwrap();
    assert_eq!(
        (resumed.steps, resumed.cycles, resumed.reason),
        (4, 12, StopReason::Budget)
    );
    assert_eq!(cpu.registers().pc, 0x200);
}

#[test]
fn stack_push_pull_flags_and_nmi_priority_are_explicit() {
    let (cpu, mut ram) = setup(&[0x48, 0x08, 0x28, 0x68]);
    let mut cpu = Cpu::from_registers(Registers {
        a: 0x80,
        sp: 0,
        p: D | C,
        ..cpu.registers()
    });
    for cycles in [3, 3, 4, 4] {
        assert_eq!(cpu.step(&mut ram).unwrap().cycles, cycles);
    }
    assert_eq!(
        (cpu.registers().a, cpu.registers().sp, cpu.registers().p),
        (0x80, 0, N | D | C | U)
    );
    assert_eq!((ram.bytes[0x100], ram.bytes[0x1ff]), (0x80, D | C | U | B));
    ram.trace.clear();
    ram.bytes[0xfffa..0xfffc].copy_from_slice(&[0, 5]);
    cpu.set_irq(true);
    cpu.set_nmi(true);
    let step = cpu.step(&mut ram).unwrap();
    assert_eq!((step.kind, step.cycles), (StepKind::Nmi, 7));
    assert_eq!(cpu.registers().pc, 0x500);
    assert_eq!(&ram.trace[5..], &[R(0xfffa), R(0xfffb)]);
}

#[test]
fn zero_page_index_and_memory_increment_decrement_wrap_without_carry_changes() {
    for (opcode, initial, expected) in [(0xf6, 0xff, 0), (0xd6, 0, 0xff)] {
        let (cpu, mut ram) = setup(&[opcode, 0xff]);
        ram.bytes[0] = initial;
        let mut cpu = Cpu::from_registers(Registers {
            x: 1,
            p: C | V,
            ..cpu.registers()
        });
        assert_eq!(cpu.step(&mut ram).unwrap().cycles, 6);
        assert_eq!(ram.bytes[0], expected);
        assert_eq!(
            ram.trace,
            [
                R(0x200),
                R(0x201),
                R(0xff),
                R(0),
                W(0, initial),
                W(0, expected)
            ]
        );
        assert_eq!(
            cpu.registers().p,
            C | V | U | if expected == 0 { Z } else { N }
        );
    }
}

#[test]
fn shifts_rotates_and_compares_exhaustive() {
    for value in 0..=255u8 {
        for carry in [false, true] {
            for opcode in [0x0a, 0x2a, 0x4a, 0x6a] {
                let (_, mut ram) = setup(&[opcode]);
                let mut cpu = state(0x200, value, D | I | u8::from(carry));
                cpu.step(&mut ram).unwrap();
                let expected = match opcode {
                    0x0a => value.wrapping_mul(2),
                    0x2a => value.wrapping_mul(2) | u8::from(carry),
                    0x4a => value / 2,
                    _ => (value / 2) | if carry { 128 } else { 0 },
                };
                let c = if opcode == 0x0a || opcode == 0x2a {
                    value >= 128
                } else {
                    value % 2 != 0
                };
                assert_eq!(
                    (cpu.registers().a, cpu.registers().p),
                    (
                        expected,
                        D | I
                            | U
                            | u8::from(c)
                            | (expected & N)
                            | if expected == 0 { Z } else { 0 }
                    )
                );
            }
        }
        let (_, mut ram) = setup(&[0xc9, 0]);
        for operand in 0..=255u8 {
            ram.trace.clear();
            ram.bytes[0x201] = operand;
            let mut cpu = state(0x200, value, D | V);
            cpu.step(&mut ram).unwrap();
            let diff = value.wrapping_sub(operand);
            assert_eq!(
                cpu.registers().p,
                D | V
                    | U
                    | u8::from(value >= operand)
                    | (diff & N)
                    | if value == operand { Z } else { 0 }
            );
            assert_eq!(cpu.registers().a, value);
        }
    }
}
