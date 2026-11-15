//! Original bounded Dragon 32 digital machine. No host I/O or third-party firmware.
#![allow(missing_docs)]

mod acia;
pub mod basic_debug;
pub mod basic_debug_protocol;
mod cassette;
pub mod debug;
pub mod development_host;
pub mod dragon64;
#[cfg(test)]
mod dragon64_tests;
#[cfg(test)]
mod expanded_tests;
mod glyphs;
#[cfg(test)]
mod hardware_tests;
mod io;
mod launch;
pub mod machine_debug;
mod machine_loader;
#[cfg(test)]
mod original_machine_tests;
pub mod original_runtime;
mod pia;
mod recording;
mod sam;
mod session;
mod video;
pub use cassette::MAX_SESSION_CASSETTE_BYTES;
use eightbit_m6809_core::{Bus, Cpu, InterruptLines, Unsupported};
use io::Io;
pub use io::{AUDIO_HZ, MASTER_HZ, TapePulse};
pub use launch::{OriginalBackend, OriginalCassette, OriginalDevelopmentLaunch, OriginalTarget};
use pia::Pia;
pub use recording::RecordingExport;
use sam::Sam;
use serde::{Deserialize, Serialize};
pub use session::MAX_MONITOR_REQUEST_BYTES;
pub use session::{MonitorCommand, MonitorRequest, MonitorSession, SessionSnapshot};
use std::collections::VecDeque;
use video::Video;

pub const WIDTH: usize = 256;
pub const HEIGHT: usize = 192;
pub const MONITOR: &[u8; 16384] = include_bytes!("../../../firmware/monitor/monitor.rom");
pub const FIRMWARE_MANIFEST: &str = include_str!("../../../firmware/monitor/manifest.json");
pub const BASIC: &[u8; 16384] = include_bytes!("../../../firmware/basic/basic.rom");
pub const BASIC_FIRMWARE_MANIFEST: &str = include_str!("../../../firmware/basic/manifest.json");
pub const BASIC_EXTENDED: &[u8; 32768] =
    include_bytes!("../../../firmware/basic-extended/basic.rom");
pub const BASIC_EXTENDED_FIRMWARE_MANIFEST: &str =
    include_str!("../../../firmware/basic-extended/manifest.json");

/// Explicit first-party firmware selection. Development is not ABI qualification.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Firmware {
    #[default]
    Monitor,
    BasicDevelopment,
    BasicExtended,
    Dragon64Compatibility,
    Dragon64Native,
}
impl Firmware {
    fn id(self) -> &'static str {
        match self {
            Self::Monitor => "eightbit-dragon32-monitor-0.1",
            Self::BasicDevelopment => "eightbit-dragon32-basic-0.1-dev",
            Self::BasicExtended => "eightbit-dragon32-basic-extended-1",
            Self::Dragon64Compatibility => "eightbit-dragon64-basic-32k-dev-1",
            Self::Dragon64Native => "eightbit-dragon64-basic-64k-dev-1",
        }
    }
    fn input_limit(self) -> usize {
        match self {
            Self::Monitor => 96,
            Self::BasicDevelopment => 251,
            Self::BasicExtended | Self::Dragon64Compatibility | Self::Dragon64Native => 249,
        }
    }
    fn is_basic(self) -> bool {
        !matches!(self, Self::Monitor)
    }
}
pub const MAX_SLICE: u32 = 32_768;
const KEY_HOLD_CYCLES: u32 = 8_000;

