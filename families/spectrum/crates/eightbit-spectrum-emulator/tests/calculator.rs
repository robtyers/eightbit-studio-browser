//! Published RST $28 programmes, written independently of any ROM implementation.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_spectrum_emulator::{MAX_SLICE, Machine};
use eightbit_z80_core::Bus;

fn word(machine: &Machine, address: u16) -> u16 {
    u16::from_le_bytes([machine.board.peek(address), machine.board.peek(address + 1)])
}
fn set_word(machine: &mut Machine, address: u16, value: u16) {
    for (offset, byte) in value.to_le_bytes().into_iter().enumerate() {
        machine
            .board
            .write(address + u16::try_from(offset).unwrap(), byte);
    }
}
fn prepared(bytes: &[u8], counter: u8) -> Machine {
    prepared_on(Machine::development().unwrap(), bytes, counter)
}
fn prepared_on(mut machine: Machine, bytes: &[u8], counter: u8) -> Machine {
    for _ in 0..24 {
        machine.run(MAX_SLICE).unwrap();
        machine.board.drain_audio();
    }
    for (offset, byte) in [0xef]
        .into_iter()
        .chain(bytes.iter().copied())
        .chain([0x76])
        .enumerate()
    {
        machine
            .board
            .write(0x8000 + u16::try_from(offset).unwrap(), byte);
    }
    for (address, value) in [
        (0x5c3d, 0xff00),
        (0xff00, 0x8100),
        (0x5c63, 0x9000),
        (0x5c65, 0x9000),
    ] {
        set_word(&mut machine, address, value);
    }
    machine.board.write(0x8100, 0x76);
    machine.cpu.registers.pc = 0x8000;
    machine.cpu.registers.sp = 0xff00;
    machine.cpu.registers.ix = 0x1234;
    machine.cpu.registers.iy = 0x5c3a;
    machine.cpu.registers.b = counter;
    machine.cpu.registers.iff1 = false;
    machine.cpu.registers.iff2 = false;
    machine.cpu.halted = false;
    machine
}
fn finish(machine: &mut Machine, length: usize) {
    for _ in 0..30 {
        machine.run(MAX_SLICE).unwrap();
        machine.board.drain_audio();
        if machine.cpu.halted {
            break;
        }
    }
    assert!(machine.cpu.halted, "PC {:04x}", machine.cpu.registers.pc);
    assert_eq!(
        machine.cpu.registers.pc,
        0x8002 + u16::try_from(length).unwrap(),
        "report {}",
        machine.board.peek(0x5c3a)
    );
    assert_eq!(machine.cpu.registers.ix, 0x1234);
    assert_eq!(machine.cpu.registers.iy, 0x5c3a);
    assert_eq!(machine.board.firmware_page(), 0);
    assert_eq!(machine.cpu.registers.de(), word(machine, 0x5c65));
    assert_eq!(machine.cpu.registers.hl(), word(machine, 0x5c65) - 5);
}
fn number(machine: &Machine, address: u16) -> f64 {
    let bytes: [u8; 5] =
        std::array::from_fn(|offset| machine.board.peek(address + u16::try_from(offset).unwrap()));
    if bytes[0] == 0 {
        return f64::from(u16::from_le_bytes([bytes[2], bytes[3]]))
            - if bytes[1] == 0 { 0.0 } else { 65536.0 };
    }
    let mantissa = u32::from_be_bytes([bytes[1] & 127, bytes[2], bytes[3], bytes[4]]);
    let sign = if bytes[1] & 128 == 0 { 1.0 } else { -1.0 };
    sign * (1.0 + f64::from(mantissa) / 2_147_483_648.0) * 2_f64.powi(i32::from(bytes[0]) - 129)
}

#[test]
fn rst_calculator_arithmetic_and_return_registers() {
    let code = [0xa1, 0xa4, 0x04, 0x31, 0xa2, 0x04, 0x0f, 0x38];
    let mut machine = prepared(&code, 0);
    finish(&mut machine, code.len());
    assert_eq!(word(&machine, 0x5c65), 0x9005);
    assert!((number(&machine, 0x9000) - 15.0).abs() < 1e-9);
}

