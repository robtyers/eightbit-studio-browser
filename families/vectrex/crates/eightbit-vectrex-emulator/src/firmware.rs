//! Reproducible independently authored guest executive; no downloaded image.
use crate::{Result, error};
pub const ID: &str = "eightbit-vectrex-original-development-0.5";
pub const SOURCE: &str = include_str!("../../../firmware/original/executive.asm");
pub fn image() -> Result<Vec<u8>> {
    let assembly = eightbit_m6809_core::assembler::assemble(SOURCE, 0xe000, "BOOT")
        .map_err(|_| error("VXO1003", "Original executive source failed validation"))?;
    if assembly.bytes.len() != 8192 || assembly.entry_address != 0xe000 {
        return Err(error("VXO1003", "Original executive identity is invalid"));
    }
    Ok(assembly.bytes)
}
