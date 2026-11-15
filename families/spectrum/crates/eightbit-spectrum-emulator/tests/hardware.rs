//! Original synthetic firmware/programmes; no Sinclair ROM or external binary output.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::cast_possible_truncation)]
use eightbit_spectrum_emulator::{
    AUDIO_HZ, Board, CONTENTION_START, CPU_HZ, DISPLAY_START, Error, FRAME_T_STATES, Firmware, Key,
    MAX_SLICE, Machine, PRIVATE_PAGE_PORT, Tape, WIDTH, contention,
};
use eightbit_z80_core::{Access, Bus, assembler::assemble};

fn board() -> Board {
    Board::new(Firmware::new(&vec![0; 16384]).unwrap())
}
fn pixel(board: &Board, x: usize, y: usize) -> &[u8] {
    &board.frame()[(y * WIDTH + x) * 4..(y * WIDTH + x) * 4 + 4]
}
fn tap(bytes: &[u8]) -> Vec<u8> {
    let mut result = u16::try_from(bytes.len()).unwrap().to_le_bytes().to_vec();
    result.extend(bytes);
    result
}

#[test]
fn expanded_firmware_is_private_and_retains_all_49152_ram_bytes() {
    let mut bytes = vec![0; 24576];
    bytes[0x2000] = 0xa5;
    bytes[0x4000] = 0x5a;
    let mut board = Board::new(Firmware::new(&bytes).unwrap());
    for address in 0x4000..=0xffff {
        board.write(address, (address >> 8) as u8 ^ address as u8);
    }
    board.write(0x2000, 1);
    assert_eq!(board.peek(0x2000), 0xa5);
    board.fetch(0x8000);
    board.output(PRIVATE_PAGE_PORT, 1);
    assert_eq!(board.firmware_page(), 0);
    board.fetch(0x01f1);
    board.output(PRIVATE_PAGE_PORT, 1);
    assert_eq!(board.peek(0x2000), 0x5a);
    assert_eq!(board.peek(0), bytes[0]);
    assert_eq!(board.input(PRIVATE_PAGE_PORT), 1);
    for address in 0x4000..=0xffff {
        assert_eq!(board.peek(address), (address >> 8) as u8 ^ address as u8);
    }
    assert!(Firmware::new(&[]).is_err());
    assert!(Firmware::new(&vec![0; 16385]).is_err());
    assert!(Firmware::new(&vec![0; 8192 * 130]).is_err());
}

#[test]
fn literal_typing_flag_is_visible_only_to_the_original_editor_page() {
    let mut machine = Machine::development().unwrap();
    machine.board.set_literal_typing(true);
    for caller in [0x1000, 0x2000, 0x4000, 0x8000] {
        machine.board.fetch(caller);
        assert_eq!(machine.board.input(0xfdfb), 255);
    }
    machine.board.fetch(0x01f1);
    machine.board.output(PRIVATE_PAGE_PORT, 16);
    machine.board.fetch(0x2100);
    assert_eq!(machine.board.input(0xfdfb), 1);
    machine.board.output(0xfdfb, 1);
    assert!(machine.board.editor_ready());
    machine.board.set_literal_typing(false);
    assert_eq!(machine.board.input(0xfdfb), 0);
    machine.board.set_literal_typing(true);
    machine.board.fetch(0x8000);
    assert_eq!(machine.board.input(0xfdfb), 255);
    machine.board.output(0xfdfb, 0);
    assert!(machine.board.editor_ready());
    machine.reset();
    assert!(!machine.board.editor_ready());
    machine.board.fetch(0x01f1);
    machine.board.output(PRIVATE_PAGE_PORT, 16);
    machine.board.fetch(0x2100);
    assert_eq!(machine.board.input(0xfdfb), 0);
}

#[test]
fn actual_z80_gate_selects_pages_and_invalid_page_stops_until_reset() {
    let source = "ld bc,$fdfd\nld a,1\njp $1f0\norg $1f0\nout (c),a\njp $2000\norg $2000\nhalt";
    let assembly = assemble(source, 0, None).unwrap();
    let mut firmware = vec![0; 24576];
    firmware[..assembly.bytes.len()].copy_from_slice(&assembly.bytes);
    firmware[0x4000..0x4004].copy_from_slice(&[0x3e, 0x42, 0x76, 0]);
    let mut machine = Machine::new(Firmware::new(&firmware).unwrap());
    machine.run(200).unwrap();
    assert_eq!(machine.board.firmware_page(), 1);
    assert_eq!(machine.cpu.registers.a, 0x42);
    assert!(machine.cpu.halted);
    machine.board.fetch(0x01f1);
    machine.board.output(PRIVATE_PAGE_PORT, 2);
    assert_eq!(machine.step(), Err(Error::FirmwarePage));
    machine.reset();
    assert_eq!(machine.board.firmware_page(), 0);
    assert!(machine.step().is_ok());
}

