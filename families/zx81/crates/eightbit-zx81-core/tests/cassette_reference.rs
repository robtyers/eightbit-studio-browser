//! Opt-in checks of original exports using opaque, exact-hash reference firmware.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_zx81_core::zx80;
use eightbit_zx81_emulator::{FRAME_T_STATES, Firmware, Key, Machine, Model};
use sha2::{Digest, Sha256};
fn machine(model: Model) -> Machine {
    let path = std::env::var("EIGHTBIT_ZX80_ROM").expect("Set an explicit local ZX80 ROM input");
    let metadata = std::fs::symlink_metadata(&path).unwrap();
    assert!(metadata.is_file());
    assert_eq!(metadata.len(), 4096);
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        "9374711cb6a5fd53c1c98d1d20ecb444d01d621a322f3d066ef216d515ac5161"
    );
    Machine::new(model, Firmware::reference(&bytes, model).unwrap())
}
fn frames(machine: &mut Machine, count: usize) {
    for _ in 0..count {
        machine.run(FRAME_T_STATES).unwrap();
    }
}
fn key(machine: &mut Machine, name: &str) {
    machine.board.set_key(Key::named(name).unwrap(), true);
    frames(machine, 4);
    machine.board.release_keys();
    frames(machine, 4);
}
#[test]
#[ignore = "Requires an explicit lawfully supplied ZX80 reference ROM"]
fn every_byte_and_count_adjustment_load_decode_and_enter_original_machine_code() {
    for model in [Model::Zx80_1k, Model::Zx80_16k] {
        for size in [118, 256] {
            let code: Vec<u8> = (0..size).map(|n| u8::try_from(n % 256).unwrap()).collect();
            let image = zx80::package_assembly(model, &code).unwrap();
            let mut m = machine(model);
            frames(&mut m, 200);
            m.board.mount_image(&image, "").unwrap();
            key(&mut m, "w");
            key(&mut m, "enter");
            m.board.play_tape();
            frames(&mut m, 3000);
            assert!(!m.board.tape_pending());
            key(&mut m, "r");
            m.board.set_key(Key::named("enter").unwrap(), true);
            let end = m.board.clocks() + 6_500_000;
            while m.cpu.registers.pc != zx80::ASSEMBLY_ORIGIN && m.board.clocks() < end {
                m.cpu.step(&mut m.board).unwrap();
            }
            assert_eq!(
                m.cpu.registers.pc,
                zx80::ASSEMBLY_ORIGIN,
                "{model:?} size {size}: did not enter decoded machine code"
            );
            for (offset, byte) in code.iter().enumerate() {
                assert_eq!(
                    m.board
                        .peek(zx80::ASSEMBLY_ORIGIN + u16::try_from(offset).unwrap()),
                    *byte,
                    "{model:?} size {size} offset {offset}"
                );
            }
        }
    }
}
