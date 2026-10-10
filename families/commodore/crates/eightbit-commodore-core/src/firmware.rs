//! Portable firmware revision checks shared by native VICE and browser integration.
// Public revision fingerprints; see docs/eightbit-studio-6502.md for provenance.
pub const ROMS: &[(&str, usize, u32)] = &[
    ("basic-901226-01.bin", 8192, 0xf833_d117),
    ("kernal-901227-03.bin", 8192, 0xdbe3_e7c7),
    ("chargen-901225-01.bin", 4096, 0xec42_72ee),
];
#[must_use]
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 == 1 { 0xedb8_8320 } else { 0 };
        }
    }
    !crc
}

pub fn validate_packed(bytes: &[u8]) -> eightbit_source_core::Result<()> {
    if bytes.len() != 20480 {
        return Err(eightbit_source_core::error(
            "DC64110",
            "C64 firmware requires BASIC, KERNAL and character ROMs",
        ));
    }
    let mut pos = 0;
    for (name, size, expected) in ROMS {
        if crc32(&bytes[pos..pos + size]) != *expected {
            return Err(eightbit_source_core::error(
                "DC64110",
                format!("{name} does not match the supported C64 firmware revision"),
            ));
        }
        pos += size;
    }
    Ok(())
}
