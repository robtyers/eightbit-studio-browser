//! Closed PAL machine identities; physical memory remains independent of ROM size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    Zx80_1k,
    Zx80_16k,
    Zx81_1k,
    Zx81_16k,
}
impl Model {
    #[must_use]
    pub fn from_target(target: &str) -> Option<Self> {
        match target {
            "zx80-1k" => Some(Self::Zx80_1k),
            "zx80-16k" => Some(Self::Zx80_16k),
            "zx81-1k" => Some(Self::Zx81_1k),
            "zx81-16k" => Some(Self::Zx81_16k),
            _ => None,
        }
    }
    #[must_use]
    pub const fn zx80(self) -> bool {
        matches!(self, Self::Zx80_1k | Self::Zx80_16k)
    }
    #[must_use]
    pub const fn ram_bytes(self) -> usize {
        if matches!(self, Self::Zx80_1k | Self::Zx81_1k) {
            1024
        } else {
            16384
        }
    }
    #[must_use]
    pub const fn rom_bytes(self) -> usize {
        if self.zx80() { 4096 } else { 8192 }
    }
    #[must_use]
    pub const fn ram_top(self) -> u16 {
        if matches!(self, Self::Zx80_1k | Self::Zx81_1k) {
            0x4400
        } else {
            0x8000
        }
    }
    #[must_use]
    pub const fn image_start(self) -> u16 {
        if self.zx80() { 0x4000 } else { 0x4009 }
    }
    #[must_use]
    pub const fn programme_start(self) -> u16 {
        if self.zx80() { 0x4028 } else { 0x407d }
    }
    #[must_use]
    pub const fn extension(self) -> &'static str {
        if self.zx80() { "o" } else { "p" }
    }
}
