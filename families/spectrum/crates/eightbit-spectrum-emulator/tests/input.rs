//! Interactive INPUT uses physical matrix events and frame IRQs, never host values.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_spectrum_emulator::{FRAME_T_STATES, Key, MAX_SLICE, Machine};
use eightbit_z80_core::Bus;

fn word(machine: &Machine, address: u16) -> u16 {
    u16::from_le_bytes([machine.board.peek(address), machine.board.peek(address + 1)])
}
fn set_word(machine: &mut Machine, address: u16, value: u16) {
    let [lo, hi] = value.to_le_bytes();
    machine.board.write(address, lo);
    machine.board.write(address + 1, hi);
}
fn prepared(source: &[u8]) -> Machine {
    let mut machine = Machine::development().unwrap();
    for _ in 0..8 {
        machine.run(MAX_SLICE).unwrap();
        machine.board.drain_audio();
    }
    let line = word(&machine, 0x5c59);
    for (offset, byte) in source.iter().copied().chain([13]).enumerate() {
        machine
            .board
            .write(line + u16::try_from(offset).unwrap(), byte);
    }
    for (offset, byte) in [0xcd, 0x40, 0x1e, 0xcd, 0x00, 0x1e, 0xf3, 0x76]
        .into_iter()
        .enumerate()
    {
        machine
            .board
            .write(0x8000 + u16::try_from(offset).unwrap(), byte);
    }
    set_word(&mut machine, 0x5c3d, 0xff00);
    set_word(&mut machine, 0xff00, 0x8100);
    machine.board.write(0x8100, 0xf3);
    machine.board.write(0x8101, 0x76);
    machine.cpu.registers.pc = 0x8000;
    machine.cpu.registers.sp = 0xff00;
    machine.cpu.registers.iff1 = true;
    machine.cpu.registers.iff2 = true;
    machine.cpu.halted = false;
    machine
}
fn start(source: &[u8]) -> Machine {
    let mut machine = prepared(source);
    settle(&mut machine);
    machine
}
fn settle(machine: &mut Machine) {
    for _ in 0..120 {
        machine.run(MAX_SLICE).unwrap();
        machine.board.drain_audio();
        if machine.cpu.halted {
            return;
        }
    }
    panic!(
        "did not settle at {:04x} page {}",
        machine.cpu.registers.pc,
        machine.board.firmware_page()
    );
}
fn chord(machine: &mut Machine, keys: &[&str]) {
    for key in keys {
        machine.board.key(Key::named(key).unwrap(), true);
    }
    for _ in 0..2 {
        machine.run(FRAME_T_STATES).unwrap();
        machine.board.drain_audio();
    }
    machine.board.release_keys();
    for _ in 0..2 {
        machine.run(FRAME_T_STATES).unwrap();
        machine.board.drain_audio();
    }
}
fn key(machine: &mut Machine, key: &str) {
    chord(machine, &[key]);
}
fn waiting(machine: &Machine) {
    assert!(machine.cpu.halted);
    assert!(
        machine.cpu.registers.pc < 0x2000,
        "input ended with error {}",
        machine.board.peek(0x5c3a)
    );
    assert_eq!(
        machine.board.firmware_page(),
        0,
        "error {} pc {:04x} cursor {:04x}",
        machine.board.peek(0x5c3a),
        machine.cpu.registers.pc,
        word(machine, 0x5c5d)
    );
}
#[test]
fn input_edits_numeric_expressions_quoted_strings_and_raw_line_text() {
    let mut machine = start(b"INPUT \"N?\";A:INPUT A$:INPUT LINE B$:POKE 49152,A:POKE 49153,LEN A$:POKE 49154,CODE B$:POKE 49155,LEN B$");
    waiting(&machine);
    key(&mut machine, "2");
    chord(&mut machine, &["caps-shift", "5"]);
    key(&mut machine, "1");
    chord(&mut machine, &["caps-shift", "8"]);
    chord(&mut machine, &["symbol-shift", "k"]);
    key(&mut machine, "3");
    key(&mut machine, "enter");
    settle(&mut machine);
    waiting(&machine);
    for letter in ["h", "i"] {
        key(&mut machine, letter);
    }
    key(&mut machine, "enter");
    settle(&mut machine);
    waiting(&machine);
    chord(&mut machine, &["symbol-shift", "p"]);
    key(&mut machine, "x");
    key(&mut machine, "enter");
    settle(&mut machine);
    assert_eq!(
        machine.cpu.registers.pc,
        0x8008,
        "error {}",
        machine.board.peek(0x5c3a)
    );
    assert_eq!(machine.board.peek(0xc000), 15);
    assert_eq!(machine.board.peek(0xc001), 2);
    assert_eq!(machine.board.peek(0xc002), 34);
    assert_eq!(machine.board.peek(0xc003), 2);
    assert_eq!(word(&machine, 0x5c65), word(&machine, 0x5c61));
    assert_eq!(machine.board.firmware_page(), 0);
}
#[test]
fn extended_keywords_inside_input_strings_remain_single_native_bytes() {
    let mut machine = start(b"INPUT A$:POKE 49152,LEN A$:POKE 49153,CODE A$");
    chord(&mut machine, &["caps-shift", "symbol-shift"]);
    key(&mut machine, "b");
    key(&mut machine, "enter");
    settle(&mut machine);
    assert_eq!(machine.cpu.registers.pc, 0x8008);
    assert_eq!(machine.board.peek(0xc000), 1);
    assert_eq!(machine.board.peek(0xc001), 196);
}
#[test]
fn malformed_numeric_input_can_be_corrected_and_break_uses_native_input_report() {
    let mut machine = start(b"INPUT A:POKE 49152,A");
    waiting(&machine);
    chord(&mut machine, &["symbol-shift", "k"]);
    key(&mut machine, "enter");
    settle(&mut machine);
    waiting(&machine);
    chord(&mut machine, &["caps-shift", "0"]);
    key(&mut machine, "7");
    key(&mut machine, "enter");
    settle(&mut machine);
    assert_eq!(machine.cpu.registers.pc, 0x8008);
    assert_eq!(machine.board.peek(0xc000), 7);
    let mut machine = start(b"INPUT A");
    chord(&mut machine, &["caps-shift", "space"]);
    settle(&mut machine);
    assert_eq!(machine.cpu.registers.pc, 0x8102);
    assert_eq!(machine.board.peek(0x5c3a), 16);
    assert_eq!(machine.board.firmware_page(), 0);
}

