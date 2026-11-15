//! Bounded digital VIA/PSG and idealised vector integrators, authored from register facts.
//! This deliberately does not model CRT phosphor, analogue drift or peripherals.
use crate::{Result, error};
use eightbit_m6809_core::{Bus, Cpu, InterruptLines};
pub const CLOCK_HZ: u32 = 1_500_000;
pub const FRAME_CYCLES: u32 = 30_000;
pub const SAMPLE_RATE: u32 = 44_100;
pub const WIDTH: usize = 256;
pub const HEIGHT: usize = 320;
const MAX_AUDIO: usize = 8192;

#[derive(Clone, Default)]
pub struct Controller {
    pub x: i8,
    pub y: i8,
    pub buttons: u8,
}
#[derive(Default)]
struct Timer {
    latch: u16,
    count: u16,
    active: bool,
}
#[derive(Default)]
struct Psg {
    registers: [u8; 16],
    address: u8,
    tone: [u32; 3],
    high: [bool; 3],
    noise_ticks: u32,
    noise: u32,
    envelope_ticks: u32,
    envelope: u8,
    envelope_up: bool,
    envelope_holding: bool,
}
impl Psg {
    fn write(&mut self, value: u8) {
        let i = usize::from(self.address & 15);
        let mask = match i {
            1 | 3 | 5 | 13 => 15,
            6 | 8..=10 => 31,
            _ => 255,
        };
        self.registers[i] = value & mask;
        if i == 13 {
            self.envelope_up = value & 4 != 0;
            self.envelope = if self.envelope_up { 0 } else { 15 };
            self.envelope_holding = false;
            self.envelope_ticks = 0;
        }
    }
    fn tick(&mut self) {
        for i in 0..3 {
            self.tone[i] += 1;
            let period = u32::from(u16::from_le_bytes([
                self.registers[i * 2],
                self.registers[i * 2 + 1],
            ]))
            .max(1)
                * 8;
            if self.tone[i] >= period {
                self.tone[i] = 0;
                self.high[i] = !self.high[i];
            }
        }
        self.noise_ticks += 1;
        if self.noise_ticks >= u32::from(self.registers[6]).max(1) * 16 {
            self.noise_ticks = 0;
            if self.noise == 0 {
                self.noise = 1;
            }
            self.noise = (self.noise >> 1) | (((self.noise ^ (self.noise >> 3)) & 1) << 16);
        }
        self.envelope_ticks += 1;
        let period =
            u32::from(u16::from_le_bytes([self.registers[11], self.registers[12]])).max(1) * 256;
        if self.envelope_ticks >= period {
            self.envelope_ticks = 0;
            if !self.envelope_holding {
                if (self.envelope_up && self.envelope == 15)
                    || (!self.envelope_up && self.envelope == 0)
                {
                    let shape = self.registers[13];
                    if shape & 8 == 0 {
                        self.envelope = 0;
                        self.envelope_holding = true;
                    } else if shape & 1 != 0 {
                        if shape & 2 != 0 {
                            self.envelope = 15 - self.envelope;
                        }
                        self.envelope_holding = true;
                    } else if shape & 2 != 0 {
                        self.envelope_up = !self.envelope_up;
                    } else {
                        self.envelope = if self.envelope_up { 0 } else { 15 };
                    }
                } else if self.envelope_up {
                    self.envelope += 1;
                } else {
                    self.envelope -= 1;
                }
            }
        }
    }
    fn sample(&self) -> i16 {
        let mut sample = 0_i16;
        for i in 0..3 {
            let amp = self.registers[8 + i];
            let volume = i16::from(if amp & 16 != 0 {
                self.envelope
            } else {
                amp & 15
            });
            let enabled = (self.registers[7] & (1 << i) != 0 || self.high[i])
                && (self.registers[7] & (8 << i) != 0 || self.noise & 1 != 0);
            let level = volume * volume * 9;
            sample += if enabled { level } else { -level };
        }
        sample
    }
}
pub struct Board {
    cartridge: Vec<u8>,
    firmware: Vec<u8>,
    ram: [u8; 1024],
    registers: [u8; 16],
    t1: Timer,
    t2: Timer,
    ifr: u8,
    ier: u8,
    pb7: bool,
    shift: u8,
    shift_left: u8,
    shift_clock: u16,
    cb2: bool,
    psg: Psg,
    pub controllers: [Controller; 2],
    x: i32,
    y: i32,
    held_y: i32,
    reference: i32,
    intensity: u8,
    frame_clock: u32,
    sample_clock: u32,
    pixels: Vec<u8>,
    drawing: Vec<u8>,
    audio: Vec<i16>,
    pub cycles: u64,
}
impl Board {
    pub fn new(cartridge: Vec<u8>, firmware: Vec<u8>) -> Result<Self> {
        crate::cartridge::validate_cartridge(&cartridge)?;
        if firmware.len() != 8192 {
            return Err(error("VXO1000", "Invalid original executive image"));
        }
        Ok(Self {
            cartridge,
            firmware,
            ram: [0; 1024],
            registers: [0; 16],
            t1: Timer::default(),
            t2: Timer::default(),
            ifr: 0,
            ier: 0,
            pb7: true,
            shift: 0,
            shift_left: 0,
            shift_clock: 0,
            cb2: false,
            psg: Psg::default(),
            controllers: std::array::from_fn(|_| Controller::default()),
            x: 0,
            y: 0,
            held_y: 0,
            reference: 0,
            intensity: 0,
            frame_clock: 0,
            sample_clock: 0,
            pixels: vec![0; WIDTH * HEIGHT],
            drawing: vec![0; WIDTH * HEIGHT],
            audio: Vec::new(),
            cycles: 0,
        })
    }
    #[must_use]
    pub fn peek(&self, address: u16) -> u8 {
        match address {
            0..=0x7fff => self
                .cartridge
                .get(usize::from(address))
                .copied()
                .unwrap_or(255),
            0xc800..=0xcfff => self.ram[usize::from(address & 1023)],
            0xe000..=0xffff => self.firmware[usize::from(address - 0xe000)],
            _ => 255,
        }
    }
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
    pub fn take_audio(&mut self) -> Vec<i16> {
        std::mem::take(&mut self.audio)
    }
    fn port_b(&self) -> u8 {
        let b = self.registers[0];
        let select = usize::from((b >> 1) & 3);
        let axis = match select {
            0 => self.controllers[0].x,
            1 => self.controllers[0].y,
            2 => self.controllers[1].x,
            _ => self.controllers[1].y,
        };
        let compare = if axis > i8::from_be_bytes([self.registers[1]]) {
            32
        } else {
            0
        };
        let mut result = (b & self.registers[2]) | (compare & !self.registers[2]);
        if self.registers[11] & 128 != 0 && self.registers[2] & 128 != 0 {
            result = (result & 127) | if self.pb7 { 128 } else { 0 };
        }
        result
    }
    fn port_a(&self) -> u8 {
        let input = if self.registers[0] & self.registers[2] & 24 == 8 {
            if self.psg.address == 14 {
                !(self.controllers[0].buttons & 15 | (self.controllers[1].buttons & 15) << 4)
            } else {
                self.psg.registers[usize::from(self.psg.address & 15)]
            }
        } else {
            255
        };
        (self.registers[1] & self.registers[3]) | (input & !self.registers[3])
    }
    fn outputs(&mut self) {
        let b = self.port_b();
        let a = self.port_a();
        match b & 24 {
            24 => self.psg.address = a & 15,
            16 => self.psg.write(a),
            _ => {}
        }
        if b & 1 == 0 {
            let value = i32::from(i8::from_be_bytes([a]));
            match (b >> 1) & 3 {
                0 => self.held_y = value,
                1 => self.reference = value,
                2 => self.intensity = a & 127,
                _ => {}
            }
        }
    }
    fn via_read(&mut self, r: usize) -> u8 {
        match r {
            0 => self.port_b(),
            1 => {
                self.ifr &= !3;
                self.port_a()
            }
            15 => self.port_a(),
            4 => {
                self.ifr &= !64;
                self.t1.count.to_le_bytes()[0]
            }
            5 => self.t1.count.to_le_bytes()[1],
            6 => self.t1.latch.to_le_bytes()[0],
            7 => self.t1.latch.to_le_bytes()[1],
            8 => {
                self.ifr &= !32;
                self.t2.count.to_le_bytes()[0]
            }
            9 => self.t2.count.to_le_bytes()[1],
            10 => {
                self.ifr &= !4;
                self.shift
            }
            13 => {
                self.ifr
                    | if self.ifr & self.ier & 127 != 0 {
                        128
                    } else {
                        0
                    }
            }
            14 => self.ier | 128,
            _ => self.registers[r],
        }
    }
    fn via_write(&mut self, r: usize, value: u8) {
        self.registers[r] = value;
        match r {
            0 | 1 | 2 | 3 | 15 => {
                if r == 15 {
                    self.registers[1] = value;
                }
                if r == 1 {
                    self.ifr &= !3;
                }
                self.outputs();
            }
            4 | 6 => self.t1.latch = u16::from_le_bytes([value, self.t1.latch.to_le_bytes()[1]]),
            5 => {
                self.t1.latch = u16::from_le_bytes([self.t1.latch.to_le_bytes()[0], value]);
                self.t1.count = self.t1.latch;
                self.t1.active = true;
                self.pb7 = false;
                self.ifr &= !64;
            }
            7 => {
                self.t1.latch = u16::from_le_bytes([self.t1.latch.to_le_bytes()[0], value]);
                self.ifr &= !64;
            }
            8 => self.t2.latch = u16::from_le_bytes([value, self.t2.latch.to_le_bytes()[1]]),
            9 => {
                self.t2.latch = u16::from_le_bytes([self.t2.latch.to_le_bytes()[0], value]);
                self.t2.count = self.t2.latch;
                self.t2.active = true;
                self.ifr &= !32;
            }
            10 => {
                self.shift = value;
                self.shift_left = 8;
                self.shift_clock = 0;
                self.cb2 = value & 128 != 0;
                self.ifr &= !4;
            }
            13 => self.ifr &= !value,
            14 => {
                if value & 128 != 0 {
                    self.ier |= value & 127;
                } else {
                    self.ier &= !value;
                }
            }
            _ => {}
        }
    }
    fn tick_one(&mut self) {
        self.cycles = self.cycles.saturating_add(1);
        let acr = self.registers[11];
        self.t1.count = self.t1.count.wrapping_sub(1);
        if self.t1.count == u16::MAX && self.t1.active {
            self.ifr |= 64;
            if acr & 64 != 0 {
                self.t1.count = self.t1.latch;
                self.pb7 = !self.pb7;
            } else {
                self.t1.active = false;
                self.pb7 = true;
            }
        }
        if acr & 32 == 0 {
            self.t2.count = self.t2.count.wrapping_sub(1);
            if self.t2.count == u16::MAX && self.t2.active {
                self.ifr |= 32;
                self.t2.active = false;
            }
        }
        let shift_mode = (acr >> 2) & 7;
        if matches!(shift_mode, 4..=6) && self.shift_left > 0 {
            self.shift_clock = self.shift_clock.wrapping_add(1);
            let divisor = if shift_mode == 6 {
                2
            } else {
                u16::from(self.registers[8]) + 2
            };
            if self.shift_clock >= divisor {
                self.shift_clock = 0;
                self.cb2 = self.shift & 128 != 0;
                self.shift = self.shift.wrapping_shl(1) | u8::from(shift_mode == 4 && self.cb2);
                self.shift_left -= 1;
                if self.shift_left == 0 {
                    self.ifr |= 4;
                    if shift_mode == 4 {
                        self.shift_left = 8;
                    }
                }
            }
        }
        let pcr = self.registers[12];
        if (pcr >> 1) & 7 == 6 {
            self.x = 0;
            self.y = 0;
        } else if self.port_b() & 128 == 0 {
            self.x = (self.x + i32::from(i8::from_be_bytes([self.registers[1]])) - self.reference)
                .clamp(-1_048_576, 1_048_576);
            self.y = (self.y + self.held_y - self.reference).clamp(-1_048_576, 1_048_576);
            let bright = match (pcr >> 5) & 7 {
                6 => false,
                7 => true,
                _ => self.cb2,
            };
            if bright && self.intensity > 0 {
                let x = 128 + self.x / 128;
                let y = 160 - self.y / 128;
                if let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y))
                    && x < WIDTH
                    && y < HEIGHT
                {
                    self.drawing[y * WIDTH + x] = self.intensity.saturating_mul(2);
                }
            }
        }
        self.psg.tick();
        self.sample_clock += SAMPLE_RATE;
        if self.sample_clock >= CLOCK_HZ {
            self.sample_clock -= CLOCK_HZ;
            if self.audio.len() < MAX_AUDIO {
                self.audio.push(self.psg.sample());
            }
        }
        self.frame_clock += 1;
        if self.frame_clock == FRAME_CYCLES {
            self.frame_clock = 0;
            std::mem::swap(&mut self.pixels, &mut self.drawing);
            self.drawing.fill(0);
        }
    }
}
impl Bus for Board {
    fn read(&mut self, address: u16) -> u8 {
        if (0xd000..=0xdfff).contains(&address) {
            self.via_read(usize::from(address & 15))
        } else {
            self.peek(address)
        }
    }
    fn write(&mut self, address: u16, value: u8) {
        match address {
            0xc800..=0xcfff => self.ram[usize::from(address & 1023)] = value,
            0xd000..=0xdfff => self.via_write(usize::from(address & 15), value),
            _ => {}
        }
    }
    fn tick(&mut self, cycles: u32) {
        for _ in 0..cycles {
            self.tick_one();
        }
    }
    fn interrupts(&self) -> InterruptLines {
        InterruptLines {
            irq: self.ifr & self.ier & 127 != 0,
            ..InterruptLines::default()
        }
    }
}
pub struct Machine {
    pub cpu: Cpu,
    pub board: Board,
}
impl Machine {
    pub fn new(cartridge: Vec<u8>) -> Result<Self> {
        let board = Board::new(cartridge, crate::firmware::image()?)?;
        let mut machine = Self {
            cpu: Cpu::default(),
            board,
        };
        machine.cpu.reset(&mut machine.board);
        Ok(machine)
    }
    pub fn advance(&mut self, cycles: u32, mut cancel: impl FnMut() -> bool) -> Result<()> {
        if cycles == 0 || cycles > FRAME_CYCLES * 4 {
            return Err(error("VXO1000", "Advance must be 1–120000 cycles"));
        }
        let end = self.board.cycles.saturating_add(u64::from(cycles));
        while self.board.cycles < end {
            if cancel() {
                return Err(error("VXO1002", "Original Vectrex execution cancelled"));
            }
            self.cpu.step(&mut self.board).map_err(|_| {
                error(
                    "VXO1001",
                    "Unsupported Vectrex instruction or executive service; reset or use VecX",
                )
            })?;
        }
        Ok(())
    }
}
