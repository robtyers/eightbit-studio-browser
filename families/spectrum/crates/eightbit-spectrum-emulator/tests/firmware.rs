//! Original guest firmware tests: all editing/input and output execute as Z80.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_spectrum_emulator::{DEVELOPMENT_MANIFEST, FRAME_T_STATES, Key, MAX_SLICE, Machine};
use eightbit_z80_core::{Bus, assembler::assemble};

fn boot() -> Machine {
    let mut machine = Machine::development().unwrap();
    for _ in 0..8 {
        machine.run(MAX_SLICE).unwrap();
        machine.board.drain_audio();
    }
    settle(&mut machine);
    machine
}
fn settle(machine: &mut Machine) {
    for _ in 0..100 {
        if machine.cpu.halted {
            return;
        }
        machine.run(20000).unwrap();
        machine.board.drain_audio();
    }
    panic!(
        "guest did not settle at {:04x} page {}",
        machine.cpu.registers.pc,
        machine.board.firmware_page()
    );
}
fn word(machine: &Machine, address: u16) -> u16 {
    u16::from_le_bytes([machine.board.peek(address), machine.board.peek(address + 1)])
}
fn frame(machine: &mut Machine) {
    machine.run(FRAME_T_STATES).unwrap();
    machine.board.drain_audio();
}

#[test]
fn original_boot_initialises_public_workspace_and_draws_a_readable_prompt() {
    let machine = boot();
    assert!(DEVELOPMENT_MANIFEST.contains("\"status\":\"development\""));
    assert_eq!(word(&machine, 0x5c36), 0x3c00);
    assert_eq!(word(&machine, 0x5c53), 0x5ccb);
    assert_eq!(word(&machine, 0x5c59), 0x5ccc);
    assert_eq!(word(&machine, 0x5c7b), 0xff58);
    assert_eq!(word(&machine, 0x5cb2), 0xff57);
    assert_eq!(word(&machine, 0x5cb4), 0xffff);
    assert_eq!(machine.board.peek(0x5c8d), 56);
    assert_eq!(machine.board.peek(0x5c88), 33);
    assert_eq!(machine.board.peek(0x5c8a), 31);
    assert_eq!(machine.board.peek(0x5c89), 23);
    assert!(machine.cpu.halted);
    assert_eq!(machine.cpu.registers.iy, 0x5c3a);
    assert!(machine.board.peek(0x5c78) > 0);
    assert!((0x4000..0x5800).any(|address| machine.board.peek(address) != 0));
    assert_eq!(
        machine.board.peek(0xff58 + 1),
        machine.board.peek(0x3e08 + 1)
    );
}

#[test]
fn literal_keyboard_chords_enter_text_and_newline_in_guest_editor() {
    let mut machine = boot();
    machine.board.set_literal_typing(true);
    for key in ["h", "e", "l", "l", "o"] {
        machine.board.key(Key::named(key).unwrap(), true);
        frame(&mut machine);
        frame(&mut machine);
        machine.board.key(Key::named(key).unwrap(), false);
        frame(&mut machine);
        frame(&mut machine);
    }
    machine.board.key(Key::named("caps-shift").unwrap(), true);
    machine.board.key(Key::named("a").unwrap(), true);
    frame(&mut machine);
    frame(&mut machine);
    machine.board.release_keys();
    frame(&mut machine);
    frame(&mut machine);
    let text: Vec<u8> = (0x5ccc..0x5cd2)
        .map(|address| machine.board.peek(address))
        .collect();
    assert_eq!(text, b"helloA");
    machine.board.key(Key::named("enter").unwrap(), true);
    frame(&mut machine);
    frame(&mut machine);
    machine.board.release_keys();
    frame(&mut machine);
    settle(&mut machine);
    // Syntax rejection retains the spelling for correction and does not turn
    // an editor error into a failed executed command/CONTINUE position.
    assert_eq!(edit_text(&machine), b"helloA");
    assert_eq!(machine.board.peek(0x5c3a), 255);
    cursor_is(&machine, b'?');
    assert!(machine.cpu.halted);
    for _ in 0..6 {
        chord(&mut machine, &["caps-shift", "0"]);
    }
    for key in ["p", "r", "i", "n", "t", "space", "7", "enter"] {
        chord(&mut machine, &[key]);
    }
    settle(&mut machine);
    assert_eq!(edit_text(&machine), b"");
    cursor_is(&machine, b'K');
    for scan in 0..8_u16 {
        assert_eq!(
            machine.board.peek(0x4020 + scan * 256),
            machine.board.peek(0x3c00 + u16::from(b'7') * 8 + scan)
        );
    }
}

