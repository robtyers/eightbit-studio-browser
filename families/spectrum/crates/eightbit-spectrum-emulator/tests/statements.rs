//! Guest editor-to-statement journeys, including native programme line records.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_spectrum_emulator::{Key, MAX_SLICE, Machine};
use eightbit_z80_core::Bus;

fn word(machine: &Machine, address: u16) -> u16 {
    u16::from_le_bytes([machine.board.peek(address), machine.board.peek(address + 1)])
}
fn set_word(machine: &mut Machine, address: u16, value: u16) {
    let [lo, hi] = value.to_le_bytes();
    machine.board.write(address, lo);
    machine.board.write(address + 1, hi);
}
fn boot() -> Machine {
    let mut machine = Machine::development().unwrap();
    for _ in 0..8 {
        machine.run(MAX_SLICE).unwrap();
        machine.board.drain_audio();
    }
    machine
}
fn execute(machine: &mut Machine, source: &[u8]) {
    execute_report(machine, source, None);
}
fn execute_report(machine: &mut Machine, source: &[u8], report: Option<u8>) {
    let line = word(machine, 0x5c59);
    for (offset, byte) in source.iter().copied().chain([13]).enumerate() {
        machine
            .board
            .write(line + u16::try_from(offset).unwrap(), byte);
    }
    for (offset, byte) in [0xcd, 0x40, 0x1e, 0xcd, 0x00, 0x1e, 0x76]
        .into_iter()
        .enumerate()
    {
        machine
            .board
            .write(0x8000 + u16::try_from(offset).unwrap(), byte);
    }
    set_word(machine, 0x5c3d, 0xff00);
    set_word(machine, 0xff00, 0x8100);
    machine.board.write(0x8100, 0x76);
    machine.cpu.registers.pc = 0x8000;
    machine.cpu.registers.sp = 0xff00;
    machine.cpu.registers.iff1 = false;
    machine.cpu.registers.iff2 = false;
    machine.cpu.halted = false;
    for _ in 0..100 {
        machine.run(MAX_SLICE).unwrap();
        machine.board.drain_audio();
        if machine.cpu.halted {
            break;
        }
    }
    assert!(
        machine.cpu.halted,
        "statement did not finish at {:04x}",
        machine.cpu.registers.pc
    );
    if let Some(report) = report {
        assert_eq!(machine.cpu.registers.pc, 0x8101, "{source:?}");
        assert_eq!(machine.board.peek(0x5c3a), report, "{source:?}");
    } else {
        assert_eq!(
            machine.cpu.registers.pc,
            0x8007,
            "{source:?}: error {} at line {}, cursor {:04x}, next {:?}",
            machine.board.peek(0x5c3a),
            word(machine, 0x5c45),
            word(machine, 0x5c5d),
            (0..12)
                .map(|i| machine.board.peek(word(machine, 0x5c5d).wrapping_add(i)))
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(machine.board.firmware_page(), 0);
}

#[test]
fn assignment_arithmetic_strings_machine_memory_and_conditional_statements_execute_together() {
    let mut machine = boot();
    execute(
        &mut machine,
        b"LET A=2+3*4:LET B=A/2:POKE 49152,B:IF A=14 THEN POKE 49153,99",
    );
    assert_eq!(machine.board.peek(0xc000), 7);
    assert_eq!(machine.board.peek(0xc001), 99);
    execute(
        &mut machine,
        b"LET A$=\"hello\":LET A$=A$+\" world\":POKE 49154,LEN A$:IF 0 THEN POKE 49153,0",
    );
    assert_eq!(machine.board.peek(0xc002), 11);
    assert_eq!(machine.board.peek(0xc001), 99);
    assert_eq!(word(&machine, 0x5c65), word(&machine, 0x5c61));
}

#[test]
fn inkey_requires_one_ordinary_key_and_stream_input_remains_nonblocking() {
    let mut machine = boot();
    for (keys, length, code) in [
        (vec![], 0, 0),
        (vec!["a"], 1, 97),
        (vec!["a", "caps-shift"], 1, 65),
        (vec!["a", "b"], 0, 0),
        (vec!["caps-shift", "symbol-shift"], 0, 0),
    ] {
        machine.board.release_keys();
        for key in keys {
            machine.board.key(Key::named(key).unwrap(), true);
        }
        execute(
            &mut machine,
            b"LET A$=INKEY$:POKE 49152,LEN A$:POKE 49153,CODE A$",
        );
        assert_eq!(machine.board.peek(0xc000), length);
        assert_eq!(machine.board.peek(0xc001), code);
        execute(
            &mut machine,
            b"LET A$=INKEY$#0:POKE 49152,LEN A$:POKE 49153,CODE A$",
        );
        assert_eq!(machine.board.peek(0xc000), length);
        assert_eq!(machine.board.peek(0xc001), code);
    }
    machine.board.release_keys();
    execute_report(&mut machine, b"LET A$=INKEY$#2", Some(18));
    execute_report(&mut machine, b"LET A$=INKEY$#4", Some(23));
    execute(&mut machine, b"LET A$=INKEY$:POKE 49152,LEN A$");
    assert_eq!(machine.board.peek(0xc000), 0);
}

#[test]
fn plot_uses_absolute_coordinates_as_the_published_language_requires() {
    let mut machine = boot();
    execute(&mut machine, b"PLOT -20,-30:POKE 49152,POINT(20,30)");
    assert_eq!(machine.board.peek(0xc000), 1);
    assert_eq!(machine.board.peek(0x5c7d), 20);
    assert_eq!(machine.board.peek(0x5c7e), 30);
}

#[test]
fn print_combines_at_strings_numeric_formatting_and_preserves_following_statements() {
    let mut machine = boot();
    execute(
        &mut machine,
        b"CLS:PRINT AT 3,4;\"A\";42.125;:POKE 49152,123",
    );
    assert_eq!(machine.board.peek(0xc000), 123);
    assert_eq!(machine.board.peek(0x5c88), 22);
    assert_eq!(machine.board.peek(0x5c89), 21);
    for y in 0..8_u16 {
        assert_eq!(
            machine.board.peek(0x4064 + y * 256),
            machine.board.peek(0x3c00 + u16::from(b'A') * 8 + y)
        );
    }
}

#[test]
fn run_and_goto_follow_native_line_records_and_clear_old_variables() {
    let mut machine = boot();
    let mut programme = Vec::new();
    for (number, body) in [
        (10_u16, vec![241, b' ', b'A', b'=', b'0']),
        (20, vec![241, b' ', b'A', b'=', b'A', b'+', b'1']),
        (
            30,
            vec![
                250, b' ', b'A', b'<', b'5', b' ', 203, b' ', 236, b' ', b'2', b'0',
            ],
        ),
        (
            40,
            vec![244, b' ', b'4', b'9', b'1', b'5', b'2', b',', b'A'],
        ),
    ] {
        programme.extend(number.to_be_bytes());
        programme.extend(u16::try_from(body.len() + 1).unwrap().to_le_bytes());
        programme.extend(body);
        programme.push(13);
    }
    for (offset, byte) in programme.iter().copied().enumerate() {
        machine
            .board
            .write(0x5ccb + u16::try_from(offset).unwrap(), byte);
    }
    let variables = 0x5ccb + u16::try_from(programme.len()).unwrap();
    machine.board.write(variables, 0x80);
    set_word(&mut machine, 0x5c4b, variables);
    set_word(&mut machine, 0x5c59, variables + 1);
    execute(&mut machine, b"RUN");
    assert_eq!(machine.board.peek(0xc000), 5);
}

#[test]
fn editor_inserts_replaces_deletes_and_runs_sorted_native_programme_lines() {
    let mut machine = boot();
    execute(&mut machine, b"30 POKE 49152,A");
    execute(&mut machine, b"10 LET A=1");
    execute(&mut machine, b"20 LET A=A+7");
    execute(&mut machine, b"10 LET A=2");
    execute(&mut machine, b"25 POKE 49152,99");
    execute(&mut machine, b"25");
    let mut address = word(&machine, 0x5c53);
    for expected in [10_u16, 20, 30] {
        assert_eq!(
            u16::from_be_bytes([machine.board.peek(address), machine.board.peek(address + 1)]),
            expected
        );
        address += 4 + word(&machine, address + 2);
    }
    assert_eq!(address, word(&machine, 0x5c4b));
    assert_eq!(machine.board.peek(address), 0x80);
    execute(&mut machine, b"RUN");
    assert_eq!(machine.board.peek(0xc000), 9);
}

#[test]
fn nested_subroutines_return_to_the_exact_following_statement() {
    let mut machine = boot();
    for line in [
        "10 LET A=1:GO SUB 100:POKE 49152,A:GO TO 9999",
        "100 LET A=A+2:GO SUB 200:LET A=A+4:RETURN",
        "200 LET A=A+8:RETURN",
    ] {
        execute(&mut machine, line.as_bytes());
    }
    execute(&mut machine, b"RUN");
    assert_eq!(machine.board.peek(0xc000), 15);
}

#[test]
fn for_records_cover_positive_negative_nested_and_zero_iteration_loops() {
    for (programme, expected) in [
        (
            vec![
                "10 LET S=0:FOR I=1 TO 5",
                "20 LET S=S+I:NEXT I",
                "30 POKE 49152,S",
            ],
            15,
        ),
        (
            vec![
                "10 LET S=0:FOR I=5 TO 1 STEP -2",
                "20 LET S=S+I:NEXT I",
                "30 POKE 49152,S",
            ],
            9,
        ),
        (
            vec![
                "10 LET S=0:FOR I=1 TO 3:FOR J=1 TO 2",
                "20 LET S=S+1:NEXT J:NEXT I",
                "30 POKE 49152,S",
            ],
            6,
        ),
        (
            vec![
                "10 LET S=7:FOR I=3 TO 1",
                "20 FOR J=1 TO 2:LET S=99:NEXT J:NEXT I",
                "30 POKE 49152,S",
            ],
            7,
        ),
        (
            vec![
                "10 LET S=0:FOR I=1 TO 1 STEP .25:LET S=S+1:NEXT I",
                "20 POKE 49152,S",
            ],
            1,
        ),
    ] {
        let mut machine = boot();
        for line in programme {
            execute(&mut machine, line.as_bytes());
        }
        execute(&mut machine, b"RUN");
        assert_eq!(machine.board.peek(0xc000), expected);
        let mut address = word(&machine, 0x5c4b);
        let end = word(&machine, 0x5c59);
        let mut controls = 0;
        while address < end && machine.board.peek(address) != 0x80 {
            let header = machine.board.peek(address);
            if header & 0xe0 == 0xe0 {
                controls += 1;
                address += 19;
            } else {
                assert_eq!(header & 0xe0, 0x60);
                address += 6;
            }
        }
        assert!(controls >= 1);
        assert_eq!(machine.board.peek(address), 0x80);
    }
}

#[test]
fn plot_point_attributes_and_screen_text_share_the_real_video_memory() {
    let mut machine = boot();
    execute(&mut machine,b"CLS:PLOT 0,0:PLOT 255,175:POKE 49152,POINT (0,0):POKE 49153,POINT (1,0):POKE 49154,ATTR (21,0)");
    assert_eq!(machine.board.peek(0xc000), 1);
    assert_eq!(machine.board.peek(0xc001), 0);
    assert_eq!(machine.board.peek(0xc002), 56);
    assert_eq!(machine.board.peek(0x57a0), 0x80);
    assert_eq!(machine.board.peek(0x401f), 1);
    assert_eq!(machine.board.peek(0x5c7d), 255);
    assert_eq!(machine.board.peek(0x5c7e), 175);
    execute(
        &mut machine,
        b"CLS:PRINT AT 3,4;\"A\";:LET A$=SCREEN$ (3,4):POKE 49152,CODE A$",
    );
    assert_eq!(machine.board.peek(0xc000), b'A');
}

#[test]
fn permanent_temporary_transparent_contrast_inverse_and_over_colours_affect_pixels() {
    let mut machine = boot();
    execute(&mut machine, b"PAPER 1:INK 6:BRIGHT 1:FLASH 1:CLS:PLOT 0,0");
    assert_eq!(machine.board.peek(0x5aa0), 0xce);
    machine.board.write(0x5aa0, 0x13);
    execute(&mut machine, b"INK 8:PAPER 8:BRIGHT 8:FLASH 8:PLOT 0,0");
    assert_eq!(machine.board.peek(0x5aa0), 0x13);
    execute(&mut machine, b"PLOT INK 9;0,0");
    assert_eq!(machine.board.peek(0x5aa0), 0x17);
    execute(
        &mut machine,
        b"INK 0:PAPER 7:BRIGHT 0:FLASH 0:PRINT INK 2;AT 3,4;\"A\";:PLOT 40,151",
    );
    assert_eq!(machine.board.peek(0x5864), 0x3a);
    assert_eq!(machine.board.peek(0x5865), 0xc8);
    execute(&mut machine, b"OVER 0:INVERSE 0:PLOT 0,0:OVER 1:PLOT 0,0");
    assert_eq!(machine.board.peek(0x57a0) & 0x80, 0);
    execute(&mut machine, b"OVER 0:PLOT 0,0:INVERSE 1:PLOT 0,0");
    assert_eq!(machine.board.peek(0x57a0) & 0x80, 0);
    machine.board.write(0x57a0, 0x80);
    execute(&mut machine, b"OVER 1:PLOT 0,0");
    assert_eq!(machine.board.peek(0x57a0) & 0x80, 0x80);
}

fn pixel(machine: &Machine, x: u16, y: u16) -> bool {
    let top = 175 - y;
    let address = 0x4000 + ((top & 0xc0) << 5) + ((top & 7) << 8) + ((top & 0x38) << 2) + (x >> 3);
    machine.board.peek(address) & (0x80 >> (x & 7)) != 0
}

#[test]
fn signed_draw_endpoints_and_negative_poke_bytes_execute_in_guest_code() {
    let mut machine = boot();
    execute(
        &mut machine,
        b"CLS:PLOT 10,10:DRAW 10,5:DRAW -10,5:DRAW 0,-10:POKE 49152,-1",
    );
    assert_eq!(machine.board.peek(0xc000), 255);
    for (x, y) in [(10, 10), (20, 15), (10, 20), (10, 19), (10, 18)] {
        assert!(pixel(&machine, x, y), "{x},{y}");
    }
    assert_eq!(machine.board.peek(0x5c7d), 10);
    assert_eq!(machine.board.peek(0x5c7e), 10);
    assert!(!pixel(&machine, 21, 15));
}

#[test]
fn substring_assignment_truncates_pads_and_preserves_overlapping_sources() {
    let mut machine = boot();
    for (command, expected) in [
        (
            "LET A$=\"abcdef\":LET A$(2 TO 5)=A$(1 TO 4)",
            b"aabcdf".as_slice(),
        ),
        ("LET A$=\"abcdef\":LET A$(2 TO 5)=\"z\"", b"az   f"),
        ("LET A$=\"abcdef\":LET A$ (2 TO 5)=\"123456\"", b"a1234f"),
        ("LET A$=\"abcdef\":LET A$()=\"q\"", b"q     "),
    ] {
        execute(&mut machine, command.as_bytes());
        let variables = word(&machine, 0x5c4b);
        assert_eq!(machine.board.peek(variables), 0x41);
        assert_eq!(word(&machine, variables + 1), 6);
        let bytes: Vec<_> = (0..6)
            .map(|i| machine.board.peek(variables + 3 + i))
            .collect();
        assert_eq!(bytes, expected, "{command}");
    }
}

#[test]
fn data_read_restore_and_repeated_run_preserve_programme_position_and_types() {
    let mut machine = boot();
    for line in [
        "10 DATA 7,\"ab\",2+3:POKE 49155,99",
        "20 READ A,A$,B",
        "30 POKE 49152,A:POKE 49153,LEN A$:POKE 49154,B",
        "40 RESTORE 10:READ C:POKE 49156,C",
        "50 GO TO 9999",
    ] {
        execute(&mut machine, line.as_bytes());
    }
    for _ in 0..2 {
        execute(&mut machine, b"RUN");
        for (address, value) in [
            (0xc000, 7),
            (0xc001, 2),
            (0xc002, 5),
            (0xc003, 99),
            (0xc004, 7),
        ] {
            assert_eq!(machine.board.peek(address), value, "{address:04x}");
        }
    }
    execute(&mut machine, b"RESTORE:READ N");
    execute(&mut machine, b"POKE 49152,N");
    assert_eq!(machine.board.peek(0xc000), 7);
}

#[test]
fn native_error_returns_cover_missing_data_return_loop_colour_and_recovery() {
    let mut machine = boot();
    for (command, error) in [
        ("READ A", 13),
        ("RETURN", 6),
        ("INK 10", 19),
        ("BORDER 8", 19),
        ("LET A=1/0", 5),
        ("POKE 49152,256", 10),
    ] {
        execute_report(&mut machine, command.as_bytes(), Some(error));
        execute(&mut machine, b"LET Good=42:POKE 49152,Good");
        assert_eq!(machine.board.peek(0xc000), 42);
    }
    execute(&mut machine, b"10 LET I=1:NEXT I");
    execute_report(&mut machine, b"RUN", Some(0));
    execute(&mut machine, b"10 FOR I=2 TO 1");
    execute_report(&mut machine, b"RUN", Some(17));
}

#[test]
fn numeric_arrays_use_native_dimensions_row_major_values_and_replacement_without_leaks() {
    let mut machine = boot();
    execute(&mut machine,b"DIM A(2,3):LET A(1,1)=7:LET A(2,3)=42:LET A(2,2)=A(1,1)+1:POKE 49152,A(2,3):POKE 49153,A(2,2):POKE 49154,A(1,2)");
    assert_eq!(machine.board.peek(0xc000), 42);
    assert_eq!(machine.board.peek(0xc001), 8);
    assert_eq!(machine.board.peek(0xc002), 0);
    let variables = word(&machine, 0x5c4b);
    assert_eq!(machine.board.peek(variables), 0x81);
    assert_eq!(word(&machine, variables + 1), 35);
    assert_eq!(machine.board.peek(variables + 3), 2);
    assert_eq!(word(&machine, variables + 4), 2);
    assert_eq!(word(&machine, variables + 6), 3);
    for size in [1_u16, 20, 3, 1] {
        execute(
            &mut machine,
            format!("DIM A({size}):LET A({size})=99:POKE 49152,A({size})").as_bytes(),
        );
        assert_eq!(machine.board.peek(0xc000), 99);
        assert_eq!(word(&machine, variables + 1), 3 + size * 5);
        assert_eq!(word(&machine, 0x5c59), variables + 7 + size * 5);
        assert_eq!(machine.board.peek(word(&machine, 0x5c59) - 1), 0x80);
    }
    for (command, report) in [
        ("LET A(0)=1", 2),
        ("LET A(2)=1", 2),
        ("LET A(1,1)=1", 2),
        ("DIM A(0)", 10),
        ("DIM A(65535,65535)", 3),
        ("LET A(1)=A(1)+B(1)", 1),
    ] {
        execute_report(&mut machine, command.as_bytes(), Some(report));
        execute(&mut machine, b"POKE 49152,A(1)");
        assert_eq!(machine.board.peek(0xc000), 99);
    }
}

#[test]
fn character_arrays_keep_fixed_width_cells_padding_slices_and_one_string_namespace() {
    let mut machine = boot();
    execute(&mut machine,b"DIM A$(2,5):LET A$(1)=\"xy\":LET A$(2)=A$(1):LET A$(2,3)=\"z\":LET A$(1)(2 TO 3)=\"AB\":POKE 49152,LEN A$(1):POKE 49153,CODE A$(2,3)");
    assert_eq!(machine.board.peek(0xc000), 5);
    assert_eq!(machine.board.peek(0xc001), b'z');
    let variables = word(&machine, 0x5c4b);
    assert_eq!(machine.board.peek(variables), 0xc1);
    assert_eq!(word(&machine, variables + 1), 15);
    let data: Vec<_> = (0..10)
        .map(|i| machine.board.peek(variables + 8 + i))
        .collect();
    assert_eq!(data, b"xAB  xyz  ");
    execute(&mut machine, b"LET A$=A$(1):POKE 49152,LEN A$");
    assert_eq!(machine.board.peek(variables), 0x41);
    assert_eq!(machine.board.peek(0xc000), 5);
    execute(
        &mut machine,
        b"DIM A$(4):LET A$()=\"Q\":POKE 49152,LEN A$:POKE 49153,CODE A$",
    );
    assert_eq!(machine.board.peek(variables), 0xc1);
    assert_eq!(machine.board.peek(0xc000), 4);
    assert_eq!(machine.board.peek(0xc001), b'Q');
    for command in ["LET A$(0)=\"a\"", "LET A$(5)=\"a\""] {
        execute_report(&mut machine, command.as_bytes(), Some(2));
    }
}

#[test]
fn defined_functions_bind_numeric_and_string_parameters_nest_and_restore_globals() {
    let mut machine = boot();
    for line in [
        "10 DEF FN f(x,y)=x*x+y",
        "20 DEF FN g$(x$,n)=x$+CHR$ n",
        "30 DEF FN h(z)=FN f(z,2)+FN f(z,3)",
        "40 DEF FN k()=42",
        "50 LET x=99:LET x$=\"global\":LET A$=FN g$(\"ab\",67)",
        "60 POKE 49152,FN f(3,4):POKE 49153,FN h(2):POKE 49154,FN k()",
        "70 POKE 49155,LEN A$:POKE 49156,CODE A$(3):POKE 49157,x:POKE 49158,LEN x$",
    ] {
        execute(&mut machine, line.as_bytes());
    }
    execute(&mut machine, b"RUN");
    for (address, value) in [
        (0xc000, 13),
        (0xc001, 13),
        (0xc002, 42),
        (0xc003, 3),
        (0xc004, b'C'),
        (0xc005, 99),
        (0xc006, 6),
    ] {
        assert_eq!(machine.board.peek(address), value, "{address:04x}");
    }
    assert_eq!(word(&machine, 0x5c0b), 0);
    for (command, error) in [
        ("LET A=FN q(1)", 24),
        ("LET A=FN f(1)", 25),
        ("LET A=FN f()", 25),
        ("LET A=FN f(\"a\",1)", 25),
    ] {
        execute_report(&mut machine, command.as_bytes(), Some(error));
        assert_eq!(word(&machine, 0x5c0b), 0);
        execute(&mut machine, b"POKE 49157,x");
        assert_eq!(machine.board.peek(0xc005), 99);
    }
}

#[test]
fn native_hidden_function_parameter_cells_are_accepted_without_becoming_globals() {
    let mut machine = boot();
    let mut programme = Vec::new();
    for (number, body) in [
        (
            10_u16,
            vec![
                206, 32, b'F', b'(', b'X', 14, 0, 0, 0, 0, 0, b')', b'=', b'X', b'+', b'1', 14,
                129, 0, 0, 0, 0,
            ],
        ),
        (
            20,
            vec![
                244, 32, b'4', b'9', b'1', b'5', b'2', b',', 168, 32, b'F', b'(', b'4', b'1', b')',
            ],
        ),
    ] {
        programme.extend(number.to_be_bytes());
        programme.extend(u16::try_from(body.len() + 1).unwrap().to_le_bytes());
        programme.extend(body);
        programme.push(13);
    }
    for (offset, byte) in programme.iter().copied().enumerate() {
        machine
            .board
            .write(0x5ccb + u16::try_from(offset).unwrap(), byte);
    }
    let variables = 0x5ccb + u16::try_from(programme.len()).unwrap();
    machine.board.write(variables, 0x80);
    set_word(&mut machine, 0x5c4b, variables);
    set_word(&mut machine, 0x5c59, variables + 1);
    execute(&mut machine, b"RUN");
    assert_eq!(machine.board.peek(0xc000), 42);
    assert_eq!(machine.board.peek(variables), 0x80);
    assert_eq!(word(&machine, 0x5c0b), 0);
}

#[test]
fn clear_rebases_live_gate_frames_below_reserved_code_and_rejects_bad_limits() {
    for top in [24575_u16, 32767, 65280, 65281, 65300, 65367] {
        let mut machine = boot();
        execute(
            &mut machine,
            format!("LET A=99:DIM B(10):CLEAR {top}:POKE 49152,42").as_bytes(),
        );
        assert_eq!(machine.board.peek(0xc000), 42, "RAMTOP {top}");
        assert_eq!(word(&machine, 0x5cb2), top);
        assert_eq!(word(&machine, 0x5c3d), top - 7);
        assert_eq!(machine.cpu.registers.sp, top - 7);
        assert_eq!(machine.cpu.registers.iy, 0x5c3a);
        assert_eq!(word(&machine, 0x5c4b), word(&machine, 0x5c53));
        assert_eq!(machine.board.peek(word(&machine, 0x5c4b)), 0x80);
        assert_eq!(word(&machine, 0x5c7d), 0);
    }
    let mut machine = boot();
    execute(&mut machine, b"LET A=99");
    for top in [0, 16384, 23755, 65535] {
        execute_report(&mut machine, format!("CLEAR {top}").as_bytes(), Some(21));
        execute(&mut machine, b"POKE 49152,A");
        assert_eq!(machine.board.peek(0xc000), 99);
    }
}

#[test]
fn circle_preserves_symmetry_draws_each_pixel_once_and_checks_extents_before_writing() {
    let mut machine = boot();
    execute(&mut machine, b"CLS:CIRCLE 128,88,20");
    for (x, y) in [(148, 88), (108, 88), (128, 108), (128, 68)] {
        assert!(pixel(&machine, x, y));
    }
    for y in 68..=108_u16 {
        for x in 108..=148_u16 {
            assert_eq!(pixel(&machine, x, y), pixel(&machine, 256 - x, y));
            assert_eq!(pixel(&machine, x, y), pixel(&machine, x, 176 - y));
        }
    }
    assert!(!pixel(&machine, 128, 88));
    assert_eq!(machine.board.peek(0x5c7d), 148);
    assert_eq!(machine.board.peek(0x5c7e), 88);
    execute(
        &mut machine,
        b"CLS:OVER 1:CIRCLE 128,88,20:CIRCLE 128,88,20",
    );
    assert!((0x4000..0x5800).all(|address| machine.board.peek(address) == 0));
    execute(&mut machine, b"OVER 0:CIRCLE 10,10,0");
    assert!(pixel(&machine, 10, 10));
    execute(&mut machine, b"CLS");
    execute_report(&mut machine, b"CIRCLE 1,1,5", Some(10));
    assert!((0x4000..0x5800).all(|address| machine.board.peek(address) == 0));
}

#[test]
fn streams_route_screen_lower_input_and_printer_without_moving_other_cursors() {
    let mut machine = boot();
    execute(&mut machine, b"CLS:PRINT #0;AT 0,0;\"K\";:PRINT AT 3,0;\"S\";:LPRINT \"C\";:OPEN #5,\"p\":PRINT #5;\"D\";:CLOSE #5");
    for line in 0..8_u16 {
        assert_eq!(
            machine.board.peek(0x50c0 + line * 256),
            machine.board.peek(0x3c00 + u16::from(b'K') * 8 + line)
        );
        assert_eq!(
            machine.board.peek(0x4060 + line * 256),
            machine.board.peek(0x3c00 + u16::from(b'S') * 8 + line)
        );
        for (column, character) in [(0, b'C'), (1, b'D')] {
            assert_eq!(
                machine.board.peek(0x5b00 + column + line * 32),
                machine.board.peek(0x3c00 + u16::from(character) * 8 + line)
            );
        }
    }
    assert_eq!(word(&machine, 0x5c88), 0x1520);
    assert_eq!(word(&machine, 0x5c8a), 0x0220);
    assert_eq!(machine.board.peek(0x5c7f), 31);
    assert_eq!(word(&machine, 0x5c20), 0);
    execute_report(&mut machine, b"PRINT #5;\"x\"", Some(23));
    execute_report(&mut machine, b"OPEN #5,\"R\"", Some(18));
    execute_report(&mut machine, b"OPEN #16,\"S\"", Some(23));
    execute(&mut machine, b"OPEN #5,\"S\":CLOSE #2:PRINT \"A\";");
    assert_eq!(word(&machine, 0x5c1a), 6);
}

#[test]
fn tab_apostrophe_and_printer_comma_use_channel_columns_and_terminate() {
    let mut machine = boot();
    execute(
        &mut machine,
        b"CLS:PRINT \"A\";TAB 5;\"B\",\"C\"'\"D\";:LPRINT \"A\",\"B\";",
    );
    for line in 0..8_u16 {
        for (address, character) in [
            (0x4000, b'A'),
            (0x4005, b'B'),
            (0x4010, b'C'),
            (0x4020, b'D'),
            (0x5b00, b'A'),
            (0x5b10, b'B'),
        ] {
            let stride = if address >= 0x5b00 { 32 } else { 256 };
            assert_eq!(
                machine.board.peek(address + line * stride),
                machine.board.peek(0x3c00 + u16::from(character) * 8 + line),
                "{address:04x} row {line}"
            );
        }
    }
    assert_eq!(machine.board.peek(0x5c7f), 16);
}

#[test]
fn continue_repeats_failed_expressions_advances_stop_and_detects_removed_lines() {
    let mut machine = boot();
    execute(&mut machine, b"20 LET A=1:STOP:LET A=A+4:POKE 49152,A");
    execute_report(&mut machine, b"RUN", Some(8));
    assert_eq!(word(&machine, 0x5c6e), 20);
    assert_eq!(machine.board.peek(0x5c70), 3);
    execute(&mut machine, b"LET A=2");
    execute(&mut machine, b"CONTINUE");
    assert_eq!(machine.board.peek(0xc000), 6);
    execute(&mut machine, b"10 LET C=1/B:POKE 49153,C*10:STOP");
    execute_report(&mut machine, b"RUN", Some(1));
    assert_eq!(word(&machine, 0x5c6e), 10);
    assert_eq!(machine.board.peek(0x5c70), 1);
    execute(&mut machine, b"LET B=2");
    execute_report(&mut machine, b"CONTINUE", Some(8));
    assert_eq!(machine.board.peek(0xc001), 5);
    execute(&mut machine, b"10");
    execute_report(&mut machine, b"CONTINUE", Some(22));
}

#[test]
fn direct_continue_uses_the_current_command_and_rejects_a_lost_statement() {
    let mut machine = boot();
    execute_report(&mut machine, b"STOP", Some(8));
    assert_eq!(word(&machine, 0x5c6e), 0xffff);
    assert_eq!(machine.board.peek(0x5c70), 2);
    execute(&mut machine, b"CONTINUE");
    execute_report(&mut machine, b":STOP", Some(8));
    assert_eq!(machine.board.peek(0x5c70), 3);
    execute_report(&mut machine, b"CONTINUE", Some(22));
    execute(&mut machine, b"10 STOP:POKE 49152,37");
    execute_report(&mut machine, b"RUN", Some(8));
    execute_report(&mut machine, b":POKE 1/0,2", Some(5));
    assert_eq!(word(&machine, 0x5c6e), 0xffff);
    assert_eq!(machine.board.peek(0x5c70), 2);
    execute(&mut machine, b"CONTINUE");
    assert_eq!(machine.board.peek(0xc000), 0);
    execute_report(&mut machine, b"RUN", Some(8));
    execute(&mut machine, b"10 REM changed");
    execute(&mut machine, b"CONTINUE");
    execute(&mut machine, b"10 STOP:STOP:STOP");
    execute_report(&mut machine, b"RUN", Some(8));
    execute_report(&mut machine, b"CONTINUE", Some(8));
    execute_report(&mut machine, b"CONTINUE", Some(8));
    execute(&mut machine, b"10 REM shorter");
    execute_report(&mut machine, b"CONTINUE", Some(22));
}

fn capture_channel(machine: &mut Machine) {
    let assembly = eightbit_z80_core::assembler::assemble(
        "push hl\nld hl,($bffe)\nld (hl),a\ninc hl\nld ($bffe),hl\npop hl\nret",
        0x9000,
        None,
    )
    .unwrap();
    for (offset, byte) in assembly.bytes.into_iter().enumerate() {
        machine
            .board
            .write(0x9000 + u16::try_from(offset).unwrap(), byte);
    }
    set_word(machine, 0x5cbb, 0x9000);
    set_word(machine, 0xbffe, 0xc100);
}
fn captured(machine: &Machine) -> Vec<u8> {
    (0xc100..word(machine, 0xbffe))
        .map(|address| machine.board.peek(address))
        .collect()
}
#[test]
fn listing_uses_public_channels_and_reports_include_public_locations() {
    let mut machine = boot();
    execute(&mut machine, b"10 LET A=2:PRINT A");
    execute(&mut machine, b"20 STOP");
    capture_channel(&mut machine);
    execute(&mut machine, b"LIST 10");
    assert_eq!(captured(&machine), b"10 \xf1 A=2:\xf5 A\r20 \xe2\r");
    assert_eq!(word(&machine, 0x5c49), 10);
    set_word(&mut machine, 0xbffe, 0xc100);
    execute_report(&mut machine, b"RUN", Some(8));
    // Invoke the original report formatter while keeping the test's error trap.
    for (offset, byte) in [0xaf, 0xcd, 0xc0, 0x16, 0x76].into_iter().enumerate() {
        machine
            .board
            .write(0x8000 + u16::try_from(offset).unwrap(), byte);
    }
    machine.cpu.registers.pc = 0x8000;
    machine.cpu.halted = false;
    machine.run(MAX_SLICE).unwrap();
    assert!(captured(&machine).ends_with(b"\r9 STOP statement,20:1\r"));
}

#[test]
fn editor_adds_native_numeric_and_function_cells_without_encoding_names_strings_or_comments() {
    let mut machine = boot();
    execute(&mut machine, b"10 DEF FN F(X,Y$)=X+LEN Y$");
    execute(
        &mut machine,
        b"20 LET ABC123=BIN 101+2.5E1:LET B$=\"42\":REM 777",
    );
    let first = word(&machine, 0x5c53);
    let body: Vec<_> = (first + 4..first + 4 + word(&machine, first + 2))
        .map(|p| machine.board.peek(p))
        .collect();
    assert_eq!(body.iter().filter(|byte| **byte == 14).count(), 2);
    assert!(
        body.windows(7)
            .any(|cell| cell == [b'X', 14, 0, 0, 0, 0, 0])
    );
    assert!(
        body.windows(8)
            .any(|cell| cell == [b'Y', b'$', 14, 0, 0, 0, 0, 0])
    );
    let second = first + 4 + word(&machine, first + 2);
    let body: Vec<_> = (second + 4..second + 4 + word(&machine, second + 2))
        .map(|p| machine.board.peek(p))
        .collect();
    assert_eq!(body.iter().filter(|byte| **byte == 14).count(), 2);
    assert!(
        body.windows(9)
            .any(|cell| cell == [b'1', b'0', b'1', 14, 131, 32, 0, 0, 0])
    );
    assert!(
        body.windows(11)
            .any(|cell| cell == [b'2', b'.', b'5', b'E', b'1', 14, 133, 72, 0, 0, 0])
    );
    assert!(body.ends_with(b"\"42\":\xea 777\r"));
    execute(&mut machine, b"RUN");
    execute(&mut machine, b"POKE 49152,ABC123");
    assert_eq!(machine.board.peek(0xc000), 30);
    // An existing payload ending in ASCII space must not be trimmed as spelling.
    let mut native = b"30 LET A=1".to_vec();
    native.extend([14, 129, 0, 0, 0, 32, b' ', b' ']);
    execute(&mut machine, &native);
    let third = second + 4 + word(&machine, second + 2);
    let body: Vec<_> = (third + 4..third + 4 + word(&machine, third + 2))
        .map(|p| machine.board.peek(p))
        .collect();
    assert!(body.ends_with(&[b'1', 14, 129, 0, 0, 0, 32, b' ', b' ', 13]));
}

#[test]
fn graphics_default_changes_ink_only_and_explicit_items_can_change_other_attributes() {
    let mut machine = boot();
    machine.board.write(0x5aa0, 0xd3);
    execute(&mut machine, b"INK 6:PAPER 7:BRIGHT 0:FLASH 0:PLOT 0,0");
    assert_eq!(machine.board.peek(0x5aa0), 0xd6);
    execute(&mut machine, b"PLOT PAPER 1;BRIGHT 0;FLASH 0;0,0");
    assert_eq!(machine.board.peek(0x5aa0), 0x0e);
}

#[test]
fn combined_character_array_slices_match_separate_slices_for_reads_and_assignment() {
    let mut machine = boot();
    execute(&mut machine,b"DIM A$(2,6):LET A$(1)=\"abcdef\":LET A$(2)=\"ghijkl\":LET A$(1,2 TO 4)=\"XY\":LET B$=A$(1,2 TO 5):LET C$=A$(2,TO 3):LET D$=A$(2,):POKE 49152,CODE B$:POKE 49153,LEN B$:POKE 49154,LEN C$:POKE 49155,LEN D$");
    for (offset, expected) in [b'X', 4, 3, 6].into_iter().enumerate() {
        assert_eq!(
            machine.board.peek(0xc000 + u16::try_from(offset).unwrap()),
            expected
        );
    }
    execute(&mut machine, b"IF A$(1,2 TO 4)<>A$(1)(2 TO 4) THEN STOP");
    execute(&mut machine, b"IF A$(2,3)<>\"i\" THEN STOP");
    execute(&mut machine, b"IF A$(2,5 TO 2)<>\"\" THEN STOP");
    execute(
        &mut machine,
        b"DIM Z$(5):LET Z$()=\"abcde\":IF Z$(2 TO 4)<>\"bcd\" THEN STOP",
    );
    execute_report(&mut machine, b"PRINT A$(1,0 TO 2)", Some(2));
}

#[test]
fn curved_draw_uses_guest_angles_correct_endpoints_and_reversible_over_pixels() {
    let mut machine = boot();
    execute(&mut machine, b"CLS:PLOT 40,40:DRAW 40,40,PI/2");
    assert_eq!(machine.board.peek(0x5c7d), 80);
    assert_eq!(machine.board.peek(0x5c7e), 80);
    assert!(pixel(&machine, 80, 80));
    assert!(!pixel(&machine, 60, 60));
    assert!((67..=69).any(|x| (51..=53).any(|y| pixel(&machine, x, y))));
    execute(&mut machine, b"CLS:PLOT 40,40:DRAW 40,40,-PI/2");
    assert!((51..=53).any(|x| (67..=69).any(|y| pixel(&machine, x, y))));
    assert!(!pixel(&machine, 60, 60));
    execute(
        &mut machine,
        b"CLS:PLOT 40,40:OVER 1:DRAW 40,40,PI/2:POKE 23677,40:POKE 23678,40:DRAW 40,40,PI/2",
    );
    for x in 40..=80 {
        for y in 40..=80 {
            assert_eq!(pixel(&machine, x, y), (x, y) == (40, 40), "{x},{y}");
        }
    }
    execute(&mut machine, b"OVER 0:PLOT 1,1:DRAW 2,3,0");
    assert_eq!(machine.board.peek(0x5c7d), 3);
    assert_eq!(machine.board.peek(0x5c7e), 4);
    execute_report(&mut machine, b"DRAW 256,0,PI", Some(10));
}

#[test]
fn read_assigns_numeric_array_cells_character_rows_and_slices_using_native_data_cursor() {
    let mut machine = boot();
    for line in [
        "10 DATA 7,\"hello\",\"XY\",9",
        "20 DIM A(2):DIM B$(2,6)",
        "30 READ A(1),B$(1),B$(1,2 TO 4),A(2)",
        "40 POKE 49152,A(1):POKE 49153,A(2):POKE 49154,CODE B$(1,2):POKE 49155,CODE B$(1,4)",
    ] {
        execute(&mut machine, line.as_bytes());
    }
    execute(&mut machine, b"RUN");
    for (offset, expected) in [7, 9, b'X', b' '].into_iter().enumerate() {
        assert_eq!(
            machine.board.peek(0xc000 + u16::try_from(offset).unwrap()),
            expected
        );
    }
    execute(&mut machine, b"RESTORE:READ A(2):POKE 49152,A(2)");
    assert_eq!(machine.board.peek(0xc000), 7);
}
