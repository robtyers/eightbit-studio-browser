//! Original bus vectors, independent of any third-party ROM or emulator.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_z80_core::{Bus, Cpu};
use eightbit_zx81_emulator::{Board, Firmware, Key, LINE_T_STATES, Model};

fn board(model: Model) -> Board {
    let mut rom = vec![0; 16384];
    rom[0x0e00] = 0x80;
    rom[0x1e00] = 0x40;
    rom[0x2000] = 0x42;
    Board::new(model, Firmware::original(&rom).unwrap())
}
#[test]
fn model_ram_and_rom_aliases_are_physical_and_rom_is_read_only() {
    for model in [
        Model::Zx80_1k,
        Model::Zx80_16k,
        Model::Zx81_1k,
        Model::Zx81_16k,
    ] {
        let mut b = board(model);
        b.write(0x4000, 0x29);
        assert_eq!(b.peek(0xc000), 0x29);
        b.write(0x4400, 0x39);
        assert_eq!(
            b.peek(0x4000),
            if model.ram_bytes() == 1024 {
                0x39
            } else {
                0x29
            }
        );
        b.write(0x0e00, 0);
        assert_eq!(b.peek(0x0e00), 0x80);
        assert_eq!(b.peek(0x2e00), 0x80);
        assert_eq!(b.peek(0x1e00), if model.zx80() { 0x80 } else { 0x40 });
    }
}
#[test]
fn private_ports_require_original_firmware_gate_and_cannot_extend_ram() {
    let mut b = board(Model::Zx81_1k);
    b.fetch(0x4000);
    b.output(0xfdfd, 1);
    assert_eq!(b.firmware_page(), 0);
    b.fetch(0x1e0);
    assert_eq!(b.input(0xfdf7), 1);
    assert_eq!(b.input(0xfdf9), 0);
    b.output(0xfdfd, 1);
    assert_eq!(b.peek(0x2000), 0x42);
    b.fetch(0x4000);
    b.output(0xfdfd, 0);
    assert_eq!(b.firmware_page(), 1);
    b.fetch(0x1e0);
    b.output(0xfdfd, 0);
    assert_eq!(b.peek(0x2000), 0);
    let mut reference = Board::new(
        Model::Zx81_1k,
        Firmware::reference(&[0; 8192], Model::Zx81_1k).unwrap(),
    );
    reference.fetch(0x1e0);
    reference.output(0xfdfd, 1);
    assert_eq!(reference.firmware_page(), 0);
    assert_eq!(reference.input(0xfdf7), 255);
}
#[test]
fn matrix_rows_compose_active_low_and_unknown_keys_are_rejected() {
    let mut b = board(Model::Zx81_16k);
    b.set_key(Key::named("SHIFT").unwrap(), true);
    b.set_key(Key::named("a").unwrap(), true);
    assert_eq!(b.input(0xfefe) & 31, 30);
    assert_eq!(b.input(0xfdfe) & 31, 30);
    assert_eq!(b.input(0xfffe) & 31, 31);
    b.release_keys();
    assert_eq!(b.input(0x00fe) & 31, 31);
    assert!(Key::named("symbol-shift").is_err());
    assert!(Key::named(&"x".repeat(1000)).is_err());
}
#[test]
fn video_comes_from_m1_refresh_and_supports_inverse_and_ram_fonts() {
    let mut b = board(Model::Zx81_16k);
    b.write(0x4000, 0);
    b.write(0x4001, 128);
    assert_eq!(b.fetch(0xc000), 0);
    b.refresh(0x1e40);
    assert_eq!(
        &b.frame()[(32 * 320 + 32) * 4..(32 * 320 + 34) * 4],
        &[255, 255, 255, 255, 0, 0, 0, 255]
    );
    assert_eq!(b.fetch(0xc001), 0);
    b.refresh(0x1e41);
    assert_eq!(
        &b.frame()[(32 * 320 + 40) * 4..(32 * 320 + 42) * 4],
        &[0, 0, 0, 255, 255, 255, 255, 255]
    );
    b.write(0x4002, 0x76);
    assert_eq!(b.fetch(0xc002), 0x76); // HALT is executed, not rendered.
    b.tick(LINE_T_STATES);
    b.write(0x4201, 0x80);
    b.fetch(0xc000);
    b.refresh(0x4242);
    assert_eq!(
        &b.frame()[(33 * 320 + 32) * 4..(33 * 320 + 33) * 4],
        &[0, 0, 0, 255]
    );
    // An ordinary data read of the display file never draws.
    let frame = b.frame().to_vec();
    b.read(0xc001);
    b.refresh(0x1e43);
    assert_eq!(b.frame(), frame);
}
#[test]
fn refresh_a6_drives_irq_and_only_zx81_has_a_switchable_nmi_generator() {
    for model in [Model::Zx80_1k, Model::Zx81_1k] {
        let mut b = board(model);
        b.refresh(0x1e40);
        assert!(!b.irq());
        b.refresh(0x1e00);
        assert!(b.irq());
        b.output(0xfe, 0);
        b.tick(LINE_T_STATES);
        assert_eq!(b.nmi(), !model.zx80());
        b.tick(1);
        assert!(!b.nmi());
        b.output(0xfd, 0);
        b.tick(LINE_T_STATES - 1);
        assert!(!b.nmi());
    }
}
#[test]
fn display_fetch_replacement_is_observed_by_the_cpu() {
    let mut b = board(Model::Zx81_16k);
    let mut cpu = Cpu::default();
    cpu.registers.pc = 0xc000;
    cpu.registers.i = 0x1e;
    b.write(0x4000, 0x26); // A glyph, not LD H,n.
    b.write(0x4001, 0x76);
    assert_eq!(cpu.step(&mut b).unwrap(), 4);
    assert_eq!(cpu.registers.pc, 0xc001);
    assert_eq!(cpu.registers.h, 0);
    assert_eq!(cpu.step(&mut b).unwrap(), 4);
    assert!(cpu.halted);
}
#[test]
fn bounded_slices_firmware_and_cassette_capture_fail_without_panicking() {
    assert!(Firmware::original(&[]).is_err());
    assert!(Firmware::original(&[0; 8193]).is_err());
    assert!(Firmware::reference(&[0; 8192], Model::Zx80_1k).is_err());
    let mut b = board(Model::Zx81_16k);
    b.record_tape(true);
    b.input(0xfe);
    b.tick(20);
    b.output(0xff, 0);
    let edges = b.drain_tape_edges().unwrap();
    assert_eq!(edges.len(), 2);
    assert_eq!(edges[1].t_state - edges[0].t_state, 20);
}
