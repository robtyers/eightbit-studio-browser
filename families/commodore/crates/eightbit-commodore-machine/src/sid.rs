//! Original three-voice SID synthesis. Integer DSP is deterministic on native/WASM.
//! Filter transfer, combined waves and envelope quirks are digital approximations,
//! not a transistor model or qualification of every 6581/8580 revision.

/// Rounded PAL PHI2 frequency in Hz; raster timing itself uses exact 63×312 cycles.
pub const CLOCK_HZ: u32 = 985_248;
/// Fixed mono PCM rate. The board retains a bounded tail, never an unbounded stream.
pub const SAMPLE_RATE: u32 = 44_100;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Envelope {
    Attack,
    Decay,
    #[default]
    Release,
}
#[derive(Clone, Copy, Debug)]
struct Voice {
    phase: u32,
    noise: u32,
    envelope: u8,
    stage: Envelope,
    rate: u16,
    exponential: u8,
}
impl Default for Voice {
    fn default() -> Self {
        Self {
            phase: 0,
            noise: 0x7f_ffff,
            envelope: 0,
            stage: Envelope::Release,
            rate: 0,
            exponential: 0,
        }
    }
}

/// Original bounded SID state with three independently controlled voices.
#[derive(Clone, Debug)]
pub struct Sid {
    registers: [u8; 32],
    voices: [Voice; 3],
    filter_low: i32,
    filter_band: i32,
    bus: u8,
    bus_age: u16,
    sample_clock: u32,
    pots: [u8; 2],
    pot_inputs: [u8; 2],
    pot_clock: u16,
    external: i16,
}
impl Default for Sid {
    fn default() -> Self {
        Self {
            registers: [0; 32],
            voices: [Voice::default(); 3],
            filter_low: 0,
            filter_band: 0,
            bus: 0,
            bus_age: 0,
            sample_clock: 0,
            pots: [255; 2],
            pot_inputs: [255; 2],
            pot_clock: 0,
            external: 0,
        }
    }
}
impl Sid {
    /// Inspect a digital oscillator frequency for an explicitly indexed voice.
    /// This is an observation API, not a guest read from write-only registers.
    #[must_use]
    pub fn frequency(&self, voice: u8) -> Option<u16> {
        if voice >= 3 {
            return None;
        }
        let index = usize::from(voice) * 7;
        Some(u16::from_le_bytes([
            self.registers[index],
            self.registers[index + 1],
        ]))
    }
    /// Present external analogue-input samples as bounded digital values.
    pub fn inputs(&mut self, pots: [u8; 2], external: i16) {
        self.pot_inputs = pots;
        self.external = external;
    }
    /// Read POT/OSC3/ENV3; write-only/unused addresses expose the decaying bus latch.
    pub fn read(&mut self, register: u8) -> u8 {
        let value = match register & 31 {
            25 => self.pots[0],
            26 => self.pots[1],
            27 => (self.waveform(2) >> 4).to_le_bytes()[0],
            28 => self.voices[2].envelope,
            _ => self.bus,
        };
        self.bus = value;
        value
    }
    /// Write a mirrored register. Gate and TEST changes affect guest synthesis only.
    pub fn write(&mut self, register: u8, value: u8) {
        let index = usize::from(register & 31);
        self.bus = value;
        self.bus_age = 0;
        if index > 24 {
            return;
        }
        if index % 7 == 4 && index < 21 {
            let voice = index / 7;
            if (self.registers[index] ^ value) & 1 != 0 {
                self.voices[voice].stage = if value & 1 != 0 {
                    Envelope::Attack
                } else {
                    Envelope::Release
                };
                self.voices[voice].exponential = 0;
            }
            if value & 8 != 0 {
                self.voices[voice].phase = 0;
                self.voices[voice].noise = 0x7f_ffff;
            }
        }
        self.registers[index] = value;
    }
    fn waveform(&self, index: usize) -> u16 {
        let control = self.registers[index * 7 + 4];
        let phase = self.voices[index].phase;
        let source = self.voices[(index + 2) % 3].phase;
        let mut value = 0x0fff;
        if control & 0xf0 == 0 {
            return 0;
        }
        if control & 0x10 != 0 {
            let negative = (phase ^ if control & 4 != 0 { source } else { 0 }) & 0x80_0000 != 0;
            let ramp = (phase >> 11) & 0xfff;
            value &= u16::try_from(if negative { ramp ^ 0xfff } else { ramp }).unwrap_or(0);
        }
        if control & 0x20 != 0 {
            value &= u16::try_from(phase >> 12).unwrap_or(0);
        }
        if control & 0x40 != 0 {
            let width = u16::from_le_bytes([
                self.registers[index * 7 + 2],
                self.registers[index * 7 + 3] & 15,
            ]);
            value &= if control & 8 != 0 || phase >> 12 >= u32::from(width) {
                0xfff
            } else {
                0
            };
        }
        if control & 0x80 != 0 {
            let noise = self.voices[index].noise;
            let bits = ((noise >> 22) & 1) << 7
                | ((noise >> 20) & 1) << 6
                | ((noise >> 16) & 1) << 5
                | ((noise >> 13) & 1) << 4
                | ((noise >> 11) & 1) << 3
                | ((noise >> 7) & 1) << 2
                | ((noise >> 4) & 1) << 1
                | ((noise >> 2) & 1);
            value &= u16::try_from(bits << 4).unwrap_or(0);
        }
        value
    }
    /// Advance one SID clock; return at most one PCM sample at the fixed rate.
    pub fn tick(&mut self) -> Option<i16> {
        self.pot_clock = (self.pot_clock + 1) & 511;
        if self.pot_clock == 0 {
            self.pots = self.pot_inputs;
        }
        let old = self.voices.map(|voice| voice.phase);
        for (index, old_phase) in old.iter().copied().enumerate() {
            let offset = index * 7;
            let control = self.registers[offset + 4];
            if control & 8 != 0 {
                continue;
            }
            let frequency =
                u16::from_le_bytes([self.registers[offset], self.registers[offset + 1]]);
            let next = (old_phase + u32::from(frequency)) & 0xff_ffff;
            self.voices[index].phase = next;
            if old_phase & 0x08_0000 == 0 && next & 0x08_0000 != 0 {
                let noise = self.voices[index].noise;
                self.voices[index].noise =
                    ((noise << 1) | ((noise >> 22) ^ (noise >> 17)) & 1) & 0x7f_ffff;
            }
        }
        let rising = std::array::from_fn::<_, 3, _>(|index| {
            old[index] & 0x80_0000 == 0 && self.voices[index].phase & 0x80_0000 != 0
        });
        for index in 0..3 {
            let source = (index + 2) % 3;
            let source_synced = self.registers[source * 7 + 4] & 2 != 0 && rising[(source + 2) % 3];
            if self.registers[index * 7 + 4] & 2 != 0 && rising[source] && !source_synced {
                self.voices[index].phase = 0;
            }
            self.envelope(index);
        }
        self.bus_age = self.bus_age.saturating_add(1);
        if self.bus_age >= 8192 {
            self.bus = 0;
        }
        let filtered = self.mix();
        self.sample_clock += SAMPLE_RATE;
        if self.sample_clock < CLOCK_HZ {
            return None;
        }
        self.sample_clock -= CLOCK_HZ;
        Some(i16::try_from(filtered.clamp(i32::from(i16::MIN), i32::from(i16::MAX))).unwrap_or(0))
    }
    fn envelope(&mut self, index: usize) {
        let voice = &mut self.voices[index];
        let ad = self.registers[index * 7 + 5];
        let sr = self.registers[index * 7 + 6];
        let rate = match voice.stage {
            Envelope::Attack => ad >> 4,
            Envelope::Decay => ad & 15,
            Envelope::Release => sr & 15,
        };
        // Counter intervals are independently specified clock-domain constants;
        // nominal durations follow the manufacturer rate ranges, not a ROM table.
        let period = match rate {
            0 => 9,
            1 => 32,
            2 => 63,
            3 => 95,
            4 => 149,
            5 => 220,
            6 => 267,
            7 => 313,
            8 => 392,
            9 => 977,
            10 => 1954,
            11 => 3126,
            12 => 3907,
            13 => 11720,
            14 => 19532,
            _ => 31251,
        };
        voice.rate = (voice.rate + 1) & 0x7fff;
        if voice.rate != period {
            return;
        }
        voice.rate = 0;
        if voice.stage == Envelope::Attack {
            voice.envelope = voice.envelope.saturating_add(1);
            if voice.envelope == 255 {
                voice.stage = Envelope::Decay;
                voice.exponential = 0;
            }
            return;
        }
        let divider = match voice.envelope {
            94..=255 => 1,
            55..=93 => 2,
            27..=54 => 4,
            15..=26 => 8,
            7..=14 => 16,
            _ => 30,
        };
        voice.exponential += 1;
        if voice.exponential < divider {
            return;
        }
        voice.exponential = 0;
        let sustain = (sr >> 4) * 17;
        if voice.stage == Envelope::Release || voice.envelope > sustain {
            voice.envelope = voice.envelope.saturating_sub(1);
        }
    }
    fn mix(&mut self) -> i32 {
        let mut dry = 0;
        let mut wet = 0;
        for index in 0..3 {
            let sample = (i32::from(self.waveform(index)) - 2048)
                * i32::from(self.voices[index].envelope)
                / 255;
            if self.registers[23] & (1 << index) != 0 {
                wet += sample;
            } else if index != 2 || self.registers[24] & 0x80 == 0 {
                dry += sample;
            }
        }
        if self.registers[23] & 8 != 0 {
            wet += i32::from(self.external);
        } else {
            dry += i32::from(self.external);
        }
        let cutoff = (u16::from(self.registers[22]) << 3) | u16::from(self.registers[21] & 7);
        let hz = 30 + i32::from(cutoff) * 9970 / 2047;
        let coefficient = i64::from(hz) * 411_775 / i64::from(CLOCK_HZ);
        let damping = 131_072 - i64::from(self.registers[23] >> 4) * 6500;
        let low = i64::from(self.filter_low) + ((coefficient * i64::from(self.filter_band)) >> 16);
        let high = i64::from(wet) - low - ((damping * i64::from(self.filter_band)) >> 16);
        let band = i64::from(self.filter_band) + ((coefficient * high) >> 16);
        self.filter_low = i32::try_from(low.clamp(-131_072, 131_072)).unwrap_or(0);
        self.filter_band = i32::try_from(band.clamp(-131_072, 131_072)).unwrap_or(0);
        let mut filtered = 0;
        if self.registers[24] & 0x10 != 0 {
            filtered += self.filter_low;
        }
        if self.registers[24] & 0x20 != 0 {
            filtered += self.filter_band;
        }
        if self.registers[24] & 0x40 != 0 {
            filtered += i32::try_from(high.clamp(-131_072, 131_072)).unwrap_or(0);
        }
        (dry + filtered) * i32::from(self.registers[24] & 15) / 8
    }
}
