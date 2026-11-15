//! Immutable imported-media admission. Reconstruction is never executable input.
use eightbit_dragon_emulator::{
    OriginalDevelopmentLaunch, machine_debug::MachineDebugTarget,
    original_runtime::OriginalRuntimeLaunch,
};
use eightbit_dragon_media::reconstruct::{
    ReconstructionInput, reconstruction_hash, reconstruction_input,
};
use eightbit_dragon_media::{create_machine_code, decode_wav_cassette};
use serde::{Deserialize, Serialize};

const MAX_MEDIA_BYTES: usize = 8 * 1024 * 1024;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MediaIdentity {
    schema_version: u32,
    input: ReconstructionInput,
    input_sha256: String,
    cassette_sha256: String,
}

/// Retain an existing cassette's exact bytes. Raw and DECB files get an original,
/// deterministic cassette envelope for the guest ROM's normal CLOADM path.
pub(crate) fn prepare(
    bytes: &[u8],
    input: ReconstructionInput,
) -> Result<(Vec<u8>, MediaIdentity), String> {
    if bytes.is_empty() || bytes.len() > MAX_MEDIA_BYTES {
        return Err("DME1401: select machine-code media of 1–8388608 bytes".into());
    }
    if input == ReconstructionInput::Cartridge {
        return Err("DSW1023: cartridges are unsupported on the website; use the desktop edition with XRoar".into());
    }
    let machine = reconstruction_input(bytes, &input).map_err(|error| error.to_string())?;
    let cassette = if input == ReconstructionInput::Cassette {
        if bytes.starts_with(b"RIFF") {
            decode_wav_cassette(bytes)
                .map_err(|error| error.to_string())?
                .cassette
        } else {
            bytes.to_vec()
        }
    } else {
        create_machine_code(
            "IMPORTED",
            &machine.program,
            machine.load_address,
            machine.execution_address,
        )
        .map_err(|error| error.to_string())?
    };
    let identity = MediaIdentity {
        schema_version: 1,
        input,
        input_sha256: reconstruction_hash(bytes),
        cassette_sha256: reconstruction_hash(&cassette),
    };
    Ok((cassette, identity))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MediaLaunch {
    schema_version: u32,
    input: ReconstructionInput,
    input_sha256: String,
    cassette_sha256: String,
    breakpoint_address: Option<u16>,
}

pub(crate) fn launch(request: &str, bytes: &[u8]) -> Result<OriginalRuntimeLaunch, String> {
    if request.len() > 4096 || bytes.is_empty() || bytes.len() > MAX_MEDIA_BYTES {
        return Err("DME1401: invalid original media identity".into());
    }
    let request: MediaLaunch =
        serde_json::from_str(request).map_err(|_| "DME1401: invalid original media identity")?;
    if request.schema_version != 1 || reconstruction_hash(bytes) != request.input_sha256 {
        return Err("DME1401: selected machine-code media changed; select it again".into());
    }
    let (cassette, identity) = prepare(bytes, request.input)?;
    if identity.cassette_sha256 != request.cassette_sha256 {
        return Err("DME1401: selected cassette changed; select it again".into());
    }
    OriginalRuntimeLaunch::new_machine(
        OriginalDevelopmentLaunch::with_cassette(cassette, None)?,
        request
            .breakpoint_address
            .map(|address| MachineDebugTarget::new(address, Vec::new()))
            .transpose()?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use eightbit_dragon_media::read_machine_code;

    #[test]
    fn original_media_binds_exact_raw_cassette_and_decb_inputs() {
        let payload = [0x86, 0x41, 0xb7, 0x04, 0x00, 0x39];
        let raw = ReconstructionInput::Raw {
            load_address: 0x7000,
            execution_address: 0x7000,
        };
        let cassette = create_machine_code("AUTHORED", &payload, 0x7000, 0x7000).unwrap();
        let mut decb = vec![0, 0, 6, 0x70, 0];
        decb.extend(payload);
        decb.extend([0xff, 0, 0, 0x70, 0]);
        for (bytes, input) in [
            (&payload[..], raw),
            (&cassette, ReconstructionInput::Cassette),
            (&decb, ReconstructionInput::Decb),
        ] {
            let (prepared, identity) = prepare(bytes, input).unwrap();
            assert_eq!(read_machine_code(&prepared).unwrap().program, payload);
            assert_eq!(identity.input_sha256, reconstruction_hash(bytes));
            let json = serde_json::to_string(&identity).unwrap();
            assert!(launch(&json, bytes).is_ok());
            let mut debug: serde_json::Value = serde_json::from_str(&json).unwrap();
            debug["breakpointAddress"] = serde_json::json!(0x7000);
            assert!(launch(&debug.to_string(), bytes).is_ok());
            debug["breakpointAddress"] = serde_json::json!(0x7100);
            assert!(launch(&debug.to_string(), bytes).is_err());
            let mut changed = bytes.to_vec();
            changed[0] ^= 1;
            assert!(launch(&json, &changed).is_err());
            let mut bad: serde_json::Value = serde_json::from_str(&json).unwrap();
            bad["cassetteSha256"] = serde_json::json!("0".repeat(64));
            assert!(launch(&bad.to_string(), bytes).is_err());
            bad["sourceMap"] = serde_json::json!({});
            assert!(launch(&bad.to_string(), bytes).is_err());
        }
        assert_eq!(
            prepare(&cassette, ReconstructionInput::Cassette).unwrap().0,
            cassette
        );
    }

    #[test]
    fn original_media_rejects_cartridge_rom_ranges_and_malformed_input_without_a_guest() {
        assert!(
            prepare(&[0; 2048], ReconstructionInput::Cartridge)
                .err()
                .unwrap()
                .contains("unsupported on the website")
        );
        for bytes in [vec![], vec![0], vec![0xff, 0, 0]] {
            assert!(prepare(&bytes, ReconstructionInput::Cassette).is_err());
        }
        let (cassette, identity) = prepare(
            &[0x39],
            ReconstructionInput::Raw {
                load_address: 0x8000,
                execution_address: 0x8000,
            },
        )
        .unwrap();
        assert!(!cassette.is_empty());
        assert!(launch(&serde_json::to_string(&identity).unwrap(), &[0x39]).is_err());
        assert!(launch("{}", &[0x39]).is_err());
    }
}
