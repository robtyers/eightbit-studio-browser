//! Bounded digital cassette, joystick, DAC and Centronics signals; no host I/O.
use crate::cassette::CassetteSignal;
use serde::Serialize;
use std::collections::VecDeque;
pub const MASTER_HZ: u32 = 14_318_180;
pub const AUDIO_HZ: u32 = 48_000;
const MAX_TAPE_PULSES: usize = 1_048_576;
const MAX_CAPTURE: usize = 4096;
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TapePulse {
    pub clocks: u32,
    pub high: bool,
}
// These booleans represent independent electrical pins, not overlapping lifecycle states.
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct Io {
    pub axes: [u8; 4],
    pub buttons: [bool; 2],
    pub printer_busy: bool,
    pub printer_ack: bool,
    pub cartridge_sound: i16,
    pub nmi: bool,
    pub halt: bool,
    pub tape_high: bool,
    tape: VecDeque<TapePulse>,
    cassette: Option<CassetteSignal>,
    tape_remaining: u32,
    sample_phase: u32,
    sample_sum: i64,
    sample_clocks: u32,
    audio: VecDeque<i16>,
    printer: VecDeque<u8>,
    output: VecDeque<TapePulse>,
    output_clocks: u32,
    output_high: bool,
    pub dropped_audio: u64,
    pub dropped_printer: u64,
    pub dropped_tape: u64,
}
impl Default for Io {
    fn default() -> Self {
        Self {
            axes: [32; 4],
            buttons: [false; 2],
            printer_busy: false,
            printer_ack: true,
            cartridge_sound: 0,
            nmi: false,
            halt: false,
            tape_high: false,
            tape: VecDeque::new(),
            cassette: None,
            tape_remaining: 0,
            sample_phase: 0,
            sample_sum: 0,
            sample_clocks: 0,
            audio: VecDeque::new(),
            printer: VecDeque::new(),
            output: VecDeque::new(),
            output_clocks: 0,
            output_high: false,
            dropped_audio: 0,
            dropped_printer: 0,
            dropped_tape: 0,
        }
    }
}
fn capture<T>(queue: &mut VecDeque<T>, dropped: &mut u64, value: T) {
    if queue.len() == MAX_CAPTURE {
        queue.pop_front();
        *dropped = dropped.saturating_add(1);
    }
    queue.push_back(value);
}
impl Io {
    pub fn mount(&mut self, pulses: Vec<TapePulse>) -> Result<(), String> {
        if pulses.is_empty()
            || pulses.len() > MAX_TAPE_PULSES
            || pulses
                .iter()
                .any(|p| p.clocks == 0 || p.clocks > MASTER_HZ * 60)
        {
            return Err(
                "DME1005: cassette pulses must be non-empty, bounded and have positive durations"
                    .into(),
            );
        }
        let total: u64 = pulses.iter().map(|p| u64::from(p.clocks)).sum();
        if total > u64::from(MASTER_HZ) * 6 * 3600 {
            return Err("DME1005: cassette duration exceeds six hours".into());
        }
        self.tape = pulses.into();
        self.cassette = None;
        self.tape_remaining = 0;
        self.tape_high = false;
        Ok(())
    }
    pub fn mount_signal(&mut self, signal: CassetteSignal) {
        self.eject();
        self.cassette = Some(signal);
    }
    pub fn eject(&mut self) {
        self.tape.clear();
        self.cassette = None;
        self.tape_remaining = 0;
        self.tape_high = false;
    }
    pub fn pending(&self) -> bool {
        self.tape_remaining != 0
            || !self.tape.is_empty()
            || self.cassette.as_ref().is_some_and(CassetteSignal::pending)
    }
    pub fn reset_audio(&mut self) {
        // A partial sample belongs to the old execution just as queued audio does.
        self.audio.clear();
        self.sample_phase = 0;
        self.sample_sum = 0;
        self.sample_clocks = 0;
    }
    pub fn audio(&mut self) -> Vec<i16> {
        self.audio.drain(..).collect()
    }
    pub fn printer(&mut self) -> Vec<u8> {
        self.printer.drain(..).collect()
    }
    pub fn recording(&mut self) -> Vec<TapePulse> {
        self.flush_recording();
        self.output.drain(..).collect()
    }
    fn flush_recording(&mut self) {
        if self.output_clocks != 0 {
            capture(
                &mut self.output,
                &mut self.dropped_tape,
                TapePulse {
                    clocks: self.output_clocks,
                    high: self.output_high,
                },
            );
            self.output_clocks = 0;
        }
    }
    pub fn print(&mut self, data: u8) {
        if !self.printer_busy {
            capture(&mut self.printer, &mut self.dropped_printer, data);
        }
    }
    pub fn advance(&mut self, clocks: u32, motor: bool, sound: i16, tape_output: bool) {
        if !motor {
            self.flush_recording();
        } else if clocks != 0 {
            let mut remaining = clocks;
            while remaining != 0 {
                if self.tape_remaining == 0 {
                    let Some(pulse) = self
                        .tape
                        .pop_front()
                        .or_else(|| self.cassette.as_mut().and_then(Iterator::next))
                    else {
                        break;
                    };
                    self.tape_remaining = pulse.clocks;
                    self.tape_high = pulse.high;
                }
                // The next pulse becomes visible only when its first clock is
                // consumed, not at the exclusive end of this bus interval.
                let elapsed = remaining.min(self.tape_remaining);
                self.tape_remaining -= elapsed;
                remaining -= elapsed;
            }
            if tape_output != self.output_high {
                self.flush_recording();
                self.output_high = tape_output;
            }
            remaining = clocks;
            while remaining != 0 {
                let elapsed = remaining.min(MASTER_HZ * 60 - self.output_clocks);
                self.output_clocks += elapsed;
                remaining -= elapsed;
                if self.output_clocks == MASTER_HZ * 60 {
                    self.flush_recording();
                }
            }
        }
        let mut remaining = clocks;
        while remaining != 0 {
            // Sound is constant for the supplied bus interval. Integrate only
            // up to the next fractional-rate sample boundary, retaining the
            // same integer clock count, mean and truncation as scalar sampling.
            let to_sample = (MASTER_HZ - self.sample_phase).div_ceil(AUDIO_HZ);
            let elapsed = remaining.min(to_sample);
            self.sample_phase += elapsed * AUDIO_HZ;
            self.sample_sum += i64::from(sound) * i64::from(elapsed);
            self.sample_clocks += elapsed;
            remaining -= elapsed;
            if self.sample_phase >= MASTER_HZ {
                self.sample_phase -= MASTER_HZ;
                let sample =
                    i16::try_from(self.sample_sum / i64::from(self.sample_clocks)).unwrap_or(0);
                capture(&mut self.audio, &mut self.dropped_audio, sample);
                self.sample_sum = 0;
                self.sample_clocks = 0;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    // Original scalar model is an independent scheduling oracle. Keep its
    // clock-by-clock integration to expose fractional boundaries and edge order.
    impl Io {
        fn advance_reference(&mut self, clocks: u32, motor: bool, sound: i16, tape_output: bool) {
            if !motor {
                self.flush_recording();
            }
            for _ in 0..clocks {
                if motor {
                    if self.tape_remaining == 0
                        && let Some(p) = self.tape.pop_front()
                    {
                        self.tape_remaining = p.clocks;
                        self.tape_high = p.high;
                    }
                    if self.tape_remaining > 0 {
                        self.tape_remaining -= 1;
                    }
                    if tape_output != self.output_high {
                        self.flush_recording();
                        self.output_high = tape_output;
                    }
                    self.output_clocks += 1;
                    if self.output_clocks == MASTER_HZ * 60 {
                        self.flush_recording();
                    }
                }
                self.sample_phase += AUDIO_HZ;
                self.sample_sum += i64::from(sound);
                self.sample_clocks += 1;
                if self.sample_phase >= MASTER_HZ {
                    self.sample_phase -= MASTER_HZ;
                    let sample =
                        i16::try_from(self.sample_sum / i64::from(self.sample_clocks)).unwrap_or(0);
                    capture(&mut self.audio, &mut self.dropped_audio, sample);
                    self.sample_sum = 0;
                    self.sample_clocks = 0;
                }
            }
        }
    }

    fn assert_signal_state(actual: &Io, expected: &Io) {
        assert_eq!(actual.tape_high, expected.tape_high);
        assert_eq!(actual.tape_remaining, expected.tape_remaining);
        assert_eq!(actual.tape.len(), expected.tape.len());
        assert_eq!(actual.sample_phase, expected.sample_phase);
        assert_eq!(actual.sample_sum, expected.sample_sum);
        assert_eq!(actual.sample_clocks, expected.sample_clocks);
        assert_eq!(actual.output_clocks, expected.output_clocks);
        assert_eq!(actual.output_high, expected.output_high);
        assert_eq!(actual.dropped_audio, expected.dropped_audio);
        assert_eq!(actual.dropped_tape, expected.dropped_tape);
        assert_eq!(actual.pending(), expected.pending());
    }

    #[test]
    fn scheduled_audio_and_tape_match_every_scalar_boundary() -> Result<(), String> {
        let mut seed = 0x6809_6821_u32;
        let mut random = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed
        };
        let pulses: Vec<_> = (0..4096)
            .map(|_| TapePulse {
                clocks: random() % 961 + 1,
                high: random() & 1 != 0,
            })
            .collect();
        let mut actual = Io::default();
        let mut expected = Io::default();
        actual.mount(pulses.clone())?;
        expected.mount(pulses)?;
        let spans = [0, 1, 2, 7, 8, 15, 16, 17, 298, 299, 300, 911, 4096];
        for interval in 0..20_000 {
            let clocks = spans[interval % spans.len()];
            let motor = random() & 7 != 0;
            let sound_bytes = random().to_le_bytes();
            let sound = i16::from_le_bytes([sound_bytes[0], sound_bytes[1]]);
            let output = random() & 1 != 0;
            actual.advance(clocks, motor, sound, output);
            expected.advance_reference(clocks, motor, sound, output);
            assert_signal_state(&actual, &expected);
            if interval % 97 == 0 {
                assert_eq!(actual.audio(), expected.audio());
                assert_eq!(actual.recording(), expected.recording());
            }
        }
        assert_eq!(actual.audio(), expected.audio());
        assert_eq!(actual.recording(), expected.recording());
        Ok(())
    }

    #[test]
    fn scheduled_capture_keeps_overflow_and_sixty_second_split_semantics() {
        let mut actual = Io::default();
        let mut expected = Io::default();
        actual.output_clocks = MASTER_HZ * 60 - 3;
        expected.output_clocks = actual.output_clocks;
        actual.advance(17, true, i16::MIN, false);
        expected.advance_reference(17, true, i16::MIN, false);
        assert_eq!(actual.recording(), expected.recording());
        actual.advance(MASTER_HZ / 8, false, i16::MAX, true);
        expected.advance_reference(MASTER_HZ / 8, false, i16::MAX, true);
        for interval in 0..MAX_CAPTURE + 17 {
            actual.advance(1, true, -17, interval % 2 == 0);
            expected.advance_reference(1, true, -17, interval % 2 == 0);
        }
        assert_signal_state(&actual, &expected);
        assert!(actual.dropped_audio > 0);
        assert!(actual.dropped_tape > 0);
        assert_eq!(actual.audio(), expected.audio());
        assert_eq!(actual.recording(), expected.recording());
    }

    #[test]
    fn tape_validation_motor_pause_and_eject() {
        let mut io = Io::default();
        assert!(
            io.mount(vec![TapePulse {
                clocks: 0,
                high: true
            }])
            .is_err()
        );
        assert!(
            io.mount(vec![
                TapePulse {
                    clocks: 3,
                    high: true
                },
                TapePulse {
                    clocks: 2,
                    high: false
                }
            ])
            .is_ok()
        );
        io.advance(100, false, 0, false);
        assert!(!io.tape_high);
        assert!(io.pending());
        io.advance(3, true, 0, false);
        assert!(io.tape_high);
        io.advance(1, true, 0, false);
        assert!(!io.tape_high);
        io.eject();
        assert!(!io.pending());
        assert!(!io.tape_high);
    }
    #[test]
    fn reset_discards_queued_and_partial_audio_at_every_sample_boundary() {
        for clocks in 0..MASTER_HZ.div_ceil(AUDIO_HZ) {
            let mut io = Io::default();
            io.advance(2 * MASTER_HZ / AUDIO_HZ + clocks, false, 12_000, false);
            io.reset_audio();
            assert!(io.audio().is_empty());
            io.advance(MASTER_HZ / 100, false, 0, false);
            let samples = io.audio();
            assert!(!samples.is_empty());
            assert!(
                samples.iter().all(|&sample| sample == 0),
                "reset phase {clocks}"
            );
        }
    }

    #[test]
    fn audio_fractional_rate_bounds_and_digital_recording() {
        let mut io = Io::default();
        io.advance(MASTER_HZ / 100, true, 1000, false);
        io.advance(300, true, 1000, true);
        let samples = io.audio();
        assert_eq!(samples.len(), 481);
        assert!(samples.iter().all(|&s| s == 1000));
        assert_eq!(
            io.recording(),
            [
                TapePulse {
                    clocks: MASTER_HZ / 100,
                    high: false
                },
                TapePulse {
                    clocks: 300,
                    high: true
                }
            ]
        );
        for n in 0..5000 {
            io.print(u8::try_from(n & 255).unwrap_or(0));
        }
        assert_eq!(io.dropped_printer, 904);
        assert_eq!(io.printer().len(), MAX_CAPTURE);
    }
    #[test]
    fn cassette_tail_streaming_and_motor_pause_preserve_every_recorded_clock() {
        let mut io = Io::default();
        io.advance(5, true, 0, true);
        assert_eq!(
            io.recording(),
            [TapePulse {
                clocks: 5,
                high: true
            }]
        );
        assert!(io.recording().is_empty());
        io.advance(3, true, 0, true);
        io.advance(100, false, 0, false);
        assert_eq!(
            io.recording(),
            [TapePulse {
                clocks: 3,
                high: true
            }]
        );
        io.advance(4, true, 0, false);
        io.advance(2, true, 0, true);
        io.advance(1, false, 0, true);
        assert_eq!(
            io.recording(),
            [
                TapePulse {
                    clocks: 4,
                    high: false
                },
                TapePulse {
                    clocks: 2,
                    high: true
                }
            ]
        );
    }
}