#[test]
fn memory_cells_survive_arithmetic_and_conditional_and_counted_jumps() {
    let code = [0xa1, 0xc0, 0x02, 0xa4, 0xc1, 0x02, 0xe0, 0xe1, 0x04, 0x38];
    let mut machine = prepared(&code, 0);
    finish(&mut machine, code.len());
    assert!((number(&machine, 0x9000) - 10.0).abs() < 1e-9);
    assert!((number(&machine, 0x5c92) - 1.0).abs() < 1e-9);
    assert!((number(&machine, 0x5c97) - 10.0).abs() < 1e-9);
    let code = [0xa0, 0xa1, 0x0f, 0x35, 0xfd, 0x38];
    let mut machine = prepared(&code, 3);
    finish(&mut machine, code.len());
    assert!((number(&machine, 0x9000) - 3.0).abs() < 1e-9);
    assert_eq!(machine.board.peek(0x5c67), 0);
    for (condition, expected) in [(0xa0, 0.0), (0xa1, 10.0)] {
        let code = [condition, 0x00, 0x04, 0xa0, 0x38, 0x76, 0xa4, 0x38];
        let mut machine = prepared(&code, 0);
        // The false branch ends before the final alternative.
        finish(&mut machine, if condition == 0xa0 { 5 } else { code.len() });
        assert!((number(&machine, 0x9000) - expected).abs() < 1e-9);
    }
}

#[test]
fn compressed_numbers_modulo_and_guest_maths() {
    let code = [
        0x34, 0x80, 0xb0, 0, 0x2c, 1, 0x34, 0x40, 0xb0, 0, 7, 0x32, 0x38,
    ];
    let mut machine = prepared(&code, 0);
    finish(&mut machine, code.len());
    assert_eq!(word(&machine, 0x5c65), 0x900a);
    assert!((number(&machine, 0x9000) - 6.0).abs() < 1e-9);
    assert!((number(&machine, 0x9005) - 42.0).abs() < 1e-9);
    let code = [0xa3, 0x1f, 0x38];
    let mut machine = prepared(&code, 99);
    finish(&mut machine, code.len());
    assert!((number(&machine, 0x9000) - 1.0).abs() < 2e-7);
    assert_eq!(machine.cpu.registers.b, 0);
    assert_eq!(machine.board.peek(0x5c67), 0);
}

#[test]
fn polynomial_series_evaluates_independently_chosen_chebyshev_coefficients() {
    // x=1/2, coefficients 2,3,4: 4*T2(x) + 6*T1(x) + 4 = 5.
    let code = [
        0xa2, 0x83, 0x40, 0xb0, 0, 2, 0x40, 0xb0, 0, 3, 0x40, 0xb0, 0, 4, 0x38,
    ];
    let mut machine = prepared(&code, 0);
    finish(&mut machine, code.len());
    assert!((number(&machine, 0x9000) - 5.0).abs() < 1e-9);
    assert_eq!(machine.board.peek(0x5c67), 0);
}

#[test]
fn string_descriptors_and_concatenation_share_the_checked_guest_workspace() {
    let code = [
        0x34, 0xb0, 0x00, 0xa0, 3, 0x34, 0xb0, 0x10, 0xa0, 2, 0x17, 0x1e, 0x38,
    ];
    let mut machine = prepared(&code, 0);
    for (address, text) in [(0xa000, b"abc".as_slice()), (0xa010, b"de".as_slice())] {
        for (offset, byte) in text.iter().copied().enumerate() {
            machine
                .board
                .write(address + u16::try_from(offset).unwrap(), byte);
        }
    }
    finish(&mut machine, code.len());
    assert!((number(&machine, word(&machine, 0x5c65) - 5) - 5.0).abs() < 1e-9);
}

#[test]
fn underflow_reports_and_a_bytecode_loop_remains_host_cancellable() {
    let mut machine = prepared(&[0x02, 0x38], 0);
    machine.run(MAX_SLICE).unwrap();
    assert!(machine.cpu.halted);
    assert_eq!(machine.cpu.registers.pc, 0x8101);
    assert_eq!(machine.board.peek(0x5c3a), 3);
    assert_eq!(machine.board.firmware_page(), 0);
    let mut machine = prepared(&[0x33, 0xff], 0);
    let elapsed = machine.run(MAX_SLICE).unwrap();
    assert!((MAX_SLICE..MAX_SLICE + 100).contains(&elapsed));
    assert!(!machine.cpu.halted);
    machine.reset();
    assert_eq!(machine.cpu.registers.pc, 0);
}

