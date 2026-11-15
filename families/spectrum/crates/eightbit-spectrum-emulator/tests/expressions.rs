//! Original guest parser tests: no host evaluator is used by the runtime.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_spectrum_emulator::Machine;
use eightbit_z80_core::Bus;

fn set_word(machine: &mut Machine, address: u16, value: u16) {
    let [lo, hi] = value.to_le_bytes();
    machine.board.write(address, lo);
    machine.board.write(address + 1, hi);
}
fn word(machine: &Machine, address: u16) -> u16 {
    u16::from_le_bytes([machine.board.peek(address), machine.board.peek(address + 1)])
}
fn evaluate(source: &[u8]) -> Machine {
    let mut machine = Machine::development().unwrap();
    for (address, value) in [
        (0x5c5d, 0x9000),
        (0x5c63, 0xa000),
        (0x5c65, 0xa000),
        (0x5c3d, 0xff00),
        (0xff00, 0x8100),
        (0x5c4b, 0x5ccb),
        (0x5c53, 0x5ccb),
        (0x5c59, 0x5ccc),
        (0x5cb2, 0xff57),
        (0x5c7b, 0xff58),
    ] {
        set_word(&mut machine, address, value);
    }
    for (offset, byte) in source.iter().copied().chain([13]).enumerate() {
        machine
            .board
            .write(0x9000 + u16::try_from(offset).unwrap(), byte);
    }
    for (offset, byte) in [0xcd, 0x40, 0x1c, 0x76].into_iter().enumerate() {
        machine
            .board
            .write(0x8000 + u16::try_from(offset).unwrap(), byte);
    }
    machine.board.write(0x8100, 0x76);
    machine.board.write(0xc000, 0xc9);
    machine.board.write(0x5ccb, 0x80);
    machine.cpu.registers.pc = 0x8000;
    machine.cpu.registers.sp = 0xff00;
    let mut clocks = 0;
    while !machine.cpu.halted && clocks < 10_000_000 {
        clocks += machine.step().unwrap();
    }
    assert!(
        machine.cpu.halted,
        "parser {source:?} stuck at {:04x}",
        machine.cpu.registers.pc
    );
    machine
}
fn number(machine: &Machine) -> f64 {
    assert_eq!(
        machine.cpu.registers.pc,
        0x8004,
        "guest error {}",
        machine.board.peek(0x5c3a)
    );
    let base = word(machine, 0x5c63);
    assert_eq!(word(machine, 0x5c65), base + 5);
    let bytes: Vec<_> = (base..base + 5).map(|a| machine.board.peek(a)).collect();
    if bytes[0] == 0 {
        return f64::from(u16::from_le_bytes([bytes[2], bytes[3]]))
            - if bytes[1] == 0 { 0.0 } else { 65536.0 };
    }
    let fraction = u32::from_be_bytes([bytes[1] & 0x7f, bytes[2], bytes[3], bytes[4]]);
    let sign = if bytes[1] & 128 == 0 { 1.0 } else { -1.0 };
    sign * (1.0 + f64::from(fraction) / 2_147_483_648.0) * 2_f64.powi(i32::from(bytes[0]) - 129)
}

#[test]
fn precedence_parentheses_unary_and_sinclair_logical_values() {
    for (source, expected) in [
        (b"2+3*4".as_slice(), 14.0),
        (b"(2+3)*4", 20.0),
        (b"-7+2*3", -1.0),
        (b"8/4/2", 1.0),
        (b"10-3-2", 5.0),
        (b"2<3+4", 1.0),
        (&[195, b'2', b'=', b'3'], 1.0),
        (&[b'7', 198, b'2'], 7.0),
        (&[b'7', 197, b'2'], 1.0),
        (&[189, b'-', b'4', b'+', b'2'], 6.0),
        (&[186, b'(', b'-', b'2', b'.', b'5', b')'], -3.0),
        (&[196, b'1', b'0', b'1', b'1', b'+', b'1'], 12.0),
    ] {
        let machine = evaluate(source);
        assert!((number(&machine) - expected).abs() < 1e-8, "{source:?}");
        assert_eq!(machine.board.peek(word(&machine, 0x5c5d)), 13);
        assert_eq!(machine.board.firmware_page(), 0);
    }
}

