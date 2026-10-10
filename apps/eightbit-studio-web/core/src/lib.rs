//! Bounded application composition of authoritative portable family libraries.
//! GPL-3.0-or-later; reused Apache-2.0 libraries retain their original notices.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::panic))]

use eightbit_dragon_basic_core::{Dialect, analyse, detokenise, tokenise};
use eightbit_dragon_media::reconstruct::{
    ReconstructionInput, ReconstructionOptions, reconstruct_machine_code,
};
use eightbit_dragon_media::{decode_wav_cassette, read_tokenised_basic};
use eightbit_dragon_mobile::{AuthoringSession, MobileError, inspect_firmware};
use serde::Deserialize;
use serde_json::{Value, json};
use wasm_bindgen::prelude::wasm_bindgen;

mod machines;
mod presentation;
mod project;

/// Browser media limit is deliberately lower than the desktop's WAV limit.
pub const MAX_BROWSER_FILE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
enum Request {
    Machines {
        #[serde(rename = "schemaVersion")]
        schema_version: u32,
    },
    TestProject {
        #[serde(rename = "schemaVersion")]
        schema_version: u32,
        project: project::Snapshot,
    },
    CreateProject {
        #[serde(rename = "schemaVersion")]
        schema_version: u32,
        name: String,
        kind: project::Kind,
        source: String,
        #[serde(default = "default_target")]
        target: String,
    },
    ImportProject {
        #[serde(rename = "schemaVersion")]
        schema_version: u32,
    },
    ExportProject {
        #[serde(rename = "schemaVersion")]
        schema_version: u32,
        project: project::Snapshot,
    },
    Presentation {
        #[serde(rename = "schemaVersion")]
        schema_version: u32,
        source: String,
        #[serde(default = "default_target")]
        target: String,
    },
    Build {
        #[serde(rename = "schemaVersion")]
        schema_version: u32,
        source: String,
    },
    Test {
        #[serde(rename = "schemaVersion")]
        schema_version: u32,
        source: String,
    },
    Decompile {
        #[serde(rename = "schemaVersion")]
        schema_version: u32,
        input: ReconstructionInput,
        #[serde(default = "default_target")]
        target: String,
    },
    Firmware {
        #[serde(rename = "schemaVersion")]
        schema_version: u32,
        #[serde(default = "default_target")]
        target: String,
    },
}

fn default_target() -> String {
    "dragon32".into()
}

fn failure(message: &str) -> Value {
    json!({"schemaVersion": 1, "ok": false, "message": message})
}

fn success(result: Value) -> Value {
    let mut response = json!({"schemaVersion": 1, "ok": true});
    response["result"] = result;
    response
}

/// Process one explicit, versioned request. There is no filesystem/network authority.
/// Bytes cross separately to avoid expanding media into a JSON integer array on input.
#[wasm_bindgen]
#[must_use]
pub fn browser_request(metadata: &str, bytes: &[u8]) -> String {
    let value = process(metadata, bytes);
    serde_json::to_string(&value).unwrap_or_else(|_| {
        "{\"schemaVersion\":1,\"ok\":false,\"message\":\"DSW1000: cannot encode browser response\"}".into()
    })
}

