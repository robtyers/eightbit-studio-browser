//! Independent original operand/result vectors for every documented addressing form.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_mos6502_core::{Bus, C, Cpu, I, N, Registers, U, V, Z};
struct Memory(Box<[u8]>);
impl Bus for Memory {
    type Error = core::convert::Infallible;
    fn read(&mut self, a: u16) -> Result<u8, Self::Error> {
        Ok(self.0[usize::from(a)])
    }
    fn write(&mut self, a: u16, v: u8) -> Result<(), Self::Error> {
        self.0[usize::from(a)] = v;
        Ok(())
    }
}
#[derive(Clone, Copy)]
enum Form {
    Immediate,
    Zero,
    ZeroX,
    ZeroY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    IndirectX,
    IndirectY,
}
type OperandCase<'a> = (&'a str, &'a [(u8, Form)], u8, u8);
use Form::{
    Absolute as Ab, AbsoluteX as Ax, AbsoluteY as Ay, Immediate as Im, IndirectX as Ix,
    IndirectY as Iy, Zero as Zp, ZeroX as Zx, ZeroY as Zy,
};
fn fixture(opcode: u8, form: Form) -> (Cpu, Memory, u16, u8) {
    let mut memory = Memory(vec![0; 65536].into_boxed_slice());
    memory.0[0x200] = opcode;
    let (operand, address, cycles) = match form {
        Im => ([0x42, 0], 0x201, 2),
        Zp => ([0x20, 0], 0x20, 3),
        Zx => ([0x20, 0], 0x24, 4),
        Zy => ([0x20, 0], 0x23, 4),
        Ab => ([0, 6], 0x600, 4),
        Ax => ([0xfc, 5], 0x600, 5),
        Ay => ([0xfd, 5], 0x600, 5),
        Ix => {
            memory.0[0x24] = 0;
            memory.0[0x25] = 6;
            ([0x20, 0], 0x600, 6)
        }
        Iy => {
            memory.0[0x20] = 0xfd;
            memory.0[0x21] = 5;
            ([0x20, 0], 0x600, 6)
        }
    };
    memory.0[0x201..0x203].copy_from_slice(&operand);
    memory.0[usize::from(address)] = 0x42;
    let cpu = Cpu::from_registers(Registers {
        pc: 0x200,
        a: 0x55,
        x: 4,
        y: 3,
        sp: 0xfd,
        p: C | I,
    });
    (cpu, memory, address, cycles)
}

