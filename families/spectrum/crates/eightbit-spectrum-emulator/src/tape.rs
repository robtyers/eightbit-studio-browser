//! Original streaming standard-speed TAP transport: no programme-memory injection.
use super::{Error, Result};

pub const MAX_TAP_BYTES: usize = 1024 * 1024;
pub const MAX_TAP_BLOCKS: usize = 8192;
const PILOT: u32 = 2168;
const SYNC_ONE: u32 = 667;
const SYNC_TWO: u32 = 735;
const ZERO: u32 = 855;
const ONE: u32 = 1710;
const PAUSE: u32 = 3_500_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Pilot,
    SyncOne,
    SyncTwo,
    Data,
    Tail,
    Pause,
    End,
}

pub struct Tape {
    bytes: Vec<u8>,
    blocks: Vec<std::ops::Range<usize>>,
    block: usize,
    cursor: usize,
    bit: u8,
    second_half: bool,
    pilot_left: u32,
    phase: Phase,
    remaining: u32,
    high: bool,
    playing: bool,
    elapsed: u64,
}
impl Tape {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_TAP_BYTES || bytes.is_empty() {
            return Err(Error::Tape("Choose a non-empty TAP up to 1 MiB"));
        }
        let mut blocks = Vec::new();
        let mut position = 0;
        while position < bytes.len() {
            let header = bytes
                .get(position..position + 2)
                .ok_or(Error::Tape("Truncated TAP length"))?;
            let count = usize::from(u16::from_le_bytes([header[0], header[1]]));
            position += 2;
            if count == 0 || count > bytes.len() - position || blocks.len() >= MAX_TAP_BLOCKS {
                return Err(Error::Tape("Invalid TAP block extent/count"));
            }
            blocks.push(position..position + count);
            position += count;
        }
        let mut result = Self {
            bytes: bytes.to_vec(),
            blocks,
            block: 0,
            cursor: 0,
            bit: 0,
            second_half: false,
            pilot_left: 0,
            phase: Phase::End,
            remaining: 0,
            high: false,
            playing: false,
            elapsed: 0,
        };
        result.begin_block();
        Ok(result)
    }
    fn begin_block(&mut self) {
        if let Some(range) = self.blocks.get(self.block) {
            self.cursor = range.start;
            self.bit = 0;
            self.second_half = false;
            self.pilot_left = if self.bytes[range.start] < 128 {
                8063
            } else {
                3223
            };
            self.phase = Phase::Pilot;
            self.remaining = PILOT;
        } else {
            self.phase = Phase::End;
            self.playing = false;
            self.high = false;
        }
    }
    fn bit_duration(&self) -> u32 {
        if self.bytes[self.cursor] & (0x80 >> self.bit) == 0 {
            ZERO
        } else {
            ONE
        }
    }
    fn boundary(&mut self) {
        match self.phase {
            Phase::Pilot => {
                self.high = !self.high;
                self.pilot_left -= 1;
                if self.pilot_left == 0 {
                    self.phase = Phase::SyncOne;
                    self.remaining = SYNC_ONE;
                } else {
                    self.remaining = PILOT;
                }
            }
            Phase::SyncOne => {
                self.high = !self.high;
                self.phase = Phase::SyncTwo;
                self.remaining = SYNC_TWO;
            }
            Phase::SyncTwo => {
                self.high = !self.high;
                self.phase = Phase::Data;
                self.remaining = self.bit_duration();
            }
            Phase::Data => {
                self.high = !self.high;
                self.second_half = !self.second_half;
                if !self.second_half {
                    self.bit += 1;
                    if self.bit == 8 {
                        self.cursor += 1;
                        self.bit = 0;
                    }
                }
                if self.cursor == self.blocks[self.block].end {
                    // Keep the last edge observable before settling the idle line.
                    self.phase = Phase::Tail;
                    self.remaining = 3500;
                } else {
                    self.remaining = self.bit_duration();
                }
            }
            Phase::Tail => {
                self.high = false;
                self.phase = Phase::Pause;
                self.remaining = PAUSE - 3500;
            }
            Phase::Pause => {
                self.block += 1;
                self.begin_block();
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
            self.elapsed += u64::from(count);
            self.remaining -= count;
            clocks -= count;
            if self.remaining == 0 {
                self.boundary();
            }
        }
    }
    pub fn play(&mut self) {
        self.playing = self.phase != Phase::End;
    }
    pub fn pause(&mut self) {
        self.playing = false;
    }
    pub fn rewind(&mut self) {
        self.block = 0;
        self.elapsed = 0;
        self.high = false;
        self.playing = false;
        self.begin_block();
    }
    #[must_use]
    pub fn high(&self) -> bool {
        self.high
    }
    #[must_use]
    pub fn playing(&self) -> bool {
        self.playing
    }
    #[must_use]
    pub fn elapsed(&self) -> u64 {
        self.elapsed
    }
    #[must_use]
    pub fn block(&self) -> usize {
        self.block
    }
    #[must_use]
    pub fn blocks(&self) -> usize {
        self.blocks.len()
    }
}
