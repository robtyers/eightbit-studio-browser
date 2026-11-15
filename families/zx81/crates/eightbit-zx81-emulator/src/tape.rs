//! Streaming four/nine pulse cassette cells; bytes enter only through EAR.
use super::{CPU_HZ, Error, Model, Result, image::Image};
const HALF: u32 = 487;
const GAP: u32 = 4875;
const LEAD: u32 = CPU_HZ * 5;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Lead,
    Pulse,
    Gap,
    End,
}
pub struct Tape {
    bytes: Vec<u8>,
    cursor: usize,
    bit: u8,
    pulses: u8,
    remaining: u32,
    phase: Phase,
    high: bool,
    playing: bool,
}
impl Tape {
    pub fn from_image(image: &Image, name: &str) -> Result<Self> {
        let mut bytes = Vec::new();
        if !image.model.zx80() {
            if name.is_empty() || name.len() > 127 || !name.is_ascii() {
                return Err(Error::Tape(
                    "Use a cassette name of 1–127 letters, digits or spaces",
                ));
            }
            for byte in name.bytes() {
                bytes.push(match byte.to_ascii_uppercase() {
                    b'A'..=b'Z' => byte.to_ascii_uppercase() - b'A' + 38,
                    b'0'..=b'9' => byte - b'0' + 28,
                    b' ' => 0,
                    _ => {
                        return Err(Error::Tape(
                            "Use letters, digits or spaces in the cassette name",
                        ));
                    }
                });
            }
            if let Some(last) = bytes.last_mut() {
                *last |= 128;
            }
        }
        bytes.extend_from_slice(image.bytes());
        Ok(Self {
            bytes,
            cursor: 0,
            bit: 0,
            pulses: 0,
            remaining: LEAD,
            phase: Phase::Lead,
            high: false,
            playing: false,
        })
    }
    pub fn parse(model: Model, bytes: &[u8], name: &str) -> Result<Self> {
        Self::from_image(&Image::parse(model, bytes)?, name)
    }
    pub fn play(&mut self) {
        self.playing = self.phase != Phase::End;
    }
    pub fn stop(&mut self) {
        self.playing = false;
    }
    pub fn rewind(&mut self) {
        self.cursor = 0;
        self.bit = 0;
        self.pulses = 0;
        self.remaining = LEAD;
        self.phase = Phase::Lead;
        self.high = false;
        self.playing = false;
    }
    #[must_use]
    pub const fn pending(&self) -> bool {
        !matches!(self.phase, Phase::End)
    }
    #[must_use]
    pub const fn high(&self) -> bool {
        self.high
    }
    #[must_use]
    pub const fn playing(&self) -> bool {
        self.playing
    }
    fn begin_bit(&mut self) {
        if self.cursor == self.bytes.len() {
            self.phase = Phase::End;
            self.high = false;
            self.playing = false;
            return;
        }
        self.pulses = if self.bytes[self.cursor] & (128 >> self.bit) == 0 {
            4
        } else {
            9
        };
        self.phase = Phase::Pulse;
        self.high = true;
        self.remaining = HALF;
    }
    fn boundary(&mut self) {
        match self.phase {
            Phase::Lead => self.begin_bit(),
            Phase::Pulse => {
                self.high = !self.high;
                if !self.high {
                    self.pulses -= 1;
                }
                if self.pulses == 0 {
                    self.phase = Phase::Gap;
                    self.remaining = GAP;
                } else {
                    self.remaining = HALF;
                }
            }
            Phase::Gap => {
                self.bit += 1;
                if self.bit == 8 {
                    self.bit = 0;
                    self.cursor += 1;
                }
                self.begin_bit();
            }
            Phase::End => {}
        }
    }
    pub fn advance(&mut self, mut clocks: u32) {
        if !self.playing {
            return;
        }
        while clocks > 0 && self.phase != Phase::End {
            let count = clocks.min(self.remaining);
            self.remaining -= count;
            clocks -= count;
            if self.remaining == 0 {
                self.boundary();
            }
        }
    }
}