fn process(metadata: &str, bytes: &[u8]) -> Value {
    // JSON escaping can expand a bounded 1 MiB source by up to six times.
    if metadata.len() > 7 * 1024 * 1024 || bytes.len() > MAX_BROWSER_FILE_BYTES {
        return failure("DSW1001: browser input exceeds its size limit");
    }
    let Ok(request) = serde_json::from_str::<Request>(metadata) else {
        return failure("DSW1002: invalid browser request");
    };
    match request {
        Request::Machines { schema_version } => {
            if schema_version == 1 && bytes.is_empty() {
                success(machines::catalog())
            } else {
                failure("DSW1002: invalid browser request")
            }
        }
        Request::TestProject {
            schema_version,
            project,
        } => {
            if schema_version != 1 || !bytes.is_empty() {
                return failure("DSW1002: invalid browser request");
            }
            if let Err(error) = project::validate(&project) {
                return failure(&error);
            }
            if project.target != "dragon32" {
                return match machines::build(&project.target, &project.source) {
                    Ok(result) => success(result),
                    Err(error) => failure(&error),
                };
            }
            if project.kind != project::Kind::Basic {
                return failure("DSW1012: assembly builds require the desktop edition");
            }
            if let Some(manifest) = project.manifest.as_deref() {
                let config = match eightbit_dragon_project::parse_project_config(manifest) {
                    Ok(config) => config,
                    Err(error) => return failure(&error.to_string()),
                };
                if let Some(entry_line) = config.basic.entry_line {
                    let dialect = match Dialect::dragon32() {
                        Ok(dialect) => dialect,
                        Err(error) => return failure(&error.to_string()),
                    };
                    let analysis = analyse(&project.source, &dialect);
                    if let Some(diagnostic) =
                        eightbit_dragon_basic_core::configured_entry_diagnostic(
                            &analysis.program,
                            analysis.program.lines.iter().map(|line| line.number).min(),
                            entry_line,
                        )
                    {
                        return json!({"schemaVersion":1,"ok":false,"message":"Correct the BASIC project entry before building", "diagnostics":[diagnostic]});
                    }
                }
            }
            build(project.source, true)
        }
        Request::CreateProject {
            schema_version,
            name,
            kind,
            source,
            target,
        } => {
            if schema_version != 1 || !bytes.is_empty() {
                return failure("DSW1002: invalid browser request");
            }
            match project::create_for(name, kind, source, target) {
                Ok(project) => success(
                    json!({"entryPath":project::validate(&project).ok(), "project":project}),
                ),
                Err(error) => failure(&error),
            }
        }
        Request::ImportProject { schema_version } => {
            if schema_version != 1 {
                return failure("DSW1002: invalid browser request");
            }
            match project::import(bytes) {
                Ok(project) => success(
                    json!({"entryPath":project::validate(&project).ok(), "project":project}),
                ),
                Err(error) => failure(&error),
            }
        }
        Request::ExportProject {
            schema_version,
            project,
        } => {
            if schema_version != 1 || !bytes.is_empty() {
                return failure("DSW1002: invalid browser request");
            }
            match project::export(&project) {
                Ok(archive) => success(json!({"archive":archive})),
                Err(error) => failure(&error),
            }
        }
        Request::Build {
            schema_version,
            source,
        }
        | Request::Test {
            schema_version,
            source,
        }
        | Request::Presentation {
            schema_version,
            source,
            ..
        } if schema_version != 1 || !bytes.is_empty() => {
            let _ = source;
            failure("DSW1002: invalid browser request")
        }
        Request::Build { source, .. } => build(source, false),
        Request::Test { source, .. } => build(source, true),
        Request::Presentation { source, target, .. } => {
            if target == "dragon32" {
                success(presentation::source_highlights(&source))
            } else if machines::profile(&target).is_ok() {
                success(json!({"highlights":[]}))
            } else {
                failure("DSW1020: unsupported browser computer")
            }
        }
        Request::Decompile {
            schema_version,
            input,
            target,
        } => {
            if schema_version != 1 {
                return failure("DSW1002: invalid browser request");
            }
            if target == "dragon32" {
                decompile(bytes, input)
            } else if !matches!(input, ReconstructionInput::Cassette) {
                failure("DSW1021: raw machine code recovery is only implemented for 6809")
            } else {
                match machines::recover(&target, bytes) {
                    Ok(source) => success(json!({"kind":"basic", "source":source, "warnings":[]})),
                    Err(error) => failure(&error),
                }
            }
        }
        Request::Firmware {
            schema_version,
            target,
        } => {
            if schema_version != 1 {
                return failure("DSW1002: invalid browser request");
            }
            if target != "dragon32" {
                return match machines::firmware(&target, bytes) {
                    Ok(result) => success(result),
                    Err(error) => failure(&error),
                };
            }
            match inspect_firmware(bytes) {
                Ok(receipt) => success(json!(receipt)),
                Err(error) => failure(&error.to_string()),
            }
        }
    }
}

fn build(source: String, verify: bool) -> Value {
    let result = (|| {
        let mut session = AuthoringSession::new()?;
        session.edit(1, source)?;
        session.build(1)?;
        Ok::<_, MobileError>(session)
    })();
    let session = match result {
        Ok(session) => session,
        Err(MobileError::Diagnostics { diagnostics }) => {
            return json!({"schemaVersion":1,"ok":false,"message":"Correct the BASIC diagnostics before building", "diagnostics": diagnostics});
        }
        Err(error) => return failure(&error.to_string()),
    };
    let Some(built) = session.current_build() else {
        return failure("DSW1003: current build is unavailable");
    };
    if verify {
        let round_trip = (|| {
            let decoded = read_tokenised_basic(&built.cassette).ok()?;
            let dialect = Dialect::dragon32().ok()?;
            let listing = detokenise(&decoded.program, &dialect).ok()?;
            let tokens = tokenise(&analyse(&listing.source, &dialect).program, &dialect).ok()?;
            Some(decoded.program == built.tokens && tokens.bytes == built.tokens)
        })();
        if round_trip != Some(true) {
            return failure("DSW1004: cassette/token/listing round trip failed");
        }
    }
    success(json!({
        "summary": built.summary, "cassette": built.cassette,
        "diagnostics": built.diagnostics, "roundTripVerified": verify,
    }))
}

