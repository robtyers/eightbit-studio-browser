//! Explicit recording of guest cassette pins; shared media code recovers CAS.
use crate::cassette::{Cassette, CassetteSnapshot};
use crate::{MASTER_HZ, TapePulse};
use eightbit_dragon_media::{decode_wav_cassette_transport, encode_pcm_wav_mono};
use serde::Serialize;

const SAMPLE_RATE: u32 = 9_600;
const MAX_SAMPLES: usize = SAMPLE_RATE as usize * 15 * 60;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingSnapshot {
    pub state: &'static str,
    pub samples: usize,
    pub sample_rate: u32,
    pub message: &'static str,
    pub cassette: Option<CassetteSnapshot>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingExport {
    pub filename: String,
    pub bytes: Vec<u8>,
}
#[derive(Default)]
pub(crate) struct Recording {
    active: bool,
    samples: Vec<i16>,
    phase: u64,
    sample_count: usize,
    failure: Option<&'static str>,
    recovered: Option<(CassetteSnapshot, Vec<u8>)>,
}
impl Recording {
    pub fn start(&mut self) -> Result<(), String> {
        if self.active || self.recovered.is_some() || self.failure.is_some() {
            return Err("DME1005: finish or discard the current cassette recording first".into());
        }
        self.active = true;
        Ok(())
    }
    pub fn active(&self) -> bool {
        self.active
    }
    pub fn failed(&mut self, message: &'static str) {
        self.active = false;
        self.failure = Some(message);
        self.samples = Vec::new();
    }
    pub fn append(&mut self, pulses: Vec<TapePulse>, dropped: bool) {
        if !self.active {
            return;
        }
        if dropped {
            self.failed("Cassette signal was lost. Discard the recording and retry.");
            return;
        }
        for pulse in pulses {
            let phase = self.phase + u64::from(pulse.clocks) * u64::from(SAMPLE_RATE);
            let count = usize::try_from(phase / u64::from(MASTER_HZ)).unwrap_or(MAX_SAMPLES + 1);
            if count > MAX_SAMPLES.saturating_sub(self.samples.len()) {
                self.failed(
                    "Cassette recording reached the 15-minute limit. Discard it and retry.",
                );
                return;
            }
            self.phase = phase % u64::from(MASTER_HZ);
            self.samples.extend(std::iter::repeat_n(
                if pulse.high { 24_576 } else { -24_576 },
                count,
            ));
            self.sample_count = self.samples.len();
        }
    }
    pub fn finish(&mut self) -> Result<(), String> {
        if !self.active {
            return Err("DME1005: no cassette recording is active".into());
        }
        self.active = false;
        let decoded = encode_pcm_wav_mono(&self.samples, SAMPLE_RATE)
            .and_then(|wav| decode_wav_cassette_transport(&wav));
        match decoded {
            Ok(decoded) => match Cassette::read(decoded.cassette.clone()) {
                Ok(cassette) => {
                    self.recovered = Some((cassette.receipt, decoded.cassette));
                    self.samples = Vec::new();
                }
                Err(_) => {
                    self.failed("Recorded cassette exceeds the supported session format or size.");
                }
            },
            Err(_) => self.failed(
                "No complete supported cassette was recorded. Discard it and retry the guest save.",
            ),
        }
        Ok(())
    }
    pub fn export(&self) -> Result<RecordingExport, String> {
        let (_, bytes) = self.recovered.as_ref().ok_or_else(|| {
            "DME1005: finish a valid cassette recording before saving".to_string()
        })?;
        Ok(RecordingExport {
            filename: "dragon-recording.cas".into(),
            bytes: bytes.clone(),
        })
    }
    pub fn snapshot(&self) -> RecordingSnapshot {
        RecordingSnapshot {
            state: if self.active { "recording" } else if self.failure.is_some() { "failed" } else if self.recovered.is_some() { "ready" } else { "idle" },
            samples: self.sample_count, sample_rate: SAMPLE_RATE,
            message: self.failure.unwrap_or(if self.active { "Recording guest cassette output. Finish after the guest has saved or closed the file." } else if self.recovered.is_some() { "Cassette checked. Save a copy before discarding or stopping the emulator." } else { "Start recording before guest cassette output." }),
            cassette: self.recovered.as_ref().map(|(receipt, _)| receipt.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_data_and_multi_file_signals_export_only_complete_checked_tapes()
    -> Result<(), String> {
        let mut data = eightbit_dragon_media::create_ascii_basic("RECORDS", "42,\"ORIGINAL\"\n")
            .map_err(|error| error.to_string())?;
        let header = eightbit_dragon_media::read_ascii_basic(&data)
            .map_err(|error| error.to_string())?
            .blocks[0]
            .clone();
        data[header.offset + 11] = 1;
        data[header.offset + 3 + header.data_length] =
            data[header.offset + 3 + header.data_length].wrapping_add(1);
        let basic = eightbit_dragon_media::create_tokenised_basic("BASIC", &[0, 0])
            .map_err(|error| error.to_string())?;
        for (bytes, format) in [
            (data.clone(), "ascii-data"),
            ([basic, data].concat(), "cassette-stream"),
        ] {
            let cassette = Cassette::read(bytes.clone())?;
            let mut signal = cassette.signal();
            let mut recording = Recording::default();
            recording.start()?;
            loop {
                let chunk = signal.by_ref().take(128).collect::<Vec<_>>();
                if chunk.is_empty() {
                    break;
                }
                recording.append(chunk, false);
            }
            recording.finish()?;
            assert_eq!(
                recording.snapshot().state,
                "ready",
                "{}",
                recording.snapshot().message
            );
            assert_eq!(
                recording
                    .snapshot()
                    .cassette
                    .ok_or("Missing receipt")?
                    .format,
                format
            );
            assert_eq!(recording.export()?.bytes, bytes);
        }
        Ok(())
    }
    #[test]
    fn corrupt_cassette_signal_cannot_be_exported() -> Result<(), String> {
        let bytes = eightbit_dragon_media::create_machine_code("BROKEN", &[0x39], 0x6000, 0x6000)
            .map_err(|error| error.to_string())?;
        let file =
            eightbit_dragon_media::read_machine_code(&bytes).map_err(|error| error.to_string())?;
        let checksum = file.blocks[0].offset + 3 + file.blocks[0].data_length;
        let cassette = Cassette::read(bytes)?;
        let mut pulses: Vec<_> = cassette.signal().collect();
        // Flip one checksum bit in the physical signal, preserving bit timing.
        for pulse in &mut pulses[checksum * 16..checksum * 16 + 2] {
            pulse.clocks = if pulse.clocks == 435 * 8 {
                813 * 8
            } else {
                435 * 8
            };
        }
        let mut recording = Recording::default();
        recording.start()?;
        recording.append(pulses, false);
        recording.finish()?;
        assert_eq!(recording.snapshot().state, "failed");
        assert!(recording.samples.is_empty());
        assert!(recording.export().is_err());
        Ok(())
    }
    #[test]
    fn idle_finish_missing_signal_drop_limit_and_discard_fail_closed() -> Result<(), String> {
        let mut recording = Recording::default();
        assert!(recording.finish().is_err());
        assert!(recording.export().is_err());
        recording.start()?;
        assert!(recording.start().is_err());
        recording.finish()?;
        assert_eq!(recording.snapshot().state, "failed");
        assert!(recording.export().is_err());
        let mut recording = Recording::default();
        recording.start()?;
        recording.append(
            vec![TapePulse {
                clocks: 1,
                high: true,
            }],
            true,
        );
        assert_eq!(recording.snapshot().state, "failed");
        assert!(recording.samples.is_empty());
        let mut recording = Recording::default();
        recording.start()?;
        recording.samples.resize(MAX_SAMPLES, 0);
        recording.append(
            vec![TapePulse {
                clocks: MASTER_HZ,
                high: true,
            }],
            false,
        );
        assert_eq!(recording.snapshot().state, "failed");
        assert!(recording.samples.is_empty());
        Ok(())
    }
}
