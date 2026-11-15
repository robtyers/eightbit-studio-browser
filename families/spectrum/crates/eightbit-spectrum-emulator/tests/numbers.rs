//! Independent host f64 oracle for the original guest five-byte arithmetic.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
use eightbit_spectrum_emulator::Machine;
use eightbit_z80_core::Bus;

fn encoded(value: f64) -> [u8; 5] {
    if value == 0.0 {
        return [0; 5];
    }
    let absolute = value.abs();
    let mut power = absolute.log2().floor() as i32;
    let fraction = absolute / 2_f64.powi(power) - 1.0;
    let mut significand = (fraction * 2_147_483_648.0).round() as u64;
    if significand == 2_147_483_648 {
        significand = 0;
        power += 1;
    }
    assert!((-128..=126).contains(&power));
    let mut mantissa = u32::try_from(significand).unwrap().to_be_bytes();
    if value < 0.0 {
        mantissa[0] |= 0x80;
    }
    [
        (power + 129) as u8,
        mantissa[0],
        mantissa[1],
        mantissa[2],
        mantissa[3],
    ]
}
fn decoded(bytes: [u8; 5]) -> f64 {
    if bytes[0] == 0 {
        let raw = u16::from_le_bytes([bytes[2], bytes[3]]);
        return f64::from(raw) - if bytes[1] == 0 { 0.0 } else { 65536.0 };
    }
    let mantissa = u32::from_be_bytes([bytes[1] & 0x7f, bytes[2], bytes[3], bytes[4]]);
    let sign = if bytes[1] & 0x80 == 0 { 1.0 } else { -1.0 };
    sign * (1.0 + f64::from(mantissa) / 2_147_483_648.0) * 2_f64.powi(i32::from(bytes[0]) - 129)
}
fn calculate(a: [u8; 5], b: [u8; 5], operation: u8) -> [u8; 5] {
    calculate_stack(&[a, b], operation)
}
fn calculate_stack(values: &[[u8; 5]], operation: u8) -> [u8; 5] {
    let mut machine = Machine::development().unwrap();
    let code = [0x3e, operation, 0xcd, 0, 0x1c, 0x76];
    for (offset, byte) in code.iter().enumerate() {
        machine
            .board
            .write(0x8000 + u16::try_from(offset).unwrap(), *byte);
    }
    for (offset, byte) in values.iter().flatten().enumerate() {
        machine
            .board
            .write(0x9000 + u16::try_from(offset).unwrap(), *byte);
    }
    let end = 0x9000 + u16::try_from(values.len() * 5).unwrap();
    for (address, value) in [(0x5c63, 0x9000_u16), (0x5c65, end)] {
        let [lo, hi] = value.to_le_bytes();
        machine.board.write(address, lo);
        machine.board.write(address + 1, hi);
    }
    machine.cpu.registers.pc = 0x8000;
    machine.cpu.registers.sp = 0xff00;
    machine.cpu.registers.iy = 0x5c3a;
    let mut clocks = 0;
    while !machine.cpu.halted && clocks < 150_000 {
        clocks += machine.step().unwrap();
    }
    assert!(
        machine.cpu.halted,
        "guest numeric call did not return; PC {:04x}",
        machine.cpu.registers.pc
    );
    assert_eq!(machine.board.firmware_page(), 0);
    assert_eq!(machine.cpu.registers.iy, 0x5c3a);
    assert_eq!(machine.board.peek(0x5c65), 5);
    assert_eq!(machine.board.peek(0x5c66), 0x90);
    let mut result = [0; 5];
    for (offset, byte) in result.iter_mut().enumerate() {
        *byte = machine.board.peek(0x9000 + u16::try_from(offset).unwrap());
    }
    result
}

#[test]
fn random_sequence_uses_the_published_modulo_65537_recurrence() {
    for seed in (0..=u16::MAX).step_by(127).chain([0, 1, 65534, 65535]) {
        let mut machine = Machine::development().unwrap();
        for (address, value) in [(0x5c76, seed), (0x5c63, 0x9000), (0x5c65, 0x9000)] {
            let [lo, hi] = value.to_le_bytes();
            machine.board.write(address, lo);
            machine.board.write(address + 1, hi);
        }
        for (offset, byte) in [0x3e, 10, 0xcd, 0, 0x1c, 0x76].into_iter().enumerate() {
            machine
                .board
                .write(0x8000 + u16::try_from(offset).unwrap(), byte);
        }
        machine.cpu.registers.pc = 0x8000;
        machine.cpu.registers.sp = 0xff00;
        machine.run(20000).unwrap();
        assert!(machine.cpu.halted);
        let expected = u16::try_from((75 * (u32::from(seed) + 1)) % 65537 - 1).unwrap();
        let actual = u16::from_le_bytes([machine.board.peek(0x5c76), machine.board.peek(0x5c77)]);
        assert_eq!(actual, expected, "seed {seed}");
        let mut value = [0; 5];
        for (offset, byte) in value.iter_mut().enumerate() {
            *byte = machine.board.peek(0x9000 + u16::try_from(offset).unwrap());
        }
        assert!((decoded(value) - f64::from(expected) / 65536.0).abs() < 1e-15);
    }
}

