//! Original C64 no-cartridge memory/6510-port foundation, not a complete PAL board.
//!
//! Firmware is borrowed, never embedded or loaded from the filesystem. All bytes of
//! physical RAM remain addressable. Unimplemented devices fault explicitly.
use eightbit_mos6502_core::Bus;
pub mod cia;
pub mod input;
pub mod original;
pub mod pal;
pub mod sid;
pub mod vic;

/// Strict firmware-window validation or unimplemented-device access failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// BASIC/KERNAL/character windows must be exactly 8192/8192/4096 bytes.
    FirmwareSize,
    /// No device implementation has been attached at this bus address.
    UnsupportedIo(u16),
    /// Host initialisation request extends outside physical RAM; nothing was written.
    RamRange,
    /// Invalid physical key/joystick coordinates or switch bits.
    InputRange,
    /// Clock budgets must be 1–1,000,000.
    CycleBudget,
    /// DMA wait failed to yield within two complete raster lines.
    BusWait,
    /// Monotone board clock capacity was exhausted.
    ClockLimit,
    /// Stopped/faulted machines cannot execute without explicit resume/reset.
    MachineState,
    /// Malformed, excessive or unsupported original firmware container/map.
    OriginalFormat,
    /// Original complete image or its source/build/image/map identity was rejected.
    OriginalIdentity,
    /// Invalid, forged, stale or interrupted private gate continuation.
    OriginalGate,
    /// Private gate or guest stack budget would be exceeded.
    OriginalDepth,
}
impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::FirmwareSize => {
                f.write_str("DC64M01: firmware windows must be 8192/8192/4096 bytes")
            }
            Self::UnsupportedIo(address) => {
                write!(f, "DC64M02: device not implemented at ${address:04X}")
            }
            Self::RamRange => f.write_str("DC64M03: RAM initialisation exceeds 64 KiB"),
            Self::InputRange => f.write_str("DC64M04: invalid physical input coordinate or switch"),
            Self::CycleBudget => f.write_str("DC64M05: cycle budget must be 1–1000000"),
            Self::BusWait => f.write_str("DC64M06: DMA bus wait exceeded two raster lines"),
            Self::ClockLimit => f.write_str("DC64M07: board clock limit reached"),
            Self::MachineState => f.write_str("DC64M08: machine is stopped or faulted"),
            Self::OriginalFormat => {
                f.write_str("DC64M09: invalid original firmware container or map")
            }
            Self::OriginalIdentity => f.write_str("DC64M10: original firmware identity rejected"),
            Self::OriginalGate => {
                f.write_str("DC64M11: invalid original firmware gate continuation")
            }
            Self::OriginalDepth => {
                f.write_str("DC64M12: original firmware gate stack limit reached")
            }
        }
    }
}
impl std::error::Error for Error {}

/// Borrowed, size-checked windows. This type establishes size, not firmware identity.
/// Existing exact-hash legacy ROM validation must still precede any future user route.
#[derive(Clone, Copy)]
pub struct Firmware<'a> {
    basic: &'a [u8; 8192],
    kernal: &'a [u8; 8192],
    characters: &'a [u8; 4096],
    original: Option<original::Image<'a>>,
}
impl<'a> Firmware<'a> {
    /// Validate exact lengths without copying, storing or printing firmware bytes.
    pub fn new(basic: &'a [u8], kernal: &'a [u8], characters: &'a [u8]) -> Result<Self, Error> {
        Ok(Self {
            basic: basic.try_into().map_err(|_| Error::FirmwareSize)?,
            kernal: kernal.try_into().map_err(|_| Error::FirmwareSize)?,
            characters: characters.try_into().map_err(|_| Error::FirmwareSize)?,
            original: None,
        })
    }
    /// Use a separately validated original container; legacy windows never enable pages.
    pub fn original(image: original::Image<'a>) -> Result<Self, Error> {
        Ok(Self {
            basic: image.pages[..8192]
                .try_into()
                .map_err(|_| Error::OriginalFormat)?,
            kernal: image.kernal,
            characters: image.characters,
            original: Some(image),
        })
    }
}

