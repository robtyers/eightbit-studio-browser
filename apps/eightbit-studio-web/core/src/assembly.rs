//! First-party, no-I/O assembly composition. Fresh source and settings own output.
use crate::project::{self, Snapshot};
use eightbit_dragon_emulator::{
    OriginalDevelopmentLaunch,
    machine_debug::{MachineDebugTarget, MachineSourceEntry},
    original_runtime::OriginalRuntimeLaunch,
};
use eightbit_dragon_media::{create_machine_code, reconstruct::reconstruction_hash};
use serde::Deserialize;
use serde_json::{Value, json};

struct Built {
    cassette: Vec<u8>,
    source_sha256: String,
    build_id: String,
    load_address: u16,
    entry_address: u16,
    byte_count: usize,
    lines: Vec<MachineSourceEntry>,
    source_paths: Vec<String>,
}
fn build(project: &Snapshot) -> Result<Built, Value> {
    let options = project::assembly_options(project).map_err(|error| crate::failure(&error))?;
    let source_paths = project::source_paths(project).map_err(|error| crate::failure(&error))?;
    let sources = source_paths
        .iter()
        .enumerate()
        .map(|(index, path)| {
            let source = if index == 0 {
                project.source.as_str()
            } else {
                project
                    .support_files
                    .get(path)
                    .map(String::as_str)
                    .ok_or_else(|| crate::failure("DSW1024: declared assembly source is missing"))?
            };
            Ok(eightbit_m6809_core::assembler::AssemblySource {
                source_uri: path.as_str(),
                source,
            })
        })
        .collect::<Result<Vec<_>, Value>>()?;
    let code = eightbit_m6809_core::assembler::assemble_project(&sources, options.load_address, &options.entry_symbol)
        .map_err(|error| json!({"schemaVersion":1,"ok":false,"message":"Correct the assembly source before building", "diagnostics":[{"sourceUri":error.source_uri,"row":error.line,"column":error.column,"code":"DSW1024","message":error.message}]}))?;
    let cassette = create_machine_code(
        "MACHINE",
        &code.bytes,
        code.load_address,
        code.entry_address,
    )
    .map_err(|error| crate::failure(&error.to_string()))?;
    let source_identity = sources
        .iter()
        .map(|item| (item.source_uri, item.source))
        .collect::<Vec<_>>();
    let identity = serde_json::to_vec(&(2, &source_identity, &options))
        .map_err(|_| crate::failure("DSW1024: cannot encode assembly identity"))?;
    let built = Built {
        source_sha256: reconstruction_hash(
            &serde_json::to_vec(&source_identity)
                .map_err(|_| crate::failure("DSW1024: cannot encode source identity"))?,
        ),
        build_id: reconstruction_hash(&identity),
        load_address: code.load_address,
        entry_address: code.entry_address,
        byte_count: code.bytes.len(),
        cassette,
        lines: code
            .lines
            .into_iter()
            .filter(|line| line.bytes > 0)
            .map(|line| MachineSourceEntry {
                source_uri: line.source_uri,
                source_line: line.source_line,
                runtime_address: line.address,
                byte_length: line.bytes,
            })
            .collect(),
        source_paths,
    };
    // Build success means this payload is admissible by the same runtime used by
    // Play. Neither a renderer nor a caller-provided source map grants admission.
    make_launch(&built, None).map_err(|error| crate::failure(&error))?;
    Ok(built)
}
fn make_launch(built: &Built, debug: Option<Breakpoint>) -> Result<OriginalRuntimeLaunch, String> {
    let target = debug
        .map(|request| {
            let address = match request {
                Breakpoint::Address { address } => address,
                Breakpoint::Source {
                    source_line,
                    source_uri,
                } => {
                    let source_uri = source_uri
                        .or_else(|| {
                            (built.source_paths.len() == 1).then(|| built.source_paths[0].clone())
                        })
                        .ok_or("DME1500: select a source file for the breakpoint")?;
                    built
                        .lines
                        .iter()
                        .find(|line| {
                            line.source_line == source_line && line.source_uri == source_uri
                        })
                        .map(|line| line.runtime_address)
                        .ok_or("DME1500: choose an assembly source line which emits bytes")?
                }
            };
            MachineDebugTarget::new(address, built.lines.clone())
        })
        .transpose()?;
    OriginalRuntimeLaunch::new_machine(
        OriginalDevelopmentLaunch::with_cassette(
            built.cassette.clone(),
            Some(built.build_id.clone()),
        )?,
        target,
    )
}
pub(crate) fn build_response(project: &Snapshot) -> Value {
    match build(project) {
        Ok(built) => crate::success(json!({
            "cassette":built.cassette,
            "summary":{"tokenBytes":built.byte_count,"sourceSha256":built.source_sha256,"cassetteSha256":reconstruction_hash(&built.cassette)},
            "assembly":{"buildId":built.build_id,"loadAddress":built.load_address,"entryAddress":built.entry_address},
            "diagnostics":[],"roundTripVerified":true
        })),
        Err(error) => error,
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum Breakpoint {
    Address {
        address: u16,
    },
    Source {
        #[serde(rename = "sourceUri")]
        source_uri: Option<String>,
        #[serde(rename = "sourceLine")]
        source_line: usize,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SourceLaunch {
    kind: String,
    schema_version: u32,
    project: Snapshot,
    build_id: String,
    cassette_sha256: String,
    debug: Option<Breakpoint>,
}
pub(crate) fn launch(request: &str) -> Result<OriginalRuntimeLaunch, String> {
    if request.len() > 7 * 1024 * 1024 {
        return Err("DME1401: assembly launch exceeds its size limit".into());
    }
    let request: SourceLaunch =
        serde_json::from_str(request).map_err(|_| "DME1401: invalid assembly launch")?;
    if request.schema_version != 1 || request.kind != "assembly-source" {
        return Err("DME1401: unsupported assembly launch".into());
    }
    let built = build(&request.project)
        .map_err(|_| "DME1401: assembly build failed; check the current source")?;
    if request.build_id != built.build_id
        || request.cassette_sha256 != reconstruction_hash(&built.cassette)
    {
        return Err("DME1401: assembly source, settings or output changed; build again".into());
    }
    make_launch(&built, request.debug)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn project() -> Snapshot {
        project::create_for(
            "Assembly demo".into(),
            project::Kind::Assembly,
            "START\n lda #65\n sta $0400\n rts\n".into(),
            "dragon32".into(),
        )
        .unwrap()
    }
    #[test]
    fn assembly_build_and_fresh_launch_bind_source_settings_and_output() {
        let project = project();
        let response = build_response(&project);
        assert_eq!(response["ok"], true, "{response}");
        let result = &response["result"];
        assert_eq!(result["assembly"]["loadAddress"], 0x7000);
        assert_eq!(result["assembly"]["entryAddress"], 0x7000);
        let bytes: Vec<u8> = serde_json::from_value(result["cassette"].clone()).unwrap();
        assert_eq!(
            eightbit_dragon_media::read_machine_code(&bytes)
                .unwrap()
                .program,
            [0x86, 65, 0xb7, 4, 0, 0x39]
        );
        let request = json!({"schemaVersion":1,"kind":"assembly-source","project":project,"buildId":result["assembly"]["buildId"],"cassetteSha256":result["summary"]["cassetteSha256"]});
        assert!(launch(&request.to_string()).is_ok());
        for (field, value) in [
            ("buildId", json!("a".repeat(64))),
            ("cassetteSha256", json!("b".repeat(64))),
            ("schemaVersion", json!(2)),
            ("sourceMap", json!({})),
        ] {
            let mut bad = request.clone();
            bad[field] = value;
            assert!(launch(&bad.to_string()).is_err());
        }
        let mut changed = request.clone();
        changed["project"]["source"] = json!("START\n rts\n");
        assert!(launch(&changed.to_string()).is_err());
        let mut changed = project;
        project::configure_assembly(
            &mut changed,
            &project::AssemblyOptions {
                load_address: 0x7100,
                entry_symbol: "START".into(),
            },
        )
        .unwrap();
        let mut bad = request;
        bad["project"] = serde_json::to_value(changed).unwrap();
        assert!(launch(&bad.to_string()).is_err());
    }
    #[test]
    fn multi_source_build_identity_diagnostics_and_breakpoints_are_source_aware() {
        let mut project = project();
        project.source = " export START\n extern OTHER\nSTART jsr OTHER\n rts\n".into();
        project.manifest = project.manifest.map(|text| {
            text.replace(
                "sources = [\"src/asm/main.asm\"]",
                "sources = [\"src/asm/main.asm\", \"src/asm/other.asm\"]",
            )
        });
        project.support_files.insert(
            "src/asm/other.asm".into(),
            " export OTHER\n nop\nOTHER nop\n rts\n".into(),
        );
        let response = build_response(&project);
        assert_eq!(response["ok"], true, "{response}");
        let result = &response["result"];
        let request = json!({"schemaVersion":1,"kind":"assembly-source","project":project,"buildId":result["assembly"]["buildId"],"cassetteSha256":result["summary"]["cassetteSha256"],"debug":{"kind":"source","sourceLine":3,"sourceUri":"src/asm/other.asm"}});
        assert!(launch(&request.to_string()).is_ok());
        let built = build(&project).unwrap();
        assert_ne!(
            built
                .lines
                .iter()
                .find(|line| line.source_uri == "src/asm/main.asm" && line.source_line == 3)
                .unwrap()
                .runtime_address,
            built
                .lines
                .iter()
                .find(|line| line.source_uri == "src/asm/other.asm" && line.source_line == 3)
                .unwrap()
                .runtime_address
        );
        for debug in [
            json!({"kind":"source","sourceLine":3}),
            json!({"kind":"source","sourceLine":3,"sourceUri":"not-listed.asm"}),
        ] {
            let mut invalid = request.clone();
            invalid["debug"] = debug;
            assert!(launch(&invalid.to_string()).is_err());
        }
        let mut stale = request.clone();
        stale["project"]["supportFiles"]["src/asm/other.asm"] = json!(" export OTHER\nOTHER rts\n");
        assert!(launch(&stale.to_string()).is_err());
        project.support_files.insert(
            "src/asm/other.asm".into(),
            " export OTHER\nOTHER illegal_opcode\n".into(),
        );
        let rejected = build_response(&project);
        assert_eq!(rejected["ok"], false);
        assert_eq!(rejected["diagnostics"][0]["sourceUri"], "src/asm/other.asm");
        assert_eq!(rejected["diagnostics"][0]["row"], 2);
    }
    #[test]
    fn assembly_reports_source_diagnostics_and_preserves_manifest_zip_settings() {
        let mut project = project();
        let archived = project::export(&project).unwrap();
        let restored = project::import(&archived).unwrap();
        assert_eq!(project::validate(&restored).unwrap(), "src/asm/main.asm");
        assert_eq!(restored.source, project.source);
        assert_eq!(restored.manifest, project.manifest);
        assert_eq!(
            restored
                .support_files
                .get("src/basic/main.bas")
                .map(String::as_str),
            Some("10 END\n")
        );
        project.source = "START\n illegal_opcode #1\n".into();
        let rejected = build_response(&project);
        assert_eq!(rejected["ok"], false);
        assert_eq!(rejected["diagnostics"][0]["row"], 2);
        assert_eq!(rejected["diagnostics"][0]["code"], "DSW1024");
        project.source = "START\n rts\n".into();
        assert_eq!(build_response(&project)["ok"], true);
    }
    #[test]
    fn fresh_machine_debug_maps_only_current_emitting_source_and_steps_real_guest() {
        let project = project();
        let result = build_response(&project)["result"].clone();
        let request = json!({"schemaVersion":1,"kind":"assembly-source","project":project,"buildId":result["assembly"]["buildId"],"cassetteSha256":result["summary"]["cassetteSha256"],"debug":{"kind":"source","sourceLine":2}});
        for invalid in [
            json!({"kind":"source","sourceLine":1}),
            json!({"kind":"address","address":32768}),
            json!({"kind":"address","address":28672,"sourceMap":[]}),
        ] {
            let mut bad = request.clone();
            bad["debug"] = invalid;
            assert!(launch(&bad.to_string()).is_err());
        }
        let mut session = launch(&request.to_string()).unwrap().start(74).unwrap();
        let state =
            |session: &eightbit_dragon_emulator::development_host::DevelopmentSession| -> Value {
                serde_json::from_str(&session.debug_state().unwrap().unwrap()).unwrap()
            };
        for _ in 0..900 {
            session
                .command(eightbit_dragon_emulator::MonitorRequest {
                    schema_version: 1,
                    session_id: 74,
                    command: eightbit_dragon_emulator::MonitorCommand::Advance { cycles: 17_920 },
                })
                .unwrap();
            let stopped = state(&session);
            if stopped["status"]["state"] == "stopped" {
                assert_eq!(stopped["kind"], "machine");
                assert_eq!(stopped["status"]["registers"]["pc"], 0x7000);
                assert_eq!(
                    stopped["status"]["location"]["sourceUri"],
                    "src/asm/main.asm"
                );
                assert_eq!(stopped["status"]["location"]["sourceLine"], 2);
                let step = json!({"schemaVersion":1,"revision":stopped["revision"],"command":{"operation":"step"}}).to_string();
                let stepped: Value =
                    serde_json::from_str(&session.debug_command(&step).unwrap()).unwrap();
                assert_eq!(stepped["status"]["registers"]["pc"], 0x7002);
                assert_eq!(stepped["status"]["registers"]["a"], 65);
                assert_eq!(stepped["status"]["location"]["sourceLine"], 3);
                assert!(
                    session
                        .debug_command(&step)
                        .unwrap_err()
                        .starts_with("DME1501:")
                );
                session
                    .command(eightbit_dragon_emulator::MonitorRequest {
                        schema_version: 1,
                        session_id: 74,
                        command: eightbit_dragon_emulator::MonitorCommand::Stop,
                    })
                    .unwrap();
                assert_eq!(state(&session)["status"]["state"], "terminated");
                return;
            }
        }
        panic!("guest machine source breakpoint was not reached");
    }
    #[test]
    fn freshly_assembled_code_executes_through_the_guest_cassette_loader() {
        let built = build(&project()).unwrap_or_else(|error| panic!("{error}"));
        let mut session = make_launch(&built, None).unwrap().start(73).unwrap();
        for _ in 0..900 {
            let frame = session
                .command(eightbit_dragon_emulator::MonitorRequest {
                    schema_version: 1,
                    session_id: 73,
                    command: eightbit_dragon_emulator::MonitorCommand::Advance { cycles: 17_920 },
                })
                .unwrap();
            if frame.machine.text.starts_with('A') {
                assert_eq!(
                    frame.machine.firmware_id,
                    "eightbit-dragon32-basic-extended-1"
                );
                session
                    .command(eightbit_dragon_emulator::MonitorRequest {
                        schema_version: 1,
                        session_id: 73,
                        command: eightbit_dragon_emulator::MonitorCommand::Stop,
                    })
                    .unwrap();
                return;
            }
        }
        panic!("fresh browser assembly did not execute through CLOADM/EXEC");
    }
}
