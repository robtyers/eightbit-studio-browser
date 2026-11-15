//! MC6883 control and logical addressing rules from the Motorola SAM datasheet.
#[derive(Default, Clone, Copy)]
pub(crate) struct Sam {
    pub control: u16,
}
impl Sam {
    pub fn write(&mut self, address: u16) {
        let bit = (address - 0xffc0) / 2;
        if address & 1 == 0 {
            self.control &= !(1 << bit);
        } else {
            self.control |= 1 << bit;
        }
    }
    pub fn all_ram(self) -> bool {
        self.control & 0x8000 != 0
    }
    pub fn base(self) -> u16 {
        ((self.control >> 3) & 127) << 9
    }
    pub fn period(self, address: u16) -> u32 {
        match (self.control >> 11) & 3 {
            0 => 16,
            1 if address < 0x8000 || (0xff00..=0xff1f).contains(&address) => 16,
            _ => 8,
        }
    }
    pub fn ram_address(self, address: u16) -> Option<usize> {
        match (self.control >> 13) & 3 {
            // The unexpanded board has one 32K bank on RAS0. RAS1 is not another bank.
            0 if address & 0x1000 != 0 => None,
            0 => Some(usize::from(
                (address & 0x7f) | ((address & 0xfc0) << 2) | 0x80,
            )),
            1 if address & 0x4000 != 0 => None,
            1 => Some(usize::from(
                (address & 0x7f) | ((address & 0x3f80) << 1) | 0x80,
            )),
            _ => Some(usize::from(address & 0x7fff)),
        }
    }
    #[cfg(test)]
    pub fn display_address(self, y: usize, byte: usize) -> u16 {
        let (vertical_divisor, row_mask) = match self.control & 7 {
            0 => (12, 31),
            1 => (3, 15),
            2 => (3, 31),
            3 => (2, 15),
            4 => (2, 31),
            5 => (1, 15),
            6 => (1, 31),
            _ => (1, 0xffff),
        };
        let count = (y / vertical_divisor) * (row_mask + 1) + byte;
        // Each DMA scanline carries on without a horizontal reset.
        let count = if self.control & 7 == 7 {
            y * 32 + byte
        } else {
            count
        };
        self.base()
            .wrapping_add(u16::try_from(count).unwrap_or(u16::MAX))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn control_bits_and_clocks() {
        let mut s = Sam::default();
        for bit in 0..16_u16 {
            s.write(0xffc1 + bit * 2);
            assert_eq!(s.control & (1 << bit), 1 << bit);
        }
        for bit in 0..16_u16 {
            s.write(0xffc0 + bit * 2);
            assert_eq!(s.control & (1 << bit), 0);
        }
        s.control = 1 << 11;
        assert_eq!(s.period(0x7000), 16);
        assert_eq!(s.period(0x8000), 8);
        assert_eq!(s.period(0xff00), 16);
        assert_eq!(s.period(0xff20), 8);
        s.control = 2 << 11;
        assert_eq!(s.period(0), 8);
    }
    #[test]
    fn display_modes_and_offsets() {
        for (mode, height, stride) in [
            (0, 12, 32),
            (1, 3, 16),
            (2, 3, 32),
            (3, 2, 16),
            (4, 2, 32),
            (5, 1, 16),
            (6, 1, 32),
        ] {
            let s = Sam {
                control: 0x10 | mode,
            };
            assert_eq!(s.display_address(0, 0), 0x400);
            assert_eq!(s.display_address(height - 1, 0), 0x400);
            assert_eq!(s.display_address(height, 0), 0x400 + stride);
        }
    }
}