fn chord(machine: &mut Machine, keys: &[&str]) {
    for key in keys {
        machine.board.key(Key::named(key).unwrap(), true);
    }
    frame(machine);
    frame(machine);
    machine.board.release_keys();
    frame(machine);
    frame(machine);
}
fn edit_text(machine: &Machine) -> Vec<u8> {
    let line = word(machine, 0x5c59);
    (0..4096)
        .map(|offset| machine.board.peek(line + offset))
        .take_while(|byte| *byte != 13)
        .collect()
}
fn cursor_is(machine: &Machine, character: u8) {
    let row = 24 - u16::from(machine.board.peek(0x5c8b));
    let column = 33 - u16::from(machine.board.peek(0x5c8a));
    for scan in 0..8_u16 {
        let address = 0x4000 | ((row & 24) << 8) | (scan << 8) | ((row & 7) << 5) | column;
        assert_eq!(
            machine.board.peek(address),
            !machine.board.peek(0x3c00 + u16::from(character) * 8 + scan)
        );
    }
    assert_eq!(machine.board.peek(0x5800 + row * 32 + column) & 128, 128);
}
#[test]
fn physical_keyword_extended_capitals_and_graphics_modes_use_guest_state() {
    let mut machine = boot();
    cursor_is(&machine, b'K');
    chord(&mut machine, &["p"]);
    assert_eq!(edit_text(&machine), &[245]);
    cursor_is(&machine, b'L');
    chord(&mut machine, &["caps-shift", "0"]);
    assert!(edit_text(&machine).is_empty());
    assert_eq!(machine.board.peek(0x5c8a), 31);
    cursor_is(&machine, b'K');
    chord(&mut machine, &["p"]);
    chord(&mut machine, &["symbol-shift", "p"]);
    chord(&mut machine, &["caps-shift", "2"]);
    cursor_is(&machine, b'C');
    chord(&mut machine, &["a"]);
    chord(&mut machine, &["caps-shift", "a"]);
    assert_eq!(edit_text(&machine), &[245, 34, 65, 97]);
    chord(&mut machine, &["caps-shift", "9"]);
    assert_eq!(machine.board.peek(0x5c41), 2);
    cursor_is(&machine, b'G');
    chord(&mut machine, &["8"]);
    chord(&mut machine, &["caps-shift", "8"]);
    chord(&mut machine, &["a"]);
    chord(&mut machine, &["9"]);
    assert_eq!(machine.board.peek(0x5c41), 0);
    assert_eq!(
        edit_text(&machine),
        [&[245, 34, 65, 97][..], &[128, 143, 144]].concat()
    );
    let mut machine = boot();
    chord(&mut machine, &["p"]);
    chord(&mut machine, &["caps-shift", "symbol-shift"]);
    assert_eq!(machine.board.peek(0x5c41), 1);
    cursor_is(&machine, b'E');
    chord(&mut machine, &["q"]);
    assert_eq!(machine.board.peek(0x5c41), 0);
    assert_eq!(edit_text(&machine), &[245, 178]);
    chord(&mut machine, &["0"]);
    chord(&mut machine, &["enter"]);
    settle(&mut machine);
    assert_eq!(machine.board.peek(0x5c3a), 255);
    assert!(machine.cpu.halted);
    let mut machine = boot();
    chord(&mut machine, &["caps-shift", "symbol-shift"]);
    chord(&mut machine, &["symbol-shift", "z"]);
    assert_eq!(edit_text(&machine), &[215]);
}