struct Board {
    ram: Box<[u8]>,
    firmware: &'static [u8],
    profile: Firmware,
    paging: dragon64::Paging,
    acia: acia::Acia,
    instruction_pc: u16,
    instruction_stack: u16,
    pia: [Pia; 2],
    sam: Sam,
    video: Video,
    io: Io,
    last_address: u16,
    master_clocks: u64,
    cartridge: Option<Box<[u8]>>,
    held: [u8; 8],
    typed_held: [u8; 8],
    cycles: u64,
    typed: VecDeque<Option<(usize, u8, bool)>>,
    key_remaining: u32,
    #[cfg(test)]
    signal_trace: Option<Vec<(u64, u16, u8)>>,
}
impl Default for Board {
    fn default() -> Self {
        Self {
            ram: vec![0; 32768].into_boxed_slice(),
            firmware: MONITOR,
            profile: Firmware::Monitor,
            paging: dragon64::Paging::default(),
            acia: acia::Acia::default(),
            instruction_pc: 0,
            instruction_stack: 0,
            pia: [Pia::default(); 2],
            sam: Sam::default(),
            video: Video::default(),
            io: Io::default(),
            last_address: 0xffff,
            master_clocks: 0,
            cartridge: None,
            held: [0; 8],
            typed_held: [0; 8],
            cycles: 0,
            typed: VecDeque::new(),
            key_remaining: 0,
            #[cfg(test)]
            signal_trace: None,
        }
    }
}
impl Board {
    // The expanded profile occupies the cartridge window without banking. This
    // is also the ROM value on the bus during the existing SAM upper-map write.
    fn rom_byte(&self, address: u16) -> u8 {
        if self.profile == Firmware::Dragon64Native {
            let offset = usize::from(address - 0xc000);
            if (0xd000..0xe000).contains(&address) && self.paging.page != 0 {
                dragon64::BASIC64_NATIVE[16384
                    + (usize::from(self.paging.page) - 1) * 4096
                    + usize::from(address - 0xd000)]
            } else {
                self.firmware[offset]
            }
        } else if address <= 0xbfff
            || matches!(
                self.profile,
                Firmware::BasicExtended | Firmware::Dragon64Compatibility
            )
        {
            self.firmware[usize::from(address - 0x8000)]
        } else {
            self.cartridge
                .as_ref()
                .and_then(|rom| rom.get(usize::from(address - 0xc000)))
                .copied()
                .unwrap_or(255)
        }
    }
    fn input(&self, chip: usize, side: usize) -> u8 {
        if chip == 0 && side == 0 {
            let columns = self.pia[0].data[1] | !self.pia[0].ddr[1];
            let mut rows = 255;
            for column in 0..8 {
                if columns & (1 << column) == 0 {
                    rows &= !(self.held[column] | self.typed_held[column]);
                }
            }
            for button in 0..2 {
                if self.io.buttons[button] {
                    rows &= !(1 << button);
                }
            }
            let selected = usize::from(self.pia[0].control_pin(0))
                | (usize::from(self.pia[0].control_pin(1)) << 1);
            let dac = self.pia[1].pins(0, 255) >> 2;
            // Host coordinates represent digital ADC bins, above the same-numbered
            // DAC level. Bin 63 therefore remains above the maximum DAC setting.
            if self.io.axes[selected] < dac {
                rows &= 0x7f;
            }
            rows
        } else if chip == 1 && side == 0 {
            0xfe | u8::from(self.io.tape_high)
        } else if chip == 1 {
            0xfa | u8::from(self.io.printer_busy)
        } else {
            255
        }
    }
    fn ram_address(&self, address: u16) -> Option<usize> {
        if self.profile.is_dragon64() && self.sam.control & 0x6000 >= 0x4000 {
            Some(usize::from(address))
        } else {
            self.sam.ram_address(address)
        }
    }
    fn peek(&self, address: u16) -> u8 {
        match address {
            0..=0x7fff => self
                .ram_address(address)
                .map_or(255, |offset| self.ram[offset]),
            0x8000..=0xbfff if self.profile == Firmware::Dragon64Native => self
                .ram_address(address)
                .map_or(255, |offset| self.ram[offset]),
            0x8000..=0xfeff if self.sam.all_ram() => self
                .ram_address(address)
                .map_or(255, |offset| self.ram[offset]),
            0x8000..=0xfeff => self.rom_byte(address),
            0xffe0..=0xffff if self.profile == Firmware::Dragon64Native => {
                self.firmware[usize::from(address - 0xc000)]
            }
            0xffe0..=0xffff => self.firmware[usize::from(address - 0xc000)],
            _ => 255,
        }
    }
    fn clear_input(&mut self) {
        self.held = [0; 8];
        self.typed_held = [0; 8];
        self.typed.clear();
        self.key_remaining = 0;
    }
}
impl Bus for Board {
    fn halted(&self) -> bool {
        self.io.halt
    }
    fn interrupts(&self) -> InterruptLines {
        InterruptLines {
            irq: self.pia[0].irq(),
            firq: self.pia[1].irq(),
            nmi: self.io.nmi,
        }
    }
    fn read(&mut self, address: u16) -> u8 {
        self.last_address = address;
        if self.profile.is_dragon64() && (0xff04..=0xff07).contains(&address) {
            self.acia.read(address)
        } else if (0xff00..=0xff3f).contains(&address) {
            let chip = usize::from((address >> 5) & 1);
            let reg = usize::from(address & 3);
            let input = self.input(chip, reg / 2);
            self.pia[chip].read(reg, input)
        } else {
            self.peek(address)
        }
    }
    fn write(&mut self, address: u16, value: u8) {
        self.last_address = address;
        match address {
            0..=0x7fff => {
                if let Some(offset) = self.ram_address(address) {
                    self.ram[offset] = value;
                }
            }
            0x8000..=0xbfff if self.profile == Firmware::Dragon64Native => {
                if let Some(offset) = self.ram_address(address) {
                    self.ram[offset] = value;
                }
            }
            0xff04..=0xff07 if self.profile.is_dragon64() => self.acia.write(address, value),
            0xff43
                if self.profile == Firmware::Dragon64Compatibility
                    && self.instruction_pc == 0xffe0 =>
            {
                self.profile = Firmware::Dragon64Native;
                self.firmware = dragon64::BASIC64_NATIVE;
                self.paging = dragon64::Paging::default();
            }
            0xff40..=0xff42 if self.profile == Firmware::Dragon64Native && !self.sam.all_ram() => {
                self.paging
                    .control(self.instruction_pc, self.instruction_stack, address, value);
            }
            0x8000..=0xfeff if self.sam.all_ram() => {
                // Unexpanded Dragon 32 decoder selects ROM during upper-map writes.
                let on_bus = if self.profile.is_dragon64() {
                    value
                } else {
                    self.rom_byte(address)
                };
                if let Some(offset) = self.ram_address(address) {
                    self.ram[offset] = on_bus;
                }
            }
            0xff00..=0xff3f => {
                let before = self.pia[1].pins(0, self.input(1, 0));
                self.pia[usize::from((address >> 5) & 1)].write(usize::from(address & 3), value);
                let after = self.pia[1].pins(0, self.input(1, 0));
                #[cfg(test)]
                if matches!(address, 0xff20 | 0xff22)
                    && let Some(trace) = &mut self.signal_trace
                    && trace.len() < 16_384
                {
                    trace.push((self.master_clocks, address, value));
                }
                if before & 2 != 0 && after & 2 == 0 {
                    self.io.print(self.pia[0].pins(1, 255));
                }
            }
            0xffc0..=0xffdf => self.sam.write(address),
            _ => {}
        }
    }
    fn tick(&mut self, cycles: u32) {
        for _ in 0..cycles {
            let clocks = self.sam.period(self.last_address);
            self.master_clocks = self.master_clocks.saturating_add(u64::from(clocks));
            if self.profile.is_dragon64() {
                self.acia.tick(u64::from(clocks));
            }
            let pins = self.pia[1].pins(1, self.input(1, 1));
            self.video.advance(clocks, &self.ram, self.sam, pins);
            let (hs, fs) = self.video.sync();
            self.pia[0].edge(0, false, hs);
            self.pia[0].edge(1, false, fs);
            self.pia[1].edge(0, false, self.io.printer_ack);
            // The explicitly mounted autorun cartridge connects CART to Q.
            // Each E cycle contains both Q edges; an empty slot remains high.
            if self.cartridge.is_some() {
                self.pia[1].edge(1, false, false);
            }
            self.pia[1].edge(1, false, true);
            let dac = self.pia[1].pins(0, self.input(1, 0)) >> 2;
            let select =
                u8::from(self.pia[0].control_pin(0)) | (u8::from(self.pia[0].control_pin(1)) << 1);
            let analogue = if self.pia[1].control_pin(1) {
                match select {
                    0 => (i16::from(dac) * 512) - 16128,
                    1 => {
                        if self.io.tape_high {
                            8192
                        } else {
                            -8192
                        }
                    }
                    2 => self.io.cartridge_sound,
                    _ => 0,
                }
            } else {
                0
            };
            let single = if self.pia[1].ddr[1] & 2 == 0 {
                0
            } else if pins & 2 == 0 {
                -4096
            } else {
                4096
            };
            let motor = self.pia[1].driven_control_pin(0) == Some(true);
            self.io
                .advance(clocks, motor, analogue.saturating_add(single), dac >= 32);
            self.pia[0].tick();
            self.pia[1].tick();
        }
        self.cycles = self.cycles.saturating_add(u64::from(cycles));
        let mut remaining = cycles;
        while remaining > 0 {
            if self.key_remaining == 0 {
                if let Some(key) = self.typed.pop_front() {
                    self.typed_held = [0; 8];
                    if let Some((column, row, shift)) = key {
                        self.typed_held[column] = row;
                        if shift {
                            self.typed_held[7] |= 1 << 6;
                        }
                    }
                    self.key_remaining = KEY_HOLD_CYCLES;
                } else {
                    break;
                }
            }
            let advance = remaining.min(self.key_remaining);
            self.key_remaining -= advance;
            remaining -= advance;
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub schema_version: u32,
    pub firmware_id: &'static str,
    pub cycles: u64,
    pub master_clocks: u64,
    pub video_frames: u64,
    pub pc: u16,
    pub state: &'static str,
    pub message: String,
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
    pub text: String,
}

pub struct Machine {
    cpu: Cpu,
    board: Board,
    failure: Option<Unsupported>,
    reset_profile: Firmware,
}
impl Default for Machine {
    fn default() -> Self {
        Self::new()
    }
}
impl Machine {
    pub fn mount_cassette(&mut self, pulses: Vec<TapePulse>) -> Result<(), String> {
        self.board.io.mount(pulses)
    }
    pub(crate) fn mount_cassette_signal(&mut self, signal: cassette::CassetteSignal) {
        self.board.io.mount_signal(signal);
    }
    pub fn eject_cassette(&mut self) {
        self.board.io.eject();
    }
    #[must_use]
    pub fn cassette_pending(&self) -> bool {
        self.board.io.pending()
    }
    pub fn take_audio(&mut self) -> Vec<i16> {
        self.board.io.audio()
    }
    /// Drain bounded digital UART output; this never opens a host serial device.
    pub fn take_serial_output(&mut self) -> Vec<u8> {
        self.board.acia.take_output()
    }
    pub fn take_printer(&mut self) -> Vec<u8> {
        self.board.io.printer()
    }
    pub fn take_cassette_recording(&mut self) -> Vec<TapePulse> {
        self.board.io.recording()
    }
    /// Set one of four digital joystick coordinate bins (0–63, default 32).
    /// A bin lies above its equal-numbered DAC level; this is not an analogue voltage.
    pub fn joystick(&mut self, index: usize, axis: u8) -> Result<(), String> {
        if index >= 4 || axis > 63 {
            return Err("DME1006: joystick axis must be 0–63 for one of four inputs".into());
        }
        self.board.io.axes[index] = axis;
        Ok(())
    }
    pub fn joystick_button(&mut self, index: usize, pressed: bool) -> Result<(), String> {
        if index >= 2 {
            return Err("DME1006: choose the right or left joystick button".into());
        }
        self.board.io.buttons[index] = pressed;
        Ok(())
    }
    /// Mount a bounded autorun cartridge through explicit host authority.
    /// Mounting is inert. Explicit advancement lets guest code handle its CART interrupt.
    pub fn load_cartridge(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        if matches!(
            self.board.profile,
            Firmware::BasicExtended | Firmware::Dragon64Compatibility | Firmware::Dragon64Native
        ) {
            return Err("DME1007: cartridges require XRoar; the expanded BASIC profile uses cartridge ROM space".into());
        }
        if bytes.is_empty() || bytes.len() > 16_384 {
            return Err("DME1007: cartridge must contain 1–16384 bytes".into());
        }
        self.board.cartridge = Some(bytes.into_boxed_slice());
        Ok(())
    }
    pub fn eject_cartridge(&mut self) {
        self.board.cartridge = None;
    }
    pub fn printer_signals(&mut self, busy: bool, ack: bool) {
        self.board.io.printer_busy = busy;
        self.board.io.printer_ack = ack;
    }
    pub fn nmi(&mut self, asserted: bool) {
        self.board.io.nmi = asserted;
    }
    pub fn halt(&mut self, asserted: bool) {
        self.board.io.halt = asserted;
    }
    #[must_use]
    pub fn new() -> Self {
        Self::with_firmware(Firmware::Monitor)
    }
    #[must_use]
    pub fn with_firmware(profile: Firmware) -> Self {
        let mut machine = Self {
            cpu: Cpu::default(),
            board: Board::default(),
            failure: None,
            reset_profile: profile,
        };
        machine.board.profile = profile;
        machine.board.firmware = match profile {
            Firmware::Monitor => MONITOR,
            Firmware::BasicDevelopment => BASIC,
            Firmware::BasicExtended => BASIC_EXTENDED,
            Firmware::Dragon64Compatibility => dragon64::BASIC64_COMPATIBILITY,
            Firmware::Dragon64Native => dragon64::BASIC64_NATIVE,
        };
        if profile.is_dragon64() {
            machine.board.ram = vec![0; 65536].into_boxed_slice();
        }
        machine.cpu.reset(&mut machine.board);
        machine
    }
    pub fn reset(&mut self) {
        if self.board.profile != self.reset_profile {
            self.board.ram[0x71] = 0;
            self.board.firmware = match self.reset_profile {
                Firmware::Monitor => MONITOR,
                Firmware::BasicDevelopment => BASIC,
                Firmware::BasicExtended => BASIC_EXTENDED,
                Firmware::Dragon64Compatibility => dragon64::BASIC64_COMPATIBILITY,
                Firmware::Dragon64Native => dragon64::BASIC64_NATIVE,
            };
        }
        self.board.profile = self.reset_profile;
        self.board.pia = [Pia::default(); 2];
        self.board.sam = Sam::default();
        self.board.paging = dragon64::Paging::default();
        self.board.acia = acia::Acia::default();
        self.board.video = Video::default();
        self.board.io.reset_audio();
        self.board.io.halt = false;
        self.board.io.nmi = false;
        self.board.clear_input();
        self.failure = None;
        self.cpu.reset(&mut self.board);
    }
    /// At most one instruction can exceed the requested cycle budget.
    pub fn advance(&mut self, cycles: u32) -> Result<(), String> {
        if cycles == 0 || cycles > MAX_SLICE {
            return Err("DME1002: use a bounded non-zero execution slice".into());
        }
        let mut elapsed = 0;
        while elapsed < cycles {
            elapsed += self.step()?;
        }
        Ok(())
    }
    pub fn release_keys(&mut self) {
        self.board.held = [0; 8];
    }
    pub fn cancel_input(&mut self) {
        self.board.clear_input();
    }
    /// User input alone; the core translates codes to the factual physical matrix.
    pub fn key(&mut self, code: &str, down: bool) -> Result<(), String> {
        if !self.board.typed.is_empty() {
            return Err("DME1003: wait for submitted keyboard input".into());
        }
        let (column, row) = key_code(code).ok_or("DME1003: unsupported monitor key")?;
        if down {
            self.board.held[column] |= row;
        } else {
            self.board.held[column] &= !row;
        }
        Ok(())
    }
    /// Atomically validate before enqueueing; guest firmware scans every key itself.
    pub fn type_text(&mut self, text: &str) -> Result<(), String> {
        self.enqueue_text(text, self.board.profile.input_limit())
    }
    fn enqueue_text(&mut self, text: &str, limit: usize) -> Result<(), String> {
        if (text.is_empty() && self.board.profile == Firmware::Monitor)
            || text.len() > limit
            || !text.is_ascii()
            || !self.board.typed.is_empty()
            || self.board.key_remaining != 0
        {
            return Err(format!(
                "DME1003: submit up to {limit} ASCII characters after previous input completes"
            ));
        }
        let keys: Option<Vec<_>> = text
            .bytes()
            .map(|b| match self.board.profile {
                Firmware::Monitor => ascii_key(b.to_ascii_uppercase()).map(|(c, r)| (c, r, false)),
                Firmware::BasicDevelopment
                | Firmware::BasicExtended
                | Firmware::Dragon64Compatibility
                | Firmware::Dragon64Native => basic_chord(b, self.peek(0x149) != 0),
            })
            .collect();
        let keys = keys.ok_or(match self.board.profile {
            Firmware::Monitor => {
                "DME1003: use monitor letters, digits, spaces and hexadecimal command input"
            }
            Firmware::BasicDevelopment
            | Firmware::BasicExtended
            | Firmware::Dragon64Compatibility
            | Firmware::Dragon64Native => {
                "DME1003: use characters available on the Dragon keyboard"
            }
        })?;
        for key in keys {
            // The guest samples Shift before scanning the letter/digit rows.
            // Settle the modifier first so a scan already in progress cannot
            // turn the opening quote into a digit or change a letter's case.
            if key.2 {
                self.board.typed.push_back(Some((7, 1 << 6, false)));
            }
            self.board.typed.push_back(Some(key));
            self.board.typed.push_back(None);
        }
        self.board.typed.push_back(Some((0, 1 << 6, false)));
        self.board.typed.push_back(None);
        Ok(())
    }
    #[must_use]
    pub fn input_pending(&self) -> bool {
        !self.board.typed.is_empty() || self.board.key_remaining > 0
    }
    #[must_use]
    pub fn peek(&self, address: u16) -> u8 {
        self.board.peek(address)
    }
    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        let base = usize::from(self.board.sam.base());
        let pixels = self.board.video.pixels.clone();
        let mut text = String::with_capacity(528);
        for row in 0..16 {
            for col in 0..32 {
                let cell = self
                    .board
                    .sam
                    .ram_address(u16::try_from((base + row * 32 + col) & 0xffff).unwrap_or(0))
                    .map_or(255, |offset| self.board.ram[offset]);
                let code = cell & 63;
                text.push(if cell & 128 != 0 {
                    ' '
                } else {
                    char::from(if code < 32 { code + 64 } else { code })
                });
            }
            text.push('\n');
        }
        Snapshot {
            schema_version: 1,
            firmware_id: self.board.profile.id(),
            cycles: self.board.cycles,
            master_clocks: self.board.master_clocks,
            video_frames: self.board.video.frames,
            pc: self.cpu.registers.pc,
            state: if self.failure.is_some() || self.board.paging.fault {
                "failed"
            } else {
                "running"
            },
            message: self.failure.map_or_else(
                || {
                    if self.board.paging.fault {
                        "DME1601: original Dragon 64 firmware page continuation is invalid".into()
                    } else {
                        match self.board.profile {
                    Firmware::Monitor => "Original monitor · BASIC is under development".into(),
                    Firmware::BasicExtended => "Original Studio BASIC · expanded ROM".into(),
                    Firmware::Dragon64Compatibility => {
                        "Original Dragon 64 · 32K compatibility".into()
                    }
                    Firmware::Dragon64Native => "Original Dragon 64 · native BASIC".into(),
                    Firmware::BasicDevelopment => {
                        "Original BASIC development · compatibility qualification incomplete".into()
                    }
                }
                    }
                },
                |error| error.to_string(),
            ),
            width: WIDTH,
            height: HEIGHT,
            pixels,
            text,
        }
    }
}
fn basic_chord(ascii: u8, capitals: bool) -> Option<(usize, u8, bool)> {
    if ascii.is_ascii_alphabetic() {
        return ascii_key(ascii.to_ascii_uppercase())
            .map(|(c, r)| (c, r, ascii.is_ascii_lowercase() == capitals));
    }
    if ascii < 32 {
        return None;
    }
    if let Some((c, r)) = ascii_key(ascii) {
        return Some((c, r, false));
    }
    let base = match ascii {
        b'!'..=b')' => ascii + 16,
        b'*' => b':',
        b'+' => b';',
        b'<'..=b'?' => ascii - 16,
        // The guest keyboard supplies these printable characters through
        // Shift with Down, Clear, Right and Up respectively.
        b'[' => 10,
        b'\\' => 12,
        b']' => 9,
        b'_' => 94,
        _ => return None,
    };
    ascii_key(base).map(|(c, r)| (c, r, true))
}
fn ascii_key(ascii: u8) -> Option<(usize, u8)> {
    let cells = b"0123456789:;,-./@ABCDEFGHIJKLMNOPQRSTUVWXYZ\x5e\x0a\x08\x09 \r\x0c\x03\0\0\0\0\0";
    cells
        .iter()
        .position(|&b| b == ascii && b != 0)
        .map(|n| (n % 8, 1 << (n / 8)))
}
fn key_code(code: &str) -> Option<(usize, u8)> {
    match code {
        "Enter" => ascii_key(13),
        "Space" => ascii_key(32),
        "Backspace" | "ArrowLeft" => ascii_key(8),
        "ArrowRight" => ascii_key(9),
        "ArrowUp" => ascii_key(94),
        "ArrowDown" => ascii_key(10),
        "Escape" => ascii_key(3),
        "Home" => ascii_key(12),
        "Comma" => ascii_key(b','),
        "Minus" => ascii_key(b'-'),
        "Period" => ascii_key(b'.'),
        "Slash" => ascii_key(b'/'),
        "Semicolon" => ascii_key(b';'),
        "Quote" => ascii_key(b':'),
        "Backquote" => ascii_key(b'@'),
        "ShiftLeft" | "ShiftRight" => Some((7, 64)),
        _ => {
            let bytes = code.as_bytes();
            if bytes.len() == 4 && code.starts_with("Key") {
                ascii_key(bytes[3])
            } else if bytes.len() == 6 && code.starts_with("Digit") {
                ascii_key(bytes[5])
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn boot() -> Machine {
        let mut m = Machine::new();
        for _ in 0..10 {
            assert!(m.advance(MAX_SLICE).is_ok());
        }
        assert!(m.snapshot().text.contains("EIGHTBIT DRAGON MONITOR 0.1"));
        m
    }
    fn command(m: &mut Machine, text: &str) {
        assert!(m.type_text(text).is_ok());
        for _ in 0..120 {
            assert!(m.advance(MAX_SLICE).is_ok());
            if !m.input_pending() {
                assert!(m.advance(MAX_SLICE).is_ok());
                return;
            }
        }
        assert!(!m.input_pending());
    }
    #[test]
    fn original_monitor_boot_and_commands_execute_in_guest() {
        let mut m = boot();
        command(&mut m, "M 0600 41");
        assert_eq!(m.peek(0x600), 0x41);
        command(&mut m, "F 0610 10 42");
        for a in 0x610..0x620 {
            assert_eq!(m.peek(a), 0x42);
        }
        command(&mut m, "C 0610 10");
        assert!(m.snapshot().text.contains("20"));
        command(&mut m, "L 0620 8641B7040039");
        assert_eq!(m.peek(0x620), 0x86);
        command(&mut m, "G 0620");
        assert_eq!(m.peek(0x400), 0x41);
        m.reset();
        assert!(m.advance(MAX_SLICE).is_ok());
        assert_eq!(m.peek(0x620), 0x86);
    }
    #[test]
    fn malformed_input_cannot_partially_write_ram() {
        let mut m = boot();
        for cmd in [
            "M 0500 41",
            "F 7FFF 2 41",
            "F 0600 0 41",
            "M 0600 100",
            "L 0600 4142XZ",
            "L 0600 414",
            "F 0600 0401 41",
            "M 8000 41",
        ] {
            command(&mut m, cmd);
            assert_eq!(m.peek(0x600), 0);
        }
        assert!(m.snapshot().text.contains("INVALID COMMAND"));
        assert!(m.type_text("a".repeat(97).as_str()).is_err());
        assert!(m.type_text("!x").is_err());
        assert!(!m.input_pending());
    }
    #[test]
    fn full_keyboard_line_is_rejected_without_truncating_to_a_valid_write() {
        let mut m = boot();
        let line = format!("M 0600 41{}", " ".repeat(87));
        assert_eq!(line.len(), 96);
        command(&mut m, &line);
        assert_eq!(m.peek(0x600), 0);
        assert!(m.snapshot().text.contains("INVALID COMMAND"));
        command(&mut m, "M 0600 41");
        assert_eq!(m.peek(0x600), 0x41);
        command(&mut m, "R BAD");
        assert!(m.snapshot().text.contains("INVALID COMMAND"));
    }
    #[test]
    fn bus_mapping_pia_and_rom_protection() {
        let mut b = Board::default();
        assert_eq!(b.peek(0xfffe), MONITOR[16382]);
        b.write(0x8000, 0);
        assert_eq!(b.peek(0x8000), MONITOR[0]);
        b.write(0xff03, 0);
        b.write(0xff02, 255);
        b.write(0xff03, 4);
        b.write(0xff02, 0xfd);
        b.held[1] = 4;
        assert_eq!(b.read(0xff00), 0);
        b.write(0xff01, 4);
        assert_eq!(b.read(0xff00) & 4, 0);
        b.clear_input();
        assert_eq!(b.read(0xff00) & 4, 4);
    }
    #[test]
    fn sam_upper_map_and_installed_ram_addressing() {
        let mut b = Board::default();
        b.sam.control = 0x4000;
        b.write(0x4200, 0x72);
        assert_eq!(b.read(0x4200), 0x72);
        assert_eq!(b.read(0x200), 0);
        b.write(0xffdf, 0);
        b.write(0x8200, 0x11);
        assert_eq!(b.read(0x200), MONITOR[0x200]);
        assert_eq!(b.read(0x8200), MONITOR[0x200]);
        b.write(0xffde, 0);
        assert_eq!(b.read(0x8200), MONITOR[0x200]);
        b.write(0xffdc, 0);
        b.write(0x300, 0x51);
        assert_eq!(b.read(0x2300), 0x51);
        assert_ne!(b.sam.ram_address(0x400), b.sam.ram_address(0x440));
    }
    #[test]
    fn sync_edges_route_irq_and_cartridge_routes_firq() {
        let mut b = Board::default();
        b.sam.control = 0x4000;
        b.write(0xff01, 5);
        b.write(0xff03, 5);
        b.tick(1);
        assert!(b.interrupts().irq);
        assert_ne!(b.read(0xff01) & 128, 0);
        b.read(0xff00);
        b.read(0xff02);
        assert!(!b.interrupts().irq);
        b.tick(60);
        assert!(b.interrupts().irq);
        b.write(0xff23, 5);
        b.cartridge = Some(vec![0x42; 100].into_boxed_slice());
        b.tick(1);
        assert!(b.interrupts().firq);
        assert_eq!(b.read(0xc000), 0x42);
        b.read(0xff22);
        assert!(!b.interrupts().firq);
    }
    #[test]
    fn joystick_comparator_buttons_and_printer_strobe() {
        let mut b = Board::default();
        b.sam.control = 0x4000;
        b.write(0xff01, 0x34);
        b.write(0xff03, 0x34);
        b.write(0xff21, 4);
        b.write(0xff20, 2);
        b.write(0xff21, 0);
        b.write(0xff20, 0xfe);
        b.write(0xff21, 4);
        b.write(0xff20, 31 << 2 | 2);
        assert_ne!(b.read(0xff00) & 128, 0);
        b.write(0xff20, 32 << 2 | 2);
        assert_ne!(b.read(0xff00) & 128, 0);
        b.write(0xff20, 33 << 2 | 2);
        assert_eq!(b.read(0xff00) & 128, 0);
        b.io.buttons[0] = true;
        assert_eq!(b.read(0xff00) & 1, 0);
        b.write(0xff03, 0x30);
        b.write(0xff02, 255);
        b.write(0xff03, 0x34);
        b.write(0xff02, 0x5a);
        b.write(0xff20, 0);
        assert_eq!(b.io.printer(), [0x5a]);
        b.write(0xff23, 4);
        assert_eq!(b.read(0xff22) & 1, 0);
        b.io.printer_busy = true;
        assert_eq!(b.read(0xff22) & 1, 1);
    }
    #[test]
    fn joystick_coordinate_bins_cover_every_axis_dac_and_button_state() {
        let mut m = Machine::new();
        assert_eq!(m.board.io.axes, [32; 4]);
        m.board.write(0xff21, 0);
        m.board.write(0xff20, 0xfe);
        m.board.write(0xff21, 4);
        for selected in 0_u8..4 {
            m.board.write(0xff01, 0x34 | ((selected & 1) << 3));
            m.board.write(0xff03, 0x34 | ((selected & 2) << 2));
            for coordinate in 0_u8..64 {
                for index in 0..4 {
                    assert!(m.joystick(index, (coordinate + 31) & 63).is_ok());
                }
                assert!(m.joystick(usize::from(selected), coordinate).is_ok());
                for buttons in 0_u8..4 {
                    for index in 0..2 {
                        assert!(
                            m.joystick_button(index, buttons & (1 << index) != 0)
                                .is_ok()
                        );
                    }
                    for dac in 0_u8..64 {
                        m.board.write(0xff20, (dac << 2) | 2);
                        let pins = m.board.read(0xff00);
                        assert_eq!(
                            pins & 0x80 != 0,
                            coordinate >= dac,
                            "axis={selected} coordinate={coordinate} dac={dac} buttons={buttons}"
                        );
                        assert_eq!(pins & 3, (!buttons) & 3);
                        assert_eq!(pins & 0x7c, 0x7c);
                    }
                }
            }
        }
        let axes = m.board.io.axes;
        let buttons = m.board.io.buttons;
        assert!(m.joystick(4, 0).is_err());
        assert!(m.joystick(0, 64).is_err());
        assert!(m.joystick_button(2, true).is_err());
        assert_eq!(m.board.io.axes, axes);
        assert_eq!(m.board.io.buttons, buttons);
    }
    #[test]
    fn host_io_validation_is_atomic_and_capture_is_bounded() {
        let mut m = Machine::new();
        assert!(m.joystick(4, 30).is_err());
        assert!(m.joystick(0, 64).is_err());
        assert!(m.joystick_button(2, true).is_err());
        assert!(m.load_cartridge(vec![]).is_err());
        assert!(m.load_cartridge(vec![1; 16385]).is_err());
        assert!(
            m.mount_cassette(vec![TapePulse {
                clocks: 1,
                high: true
            }])
            .is_ok()
        );
        assert!(
            m.mount_cassette(vec![TapePulse {
                clocks: 0,
                high: true
            }])
            .is_err()
        );
        assert!(m.cassette_pending());
        m.eject_cassette();
        assert!(!m.cassette_pending());
        for _ in 0..40 {
            assert!(m.advance(MAX_SLICE).is_ok());
        }
        assert!(m.take_audio().len() <= 4096);
        assert!(m.take_audio().is_empty());
        let s = m.snapshot();
        assert!(s.video_frames > 0);
        assert_eq!(s.master_clocks, s.cycles * 16);
    }
    #[test]
    #[ignore = "Explicit opt-in: exact lawfully provisioned Dragon 32 1982 ROM; never ordinary CI"]
    fn lawful_rom_boots_on_original_cpu_and_board_and_accepts_matrix_input() -> Result<(), String> {
        let mut m = lawful_machine()?;
        assert_eq!((m.peek(0x19), m.peek(0x1a)), (0x1e, 1));
        assert_eq!((m.peek(0x21), m.peek(0x22)), (0x7f, 0x36));
        assert_eq!((m.peek(0x27), m.peek(0x28)), (0x7f, 0xfe));
        // A physical press spans several PAL fields; the monitor's shorter
        // synthetic typing cadence is not assumed to match the OEM debounce.
        for byte in b"POKE 25000,77\r" {
            let code = match byte {
                b'A'..=b'Z' => format!("Key{}", char::from(*byte)),
                b'0'..=b'9' => format!("Digit{}", char::from(*byte)),
                b' ' => "Space".into(),
                b',' => "Comma".into(),
                _ => "Enter".into(),
            };
            assert!(m.key(&code, true).is_ok());
            for _ in 0..2 {
                assert!(m.advance(MAX_SLICE).is_ok());
            }
            assert!(m.key(&code, false).is_ok());
            for _ in 0..2 {
                assert!(m.advance(MAX_SLICE).is_ok());
            }
        }
        assert_eq!(
            m.peek(25000),
            77,
            "Original matrix input did not execute the synthetic POKE"
        );
        Ok(())
    }
    #[test]
    #[ignore = "Explicit opt-in: lawful exact-profile ROM, public keyboard results only"]
    fn lawful_rom_editor_control_key_observations() -> Result<(), String> {
        for (key, expected) in [
            ("Backspace", [8, 21]),
            ("ArrowRight", [9, 93]),
            ("ArrowUp", [94, 95]),
            ("ArrowDown", [10, 91]),
            ("Escape", [3, 3]),
            ("Enter", [13, 13]),
            ("Home", [12, 92]),
        ] {
            for shifted in [false, true] {
                let mut m = lawful_machine()?;
                // Original RAM driver waits on the published keyboard call;
                // only the returned character is retained, never ROM code.
                let driver = [0xbd, 0x80, 0x06, 0x27, 0xfb, 0xb7, 0x63, 0x00, 0x20, 0xfe];
                for (i, byte) in driver.iter().enumerate() {
                    m.board.ram[0x6200 + i] = *byte;
                }
                m.cpu.registers.pc = 0x6200;
                m.cpu.registers.s = 0x7000;
                if shifted {
                    m.key("ShiftLeft", true)?;
                }
                m.key(key, true)?;
                for _ in 0..8 {
                    m.advance(MAX_SLICE)?;
                }
                assert_eq!(
                    m.peek(0x6300),
                    expected[usize::from(shifted)],
                    "{key}, shifted={shifted}"
                );
            }
        }
        Ok(())
    }

    #[test]
    #[ignore = "Explicit opt-in: lawful exact-profile ROM, public keyboard results only"]
    fn lawful_rom_keyboard_mode_key_observations() -> Result<(), String> {
        for key in ["Digit0", "Backquote"] {
            let mut m = lawful_machine()?;
            let driver = [0xbd, 0x80, 0x06, 0x27, 0xfb, 0xb7, 0x63, 0x00, 0x20, 0xfe];
            for (i, byte) in driver.iter().enumerate() {
                m.board.ram[0x6200 + i] = *byte;
            }
            m.cpu.registers.pc = 0x6200;
            m.cpu.registers.s = 0x7000;
            let before = m.peek(0x149);
            m.key("ShiftLeft", true)?;
            m.key(key, true)?;
            for _ in 0..8 {
                m.advance(MAX_SLICE)?;
            }
            assert_eq!(m.peek(0x6300), if key == "Digit0" { 0 } else { 19 });
            assert_eq!(
                m.peek(0x149),
                if key == "Digit0" { !before } else { before }
            );
        }
        Ok(())
    }

    #[test]
    #[ignore = "Explicit opt-in: lawful exact-profile ROM, original EDIT/Break observations only"]
    fn lawful_rom_edit_break_observations() -> Result<(), String> {
        fn type_keys(m: &mut Machine, text: &str) -> Result<(), String> {
            for byte in text.bytes().chain([13]) {
                let (column, row, shifted) = if byte == 13 {
                    (0, 1 << 6, false)
                } else {
                    basic_chord(byte, m.peek(0x149) != 0)
                        .ok_or("Original fixture key is unavailable")?
                };
                if shifted {
                    m.key("ShiftLeft", true)?;
                    for _ in 0..2 {
                        m.advance(MAX_SLICE)?;
                    }
                }
                m.board.held[column] |= row;
                for _ in 0..2 {
                    m.advance(MAX_SLICE)?;
                }
                m.release_keys();
                for _ in 0..2 {
                    m.advance(MAX_SLICE)?;
                }
            }
            Ok(())
        }
        for insertion in [false, true] {
            let mut m = lawful_machine()?;
            type_keys(&mut m, "10 REM ABC")?;
            for _ in 0..64 {
                m.advance(MAX_SLICE)?;
            }
            let end = u16::from_be_bytes([m.peek(0x1b), m.peek(0x1c)]);
            assert!((0x1e02..0x1e40).contains(&end));
            let original: Vec<_> = (0x1e01..end).map(|a| m.peek(a)).collect();
            type_keys(&mut m, "EDIT10")?;
            for _ in 0..64 {
                m.advance(MAX_SLICE)?;
            }
            if insertion {
                for key in ["KeyI", "KeyZ"] {
                    m.key(key, true)?;
                    for _ in 0..2 {
                        m.advance(MAX_SLICE)?;
                    }
                    m.key(key, false)?;
                    for _ in 0..2 {
                        m.advance(MAX_SLICE)?;
                    }
                }
            }
            m.key("Escape", true)?;
            for _ in 0..2 {
                m.advance(MAX_SLICE)?;
            }
            m.key("Escape", false)?;
            for _ in 0..8 {
                m.advance(MAX_SLICE)?;
            }
            let break_shown = m.snapshot().text.contains("BREAK");
            type_keys(&mut m, "Z=85")?;
            for _ in 0..64 {
                m.advance(MAX_SLICE)?;
            }
            let start = u16::from_be_bytes([m.peek(0x1b), m.peek(0x1c)]);
            let variables_end = u16::from_be_bytes([m.peek(0x1d), m.peek(0x1e)]);
            assert!((0x1e02..0x1e80).contains(&start));
            assert!((start..start + 32).contains(&variables_end));
            let assigned =
                variables_end == start + 7 && m.peek(start) == b'Z' && m.peek(start + 1) == 0;
            let unchanged = start == end && (0x1e01..end).map(|a| m.peek(a)).eq(original);
            assert!(!break_shown);
            assert!(!assigned, "Break remains inside the original EDIT mode");
            assert_eq!(unchanged, !insertion);
            println!(
                "EDIT insertion={insertion}: Break-display={break_shown}, next-assignment={assigned}, original-line-retained={unchanged}"
            );
        }
        Ok(())
    }

    #[test]
    #[ignore = "Explicit opt-in: lawful exact-profile ROM, original runtime Break observations only"]
    fn lawful_rom_runtime_break_observations() -> Result<(), String> {
        fn type_keys(m: &mut Machine, text: &str) -> Result<(), String> {
            for byte in text.bytes().chain([13]) {
                let (column, row, shifted) = if byte == 13 {
                    (0, 1 << 6, false)
                } else {
                    basic_chord(byte, m.peek(0x149) != 0)
                        .ok_or("Original fixture key is unavailable")?
                };
                if shifted {
                    m.key("ShiftLeft", true)?;
                    m.advance(MAX_SLICE)?;
                    m.advance(MAX_SLICE)?;
                }
                m.board.held[column] |= row;
                m.advance(MAX_SLICE)?;
                m.advance(MAX_SLICE)?;
                m.release_keys();
                m.advance(MAX_SLICE)?;
                m.advance(MAX_SLICE)?;
            }
            Ok(())
        }
        for statement in [
            "GOTO10",
            "STOP",
            "SOUND89,64",
            "PLAY\"O1T1L1C\"",
            "PLAY\"T1P1\"",
        ] {
            let mut m = lawful_machine()?;
            type_keys(&mut m, &format!("10 POKE25000,1:{statement}:POKE25000,85"))?;
            type_keys(&mut m, "RUN")?;
            assert_eq!(m.peek(25000), 1, "original programme entered");
            m.key("Escape", true)?;
            for _ in 0..4 {
                m.advance(MAX_SLICE)?;
            }
            m.key("Escape", false)?;
            for _ in 0..8 {
                m.advance(MAX_SLICE)?;
            }
            let immediate_break = m.snapshot().text.contains("BREAK");
            let immediate_marker = m.peek(25000);
            // Bounded observation includes the complete longest original note.
            for _ in 0..512 {
                m.advance(MAX_SLICE)?;
            }
            let delayed_break = m.snapshot().text.contains("BREAK");
            let final_marker = m.peek(25000);
            let interruptible = matches!(statement, "GOTO10" | "STOP");
            assert_eq!(immediate_break, interruptible);
            assert_eq!(immediate_marker, 1);
            assert_eq!(delayed_break, interruptible);
            assert_eq!(final_marker, if interruptible { 1 } else { 85 });
            assert_eq!(m.board.pia[1].control[1] & 8, 0);
            println!(
                "runtime={statement} immediateBreak={immediate_break} immediateMarker={immediate_marker} delayedBreak={delayed_break} finalMarker={final_marker} DACgate={}",
                m.board.pia[1].control[1] & 8
            );
            type_keys(&mut m, "POKE25001,86")?;
            assert_eq!(m.peek(25001), 86, "command recovery after original probe");
            if statement == "STOP" {
                type_keys(&mut m, "CONT")?;
                assert_eq!(m.peek(25000), 85, "STOP resumes after its statement");
            }
        }
        Ok(())
    }

    fn lawful_machine() -> Result<Machine, String> {
        use sha2::{Digest, Sha256};
        use std::io::Read;
        let path = std::env::var_os("EIGHTBIT_STUDIO_ROM_PATH")
            .ok_or("Set EIGHTBIT_STUDIO_ROM_PATH to the lawfully provisioned exact-profile ROM")?;
        let file =
            std::fs::File::open(path).map_err(|_| "The selected lawful ROM must be readable")?;
        let mut bytes = Vec::new();
        file.take(16_385)
            .read_to_end(&mut bytes)
            .map_err(|_| "The selected lawful ROM must be readable within the size limit")?;
        assert_eq!(bytes.len(), 16_384, "Only the exact 16K profile is allowed");
        assert_eq!(
            hex::encode(Sha256::digest(&bytes)),
            "fc0e900bfec6b52f0f80ba1e65a4712808d2a411b5b00496639ef1a2152351f1"
        );
        let image: Box<[u8; 16_384]> = bytes
            .into_boxed_slice()
            .try_into()
            .map_err(|_| "Only the exact 16K profile is allowed")?;
        // Test-process-only storage; no third-party bytes enter build inputs,
        // snapshots, logs, fixtures or packages. The production ROM is unchanged.
        let mut m = Machine::new();
        m.board.firmware = Box::leak(image);
        m.cpu.reset(&mut m.board);
        for _ in 0..128 {
            assert!(
                m.advance(MAX_SLICE).is_ok(),
                "Reference boot encountered unsupported CPU behaviour"
            );
        }
        Ok(m)
    }
    #[test]
    #[ignore = "Explicit opt-in: lawful exact-profile ROM, signal timings only"]
    fn lawful_rom_sound_signal_observations() -> Result<(), String> {
        let mut m = lawful_machine()?;
        for pitch in [1, 89, 255] {
            m.board.signal_trace = Some(Vec::new());
            for byte in format!("SOUND{pitch},1\r").bytes() {
                let code = match byte {
                    b'A'..=b'Z' => format!("Key{}", char::from(byte)),
                    b'0'..=b'9' => format!("Digit{}", char::from(byte)),
                    b',' => "Comma".into(),
                    _ => "Enter".into(),
                };
                m.key(&code, true)?;
                for _ in 0..2 {
                    m.advance(MAX_SLICE)?;
                }
                m.key(&code, false)?;
                for _ in 0..2 {
                    m.advance(MAX_SLICE)?;
                }
            }
            let events = m.board.signal_trace.take().ok_or("Missing signal trace")?;
            let mut levels: Vec<_> = events
                .iter()
                .filter(|e| e.1 == 0xff20)
                .map(|e| e.2 >> 2)
                .collect();
            levels.sort_unstable();
            levels.dedup();
            println!("pitch={pitch} DAC-levels={levels:?}");
            for port in [0xff20, 0xff22] {
                let mut previous = None;
                let edges: Vec<_> = events
                    .iter()
                    .filter_map(|&(t, a, v)| {
                        let signal = if a == 0xff20 { v >> 2 } else { v & 2 };
                        if a != port || previous == Some(signal) {
                            return None;
                        }
                        previous = Some(signal);
                        Some(t)
                    })
                    .collect();
                let mut intervals: Vec<_> = edges.windows(2).map(|p| p[1] - p[0]).collect();
                intervals.sort_unstable();
                if let Some(median) = intervals.get(intervals.len() / 2) {
                    println!(
                        "pitch={pitch} port={port:04x} edges={} medianMasterClocks={median} totalMasterClocks={}",
                        edges.len(),
                        edges.last().copied().unwrap_or(0) - edges.first().copied().unwrap_or(0)
                    );
                }
            }
        }
        Ok(())
    }
    #[test]
    #[ignore = "Explicit opt-in: lawful exact-profile ROM, original PLAY signals and public workspace only"]
    fn lawful_rom_play_signal_observations() -> Result<(), String> {
        let mut m = lawful_machine()?;
        let cases: Vec<_> = (1..=5)
            .map(|octave| format!("O{octave}T8L4C"))
            .chain(
                [
                    "O2T8L4D",
                    "O2T8L4E",
                    "O2T8L4F",
                    "O2T8L4G",
                    "O2T8L4A",
                    "O2T8L4B",
                    "O2T8L4C#",
                    "O2T8L4V0C",
                    "O2T8L4V31C",
                    "O2T8L4.C",
                    "O2T8L4..C",
                    "O2T8L8C",
                    "O2T8L4P8",
                    "O2T16L4C",
                ]
                .map(String::from),
            )
            .collect();
        for text in cases {
            let mut previous_workspace = Vec::new();
            let mut snapshots = Vec::new();
            // An original tokenised one-line programme, using only the published
            // PLAY/POKE token contract. No reference code or internal entry is read.
            let mut payload = vec![0xb8, b'"'];
            payload.extend_from_slice(text.as_bytes());
            payload.extend_from_slice(&[b'"', b':', 0x93]);
            payload.extend_from_slice(b"25000,85");
            payload.push(0);
            let next = 0x1e05 + u16::try_from(payload.len()).map_err(|_| "Probe length")?;
            let mut line = next.to_be_bytes().to_vec();
            line.extend_from_slice(&10_u16.to_be_bytes());
            line.extend_from_slice(&payload);
            line.extend_from_slice(&[0, 0]);
            for (offset, byte) in line.into_iter().enumerate() {
                m.board.write(
                    0x1e01 + u16::try_from(offset).map_err(|_| "Probe length")?,
                    byte,
                );
            }
            for address in [0x1b, 0x1d, 0x1f] {
                let end = (next + 2).to_be_bytes();
                m.board.write(address, end[0]);
                m.board.write(address + 1, end[1]);
            }
            m.board.write(25000, 0);
            m.board.signal_trace = Some(Vec::new());
            for code in ["KeyR", "KeyU", "KeyN", "Enter"] {
                m.key(code, true)?;
                for _ in 0..2 {
                    if ["O2T8L4C", "O2T8L4.C", "O2T8L4..C", "O2T8L4P8"].contains(&text.as_str()) {
                        for _ in 0..512 {
                            m.advance(64)?;
                            let values: Vec<_> =
                                (0xdb..=0xe7).map(|address| m.peek(address)).collect();
                            if snapshots.len() < 24 && previous_workspace != values {
                                snapshots.push(serde_json::json!({"masterClocks":m.board.master_clocks,"workspace":values}));
                                previous_workspace = values;
                            }
                        }
                    } else {
                        m.advance(MAX_SLICE)?;
                    }
                }
                m.key(code, false)?;
                for _ in 0..2 {
                    if ["O2T8L4C", "O2T8L4.C", "O2T8L4..C", "O2T8L4P8"].contains(&text.as_str()) {
                        for _ in 0..512 {
                            m.advance(64)?;
                            let values: Vec<_> =
                                (0xdb..=0xe7).map(|address| m.peek(address)).collect();
                            if snapshots.len() < 24 && previous_workspace != values {
                                snapshots.push(serde_json::json!({"masterClocks":m.board.master_clocks,"workspace":values}));
                                previous_workspace = values;
                            }
                        }
                    } else {
                        m.advance(MAX_SLICE)?;
                    }
                }
            }
            for _ in 0..128 {
                if m.peek(25000) == 85 {
                    break;
                }
                if ["O2T8L4C", "O2T8L4.C", "O2T8L4..C", "O2T8L4P8"].contains(&text.as_str()) {
                    for _ in 0..512 {
                        m.advance(64)?;
                        let values: Vec<_> = (0xdb..=0xe7).map(|address| m.peek(address)).collect();
                        if snapshots.len() < 24 && previous_workspace != values {
                            snapshots.push(serde_json::json!({"masterClocks":m.board.master_clocks,"workspace":values}));
                            previous_workspace = values;
                        }
                    }
                } else {
                    m.advance(MAX_SLICE)?;
                }
            }
            assert_eq!(m.peek(25000), 85, "Synthetic PLAY did not finish: {text}");
            let events = m.board.signal_trace.take().ok_or("Missing signal trace")?;
            let mut previous = None;
            let edges: Vec<_> = events
                .iter()
                .filter_map(|&(time, address, byte)| {
                    if address != 0xff20 {
                        return None;
                    }
                    let level = byte >> 2;
                    if previous == Some(level) {
                        return None;
                    }
                    previous = Some(level);
                    Some((time, level))
                })
                .collect();
            let mut intervals: Vec<_> =
                edges.windows(2).map(|pair| pair[1].0 - pair[0].0).collect();
            intervals.sort_unstable();
            let workspace: Vec<_> = (0xd8..=0xe2).map(|address| m.peek(address)).collect();
            println!(
                "{}",
                serde_json::json!({"source":text,"audioGate":m.board.pia[1].control[1]&8,"samples":snapshots,"workspace":workspace,"edgeCount":edges.len(),"initialLevels":edges.iter().take(12).map(|event|event.1).collect::<Vec<_>>(),"medianMasterClocks":intervals.get(intervals.len()/2),"durationMasterClocks":edges.last().map(|last|last.0-edges[0].0)})
            );
        }
        Ok(())
    }
    #[test]
    #[ignore = "Explicit opt-in: lawful exact-profile ROM, cassette signal timings only"]
    fn lawful_rom_cassette_signal_observations() -> Result<(), String> {
        let mut m = lawful_machine()?;
        // Original RAM driver calls only published firmware interfaces.
        for byte in [0, 255, 85] {
            m.board.write(0x90, 0);
            m.board.write(0x91, 1);
            m.board.write(0x95, 0);
            m.board.write(0x96, 1);
            let driver = [
                0xbd, 0x80, 0x1b, 0x86, byte, 0xbd, 0x80, 0x1e, 0x7e, 0x62, 0x08,
            ];
            for (offset, &value) in driver.iter().enumerate() {
                m.board.write(
                    0x6200 + u16::try_from(offset).map_err(|_| "Driver size")?,
                    value,
                );
            }
            m.cpu.registers.pc = 0x6200;
            m.cpu.registers.s = 0x7e00;
            m.board.signal_trace = Some(Vec::new());
            let mut reached = false;
            for _ in 0..1_048_576 {
                if m.cpu.registers.pc == 0x6205 {
                    m.board.signal_trace = Some(Vec::new());
                }
                if m.cpu.registers.pc == 0x6208 {
                    reached = true;
                    break;
                }
                m.cpu
                    .step(&mut m.board)
                    .map_err(|_| "Unsupported CPU during cassette interface probe")?;
            }
            assert!(reached, "Public cassette byte interface did not return");
            let events = m.board.signal_trace.take().ok_or("Missing signal trace")?;
            let mut high = false;
            let rising: Vec<_> = events
                .iter()
                .filter_map(|&(t, a, v)| {
                    if a != 0xff20 {
                        return None;
                    }
                    let next = v >> 2 >= 32;
                    let rise = next && !high;
                    high = next;
                    rise.then_some(t)
                })
                .collect();
            let periods: Vec<_> = rising.windows(2).map(|p| p[1] - p[0]).collect();
            println!(
                "cassetteByte={byte} risingEdges={} bitPeriodsMasterClocks={periods:?}",
                rising.len()
            );
        }
        Ok(())
    }
    #[test]
    fn basic_rom_development_boot_uses_guest_workspace_and_console() {
        let mut m = Machine::new();
        m.board.firmware = include_bytes!("../../../firmware/basic/basic.rom");
        m.cpu.reset(&mut m.board);
        assert!(m.advance(MAX_SLICE).is_ok());
        assert!(
            m.snapshot().text.contains("EIGHTBIT BASIC DEVELOPMENT"),
            "PC {:04X} S {:04X} SAM {:04X} screen {:?}",
            m.cpu.registers.pc,
            m.cpu.registers.s,
            m.board.sam.control,
            m.snapshot().text
        );
        for _ in 0..10 {
            assert!(m.advance(MAX_SLICE).is_ok());
        }
        assert!(
            m.snapshot().text.contains("EIGHTBIT BASIC DEVELOPMENT"),
            "PC {:04X} S {:04X} SAM {:04X} screen {:?}",
            m.cpu.registers.pc,
            m.cpu.registers.s,
            m.board.sam.control,
            m.snapshot().text
        );
        assert!(m.snapshot().text.contains("NOT QUALIFIED"));
        assert_eq!((m.peek(0x19), m.peek(0x1a)), (0x1e, 1));
        assert_eq!((m.peek(0x68), m.peek(0x69)), (255, 255));
        assert_eq!(m.peek(0x167), 0x39);
        assert_eq!(m.peek(0x10c), 0x7e);
        assert_eq!(m.peek(0x120), 78);
        assert_eq!(m.peek(0x125), 34);
        assert!(m.take_printer().is_empty());
        command(&mut m, "HELLO");
        assert!(m.snapshot().text.contains("HELLO"));
        assert!(m.snapshot().text.contains("?SN ERROR"));
        assert_eq!(&m.board.ram[0x2dd..0x2e3], b"HELLO\0");
        command(&mut m, "10 END");
        assert_eq!(m.peek(0x1e01), 0x1e);
        assert_eq!(m.peek(0x1e02), 7);
        assert_eq!(m.peek(0x1e04), 10);
        assert_eq!(m.peek(0x1e05), 0x8a);
        command(&mut m, "10 CLS");
        assert_eq!(m.peek(0x1e05), 0xa0);
        command(&mut m, "10");
        assert_eq!((m.peek(0x1e01), m.peek(0x1e02)), (0, 0));
        m.board.write(0x6000, 0x5a);
        m.reset();
        assert!(m.advance(MAX_SLICE).is_ok());
        assert_eq!(m.peek(0x6000), 0x5a);
        assert!(m.snapshot().text.contains("DEV>"));
    }
    #[test]
    fn basic_rom_printer_guest_signals_wait_resume_and_reset() -> Result<(), String> {
        let mut m = Machine::with_firmware(Firmware::BasicDevelopment);
        for _ in 0..4 {
            m.advance(MAX_SLICE)?;
        }
        assert_eq!(m.board.read(0xff22) & 1, 0);
        command(&mut m, "PRINT#-2,\"ABC\";TAB(8);\"X\";");
        for _ in 0..16 {
            m.advance(MAX_SLICE)?;
        }
        assert_eq!(m.take_printer(), b"ABC     X");
        assert_eq!(m.peek(0x9c), 9);
        assert_eq!(m.peek(0x6f), 0);
        m.printer_signals(true, true);
        assert_eq!(m.board.read(0xff22) & 1, 1);
        command(&mut m, "PRINT#-2,\"WAIT\"");
        for _ in 0..4 {
            m.advance(MAX_SLICE)?;
        }
        assert!(m.take_printer().is_empty());
        assert_eq!(m.peek(0x6f), 0xfe);
        m.printer_signals(false, true);
        m.advance(MAX_SLICE)?;
        assert_eq!(m.take_printer(), b"WAIT\r");
        assert_eq!(m.peek(0x6f), 0);
        assert_eq!(m.peek(0x9c), 0);
        m.printer_signals(true, true);
        command(&mut m, "PRINT#-2,\"\"");
        for _ in 0..4 {
            m.advance(MAX_SLICE)?;
        }
        assert!(m.take_printer().is_empty());
        assert_eq!(m.peek(0x9c), 0);
        assert_eq!(m.peek(0x6f), 0xfe);
        m.printer_signals(false, true);
        m.advance(MAX_SLICE)?;
        assert_eq!(m.take_printer(), b" \r");
        assert_eq!(m.peek(0x9c), 0);
        assert_eq!(m.peek(0x6f), 0);
        m.printer_signals(true, true);
        command(&mut m, "PRINT#-2,\"CANCEL\"");
        for _ in 0..4 {
            m.advance(MAX_SLICE)?;
        }
        assert!(m.take_printer().is_empty());
        assert_eq!(m.peek(0x6f), 0xfe);
        m.reset();
        for _ in 0..4 {
            m.advance(MAX_SLICE)?;
        }
        assert_eq!(m.peek(0x6f), 0);
        assert_eq!(
            m.snapshot()
                .text
                .lines()
                .rev()
                .find(|line| !line.trim().is_empty())
                .map(str::trim),
            Some("DEV>")
        );
        command(&mut m, "PRINT#0,\"RECOVERED\"");
        for _ in 0..16 {
            m.advance(MAX_SLICE)?;
        }
        assert!(m.snapshot().text.contains("RECOVERED"));
        assert!(m.take_printer().is_empty());
        Ok(())
    }
    #[test]
    fn basic_rom_warm_reset_cancels_an_active_ascii_output_sink() -> Result<(), String> {
        let mut machine = Machine::with_firmware(Firmware::BasicDevelopment);
        for _ in 0..4 {
            machine.advance(MAX_SLICE)?;
        }
        // A full text block keeps the sink active during its physical gap,
        // independently of the instruction at which the advance slice ends.
        for number in [10, 20, 30] {
            command(
                &mut machine,
                &format!("{number} REM {}", "ORIGINAL".repeat(23)),
            );
            for _ in 0..4 {
                machine.advance(MAX_SLICE)?;
            }
        }
        command(&mut machine, "CSAVE\"RESET\",A");
        let mut active = false;
        for _ in 0..128 {
            machine.advance(MAX_SLICE)?;
            if machine.peek(0x03f6) == 0xff {
                active = true;
                break;
            }
        }
        assert!(
            active,
            "the regression must interrupt an active ASCII sink: {}",
            machine.snapshot().text
        );
        assert_ne!(machine.board.read(0xff21) & 8, 0);
        machine.reset();
        for _ in 0..4 {
            machine.advance(MAX_SLICE)?;
        }
        assert_eq!(machine.peek(0x03f6), 0);
        assert_eq!(machine.peek(0x6f), 0);
        assert_eq!(machine.board.read(0xff21) & 8, 0);
        assert_eq!(
            machine
                .snapshot()
                .text
                .lines()
                .rev()
                .find(|line| !line.trim().is_empty())
                .map(str::trim),
            Some("DEV>")
        );
        command(&mut machine, "PRINT\"RESET RECOVERED\"");
        for _ in 0..16 {
            machine.advance(MAX_SLICE)?;
        }
        assert!(machine.snapshot().text.contains("RESET RECOVERED"));
        assert!(machine.take_printer().is_empty());
        Ok(())
    }
    #[test]
    fn physical_keyboard_matrix_covers_cursor_punctuation_and_shift() {
        let mut m = boot();
        for (code, column, row) in [
            ("ArrowUp", 3, 5),
            ("ArrowDown", 4, 5),
            ("ArrowLeft", 5, 5),
            ("ArrowRight", 6, 5),
            ("Comma", 4, 1),
            ("Minus", 5, 1),
            ("Period", 6, 1),
            ("Slash", 7, 1),
            ("Semicolon", 3, 1),
            ("Quote", 2, 1),
            ("Backquote", 0, 2),
            ("ShiftLeft", 7, 6),
        ] {
            assert!(m.key(code, true).is_ok());
            m.board.write(0xff02, !(1 << column));
            assert_eq!(m.board.read(0xff00) & (1 << row), 0, "pressed {code}");
            assert!(m.key(code, false).is_ok());
            assert_ne!(m.board.read(0xff00) & (1 << row), 0, "released {code}");
        }
    }
    #[test]
    fn deterministic_replay_and_bounded_failure_recovery() {
        let mut a = boot();
        let mut b = boot();
        command(&mut a, "M 0700 02");
        command(&mut b, "M 0700 02");
        assert_eq!(a.snapshot(), b.snapshot());
        assert!(a.advance(MAX_SLICE + 1).is_err());
        assert!(a.type_text("G 0700").is_ok());
        let mut stopped = false;
        for _ in 0..50 {
            if a.advance(MAX_SLICE).is_err() {
                stopped = true;
                break;
            }
        }
        assert!(stopped);
        assert_eq!(a.snapshot().state, "failed");
        a.reset();
        assert!(a.advance(MAX_SLICE).is_ok());
    }
    #[test]
    fn infinite_guest_program_obeys_slice_budget_and_reset_recovers() {
        let mut m = boot();
        command(&mut m, "L 0600 20FE");
        command(&mut m, "G 0600");
        for _ in 0..10 {
            let before = m.board.cycles;
            assert!(m.advance(MAX_SLICE).is_ok());
            assert!(
                (u64::from(MAX_SLICE)..=u64::from(MAX_SLICE + 2))
                    .contains(&(m.board.cycles - before))
            );
            assert_eq!(m.cpu.registers.pc, 0x600);
        }
        m.reset();
        assert!(m.advance(MAX_SLICE).is_ok());
        assert!(m.snapshot().text.contains("EIGHTBIT DRAGON MONITOR"));
        assert_eq!(m.peek(0x600), 0x20);
    }
    #[test]
    fn key_release_and_firmware_identity() {
        let mut m = boot();
        assert!(m.key("KeyA", true).is_ok());
        assert!(m.advance(MAX_SLICE).is_ok());
        m.release_keys();
        assert!(m.advance(MAX_SLICE).is_ok());
        assert!(m.key("Bogus", true).is_err());
        use sha2::{Digest, Sha256};
        assert!(FIRMWARE_MANIFEST.contains(&hex::encode(Sha256::digest(MONITOR))));
    }
    #[test]
    fn blur_releases_physical_keys_without_truncating_submitted_commands() {
        let mut m = boot();
        assert!(m.type_text("M 0600 41").is_ok());
        assert!(m.advance(8_000).is_ok());
        m.release_keys();
        assert!(m.input_pending());
        for _ in 0..10 {
            assert!(m.advance(MAX_SLICE).is_ok());
        }
        assert_eq!(m.peek(0x600), 0x41);
        assert!(!m.input_pending());
        assert!(m.type_text("M 0600 42").is_ok());
        m.cancel_input();
        assert!(!m.input_pending());
        assert!(m.advance(MAX_SLICE).is_ok());
        assert_eq!(m.peek(0x600), 0x41);
    }
    #[test]
    fn development_basic_profile_types_shift_chords_preserves_case_and_resets() -> Result<(), String>
    {
        let mut m = Machine::with_firmware(Firmware::BasicDevelopment);
        for _ in 0..4 {
            m.advance(MAX_SLICE)?;
        }
        assert_eq!(
            m.snapshot().firmware_id,
            serde_json::from_str::<serde_json::Value>(BASIC_FIRMWARE_MANIFEST)
                .map_err(|e| e.to_string())?["id"]
        );
        assert!(m.snapshot().message.contains("qualification incomplete"));
        m.type_text("a$=\"Mixed !?\":PRINT a$")?;
        assert!(
            m.type_text("PRINT 1").is_err(),
            "typed lines cannot overlap"
        );
        for _ in 0..32 {
            m.advance(MAX_SLICE)?;
        }
        assert!(!m.input_pending());
        let table = u16::from_be_bytes([m.peek(0x1b), m.peek(0x1c)]);
        assert_eq!((m.peek(table), m.peek(table + 1)), (b'A', 128));
        let address = u16::from_be_bytes([m.peek(table + 4), m.peek(table + 5)]);
        let result: Vec<_> = (0..u16::from(m.peek(table + 2)))
            .map(|n| m.peek(address + n))
            .collect();
        assert_eq!(result, b"Mixed !?", "{}", m.snapshot().text);
        assert!(m.snapshot().text.contains("MIXED !?"));
        assert!(!m.snapshot().text.contains(" ERROR"));
        assert!(m.type_text(&"A".repeat(252)).is_err());
        assert!(m.type_text("PRINT {1}").is_err());
        assert!(
            !m.input_pending(),
            "invalid input must not enqueue a partial chord"
        );
        m.type_text("LINE INPUT B$:PRINT LEN(B$)")?;
        for _ in 0..64 {
            m.advance(MAX_SLICE)?;
        }
        m.type_text("")?;
        for _ in 0..16 {
            m.advance(MAX_SLICE)?;
        }
        assert!(m.snapshot().text.contains(" 0 "));
        assert!(!m.snapshot().text.contains(" ERROR"));
        m.type_text("PRINT 123")?;
        m.reset();
        assert!(!m.input_pending());
        for _ in 0..4 {
            m.advance(MAX_SLICE)?;
        }
        assert_eq!(
            m.snapshot().firmware_id,
            serde_json::from_str::<serde_json::Value>(BASIC_FIRMWARE_MANIFEST)
                .map_err(|e| e.to_string())?["id"]
        );
        assert!(m.snapshot().text.contains("DEV>"));
        Ok(())
    }
    #[test]
    fn typed_shifted_navigation_characters_reach_guest_strings_in_both_case_modes()
    -> Result<(), String> {
        for capitals in [0, 255] {
            let mut machine = Machine::with_firmware(Firmware::BasicDevelopment);
            for _ in 0..4 {
                machine.advance(MAX_SLICE)?;
            }
            machine.board.ram[0x149] = capitals;
            machine.type_text(r#"A$="[]\_^":B$="MiXeD[]\_":POKE25344,85"#)?;
            for _ in 0..128 {
                machine.advance(MAX_SLICE)?;
                if !machine.input_pending() && machine.peek(0x6300) == 85 {
                    break;
                }
            }
            assert!(!machine.input_pending());
            assert_eq!(machine.peek(0x6300), 85, "{}", machine.snapshot().text);
            let table = u16::from_be_bytes([machine.peek(0x1b), machine.peek(0x1c)]);
            for (offset, name, expected) in [
                (0, b'A', b"[]\\_^".as_slice()),
                (7, b'B', b"MiXeD[]\\_".as_slice()),
            ] {
                let entry = table + offset;
                assert_eq!((machine.peek(entry), machine.peek(entry + 1)), (name, 128));
                let address =
                    u16::from_be_bytes([machine.peek(entry + 4), machine.peek(entry + 5)]);
                let actual: Vec<_> = (0..u16::from(machine.peek(entry + 2)))
                    .map(|index| machine.peek(address + index))
                    .collect();
                assert_eq!(actual, expected);
            }
            assert_eq!(machine.peek(0x149), capitals, "case mode is unchanged");
            assert_eq!(machine.board.typed_held, [0; 8]);
            assert!(machine.type_text("PRINT {1}").is_err());
            assert!(!machine.input_pending());
        }
        Ok(())
    }
    #[test]
    fn typed_input_uses_compiled_identity_and_both_basic_limits_are_atomic() -> Result<(), String> {
        assert_eq!(Firmware::Monitor.input_limit(), 96);
        for (profile, limit) in [
            (Firmware::BasicDevelopment, 251),
            (Firmware::BasicExtended, 249),
        ] {
            assert_eq!(profile.input_limit(), limit);
            let mut machine = Machine::with_firmware(profile);
            for invalid in [
                "A".repeat(limit + 1),
                format!("{}{{", "A".repeat(limit - 1)),
                "é".into(),
            ] {
                assert!(machine.type_text(&invalid).is_err());
                assert!(!machine.input_pending());
                assert_eq!(machine.board.typed_held, [0; 8]);
            }
            machine.type_text(&"A".repeat(limit))?;
            let queued = machine.board.typed.clone();
            assert_eq!(
                queued.len(),
                limit * 3 + 2,
                "Shift setup, characters and Return/release"
            );
            assert!(machine.type_text("B").is_err());
            assert_eq!(machine.board.typed, queued, "pending input is not replaced");
            machine.reset();
            assert!(!machine.input_pending());
            machine.type_text("")?;
            assert_eq!(
                machine.board.typed.len(),
                2,
                "empty BASIC reply still submits Return"
            );
            machine.cancel_input();
            assert!(!machine.input_pending());
        }
        Ok(())
    }

    #[test]
    fn development_basic_play_uses_board_irqs_audio_and_reset_recovery() -> Result<(), String> {
        let mut m = Machine::with_firmware(Firmware::BasicDevelopment);
        for _ in 0..4 {
            m.advance(MAX_SLICE)?;
        }
        m.board.signal_trace = Some(Vec::new());
        command(&mut m, "PLAY\"O3V15T8L4C\":POKE25000,85");
        let mut audio = Vec::new();
        for _ in 0..64 {
            m.advance(MAX_SLICE)?;
            audio.extend(m.take_audio());
            if m.peek(25000) == 85 {
                break;
            }
        }
        assert_eq!(m.peek(25000), 85, "{}", m.snapshot().text);
        assert!(audio.windows(2).any(|p| p[0] != p[1]));
        assert!(!m.snapshot().text.contains(" ERROR"));
        let events = m
            .board
            .signal_trace
            .take()
            .ok_or("Missing synthetic trace")?;
        let mut levels: Vec<_> = events
            .iter()
            .filter(|e| e.1 == 0xff20)
            .map(|e| e.2 >> 2)
            .collect();
        levels.sort_unstable();
        levels.dedup();
        assert_eq!(levels, [16, 31, 46]);
        assert_eq!(
            m.board.pia[1].control[1] & 8,
            0,
            "PLAY closes the observed DAC gate"
        );
        command(&mut m, "PLAY\"O1T1L1C\":POKE25000,99");
        assert_eq!(m.peek(25000), 85, "long note is still running");
        m.reset();
        assert!(m.take_audio().is_empty());
        for _ in 0..4 {
            m.advance(MAX_SLICE)?;
        }
        assert!(m.snapshot().text.contains("DEV>"));
        assert_eq!(m.board.pia[1].control[1] & 8, 0);
        command(&mut m, "POKE25000,86");
        assert_eq!(m.peek(25000), 86);
        Ok(())
    }

    #[test]
    fn development_basic_physical_break_preserves_nested_continuation() -> Result<(), String> {
        let mut m = Machine::with_firmware(Firmware::BasicDevelopment);
        for _ in 0..8 {
            m.advance(MAX_SLICE)?;
        }
        for text in [
            "10 FOR I=1 TO 2",
            "20 GOSUB100",
            "30 NEXT I",
            "40 POKE25002,85:END",
            "100 POKE25000,I:IF PEEK(25001)=0 THEN100",
            "110 RETURN",
            "RUN",
        ] {
            command(&mut m, text);
            for _ in 0..4 {
                m.advance(MAX_SLICE)?;
            }
        }
        assert_eq!(m.peek(25000), 1);
        m.key("KeyA", true)?;
        for _ in 0..4 {
            m.advance(MAX_SLICE)?;
        }
        m.key("KeyA", false)?;
        assert!(!m.snapshot().text.contains("BREAK"));
        m.key("Escape", true)?;
        for _ in 0..4 {
            m.advance(MAX_SLICE)?;
        }
        m.key("Escape", false)?;
        for _ in 0..4 {
            m.advance(MAX_SLICE)?;
        }
        assert!(m.snapshot().text.contains("BREAK IN 100"));
        assert_eq!(m.peek(25002), 0);
        command(&mut m, "POKE25001,1:CONT");
        for _ in 0..32 {
            m.advance(MAX_SLICE)?;
            if m.peek(25002) == 85 {
                break;
            }
        }
        assert_eq!(m.peek(25002), 85, "{}", m.snapshot().text);
        assert_eq!(m.peek(25000), 2);
        assert!(!m.snapshot().text.contains(" ERROR"));
        Ok(())
    }

    #[test]
    fn development_basic_tones_ignore_brief_break_and_recover_after_completion()
    -> Result<(), String> {
        for statement in ["SOUND89,64", "PLAY\"O1T1L1C\"", "PLAY\"T1P1\""] {
            let mut m = Machine::with_firmware(Firmware::BasicDevelopment);
            for _ in 0..4 {
                m.advance(MAX_SLICE)?;
            }
            command(&mut m, &format!("POKE25000,1:{statement}:POKE25000,85"));
            for _ in 0..8 {
                if m.peek(25000) == 1 {
                    break;
                }
                m.advance(MAX_SLICE)?;
            }
            assert_eq!(m.peek(25000), 1, "{statement}: {}", m.snapshot().text);
            m.key("Escape", true)?;
            for _ in 0..4 {
                m.advance(MAX_SLICE)?;
            }
            m.key("Escape", false)?;
            for _ in 0..8 {
                m.advance(MAX_SLICE)?;
            }
            assert_eq!(m.peek(25000), 1, "{statement} remains active");
            assert!(!m.snapshot().text.contains("BREAK"));
            for _ in 0..512 {
                m.advance(MAX_SLICE)?;
                if m.peek(25000) == 85 {
                    break;
                }
            }
            assert_eq!(m.peek(25000), 85, "{statement} completes normally");
            assert_eq!(m.board.pia[1].control[1] & 8, 0);
            command(&mut m, "POKE25001,86");
            assert_eq!(m.peek(25001), 86);
        }
        Ok(())
    }
    #[test]
    fn explicit_cartridge_load_supplies_both_q_edges_and_eject_clears_the_source()
    -> Result<(), String> {
        for control in [4_u8, 5, 6, 7] {
            let mut machine = Machine::new();
            machine.board.write(0xff23, control);
            machine.board.read(0xff22);
            let before = machine.snapshot();
            // Authored NOP/BRA idle cartridge, never executed by this pin test.
            machine.load_cartridge(vec![0x12, 0x20, 0xfd])?;
            assert_eq!(
                machine.snapshot(),
                before,
                "mount and state reads are inert"
            );
            assert!(!machine.board.interrupts().firq);
            for _ in 0..3 {
                machine.board.tick(1);
                assert_ne!(
                    machine.board.read(0xff23) & 0x80,
                    0,
                    "both selectable edges"
                );
                assert_eq!(machine.board.interrupts().firq, control & 1 != 0);
                machine.board.read(0xff22);
                assert!(!machine.board.interrupts().firq);
            }
            assert!(machine.load_cartridge(Vec::new()).is_err());
            assert!(machine.load_cartridge(vec![0; 16_385]).is_err());
            assert_eq!(machine.peek(0xc000), 0x12, "invalid replacement is atomic");
            machine.eject_cartridge();
            machine.board.read(0xff22);
            for _ in 0..3 {
                machine.board.tick(1);
            }
            assert_eq!(machine.board.read(0xff23) & 0x80, 0);
            assert!(!machine.board.interrupts().firq);
            assert_eq!(machine.peek(0xc000), 0xff);
        }
        Ok(())
    }
}
