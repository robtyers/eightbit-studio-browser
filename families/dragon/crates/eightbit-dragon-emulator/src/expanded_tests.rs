//! Real Board coverage for the separately identified unbanked Studio ROM.
use super::*;

#[test]
fn expanded_rom_preserves_io_vectors_ram_size_and_sam_aliases() {
    let mut machine = Machine::with_firmware(Firmware::BasicExtended);
    assert_eq!(machine.board.ram.len(), 32_768);
    assert_eq!(machine.registers().pc, 0xb3b4);
    for address in 0x8000..=0xfeff {
        let expected = BASIC_EXTENDED[usize::from(address - 0x8000)];
        assert_eq!(machine.peek(address), expected, "ROM address {address:04x}");
        machine.board.write(address, !expected);
        assert_eq!(machine.peek(address), expected, "ROM is read-only");
    }
    for address in 0xffe0..=0xffff {
        assert_eq!(
            machine.peek(address),
            machine.peek(address - 0x4000),
            "legacy vector alias"
        );
    }
    machine.board.write(0xff01, 0);
    machine.board.write(0xff00, 0xa5);
    assert_eq!(machine.board.read(0xff00), 0xa5, "PIA DDR remains mapped");
    assert_eq!(machine.board.read(0xff04), 0xa5, "PIA register alias");
    for address in 0xff40..=0xffbf {
        machine.board.write(address, 0x55);
        assert_eq!(
            machine.board.read(address),
            255,
            "I/O hole remains open bus"
        );
    }
    machine.board.write(0xffda, 0);
    machine.board.write(0xffdd, 0);
    machine.board.write(0xffdf, 0);
    assert!(machine.board.sam.all_ram());
    for address in [0x8000, 0x84d6, 0xbfff, 0xc000, 0xfeff] {
        let offset = address & 0x7fff;
        machine.board.write(offset, 0x57);
        assert_eq!(machine.peek(address), 0x57, "existing 32 KiB RAM alias");
        let expected = BASIC_EXTENDED[usize::from(address - 0x8000)];
        machine.board.write(address, !expected);
        assert_eq!(
            machine.peek(offset),
            expected,
            "upper write retains ROM bus value"
        );
    }
    assert_eq!(machine.peek(0xffff), BASIC_EXTENDED[0x3fff]);
    machine.board.write(0xffde, 0);
    assert!(!machine.board.sam.all_ram());
    assert_eq!(machine.peek(0xc000), BASIC_EXTENDED[0x4000]);
}

#[test]
fn expanded_cartridge_rejection_is_atomic_and_keeps_physical_profiles() -> Result<(), String> {
    let mut machine = Machine::with_firmware(Firmware::BasicExtended);
    machine.type_text("PRINT 1")?;
    let before = machine.snapshot();
    let queued = machine.board.typed.clone();
    let ram = machine.board.ram.clone();
    for length in [0, 1, 16_384, 16_385] {
        let error = machine.load_cartridge(vec![0x39; length]);
        assert!(error.is_err_and(|message| message.contains("cartridges require XRoar")));
        assert_eq!(machine.snapshot(), before);
        assert_eq!(machine.board.typed, queued);
        assert_eq!(machine.board.ram, ram);
        assert!(machine.board.cartridge.is_none());
    }
    for profile in [Firmware::Monitor, Firmware::BasicDevelopment] {
        let mut physical = Machine::with_firmware(profile);
        physical.load_cartridge(vec![0x39])?;
        assert_eq!(physical.peek(0xc000), 0x39);
    }
    assert_eq!(
        Firmware::BasicDevelopment.id(),
        "eightbit-dragon32-basic-0.1-dev"
    );
    assert_eq!(
        Firmware::BasicExtended.id(),
        "eightbit-dragon32-basic-extended-1"
    );
    Ok(())
}

#[test]
fn expanded_guest_executes_high_rom_and_formats_physical_keyboard_input() -> Result<(), String> {
    let mut machine = Machine::with_firmware(Firmware::BasicExtended);
    machine.step()?;
    assert!((0xc000..0xff00).contains(&machine.registers().pc));
    machine.step()?;
    for _ in 0..8 {
        machine.advance(MAX_SLICE)?;
    }
    machine.type_text("PRINT USING\"###\";42:PRINT 2*3*4")?;
    for _ in 0..128 {
        machine.advance(MAX_SLICE)?;
        let text = machine.snapshot().text;
        if text.lines().any(|line| line.trim() == "42")
            && text.lines().any(|line| line.trim() == "24")
        {
            return Ok(());
        }
    }
    Err(format!(
        "Expanded guest did not display the physical input's formatted results: {:?}",
        machine.snapshot().text
    ))
}

#[test]
fn expanded_guest_boots_and_default_cart_interrupt_returns_safely() -> Result<(), String> {
    let mut machine = Machine::with_firmware(Firmware::BasicExtended);
    for _ in 0..8 {
        machine.advance(MAX_SLICE)?;
    }
    assert!(machine.snapshot().text.contains("OK>"), "guest prompt");
    assert_eq!(machine.peek(0x010f), 0x7e, "public RAM FIRQ hook retained");
    // An authored RAM loop lets the actual CPU take and return from FIRQ. No
    // cartridge is mounted; a latched PIA CART request must not enter $C000.
    machine.board.ram[0x6500..0x6502].copy_from_slice(&[0x20, 0xfe]);
    machine.cpu.registers.pc = 0x6500;
    machine.cpu.registers.s = 0x7000;
    machine.cpu.registers.cc = 0x10;
    machine.board.pia[1].control[1] = 0x87;
    assert!(machine.board.interrupts().firq);
    for _ in 0..32 {
        machine.step()?;
        assert_ne!(machine.registers().pc, 0xc000, "no cartridge auto-run");
    }
    assert_eq!(machine.registers().pc, 0x6500);
    assert_eq!(machine.registers().s, 0x7000);
    assert!(!machine.board.interrupts().firq);
    Ok(())
}
