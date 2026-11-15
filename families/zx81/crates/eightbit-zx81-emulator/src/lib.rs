//! Original ZX80/ZX81 digital board. BASIC executes only in guest Z80 firmware.
#![allow(missing_docs)]
pub mod image;
mod model;
pub mod recording;
pub mod tape;
pub mod wave;
use eightbit_z80_core::{Bus, Cpu, PrefixLimit};
pub use model::Model;
use tape::Tape;
use wave::Wave;

pub const CPU_HZ: u32 = 3_250_000;
pub const LINE_T_STATES: u32 = 207;
pub const FRAME_T_STATES: u32 = LINE_T_STATES * 312;
pub const MAX_SLICE: u32 = CPU_HZ / 10;
pub const WIDTH: usize = 320;
pub const HEIGHT: usize = 256;
pub const DEVELOPMENT_FIRMWARE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/zx81.rom"));
pub const DEVELOPMENT_MANIFEST: &str =
    include_str!(concat!(env!("OUT_DIR"), "/zx81-manifest.json"));
pub const ZX80_DEVELOPMENT_FIRMWARE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/zx80.rom"));
pub const ZX80_DEVELOPMENT_MANIFEST: &str =
    include_str!(concat!(env!("OUT_DIR"), "/zx80-manifest.json"));