#[test]
fn decimal_exponents_and_hidden_payloads_are_evaluated_in_guest_code() {
    for (source, expected) in [
        (b"1.25+2.5".as_slice(), 3.75),
        (b".125*8", 1.0),
        (b"25E-2", 0.25),
        (b"2.5e+2", 250.0),
        (b"0.0001*10000", 1.0),
        (&[b'9', 14, 0, 0, 42, 0, 0, b'+', b'1'], 43.0),
        (
            &[196, b'1', b'0', b'1', 14, 0, 0, 42, 0, 0, b'+', b'1'],
            43.0,
        ),
    ] {
        let machine = evaluate(source);
        assert!((number(&machine) - expected).abs() < 1e-7, "{source:?}");
    }
}

#[test]
fn malformed_inputs_and_missing_variables_use_the_public_error_return() {
    for (source, error) in [
        (b".".as_slice(), 11),
        (b"(1+2", 11),
        (b"1E+", 11),
        (b"1/0", 5),
        (b"MISSING", 1),
        (b"1+", 11),
    ] {
        let machine = evaluate(source);
        assert_eq!(machine.cpu.registers.pc, 0x8101, "{source:?}");
        assert_eq!(machine.board.peek(0x5c3a), error, "{source:?}");
        assert_eq!(machine.board.firmware_page(), 0);
    }
}

#[test]
fn peek_port_input_and_user_machine_code_use_guest_addresses() {
    assert!((number(&evaluate(&[190, b'3', b'2', b'7', b'6', b'8'])) - 205.0).abs() < 1e-8);
    // The address holds RET, so USR returns the original address in BC.
    assert!((number(&evaluate(&[192, b'4', b'9', b'1', b'5', b'2'])) - 49152.0).abs() < 1e-8);
}

fn string(machine: &Machine) -> Vec<u8> {
    assert_eq!(
        machine.cpu.registers.pc,
        0x8004,
        "guest error {}",
        machine.board.peek(0x5c3a)
    );
    assert_eq!(machine.board.peek(0x5c3b) & 64, 0);
    let descriptor = word(machine, 0x5c65) - 5;
    let address = word(machine, descriptor + 1);
    let length = word(machine, descriptor + 3);
    (address..address + length)
        .map(|a| machine.board.peek(a))
        .collect()
}

#[test]
fn string_literals_concatenation_length_code_chr_and_comparison() {
    for (source, expected) in [
        (b"\"hello\"".as_slice(), b"hello".as_slice()),
        (b"\"\"", b""),
        (b"\"a\"\"b\"", b"a\"b"),
        (b"\"ab\"+\"cd\"", b"abcd"),
        (&[194, b'6', b'5'], b"A"),
        (&[b'"', b'a', b'"', 198, b'1'], b"a"),
        (&[b'"', b'a', b'"', 198, b'0'], b""),
    ] {
        assert_eq!(string(&evaluate(source)), expected, "{source:?}");
    }
    for (source, expected) in [
        (&[177, b'"', b'a', b'b', b'"'][..], 2.0),
        (&[175, b'"', b'A', b'"'], 65.0),
        (&[175, b'"', b'"'], 0.0),
        (b"\"ab\"<\"abc\"", 1.0),
        (b"\"abc\"=\"abc\"", 1.0),
        (b"\"abc\">\"abd\"", 0.0),
    ] {
        assert!(
            (number(&evaluate(source)) - expected).abs() < 1e-8,
            "{source:?}"
        );
    }
    for source in [b"\"unterminated".as_slice(), b"1+\"a\"", b"\"a\"-\"b\""] {
        let machine = evaluate(source);
        assert_eq!(machine.cpu.registers.pc, 0x8101, "{source:?}");
        assert_eq!(machine.board.peek(0x5c3a), 11);
    }
}

#[test]
fn native_decimal_output_covers_zero_sign_fraction_and_scientific_ranges() {
    for (text, expected) in [
        ("0", "0"),
        ("1", "1"),
        ("-12", "-12"),
        ("42.125", "42.125"),
        ("0.125", "0.125"),
        ("0.00125", ".00125"),
        ("123456789", "123456790"),
        ("1E20", "1E+20"),
        ("1E-20", "1E-20"),
    ] {
        let source = [vec![193], text.as_bytes().to_vec()].concat();
        let machine = evaluate(&source);
        assert_eq!(string(&machine), expected.as_bytes(), "{text}");
    }
}