#[test]
fn all_operand_instructions_produce_named_results_in_every_addressing_mode() {
    // Each row gives a semantic expectation independent of the production decoder.
    let cases: &[OperandCase] = &[
        (
            "lda",
            &[
                (0xa9, Im),
                (0xa5, Zp),
                (0xb5, Zx),
                (0xad, Ab),
                (0xbd, Ax),
                (0xb9, Ay),
                (0xa1, Ix),
                (0xb1, Iy),
            ],
            0x42,
            C | I | U,
        ),
        (
            "ldx",
            &[(0xa2, Im), (0xa6, Zp), (0xb6, Zy), (0xae, Ab), (0xbe, Ay)],
            0x42,
            C | I | U,
        ),
        (
            "ldy",
            &[(0xa0, Im), (0xa4, Zp), (0xb4, Zx), (0xac, Ab), (0xbc, Ax)],
            0x42,
            C | I | U,
        ),
        (
            "ora",
            &[
                (0x09, Im),
                (0x05, Zp),
                (0x15, Zx),
                (0x0d, Ab),
                (0x1d, Ax),
                (0x19, Ay),
                (0x01, Ix),
                (0x11, Iy),
            ],
            0x57,
            C | I | U,
        ),
        (
            "and",
            &[
                (0x29, Im),
                (0x25, Zp),
                (0x35, Zx),
                (0x2d, Ab),
                (0x3d, Ax),
                (0x39, Ay),
                (0x21, Ix),
                (0x31, Iy),
            ],
            0x40,
            C | I | U,
        ),
        (
            "eor",
            &[
                (0x49, Im),
                (0x45, Zp),
                (0x55, Zx),
                (0x4d, Ab),
                (0x5d, Ax),
                (0x59, Ay),
                (0x41, Ix),
                (0x51, Iy),
            ],
            0x17,
            C | I | U,
        ),
        (
            "adc",
            &[
                (0x69, Im),
                (0x65, Zp),
                (0x75, Zx),
                (0x6d, Ab),
                (0x7d, Ax),
                (0x79, Ay),
                (0x61, Ix),
                (0x71, Iy),
            ],
            0x98,
            I | U | N | V,
        ),
        (
            "sbc",
            &[
                (0xe9, Im),
                (0xe5, Zp),
                (0xf5, Zx),
                (0xed, Ab),
                (0xfd, Ax),
                (0xf9, Ay),
                (0xe1, Ix),
                (0xf1, Iy),
            ],
            0x13,
            C | I | U,
        ),
        (
            "cmp",
            &[
                (0xc9, Im),
                (0xc5, Zp),
                (0xd5, Zx),
                (0xcd, Ab),
                (0xdd, Ax),
                (0xd9, Ay),
                (0xc1, Ix),
                (0xd1, Iy),
            ],
            0x55,
            C | I | U,
        ),
        (
            "cpx",
            &[(0xe0, Im), (0xe4, Zp), (0xec, Ab)],
            0x55,
            I | U | N,
        ),
        (
            "cpy",
            &[(0xc0, Im), (0xc4, Zp), (0xcc, Ab)],
            0x55,
            I | U | N,
        ),
        ("bit", &[(0x24, Zp), (0x2c, Ab)], 0x55, C | I | U | V),
    ];
    let mut tested = 0;
    for &(name, forms, value, flags) in cases {
        for &(opcode, form) in forms {
            let (mut cpu, mut memory, _, cycles) = fixture(opcode, form);
            let before = cpu.registers();
            let actual = cpu.step(&mut memory).unwrap();
            let after = cpu.registers();
            let expected = match name {
                "ldx" => Registers {
                    x: value,
                    p: flags,
                    ..before
                },
                "ldy" => Registers {
                    y: value,
                    p: flags,
                    ..before
                },
                _ => Registers {
                    a: value,
                    p: flags,
                    ..before
                },
            };
            assert_eq!(
                Registers {
                    pc: before.pc,
                    ..after
                },
                expected,
                "{name} opcode {opcode:02x}"
            );
            assert_eq!(actual.cycles, cycles, "{name} {opcode:02x}");
            assert_eq!(
                after.pc,
                if matches!(form, Ab | Ax | Ay) {
                    0x203
                } else {
                    0x202
                }
            );
            tested += 1;
        }
    }
    assert_eq!(tested, 74);
}

#[test]
fn all_store_and_memory_modify_forms_preserve_or_update_the_right_state() {
    let cases: &[OperandCase] = &[
        (
            "sta",
            &[
                (0x85, Zp),
                (0x95, Zx),
                (0x8d, Ab),
                (0x9d, Ax),
                (0x99, Ay),
                (0x81, Ix),
                (0x91, Iy),
            ],
            0x55,
            C | I | U,
        ),
        ("stx", &[(0x86, Zp), (0x96, Zy), (0x8e, Ab)], 4, C | I | U),
        ("sty", &[(0x84, Zp), (0x94, Zx), (0x8c, Ab)], 3, C | I | U),
        (
            "asl",
            &[(0x06, Zp), (0x16, Zx), (0x0e, Ab), (0x1e, Ax)],
            0x84,
            N | I | U,
        ),
        (
            "rol",
            &[(0x26, Zp), (0x36, Zx), (0x2e, Ab), (0x3e, Ax)],
            0x85,
            N | I | U,
        ),
        (
            "lsr",
            &[(0x46, Zp), (0x56, Zx), (0x4e, Ab), (0x5e, Ax)],
            0x21,
            I | U,
        ),
        (
            "ror",
            &[(0x66, Zp), (0x76, Zx), (0x6e, Ab), (0x7e, Ax)],
            0xa1,
            N | I | U,
        ),
        (
            "dec",
            &[(0xc6, Zp), (0xd6, Zx), (0xce, Ab), (0xde, Ax)],
            0x41,
            C | I | U,
        ),
        (
            "inc",
            &[(0xe6, Zp), (0xf6, Zx), (0xee, Ab), (0xfe, Ax)],
            0x43,
            C | I | U,
        ),
    ];
    let mut tested = 0;
    for &(name, forms, value, flags) in cases {
        for &(opcode, form) in forms {
            let (mut cpu, mut memory, address, cycles) = fixture(opcode, form);
            let before = cpu.registers();
            let step = cpu.step(&mut memory).unwrap();
            let after = cpu.registers();
            let modify = !name.starts_with("st");
            let expected_cycles = cycles + if modify { 2 } else { 0 };
            assert_eq!(step.cycles, expected_cycles, "{name} {opcode:02x}");
            assert_eq!(memory.0[usize::from(address)], value);
            assert_eq!(
                Registers {
                    pc: before.pc,
                    ..after
                },
                Registers { p: flags, ..before }
            );
            tested += 1;
        }
    }
    assert_eq!(tested, 37);
}