#[test]
fn multiple_array_targets_share_the_input_channel_and_output_only_streams_fail() {
    let mut machine=start(b"DIM A(2):DIM B$(2,5):INPUT A(2),B$(1):POKE 49152,A(2):POKE 49153,CODE B$(1):POKE 49154,LEN B$(1)");
    waiting(&machine);
    key(&mut machine, "9");
    key(&mut machine, "enter");
    settle(&mut machine);
    waiting(&machine);
    key(&mut machine, "z");
    key(&mut machine, "enter");
    settle(&mut machine);
    assert_eq!(
        machine.cpu.registers.pc,
        0x8008,
        "error {}",
        machine.board.peek(0x5c3a)
    );
    assert_eq!(machine.board.peek(0xc000), 9);
    assert_eq!(machine.board.peek(0xc001), b'z');
    assert_eq!(machine.board.peek(0xc002), 5);
    let machine = start(b"INPUT #2;A");
    assert_eq!(machine.cpu.registers.pc, 0x8102);
    assert_eq!(machine.board.peek(0x5c3a), 18);
}

#[test]
fn input_print_items_tokenised_functions_stop_and_raw_line_interruption_use_guest_rules() {
    let mut machine = start(b"INPUT AT 0,0;INK 2;(\"value\");TAB 10;2+2;A:POKE 49152,INT (A*10)");
    waiting(&machine);
    key(&mut machine, "p");
    key(&mut machine, "i");
    key(&mut machine, "enter");
    settle(&mut machine);
    assert_eq!(
        machine.cpu.registers.pc,
        0x8008,
        "error {}",
        machine.board.peek(0x5c3a)
    );
    assert_eq!(machine.board.peek(0xc000), 31);
    let mut machine = start(b"INPUT A");
    for letter in ["s", "t", "o", "p", "enter"] {
        key(&mut machine, letter);
    }
    settle(&mut machine);
    assert_eq!(machine.cpu.registers.pc, 0x8102);
    assert_eq!(machine.board.peek(0x5c3a), 16);
    let mut machine = start(b"INPUT LINE A$:POKE 49152,LEN A$");
    for letter in ["s", "t", "o", "p", "enter"] {
        key(&mut machine, letter);
    }
    settle(&mut machine);
    assert_eq!(machine.cpu.registers.pc, 0x8008);
    assert_eq!(machine.board.peek(0xc000), 4);
    let mut machine = start(b"INPUT LINE A$");
    chord(&mut machine, &["caps-shift", "6"]);
    settle(&mut machine);
    assert_eq!(machine.cpu.registers.pc, 0x8102);
    assert_eq!(machine.board.peek(0x5c3a), 16);
}