#[test]
fn keyword_mode_follows_then_and_colons_and_colour_pairs_remain_atomic() {
    let mut machine = boot();
    chord(&mut machine, &["u"]);
    chord(&mut machine, &["1"]);
    chord(&mut machine, &["symbol-shift", "g"]);
    chord(&mut machine, &["p"]);
    chord(&mut machine, &["symbol-shift", "p"]);
    chord(&mut machine, &["symbol-shift", "z"]);
    chord(&mut machine, &["p"]);
    chord(&mut machine, &["symbol-shift", "p"]);
    chord(&mut machine, &["symbol-shift", "z"]);
    chord(&mut machine, &["v"]);
    assert_eq!(
        edit_text(&machine),
        &[250, 49, 203, 245, 34, 58, 112, 34, 58, 251]
    );
    let mut machine = boot();
    chord(&mut machine, &["caps-shift", "symbol-shift"]);
    chord(&mut machine, &["6"]);
    assert_eq!(edit_text(&machine), [17, 6]);
    assert_eq!(machine.board.peek(0x5c0e), 0);
    chord(&mut machine, &["p"]);
    chord(&mut machine, &["enter"]);
    settle(&mut machine);
    assert_eq!(machine.board.peek(0x5c3a), 255);
    assert_eq!(machine.board.peek(0x5c41), 0);
    assert!(machine.cpu.halted);
}

#[test]
fn editor_character_limit_preserves_following_ram_and_still_accepts_enter() {
    let mut machine = boot();
    machine.board.set_literal_typing(true);
    // Enter the bounded guest editing service at its final writable slot, then
    // use real IRQ events to attempt overflow and accept the completed line.
    for address in 0x5ccc..0x6cca {
        machine.board.write(address, b'x');
    }
    machine.board.write(0x6cca, 13);
    machine.board.write(0x6ccb, 0xa5);
    machine.board.write(0x6ccc, 0xa6);
    let assembly = assemble(
        "ld hl,$5ccc\nld bc,4095\nld de,4094\nxor a\ncall $1640\ndi\nhalt",
        0x8000,
        None,
    )
    .unwrap();
    let end = 0x8000 + u16::try_from(assembly.bytes.len()).unwrap();
    for (offset, byte) in assembly.bytes.into_iter().enumerate() {
        machine
            .board
            .write(0x8000 + u16::try_from(offset).unwrap(), byte);
    }
    machine.cpu.registers.pc = 0x8000;
    machine.cpu.registers.sp = 0xff00;
    machine.cpu.halted = false;
    for _ in 0..500 {
        machine.run(MAX_SLICE).unwrap();
        machine.board.drain_audio();
        if machine.cpu.halted {
            break;
        }
    }
    assert!(
        machine.cpu.halted,
        "the full edit line must finish rendering before key input"
    );
    for _ in 0..5 {
        machine.board.write(0x5c08, b'x');
        machine.board.write(0x5c3b, machine.board.peek(0x5c3b) | 32);
        machine.run(FRAME_T_STATES * 2).unwrap();
        machine.board.drain_audio();
    }
    assert_eq!(machine.board.peek(0x6cca), b'x');
    assert_eq!(machine.board.peek(0x6ccb), 13);
    assert_eq!(machine.board.peek(0x6ccc), 0xa6);
    machine.board.write(0x5c08, 13);
    machine.board.write(0x5c3b, machine.board.peek(0x5c3b) | 32);
    for _ in 0..25 {
        machine.run(MAX_SLICE).unwrap();
        machine.board.drain_audio();
        if machine.cpu.halted {
            break;
        }
    }
    assert!(machine.cpu.halted);
    assert_eq!(machine.cpu.registers.pc, end);
    assert_eq!(machine.cpu.registers.hl(), 0x6ccb);
    assert_eq!(machine.board.peek(0x6ccc), 0xa6);
}

#[test]
fn main_editor_limits_input_to_the_free_space_below_its_actual_stack() {
    let mut machine = boot();
    machine.board.set_literal_typing(true);
    for (address, value) in [
        (0x5c3d, 0x00),
        (0x5c3e, 0x5f),
        (0x5f00, 0x40),
        (0x5f01, 0x0e),
    ] {
        machine.board.write(address, value);
    }
    for address in 0x5f08..0x5f40 {
        machine.board.write(address, 0xa5);
    }
    machine.cpu.registers.pc = 0x2000;
    machine.cpu.halted = false;
    machine.run(MAX_SLICE).unwrap();
    for _ in 0..600 {
        machine.board.write(0x5c08, b'x');
        machine.board.write(0x5c3b, machine.board.peek(0x5c3b) | 32);
        machine.run(FRAME_T_STATES * 2).unwrap();
        machine.board.drain_audio();
    }
    let line = word(&machine, 0x5c59);
    let length = (line..0x5f00)
        .take_while(|address| machine.board.peek(*address) == b'x')
        .count();
    assert!(length > 0 && length <= usize::from(0x5f00 - line - 128));
    assert_eq!(
        machine.board.peek(line + u16::try_from(length).unwrap()),
        13
    );
    assert!((0x5f08..0x5f40).all(|address| machine.board.peek(address) == 0xa5));
    assert!(machine.cpu.halted);
}

