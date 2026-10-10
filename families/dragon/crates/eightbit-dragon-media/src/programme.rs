//! Content-based cassette routing shared by desktop import callers.
use crate::{
    BasicMediaInspection, MediaFormatHint, decode_wav_cassette, inspect_basic_media,
    read_machine_code,
};
use serde::Serialize;

/// Read-only programme classification; never launches or writes source.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ProgrammeInspection {
    /// Canonical editable BASIC and original transport provenance.
    Basic {
        /// Validated inspection.
        inspection: BasicMediaInspection,
    },
    /// A machine-code recording suitable for a separate disassembly workflow.
    MachineCode {
        /// Validated name.
        filename: String,
        /// First payload address.
        #[serde(rename = "loadAddress")]
        load_address: u16,
        /// Cassette execution address.
        #[serde(rename = "executionAddress")]
        execution_address: u16,
        /// Exact payload length.
        #[serde(rename = "payloadBytes")]
        payload_bytes: usize,
    },
}
/// Identify one supported programme by content, including supported PCM WAV.
pub fn inspect_programme(bytes: &[u8]) -> Result<ProgrammeInspection, String> {
    if bytes.len() > crate::MAX_WAV_BYTES {
        return Err("Programme input exceeds 64 MiB.".to_owned());
    }
    if let Ok(inspection) = inspect_basic_media(bytes, MediaFormatHint::Auto).or_else(|_| {
        crate::inspect_basic_media_for_dialect(bytes, MediaFormatHint::Auto, "dragon64-basic")
    }) {
        return Ok(ProgrammeInspection::Basic { inspection });
    }
    let cassette = if bytes.starts_with(b"RIFF") {
        decode_wav_cassette(bytes)
            .map_err(|error| error.to_string())?
            .cassette
    } else {
        bytes.to_vec()
    };
    let machine = read_machine_code(&cassette).map_err(|error| error.to_string())?;
    Ok(ProgrammeInspection::MachineCode {
        filename: machine.filename,
        load_address: machine.load_address,
        execution_address: machine.execution_address,
        payload_bytes: machine.program.len(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn machine_cas_and_wav_route_identically_and_corruption_is_rejected() {
        let cas = crate::create_machine_code("TEST", &[0x39], 0x4000, 0x4000).unwrap();
        for bytes in [cas.clone(), crate::encode_cassette_wav(&cas).unwrap()] {
            assert!(matches!(
                inspect_programme(&bytes).unwrap(),
                ProgrammeInspection::MachineCode {
                    payload_bytes: 1,
                    ..
                }
            ));
        }
        assert!(inspect_programme(&cas[..cas.len() / 2]).is_err());
        assert!(inspect_programme(&[]).is_err());
    }
}
