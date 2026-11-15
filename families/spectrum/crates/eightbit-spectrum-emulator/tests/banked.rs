//! Original synthetic cases for UK 128K and grey +2; no reference ROM bytes.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_spectrum_emulator::{Board, Firmware, Machine, Model, WIDTH};
use eightbit_z80_core::{Access, Bus};
fn board(model: Model) -> Board {
    Board::with_model(Firmware::new(&vec![0; 16384]).unwrap(), model)
}
#[test]
fn eight_banks_alias_fixed_windows_lock_until_reset_and_leave_rom_read_only() {
    for model in [Model::Spectrum128, Model::Plus2] {
        let mut machine = Machine::with_model(Firmware::new(&vec![0; 16384]).unwrap(), model);
        for bank in 0..8 {
            machine.board.output(0x7ffd, bank);
            machine.board.write(0xc000, bank + 10);
        }
        for bank in 0..8 {
            machine.board.output(0x7ffd, bank);
            assert_eq!(machine.board.peek(0xc000), bank + 10);
        }
        assert_eq!(machine.board.peek(0x4000), 15);
        assert_eq!(machine.board.peek(0x8000), 12);
        machine.board.output(0x3ffd, 5); // Hardware decodes only A15/A1.
        machine.board.write(0xffff, 99);
        assert_eq!(machine.board.peek(0x7fff), 99);
        machine.board.write(0, 42);
        assert_eq!(machine.board.peek(0), 0);
        machine.board.output(0x7ffd, 0x32);
        assert_eq!(machine.board.paging(), 0x32);
        machine.board.output(0x7ffd, 0);
        assert_eq!(machine.board.paging(), 0x32);
        machine.reset();
        assert_eq!(machine.board.paging(), 0);
        machine.board.output(0x7ffd, 5);
        assert_eq!(machine.board.peek(0xffff), 99);
    }
    let mut legacy = board(Model::Spectrum48);
    legacy.write(0xc000, 99);
    legacy.output(0x7ffd, 7);
    assert_eq!(legacy.peek(0xc000), 99);
    assert_eq!(legacy.paging(), 0);
}
#[test]
fn shadow_screen_and_floating_bus_follow_display_selection_not_cpu_bank() {
    for model in [Model::Spectrum128, Model::Plus2] {
        let mut board = board(model);
        board.write(0x4000, 0x80);
        board.write(0x5800, 2);
        board.output(0x7ffd, 7);
        board.write(0xc000, 0x80);
        board.write(0xd800, 4);
        board.output(0x7ffd, 0);
        board.tick(model.frame_t_states());
        let p = (32 * WIDTH + 32) * 4;
        assert_eq!(&board.frame()[p..p + 4], &[205, 0, 0, 255]);
        board.output(0x7ffd, 8);
        board.tick(model.frame_t_states());
        assert_eq!(&board.frame()[p..p + 4], &[0, 205, 0, 255]);
        board.tick(14_364);
        assert_eq!(board.input(0xffff), 0x80);
        board.tick(1);
        assert_eq!(board.input(0xffff), 4);
    }
}
#[test]
fn bank_contention_ay_decoding_and_private_gates_do_not_interfere() {
    for model in [Model::Spectrum128, Model::Plus2] {
        let mut board = board(model);
        board.tick(14_361);
        assert_eq!(board.wait_states(0x4000, Access::Read), 6);
        assert_eq!(board.wait_states(0x8000, Access::Read), 0);
        for bank in 0..8 {
            board.output(0x7ffd, bank);
            assert_eq!(
                board.wait_states(0xc000, Access::Read),
                if bank & 1 != 0 { 6 } else { 0 }
            );
        }
        board.output(0xfffd, 1);
        board.output(0xbffd, 255);
        assert_eq!(board.input(0xfffd), 15);
        board.fetch(0x1f0);
        board.output(0xfdfd, 0);
        assert_eq!(board.input(0xfffd), 15);
        assert_eq!(
            board.input(0xfdf9),
            if model == Model::Plus2 { 2 } else { 1 }
        );
        board.tick(228);
        assert_eq!(board.wait_states(0x4000, Access::Read), 6);
        board.tick(model.frame_t_states() - 14_361 - 228);
        assert!(board.irq());
        board.tick(36);
        assert!(!board.irq());
    }
}