#[test]
fn long_line_viewport_scrolls_back_and_preserves_edits_and_buffer_guards() {
    fn waiting(machine: &mut Machine) {
        for _ in 0..100 {
            machine.run(MAX_SLICE).unwrap();
            machine.board.drain_audio();
            if machine.cpu.halted && machine.board.editor_ready() {
                return;
            }
        }
        panic!(
            "long editor did not wait at {:04x}",
            machine.cpu.registers.pc
        );
    }
    fn glyph(machine: &Machine, address: u16, character: u8) {
        for scan in 0..8_u16 {
            assert_eq!(
                machine.board.peek(address + scan * 256),
                machine.board.peek(0x3c00 + u16::from(character) * 8 + scan)
            );
        }
    }
    let mut machine = boot();
    machine.board.set_literal_typing(true);
    let mut text = vec![b'x'; 900];
    text[0] = b'A';
    text[158] = b'M';
    text[190] = b'N';
    text[890] = b'Z';
    for (offset, byte) in text.iter().copied().chain([13]).enumerate() {
        machine
            .board
            .write(0x5ccc + u16::try_from(offset).unwrap(), byte);
    }
    machine.board.write(0x5ccc + 1001, 0xa6);
    let code = assemble(
        "ld hl,$5ccc\nld bc,1000\nld de,200\nxor a\ncall $1640\ndi\nhalt",
        0x8000,
        None,
    )
    .unwrap();
    let end = 0x8000 + u16::try_from(code.bytes.len()).unwrap();
    for (offset, byte) in code.bytes.into_iter().enumerate() {
        machine
            .board
            .write(0x8000 + u16::try_from(offset).unwrap(), byte);
    }
    machine.cpu.registers.pc = 0x8000;
    machine.cpu.registers.sp = 0xff00;
    machine.cpu.halted = false;
    waiting(&mut machine);
    assert_eq!(machine.board.peek(0x5c6b), 23);
    glyph(&machine, 0x4020, b'N');
    cursor_is(&machine, b'L');
    for _ in 0..13 {
        chord(&mut machine, &["caps-shift", "5"]);
        waiting(&mut machine);
    }
    glyph(&machine, 0x4020, b'M');
    cursor_is(&machine, b'L');
    for _ in 0..13 {
        chord(&mut machine, &["caps-shift", "8"]);
        waiting(&mut machine);
    }
    chord(&mut machine, &["z"]);
    waiting(&mut machine);
    assert_eq!(machine.board.peek(0x5ccc + 200), b'z');
    assert_eq!(machine.board.peek(0x5ccc + 891), b'Z');
    assert_eq!(machine.board.peek(0x5ccc + 901), 13);
    chord(&mut machine, &["caps-shift", "0"]);
    waiting(&mut machine);
    assert_eq!(
        (0..900)
            .map(|offset| machine.board.peek(0x5ccc + offset))
            .collect::<Vec<_>>(),
        text
    );
    assert_eq!(machine.board.peek(0x5ccc + 1001), 0xa6);
    chord(&mut machine, &["enter"]);
    for _ in 0..100 {
        machine.run(MAX_SLICE).unwrap();
        if machine.cpu.halted && machine.cpu.registers.pc == end {
            break;
        }
    }
    assert_eq!(machine.cpu.registers.pc, end);
    assert_eq!(machine.cpu.registers.hl(), 0x5ccc + 900);
}

