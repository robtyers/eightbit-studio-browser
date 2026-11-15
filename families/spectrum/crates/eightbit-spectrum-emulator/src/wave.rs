//! Bounded PCM cassette input. The board observes EAR levels at guest clock times.
use super::{CPU_HZ, Error, MAX_TAP_BYTES, Result};
pub struct Wave {
    pcm: Vec<u8>,
    rate: u32,
    channels: usize,
    bits: u16,
    align: usize,
    cursor: usize,
    clock: u64,
    high: bool,
    playing: bool,
}
fn invalid() -> Error {
    Error::Tape("Choose a bounded PCM WAV: 8/16 bit, mono/stereo, 8–96 kHz")
}
fn word(bytes: &[u8], p: usize) -> Result<u16> {
    let b = bytes.get(p..p + 2).ok_or_else(invalid)?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}
fn dword(bytes: &[u8], p: usize) -> Result<u32> {
    let b = bytes.get(p..p + 4).ok_or_else(invalid)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
impl Wave {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_TAP_BYTES
            || bytes.get(..4) != Some(b"RIFF")
            || bytes.get(8..12) != Some(b"WAVE")
            || usize::try_from(dword(bytes, 4)?)
                .ok()
                .and_then(|n| n.checked_add(8))
                != Some(bytes.len())
        {
            return Err(invalid());
        }
        let (mut format, mut data) = (None, None);
        let mut p = 12;
        for _ in 0..128 {
            if p == bytes.len() {
                break;
            }
            let n = usize::try_from(dword(bytes, p + 4)?).map_err(|_| invalid())?;
            let start = p.checked_add(8).ok_or_else(invalid)?;
            let end = start
                .checked_add(n)
                .filter(|end| *end <= bytes.len())
                .ok_or_else(invalid)?;
            match bytes.get(p..p + 4) {
                Some(b"fmt ") if format.is_none() && n >= 16 => {
                    format = Some(&bytes[start..end]);
                }
                Some(b"data") if data.is_none() => {
                    data = Some(&bytes[start..end]);
                }
                Some(b"fmt " | b"data") => return Err(invalid()),
                _ => {}
            }
            p = end
                .checked_add(n & 1)
                .filter(|p| *p <= bytes.len())
                .ok_or_else(invalid)?;
        }
        if p != bytes.len() {
            return Err(invalid());
        }
        let f = format.ok_or_else(invalid)?;
        let channels = usize::from(word(f, 2)?);
        let rate = dword(f, 4)?;
        let bits = word(f, 14)?;
        let align = channels * usize::from(bits / 8);
        let data = data.filter(|d| !d.is_empty()).ok_or_else(invalid)?;
        if word(f, 0)? != 1
            || !matches!(channels, 1 | 2)
            || !matches!(bits, 8 | 16)
            || !(8000..=96000).contains(&rate)
            || usize::from(word(f, 12)?) != align
            || dword(f, 8)? != rate * u32::try_from(align).map_err(|_| invalid())?
            || !data.len().is_multiple_of(align)
        {
            return Err(invalid());
        }
        Ok(Self {
            pcm: data.into(),
            rate,
            channels,
            bits,
            align,
            cursor: 0,
            clock: 0,
            high: false,
            playing: false,
        })
    }
    pub fn play(&mut self) {
        self.playing = self.cursor < self.pcm.len();
    }
    pub fn rewind(&mut self) {
        self.cursor = 0;
        self.clock = 0;
        self.high = false;
        self.playing = false;
    }
    #[must_use]
    pub fn pending(&self) -> bool {
        self.cursor < self.pcm.len()
    }
    #[must_use]
    pub fn high(&self) -> bool {
        self.high
    }
    pub fn advance(&mut self, clocks: u32) {
        self.advance_at(clocks, CPU_HZ);
    }
    pub(crate) fn advance_at(&mut self, clocks: u32, cpu_hz: u32) {
        if !self.playing {
            return;
        }
        self.clock += u64::from(clocks) * u64::from(self.rate);
        while self.clock >= u64::from(cpu_hz) && self.cursor < self.pcm.len() {
            self.clock -= u64::from(cpu_hz);
            // Select the stronger channel, so a silent channel cannot halve the signal.
            let mut sample = 0_i32;
            for channel in 0..self.channels {
                let p = self.cursor + channel * usize::from(self.bits / 8);
                let value = if self.bits == 8 {
                    (i32::from(self.pcm[p]) - 128) * 256
                } else {
                    i32::from(i16::from_le_bytes([self.pcm[p], self.pcm[p + 1]]))
                };
                if value.abs() > sample.abs() {
                    sample = value;
                }
            }
            if sample > 512 {
                self.high = true;
            } else if sample < -512 {
                self.high = false;
            }
            self.cursor += self.align;
        }
        if self.cursor == self.pcm.len() {
            self.playing = false;
            self.high = false;
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    fn wav() -> Vec<u8> {
        let mut out = b"RIFF".to_vec();
        out.extend(40_u32.to_le_bytes());
        out.extend(b"WAVEfmt ");
        out.extend(16_u32.to_le_bytes());
        out.extend(1_u16.to_le_bytes());
        out.extend(1_u16.to_le_bytes());
        out.extend(8000_u32.to_le_bytes());
        out.extend(8000_u32.to_le_bytes());
        out.extend(1_u16.to_le_bytes());
        out.extend(8_u16.to_le_bytes());
        out.extend(b"data");
        out.extend(4_u32.to_le_bytes());
        out.extend([255, 255, 0, 0]);
        out
    }
    #[test]
    fn wave_levels_follow_guest_time_and_invalid_chunks_are_total() {
        let bytes = wav();
        let mut tape = Wave::parse(&bytes).unwrap();
        tape.play();
        tape.advance(438);
        assert!(tape.high());
        tape.advance(875);
        assert!(!tape.high());
        tape.advance(1000);
        assert!(!tape.pending());
        tape.rewind();
        assert!(tape.pending());
        for n in 0..bytes.len() {
            assert!(Wave::parse(&bytes[..n]).is_err());
        }
        let mut invalid = bytes.clone();
        invalid[20] = 3;
        assert!(Wave::parse(&invalid).is_err());
        invalid = bytes.clone();
        invalid[40..44].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(Wave::parse(&invalid).is_err());
        invalid = bytes;
        invalid[32] = 4;
        assert!(Wave::parse(&invalid).is_err());
    }
}