fn word(machine: &Machine, address: u16) -> u16 {
    u16::from_le_bytes([machine.board.peek(address), machine.board.peek(address + 1)])
}
fn set_word(machine: &mut Machine, address: u16, value: u16) {
    let [lo, hi] = value.to_le_bytes();
    machine.board.write(address, lo);
    machine.board.write(address + 1, hi);
}
fn boot(model: Model) -> Machine {
    let mut machine = Machine::development_model(model).unwrap();
    for _ in 0..8 {
        machine.run(350_000).unwrap();
        machine.board.drain_audio();
    }
    machine
}
fn execute(machine: &mut Machine, source: &str, error: Option<u8>) -> Vec<i16> {
    let line = word(machine, 0x5c59);
    for (i, byte) in source.bytes().chain([13]).enumerate() {
        machine.board.write(line + u16::try_from(i).unwrap(), byte);
    }
    for (i, byte) in [0xcd, 0x40, 0x1e, 0xcd, 0, 0x1e, 0x76]
        .into_iter()
        .enumerate()
    {
        machine
            .board
            .write(0x8000 + u16::try_from(i).unwrap(), byte);
    }
    set_word(machine, 0x5c3d, 0xff00);
    set_word(machine, 0xff00, 0x8100);
    machine.board.write(0x8100, 0xf3);
    machine.board.write(0x8101, 0x76);
    machine.cpu.registers.pc = 0x8000;
    machine.cpu.registers.sp = 0xff00;
    machine.cpu.registers.iff1 = false;
    machine.cpu.registers.iff2 = false;
    machine.cpu.halted = false;
    let mut audio = Vec::new();
    for slice in 0..400 {
        machine.run(350_000).unwrap();
        audio.extend(machine.board.drain_audio());
        if source == "NEW" && slice == 7 {
            assert_eq!(word(machine, 0x5c53), 0x5ccb);
            assert_eq!(machine.board.peek(word(machine, 0x5c4b)), 128);
            return audio;
        }
        if machine.cpu.halted && [0x8007, 0x8102].contains(&machine.cpu.registers.pc) {
            break;
        }
    }
    assert_eq!(
        machine.cpu.registers.pc,
        if error.is_some() { 0x8102 } else { 0x8007 },
        "{source}: error {} page {}",
        machine.board.peek(0x5c3a),
        machine.board.firmware_page()
    );
    if let Some(error) = error {
        assert_eq!(machine.board.peek(0x5c3a), error, "{source}");
    }
    assert_eq!(machine.board.firmware_page(), 0);
    audio
}
fn ay(machine: &mut Machine, r: u8) -> u8 {
    machine.board.output(0xfffd, r);
    machine.board.input(0xfffd)
}
#[test]
fn guest_play_runs_three_voices_and_extended_controls_then_returns_to_basic() {
    for model in [Model::Spectrum128, Model::Plus2] {
        let mut machine = boot(model);
        let audio = execute(
            &mut machine,
            "PLAY \"T240O5N1C\",\"O4N1E\",\"O3N1G\":POKE 49152,123",
            None,
        );
        assert_eq!(machine.board.peek(0xc000), 123);
        assert!(audio.iter().any(|n| *n != -8192));
        assert_eq!(
            u16::from_le_bytes([ay(&mut machine, 0), ay(&mut machine, 1)]),
            424
        );
        for r in 8..11 {
            assert_eq!(ay(&mut machine, r), 0);
        }
        execute(
            &mut machine,
            "PLAY \"T240O5N1(#C$D)&!comment!UX12W7M56C\":POKE 49153,42",
            None,
        );
        assert_eq!(machine.board.peek(0xc001), 42);
        assert_eq!(ay(&mut machine, 7) & 63, 7);
        execute(&mut machine, "PLAY \"O9C\"", Some(10));
        execute(&mut machine, "PLAY \"T240O5N1CH\",\"O2N1CG))\"", None);
        execute(&mut machine, "SPECTRUM:POKE 49154,99", None);
        assert_eq!(machine.board.paging(), 0x30);
        assert_eq!(machine.board.peek(0xc002), 99);
        execute(&mut machine, "PLAY \"C\"", Some(11));
        machine.reset();
        for _ in 0..8 {
            machine.run(350_000).unwrap();
            machine.board.drain_audio();
        }
        assert_eq!(machine.board.paging(), 0);
        execute(&mut machine, "PLAY \"T240O5N1C\"", None);
    }
    let mut legacy = boot(Model::Spectrum48);
    // Native token 164 is a UDG in the legacy machine, never an extended statement.
    execute(&mut legacy, "PLAY \"C\"", Some(11));
}
#[test]
fn guest_ram_disc_code_spans_banks_reuses_file_validation_and_survives_new() {
    for model in [Model::Spectrum128, Model::Plus2] {
        let mut machine = boot(model);
        for address in 0x9000..0xde20 {
            machine
                .board
                .write(address, address.to_le_bytes()[0] ^ address.to_le_bytes()[1]);
        }
        execute(&mut machine, "SAVE! \"code\" CODE 36864,20000", None);
        for address in 0x9000..0xde20 {
            machine.board.write(address, 0);
        }
        execute(&mut machine, "LOAD! \"code\" CODE 36864,20000", None);
        for address in 0x9000..0xde20 {
            assert_eq!(
                machine.board.peek(address),
                address.to_le_bytes()[0] ^ address.to_le_bytes()[1],
                "{address:04x}"
            );
        }
        execute(&mut machine, "SAVE! \"zeta\" CODE 36864,20000", None);
        execute(&mut machine, "SAVE! \"alpha\" CODE 36864,20000", None);
        execute(&mut machine, "ERASE! \"zeta\"", None);
        for _ in 0..2 {
            execute(&mut machine, "SAVE! \"alpha\" CODE 36864,20000", None);
        }
        execute(&mut machine, "NEW", None);
        execute(&mut machine, "LOAD! \"code\" CODE 36864,20000", None);
        execute(&mut machine, "CAT!", None);
        execute(&mut machine, "ERASE! \"code\"", None);
        execute(&mut machine, "LOAD! \"code\" CODE 36864,20000", Some(14));
        execute(&mut machine, "VERIFY! \"code\"", Some(11));
    }
}

