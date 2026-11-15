//! Original Spectrum digital board shared by native and WebAssembly hosts.
//! Third-party firmware, external emulators and host BASIC evaluation are absent.
#![allow(missing_docs)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

mod ay;
mod model;
pub use model::Model;
mod recording;
mod tape;
pub use recording::TapeCapture;
mod wave;
use eightbit_z80_core::{Access, Bus, Cpu, PrefixLimit};
use std::collections::VecDeque;
pub use tape::{MAX_TAP_BLOCKS, MAX_TAP_BYTES, Tape};
pub use wave::Wave;

pub const CPU_HZ: u32 = 3_500_000;
pub const FRAME_T_STATES: u32 = 69888;
pub const LINE_T_STATES: u32 = 224;
pub const DISPLAY_START: u32 = 14336;
pub const CONTENTION_START: u32 = 14335;
pub const AUDIO_HZ: u32 = 44100;
pub const WIDTH: usize = 320;
pub const HEIGHT: usize = 256;
pub const MAX_SLICE: u32 = 350_000;
pub const PRIVATE_PAGE_PORT: u16 = 0xfdfd;
/// Original development image only; its presence does not admit normal Run/Debug.
pub const DEVELOPMENT_FIRMWARE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/spectrum48.rom"));
pub const DEVELOPMENT_MANIFEST: &str =
    include_str!(concat!(env!("OUT_DIR"), "/spectrum48-manifest.json"));