#[test]
#[ignore = "requires an explicitly supplied, exact-hash UK 48K reference ROM"]
fn lawful_reference_calculator_matches_original_independent_programmes() {
    use eightbit_spectrum_emulator::Firmware;
    use sha2::{Digest, Sha256};
    let path = std::env::var_os("EIGHTBIT_SPECTRUM48_REFERENCE_ROM")
        .expect("set EIGHTBIT_SPECTRUM48_REFERENCE_ROM to a lawfully obtained UK 48K ROM");
    let bytes = std::fs::read(path).expect("read reference ROM");
    assert_eq!(bytes.len(), 16384, "reference ROM must be 16 KiB");
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        "d55daa439b673b0e3f5897f99ac37ecb45f974d1862b4dadb85dec34af99cb42",
        "unrecognised reference ROM"
    );
    let programmes: &[(&[u8], u8, &[f64])] = &[
        (
            &[0xa1, 0xa4, 0x04, 0x31, 0xa2, 0x04, 0x0f, 0x38],
            0,
            &[15.0],
        ),
        (
            &[0xa1, 0xc0, 0x02, 0xa4, 0xc1, 0x02, 0xe0, 0xe1, 0x04, 0x38],
            0,
            &[10.0],
        ),
        (&[0xa0, 0xa1, 0x0f, 0x35, 0xfd, 0x38], 3, &[3.0]),
        (
            &[
                0x34, 0x80, 0xb0, 0, 0x2c, 1, 0x34, 0x40, 0xb0, 0, 7, 0x32, 0x38,
            ],
            0,
            &[6.0, 42.0],
        ),
        (&[0xa3, 0x1f, 0x38], 99, &[1.0]),
        (&[0xa2, 0x81, 0x40, 0xb0, 0, 2, 0x38], 0, &[2.0]),
        (
            &[0xa2, 0x82, 0x40, 0xb0, 0, 2, 0x40, 0xb0, 0, 4, 0x38],
            0,
            &[6.0],
        ),
        (
            &[
                0xa2, 0x83, 0x40, 0xb0, 0, 2, 0x40, 0xb0, 0, 3, 0x40, 0xb0, 0, 4, 0x38,
            ],
            0,
            &[5.0],
        ),
        (
            &[
                0xa0, 0x83, 0x40, 0xb0, 0, 2, 0x40, 0xb0, 0, 3, 0x40, 0xb0, 0, 4, 0x38,
            ],
            0,
            &[0.0],
        ),
        (
            &[
                0xa1, 0x83, 0x40, 0xb0, 0, 2, 0x40, 0xb0, 0, 3, 0x40, 0xb0, 0, 4, 0x38,
            ],
            0,
            &[14.0],
        ),
    ];
    for (code, counter, expected) in programmes {
        for reference in [false, true] {
            let machine = if reference {
                Machine::new(Firmware::new(&bytes).unwrap())
            } else {
                Machine::development().unwrap()
            };
            let mut machine = prepared_on(machine, code, *counter);
            for _ in 0..30 {
                machine.run(MAX_SLICE).unwrap();
                machine.board.drain_audio();
                if machine.cpu.halted {
                    break;
                }
            }
            assert!(
                machine.cpu.halted,
                "reference {reference}, PC {:04x}",
                machine.cpu.registers.pc
            );
            assert_eq!(
                machine.cpu.registers.pc,
                0x8002 + u16::try_from(code.len()).unwrap(),
                "reference {reference}, report {}, MEM {:04x}",
                machine.board.peek(0x5c3a),
                word(&machine, 0x5c68)
            );
            assert_eq!(
                word(&machine, 0x5c65),
                0x9000 + u16::try_from(expected.len() * 5).unwrap()
            );
            if *counter == 99 {
                assert_eq!(
                    machine.board.peek(0x5c67),
                    0,
                    "reference {reference}: sine calculator register"
                );
            }
            for (index, expected) in expected.iter().enumerate() {
                let actual = number(&machine, 0x9000 + u16::try_from(index * 5).unwrap());
                assert!(
                    (actual - expected).abs() < 2e-7,
                    "reference {reference}: {actual} != {expected}"
                );
            }
        }
    }
}