#[test]
fn guest_ram_disc_round_trips_programmes_merges_arrays_and_screen_files() {
    for model in [Model::Spectrum128, Model::Plus2] {
        let mut machine = boot(model);
        execute(&mut machine, "10 LET A=42", None);
        execute(&mut machine, "20 STOP", None);
        let end = word(&machine, 0x5c4b);
        let original: Vec<_> = (0x5ccb..end).map(|a| machine.board.peek(a)).collect();
        execute(&mut machine, "SAVE! \"basic\"", None);
        execute(&mut machine, "NEW", None);
        execute(&mut machine, "LOAD! \"basic\"", None);
        assert_eq!(word(&machine, 0x5c4b), end);
        assert_eq!(
            (0x5ccb..end)
                .map(|a| machine.board.peek(a))
                .collect::<Vec<_>>(),
            original
        );
        execute(&mut machine, "NEW", None);
        execute(&mut machine, "10 LET A=7", None);
        execute(&mut machine, "30 STOP", None);
        execute(&mut machine, "MERGE! \"basic\"", None);
        let mut line = word(&machine, 0x5c53);
        for number in [10_u16, 20, 30] {
            assert_eq!(
                u16::from_be_bytes([machine.board.peek(line), machine.board.peek(line + 1)]),
                number
            );
            line += 4 + word(&machine, line + 2);
        }
        assert_eq!(line, word(&machine, 0x5c4b));
        assert_eq!(
            (0x5ccb..end)
                .map(|a| machine.board.peek(a))
                .collect::<Vec<_>>(),
            original
        );
        execute(
            &mut machine,
            "DIM A(2):LET A(1)=42:DIM B$(2,3):LET B$(2)=\"xyz\"",
            None,
        );
        execute(
            &mut machine,
            "SAVE! \"nums\" DATA A():SAVE! \"chars\" DATA B$()",
            None,
        );
        execute(&mut machine, "CLEAR", None);
        execute(
            &mut machine,
            "LOAD! \"nums\" DATA A():LOAD! \"chars\" DATA B$()",
            None,
        );
        execute(&mut machine, "POKE 49152,A(1):POKE 49153,CODE B$(2)", None);
        assert_eq!(machine.board.peek(0xc000), 42);
        assert_eq!(machine.board.peek(0xc001), b'x');
        execute(&mut machine, "CLS:PRINT AT 3,4;\"Z\";", None);
        let screen: Vec<_> = (0x4000..0x5b00).map(|a| machine.board.peek(a)).collect();
        execute(
            &mut machine,
            "SAVE! \"screen\" SCREEN$:CLS:LOAD! \"screen\" SCREEN$",
            None,
        );
        assert_eq!(
            (0x4000..0x5b00)
                .map(|a| machine.board.peek(a))
                .collect::<Vec<_>>(),
            screen
        );
        execute(
            &mut machine,
            "SAVE! \"large1\" CODE 36864,20000:SAVE! \"large2\" CODE 36864,20000",
            None,
        );
        execute(&mut machine, "SAVE! \"nums\" CODE 24576,40960", Some(3));
        execute(
            &mut machine,
            "CLEAR:LOAD! \"nums\" DATA A():POKE 49152,A(1)",
            None,
        );
        assert_eq!(machine.board.peek(0xc000), 42);
    }
}
