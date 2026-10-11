//! Shared, portable validation of public MAME 0.289 BBC revision metadata.
use eightbit_source_core::{Result, error};
pub const ROMS: &[(&str, usize, u32, &str)] = &[
    ("os12.rom", 16384, 0x3c14_fc70, "bbcb"),
    ("basic2.rom", 16384, 0x7943_4781, "bbcb"),
    ("dnfs120.rom", 16384, 0x8ccd_2157, "bbc_acorn8271"),
    ("cm62024.bin", 16384, 0x98e1_bf9e, "bbcb"),
    ("saa5050", 960, 0x2014_90f3, "saa5050"),
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
pub fn validate_packed(bytes: &[u8]) -> Result<()> {
    if bytes.len() != 65536 + 960 {
        return Err(error(
            "DBBC110",
            "Choose the exact five-file BBC firmware set",
        ));
    }
    let mut offset = 0;
    for (name, size, crc, _) in ROMS {
        let part = &bytes[offset..offset + size];
        if crc32(part) != *crc {
            return Err(error(
                "DBBC110",
                format!("{name} does not match the supported BBC firmware revision"),
            ));
        }
        offset += size;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generated_firmware_and_wrong_sizes_are_rejected_without_native_services() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        for size in [0, 960, 65536, 65536 + 960, 65536 + 961] {
            assert!(validate_packed(&vec![0; size]).is_err());
        }
    }
}
