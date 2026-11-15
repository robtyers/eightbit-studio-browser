//! Original PAL VIC-II digital display model. No host fonts or reference renderer.

/// Cropped PAL observation width, retaining a border around the 320-pixel display.
pub const WIDTH: usize = 384;
/// Cropped PAL observation height, retaining a border around the 200-pixel display.
pub const HEIGHT: usize = 272;
/// PHI2 cycles per PAL raster line.
pub const CYCLES_PER_LINE: u8 = 63;
/// PAL raster lines per frame.
pub const LINES_PER_FRAME: u16 = 312;
/// Exact raster-frame clocks (independent of the rounded audio clock rate).
pub const FRAME_CYCLES: u32 = 63 * 312;

pub(crate) struct VideoMemory<'a> {
    pub ram: &'a [u8],
    pub colour: &'a [u8; 1024],
    pub characters: &'a [u8; 4096],
    pub bank: u8,
}
impl VideoMemory<'_> {
    fn byte(&self, offset: u16) -> u8 {
        let address = (u16::from(self.bank & 3) << 14) | (offset & 0x3fff);
        if address & 0x7000 == 0x1000 {
            self.characters[usize::from(address & 0xfff)]
        } else {
            self.ram[usize::from(address)]
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
struct Sprite {
    dma: bool,
    display: bool,
    row: u8,
    repeat: bool,
    pointer: u8,
    bytes: [u8; 3],
}

/// PAL display and raster/DMA state, with two fixed-size palette-index buffers.
// Independent physical pins/flip-flops are intentionally represented as booleans.
#[allow(clippy::struct_excessive_bools)]
pub struct Vic {
    registers: [u8; 64],
    line: u16,
    bus_line: u16,
    cycle: u8,
    frames: u64,
    irq_flags: u8,
    irq_mask: u8,
    raster_compare: u16,
    sprite_collision: u8,
    data_collision: u8,
    sprites: [Sprite; 8],
    display_enabled: bool,
    bad_line: bool,
    vertical_border: bool,
    horizontal_border: bool,
    row_base: u16,
    row_counter: u8,
    refresh_counter: u8,
    display_state: bool,
    matrix: [u8; 40],
    colours: [u8; 40],
    glyphs: [u8; 40],
    drawing: Vec<u8>,
    completed: Vec<u8>,
    light_pen_armed: bool,
    invalid_mode_seen: bool,
}
impl Default for Vic {
    fn default() -> Self {
        Self {
            registers: [0; 64],
            line: 0,
            bus_line: 0,
            cycle: 0,
            frames: 0,
            irq_flags: 0,
            irq_mask: 0,
            raster_compare: 0,
            sprite_collision: 0,
            data_collision: 0,
            sprites: [Sprite::default(); 8],
            display_enabled: false,
            bad_line: false,
            vertical_border: true,
            horizontal_border: true,
            row_base: 0,
            row_counter: 0,
            refresh_counter: 255,
            display_state: false,
            matrix: [0; 40],
            colours: [0; 40],
            glyphs: [0; 40],
            drawing: vec![0; WIDTH * HEIGHT],
            completed: vec![0; WIDTH * HEIGHT],
            light_pen_armed: true,
            invalid_mode_seen: false,
        }
    }
}
impl Vic {
    /// Current zero-based raster line/cycle.
    #[must_use]
    pub const fn position(&self) -> (u16, u8) {
        (self.line, self.cycle)
    }
    /// Completed frames since reset; wraps only after the physical u64 counter span.
    #[must_use]
    pub const fn frames(&self) -> u64 {
        self.frames
    }
    /// Latest complete palette-index frame; no guest time advances when it is read.
    #[must_use]
    pub fn frame(&self) -> &[u8] {
        &self.completed
    }
    /// Aggregate raster/sprite/data/light-pen interrupt output.
    #[must_use]
    pub const fn irq(&self) -> bool {
        self.irq_flags & self.irq_mask & 15 != 0
    }
    /// Whether an invalid mode has been observed since reset (rendered black).
    #[must_use]
    pub const fn invalid_mode_seen(&self) -> bool {
        self.invalid_mode_seen
    }
    /// Active DMA owns PHI2; CPU writes may finish only during the BA lead interval.
    #[must_use]
    pub fn aec_low(&self) -> bool {
        self.dma_at(self.cycle)
    }
    /// BA lead warns read cycles three clocks before a DMA slot.
    #[must_use]
    pub fn ba_low(&self) -> bool {
        (0..=3).any(|ahead| self.dma_at((self.cycle + ahead) % CYCLES_PER_LINE))
    }
    fn dma_at(&self, cycle: u8) -> bool {
        if self.bad_line && (14..54).contains(&cycle) {
            return true;
        }
        for (index, sprite) in self.sprites.iter().enumerate() {
            let start = (57 + index * 2) % 63;
            let starting = self.cycle <= 54
                && cycle >= 54
                && self.registers[21] & (1 << index) != 0
                && self.registers[index * 2 + 1] == self.line.to_le_bytes()[0];
            if (usize::from(cycle) == start || usize::from(cycle) == (start + 1) % 63)
                && (sprite.dma || starting)
            {
                return true;
            }
        }
        false
    }
    /// Latch a physical light-pen falling edge once per frame.
    pub fn light_pen(&mut self) {
        if self.light_pen_armed {
            let x = (u16::from(self.cycle) * 8 + 400) % 504;
            self.registers[19] = u8::try_from(x / 2).unwrap_or(0);
            self.registers[20] = self.line.to_le_bytes()[0];
            self.irq_flags |= 8;
            self.light_pen_armed = false;
        }
    }
    /// Read a mirrored VIC register; collision reads clear their latches.
    pub fn read(&mut self, register: u8) -> u8 {
        let register = register & 63;
        match register {
            17 => (self.registers[17] & 0x7f) | if self.bus_line & 256 != 0 { 0x80 } else { 0 },
            18 => self.bus_line.to_le_bytes()[0],
            22 => self.registers[22] | 0xc0,
            24 => self.registers[24] | 1,
            25 => self.irq_flags | 0x70 | if self.irq() { 0x80 } else { 0 },
            26 => self.irq_mask | 0xf0,
            30 => {
                let value = self.sprite_collision;
                self.sprite_collision = 0;
                value
            }
            31 => {
                let value = self.data_collision;
                self.data_collision = 0;
                value
            }
            32..=46 => self.registers[usize::from(register)] | 0xf0,
            47..=63 => 0xff,
            _ => self.registers[usize::from(register)],
        }
    }
    /// Write register state; IRQ flags acknowledge only the written set bits.
    pub fn write(&mut self, register: u8, value: u8) {
        let register = register & 63;
        match register {
            17 => {
                self.registers[17] = value;
                self.raster_compare = (self.raster_compare & 255) | (u16::from(value & 0x80) << 1);
            }
            18 => self.raster_compare = (self.raster_compare & 256) | u16::from(value),
            19 | 20 | 30 | 31 | 47..=63 => {}
            25 => self.irq_flags &= !(value & 15),
            26 => self.irq_mask = value & 15,
            32..=46 => self.registers[usize::from(register)] = value & 15,
            _ => self.registers[usize::from(register)] = value,
        }
    }
    pub(crate) fn clock(&mut self, memory: &VideoMemory<'_>) -> Option<u8> {
        self.bus_line = self.line;
        // Idle/refresh PHI1 reads also drive the shared data bus.
        let mut fetched = Some(memory.byte(0x3fff));
        if (10..15).contains(&self.cycle) {
            fetched = Some(memory.byte(0x3f00 | u16::from(self.refresh_counter)));
            self.refresh_counter = self.refresh_counter.wrapping_sub(1);
        }
        if self.cycle == 0 {
            if self.line == 0 {
                self.row_base = 0;
                self.display_state = false;
                self.display_enabled = false;
            }
            if self.line == 48 {
                self.display_enabled = self.registers[17] & 0x10 != 0;
            }
            self.bad_line = self.display_enabled
                && (48..=247).contains(&self.line)
                && (self.line & 7) == u16::from(self.registers[17] & 7);
            if self.bad_line {
                self.row_counter = 0;
                self.display_state = true;
            }
            let (top, bottom) = if self.registers[17] & 8 != 0 {
                (51, 251)
            } else {
                (55, 247)
            };
            if self.line == bottom {
                self.vertical_border = true;
            }
            if self.line == top && self.registers[17] & 0x10 != 0 {
                self.vertical_border = false;
            }
            if self.line == self.raster_compare {
                self.irq_flags |= 1;
            }
            for sprite in &mut self.sprites {
                sprite.display = sprite.dma;
            }
        }
        let screen = u16::from(self.registers[24] >> 4) << 10;
        if self.bad_line && (14..54).contains(&self.cycle) {
            let col = usize::from(self.cycle - 14);
            let cell = (self.row_base + u16::try_from(col).unwrap_or(0)) & 0x3ff;
            self.matrix[col] = memory.byte(screen + cell);
            self.colours[col] = memory.colour[usize::from(cell)] & 15;
            fetched = Some(self.matrix[col]);
        }
        if self.display_state && (15..55).contains(&self.cycle) {
            let col = usize::from(self.cycle - 15);
            let bitmap = self.registers[17] & 0x20 != 0;
            let offset = if bitmap {
                (u16::from(self.registers[24] & 8) << 10)
                    + ((self.row_base + u16::try_from(col).unwrap_or(0)) & 0x3ff) * 8
                    + u16::from(self.row_counter)
            } else {
                let code = if self.registers[17] & 0x40 != 0 {
                    self.matrix[col] & 63
                } else {
                    self.matrix[col]
                };
                (u16::from(self.registers[24] & 14) << 10)
                    + u16::from(code) * 8
                    + u16::from(self.row_counter)
            };
            self.glyphs[col] = memory.byte(offset);
            fetched = Some(self.glyphs[col]);
        }
        if self.cycle == 54 {
            for index in 0..8 {
                if self.registers[21] & (1 << index) != 0
                    && self.registers[index * 2 + 1] == self.line.to_le_bytes()[0]
                {
                    let sprite = &mut self.sprites[index];
                    sprite.dma = true;
                    sprite.row = 0;
                    sprite.repeat = false;
                }
            }
        }
        for index in 0..8 {
            let start = u8::try_from((57 + index * 2) % 63).unwrap_or(0);
            let sprite = &mut self.sprites[index];
            if self.cycle == start {
                sprite.pointer = memory.byte(screen + 0x3f8 + u16::try_from(index).unwrap_or(0));
                fetched = Some(sprite.pointer);
            }
            if !sprite.dma {
                continue;
            }
            if self.cycle == start {
                sprite.bytes[0] =
                    memory.byte(u16::from(sprite.pointer) * 64 + u16::from(sprite.row) * 3);
                fetched = Some(sprite.bytes[0]);
            }
            if self.cycle == (start + 1) % 63 {
                for byte in 1..3 {
                    sprite.bytes[byte] = memory.byte(
                        u16::from(sprite.pointer) * 64
                            + u16::from(sprite.row) * 3
                            + u16::try_from(byte).unwrap_or(0),
                    );
                }
                fetched = Some(sprite.bytes[2]);
            }
        }
        self.draw_cycle();
        if self.cycle == 15 {
            for index in 0..8 {
                let sprite = &mut self.sprites[index];
                if !sprite.dma {
                    continue;
                }
                let expanded = self.registers[23] & (1 << index) != 0;
                if !expanded || sprite.repeat {
                    sprite.row += 1;
                    if sprite.row >= 21 {
                        sprite.dma = false;
                    }
                }
                if expanded {
                    sprite.repeat = !sprite.repeat;
                }
            }
        }
        if self.cycle == 57 && self.display_state {
            if self.row_counter == 7 {
                self.row_base = (self.row_base + 40) & 0x3ff;
                self.display_state = false;
            }
            self.row_counter = (self.row_counter + 1) & 7;
        }
        self.cycle += 1;
        if self.cycle == CYCLES_PER_LINE {
            self.cycle = 0;
            self.line += 1;
            if self.line == LINES_PER_FRAME {
                self.line = 0;
                self.frames = self.frames.wrapping_add(1);
                self.light_pen_armed = true;
                std::mem::swap(&mut self.drawing, &mut self.completed);
            }
        }
        fetched
    }
    fn draw_cycle(&mut self) {
        for dot in 0..8u16 {
            let x = (u16::from(self.cycle) * 8 + 400 + dot) % 504;
            let (left, right) = if self.registers[22] & 8 != 0 {
                (24, 344)
            } else {
                (31, 335)
            };
            if x == right {
                self.horizontal_border = true;
            }
            if x == left && !self.vertical_border {
                self.horizontal_border = false;
            }
            let (mut colour, foreground) = self.graphics(x);
            let mut occupants = 0u8;
            let mut first = None;
            for index in 0..8 {
                if let Some(pixel) = self.sprite_pixel(index, x) {
                    occupants |= 1 << index;
                    if first.is_none() {
                        first = Some((index, pixel));
                    }
                }
            }
            if occupants.count_ones() > 1 {
                self.sprite_collision |= occupants;
                self.irq_flags |= 4;
            }
            if foreground && occupants != 0 {
                self.data_collision |= occupants;
                self.irq_flags |= 2;
            }
            if let Some((index, pixel)) = first
                && (!foreground || self.registers[27] & (1 << index) == 0)
            {
                colour = pixel;
            }
            if self.horizontal_border || self.vertical_border {
                colour = self.registers[32];
            }
            // x is the VIC sprite coordinate; viewport starts eight pixels before 0.
            let view_x = if x >= 496 {
                usize::from(x - 496)
            } else {
                usize::from(x) + 8
            };
            if view_x < WIDTH && (14..286).contains(&self.line) {
                self.drawing[usize::from(self.line - 14) * WIDTH + view_x] = colour & 15;
            }
        }
    }
    fn graphics(&mut self, x: u16) -> (u8, bool) {
        let background = self.registers[33];
        if !(24..344).contains(&x) || !self.display_state {
            return (background, false);
        }
        let scroll = i32::from(self.registers[22] & 7);
        let position = i32::from(x) - 24 - scroll;
        if !(0..320).contains(&position) {
            return (background, false);
        }
        let position = usize::try_from(position).unwrap_or(0);
        let col = position / 8;
        let bit = 7 - position % 8;
        let byte = self.glyphs[col];
        let matrix = self.matrix[col];
        let colour = self.colours[col];
        let bitmap = self.registers[17] & 0x20 != 0;
        let extended = self.registers[17] & 0x40 != 0;
        let multi = self.registers[22] & 0x10 != 0;
        if extended && (bitmap || multi) {
            self.invalid_mode_seen = true;
            return (0, false);
        }
        if multi && (bitmap || colour & 8 != 0) {
            let pair = (byte >> ((bit / 2) * 2)) & 3;
            let palette = if bitmap {
                [background, matrix >> 4, matrix & 15, colour]
            } else {
                [
                    background,
                    self.registers[34],
                    self.registers[35],
                    colour & 7,
                ]
            };
            (palette[usize::from(pair)], pair & 2 != 0)
        } else {
            let set = byte & (1 << bit) != 0;
            let (fg, bg) = if bitmap {
                (matrix >> 4, matrix & 15)
            } else if extended {
                (colour, self.registers[33 + usize::from(matrix >> 6)])
            } else {
                (colour, background)
            };
            (if set { fg } else { bg }, set)
        }
    }
    fn sprite_pixel(&self, index: usize, x: u16) -> Option<u8> {
        let sprite = self.sprites[index];
        if !sprite.display {
            return None;
        }
        let start = u16::from(self.registers[index * 2])
            | if self.registers[16] & (1 << index) != 0 {
                256
            } else {
                0
            };
        let expansion = if self.registers[29] & (1 << index) != 0 {
            2
        } else {
            1
        };
        let delta = x.checked_sub(start)? / expansion;
        if delta >= 24 {
            return None;
        }
        let value = (u32::from(sprite.bytes[0]) << 16)
            | (u32::from(sprite.bytes[1]) << 8)
            | u32::from(sprite.bytes[2]);
        if self.registers[28] & (1 << index) != 0 {
            let pair = (value >> ((11 - u32::from(delta / 2)) * 2)) & 3;
            match pair {
                0 => None,
                1 => Some(self.registers[37]),
                2 => Some(self.registers[39 + index]),
                _ => Some(self.registers[38]),
            }
        } else if value & (1 << (23 - u32::from(delta))) != 0 {
            Some(self.registers[39 + index])
        } else {
            None
        }
    }
}
