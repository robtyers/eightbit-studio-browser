//! Authored, actual CPU/PIA/Board regressions; no external firmware or host evaluation.
use super::*;

fn boot(profile: Firmware) -> Result<Machine, String> {
    let mut machine = Machine::with_firmware(profile);
    for _ in 0..8 {
        machine.advance(MAX_SLICE)?;
    }
    Ok(machine)
}

#[test]
fn bundled_joystick_public_entries_preserve_all_coordinate_bins_and_registers() -> Result<(), String>
{
    let mut machine = boot(Firmware::BasicDevelopment)?;
    assert!(machine.snapshot().text.contains("DEV>"));
    for indirect in [false, true] {
        let call: &[u8] = if indirect {
            &[0xad, 0x9f, 0xa0, 0x0a] // JSR [$A00A]
        } else {
            &[0xbd, 0x80, 0x12] // JSR $8012
        };
        machine.board.ram[0x6500..0x6500 + call.len()].copy_from_slice(call);
        let returned = 0x6500 + u16::try_from(call.len()).map_err(|_| "call length")?;
        for coordinate in 0_u8..64 {
            let axes = [
                coordinate,
                (coordinate + 17) & 63,
                (coordinate + 31) & 63,
                (coordinate + 47) & 63,
            ];
            for (index, value) in axes.into_iter().enumerate() {
                machine.joystick(index, value)?;
            }
            for buttons in 0_u8..4 {
                machine.joystick_button(0, buttons & 1 != 0)?;
                machine.joystick_button(1, buttons & 2 != 0)?;
                machine.cpu.registers.pc = 0x6500;
                machine.cpu.registers.s = 0x7000;
                machine.cpu.registers.y = 0x1234;
                machine.cpu.registers.u = 0x5678;
                machine.cpu.registers.cc = 0x50;
                for _ in 0..10_000 {
                    machine.step()?;
                    if machine.registers().pc == returned {
                        break;
                    }
                }
                assert_eq!(machine.registers().pc, returned, "bounded public call");
                assert_eq!(machine.read_memory(0x015a, 4)?, axes);
                assert_eq!(
                    (
                        machine.registers().s,
                        machine.registers().y,
                        machine.registers().u
                    ),
                    (0x7000, 0x1234, 0x5678)
                );
                assert_eq!(machine.board.pia[0].control[0] & 8, 0);
                assert_eq!(machine.board.pia[0].control[1] & 8, 0);
                assert_eq!(machine.board.read(0xff00) & 3, (!buttons) & 3);
            }
        }
    }
    Ok(())
}

#[test]
fn mounted_cartridge_executes_only_after_explicit_advance_and_guest_interrupt_enable()
-> Result<(), String> {
    let mut machine = boot(Firmware::BasicDevelopment)?;
    // Authored cartridge disables/acknowledges CB1, increments a RAM marker, RTI.
    let cartridge = vec![
        0x86, 0x34, 0xb7, 0xff, 0x23, 0xb6, 0xff, 0x22, 0x7c, 0x62, 0x00, 0x3b,
    ];
    machine.board.ram[0x010f..0x0112].copy_from_slice(&[0x7e, 0xc0, 0x00]);
    machine.board.ram[0x6500..0x6502].copy_from_slice(&[0x20, 0xfe]);
    machine.cpu.registers.pc = 0x6500;
    machine.cpu.registers.s = 0x7000;
    machine.cpu.registers.cc = 0x10; // IRQ masked, FIRQ enabled.
    machine.board.write(0xff23, 7); // Rising CB1 interrupt, no implicit clock.
    machine.board.read(0xff22);
    let before = machine.snapshot();
    machine.load_cartridge(cartridge)?;
    assert_eq!(machine.snapshot(), before);
    assert_eq!(machine.read_memory(0xc000, 2)?, [0x86, 0x34]);
    assert_eq!(machine.snapshot(), before, "mapped reads remain inert");
    assert_eq!(machine.peek(0x6200), 0);
    machine.advance(128)?;
    assert_eq!(
        machine.peek(0x6200),
        1,
        "actual guest FIRQ cartridge execution"
    );
    assert_eq!(
        machine.registers().s,
        0x7000,
        "actual interrupt stack restored"
    );
    assert_eq!(machine.registers().pc, 0x6500);
    machine.advance(128)?;
    assert_eq!(
        machine.peek(0x6200),
        1,
        "guest disabled subsequent requests"
    );
    // Ejection removes the source; already latched PIA status belongs to the guest.
    assert_ne!(machine.board.read(0xff23) & 0x80, 0);
    machine.eject_cartridge();
    assert_ne!(machine.board.read(0xff23) & 0x80, 0);
    machine.board.read(0xff22);
    machine.board.write(0xff23, 7);
    machine.advance(128)?;
    assert!(!machine.board.interrupts().firq);
    assert_eq!(machine.peek(0x6200), 1);
    assert_eq!(machine.peek(0xc000), 0xff);
    Ok(())
}

