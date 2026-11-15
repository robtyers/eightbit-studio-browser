//! Opt-in black-box observations. Third-party ROMs are never fixtures or output.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_zx81_emulator::{FRAME_T_STATES, Firmware, Key, Machine, Model};
use sha2::{Digest, Sha256};
fn reference(model: Model) -> Machine {
    let (variable, hash) = if model.zx80() {
        (
            "EIGHTBIT_ZX80_ROM",
            "9374711cb6a5fd53c1c98d1d20ecb444d01d621a322f3d066ef216d515ac5161",
        )
    } else {
        (
            "EIGHTBIT_ZX81_ROM",
            "14ad84f4243efcd41587ff46ab932d11087043e8d455a1ed2a227b9657828dfa",
        )
    };
    let path = std::env::var(variable).expect("Set the explicit local reference ROM input");
    let metadata = std::fs::symlink_metadata(&path).unwrap();
    assert!(metadata.is_file());
    assert_eq!(metadata.len(), u64::try_from(model.rom_bytes()).unwrap());
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        hash,
        "Reference identity mismatch"
    );
    Machine::new(model, Firmware::reference(&bytes, model).unwrap())
}
fn frames(machine: &mut Machine, count: usize) {
    for _ in 0..count {
        machine.run(FRAME_T_STATES).unwrap();
    }
}
fn key(machine: &mut Machine, name: &str, shift: bool) {
    machine.board.set_key(Key::named(name).unwrap(), true);
    if shift {
        machine.board.set_key(Key::named("shift").unwrap(), true);
    }
    frames(machine, 4);
    machine.board.release_keys();
    frames(machine, 4);
}
#[test]
#[ignore = "Requires explicit lawfully supplied ZX80 and ZX81 ROM paths"]
fn observe_save_signal_timing_without_inspecting_reference_code() {
    use std::collections::BTreeMap;
    for model in [Model::Zx80_1k, Model::Zx81_1k] {
        let mut machine = reference(model);
        frames(&mut machine, 200);
        key(&mut machine, if model.zx80() { "e" } else { "s" }, false);
        if !model.zx80() {
            key(&mut machine, "p", true);
            key(&mut machine, "a", false);
            key(&mut machine, "p", true);
        }
        machine.board.record_tape(true);
        key(&mut machine, "enter", false);
        frames(&mut machine, 500);
        let edges = machine.board.drain_tape_edges().unwrap();
        let mut counts = BTreeMap::<u64, usize>::new();
        for pair in edges.windows(2) {
            let duration = pair[1].t_state - pair[0].t_state;
            if duration > 100 {
                *counts.entry(duration).or_default() += 1;
            }
        }
        eprintln!("{model:?}: SAVE signal duration/count {counts:?}");
        assert!(edges.len() > 500);
        let decoded = eightbit_zx81_emulator::recording::decode(model, &edges).unwrap();
        assert!(decoded.image.programme().is_empty());
        let wav = eightbit_zx81_emulator::recording::wave(&edges).unwrap();
        assert!(eightbit_zx81_emulator::wave::Wave::parse(&wav).is_ok());
    }
}
#[test]
#[ignore = "Requires explicit lawfully supplied ZX80 and ZX81 ROM paths"]
fn generated_original_media_loads_through_reference_rom_ear_routines() {
    use eightbit_zx81_emulator::image::Image;
    for model in [
        Model::Zx80_1k,
        Model::Zx80_16k,
        Model::Zx81_1k,
        Model::Zx81_16k,
    ] {
        let programme: Vec<u8> = if model.zx80() {
            vec![0, 10, 240, 0, 38, 227, 35, 118] // LET A=7, original integer characters.
        } else {
            vec![0, 10, 12, 0, 241, 0, 38, 20, 35, 126, 131, 96, 0, 0, 0, 118]
        };
        let image = Image::from_program(model, &programme).unwrap();
        let mut machine = reference(model);
        frames(&mut machine, 200);
        machine.board.mount_image(image.bytes(), "STUDIO").unwrap();
        key(&mut machine, if model.zx80() { "w" } else { "j" }, false);
        if !model.zx80() {
            key(&mut machine, "p", true);
            key(&mut machine, "p", true);
        }
        key(&mut machine, "enter", false);
        machine.board.play_tape();
        frames(&mut machine, 1800);
        for (offset, expected) in programme.iter().enumerate() {
            let address = model.programme_start() + u16::try_from(offset).unwrap();
            assert_eq!(
                machine.board.peek(address),
                *expected,
                "{model:?}: cassette programme mismatch at {address:04x}"
            );
        }
        assert!(!machine.board.tape_pending());
    }
}
#[test]
#[ignore = "Requires explicit lawfully supplied ZX80 and ZX81 ROM paths"]
fn original_roms_boot_with_model_correct_ram_and_fetch_generated_video() {
    for model in [
        Model::Zx80_1k,
        Model::Zx80_16k,
        Model::Zx81_1k,
        Model::Zx81_16k,
    ] {
        let mut machine = reference(model);
        for _ in 0..200 {
            machine.run(FRAME_T_STATES).unwrap();
        }
        let pointer = if model.zx80() { 0x4008 } else { 0x400c };
        let word =
            u16::from_le_bytes([machine.board.peek(pointer), machine.board.peek(pointer + 1)]);
        assert!(
            (model.programme_start()..model.ram_top()).contains(&word),
            "{model:?} public programme/display pointer {word:04x}"
        );
        assert!(
            machine
                .board
                .frame()
                .chunks_exact(4)
                .any(|pixel| pixel == [0, 0, 0, 255]),
            "{model:?}: no refresh-generated cursor pixels"
        );
    }
}