const PAGE_BYTES: usize = 8192;
const MAX_PAGES: usize = 128;
const SIGNAL_CAPACITY: usize = 2_500_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Firmware,
    Page,
    Slice,
    Key,
    Tape(&'static str),
    Cpu(PrefixLimit),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Firmware => f.write_str("ZXMEMU1001: invalid original firmware container"),
            Self::Page => f.write_str("ZXMEMU1002: firmware selected an unavailable page"),
            Self::Slice => f.write_str("ZXMEMU1003: execution slice exceeds its limit"),
            Self::Key => f.write_str("ZXMEMU1004: unsupported keyboard key"),
            Self::Tape(message) => write!(f, "ZXMEMU1005: {message}"),
            Self::Cpu(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

pub struct Firmware {
    bytes: Box<[u8]>,
    expanded: bool,
}
impl Firmware {
    pub fn original(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < PAGE_BYTES * 2
            || bytes.len() > PAGE_BYTES * (MAX_PAGES + 1)
            || !bytes.len().is_multiple_of(PAGE_BYTES)
        {
            return Err(Error::Firmware);
        }
        Ok(Self {
            bytes: bytes.into(),
            expanded: true,
        })
    }
    /// Explicit opaque reference input; no private ports or host services apply.
    pub fn reference(bytes: &[u8], model: Model) -> Result<Self> {
        if bytes.len() != model.rom_bytes() {
            return Err(Error::Firmware);
        }
        Ok(Self {
            bytes: bytes.into(),
            expanded: false,
        })
    }
    fn read(&self, model: Model, page: u8, address: u16) -> u8 {
        let address = usize::from(address & 0x3fff);
        let offset = if self.expanded && page != 0 && address >= PAGE_BYTES {
            usize::from(page) * PAGE_BYTES + address - PAGE_BYTES
        } else {
            address & (model.rom_bytes() - 1)
        };
        self.bytes[offset]
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TapeEdge {
    pub t_state: u64,
    pub high: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    row: usize,
    bit: u8,
}
impl Key {
    pub fn named(name: &str) -> Result<Self> {
        const KEYS: [[&str; 5]; 8] = [
            ["shift", "z", "x", "c", "v"],
            ["a", "s", "d", "f", "g"],
            ["q", "w", "e", "r", "t"],
            ["1", "2", "3", "4", "5"],
            ["0", "9", "8", "7", "6"],
            ["p", "o", "i", "u", "y"],
            ["enter", "l", "k", "j", "h"],
            ["space", ".", "m", "n", "b"],
        ];
        if name.len() > 12 {
            return Err(Error::Key);
        }
        let name = if name.eq_ignore_ascii_case("caps-shift") {
            "shift"
        } else {
            name
        };
        for (row, keys) in KEYS.iter().enumerate() {
            for (bit, key) in keys.iter().enumerate() {
                if name.eq_ignore_ascii_case(key) {
                    return Ok(Self { row, bit: 1 << bit });
                }
            }
        }
        Err(Error::Key)
    }
}

enum Cassette {
    Image(Tape),
    Wave(Wave),
}
// Physical control lines are independent, rather than exclusive lifecycle states.
#[allow(clippy::struct_excessive_bools)]
pub struct Board {
    model: Model,
    firmware: Firmware,
    ram: Box<[u8]>,
    page: u8,
    fault: bool,
    last_fetch: u16,
    keys: [u8; 8],
    clocks: u64,
    line_clock: u32,
    nmi_enabled: bool,
    irq: bool,
    vsync: bool,
    lcntr: u8,
    row: usize,
    column: usize,
    pending: Option<u8>,
    drawing: bool,
    video_seen: u64,
    frame_number: u64,
    pixels: Vec<u8>,
    ear: bool,
    cassette: Option<Cassette>,
    recording: bool,
    signal: Vec<TapeEdge>,
    dropped: bool,
    literal: bool,
    typing_ready: bool,
}
impl Board {
    #[must_use]
    pub fn new(model: Model, firmware: Firmware) -> Self {
        Self {
            model,
            firmware,
            ram: vec![0; model.ram_bytes()].into_boxed_slice(),
            page: 0,
            fault: false,
            last_fetch: 0,
            keys: [31; 8],
            clocks: 0,
            line_clock: 0,
            nmi_enabled: false,
            irq: false,
            vsync: false,
            lcntr: 0,
            row: 0,
            column: 0,
            pending: None,
            drawing: false,
            video_seen: 0,
            frame_number: 0,
            pixels: vec![255; WIDTH * HEIGHT * 4],
            ear: false,
            cassette: None,
            recording: false,
            signal: Vec::new(),
            dropped: false,
            literal: false,
            typing_ready: false,
        }
    }
    #[must_use]
    pub const fn model(&self) -> Model {
        self.model
    }
    #[must_use]
    pub const fn clocks(&self) -> u64 {
        self.clocks
    }
    #[must_use]
    pub const fn firmware_page(&self) -> u8 {
        self.page
    }
    #[must_use]
    pub fn peek(&self, address: u16) -> u8 {
        if address & 0x4000 != 0 {
            self.ram[usize::from(address) & (self.ram.len() - 1)]
        } else {
            self.firmware.read(self.model, self.page, address)
        }
    }
    #[must_use]
    pub fn frame(&self) -> &[u8] {
        &self.pixels
    }
    #[must_use]
    pub const fn frame_number(&self) -> u64 {
        self.frame_number
    }
    pub fn set_key(&mut self, key: Key, pressed: bool) {
        if pressed {
            self.keys[key.row] &= !key.bit;
        } else {
            self.keys[key.row] |= key.bit;
        }
    }
    pub fn release_keys(&mut self) {
        self.keys = [31; 8];
    }
    pub fn set_literal_typing(&mut self, active: bool) {
        self.literal = active;
    }
    #[must_use]
    pub const fn typing_ready(&self) -> bool {
        self.typing_ready
    }
    pub fn set_ear(&mut self, high: bool) {
        self.ear = high;
    }
    pub fn mount_image(&mut self, bytes: &[u8], name: &str) -> Result<()> {
        let tape = Tape::parse(self.model, bytes, name)?;
        self.cassette = Some(Cassette::Image(tape));
        self.ear = false;
        Ok(())
    }
    pub fn mount_wave(&mut self, bytes: &[u8]) -> Result<()> {
        let wave = Wave::parse(bytes)?;
        self.cassette = Some(Cassette::Wave(wave));
        self.ear = false;
        Ok(())
    }
    pub fn play_tape(&mut self) {
        match &mut self.cassette {
            Some(Cassette::Image(tape)) => tape.play(),
            Some(Cassette::Wave(wave)) => wave.play(),
            None => {}
        }
    }
    pub fn rewind_tape(&mut self) {
        match &mut self.cassette {
            Some(Cassette::Image(tape)) => tape.rewind(),
            Some(Cassette::Wave(wave)) => wave.rewind(),
            None => {}
        }
        self.ear = false;
    }
    pub fn eject_tape(&mut self) {
        self.cassette = None;
        self.ear = false;
    }
    #[must_use]
    pub fn tape_pending(&self) -> bool {
        match &self.cassette {
            Some(Cassette::Image(tape)) => tape.pending(),
            Some(Cassette::Wave(wave)) => wave.pending(),
            None => false,
        }
    }
    pub fn record_tape(&mut self, active: bool) {
        self.recording = active;
        self.signal.clear();
        if active {
            self.signal.push(TapeEdge {
                t_state: self.clocks,
                high: self.vsync,
            });
        }
        self.dropped = false;
    }
    pub fn drain_tape_edges(&mut self) -> Result<Vec<TapeEdge>> {
        if self.dropped {
            return Err(Error::Tape("Cassette transition limit exceeded"));
        }
        Ok(std::mem::take(&mut self.signal))
    }
    fn sync(&mut self, high: bool) {
        if self.vsync == high {
            return;
        }
        self.vsync = high;
        if self.recording {
            if let Some(last) = self
                .signal
                .last_mut()
                .filter(|edge| edge.t_state == self.clocks)
            {
                last.high = high;
            } else if self.signal.len() == SIGNAL_CAPACITY {
                self.dropped = true;
            } else {
                self.signal.push(TapeEdge {
                    t_state: self.clocks,
                    high,
                });
            }
        }
        if high {
            self.row = 0;
            self.column = 0;
            self.lcntr = 0;
            self.drawing = false;
        }
    }
    fn hsync(&mut self) {
        if self.drawing {
            self.row = (self.row + 1).min(HEIGHT);
        }
        self.column = 0;
        self.lcntr = self.lcntr.wrapping_add(1) & 7;
        self.drawing = false;
    }
    fn render(&mut self, refresh: u16, character: u8) {
        self.video_seen = self.clocks;
        self.drawing = true;
        let glyph = (refresh & 0xfe00) | (u16::from(character & 63) << 3) | u16::from(self.lcntr);
        let pattern = self.peek(glyph) ^ if character & 128 != 0 { 255 } else { 0 };
        if self.row < 192 && self.column < 32 {
            let x = 32 + self.column * 8;
            let y = 32 + self.row;
            for bit in 0..8 {
                let level = if pattern & (128 >> bit) != 0 { 0 } else { 255 };
                let pixel = (y * WIDTH + x + bit) * 4;
                self.pixels[pixel..pixel + 4].copy_from_slice(&[level, level, level, 255]);
            }
        }
        self.column = self.column.saturating_add(1);
    }
}
impl Bus for Board {
    fn read(&mut self, address: u16) -> u8 {
        self.peek(address)
    }
    fn write(&mut self, address: u16, value: u8) {
        if address & 0x4000 != 0 {
            let offset = usize::from(address) & (self.ram.len() - 1);
            self.ram[offset] = value;
        }
    }
    fn fetch(&mut self, address: u16) -> u8 {
        self.last_fetch = address;
        let value = self.peek(address);
        if address & 0x8000 != 0 && value & 64 == 0 {
            self.pending = Some(value);
            0
        } else {
            self.pending = None;
            value
        }
    }
    fn refresh(&mut self, address: u16) {
        self.irq = address & 64 == 0;
        if let Some(character) = self.pending.take() {
            self.render(address, character);
        }
    }
    fn input(&mut self, port: u16) -> u8 {
        if self.firmware.expanded && (0x1e0..0x200).contains(&self.last_fetch) {
            match port {
                0xfdfd => return self.page,
                0xfdf9 => return u8::from(self.model.zx80()),
                0xfdf7 => {
                    return if self.model.ram_bytes() == 1024 {
                        1
                    } else {
                        16
                    };
                }
                0xfdfb => return u8::from(self.literal),
                _ => {}
            }
        }
        if port & 1 == 0 {
            self.sync(true);
            let mut value = 31;
            for (row, keys) in self.keys.iter().enumerate() {
                if port & (1 << (row + 8)) == 0 {
                    value &= keys;
                }
            }
            value | 0x60 | if self.ear { 128 } else { 0 }
        } else {
            255
        }
    }
    fn output(&mut self, port: u16, value: u8) {
        if self.firmware.expanded && (0x1e0..0x200).contains(&self.last_fetch) {
            if port == 0xfdfd {
                if usize::from(value) >= self.firmware.bytes.len() / PAGE_BYTES {
                    self.fault = true;
                } else {
                    self.page = value;
                }
                return;
            }
            if port == 0xfdfb {
                self.typing_ready = value != 0;
                return;
            }
        }
        if !self.model.zx80() {
            if port & 2 == 0 {
                self.nmi_enabled = false;
            }
            if port & 1 == 0 {
                self.nmi_enabled = true;
            }
        }
        self.sync(false);
        if self.model.zx80() {
            self.hsync();
        }
    }
    fn tick(&mut self, t_states: u32) {
        for _ in 0..t_states {
            self.clocks += 1;
            match &mut self.cassette {
                Some(Cassette::Image(tape)) => {
                    tape.advance(1);
                    self.ear = tape.high();
                }
                Some(Cassette::Wave(wave)) => {
                    wave.advance(1);
                    self.ear = wave.high();
                }
                None => {}
            }
            self.line_clock += 1;
            if self.line_clock == LINE_T_STATES {
                self.line_clock = 0;
                if !self.model.zx80() {
                    self.hsync();
                }
            }
            if self.clocks.is_multiple_of(u64::from(FRAME_T_STATES)) {
                self.frame_number += 1;
                if self.clocks - self.video_seen > u64::from(FRAME_T_STATES) {
                    self.pixels.fill(255);
                }
            }
        }
    }
    fn irq(&self) -> bool {
        self.irq
    }
    fn nmi(&self) -> bool {
        self.nmi_enabled && self.line_clock == 0
    }
    fn interrupt_acknowledge(&mut self) -> u8 {
        self.irq = false;
        255
    }
}
pub struct Machine {
    pub cpu: Cpu,
    pub board: Board,
}
impl Machine {
    pub fn development(model: Model) -> Result<Self> {
        Ok(Self::new(
            model,
            Firmware::original(if model.zx80() {
                ZX80_DEVELOPMENT_FIRMWARE
            } else {
                DEVELOPMENT_FIRMWARE
            })?,
        ))
    }
    #[must_use]
    pub fn new(model: Model, firmware: Firmware) -> Self {
        let mut cpu = Cpu::default();
        cpu.reset();
        Self {
            cpu,
            board: Board::new(model, firmware),
        }
    }
    pub fn reset(&mut self) {
        self.cpu.reset();
        self.board.page = 0;
        self.board.fault = false;
        self.board.nmi_enabled = false;
        self.board.vsync = false;
        self.board.irq = false;
        self.board.pending = None;
        self.board.literal = false;
        self.board.typing_ready = false;
        self.board.release_keys();
        self.board.record_tape(false);
        self.board.rewind_tape();
        self.board.pixels.fill(255);
    }
    pub fn run(&mut self, cycles: u32) -> Result<u32> {
        if cycles == 0 || cycles > MAX_SLICE {
            return Err(Error::Slice);
        }
        let mut elapsed = 0;
        while elapsed < cycles {
            elapsed += self.cpu.step(&mut self.board).map_err(Error::Cpu)?;
            if self.board.fault {
                return Err(Error::Page);
            }
        }
        Ok(elapsed)
    }
}