#[test]
fn current_firmware_reset_preserves_mounted_bytes_without_implicit_cartridge_launch()
-> Result<(), String> {
    for profile in [Firmware::Monitor, Firmware::BasicDevelopment] {
        let mut machine = boot(profile)?;
        // If entered this fixture would visibly write a marker and loop forever.
        machine.load_cartridge(vec![0x86, 0x55, 0xb7, 0x62, 0x00, 0x20, 0xfe])?;
        machine.joystick(0, 63)?;
        machine.joystick_button(1, true)?;
        let before = machine.snapshot();
        assert_eq!(machine.snapshot(), before);
        for _ in 0..8 {
            machine.advance(MAX_SLICE)?;
        }
        assert_eq!(machine.peek(0x6200), 0);
        assert!(
            !machine.board.interrupts().firq,
            "current guest does not enable CART"
        );
        machine.reset();
        assert_eq!(
            machine.peek(0xc000),
            0x86,
            "reset retains explicitly mounted media"
        );
        assert_eq!(machine.board.io.axes[0], 63);
        assert!(machine.board.io.buttons[1]);
        assert!(
            !machine.board.interrupts().firq,
            "reset disables PIA interrupt enable"
        );
        assert_eq!(
            machine.peek(0x6200),
            0,
            "CPU reset performs no guest instructions"
        );
        for _ in 0..8 {
            machine.advance(MAX_SLICE)?;
        }
        assert_eq!(machine.peek(0x6200), 0);
        let prompt = if profile == Firmware::Monitor {
            "EIGHTBIT DRAGON MONITOR"
        } else {
            "DEV>"
        };
        assert!(machine.snapshot().text.contains(prompt));
        machine.eject_cartridge();
        machine.reset();
        assert_eq!(machine.peek(0xc000), 0xff);
        assert!(!machine.board.interrupts().firq);
    }
    Ok(())
}

#[test]
fn cartridge_size_boundaries_preserve_firmware_vectors_and_atomic_replacement() -> Result<(), String>
{
    let mut machine = Machine::new();
    let vectors = machine.read_memory(0xffe0, 32)?;
    machine.load_cartridge(vec![0x12])?;
    assert_eq!(machine.peek(0xc000), 0x12);
    assert_eq!(machine.peek(0xc001), 0xff);
    let before = machine.snapshot();
    let mut cartridge = vec![0x55; 16_384];
    cartridge[0x3eff] = 0x66;
    machine.load_cartridge(cartridge)?;
    assert_eq!(machine.snapshot(), before);
    assert_eq!(machine.peek(0xc000), 0x55);
    assert_eq!(machine.peek(0xfeff), 0x66);
    assert_eq!(
        machine.read_memory(0xffe0, 32)?,
        vectors,
        "fixed vectors remain firmware-owned"
    );
    for invalid in [Vec::new(), vec![0; 16_385]] {
        let error = machine
            .load_cartridge(invalid)
            .err()
            .ok_or("invalid cartridge was accepted")?;
        assert!(error.starts_with("DME1007:"));
        assert_eq!(machine.peek(0xc000), 0x55);
        assert_eq!(machine.peek(0xfeff), 0x66);
        assert_eq!(machine.snapshot(), before);
    }
    Ok(())
}