#[test]
fn spectrum_bitmap_interleaving_attributes_brightness_and_border_are_rendered() {
    let mut board = board();
    board.output(0xfe, 5);
    for y in [0_u16, 7, 8, 63, 64, 127, 128, 191] {
        let address = 0x4000 | ((y & 0xc0) << 5) | ((y & 7) << 8) | ((y & 0x38) << 2);
        board.write(address, 0x80);
        board.write(address + 31, 1);
        board.write(0x5800 + (y / 8) * 32, 0x42);
        board.write(0x5800 + (y / 8) * 32 + 31, 0x0c);
    }
    board.tick(FRAME_T_STATES);
    for y in [0, 7, 8, 63, 64, 127, 128, 191] {
        assert_eq!(pixel(&board, 32, 32 + y), &[255, 0, 0, 255]);
        assert_eq!(pixel(&board, 33, 32 + y), &[0, 0, 0, 255]);
        assert_eq!(pixel(&board, 287, 32 + y), &[0, 205, 0, 255]);
        assert_eq!(pixel(&board, 286, 32 + y), &[0, 0, 205, 255]);
    }
    for (x, y) in [(0, 0), (319, 0), (0, 255), (319, 255), (31, 32), (288, 223)] {
        assert_eq!(pixel(&board, x, y), &[0, 205, 205, 255]);
    }
}

#[test]
fn pixel_and_attribute_latches_do_not_change_in_the_middle_of_a_character_byte() {
    let mut board = board();
    board.write(0x4000, 255);
    board.write(0x5800, 7);
    board.tick(DISPLAY_START + 1);
    board.write(0x4000, 0);
    board.write(0x5800, 2);
    board.tick(FRAME_T_STATES - DISPLAY_START - 1);
    for x in 32..40 {
        assert_eq!(pixel(&board, x, 32), &[205, 205, 205, 255]);
    }
}

#[test]
fn flash_alternates_after_sixteen_frames() {
    let mut board = board();
    board.write(0x4000, 0x80);
    board.write(0x5800, 0x8a);
    board.tick(FRAME_T_STATES);
    assert_eq!(pixel(&board, 32, 32), &[205, 0, 0, 255]);
    assert_eq!(pixel(&board, 33, 32), &[0, 0, 205, 255]);
    for _ in 0..16 {
        board.tick(FRAME_T_STATES);
        board.drain_audio();
    }
    assert_eq!(pixel(&board, 32, 32), &[0, 0, 205, 255]);
    assert_eq!(pixel(&board, 33, 32), &[205, 0, 0, 255]);
}

#[test]
fn keyboard_rows_combine_active_low_and_release_without_resetting_tape() {
    let mut board = board();
    board.key(Key::named("caps-shift").unwrap(), true);
    board.key(Key::named("x").unwrap(), true);
    board.key(Key::named("Q").unwrap(), true);
    assert_eq!(board.input(0xfefe) & 31, 26);
    assert_eq!(board.input(0xfbfe) & 31, 30);
    assert_eq!(board.input(0xfafe) & 31, 26);
    assert_eq!(board.input(0xfffe) & 31, 31);
    board.key(Key::named("symbol-shift").unwrap(), true);
    assert_eq!(board.input(0x7ffe) & 31, 29);
    board.release_keys();
    assert_eq!(board.input(0xfe) & 31, 31);
    assert!(Key::named("not-a-key").is_err());
}

#[test]
fn contention_and_io_phase_delays_follow_visible_scanlines_only() {
    assert_eq!(contention(CONTENTION_START - 1), 0);
    let pattern: Vec<_> = (0..8).map(|i| contention(CONTENTION_START + i)).collect();
    assert_eq!(pattern, [6, 5, 4, 3, 2, 1, 0, 0]);
    assert_eq!(contention(CONTENTION_START + 128), 0);
    assert_eq!(contention(CONTENTION_START + 224), 6);
    assert_eq!(contention(CONTENTION_START + 224 * 192), 0);
    assert_eq!(contention(FRAME_T_STATES + CONTENTION_START), 6);
    let mut board = board();
    board.tick(CONTENTION_START);
    assert_eq!(board.wait_states(0x4000, Access::Read), 6);
    assert_eq!(board.wait_states(0x8000, Access::Read), 0);
    assert_eq!(board.wait_states(0x40ff, Access::Input), 12);
    assert_eq!(board.wait_states(0x40fe, Access::Output), 6);
    assert_eq!(board.wait_states(0x80fe, Access::Output), 5);
    assert_eq!(board.wait_states(0x80ff, Access::Input), 0);
}

