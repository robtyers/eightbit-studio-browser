//! Original generated ROM-shaped bytes only; no historical firmware or binary fixtures.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_commodore_machine::{Device, Devices, Error, Firmware, Memory};
use eightbit_mos6502_core::{Bus, Cpu, Error as CpuError, Registers, StopReason};

const BASIC: [u8; 8192] = [0xba; 8192];
const KERNAL: [u8; 8192] = [0xe1; 8192];
const CHARACTERS: [u8; 4096] = [0xc4; 4096];
fn firmware() -> Firmware<'static> {
    Firmware::new(&BASIC, &KERNAL, &CHARACTERS).unwrap()
}
fn bank<T: Devices>(memory: &mut Memory<'_, T>, value: u8) {
    memory.write(0, 7).unwrap();
    memory.write(1, value).unwrap();
}

#[test]
fn exact_firmware_window_lengths_are_checked_individually() {
    for length in [0, 1, 8191, 8193, 65536] {
        let data = vec![0; length];
        assert!(matches!(
            Firmware::new(&data, &KERNAL, &CHARACTERS),
            Err(Error::FirmwareSize)
        ));
        assert!(matches!(
            Firmware::new(&BASIC, &data, &CHARACTERS),
            Err(Error::FirmwareSize)
        ));
    }
    for length in [0, 1, 4095, 4097, 65536] {
        let data = vec![0; length];
        assert!(matches!(
            Firmware::new(&BASIC, &KERNAL, &data),
            Err(Error::FirmwareSize)
        ));
    }
}

#[test]
fn all_eight_no_cartridge_pla_maps_and_ram_under_every_rom() {
    // Independent truth rows: pins, BASIC visible, KERNAL visible, D window (r/c/i).
    for (pins, basic, kernal, d) in [
        (0, false, false, 'r'),
        (1, false, false, 'c'),
        (2, false, true, 'c'),
        (3, true, true, 'c'),
        (4, false, false, 'r'),
        (5, false, false, 'i'),
        (6, false, true, 'i'),
        (7, true, true, 'i'),
    ] {
        let mut m = Memory::new(firmware());
        bank(&mut m, pins);
        for (address, value) in [
            (0xa000, 0x11),
            (0xbfff, 0x12),
            (0xe000, 0x21),
            (0xffff, 0x22),
        ] {
            m.write(address, value).unwrap();
            assert_eq!(m.ram(address), value);
        }
        m.initialise_ram(0xd000, &[0x31]).unwrap();
        m.initialise_ram(0xdfff, &[0x32]).unwrap();
        assert_eq!(m.read(0xa000).unwrap(), if basic { 0xba } else { 0x11 });
        assert_eq!(m.read(0xbfff).unwrap(), if basic { 0xba } else { 0x12 });
        assert_eq!(m.read(0xe000).unwrap(), if kernal { 0xe1 } else { 0x21 });
        assert_eq!(m.read(0xffff).unwrap(), if kernal { 0xe1 } else { 0x22 });
        for (address, ram) in [(0xd000, 0x31), (0xdfff, 0x32)] {
            if d == 'i' {
                assert_eq!(m.read(address), Err(Error::UnsupportedIo(address)));
                assert_eq!(m.write(address, 0x44), Err(Error::UnsupportedIo(address)));
            } else {
                assert_eq!(m.read(address).unwrap(), if d == 'c' { 0xc4 } else { ram });
                m.write(address, 0x44).unwrap();
                assert_eq!(m.ram(address), 0x44);
            }
        }
        bank(&mut m, 0);
        assert_eq!(m.read(0xa000), Ok(0x11));
        assert_eq!(m.read(0xffff), Ok(0x22));
    }
}

#[test]
fn direction_output_latches_inputs_and_reset_preserve_all_ram() {
    let mut m = Memory::new(firmware());
    assert_eq!(m.read(0), Ok(0));
    assert_eq!(m.read(1), Ok(0x3f));
    m.initialise_ram(0, &[0xaa, 0xbb]).unwrap();
    m.write(1, 0).unwrap();
    assert_eq!(m.read(1), Ok(0x3f)); // Input pins ignore output latch.
    m.write(0, 7).unwrap();
    assert_eq!(m.read(1), Ok(0x38));
    assert_eq!(m.read(0xa000), Ok(0));
    m.set_port_inputs(0x10);
    assert_eq!(m.read(1), Ok(0x10));
    m.write(0, 0xff).unwrap();
    m.write(1, 0xff).unwrap();
    assert_eq!(m.port().latch(), 0x3f);
    assert_eq!(m.read(0), Ok(0x3f));
    assert_eq!(m.read(1), Ok(0x3f));
    m.reset_port();
    assert_eq!(m.port().direction(), 0);
    assert_eq!(m.read(1), Ok(0x10));
    assert_eq!((m.ram(0), m.ram(1)), (0xaa, 0xbb));
    m.set_port_inputs(0x3f);
    assert_eq!(m.read(0xa000), Ok(0xba));
}