// Bound to this original firmware build, never to a third-party ROM address.
include!(concat!(env!("OUT_DIR"), "/statement-entry.rs"));
const PAGE_BYTES: usize = 8192;
const MAX_PAGES: usize = 128;
const AUDIO_CAPACITY: usize = 8192;
const SIGNAL_CAPACITY: usize = 65536;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Firmware,
    FirmwarePage,
    Tape(&'static str),
    Input,
    Slice,
    Cpu(PrefixLimit),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Firmware => f.write_str("S48EMU1001: invalid original firmware container"),
            Self::FirmwarePage => f.write_str("S48EMU1002: firmware selected an unavailable page"),
            Self::Tape(message) => write!(f, "S48EMU1003: {message}"),
            Self::Input => f.write_str("S48EMU1004: unsupported keyboard key"),
            Self::Slice => f.write_str("S48EMU1005: execution slice exceeds its limit"),
            Self::Cpu(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

/// Fixed 8 KiB common ROM plus 1–128 privately selected 8 KiB pages; no extra RAM.
pub struct Firmware {
    bytes: Box<[u8]>,
}
impl Firmware {
    pub fn new(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < PAGE_BYTES * 2
            || bytes.len() > PAGE_BYTES * (MAX_PAGES + 1)
            || !bytes.len().is_multiple_of(PAGE_BYTES)
        {
            return Err(Error::Firmware);
        }
        Ok(Self {
            bytes: bytes.into(),
        })
    }
    fn pages(&self) -> usize {
        self.bytes.len() / PAGE_BYTES - 1
    }
    fn read(&self, page: u8, address: u16) -> u8 {
        let address = usize::from(address);
        self.bytes[if address < PAGE_BYTES {
            address
        } else {
            usize::from(page) * PAGE_BYTES + address
        }]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    row: usize,
    bit: u8,
}
impl Key {
    pub fn named(name: &str) -> Result<Self> {
        const KEYS: [[&str; 5]; 8] = [
            ["caps-shift", "z", "x", "c", "v"],
            ["a", "s", "d", "f", "g"],
            ["q", "w", "e", "r", "t"],
            ["1", "2", "3", "4", "5"],
            ["0", "9", "8", "7", "6"],
            ["p", "o", "i", "u", "y"],
            ["enter", "l", "k", "j", "h"],
            ["space", "symbol-shift", "m", "n", "b"],
        ];
        if name.len() > 12 {
            return Err(Error::Input);
        }
        for (row, keys) in KEYS.iter().enumerate() {
            for (bit, key) in keys.iter().enumerate() {
                if name.eq_ignore_ascii_case(key) {
                    return Ok(Self { row, bit: 1 << bit });
                }
            }
        }
        Err(Error::Input)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TapeEdge {
    pub t_state: u64,
    pub high: bool,
}

#[derive(Default)]
struct TypingState {
    literal: bool,
    ready: bool,
}

pub struct Board {
    ram: Box<[u8]>,
    model: Model,
    paging: u8,
    ay: ay::Ay,
    firmware: Firmware,
    page: u8,
    firmware_fault: bool,
    last_fetch: u16,
    typing: TypingState,
    keys: [u8; 8],
    tape: Option<Tape>,
    wave: Option<Wave>,
    port_fe: u8,
    clocks: u64,
    frame_clock: u32,
    frames: u64,
    pixel_latch: u8,
    attribute_latch: u8,
    drawing: Box<[u8]>,
    frame: Box<[u8]>,
    audio_clock: u32,
    audio_sum: i64,
    audio_count: u32,
    audio: VecDeque<i16>,
    audio_lost: u64,
    recording: bool,
    signal: VecDeque<TapeEdge>,
    signal_lost: bool,
}
impl Board {
    #[must_use]
    pub fn new(firmware: Firmware) -> Self {
        Self::with_model(firmware, Model::Spectrum48)
    }
    #[must_use]
    pub fn with_model(firmware: Firmware, model: Model) -> Self {
        Self {
            ram: vec![0; if model.banked() { 131_072 } else { 49152 }].into_boxed_slice(),
            model,
            paging: 0,
            ay: ay::Ay::default(),
            firmware,
            page: 0,
            firmware_fault: false,
            last_fetch: 0xffff,
            typing: TypingState::default(),
            keys: [0; 8],
            tape: None,
            wave: None,
            port_fe: 0,
            clocks: 0,
            frame_clock: 0,
            frames: 0,
            pixel_latch: 0,
            attribute_latch: 0,
            drawing: vec![0; WIDTH * HEIGHT * 4].into_boxed_slice(),
            frame: vec![0; WIDTH * HEIGHT * 4].into_boxed_slice(),
            audio_clock: 0,
            audio_sum: 0,
            audio_count: 0,
            audio: VecDeque::new(),
            audio_lost: 0,
            recording: false,
            signal: VecDeque::new(),
            signal_lost: false,
        }
    }
    #[must_use]
    pub fn model(&self) -> Model {
        self.model
    }
    #[must_use]
    pub fn paging(&self) -> u8 {
        self.paging
    }
    fn ram_offset(&self, address: u16) -> usize {
        if !self.model.banked() {
            return usize::from(address - 0x4000);
        }
        let bank = match address >> 14 {
            1 => 5,
            2 => 2,
            _ => usize::from(self.paging & 7),
        };
        bank * 16384 + usize::from(address & 0x3fff)
    }
    fn screen_offset(&self) -> usize {
        if self.model.banked() {
            if self.paging & 8 != 0 {
                7 * 16384
            } else {
                5 * 16384
            }
        } else {
            0
        }
    }
    fn contention(&self, clock: u32) -> u8 {
        model_contention(self.model, clock)
    }
    #[must_use]
    pub fn peek(&self, address: u16) -> u8 {
        if address < 0x4000 {
            self.firmware.read(self.page, address)
        } else {
            self.ram[self.ram_offset(address)]
        }
    }
    pub fn key(&mut self, key: Key, down: bool) {
        if down {
            self.keys[key.row] |= key.bit;
        } else {
            self.keys[key.row] &= !key.bit;
        }
    }
    pub fn release_keys(&mut self) {
        self.keys = [0; 8];
    }
    pub fn mount_tap(&mut self, bytes: &[u8]) -> Result<()> {
        let tape = Tape::parse(bytes)?;
        self.tape = Some(tape);
        self.wave = None;
        Ok(())
    }
    pub fn mount_wav(&mut self, bytes: &[u8]) -> Result<()> {
        let wave = Wave::parse(bytes)?;
        self.wave = Some(wave);
        self.tape = None;
        Ok(())
    }
    pub fn play_cassette(&mut self) {
        if let Some(tape) = &mut self.tape {
            tape.play();
        }
        if let Some(wave) = &mut self.wave {
            wave.play();
        }
    }
    pub fn rewind_cassette(&mut self) {
        if let Some(tape) = &mut self.tape {
            tape.rewind();
        }
        if let Some(wave) = &mut self.wave {
            wave.rewind();
        }
    }
    #[must_use]
    pub fn cassette_pending(&self) -> bool {
        self.tape.as_ref().is_some_and(|t| t.block() < t.blocks())
            || self.wave.as_ref().is_some_and(Wave::pending)
    }
    pub fn tape_mut(&mut self) -> Option<&mut Tape> {
        self.tape.as_mut()
    }
    #[must_use]
    pub fn tape(&self) -> Option<&Tape> {
        self.tape.as_ref()
    }
    pub fn eject_tape(&mut self) {
        self.tape = None;
        self.wave = None;
    }
    #[must_use]
    pub fn frame(&self) -> &[u8] {
        &self.frame
    }
    #[must_use]
    pub fn frames(&self) -> u64 {
        self.frames
    }
    #[must_use]
    pub fn clocks(&self) -> u64 {
        self.clocks
    }
    #[must_use]
    pub fn beeper_high(&self) -> bool {
        self.port_fe & 16 != 0
    }
    #[must_use]
    pub fn firmware_page(&self) -> u8 {
        self.page
    }
    #[must_use]
    pub fn audio_lost(&self) -> u64 {
        self.audio_lost
    }
    pub fn drain_audio(&mut self) -> Vec<i16> {
        self.audio.drain(..).collect()
    }
    pub fn record_tape(&mut self, enabled: bool) {
        self.signal.clear();
        self.signal_lost = false;
        self.recording = enabled;
        if enabled {
            self.signal.push_back(TapeEdge {
                t_state: self.clocks,
                high: self.port_fe & 8 != 0,
            });
        }
    }
    pub fn drain_tape_edges(&mut self) -> Result<Vec<TapeEdge>> {
        if self.signal_lost {
            return Err(Error::Tape(
                "Cassette recording overflow; drain each execution slice",
            ));
        }
        Ok(self.signal.drain(..).collect())
    }
    fn gate(&self) -> bool {
        (0x01f0..=0x01ff).contains(&self.last_fetch)
    }
    /// Marks an owned literal text operation without changing guest RAM or keys.
    pub fn set_literal_typing(&mut self, active: bool) {
        self.typing.literal = active;
    }
    /// Guest acknowledgement used only to pace the BASIC text convenience queue.
    #[must_use]
    pub fn editor_ready(&self) -> bool {
        self.typing.ready
    }
    fn colour(colour: u8, bright: bool) -> [u8; 4] {
        let level = if bright { 255 } else { 205 };
        [
            if colour & 2 != 0 { level } else { 0 },
            if colour & 4 != 0 { level } else { 0 },
            if colour & 1 != 0 { level } else { 0 },
            255,
        ]
    }
    fn raster(&mut self) {
        let view_start = self.model.display_start() - 32 * self.model.line_t_states() - 16;
        if self.frame_clock < view_start {
            return;
        }
        let elapsed = self.frame_clock - view_start;
        let row = elapsed / self.model.line_t_states();
        let column = elapsed % self.model.line_t_states();
        if row >= HEIGHT as u32 || column >= WIDTH as u32 / 2 {
            return;
        }
        for sub in 0..2 {
            let x = column as usize * 2 + sub;
            let y = row as usize;
            let pixel = if (32..288).contains(&x) && (32..224).contains(&y) {
                let sx = x - 32;
                let sy = y - 32;
                let bitmap = ((sy & 0xc0) << 5) | ((sy & 7) << 8) | ((sy & 0x38) << 2) | (sx >> 3);
                if sx.is_multiple_of(8) {
                    self.pixel_latch = self.ram[self.screen_offset() + bitmap];
                    self.attribute_latch =
                        self.ram[self.screen_offset() + 0x1800 + (sy >> 3) * 32 + (sx >> 3)];
                }
                let attr = self.attribute_latch;
                let ink = self.pixel_latch & (0x80 >> (sx & 7)) != 0;
                let invert = attr & 0x80 != 0 && self.frames & 16 != 0;
                let colour = if ink ^ invert {
                    attr & 7
                } else {
                    (attr >> 3) & 7
                };
                Self::colour(colour, attr & 0x40 != 0)
            } else {
                Self::colour(self.port_fe & 7, false)
            };
            let offset = (y * WIDTH + x) * 4;
            self.drawing[offset..offset + 4].copy_from_slice(&pixel);
        }
    }
    fn clock(&mut self) {
        self.raster();
        self.audio_clock += AUDIO_HZ;
        if self.model.banked() {
            self.ay.tick();
        }
        self.audio_sum += i64::from(
            (if self.port_fe & 16 != 0 { 8192 } else { -8192 })
                + if self.model.banked() {
                    self.ay.sample()
                } else {
                    0
                },
        );
        self.audio_count += 1;
        if self.audio_clock >= self.model.cpu_hz() {
            self.audio_clock -= self.model.cpu_hz();
            if self.audio.len() >= AUDIO_CAPACITY {
                self.audio.pop_front();
                self.audio_lost += 1;
            }
            self.audio
                .push_back((self.audio_sum / i64::from(self.audio_count)) as i16);
            self.audio_sum = 0;
            self.audio_count = 0;
        }
        if let Some(tape) = &mut self.tape {
            tape.advance(1);
        }
        if let Some(wave) = &mut self.wave {
            wave.advance_at(1, self.model.cpu_hz());
        }
        self.clocks = self.clocks.wrapping_add(1);
        self.frame_clock += 1;
        if self.frame_clock == self.model.frame_t_states() {
            self.frame_clock = 0;
            self.frames = self.frames.wrapping_add(1);
            self.frame.copy_from_slice(&self.drawing);
        }
    }
    fn floating_bus(&self) -> u8 {
        let Some(elapsed) = self.frame_clock.checked_sub(self.model.display_start()) else {
            return 255;
        };
        let y = elapsed / self.model.line_t_states();
        let x = elapsed % self.model.line_t_states();
        if y >= 192 || x >= 128 || x & 7 >= 4 {
            return 255;
        }
        let column = ((x / 8) * 2 + (x & 3) / 2) as usize;
        let y = y as usize;
        if x & 1 == 0 {
            let offset = ((y & 0xc0) << 5) | ((y & 7) << 8) | ((y & 0x38) << 2) | column;
            self.ram[self.screen_offset() + offset]
        } else {
            self.ram[self.screen_offset() + 0x1800 + (y >> 3) * 32 + column]
        }
    }
}

#[must_use]
pub fn contention(clock: u32) -> u8 {
    model_contention(Model::Spectrum48, clock)
}
fn model_contention(model: Model, clock: u32) -> u8 {
    let Some(elapsed) = (clock % model.frame_t_states()).checked_sub(model.contention_start())
    else {
        return 0;
    };
    if elapsed / model.line_t_states() >= 192 || elapsed % model.line_t_states() >= 128 {
        return 0;
    }
    [6, 5, 4, 3, 2, 1, 0, 0][((elapsed % model.line_t_states()) & 7) as usize]
}
impl Bus for Board {
    fn read(&mut self, address: u16) -> u8 {
        self.peek(address)
    }
    fn fetch(&mut self, address: u16) -> u8 {
        self.last_fetch = address;
        self.peek(address)
    }
    fn write(&mut self, address: u16, value: u8) {
        if address >= 0x4000 {
            let offset = self.ram_offset(address);
            self.ram[offset] = value;
        }
    }
    fn input(&mut self, port: u16) -> u8 {
        if port == 0xfdfb && self.page == 16 && (0x2000..0x3a00).contains(&self.last_fetch) {
            return u8::from(self.typing.literal);
        }
        if port == PRIVATE_PAGE_PORT && self.gate() {
            return self.page;
        }
        if port == 0xfdf9 && self.gate() {
            return self.model.identity();
        }
        if port == 0xfdf7 && self.gate() {
            return self.paging;
        }
        if self.model.banked() && port & 0xc002 == 0xc000 {
            return self.ay.read();
        }
        if port & 1 != 0 {
            return self.floating_bus();
        }
        let mut keys = 31;
        for (row, held) in self.keys.iter().enumerate() {
            if port & (0x100 << row) == 0 {
                keys &= !held;
            }
        }
        0xa0 | keys
            | if self.tape.as_ref().is_some_and(Tape::high)
                || self.wave.as_ref().is_some_and(Wave::high)
            {
                0x40
            } else {
                0
            }
    }
    fn output(&mut self, port: u16, value: u8) {
        if port == 0xfdfb
            && self.page == 16
            && (0x2000..0x3a00).contains(&self.last_fetch)
            && value <= 1
        {
            self.typing.ready = value == 1;
        }
        if port == PRIVATE_PAGE_PORT && self.gate() {
            if usize::from(value) >= self.firmware.pages() {
                self.firmware_fault = true;
            } else {
                self.page = value;
            }
            return;
        }
        if self.model.banked() {
            if port & 0x8002 == 0 && self.paging & 32 == 0 {
                self.paging = value;
            }
            match port & 0xc002 {
                0xc000 => self.ay.select(value),
                0x8000 => self.ay.write(value),
                _ => {}
            }
        }
        if port & 1 == 0 {
            if self.recording && (value ^ self.port_fe) & 8 != 0 {
                if self.signal.len() >= SIGNAL_CAPACITY {
                    self.signal_lost = true;
                } else {
                    self.signal.push_back(TapeEdge {
                        t_state: self.clocks,
                        high: value & 8 != 0,
                    });
                }
            }
            self.port_fe = value;
        }
    }
    fn tick(&mut self, t_states: u32) {
        for _ in 0..t_states {
            self.clock();
        }
    }
    fn irq(&self) -> bool {
        self.frame_clock < if self.model.banked() { 36 } else { 32 }
    }
    fn wait_states(&self, address: u16, access: Access) -> u8 {
        let high_contended = (0x4000..0x8000).contains(&address)
            || (self.model.banked() && address >= 0xc000 && self.paging & 1 != 0);
        if !matches!(access, Access::Input | Access::Output) {
            return if high_contended {
                self.contention(self.frame_clock)
            } else {
                0
            };
        }
        let mut elapsed = 0_u32;
        let mut waits = 0;
        let phases: &[(bool, u32)] = if address & 1 == 0 {
            &[(high_contended, 1), (true, 3)]
        } else if high_contended {
            &[(true, 1), (true, 1), (true, 1), (true, 1)]
        } else {
            &[(false, 4)]
        };
        for (contended, length) in phases {
            let delay = if *contended {
                self.contention(self.frame_clock + elapsed)
            } else {
                0
            };
            waits += delay;
            elapsed += u32::from(delay) + length;
        }
        waits
    }
}

pub struct Machine {
    pub cpu: Cpu,
    pub board: Board,
}
impl Machine {
    pub fn development() -> Result<Self> {
        Self::development_model(Model::Spectrum48)
    }
    pub fn development_model(model: Model) -> Result<Self> {
        Ok(Self::with_model(
            Firmware::new(DEVELOPMENT_FIRMWARE)?,
            model,
        ))
    }
    #[must_use]
    pub fn new(firmware: Firmware) -> Self {
        Self::with_model(firmware, Model::Spectrum48)
    }
    #[must_use]
    pub fn with_model(firmware: Firmware, model: Model) -> Self {
        let mut cpu = Cpu::default();
        cpu.reset();
        Self {
            cpu,
            board: Board::with_model(firmware, model),
        }
    }
    pub fn step(&mut self) -> Result<u32> {
        if self.board.firmware_fault {
            return Err(Error::FirmwarePage);
        }
        let elapsed = self.cpu.step(&mut self.board).map_err(Error::Cpu)?;
        if self.board.firmware_fault {
            return Err(Error::FirmwarePage);
        }
        Ok(elapsed)
    }
    pub fn run(&mut self, budget: u32) -> Result<u32> {
        if budget > MAX_SLICE {
            return Err(Error::Slice);
        }
        let mut elapsed = 0;
        while elapsed < budget {
            elapsed += self.step()?;
        }
        Ok(elapsed)
    }
    pub fn reset(&mut self) {
        self.cpu.reset();
        self.board.page = 0;
        self.board.paging = 0;
        self.board.ay = ay::Ay::default();
        self.board.firmware_fault = false;
        self.board.last_fetch = 0xffff;
        self.board.typing = TypingState::default();
        self.board.release_keys();
        self.board.record_tape(false);
        self.board.audio.clear();
        self.board.audio_sum = 0;
        self.board.audio_count = 0;
        self.board.audio_clock = 0;
        self.board.port_fe = 0;
    }
}