#[test]
fn irq_audio_and_mic_are_bounded_and_observe_actual_guest_bus_time() {
    let mut board = board();
    assert!(board.irq());
    board.tick(32);
    assert!(!board.irq());
    board.tick(FRAME_T_STATES - 32);
    assert!(board.irq());
    board.drain_audio();
    board.output(0xfe, 16);
    board.tick(CPU_HZ / 100);
    let audio = board.drain_audio();
    assert!((i64::try_from(audio.len()).unwrap() - i64::from(AUDIO_HZ / 100)).abs() <= 1);
    assert!(audio.iter().skip(1).all(|v| *v == 8192));
    board.record_tape(true);
    board.tick(855);
    board.output(0xfe, 8);
    board.tick(855);
    board.output(0xfe, 0);
    let edges = board.drain_tape_edges().unwrap();
    assert_eq!(edges.len(), 3);
    assert_eq!(edges[1].t_state - edges[0].t_state, 855);
    assert_eq!(edges[2].t_state - edges[1].t_state, 855);
    assert!(edges[1].high);
    assert!(!edges[2].high);
    board.tick(CPU_HZ);
    assert!(board.audio_lost() > 0);
    assert!(board.drain_audio().len() <= 8192);
}

#[test]
fn tape_streams_pilot_sync_and_bits_and_preserves_position_on_pause_or_bad_mount() {
    let bytes = tap(&[0x80, 0x80]);
    let mut tape = Tape::parse(&bytes).unwrap();
    assert!(!tape.playing());
    tape.play();
    tape.advance(2167);
    assert!(!tape.high());
    tape.advance(1);
    assert!(tape.high());
    tape.advance(2168 * 3222);
    assert!(tape.high());
    tape.advance(667);
    assert!(!tape.high());
    tape.advance(735);
    assert!(tape.high());
    tape.advance(1710);
    assert!(!tape.high());
    tape.advance(1710);
    assert!(tape.high());
    tape.advance(855);
    assert!(!tape.high());
    tape.pause();
    let elapsed = tape.elapsed();
    tape.advance(9999);
    assert_eq!(tape.elapsed(), elapsed);
    let mut board = board();
    board.mount_tap(&bytes).unwrap();
    board.tape_mut().unwrap().play();
    board.tick(5000);
    assert!(board.mount_tap(&[1, 0]).is_err());
    assert_eq!(board.tape().unwrap().elapsed(), 5000);
    board.tape_mut().unwrap().rewind();
    assert_eq!(board.tape().unwrap().elapsed(), 0);
    assert!(!board.tape().unwrap().playing());
}

#[test]
fn tape_finishes_and_rewinds_and_keeps_opaque_checksum_bytes_for_guest_validation() {
    for bytes in [&[][..], &[0][..], &[0, 0][..], &[3, 0, 1, 2][..]] {
        assert!(Tape::parse(bytes).is_err());
    }
    let mut bytes = tap(&[0xff]);
    bytes.extend(tap(&[0x22, 0x33]));
    let mut tape = Tape::parse(&bytes).unwrap();
    assert_eq!(tape.blocks(), 2);
    tape.play();
    tape.advance(100_000_000);
    assert!(!tape.playing());
    assert_eq!(tape.block(), 2);
    tape.rewind();
    assert_eq!(tape.block(), 0);
    assert_eq!(tape.elapsed(), 0);
    tape.play();
    tape.advance(2168);
    assert!(tape.high());
}

#[test]
fn execution_is_bounded_and_an_original_assembled_guest_draws_without_external_tools() {
    let source = "jp start\norg $38\nei\nret\norg $100\nstart: di\nld sp,$ffff\nld hl,$5800\nld (hl),$47\nld de,$5801\nld bc,767\nldir\nld hl,$4000\nld (hl),$aa\nld de,$4001\nld bc,6143\nldir\nld a,2\nout ($fe),a\nei\nloop: halt\njr loop";
    let assembly = assemble(source, 0, Some("start")).unwrap();
    let mut image = vec![0; 16384];
    image[..assembly.bytes.len()].copy_from_slice(&assembly.bytes);
    let mut machine = Machine::new(Firmware::new(&image).unwrap());
    assert_eq!(machine.run(MAX_SLICE + 1), Err(Error::Slice));
    for _ in 0..5 {
        machine.run(FRAME_T_STATES).unwrap();
        machine.board.drain_audio();
    }
    assert_eq!(machine.board.peek(0x4000), 0xaa);
    assert_eq!(machine.board.peek(0x57ff), 0xaa);
    assert_eq!(machine.board.peek(0x5800), 0x47);
    assert_eq!(pixel(&machine.board, 32, 32), &[255, 255, 255, 255]);
    assert_eq!(pixel(&machine.board, 33, 32), &[0, 0, 0, 255]);
    assert!(machine.cpu.halted || machine.cpu.registers.pc >= 0x38);
    let mut machine = Machine::new(Firmware::new(&vec![0xdd; 16384]).unwrap());
    assert!(matches!(machine.step(), Err(Error::Cpu(_))));
}