fn decompile(bytes: &[u8], input: ReconstructionInput) -> Value {
    let cassette;
    let original = if matches!(input, ReconstructionInput::Cassette) && bytes.starts_with(b"RIFF") {
        match decode_wav_cassette(bytes) {
            Ok(decoded) => {
                cassette = decoded.cassette;
                cassette.as_slice()
            }
            Err(error) => return failure(&error.to_string()),
        }
    } else {
        bytes
    };
    if matches!(input, ReconstructionInput::Cassette)
        && let Ok(decoded) = read_tokenised_basic(original)
    {
        let Ok(dialect) = Dialect::dragon32() else {
            return failure("DSI1003: cannot load the built-in Dragon 32 BASIC dialect");
        };
        return match detokenise(&decoded.program, &dialect) {
            Ok(listing) => success(
                json!({"kind":"basic", "source":listing.source, "warnings":listing.warnings, "cassette": original}),
            ),
            Err(error) => failure(&error.to_string()),
        };
    }
    let options = ReconstructionOptions {
        input,
        ..ReconstructionOptions::default()
    };
    match reconstruct_machine_code(original, &options) {
        Ok(preview) => success(
            json!({"kind":"assembly", "source":preview.source, "warnings":preview.warnings, "preview":preview}),
        ),
        Err(error) => failure(&error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::needless_pass_by_value)]
    fn request(value: Value, bytes: &[u8]) -> Value {
        serde_json::from_str(&browser_request(&value.to_string(), bytes)).unwrap()
    }

    #[test]
    fn project_build_honours_the_desktop_entry_contract_and_preserves_support_files() {
        let created = request(
            json!({"schemaVersion":1,"operation":"create-project","name":"Web game","kind":"basic","source":"10 END\n"}),
            &[],
        );
        assert_eq!(created["ok"], true);
        assert!(
            created["result"]["project"]["supportFiles"]["AGENTS.md"]
                .as_str()
                .unwrap()
                .contains("Source control")
        );
        let mut project = created["result"]["project"].clone();
        assert_eq!(
            request(
                json!({"schemaVersion":1,"operation":"test-project","project":project}),
                &[]
            )["result"]["roundTripVerified"],
            true
        );
        project["source"] = json!("20 END\n");
        let failed = request(
            json!({"schemaVersion":1,"operation":"test-project","project":project}),
            &[],
        );
        assert_eq!(failed["ok"], false);
        assert_eq!(failed["diagnostics"][0]["code"], "DSB1005");
        assert!(failed.get("cassette").is_none());
    }
    #[test]
    fn original_beginner_game_builds_and_presentation_never_returns_launchable_media() {
        let source =
            include_str!("../../../../packages/browser-workbench/src/guess-the-number.bas");
        let built = request(
            json!({"schemaVersion":1,"operation":"test","source":source}),
            &[],
        );
        assert_eq!(built["ok"], true);
        assert_eq!(built["result"]["roundTripVerified"], true);
        let presented = request(
            json!({"schemaVersion":1,"operation":"presentation","source":source}),
            &[],
        );
        assert_eq!(presented["ok"], true);
        assert!(
            presented["result"]["highlights"]
                .as_array()
                .is_some_and(|ranges| !ranges.is_empty())
        );
        assert!(presented["result"].get("cassette").is_none());
        for metadata in [
            json!({"schemaVersion":2,"operation":"presentation","source":source}),
            json!({"schemaVersion":1,"operation":"presentation","source":source,"hook":"shell"}),
        ] {
            assert_eq!(request(metadata, &[])["ok"], false);
        }
        assert_eq!(
            request(
                json!({"schemaVersion":1,"operation":"presentation","source":source}),
                &[1]
            )["ok"],
            false
        );
    }

    #[test]
    fn actual_bridge_build_test_and_decompile_preserve_basic_semantics() {
        let source = "10 PRINT \"BROWSER GAME\"\n20 GOTO 10\n";
        let result = request(
            json!({"schemaVersion":1,"operation":"test","source":source}),
            &[],
        );
        assert_eq!(result["ok"], true);
        assert_eq!(result["result"]["roundTripVerified"], true);
        let bytes: Vec<u8> = serde_json::from_value(result["result"]["cassette"].clone()).unwrap();
        let decoded = request(
            json!({"schemaVersion":1,"operation":"decompile","input":{"format":"cassette"}}),
            &bytes,
        );
        assert_eq!(decoded["result"]["source"], source);
        let wav = eightbit_dragon_media::encode_cassette_wav(&bytes).unwrap();
        let decoded_wav = request(
            json!({"schemaVersion":1,"operation":"decompile","input":{"format":"cassette"}}),
            &wav,
        );
        assert_eq!(decoded_wav["result"]["source"], source);
        assert_eq!(
            request(
                json!({"schemaVersion":1,"operation":"decompile","input":{"format":"cassette"}}),
                b"RIFFtruncated"
            )["ok"],
            false
        );
    }

    #[test]
    fn malformed_requests_and_failed_builds_return_no_launchable_media() {
        for value in [
            json!({"schemaVersion":2,"operation":"build","source":"10 END"}),
            json!({"schemaVersion":1,"operation":"build","source":"10 END","hook":"shell"}),
            json!({"schemaVersion":1,"operation":"build","source":"10 PRINT \"broken"}),
        ] {
            let response = request(value, &[]);
            assert_eq!(response["ok"], false);
            assert!(response.get("result").is_none());
        }
        assert_eq!(process("{", &[])["ok"], false);
        assert_eq!(
            request(
                json!({"schemaVersion":1,"operation":"build","source":"10 END"}),
                &[1]
            )["ok"],
            false
        );
    }

    #[test]
    fn inert_assembly_preview_preserves_unknown_bytes_and_checks_addresses() {
        let bytes = [0x86, 0x42, 0x39];
        let value = json!({"schemaVersion":1,"operation":"decompile","input":{"format":"raw","loadAddress":1024,"executionAddress":1024}});
        let original = bytes;
        let response = request(value, &bytes);
        assert_eq!(bytes, original);
        assert_eq!(response["ok"], true);
        assert_eq!(response["result"]["kind"], "assembly");
        assert_eq!(
            request(
                json!({"schemaVersion":1,"operation":"decompile","input":{"format":"raw","loadAddress":0,"executionAddress":0}}),
                &bytes
            )["ok"],
            false
        );
    }

    #[test]
    fn firmware_receipt_contains_no_bytes_and_generated_rom_is_unverified() {
        let value = request(
            json!({"schemaVersion":1,"operation":"firmware"}),
            &vec![0xa5; 16384],
        );
        assert_eq!(value["ok"], true);
        assert_eq!(value["result"]["matchesVerifiedDragon32"], false);
        assert!(value["result"].get("bytes").is_none());
        assert_eq!(
            request(json!({"schemaVersion":1,"operation":"firmware"}), &[])["ok"],
            false
        );
    }

    #[test]
    fn oversized_inputs_fail_before_parsing_or_reconstruction() {
        assert_eq!(
            process("{}", &vec![0; MAX_BROWSER_FILE_BYTES + 1])["ok"],
            false
        );
        assert_eq!(
            process(&" ".repeat(6 * 1024 * 1024 + 4097), &[])["ok"],
            false
        );
    }
}