/// Decoded address inside one device; mirroring is handled by the board bus.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Device {
    /// VIC-II register, 0..63.
    Vic(u8),
    /// SID register, 0..31.
    Sid(u8),
    /// CIA1 register, 0..15.
    Cia1(u8),
    /// CIA2 register, 0..15.
    Cia2(u8),
    /// Unattached expansion I/O, original full address.
    Expansion(u16),
}
/// Future family-owned devices receive explicit addresses and the previous bus latch.
/// They may fault, rather than fabricate a successful hardware operation.
pub trait Devices {
    /// Read a decoded I/O register. `address` retains the original CPU address.
    fn read(&mut self, device: Device, address: u16, bus_latch: u8) -> Result<u8, Error>;
    /// Write a decoded I/O register.
    fn write(&mut self, device: Device, address: u16, value: u8) -> Result<(), Error>;
}
/// Default stage-1 adapter: no video, audio, CIA or expansion hardware exists yet.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoDevices;
impl Devices for NoDevices {
    fn read(&mut self, _: Device, address: u16, _: u8) -> Result<u8, Error> {
        Err(Error::UnsupportedIo(address))
    }
    fn write(&mut self, _: Device, address: u16, _: u8) -> Result<(), Error> {
        Err(Error::UnsupportedIo(address))
    }
}

/// Six-pin 6510 port. Inputs are board-supplied logical levels, not an attached tape.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Port {
    direction: u8,
    latch: u8,
}
impl Port {
    /// Direction bits, with unimplemented pins 6/7 reading zero.
    #[must_use]
    pub const fn direction(self) -> u8 {
        self.direction
    }
    /// Output latch. Only six physical pins are represented.
    #[must_use]
    pub const fn latch(self) -> u8 {
        self.latch
    }
    /// Combine output drivers and externally supplied input levels.
    #[must_use]
    pub const fn pins(self, inputs: u8) -> u8 {
        ((self.latch & self.direction) | (inputs & !self.direction)) & 0x3f
    }
}