#[test]
fn physical_ram_initialisation_is_atomic_bounded_and_keeps_all_64k() {
    let mut m = Memory::new(firmware());
    let bytes: Vec<_> = (0..65536u32)
        .map(|n| n.to_le_bytes()[0] ^ n.to_le_bytes()[1])
        .collect();
    m.initialise_ram(0, &bytes).unwrap();
    for address in 0..=65535u16 {
        assert_eq!(m.ram(address), bytes[usize::from(address)]);
    }
    let last = m.ram(0xffff);
    assert_eq!(m.initialise_ram(0xffff, &[3, 4]), Err(Error::RamRange));
    assert_eq!(m.ram(0xffff), last);
    m.initialise_ram(0xffff, &[]).unwrap();
    m.initialise_ram(0xffff, &[0x88]).unwrap();
    assert_eq!(m.ram(0xffff), 0x88);
}

#[test]
fn colour_ram_uses_only_nibbles_and_is_hidden_when_banked_out() {
    let mut m = Memory::new(firmware());
    bank(&mut m, 7);
    m.initialise_ram(0xd800, &[0x12]).unwrap();
    m.write(0xd800, 0xab).unwrap();
    assert_eq!(m.colour(0), 0xb);
    m.write(0x0200, 0x50).unwrap();
    assert_eq!(m.read(0xd800), Ok(0x5b));
    m.write(0xdbff, 0xfe).unwrap();
    assert_eq!(m.colour(1023), 14);
    assert_eq!(m.ram(0xd800), 0x12);
    bank(&mut m, 3);
    assert_eq!(m.read(0xd800), Ok(0xc4));
    m.write(0xd800, 0x23).unwrap();
    assert_eq!(m.colour(0), 0xb);
    bank(&mut m, 0);
    assert_eq!(m.read(0xd800), Ok(0x23));
}

#[test]
fn vic_view_is_independent_of_cpu_banking_and_port_registers() {
    let mut m = Memory::new(firmware());
    m.initialise_ram(0, &[0x61, 0x62]).unwrap();
    for address in [0x1000, 0x5000, 0x9000, 0xd000] {
        m.initialise_ram(address, &[address.to_be_bytes()[0]])
            .unwrap();
    }
    for pins in 0..8 {
        bank(&mut m, pins);
        assert_eq!(m.vic_read(0, 0), 0x61);
        assert_eq!(m.vic_read(0, 1), 0x62);
        assert_eq!(m.vic_read(0, 0x1000), 0xc4);
        assert_eq!(m.vic_read(1, 0x1000), 0x50);
        assert_eq!(m.vic_read(2, 0x1000), 0xc4);
        assert_eq!(m.vic_read(3, 0x1000), 0xd0);
    }
    assert_eq!(m.vic_read(7, 0x5000), 0xd0);
}

#[derive(Default)]
struct Probe;
impl Devices for Probe {
    fn read(&mut self, device: Device, address: u16, latch: u8) -> Result<u8, Error> {
        match device {
            Device::Vic(r) => {
                assert!((0xd000..=0xd3ff).contains(&address));
                Ok(r)
            }
            Device::Sid(r) => Ok(0x40 | r),
            Device::Cia1(r) => Ok(0x80 | r),
            Device::Cia2(r) => Ok(0xc0 | r),
            Device::Expansion(_) => Ok(latch),
        }
    }
    fn write(&mut self, device: Device, address: u16, value: u8) -> Result<(), Error> {
        assert_eq!(device, Device::Sid(0));
        assert_eq!(address, 0xd420);
        assert_eq!(value, 0x55);
        Ok(())
    }
}
#[test]
fn device_ranges_are_mirrored_before_adapter_and_expansion_is_explicit() {
    let mut m = Memory::with_devices(firmware(), Probe);
    bank(&mut m, 7);
    for (address, value) in [
        (0xd000, 0),
        (0xd03f, 63),
        (0xd040, 0),
        (0xd3ff, 63),
        (0xd400, 0x40),
        (0xd7ff, 0x5f),
        (0xdc00, 0x80),
        (0xdcff, 0x8f),
        (0xdd00, 0xc0),
        (0xddff, 0xcf),
    ] {
        assert_eq!(m.read(address), Ok(value));
    }
    m.write(0xd420, 0x55).unwrap();
    assert_eq!(m.ram(0xd420), 0);
    assert_eq!(m.read(0xde00), Ok(0x55));
    assert_eq!(m.read(0xdfff), Ok(0x55));
}

#[test]
fn original_cpu_executes_banking_programme_and_faults_on_missing_hardware() {
    let mut m = Memory::new(firmware());
    // Original guest: set banking pins as outputs, hide ROM, store/read RAM, loop.
    let programme = [
        0xa9, 7, 0x85, 0, 0xa9, 0, 0x85, 1, 0xa9, 0x42, 0x8d, 0, 0xa0, 0xad, 0, 0xa0, 0x4c, 0x10, 2,
    ];
    m.initialise_ram(0x200, &programme).unwrap();
    let mut cpu = Cpu::from_registers(Registers {
        pc: 0x200,
        ..Registers::default()
    });
    let run = cpu.run(&mut m, 8, || false).unwrap();
    assert_eq!(run.reason, StopReason::Budget);
    assert_eq!(run.cycles, 23);
    assert_eq!(cpu.registers().a, 0x42);
    assert_eq!(m.ram(0xa000), 0x42);
    m.initialise_ram(0x200, &[0xad, 0, 0xd0]).unwrap();
    bank(&mut m, 7);
    let mut cpu = Cpu::from_registers(Registers {
        pc: 0x200,
        ..Registers::default()
    });
    assert_eq!(
        cpu.step(&mut m),
        Err(CpuError::Bus(Error::UnsupportedIo(0xd000)))
    );
    assert!(cpu.is_faulted());
}