#[cfg(test)]
mod multi_machine_tests {
    use super::*;
    #[cfg(feature = "native-acceptance")]
    use std::{collections::BTreeMap, io::Read};
    #[cfg(feature = "native-acceptance")]
    #[test]
    fn every_registered_basic_target_builds_recovers_and_interchanges_with_desktop() {
        for family in [
            &eightbit_commodore_core::FAMILY,
            &eightbit_spectrum_core::FAMILY,
            &eightbit_zx81_core::FAMILY,
        ] {
            for profile in family.profiles {
                let target = profile.target.id;
                let mut project = project::create_for(
                    "Portable game".into(),
                    project::Kind::Basic,
                    profile.basic_template.into(),
                    target.into(),
                )
                .unwrap();
                let engine = machines::engine(target).unwrap();
                let defaults = engine
                    .scaffold("portable-game", family.definition.id, target, "basic")
                    .unwrap();
                for (name, content) in &defaults {
                    if name != family.manifest && name != "src/basic/main.bas" {
                        assert_eq!(project.support_files.get(name), Some(content));
                    }
                }
                assert_eq!(project.manifest.as_ref(), defaults.get(family.manifest));
                let built = machines::build(target, &project.source).unwrap();
                assert_eq!(built["roundTripVerified"], true);
                let media: Vec<u8> = serde_json::from_value(built["cassette"].clone()).unwrap();
                let source = machines::recover(target, &media).unwrap();
                assert_eq!(machines::compile(target, &source).unwrap(), media);
                project
                    .support_files
                    .insert("AGENTS.md".into(), "User-owned guidance\n".into());
                let archive = project::export(&project).unwrap();
                let folder = tempfile::tempdir().unwrap();
                let mut zip = zip::ZipArchive::new(std::io::Cursor::new(&archive)).unwrap();
                let mut files = BTreeMap::new();
                for index in 0..zip.len() {
                    let mut entry = zip.by_index(index).unwrap();
                    let mut text = String::new();
                    entry.read_to_string(&mut text).unwrap();
                    let path = folder.path().join(entry.name());
                    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                    std::fs::write(&path, &text).unwrap();
                    files.insert(entry.name().to_owned(), text);
                }
                let host = eightbit_source_native::Engine {
                    families: engine.families,
                    foreign_manifests: engine.foreign_manifests,
                    product: engine.product,
                };
                let native = host.open(folder.path()).unwrap();
                assert_eq!(native.source, project.source);
                assert_eq!(native.target.id, target);
                files.insert(
                    "src/basic/main.bas".into(),
                    "10 PRINT \"DESKTOP EDIT\"\n".into(),
                );
                files.insert("AGENTS.md".into(), "Desktop custom guidance\n".into());
                let mut output = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
                for (name, text) in files {
                    output
                        .start_file(
                            name,
                            zip::write::SimpleFileOptions::default()
                                .compression_method(zip::CompressionMethod::Deflated),
                        )
                        .unwrap();
                    std::io::Write::write_all(&mut output, text.as_bytes()).unwrap();
                }
                let imported = project::import(&output.finish().unwrap().into_inner()).unwrap();
                assert_eq!(imported.source, "10 PRINT \"DESKTOP EDIT\"\n");
                assert_eq!(
                    imported.support_files["AGENTS.md"],
                    "Desktop custom guidance\n"
                );
                assert_eq!(imported.target, target);
            }
        }
    }
    #[test]
    fn corrupt_media_wrong_targets_memory_overflows_and_unknown_profiles_fail_inertly() {
        assert!(machines::profile("nes").is_err());
        for family in [
            &eightbit_commodore_core::FAMILY,
            &eightbit_spectrum_core::FAMILY,
            &eightbit_zx81_core::FAMILY,
        ] {
            for p in family.profiles {
                let media = machines::compile(p.target.id, p.basic_template).unwrap();
                for end in [0, 1, media.len() / 2, media.len() - 1] {
                    assert!(machines::recover(p.target.id, &media[..end]).is_err());
                }
                assert_eq!(
                    machines::firmware(p.target.id, &vec![0xa5; p.rom_size]).unwrap()["matchesVerifiedFirmware"],
                    false
                );
                assert!(machines::firmware(p.target.id, &[0xa5]).is_err());
                let mut project = project::create_for(
                    "Game".into(),
                    project::Kind::Basic,
                    p.basic_template.into(),
                    p.target.id.into(),
                )
                .unwrap();
                project.target = "dragon32".into();
                assert!(project::validate(&project).is_err());
            }
        }
        let prg = machines::compile("c64-pal", "10 PRINT \"LINK\"\n").unwrap();
        let mut corrupt = prg.clone();
        corrupt[2] = 0;
        corrupt[3] = 0;
        assert!(machines::recover("c64-pal", &corrupt).is_err());
        let mut tail = prg;
        tail.push(0x60);
        assert!(machines::recover("c64-pal", &tail).is_err());
        let mut tap = machines::compile("spectrum48", "10 PRINT 1.5\n").unwrap();
        tap[20] ^= 1;
        assert!(machines::recover("spectrum48", &tap).is_err());
        let mut oversized = String::new();
        for n in 1..120 {
            oversized.push_str(&format!("{n} PRINT \"{}\"\n", "A".repeat(40)));
        }
        assert!(machines::compile("zx81-1k", &oversized).is_err());
    }
    #[test]
    fn recovery_retains_quotes_rem_data_and_numeric_tokens() {
        for (target, source) in [
            (
                "c64-pal",
                "10 DATA PRINT,\"FOR\":PRINT 1\n20 REM PRINT \"STRING\"\n",
            ),
            (
                "spectrum48",
                "10 PRINT \"PRINT\":PRINT 1.5\n20 REM PRINT 12\n",
            ),
            (
                "zx81-16k",
                "10 PRINT \"PRINT\"\n20 PRINT 1.5\n30 REM PRINT 12\n",
            ),
            ("spectrum128", "10 PLAY \"A\"\n"),
        ] {
            let media = machines::compile(target, source).unwrap();
            let recovered = machines::recover(target, &media).unwrap();
            assert_eq!(machines::compile(target, &recovered).unwrap(), media);
        }
    }
}
