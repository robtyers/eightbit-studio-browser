//! Disconnected SY6551 register/transmit boundary, from the D64 supplement.
//! No host serial device or network is opened. Completed guest bytes are bounded.
use std::collections::VecDeque;

const MAX_OUTPUT: usize = 4096;
#[derive(Default)]
pub(crate) struct Acia {
    command: u8,
    control: u8,
    shifting: Option<u8>,
    pending: Option<u8>,
    remaining: u64,
    output: VecDeque<u8>,
}
impl Acia {
    pub fn read(&self, register: u16) -> u8 {
        match register & 3 {
            0 => 0,
            // DSR/DCD are unconnected; there is no fabricated received byte.
            1 => 0x60 | if self.pending.is_none() { 0x10 } else { 0 },
            2 => self.command,
            _ => self.control,
        }
    }
    pub fn write(&mut self, register: u16, value: u8) {
        match register & 3 {
            0 => {
                if self.shifting.is_none() {
                    self.shifting = Some(value);
                    self.remaining = self.frame_clocks();
                } else if self.pending.is_none() {
                    self.pending = Some(value);
                }
            }
            1 => {
                self.command &= 0xe0;
                self.pending = None;
                self.shifting = None;
                self.remaining = 0;
            }
            2 => self.command = value,
            _ => {
                let external = self.control.trailing_zeros() >= 4;
                self.control = value;
                if external && self.shifting.is_some() {
                    self.remaining = self.frame_clocks();
                }
            }
        }
    }
    fn frame_clocks(&self) -> u64 {
        // Rates in hundredths of baud include the 110/134.58 settings.
        const RATES: [u32; 16] = [
            0, 5000, 7500, 10_992, 13_458, 15_000, 30_000, 60_000, 120_000, 180_000, 240_000,
            360_000, 480_000, 720_000, 960_000, 1_920_000,
        ];
        let rate = RATES[usize::from(self.control & 15)];
        if rate == 0 {
            return u64::MAX;
        } // External clock is disconnected.
        let data_bits = 8 - ((self.control >> 5) & 3);
        let parity = u8::from(self.command & 0x20 != 0);
        let stop_halves = if self.control & 0x80 == 0 {
            2
        } else if data_bits == 5 && parity == 0 {
            3
        } else {
            4
        };
        let halves = 2 + 2 * u64::from(data_bits + parity) + stop_halves;
        (u64::from(crate::MASTER_HZ) * halves * 100).div_ceil(u64::from(rate) * 2)
    }
    pub fn tick(&mut self, mut clocks: u64) {
        // At most two owned characters can complete in one tick.
        for _ in 0..2 {
            let Some(byte) = self.shifting else {
                return;
            };
            if clocks < self.remaining {
                self.remaining -= clocks;
                return;
            }
            clocks -= self.remaining;
            if self.output.len() == MAX_OUTPUT {
                self.output.pop_front();
            }
            self.output.push_back(byte);
            self.shifting = self.pending.take();
            self.remaining = if self.shifting.is_some() {
                self.frame_clocks()
            } else {
                0
            };
        }
    }
    pub fn take_output(&mut self) -> Vec<u8> {
        self.output.drain(..).collect()
    }
}
