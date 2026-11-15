//! Strict, volatile cassette attachment. Loading remains guest ROM/PIA work.
use crate::{MASTER_HZ, TapePulse};
use eightbit_dragon_media::{MAX_TRANSPORT_BYTES, read_cassette_transport};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub const MAX_SESSION_CASSETTE_BYTES: usize = MAX_TRANSPORT_BYTES;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CassetteSnapshot {
    pub filename: String,
    pub format: &'static str,
    pub bytes: usize,
    pub sha256: String,
    pub pending: bool,
}

pub(crate) struct Cassette {
    bytes: Arc<[u8]>,
    gap_offsets: Arc<[usize]>,
    pub receipt: CassetteSnapshot,
}
impl Cassette {
    pub fn read_wav(bytes: &[u8]) -> Result<Self, String> {
        // Keep both ingress routes within the existing session envelope. The
        // shared media decoder owns PCM geometry, channel ambiguity and checksums;
        // the mounted identity is the recovered transport actually sent to PIA.
        if bytes.is_empty() || bytes.len() > MAX_SESSION_CASSETTE_BYTES {
            return Err("DME1005: select cassette audio of 1–1048576 bytes".into());
        }
        let decoded = eightbit_dragon_media::decode_wav_cassette_transport(bytes)
            .map_err(|error| format!("DME1005: invalid cassette audio: {error}"))?;
        Self::read(decoded.cassette)
    }

    pub fn read(bytes: Vec<u8>) -> Result<Self, String> {
        if bytes.is_empty() || bytes.len() > MAX_SESSION_CASSETTE_BYTES {
            return Err("DME1005: select a cassette of 1–1048576 bytes".into());
        }
        let transport = read_cassette_transport(&bytes)
            .map_err(|error| format!("DME1005: invalid cassette transport: {error}"))?;
        let first = transport
            .files
            .first()
            .ok_or("DME1005: cassette has no files")?;
        let filename = first.filename.clone();
        let format = if transport.files.len() == 1 {
            first.format
        } else {
            "cassette-stream"
        };
        let receipt = CassetteSnapshot {
            filename,
            format,
            bytes: bytes.len(),
            sha256: hex::encode(Sha256::digest(&bytes)),
            pending: true,
        };
        Ok(Self {
            bytes: bytes.into(),
            gap_offsets: transport.gap_offsets.into(),
            receipt,
        })
    }
    pub fn signal(&self) -> CassetteSignal {
        CassetteSignal {
            bytes: Arc::clone(&self.bytes),
            gap_offsets: Arc::clone(&self.gap_offsets),
            offset: 0,
            bit: 0,
            high: true,
            next_gap: 0,
        }
    }
}

/// Encode on demand: even a full-bus cassette uses only its bounded original
/// bytes and validated gap offsets, never a multi-million-entry pulse array.
pub(crate) struct CassetteSignal {
    bytes: Arc<[u8]>,
    gap_offsets: Arc<[usize]>,
    offset: usize,
    bit: u8,
    high: bool,
    next_gap: usize,
}
impl CassetteSignal {
    pub fn pending(&self) -> bool {
        self.offset < self.bytes.len()
    }
}
impl Iterator for CassetteSignal {
    type Item = TapePulse;
    fn next(&mut self) -> Option<TapePulse> {
        let byte = *self.bytes.get(self.offset)?;
        if self.bit == 0 && self.high && self.gap_offsets.get(self.next_gap) == Some(&self.offset) {
            self.next_gap += 1;
            return Some(TapePulse {
                clocks: MASTER_HZ / 2,
                high: false,
            });
        }
        // Published Dragon bit periods, also used by the media WAV encoder:
        // one full cycle per bit, least-significant bit first.
        let clocks = if byte & (1 << self.bit) == 0 {
            813
        } else {
            435
        } * 8;
        let high = self.high;
        self.high = !self.high;
        if self.high {
            self.bit += 1;
            if self.bit == 8 {
                self.bit = 0;
                self.offset += 1;
            }
        }
        Some(TapePulse { clocks, high })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_bus_capture_fits_with_framing_without_expanding_its_pulses() -> Result<(), String> {
        let bytes = eightbit_dragon_media::create_machine_code("FULLBUS", &vec![0; 65_536], 0, 0)
            .map_err(|error| error.to_string())?;
        assert!(bytes.len() > 65_536);
        let cassette = Cassette::read(bytes)?;
        let mut signal = cassette.signal();
        assert!(signal.pending());
        for _ in 0..32 {
            assert!(signal.next().is_some());
        }
        assert!(signal.pending());
        assert_eq!(Arc::strong_count(&cassette.bytes), 2);
        Ok(())
    }
    #[test]
    fn strict_formats_keep_original_identity_and_bounded_physical_signals() -> Result<(), String> {
        let ascii = eightbit_dragon_media::create_ascii_basic("ASCII", "10 PRINT 42\n")
            .map_err(|error| error.to_string())?;
        let cassette = Cassette::read(ascii.clone())?;
        assert_eq!(cassette.receipt.filename, "ASCII");
        assert_eq!(cassette.receipt.format, "ascii-basic");
        assert_eq!(cassette.receipt.sha256, hex::encode(Sha256::digest(&ascii)));
        assert_eq!(cassette.gap_offsets.len(), 2);
        let pulses: Vec<_> = cassette.signal().collect();
        assert_eq!(pulses.len(), ascii.len() * 16 + 2);
        assert_eq!(
            pulses
                .iter()
                .filter(|pulse| pulse.clocks == MASTER_HZ / 2)
                .count(),
            2
        );
        assert_eq!(
            pulses[0],
            TapePulse {
                clocks: 435 * 8,
                high: true
            }
        );
        assert_eq!(
            pulses[1],
            TapePulse {
                clocks: 435 * 8,
                high: false
            }
        );
        assert_eq!(
            pulses[2],
            TapePulse {
                clocks: 813 * 8,
                high: true
            }
        );
        for invalid in [
            Vec::new(),
            vec![0x55; MAX_SESSION_CASSETTE_BYTES + 1],
            vec![0x55; 128],
        ] {
            assert!(Cassette::read(invalid).is_err());
        }
        let mut corrupt = ascii;
        corrupt[140] ^= 1;
        assert!(Cassette::read(corrupt).is_err());
        Ok(())
    }
}
