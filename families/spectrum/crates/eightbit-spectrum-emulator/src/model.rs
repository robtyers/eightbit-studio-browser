//! Family machine facts; private firmware capacity is independent of guest RAM.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Model {
    #[default]
    Spectrum48,
    Spectrum128,
    Plus2,
}
impl Model {
    #[must_use]
    pub fn from_target(target: &str) -> Option<Self> {
        match target {
            "spectrum48" => Some(Self::Spectrum48),
            "spectrum128" => Some(Self::Spectrum128),
            "spectrum-plus2" => Some(Self::Plus2),
            _ => None,
        }
    }
    #[must_use]
    pub const fn banked(self) -> bool {
        !matches!(self, Self::Spectrum48)
    }
    #[must_use]
    pub const fn cpu_hz(self) -> u32 {
        if self.banked() {
            3_546_900
        } else {
            super::CPU_HZ
        }
    }
    #[must_use]
    pub const fn frame_t_states(self) -> u32 {
        if self.banked() {
            70_908
        } else {
            super::FRAME_T_STATES
        }
    }
    pub(crate) const fn line_t_states(self) -> u32 {
        if self.banked() {
            228
        } else {
            super::LINE_T_STATES
        }
    }
    pub(crate) const fn display_start(self) -> u32 {
        if self.banked() {
            14_364
        } else {
            super::DISPLAY_START
        }
    }
    pub(crate) const fn contention_start(self) -> u32 {
        if self.banked() {
            14_361
        } else {
            super::CONTENTION_START
        }
    }
    pub(crate) const fn identity(self) -> u8 {
        match self {
            Self::Spectrum48 => 0,
            Self::Spectrum128 => 1,
            Self::Plus2 => 2,
        }
    }
}