#[test]
fn implied_transfers_flags_and_inc_dec_cover_zero_and_wrap() {
    for (opcode, expected) in [
        (
            0xaa,
            Registers {
                x: 0,
                p: Z | I | U | C,
                ..Registers {
                    a: 0,
                    x: 0xff,
                    y: 1,
                    sp: 0x80,
                    p: I | U | C,
                    pc: 0x200,
                }
            },
        ),
        (
            0xa8,
            Registers {
                y: 0,
                p: Z | I | U | C,
                ..Registers {
                    a: 0,
                    x: 0xff,
                    y: 1,
                    sp: 0x80,
                    p: I | U | C,
                    pc: 0x200,
                }
            },
        ),
        (
            0x8a,
            Registers {
                a: 0xff,
                p: N | I | U | C,
                ..Registers {
                    a: 0,
                    x: 0xff,
                    y: 1,
                    sp: 0x80,
                    p: I | U | C,
                    pc: 0x200,
                }
            },
        ),
        (
            0x98,
            Registers {
                a: 1,
                p: I | U | C,
                ..Registers {
                    a: 0,
                    x: 0xff,
                    y: 1,
                    sp: 0x80,
                    p: I | U | C,
                    pc: 0x200,
                }
            },
        ),
        (
            0xba,
            Registers {
                x: 0x80,
                p: N | I | U | C,
                ..Registers {
                    a: 0,
                    x: 0xff,
                    y: 1,
                    sp: 0x80,
                    p: I | U | C,
                    pc: 0x200,
                }
            },
        ),
        (
            0x9a,
            Registers {
                sp: 0xff,
                ..Registers {
                    a: 0,
                    x: 0xff,
                    y: 1,
                    sp: 0x80,
                    p: I | U | C,
                    pc: 0x200,
                }
            },
        ),
        (
            0xe8,
            Registers {
                x: 0,
                p: Z | I | U | C,
                ..Registers {
                    a: 0,
                    x: 0xff,
                    y: 1,
                    sp: 0x80,
                    p: I | U | C,
                    pc: 0x200,
                }
            },
        ),
        (
            0xc8,
            Registers {
                y: 2,
                ..Registers {
                    a: 0,
                    x: 0xff,
                    y: 1,
                    sp: 0x80,
                    p: I | U | C,
                    pc: 0x200,
                }
            },
        ),
        (
            0xca,
            Registers {
                x: 0xfe,
                p: N | I | U | C,
                ..Registers {
                    a: 0,
                    x: 0xff,
                    y: 1,
                    sp: 0x80,
                    p: I | U | C,
                    pc: 0x200,
                }
            },
        ),
        (
            0x88,
            Registers {
                y: 0,
                p: Z | I | U | C,
                ..Registers {
                    a: 0,
                    x: 0xff,
                    y: 1,
                    sp: 0x80,
                    p: I | U | C,
                    pc: 0x200,
                }
            },
        ),
    ] {
        let mut memory = Memory(vec![0; 65536].into_boxed_slice());
        memory.0[0x200] = opcode;
        let mut cpu = Cpu::from_registers(Registers {
            a: 0,
            x: 0xff,
            y: 1,
            sp: 0x80,
            p: I | U | C,
            pc: 0x200,
        });
        assert_eq!(cpu.step(&mut memory).unwrap().cycles, 2);
        assert_eq!(
            cpu.registers(),
            Registers {
                pc: 0x201,
                ..expected
            }
        );
    }
    for (opcode, initial, expected) in [
        (0x18, 0xff, 0xfe),
        (0x38, 0, 1),
        (0x58, 0xff, 0xfb),
        (0x78, 0, 4),
        (0xb8, 0xff, 0xbf),
        (0xd8, 0xff, 0xf7),
        (0xf8, 0, 8),
        (0xea, 0xc5, 0xc5),
    ] {
        let mut memory = Memory(vec![0; 65536].into_boxed_slice());
        memory.0[0x200] = opcode;
        let mut cpu = Cpu::from_registers(Registers {
            pc: 0x200,
            p: initial,
            ..Registers::default()
        });
        cpu.step(&mut memory).unwrap();
        assert_eq!(cpu.registers().p, (expected | U) & !0x10);
    }
}