/// Pure C64 memory foundation. Reset/power initialisation is explicit and deterministic.
/// No physical power-up RAM pattern, floating-pin decay or VIC bus arbitration is claimed.
pub struct Memory<'a, T = NoDevices> {
    ram: Box<[u8]>,
    colour: [u8; 1024],
    firmware: Firmware<'a>,
    port: Port,
    inputs: u8,
    bus_latch: u8,
    devices: T,
    paging: Option<original::Paging<'a>>,
}
impl<'a> Memory<'a, NoDevices> {
    /// Allocate zeroed deterministic RAM; no I/O devices or execution are started.
    #[must_use]
    pub fn new(firmware: Firmware<'a>) -> Self {
        Self::with_devices(firmware, NoDevices)
    }
}
impl<'a, T: Devices> Memory<'a, T> {
    /// Compose an explicitly supplied family device adapter; no host I/O is performed.
    #[must_use]
    pub fn with_devices(firmware: Firmware<'a>, devices: T) -> Self {
        Self {
            ram: vec![0; 65536].into_boxed_slice(),
            colour: [0; 1024],
            firmware,
            port: Port::default(),
            inputs: 0x3f,
            bus_latch: 0,
            devices,
            paging: firmware.original.map(original::Paging::new),
        }
    }
    /// Inspect the processor-port drivers without a CPU access.
    #[must_use]
    pub const fn port(&self) -> Port {
        self.port
    }
    /// Set external port inputs. Pins 0..2 normally have pulled-high banking inputs.
    pub fn set_port_inputs(&mut self, inputs: u8) {
        self.inputs = inputs & 0x3f;
    }
    /// Reset port direction and bus latch, preserving physical RAM, colour and devices.
    /// The caller must reset its device adapter separately when a full board exists.
    pub fn reset_port(&mut self) {
        self.port = Port::default();
        self.bus_latch = 0;
        if let Some(paging) = &mut self.paging {
            paging.revoke();
        }
    }
    /// Current original page/depth; legacy firmware has no private page authority.
    #[must_use]
    pub fn original_page(&self) -> Option<(u8, usize)> {
        self.paging.as_ref().map(|p| (p.page(), p.depth()))
    }
    /// Read physical RAM without banking, side effects or bus-latch changes.
    #[must_use]
    pub fn ram(&self, address: u16) -> u8 {
        self.ram[usize::from(address)]
    }
    /// Explicit host/test initialisation, not a guest PRG loader. Checked before writing.
    pub fn initialise_ram(&mut self, start: u16, bytes: &[u8]) -> Result<(), Error> {
        let start = usize::from(start);
        let end = start
            .checked_add(bytes.len())
            .filter(|end| *end <= 65536)
            .ok_or(Error::RamRange)?;
        self.ram[start..end].copy_from_slice(bytes);
        Ok(())
    }
    /// Inspect colour RAM's four stored bits; index wraps to the 1 KiB physical chip.
    #[must_use]
    pub fn colour(&self, index: u16) -> u8 {
        self.colour[usize::from(index & 0x3ff)]
    }
    /// VIC-visible byte for an explicitly selected 16 KiB bank (low two bits).
    /// CIA2 bank-pin driving and VIC fetch timing belong to stage 2.
    #[must_use]
    pub fn vic_read(&self, bank: u8, offset: u16) -> u8 {
        let address = (u16::from(bank & 3) << 14) | (offset & 0x3fff);
        if address & 0x7000 == 0x1000 {
            self.firmware.characters[usize::from(address & 0xfff)]
        } else {
            self.ram[usize::from(address)]
        }
    }
    fn decode(address: u16) -> Device {
        let low = address.to_le_bytes()[0];
        match address {
            0xd000..=0xd3ff => Device::Vic(low & 63),
            0xd400..=0xd7ff => Device::Sid(low & 31),
            0xdc00..=0xdcff => Device::Cia1(low & 15),
            0xdd00..=0xddff => Device::Cia2(low & 15),
            _ => Device::Expansion(address),
        }
    }
}
impl<T: Devices> Bus for Memory<'_, T> {
    type Error = Error;
    fn read(&mut self, address: u16) -> Result<u8, Error> {
        let pins = self.port.pins(self.inputs);
        let value = match address {
            0 => self.port.direction,
            1 => pins,
            0xa000..=0xbfff if pins & 3 == 3 => self.paging.as_ref().map_or_else(
                || self.firmware.basic[usize::from(address - 0xa000)],
                |p| p.read(address),
            ),
            0xe000..=0xffff if pins & 2 != 0 => self.firmware.kernal[usize::from(address - 0xe000)],
            0xd000..=0xdfff if pins & 3 != 0 => {
                if pins & 4 == 0 {
                    self.firmware.characters[usize::from(address & 0xfff)]
                } else if (0xd800..=0xdbff).contains(&address) {
                    (self.bus_latch & 0xf0) | self.colour[usize::from(address & 0x3ff)]
                } else {
                    self.devices
                        .read(Self::decode(address), address, self.bus_latch)?
                }
            }
            _ => self.ram[usize::from(address)],
        };
        self.bus_latch = value;
        Ok(value)
    }
    fn write(&mut self, address: u16, value: u8) -> Result<(), Error> {
        let pins = self.port.pins(self.inputs);
        match address {
            // Internal-port writes do not claim the VIC-driven RAM-under-port quirk.
            0 => self.port.direction = value & 0x3f,
            1 => self.port.latch = value & 0x3f,
            0xd000..=0xdfff if pins & 3 != 0 && pins & 4 != 0 => {
                if (0xd800..=0xdbff).contains(&address) {
                    self.colour[usize::from(address & 0x3ff)] = value & 15;
                } else {
                    let handled = if (0xde00..=0xde02).contains(&address) {
                        if let Some(paging) = &mut self.paging {
                            paging.control(address, value, pins)?
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    if !handled {
                        self.devices.write(Self::decode(address), address, value)?;
                    }
                }
            }
            _ => self.ram[usize::from(address)] = value,
        }
        self.bus_latch = value;
        Ok(())
    }
    fn execution(&mut self, event: eightbit_mos6502_core::Execution) -> Result<(), Error> {
        let pins = self.port.pins(self.inputs);
        if let Some(paging) = &mut self.paging {
            let stack = if let eightbit_mos6502_core::Execution::Complete { registers, .. } = event
            {
                [1u8, 2, 3]
                    .map(|offset| self.ram[0x100 | usize::from(registers.sp.wrapping_add(offset))])
            } else {
                [0; 3]
            };
            paging.event(event, pins, stack)?;
        }
        Ok(())
    }
}