#[test]
fn unary_sign_absolute_floor_and_not_execute_on_the_guest_stack() {
    for value in [
        -65536.25_f64,
        -256.0,
        -2.5,
        -1.0,
        -0.75,
        -0.1,
        0.0,
        0.1,
        0.5,
        1.0,
        2.75,
        65535.5,
        1e15,
    ] {
        let input = encoded(value);
        let value = decoded(input);
        for (operation, expected) in [
            (4, -value),
            (5, value.abs()),
            (6, if value == 0.0 { 0.0 } else { value.signum() }),
            (7, value.floor()),
            (9, f64::from(value == 0.0)),
        ] {
            assert_eq!(
                calculate_stack(&[input], operation),
                encoded(expected),
                "unary {operation} {value}"
            );
        }
    }
}

#[test]
fn rounded_unsigned_conversion_preserves_the_public_compact_integer_format() {
    for value in [
        -0.49_f64, 0.0, 0.49, 0.5, 0.51, 1.49, 1.5, 2.75, 32767.5, 65535.0, 65535.49,
    ] {
        let input = encoded(value);
        let expected = decoded(input).round() as u16;
        let [lo, hi] = expected.to_le_bytes();
        assert_eq!(
            calculate_stack(&[input], 8),
            [0, 0, lo, hi, 0],
            "word conversion {value}"
        );
    }
}

#[test]
#[allow(clippy::float_cmp)] // BASIC equality is exact for these decoded finite operands.
fn numeric_comparisons_and_sinclair_logical_results_cover_zero_and_signs() {
    for a in [-100.0_f64, -0.1, 0.0, 0.1, 1.0, 100.0] {
        for b in [-100.0_f64, -0.1, 0.0, 0.1, 1.0, 100.0] {
            let aa = encoded(a);
            let bb = encoded(b);
            let a = decoded(aa);
            let b = decoded(bb);
            for (operation, result) in [
                (16, f64::from(a == b)),
                (17, f64::from(a < b)),
                (18, f64::from(a > b)),
                (19, f64::from(a <= b)),
                (20, f64::from(a >= b)),
                (21, f64::from(a != b)),
                (22, if b == 0.0 { 0.0 } else { a }),
                (23, if b == 0.0 { a } else { 1.0 }),
            ] {
                assert_eq!(
                    calculate(aa, bb, operation),
                    encoded(result),
                    "comparison {a} {operation} {b}"
                );
            }
        }
    }
}

#[test]
fn five_byte_guest_add_and_subtract_cover_sign_alignment_cancellation_and_rounding() {
    for a in [
        0.0,
        1.0,
        -1.0,
        3.0,
        -255.0,
        65535.0,
        -65536.0,
        0.1,
        0.5,
        1e-12,
        1e15,
        2_147_483_648.0,
        4_294_967_295.0,
    ] {
        for b in [0.0, 1.0, -1.0, 0.5, 0.1, -255.0, 65536.0, 1e-15, 1e15] {
            for operation in 0..=1 {
                let aa = encoded(a);
                let bb = encoded(b);
                let result = if operation == 0 {
                    decoded(aa) + decoded(bb)
                } else {
                    decoded(aa) - decoded(bb)
                };
                assert_eq!(
                    calculate(aa, bb, operation),
                    encoded(result),
                    "{a} {operation} {b}"
                );
            }
        }
    }
}

#[test]
fn compact_integer_inputs_have_the_same_guest_numeric_value_as_normalised_inputs() {
    for (raw, value) in [
        ([0, 0, 0, 0, 0], 0.0),
        ([0, 0, 1, 0, 0], 1.0),
        ([0, 0, 255, 255, 0], 65535.0),
        ([0, 255, 255, 255, 0], -1.0),
        ([0, 255, 0, 128, 0], -32768.0),
        ([0, 255, 0, 0, 0], -65536.0),
    ] {
        assert_eq!(calculate(raw, encoded(0.0), 0), encoded(value));
        assert_eq!(calculate(raw, encoded(0.25), 0), encoded(value + 0.25));
        assert_eq!(calculate(encoded(0.0), raw, 1), encoded(-value));
    }
}

#[test]
fn original_random_five_byte_addition_corpus_matches_independent_arithmetic() {
    let mut seed = 0x5a17_c039_u32;
    for _ in 0..512 {
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed
        };
        let a = next();
        let b = next();
        let aa = [
            (60 + (a % 130)) as u8,
            (a >> 24) as u8,
            (a >> 16) as u8,
            (a >> 8) as u8,
            a as u8,
        ];
        let bb = [
            (60 + (b % 130)) as u8,
            (b >> 24) as u8,
            (b >> 16) as u8,
            (b >> 8) as u8,
            b as u8,
        ];
        for operation in 0..=1 {
            let value = if operation == 0 {
                decoded(aa) + decoded(bb)
            } else {
                decoded(aa) - decoded(bb)
            };
            assert_eq!(
                calculate(aa, bb, operation),
                encoded(value),
                "{aa:?} {operation} {bb:?}"
            );
        }
    }
}

#[test]
fn guest_multiplication_and_division_retain_the_same_five_byte_value() {
    for a in [
        0.0,
        1.0,
        -1.0,
        2.0,
        -3.0,
        0.1,
        0.5,
        1e-12,
        1e15,
        65535.0,
        -65536.0,
        4_294_967_295.0,
    ] {
        for b in [1.0, -1.0, 2.0, -3.0, 0.1, 0.5, 1e-12, 65535.0] {
            let aa = encoded(a);
            let bb = encoded(b);
            assert_eq!(
                calculate(aa, bb, 2),
                encoded(decoded(aa) * decoded(bb)),
                "{a} * {b}"
            );
            assert_eq!(
                calculate(aa, bb, 3),
                encoded(decoded(aa) / decoded(bb)),
                "{a} / {b}"
            );
        }
    }
}
