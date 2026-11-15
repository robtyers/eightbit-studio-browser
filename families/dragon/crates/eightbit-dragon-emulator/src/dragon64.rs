//! Original D64 firmware paging. Only immutable common guest gates own controls.
//! Frames follow the guest stack, so non-local BASIC error unwinds cannot restore
//! an abandoned page. This is bounded ROM storage, never additional guest RAM.
use crate::Firmware;
include!("../../../firmware/basic64/native-controls.rs");
pub const BASIC64_COMPATIBILITY: &[u8; 32768] =
    include_bytes!("../../../firmware/basic64/compatibility.rom");
pub const BASIC64_NATIVE: &[u8] = include_bytes!("../../../firmware/basic64/native.rom");
pub const BASIC64_MANIFEST: &str = include_str!("../../../firmware/basic64/manifest.json");
const MAX_PAGE_FRAMES: usize = 4096;

#[derive(Default)]
pub(crate) struct Paging {
    pub page: u8,
    frames: Vec<(u16, u8)>,
    pub fault: bool,
}
impl Paging {
    pub fn control(&mut self, pc: u16, stack: u16, address: u16, value: u8) {
        if !D64_CONTROLS.contains(&(pc, address)) {
            return;
        }
        match address {
            0xff40 => {
                let Some(owner) = stack.checked_add(2) else {
                    self.fault = true;
                    return;
                };
                self.frames.retain(|(saved, _)| *saved > owner);
                let pages = (BASIC64_NATIVE.len() - 16384) / 4096;
                if value == 0 || usize::from(value) > pages || self.frames.len() == MAX_PAGE_FRAMES
                {
                    self.fault = true;
                    return;
                }
                self.frames.push((owner, self.page));
                self.page = value;
            }
            0xff41 => {
                let owner = stack.checked_add(1);
                let frame = self
                    .frames
                    .iter()
                    .rposition(|(saved, _)| Some(*saved) == owner);
                if let Some(index) = frame {
                    self.page = self.frames[index].1;
                    self.frames.truncate(index);
                } else {
                    self.fault = true;
                }
            }
            0xff42 => {
                self.page = 0;
                self.frames.clear();
            }
            _ => {}
        }
    }
}
impl Firmware {
    pub(crate) fn is_dragon64(self) -> bool {
        matches!(self, Self::Dragon64Compatibility | Self::Dragon64Native)
    }
    pub(crate) fn ram_end(self) -> u16 {
        if self == Self::Dragon64Native {
            0xbfff
        } else {
            0x7fff
        }
    }
    pub(crate) fn statement_hook(self) -> u16 {
        if self == Self::Dragon64Native {
            0xc4d6
        } else {
            0x84d6
        }
    }
}
