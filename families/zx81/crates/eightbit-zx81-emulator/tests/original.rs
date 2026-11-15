//! Boot and cassette vectors use only our original guest code and generated media.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_zx81_emulator::{FRAME_T_STATES, Key, Machine, Model, image::Image};
fn frames(machine: &mut Machine, count: usize) {
    for _ in 0..count {
        machine.run(FRAME_T_STATES).unwrap();
    }
}
#[test]
fn own_guest_save_round_trips_signal_and_pcm_without_host_memory_loading() {
    use eightbit_zx81_emulator::recording;
    for model in [
        Model::Zx80_1k,
        Model::Zx80_16k,
        Model::Zx81_1k,
        Model::Zx81_16k,
    ] {
        let mut machine = Machine::development(model).unwrap();
        frames(&mut machine, 20);
        key(&mut machine, if model.zx80() { "e" } else { "s" });
        machine.board.record_tape(true);
        key(&mut machine, "enter");
        frames(&mut machine, 550);
        let edges = machine.board.drain_tape_edges().unwrap();
        let saved = recording::decode(model, &edges).unwrap();
        assert!(saved.image.programme().is_empty());
        let wave = recording::wave(&edges).unwrap();
        machine.board.record_tape(false);
        machine.board.mount_wave(&wave).unwrap();
        key(&mut machine, if model.zx80() { "w" } else { "j" });
        key(&mut machine, "enter");
        machine.board.play_tape();
        frames(&mut machine, 600);
        assert!(!machine.board.tape_pending());
        assert!(
            machine.board.typing_ready(),
            "{model:?}: WAV LOAD did not return"
        );
    }
}
fn key(machine: &mut Machine, name: &str) {
    machine.board.set_key(Key::named(name).unwrap(), true);
    frames(machine, 4);
    machine.board.release_keys();
    frames(machine, 4);
}
#[test]
fn own_guest_boots_in_physical_ram_and_reads_keyboard_loads_cassette() {
    for model in [
        Model::Zx80_1k,
        Model::Zx80_16k,
        Model::Zx81_1k,
        Model::Zx81_16k,
    ] {
        let mut machine = Machine::development(model).unwrap();
        frames(&mut machine, 20);
        assert!(
            machine.board.typing_ready(),
            "{model:?} guest prompt is not ready"
        );
        assert!(
            machine
                .board
                .frame()
                .chunks_exact(4)
                .any(|pixel| pixel == [0, 0, 0, 255]),
            "{model:?}: no guest-generated cursor"
        );
        let programme = if model.zx80() {
            vec![0, 10, 244, 0, 29, 118]
        } else {
            vec![0, 10, 4, 0, 245, 0, 29, 118]
        };
        let image = Image::from_program(model, &programme).unwrap();
        machine.board.mount_image(image.bytes(), "STUDIO").unwrap();
        key(&mut machine, if model.zx80() { "w" } else { "j" });
        key(&mut machine, "enter");
        assert!(!machine.board.typing_ready());
        machine.board.play_tape();
        frames(&mut machine, 1800);
        assert!(
            !machine.board.tape_pending(),
            "{model:?}: signal has not finished"
        );
        assert!(
            machine.board.typing_ready(),
            "{model:?}: guest LOAD did not return"
        );
        for (offset, byte) in programme.iter().enumerate() {
            assert_eq!(
                machine
                    .board
                    .peek(model.programme_start() + u16::try_from(offset).unwrap()),
                *byte,
                "{model:?} loaded byte {offset}"
            );
        }
        machine.reset();
        frames(&mut machine, 20);
        assert!(machine.board.typing_ready());
        assert!(machine.board.tape_pending()); // Reset rewinds, and does not autoplay.
    }
}