#[test]
fn elementary_functions_execute_in_guest_code_with_independent_numeric_oracles() {
    for (token, function) in [
        (178_u8, f64::sin as fn(f64) -> f64),
        (179, f64::cos),
        (180, f64::tan),
        (183, f64::atan),
        (184, f64::ln),
        (185, f64::exp),
        (187, f64::sqrt),
    ] {
        for value in [-2.0_f64, -0.5, 0.0, 0.1, 0.5, 1.0, 2.0, 5.0] {
            if (token == 184 && value <= 0.0) || (token == 187 && value < 0.0) {
                continue;
            }
            let source = [vec![token], format!("({value})").into_bytes()].concat();
            let actual = number(&evaluate(&source));
            let expected = function(value);
            assert!(
                (actual - expected).abs() < 2e-7 * expected.abs().max(1.0),
                "function {token}({value}): {actual} vs {expected}"
            );
        }
    }
    for value in [-1.0_f64, -0.75, -0.1, 0.0, 0.1, 0.75, 1.0] {
        for (token, expected) in [(181, value.asin()), (182, value.acos())] {
            let source = [vec![token], format!("({value})").into_bytes()].concat();
            let actual = number(&evaluate(&source));
            assert!(
                (actual - expected).abs() < 2e-7,
                "function {token}({value}): {actual} vs {expected}"
            );
        }
    }
    for (text, expected) in [
        ("2^3", 8.0),
        ("9^.5", 3.0),
        ("2^-3", 0.125),
        ("-2^2", -4.0),
        ("0^0", 1.0),
        ("0^3", 0.0),
    ] {
        assert!(
            (number(&evaluate(text.as_bytes())) - expected).abs() < 1e-7,
            "{text}"
        );
    }
    for (source, error) in [
        (&[184, b'0'][..], 9),
        (&[187, b'(', b'-', b'1', b')'], 9),
        (&[181, b'2'], 9),
        (b"(-2)^2", 10),
    ] {
        let machine = evaluate(source);
        assert_eq!(machine.cpu.registers.pc, 0x8101);
        assert_eq!(machine.board.peek(0x5c3a), error);
    }
}

#[test]
fn val_val_string_user_graphic_and_empty_inkey_keep_the_outer_expression() {
    for (source, expected) in [
        (
            &[b'1', b'+', 176, b'"', b'2', b'+', b'3', b'*', b'4', b'"'][..],
            15.0,
        ),
        (&[176, b'"', 178, b'1', b'"'], 1_f64.sin()),
        (&[192, b'"', b'a', b'"'], 65368.0),
        (&[192, b'"', b'U', b'"'], 65528.0),
        (&[177, 166], 0.0),
    ] {
        assert!(
            (number(&evaluate(source)) - expected).abs() < 2e-7,
            "{source:?}"
        );
    }
    let value = [vec![174], b"\"\"\"ab\"\"\"".to_vec()].concat();
    assert_eq!(string(&evaluate(&value)), b"ab");
    for source in [
        [vec![176], b"\"\"\"a\"\"\"".to_vec()].concat(),
        [vec![174], b"\"1+2\"".to_vec()].concat(),
        [vec![176], b"\"1:2\"".to_vec()].concat(),
    ] {
        let machine = evaluate(&source);
        assert_eq!(machine.cpu.registers.pc, 0x8101);
        assert_eq!(machine.board.peek(0x5c3a), 11);
    }
}

#[test]
fn string_slices_follow_native_defaults_bounds_empty_ranges_and_precedence() {
    for (source, expected) in [
        (b"\"hello\"(2)".to_vec(), b"e".as_slice()),
        (
            [b"\"hello\"(2 ".to_vec(), vec![204], b" 4)".to_vec()].concat(),
            b"ell",
        ),
        (
            [b"\"hello\"(".to_vec(), vec![204], b" 2)".to_vec()].concat(),
            b"he",
        ),
        (
            [b"\"hello\"(3 ".to_vec(), vec![204], b")".to_vec()].concat(),
            b"llo",
        ),
        (
            [b"\"hello\"(9 ".to_vec(), vec![204], b" 0)".to_vec()].concat(),
            b"",
        ),
        (b"\"hello\"()".to_vec(), b"hello"),
        (b"(\"ab\"+\"cd\")(3)".to_vec(), b"c"),
    ] {
        assert_eq!(string(&evaluate(&source)), expected, "{source:?}");
    }
    assert!(
        (number(&evaluate(&[vec![177], b"\"hello\"(2)".to_vec()].concat())) - 1.0).abs() < 1e-8
    );
    for source in [b"\"a\"(0)".as_slice(), b"\"a\"(2)"] {
        let machine = evaluate(source);
        assert_eq!(machine.cpu.registers.pc, 0x8101);
        assert_eq!(machine.board.peek(0x5c3a), 2);
    }
}