#[test]
fn long_input_expands_the_lower_window_without_losing_the_first_characters() {
    let mut machine = start(b"INPUT LINE A$:POKE 49152,LEN A$");
    let column = 33 - u16::from(machine.board.peek(0x5c8a));
    for _ in 0..90 {
        key(&mut machine, "x");
    }
    assert_eq!(machine.board.peek(0x5c6b), 3);
    for line in 0..8_u16 {
        assert_eq!(
            machine.board.peek(0x50a0 + column + line * 256),
            machine.board.peek(0x3c00 + u16::from(b'x') * 8 + line)
        );
    }
    key(&mut machine, "enter");
    settle(&mut machine);
    assert_eq!(machine.cpu.registers.pc, 0x8008);
    assert_eq!(machine.board.peek(0xc000), 90);
}

#[test]
fn scrolling_waits_for_a_fresh_answer_and_can_continue_or_report_break() {
    for (answer, expected) in [("y", 42), ("n", 0), ("space", 0)] {
        let mut machine = start(b"FOR A=1 TO 30:PRINT A:NEXT A:POKE 49152,42");
        waiting(&machine);
        assert_eq!(machine.board.peek(0x5c8c), 0);
        assert_eq!(machine.board.peek(0xc000), 0);
        key(&mut machine, answer);
        settle(&mut machine);
        assert_eq!(machine.board.peek(0xc000), expected);
        if expected == 0 {
            assert_eq!(machine.cpu.registers.pc, 0x8102);
            assert_eq!(machine.board.peek(0x5c3a), 12);
        } else {
            assert_eq!(machine.cpu.registers.pc, 0x8008);
        }
    }
}

#[test]
fn pause_counts_actual_frame_interrupts_and_indefinite_pause_wakes_on_a_fresh_key() {
    let mut machine = prepared(b"PAUSE 3:POKE 49152,1");
    for _ in 0..50000 {
        machine.step().unwrap();
        if machine.cpu.halted {
            break;
        }
    }
    assert!(machine.cpu.halted);
    assert_eq!(machine.board.peek(0xc000), 0);
    machine.run(FRAME_T_STATES * 2).unwrap();
    machine.board.drain_audio();
    assert_eq!(machine.board.peek(0xc000), 0);
    let before = word(&machine, 0x5c78);
    for _ in 0..50000 {
        machine.step().unwrap();
        if machine.board.peek(0x5c47) == 2 {
            break;
        }
    }
    assert_eq!(machine.board.peek(0x5c47), 2);
    assert_eq!(word(&machine, 0x5c78).wrapping_sub(before), 1);
    settle(&mut machine);
    assert_eq!(machine.cpu.registers.pc, 0x8008);
    assert_eq!(machine.board.peek(0xc000), 1);
    let mut machine = start(b"PAUSE 0:POKE 49152,7");
    for _ in 0..5 {
        machine.run(MAX_SLICE).unwrap();
        machine.board.drain_audio();
    }
    assert_eq!(machine.board.peek(0xc000), 0);
    assert!(machine.cpu.halted);
    key(&mut machine, "a");
    settle(&mut machine);
    assert_eq!(machine.cpu.registers.pc, 0x8008);
    assert_eq!(machine.board.peek(0xc000), 7);
}