#[test]
fn guest_machine_code_uses_public_print_entry_and_at_control_bytes() {
    let mut machine = boot();
    let assembly = assemble(
        "ld a,2\ncall $1601\nld a,22\nrst $10\nld a,3\nrst $10\nld a,4\nrst $10\nld a,'Z'\nrst $10\nhalt",
        0x8000,
        None,
    )
    .unwrap();
    for (offset, byte) in assembly.bytes.iter().enumerate() {
        machine
            .board
            .write(0x8000 + u16::try_from(offset).unwrap(), *byte);
    }
    machine.cpu.registers.pc = 0x8000;
    machine.cpu.halted = false;
    machine.cpu.registers.iff1 = false;
    machine.cpu.registers.iff2 = false;
    machine.run(20000).unwrap();
    assert!(machine.cpu.halted);
    for line in 0..8_u16 {
        let address = 0x4064 + line * 256;
        let glyph = 0x3c00 + u16::from(b'Z') * 8 + line;
        assert_eq!(machine.board.peek(address), machine.board.peek(glyph));
    }
    assert_eq!(machine.board.peek(0x5864), 56);
    assert_eq!(machine.board.peek(0x5c88), 28);
    assert_eq!(machine.board.peek(0x5c89), 21);
}

#[test]
fn public_print_wraps_before_the_thirty_third_character_without_overwriting_a_column() {
    let mut machine = boot();
    let source = "ld a,2\ncall $1601\nld a,22\nrst $10\nld a,10\nrst $10\nxor a\nrst $10\nld hl,text\nld b,33\nloop: ld a,(hl)\nrst $10\ninc hl\ndjnz loop\nhalt\ntext: ds 33,'A'";
    let assembly = assemble(source, 0x8000, None).unwrap();
    for (offset, byte) in assembly.bytes.iter().enumerate() {
        machine
            .board
            .write(0x8000 + u16::try_from(offset).unwrap(), *byte);
    }
    machine.cpu.registers.pc = 0x8000;
    machine.cpu.halted = false;
    machine.cpu.registers.iff1 = false;
    machine.cpu.registers.iff2 = false;
    machine.run(150_000).unwrap();
    for line in 0..8_u16 {
        let glyph = machine.board.peek(0x3c00 + u16::from(b'A') * 8 + line);
        for address in [0x4840, 0x485f, 0x4860] {
            assert_eq!(machine.board.peek(address + line * 256), glyph);
        }
        assert_eq!(machine.board.peek(0x4861 + line * 256), 0);
    }
    assert_eq!(machine.board.peek(0x5c88), 32);
    assert_eq!(machine.board.peek(0x5c89), 13);
}

#[test]
fn new_preserves_documented_configuration_and_reserved_code_while_resetting_workspace() {
    let mut machine = boot();
    for (address, value) in [(0x5cb2, 0x7fff_u16), (0x5c7b, 0x7f00), (0x5c38, 0x4321)] {
        for (offset, byte) in value.to_le_bytes().into_iter().enumerate() {
            machine
                .board
                .write(address + u16::try_from(offset).unwrap(), byte);
        }
    }
    machine.board.write(0xc000, 0xa5);
    machine.board.write(0x7f00, 0xc3);
    for (offset, byte) in b"NEW\r".iter().copied().enumerate() {
        machine
            .board
            .write(0x5ccc + u16::try_from(offset).unwrap(), byte);
    }
    for (offset, byte) in [0xcd, 0x40, 0x1e, 0xcd, 0, 0x1e, 0x76]
        .into_iter()
        .enumerate()
    {
        machine
            .board
            .write(0x8000 + u16::try_from(offset).unwrap(), byte);
    }
    machine.cpu.registers.pc = 0x8000;
    machine.cpu.registers.sp = 0xff00;
    machine.cpu.halted = false;
    for _ in 0..3 {
        machine.run(MAX_SLICE).unwrap();
        machine.board.drain_audio();
    }
    assert!(machine.cpu.halted);
    assert_eq!(machine.board.firmware_page(), 0);
    assert_eq!(word(&machine, 0x5c53), 0x5ccb);
    assert_eq!(word(&machine, 0x5c4b), 0x5ccb);
    assert_eq!(word(&machine, 0x5cb2), 0x7fff);
    assert_eq!(word(&machine, 0x5c7b), 0x7f00);
    assert_eq!(word(&machine, 0x5c38), 0x4321);
    assert_eq!(word(&machine, 0x5cb4), 0xffff);
    assert_eq!(machine.board.peek(0xc000), 0xa5);
    assert_eq!(machine.board.peek(0x7f00), 0xc3);
    assert_eq!(machine.board.peek(0x5ccb), 0x80);
    assert_eq!(machine.board.peek(0x5ccc), 13);
    assert!(machine.cpu.registers.sp < 0x7fff);
}
