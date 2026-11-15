//! Digital MC6847 display, with an explicitly selected Dragon 32 PAL timing model.
use crate::{HEIGHT, WIDTH, glyphs, sam::Sam};
pub(crate) const LINE_CLOCKS: u32 = 910;
pub(crate) const FRAME_LINES: u32 = 312;
const ACTIVE_LINE: u32 = 63;
const ACTIVE_CLOCK: u32 = 254;
const COLOURS: [[u8; 4]; 9] = [
    [88, 202, 92, 255],
    [230, 222, 66, 255],
    [64, 83, 216, 255],
    [219, 57, 59, 255],
    [224, 216, 182, 255],
    [71, 206, 210, 255],
    [208, 67, 200, 255],
    [230, 140, 59, 255],
    [12, 42, 13, 255],
];
pub(crate) struct Video {
    phase: u32,
    line: u32,
    pub frames: u64,
    pub pixels: Vec<u8>,
    counter: u16,
    divider: usize,
    latch: u8,
}
impl Default for Video {
    fn default() -> Self {
        Self {
            phase: 0,
            line: 0,
            frames: 0,
            pixels: vec![0; WIDTH * HEIGHT * 4],
            counter: 0,
            divider: 0,
            latch: 0,
        }
    }
}
impl Video {
    pub fn sync(&self) -> (bool, bool) {
        (self.phase >= 66, self.line >= 57)
    }
    pub fn advance(&mut self, mut clocks: u32, ram: &[u8], sam: Sam, pins: u8) {
        // RAM and pins are stable for this bus interval. Visit only the pixel
        // clocks it contains, but still perform every horizontal counter reset.
        // The exclusive end keeps a pixel at the next interval's start unread
        // until that interval supplies its potentially changed RAM/SAM/pins.
        while clocks != 0 {
            let elapsed = clocks.min(LINE_CLOCKS - self.phase);
            let end = self.phase + elapsed;
            if (ACTIVE_LINE..ACTIVE_LINE + 192).contains(&self.line) {
                let start = self.phase.max(ACTIVE_CLOCK);
                let mut pixel_clock = (start + 1) & !1;
                let pixel_end = end.min(ACTIVE_CLOCK + 512);
                let span = if pins & 128 != 0 && matches!((pins >> 4) & 7, 0 | 1 | 3 | 5) {
                    16
                } else {
                    8
                };
                while pixel_clock < pixel_end {
                    let x = usize::try_from((pixel_clock - ACTIVE_CLOCK) / 2).unwrap_or(0);
                    let y = usize::try_from(self.line - ACTIVE_LINE).unwrap_or(0);
                    if x == 0 && y == 0 {
                        self.counter = sam.base();
                        self.divider = 0;
                    }
                    if x % span == 0 {
                        self.latch = sam
                            .ram_address(self.counter)
                            .map_or(255, |offset| ram[offset]);
                        self.increment(sam);
                    }
                    let colour = colour(self.latch, pins, x, y);
                    let offset = (y * WIDTH + x) * 4;
                    self.pixels[offset..offset + 4].copy_from_slice(&COLOURS[colour]);
                    pixel_clock += 2;
                }
            }
            self.phase = end;
            clocks -= elapsed;
            if self.phase == LINE_CLOCKS {
                // HS clears the horizontal counter bits, even when SAM and VDG differ.
                self.counter &= match sam.control & 7 {
                    0 | 2 | 4 | 6 => !0x1e,
                    1 | 3 | 5 => !0x0e,
                    _ => u16::MAX,
                };
                self.phase = 0;
                self.line += 1;
                if self.line == FRAME_LINES {
                    self.line = 0;
                    self.frames = self.frames.saturating_add(1);
                }
            }
        }
    }
    fn increment(&mut self, sam: Sam) {
        let (mask, repeat) = match sam.control & 7 {
            0 => (31, 12),
            1 => (15, 3),
            2 => (31, 3),
            3 => (15, 2),
            4 => (31, 2),
            5 => (15, 1),
            6 => (31, 1),
            _ => (u16::MAX, 1),
        };
        if self.counter & mask == mask {
            self.counter &= !mask;
            self.divider += 1;
            if self.divider >= repeat {
                self.counter = self.counter.wrapping_add(mask.wrapping_add(1));
                self.divider = 0;
            }
        } else {
            self.counter = self.counter.wrapping_add(1);
        }
    }
}
#[cfg(test)]
fn byte(ram: &[u8], sam: Sam, y: usize, index: usize) -> u8 {
    sam.ram_address(sam.display_address(y, index))
        .map_or(255, |offset| ram[offset])
}
#[cfg(test)]
fn pixel(ram: &[u8], sam: Sam, pins: u8, x: usize, y: usize) -> usize {
    let index = if pins & 128 == 0 {
        x / 8
    } else {
        let (scale, bits) = match (pins >> 4) & 7 {
            0 => (4, 2),
            1 | 3 | 5 => (2, 1),
            2 | 4 | 6 => (2, 2),
            _ => (1, 1),
        };
        (x / scale) / (8 / bits)
    };
    colour(byte(ram, sam, y, index), pins, x, y)
}
fn colour(value: u8, pins: u8, x: usize, y: usize) -> usize {
    let css = usize::from((pins >> 3) & 1);
    if pins & 128 != 0 {
        let mode = (pins >> 4) & 7;
        let (scale, bits) = match mode {
            0 => (4, 2),
            1 | 3 | 5 => (2, 1),
            2 | 4 | 6 => (2, 2),
            _ => (1, 1),
        };
        let logical_x = x / scale;
        let elements = 8 / bits;
        let shift = 8 - bits * (logical_x % elements + 1);
        let index = usize::from((value >> shift) & ((1 << bits) - 1));
        if bits == 2 {
            css * 4 + index
        } else if index == 0 {
            8
        } else {
            css * 4
        }
    } else {
        let sx = x % 8;
        let sy = y % 12;
        if value & 128 != 0 {
            if pins & 16 == 0 {
                let bit = 3 - (sy / 6) * 2 - sx / 4;
                if value & (1 << bit) == 0 {
                    8
                } else {
                    usize::from((value >> 4) & 7)
                }
            } else {
                let bit = 5 - (sy / 4) * 2 - sx / 4;
                if value & (1 << bit) == 0 {
                    8
                } else {
                    css * 4 + usize::from(value >> 6)
                }
            }
        } else if pins & 16 != 0 {
            // No external character generator is fitted to a Dragon 32.
            if sx == 1 || value & (1 << (7 - sx)) == 0 {
                8
            } else {
                css * 4
            }
        } else {
            let code = value & 63;
            let glyph = glyphs::glyph(if code < 32 { code + 64 } else { code });
            let ink = (3..10).contains(&sy)
                && (2..7).contains(&sx)
                && glyph[sy - 3] & (1 << (6 - sx)) != 0;
            if ink ^ (value & 64 != 0) {
                if css == 0 { 0 } else { 7 }
            } else {
                8
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Deliberately scalar oracle retained from the original clock-by-clock
    // model. It shares unchanged colour/address rules, not the new scheduler.
    impl Video {
        fn advance_reference(&mut self, clocks: u32, ram: &[u8], sam: Sam, pins: u8) {
            for _ in 0..clocks {
                if (ACTIVE_LINE..ACTIVE_LINE + 192).contains(&self.line)
                    && (ACTIVE_CLOCK..ACTIVE_CLOCK + 512).contains(&self.phase)
                    && self.phase & 1 == 0
                {
                    let x = usize::try_from((self.phase - ACTIVE_CLOCK) / 2).unwrap_or(0);
                    let y = usize::try_from(self.line - ACTIVE_LINE).unwrap_or(0);
                    let span = if pins & 128 != 0 && matches!((pins >> 4) & 7, 0 | 1 | 3 | 5) {
                        16
                    } else {
                        8
                    };
                    if x == 0 && y == 0 {
                        self.counter = sam.base();
                        self.divider = 0;
                    }
                    if x % span == 0 {
                        self.latch = sam
                            .ram_address(self.counter)
                            .map_or(255, |offset| ram[offset]);
                        self.increment(sam);
                    }
                    let colour = colour(self.latch, pins, x, y);
                    let offset = (y * WIDTH + x) * 4;
                    self.pixels[offset..offset + 4].copy_from_slice(&COLOURS[colour]);
                }
                self.phase += 1;
                if self.phase == LINE_CLOCKS {
                    // HS clears the horizontal counter bits, even when SAM and VDG differ.
                    self.counter &= match sam.control & 7 {
                        0 | 2 | 4 | 6 => !0x1e,
                        1 | 3 | 5 => !0x0e,
                        _ => u16::MAX,
                    };
                    self.phase = 0;
                    self.line += 1;
                    if self.line == FRAME_LINES {
                        self.line = 0;
                        self.frames = self.frames.saturating_add(1);
                    }
                }
            }
        }
    }

    fn assert_timing(actual: &Video, expected: &Video) {
        assert_eq!(actual.phase, expected.phase);
        assert_eq!(actual.line, expected.line);
        assert_eq!(actual.frames, expected.frames);
        assert_eq!(actual.counter, expected.counter);
        assert_eq!(actual.divider, expected.divider);
        assert_eq!(actual.latch, expected.latch);
        assert_eq!(actual.sync(), expected.sync());
    }

    #[test]
    fn scheduled_raster_matches_master_clock_oracle_across_modes_and_boundaries() {
        let mut seed = 0x3182_6509_u32;
        let mut random = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed
        };
        let pins = [
            0, 8, 16, 24, 128, 136, 144, 152, 160, 168, 176, 184, 192, 200, 208, 216, 224, 232,
            240, 248,
        ];
        let intervals = [0, 1, 2, 7, 8, 15, 16, 17, 65, 254, 511, 910, 1365, 4096];
        for mode in 0..8_u16 {
            for (index, &pins) in pins.iter().enumerate() {
                let mut ram = vec![0; 32768];
                for value in &mut ram {
                    *value = random().to_le_bytes()[0];
                }
                let sam = Sam {
                    control: mode | [0, 0x2000, 0x4000, 0x6000][index & 3] | 0x1fd0,
                };
                let mut actual = Video::default();
                let mut expected = Video::default();
                let mut remaining = LINE_CLOCKS * FRAME_LINES + ACTIVE_LINE * LINE_CLOCKS + 767;
                let mut calls = 0;
                while remaining != 0 {
                    let clocks = intervals[calls % intervals.len()].min(remaining);
                    // Guest RAM changes between bus intervals, including within
                    // a latched byte. A scheduler must neither prefetch nor reread.
                    let offset = usize::from(random().to_le_bytes()[0]) * 128;
                    ram[offset] = random().to_le_bytes()[0];
                    actual.advance(clocks, &ram, sam, pins);
                    expected.advance_reference(clocks, &ram, sam, pins);
                    assert_timing(&actual, &expected);
                    remaining -= clocks;
                    calls += 1;
                }
                assert_eq!(actual.pixels, expected.pixels, "SAM {mode}, pins {pins}");
            }
        }
    }

    #[test]
    fn raster_bus_interval_changes_are_visible_at_exact_fetch_boundaries() {
        let mut actual = Video::default();
        let mut expected = Video::default();
        let mut ram = vec![0; 32768];
        for (index, value) in ram.iter_mut().enumerate() {
            *value = index.to_le_bytes()[0];
        }
        let mut sam = Sam { control: 0x4016 };
        let lead = ACTIVE_LINE * LINE_CLOCKS + ACTIVE_CLOCK - 1;
        actual.advance(lead, &ram, sam, 0xf0);
        expected.advance_reference(lead, &ram, sam, 0xf0);
        // Change VDG mode, CSS, RAM mapping and display base at every phase,
        // spanning pixel/byte/line boundaries and the next frame's first fetch.
        for clock in 0..LINE_CLOCKS * FRAME_LINES + 512 {
            let pins = (clock.wrapping_mul(17)).to_le_bytes()[0] & 0xf8;
            sam.control = u16::from((clock.wrapping_mul(13)).to_le_bytes()[0])
                | (u16::from((clock / 97).to_le_bytes()[0] & 3) << 13);
            let offset = usize::try_from(clock % 32768).unwrap_or(0);
            ram[offset] = clock.to_le_bytes()[0];
            actual.advance(1, &ram, sam, pins);
            expected.advance_reference(1, &ram, sam, pins);
            assert_timing(&actual, &expected);
        }
        assert_eq!(actual.pixels, expected.pixels);
    }

    #[test]
    fn all_graphics_modes_have_correct_element_width_and_colour_sets() {
        let mut ram = vec![0; 32768];
        for (gm, sam_mode, scale, bits) in [
            (0, 1, 4, 2),
            (1, 1, 2, 1),
            (2, 2, 2, 2),
            (3, 3, 2, 1),
            (4, 4, 2, 2),
            (5, 5, 2, 1),
            (6, 6, 2, 2),
            (7, 6, 1, 1),
        ] {
            let sam = Sam {
                control: 0x4010 | sam_mode,
            };
            ram[0x400] = if bits == 2 { 0x40 } else { 0x80 };
            for css in 0..2 {
                for x in 0..scale {
                    assert_eq!(
                        pixel(&ram, sam, 128 | gm << 4 | css << 3, x, 0),
                        usize::from(css * 4) + usize::from(bits == 2)
                    );
                }
                assert_eq!(
                    pixel(&ram, sam, 128 | gm << 4 | css << 3, scale, 0),
                    if bits == 2 { usize::from(css * 4) } else { 8 }
                );
            }
        }
    }
    #[test]
    fn semigraphics_quadrants_and_sam_vertical_extension() {
        let mut ram = vec![0; 32768];
        let sam = Sam { control: 0x4010 };
        ram[0x400] = 0xb8;
        assert_eq!(pixel(&ram, sam, 0, 0, 0), 3);
        assert_eq!(pixel(&ram, sam, 0, 4, 0), 8);
        assert_eq!(pixel(&ram, sam, 0, 0, 6), 8);
        ram[0x400] = 0xe0;
        assert_eq!(pixel(&ram, sam, 16, 0, 0), 3);
        assert_eq!(pixel(&ram, sam, 24, 0, 0), 7);
        assert_eq!(pixel(&ram, sam, 16, 0, 4), 8);
        let sam = Sam { control: 0x4016 };
        ram[0x420] = 0xaf;
        assert_eq!(pixel(&ram, sam, 0, 0, 1), 2);
    }
    #[test]
    fn pal_frame_and_sync_are_deterministic() {
        let mut video = Video::default();
        let ram = vec![0; 32768];
        let sam = Sam { control: 0x4010 };
        assert_eq!(video.sync(), (false, false));
        video.advance(66, &ram, sam, 0);
        assert_eq!(video.sync(), (true, false));
        video.advance(LINE_CLOCKS * 57 - 66, &ram, sam, 0);
        assert_eq!(video.sync(), (false, true));
        video.advance(LINE_CLOCKS * (FRAME_LINES - 57), &ram, sam, 0);
        assert_eq!(video.frames, 1);
        assert_eq!(video.sync(), (false, false));
        assert!(video.pixels.chunks_exact(4).all(|p| p[3] == 255));
    }
}
