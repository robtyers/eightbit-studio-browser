//! Original AY-3-8912 digital generators from the GI register/timing contract.
//! Analogue output uses a bounded logarithmic approximation, mixed by the board.
#[derive(Clone)]
pub(crate) struct Ay {
    registers: [u8; 16],
    selected: u8,
    divider: u8,
    tone_count: [u16; 3],
    tone: [bool; 3],
    noise_count: u8,
    noise: u32,
    envelope_count: u16,
    envelope: u8,
    rising: bool,
    holding: bool,
}
impl Default for Ay {
    fn default() -> Self {
        Self {
            registers: [0; 16],
            selected: 0,
            divider: 0,
            tone_count: [0; 3],
            tone: [false; 3],
            noise_count: 0,
            noise: 0x1ffff,
            envelope_count: 0,
            envelope: 0,
            rising: false,
            holding: true,
        }
    }
}
impl Ay {
    pub(crate) fn select(&mut self, value: u8) {
        self.selected = value;
    }
    pub(crate) fn read(&self) -> u8 {
        match self.selected {
            0..=13 => self.registers[usize::from(self.selected)],
            14 if self.registers[7] & 64 != 0 => self.registers[14],
            _ => 255,
        }
    }
    pub(crate) fn write(&mut self, value: u8) {
        const MASKS: [u8; 15] = [
            255, 15, 255, 15, 255, 15, 31, 255, 31, 31, 31, 255, 255, 15, 255,
        ];
        let index = usize::from(self.selected);
        let Some(mask) = MASKS.get(index) else {
            return;
        };
        self.registers[index] = value & mask;
        if index == 13 {
            self.rising = value & 4 != 0;
            self.envelope = if self.rising { 0 } else { 15 };
            self.envelope_count = 0;
            self.holding = false;
        }
    }
    // The chip receives half the CPU clock. Tone half-waves divide it by eight;
    // noise shifts and envelope steps divide it by sixteen.
    pub(crate) fn tick(&mut self) {
        self.divider = self.divider.wrapping_add(1) & 31;
        if self.divider & 15 != 0 {
            return;
        }
        for channel in 0..3 {
            let period =
                u16::from_le_bytes([self.registers[channel * 2], self.registers[channel * 2 + 1]])
                    .max(1);
            self.tone_count[channel] += 1;
            if self.tone_count[channel] >= period {
                self.tone_count[channel] = 0;
                self.tone[channel] = !self.tone[channel];
            }
        }
        if self.divider != 0 {
            return;
        }
        self.noise_count += 1;
        if self.noise_count >= self.registers[6].max(1) {
            self.noise_count = 0;
            let feedback = (self.noise ^ (self.noise >> 3)) & 1;
            self.noise = (self.noise >> 1) | (feedback << 16);
        }
        if self.holding {
            return;
        }
        self.envelope_count = self.envelope_count.saturating_add(1);
        let period = u16::from_le_bytes([self.registers[11], self.registers[12]]).max(1);
        if self.envelope_count < period {
            return;
        }
        self.envelope_count = 0;
        if (self.rising && self.envelope < 15) || (!self.rising && self.envelope > 0) {
            if self.rising {
                self.envelope += 1;
            } else {
                self.envelope -= 1;
            }
            return;
        }
        let shape = self.registers[13];
        if shape & 8 == 0 {
            self.envelope = 0;
            self.holding = true;
        } else if shape & 1 != 0 {
            if shape & 2 != 0 {
                self.envelope = 15 - self.envelope;
            }
            self.holding = true;
        } else if shape & 2 != 0 {
            self.rising = !self.rising;
        } else {
            self.envelope = if self.rising { 0 } else { 15 };
        }
    }
    pub(crate) fn sample(&self) -> i32 {
        const LEVEL: [i32; 16] = [
            0, 32, 45, 64, 91, 128, 181, 256, 362, 512, 724, 1024, 1448, 2048, 2896, 4096,
        ];
        let mut sum = 0;
        for channel in 0..3 {
            let enabled = (self.registers[7] & (1 << channel) != 0 || self.tone[channel])
                && (self.registers[7] & (8 << channel) != 0 || self.noise & 1 != 0);
            let volume = self.registers[8 + channel];
            if enabled {
                sum += LEVEL[usize::from(if volume & 16 != 0 {
                    self.envelope
                } else {
                    volume & 15
                })];
            }
        }
        sum
    }
}

#[cfg(test)]
mod tests {
    use super::Ay;
    fn put(ay: &mut Ay, register: u8, value: u8) {
        ay.select(register);
        ay.write(value);
    }
    #[test]
    fn masks_selection_and_port_direction_do_not_alias_unknown_registers() {
        let mut ay = Ay::default();
        for (r, mask) in [(1, 15), (6, 31), (8, 31), (13, 15)] {
            put(&mut ay, r, 255);
            assert_eq!(ay.read(), mask);
        }
        put(&mut ay, 14, 42);
        assert_eq!(ay.read(), 255);
        put(&mut ay, 7, 64);
        ay.select(14);
        assert_eq!(ay.read(), 42);
        put(&mut ay, 255, 99);
        assert_eq!(ay.read(), 255);
        ay.select(1);
        assert_eq!(ay.read(), 15);
    }
    #[test]
    fn three_independent_tones_and_noise_are_clocked_and_repeatable() {
        let mut ay = Ay::default();
        put(&mut ay, 7, 0x38);
        put(&mut ay, 8, 15);
        put(&mut ay, 0, 2);
        assert_eq!(ay.sample(), 0);
        for _ in 0..32 {
            ay.tick();
        }
        assert_eq!(ay.sample(), 4096);
        for _ in 0..32 {
            ay.tick();
        }
        assert_eq!(ay.sample(), 0);
        put(&mut ay, 7, 0x3f);
        put(&mut ay, 9, 15);
        put(&mut ay, 10, 15);
        assert_eq!(ay.sample(), 12288);
        put(&mut ay, 7, 7);
        let initial = ay.noise;
        for _ in 0..1000 {
            ay.tick();
        }
        assert_ne!(ay.noise, initial);
    }
    #[test]
    fn all_envelope_shapes_restart_and_hold_or_repeat_at_the_documented_levels() {
        for shape in 0..16 {
            let mut ay = Ay::default();
            put(&mut ay, 13, shape);
            assert_eq!(ay.envelope, if shape & 4 != 0 { 0 } else { 15 });
            for _ in 0..512 {
                ay.tick();
            }
            let holds = shape & 8 == 0 || shape & 1 != 0;
            assert_eq!(ay.holding, holds, "shape {shape}");
            if holds {
                let expected = if shape & 8 == 0 {
                    0
                } else if (shape & 4 != 0) ^ (shape & 2 != 0) {
                    15
                } else {
                    0
                };
                assert_eq!(ay.envelope, expected, "shape {shape}");
            }
            put(&mut ay, 13, shape);
            assert!(!ay.holding);
        }
    }
}
