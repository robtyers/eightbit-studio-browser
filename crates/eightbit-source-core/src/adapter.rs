//! Immutable, compiled family tool policies. These callbacks never come from project input.
use crate::{Profile, Result};
use std::{path::Path, sync::atomic::AtomicBool};
pub type ToolProbe = fn(&Path, &AtomicBool) -> Result<String>;
pub type Assembler = fn(&str, &Path, &Profile, &AtomicBool) -> Result<(Vec<u8>, String)>;
pub type PrepareRun = fn(&Profile, &Path, &Path, &Path) -> Result<(Vec<String>, String)>;
pub struct Adapter {
    pub runtime_name: &'static str,
    pub assembler_name: &'static str,
    pub probe_runtime: ToolProbe,
    pub probe_assembler: ToolProbe,
    pub validate_assembly: fn(&str, u16) -> Result<()>,
    pub assemble: Assembler,
    pub validate_firmware: fn(&Path, &Profile) -> Result<String>,
    pub prepare_run: PrepareRun,
}
impl crate::Engine {
    #[must_use]
    pub fn adapter(&self) -> Option<&'static Adapter> {
        self.families.first()?.profiles.first()?.adapter
    }
}
