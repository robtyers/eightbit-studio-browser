//! Bounded application composition of authoritative portable family libraries.
//! GPL-3.0-or-later; reused Apache-2.0 libraries retain their original notices.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::panic))]

use eightbit_dragon_basic_core::{Dialect, analyse, detokenise, tokenise};
use eightbit_dragon_media::reconstruct::{
    ReconstructionInput, ReconstructionOptions, reconstruct_machine_code,
};
use eightbit_dragon_media::{
    MediaFormatHint, decode_wav_cassette, inspect_basic_media, read_tokenised_basic,
};
use eightbit_dragon_mobile::{AuthoringSession, MobileError, inspect_firmware};
use serde::Deserialize;
use serde_json::{Value, json};
use wasm_bindgen::prelude::wasm_bindgen;

mod vectrex;
pub use vectrex::VectrexOriginalRuntime;
mod assembly;
mod c64;
pub use c64::C64OriginalRuntime;
mod machines;
mod original_media;
mod presentation;
mod project;

/// Explicit original Spectrum development session. Promotion to the normal
/// workbench default remains subject to ADR 0058 acceptance.
#[wasm_bindgen]
pub struct SpectrumDevelopmentRuntime {
    session: eightbit_spectrum_core::original_session::Session,
}
#[wasm_bindgen]
impl SpectrumDevelopmentRuntime {
    /// Boot the original development firmware in an owned worker session.
    #[wasm_bindgen(constructor)]
    pub fn new(session_id: u32) -> Result<SpectrumDevelopmentRuntime, String> {
        Ok(Self {
            session: eightbit_spectrum_core::original_session::Session::new(session_id)
                .map_err(|e| e.to_string())?,
        })
    }
    /// Boot a validated original Spectrum model without source or external tools.
    #[wasm_bindgen(js_name = forTarget)]
    pub fn for_target(session_id: u32, target: &str) -> Result<SpectrumDevelopmentRuntime, String> {
        Ok(Self {
            session: eightbit_spectrum_core::original_session::Session::for_target(
                session_id, target,
            )
            .map_err(|e| e.to_string())?,
        })
    }
    #[wasm_bindgen(js_name = fromSource)]
    /// Rebuild current source and check its cassette identity before execution.
    pub fn from_source(
        session_id: u32,
        request: &str,
    ) -> Result<SpectrumDevelopmentRuntime, String> {
        Ok(Self {
            session: eightbit_spectrum_core::original_session::Session::from_source_json(
                session_id, request,
            )
            .map_err(|e| e.to_string())?,
        })
    }
    /// Apply a bounded command to this exact session identity.
    pub fn command(&mut self, request: &str) -> Result<String, String> {
        self.session
            .command_json(request)
            .map_err(|e| e.to_string())
    }
}

/// Disposable monitor session owned only by its application worker.
#[wasm_bindgen]
pub struct MonitorRuntime {
    session: eightbit_dragon_emulator::development_host::DevelopmentSession,
}
#[wasm_bindgen]
impl MonitorRuntime {
    /// Boot only the original bundled monitor, never user-supplied firmware.
    #[wasm_bindgen(constructor)]
    pub fn new(session_id: u32) -> Result<MonitorRuntime, String> {
        Ok(Self {
            session: eightbit_dragon_emulator::development_host::DevelopmentSession::from_monitor(
                eightbit_dragon_emulator::MonitorSession::new(session_id)?,
            ),
        })
    }
    /// Explicit development firmware; this does not select a project backend.
    #[wasm_bindgen(js_name = basicDevelopment)]
    pub fn basic_development(session_id: u32) -> Result<MonitorRuntime, String> {
        Ok(Self {
            session: eightbit_dragon_emulator::development_host::DevelopmentSession::from_monitor(
                eightbit_dragon_emulator::MonitorSession::with_firmware(
                    session_id,
                    eightbit_dragon_emulator::Firmware::BasicDevelopment,
                )?,
            ),
        })
    }
    /// Boot an explicit typed development launch with digest-bound cassette media.
    #[wasm_bindgen(js_name = fromDevelopmentLaunch)]
    pub fn from_development_launch(
        session_id: u32,
        request: &str,
    ) -> Result<MonitorRuntime, String> {
        Ok(Self {
            session: eightbit_dragon_emulator::development_host::DevelopmentSession::from_monitor(
                eightbit_dragon_emulator::OriginalDevelopmentLaunch::from_json(request)?
                    .start(session_id)?,
            ),
        })
    }
    /// Explicit development-only fresh source build. No external source map is accepted.
    #[wasm_bindgen(js_name = fromDevelopmentDebugSource)]
    pub fn from_development_debug_source(
        session_id: u32,
        request: &str,
    ) -> Result<MonitorRuntime, String> {
        Ok(Self {
            session: development_debug_source(request)?.start(session_id)?,
        })
    }
    /// Normal original Run/Debug is admitted only by the compiled firmware.
    /// Admission occurs before parsing/building user source or creating a session.
    #[wasm_bindgen(js_name = fromOriginalSource)]
    pub fn from_original_source(session_id: u32, request: &str) -> Result<MonitorRuntime, String> {
        use eightbit_dragon_emulator::original_runtime::{
            OriginalRuntimeLaunch, admit_bundled_original_runtime,
        };
        if request.len() > 7 * 1024 * 1024 {
            return Err("DME1401: original source request is too large".into());
        }
        let request: OriginalRuntimeSource = serde_json::from_str(request)
            .map_err(|_| "DME1401: invalid original source request")?;
        if request.schema_version != 1 {
            return Err("DME1401: unsupported original source request".into());
        }
        if !project::is_dragon(&request.target) {
            return Err("DME1401: unsupported original target".into());
        }
        if request.target == "dragon32" {
            admit_bundled_original_runtime()?;
        } else {
            eightbit_dragon_emulator::original_runtime::admit_bundled_dragon64_runtime(
                request.target == "dragon64-native",
            )?;
        }
        let (launch, tokens, names) = fresh_source_launch_for_target(
            &request.target,
            &request.source,
            &request.source_sha256,
            &request.cassette_sha256,
        )?;
        Ok(Self {
            session: if request.target == "dragon32" {
                OriginalRuntimeLaunch::new(launch, tokens, request.breakpoint_line, names)
            } else {
                OriginalRuntimeLaunch::new_dragon64(
                    request.target == "dragon64-native",
                    launch,
                    tokens,
                    request.breakpoint_line,
                    names,
                )
            }?
            .start(session_id)?,
        })
    }
    /// Rebuild current assembly and bind its settings and output before execution.
    #[wasm_bindgen(js_name = fromOriginalAssembly)]
    pub fn from_original_assembly(
        session_id: u32,
        request: &str,
    ) -> Result<MonitorRuntime, String> {
        eightbit_dragon_emulator::original_runtime::admit_bundled_original_runtime()?;
        Ok(Self {
            session: assembly::launch(request)?.start(session_id)?,
        })
    }
    /// Revalidate the exact selected media, then let the ROM load and execute it.
    #[wasm_bindgen(js_name = fromOriginalMedia)]
    pub fn from_original_media(
        session_id: u32,
        request: &str,
        bytes: &[u8],
    ) -> Result<MonitorRuntime, String> {
        eightbit_dragon_emulator::original_runtime::admit_bundled_original_runtime()?;
        Ok(Self {
            session: original_media::launch(request, bytes)?.start(session_id)?,
        })
    }
    /// Read this worker's optional development inspection without executing.
    #[wasm_bindgen(js_name = debugState)]
    pub fn debug_state(&self) -> Result<Option<String>, String> {
        self.session.debug_state()
    }
    /// Apply owned Continue/cancel; frame ticks remain the sole execution scheduler.
    #[wasm_bindgen(js_name = debugCommand)]
    pub fn debug_command(&mut self, request: &str) -> Result<String, String> {
        self.session.debug_command(request)
    }
    /// Apply a bounded, strict request to this exact session.
    pub fn command(&mut self, request: &str) -> Result<String, String> {
        let snapshot = self.session.command_json(request)?;
        serde_json::to_string(&snapshot).map_err(|_| "DME1002: cannot encode monitor state".into())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DevelopmentDebugSource {
    schema_version: u32,
    source: String,
    source_sha256: String,
    cassette_sha256: String,
    breakpoint_line: u16,
}
fn development_debug_source(
    json: &str,
) -> Result<eightbit_dragon_emulator::development_host::DevelopmentDebugLaunch, String> {
    use eightbit_dragon_emulator::development_host::DevelopmentDebugLaunch;
    if json.len() > 7 * 1024 * 1024 {
        return Err("DME1302: development source request is too large".into());
    }
    let request: DevelopmentDebugSource =
        serde_json::from_str(json).map_err(|_| "DME1302: invalid development source request")?;
    if request.schema_version != 1
        || request.source.len() > eightbit_dragon_basic_core::MAX_SOURCE_BYTES
    {
        return Err("DME1302: unsupported development source request".into());
    }
    let (launch, tokenised, names) = fresh_source_launch(
        &request.source,
        &request.source_sha256,
        &request.cassette_sha256,
    )?;
    DevelopmentDebugLaunch::new(launch, tokenised, request.breakpoint_line, names)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OriginalRuntimeSource {
    schema_version: u32,
    #[serde(default = "default_target")]
    target: String,
    source: String,
    source_sha256: String,
    cassette_sha256: String,
    breakpoint_line: Option<u16>,
}
type FreshSourceBuild = (
    eightbit_dragon_emulator::OriginalDevelopmentLaunch,
    eightbit_dragon_basic_core::TokenisedProgram,
    std::collections::BTreeMap<String, String>,
);
fn fresh_source_launch(
    source: &str,
    source_sha256: &str,
    cassette_sha256: &str,
) -> Result<FreshSourceBuild, String> {
    fresh_source_launch_for_target("dragon32", source, source_sha256, cassette_sha256)
}
fn dragon_dialect(target: &str) -> Result<&'static str, String> {
    match target {
        "dragon32" | "dragon64" => Ok("dragon32-basic"),
        "dragon64-native" => Ok("dragon64-basic"),
        _ => Err("DME1401: unsupported Dragon target".into()),
    }
}
fn fresh_source_launch_for_target(
    target: &str,
    source: &str,
    source_sha256: &str,
    cassette_sha256: &str,
) -> Result<FreshSourceBuild, String> {
    use eightbit_dragon_basic_core::inspection::source_variable_names;
    use eightbit_dragon_emulator::OriginalDevelopmentLaunch;
    if source.len() > eightbit_dragon_basic_core::MAX_SOURCE_BYTES {
        return Err("DME1302: source is too large".into());
    }
    let mut author = AuthoringSession::for_dialect(dragon_dialect(target)?)
        .map_err(|_| "DME1302: source build is unavailable")?;
    author
        .edit(1, source.to_owned())
        .map_err(|_| "DME1302: invalid development source")?;
    let summary = author
        .build(1)
        .map_err(|_| "DME1302: development source build failed")?;
    if summary.source_sha256 != source_sha256 || summary.cassette_sha256 != cassette_sha256 {
        return Err("DME1303: development source or cassette changed; build again".into());
    }
    let built = author
        .current_build()
        .ok_or("DME1302: current development build is unavailable")?;
    let dialect = Dialect::for_id(dragon_dialect(target)?)
        .map_err(|_| "DME1302: development dialect is unavailable")?;
    let tokenised = tokenise(&analyse(source, &dialect).program, &dialect)
        .map_err(|_| "DME1302: source map construction failed")?;
    let launch = OriginalDevelopmentLaunch::with_cassette(
        built.cassette.clone(),
        Some(summary.source_sha256),
    )?;
    Ok((launch, tokenised, source_variable_names(source)))
}

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
        assembly: Option<project::AssemblyOptions>,
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

/// Render the explicitly selected current Dragon cassette as bounded binary PCM.
/// Keep audio out of JSON to avoid expanding every sample into an integer array.
#[wasm_bindgen]
pub fn cassette_audio(metadata: &str, bytes: &[u8]) -> Result<Vec<u8>, String> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct AudioRequest {
        schema_version: u32,
        operation: String,
        target: String,
        cassette_sha256: String,
    }
    if metadata.len() > 512 || bytes.len() > MAX_BROWSER_FILE_BYTES {
        return Err("DSW1025: cassette audio input exceeds its size limit".into());
    }
    let request: AudioRequest =
        serde_json::from_str(metadata).map_err(|_| "DSW1025: invalid cassette audio request")?;
    if request.schema_version != 1
        || request.operation != "cassette-audio"
        || !project::is_dragon(&request.target)
        || request.cassette_sha256 != eightbit_dragon_media::reconstruct::reconstruction_hash(bytes)
    {
        return Err("DSW1025: cassette identity changed; check and build again".into());
    }
    eightbit_dragon_media::encode_cassette_wav(bytes).map_err(|error| format!("DSW1025: {error}"))
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
                let mut catalog = machines::catalog();
                catalog["originalRuntime"] = serde_json::json!(
                    eightbit_dragon_emulator::original_runtime::bundled_original_runtime_capability(
                    )
                );
                catalog["originalRuntimes"] = serde_json::json!({"dragon64":eightbit_dragon_emulator::original_runtime::bundled_original_runtime_capability_for_target("dragon64"),"dragon64-native":eightbit_dragon_emulator::original_runtime::bundled_original_runtime_capability_for_target("dragon64-native")});
                success(catalog)
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
            if !project::is_dragon(&project.target) {
                return match if project.kind == project::Kind::Assembly {
                    machines::build_original_assembly(&project)
                } else {
                    machines::build(&project.target, &project.source)
                } {
                    Ok(result) => success(result),
                    Err(error) => failure(&error),
                };
            }
            if project.kind == project::Kind::Assembly {
                return assembly::build_response(&project);
            }
            if let Some(manifest) = project.manifest.as_deref() {
                let config = match eightbit_dragon_project::parse_project_config(manifest) {
                    Ok(config) => config,
                    Err(error) => return failure(&error.to_string()),
                };
                if let Some(entry_line) = config.basic.entry_line {
                    let dialect = match Dialect::for_id(&config.basic.dialect) {
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
            build_target(project.source, true, &project.target)
        }
        Request::CreateProject {
            schema_version,
            name,
            kind,
            source,
            target,
            assembly,
        } => {
            if schema_version != 1 || !bytes.is_empty() {
                return failure("DSW1002: invalid browser request");
            }
            match project::create_for(name, kind, source, target) {
                Ok(mut project) => {
                    if let Some(options) = assembly
                        && let Err(error) = project::configure_assembly(&mut project, &options)
                    {
                        return failure(&error);
                    }
                    success(
                        json!({"entryPath":project::validate(&project).ok(), "sourcePaths":project::source_paths(&project).ok(), "project":project}),
                    )
                }
                Err(error) => failure(&error),
            }
        }
        Request::ImportProject { schema_version } => {
            if schema_version != 1 {
                return failure("DSW1002: invalid browser request");
            }
            match project::import(bytes) {
                Ok(project) => success(
                    json!({"entryPath":project::validate(&project).ok(), "sourcePaths":project::source_paths(&project).ok(), "project":project}),
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
            if project::is_dragon(&target) {
                success(presentation::source_highlights(&source))
            } else if target == "vectrex" || machines::profile(&target).is_ok() {
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
    build_target(source, verify, "dragon32")
}
fn build_target(source: String, verify: bool, target: &str) -> Value {
    let dialect_id = match dragon_dialect(target) {
        Ok(id) => id,
        Err(error) => return failure(&error),
    };
    let result = (|| {
        let mut session = AuthoringSession::for_dialect(dialect_id)?;
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
            let dialect = Dialect::for_id(dialect_id).ok()?;
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
    if input == ReconstructionInput::Cartridge {
        return failure(
            "DSW1023: cartridges are unsupported on the website; use the desktop edition with XRoar",
        );
    }
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
    if matches!(input, ReconstructionInput::Cassette)
        && let Ok(inspection) = inspect_basic_media(original, MediaFormatHint::CassetteAsciiBasic)
    {
        return success(json!({"kind":"basic", "source":inspection.source,
            "warnings":inspection.warnings, "cassette": original}));
    }
    let identity = match original_media::prepare(bytes, input.clone()) {
        Ok((_, identity)) => identity,
        Err(error) => return failure(&error),
    };
    let options = ReconstructionOptions {
        input,
        ..ReconstructionOptions::default()
    };
    match reconstruct_machine_code(bytes, &options) {
        Ok(preview) => success(
            json!({"kind":"assembly", "source":preview.source, "warnings":preview.warnings, "preview":preview, "machineCode":identity}),
        ),
        Err(error) => failure(&error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cassette_audio_preserves_checked_basic_and_machine_payloads() {
        let basic = process(
            r#"{"schemaVersion":1,"operation":"test","source":"10 PRINT \"STOCK DRAGON\"\n20 END\n"}"#,
            &[],
        );
        let basic: Vec<u8> = serde_json::from_value(basic["result"]["cassette"].clone()).unwrap();
        let machine = eightbit_dragon_media::create_machine_code(
            "STOCK",
            &[0x86, 77, 0xb7, 0x63, 0, 0x39],
            0x7000,
            0x7000,
        )
        .unwrap();
        for cassette in [basic, machine] {
            let metadata = json!({"schemaVersion":1,"operation":"cassette-audio","target":"dragon32","cassetteSha256":eightbit_dragon_media::reconstruct::reconstruction_hash(&cassette)});
            let audio = cassette_audio(&metadata.to_string(), &cassette).unwrap();
            assert_eq!(
                audio,
                cassette_audio(&metadata.to_string(), &cassette).unwrap()
            );
            assert_eq!(
                audio,
                eightbit_dragon_media::encode_cassette_wav(&cassette).unwrap()
            );
            assert_eq!(decode_wav_cassette(&audio).unwrap().cassette, cassette);
            for (key, value) in [
                ("schemaVersion", json!(2)),
                ("operation", json!("build")),
                ("target", json!("c64-pal")),
                ("cassetteSha256", json!("0".repeat(64))),
                ("extra", json!(true)),
            ] {
                let mut invalid = metadata.clone();
                invalid[key] = value;
                assert!(
                    cassette_audio(&invalid.to_string(), &cassette)
                        .unwrap_err()
                        .starts_with("DSW1025:")
                );
            }
            assert!(cassette_audio(&metadata.to_string(), &[]).is_err());
        }
        assert!(cassette_audio(&" ".repeat(513), &[]).is_err());
        assert!(cassette_audio("{}", &vec![0; MAX_BROWSER_FILE_BYTES + 1]).is_err());
        let corrupt = [0, 1, 255];
        let metadata = json!({"schemaVersion":1,"operation":"cassette-audio","target":"dragon32","cassetteSha256":eightbit_dragon_media::reconstruct::reconstruction_hash(&corrupt)});
        assert!(cassette_audio(&metadata.to_string(), &corrupt).is_err());
    }

    #[test]
    fn browser_catalog_reports_original_default_and_explicit_legacy_adapter() {
        let value = process(r#"{"schemaVersion":1,"operation":"machines"}"#, &[]);
        assert_eq!(
            value["result"]["originalRuntime"],
            serde_json::to_value(
                eightbit_dragon_emulator::original_runtime::bundled_original_runtime_capability()
            )
            .unwrap()
        );
        assert_eq!(value["result"]["machines"][0]["runtime"], "xroar");
        assert_eq!(value["result"]["machines"][0]["defaultRuntime"], "original");
        let machines = value["result"]["machines"].as_array().unwrap();
        let spectrum = machines
            .iter()
            .find(|machine| machine["id"] == "spectrum48")
            .unwrap();
        assert_eq!(spectrum["runtime"], "mame-z80");
        assert_eq!(spectrum["defaultRuntime"], "original");
        for machine in machines.iter().filter(|machine| {
            machine["id"] != "dragon32"
                && machine["id"] != "dragon64"
                && machine["id"] != "dragon64-native"
                && machine["id"]
                    .as_str()
                    .is_none_or(|id| eightbit_spectrum_core::original_target(id).is_none())
        }) {
            assert!(machine.get("defaultRuntime").is_none());
        }
    }

    #[test]
    fn decompiles_ascii_cassette_and_gapped_wav_without_running_or_writing() {
        let source = "10 PRINT\"ASCII READY\"\n20 END\n";
        let cas = eightbit_dragon_media::create_ascii_basic("ASCII", source).unwrap();
        for bytes in [
            cas.clone(),
            eightbit_dragon_media::encode_cassette_wav(&cas).unwrap(),
        ] {
            let result = request(
                json!({"schemaVersion":1,"operation":"decompile","input":{"format":"cassette"}}),
                &bytes,
            );
            assert_eq!(result["ok"], true);
            assert_eq!(result["result"]["kind"], "basic");
            assert_eq!(result["result"]["source"], source);
            let recovered: Vec<u8> =
                serde_json::from_value(result["result"]["cassette"].clone()).unwrap();
            assert_eq!(recovered, cas);
        }
    }

    #[test]
    fn typed_development_launch_preserves_media_identity_and_default_constructor()
    -> Result<(), String> {
        let bytes = eightbit_dragon_media::create_machine_code("TYPED", &[0x39], 0x6000, 0x6000)
            .map_err(|error| error.to_string())?;
        let launch =
            eightbit_dragon_emulator::OriginalDevelopmentLaunch::with_cassette(bytes, None)?;
        let json = serde_json::to_string(&launch).map_err(|error| error.to_string())?;
        let mut runtime = MonitorRuntime::from_development_launch(2, &json)?;
        let state: Value = serde_json::from_str(
            &runtime
                .command(r#"{"schemaVersion":1,"sessionId":2,"command":{"operation":"state"}}"#)?,
        )
        .map_err(|error| error.to_string())?;
        assert_eq!(state["cassette"]["filename"], "TYPED");
        assert_eq!(
            state["firmwareId"],
            serde_json::from_str::<Value>(eightbit_dragon_emulator::BASIC_FIRMWARE_MANIFEST)
                .map_err(|error| error.to_string())?["id"]
        );
        assert!(
            MonitorRuntime::from_development_launch(2, &json.replace("dragon32", "dragon64"))
                .is_err()
        );
        let mut monitor = MonitorRuntime::new(3)?;
        let state: Value = serde_json::from_str(
            &monitor
                .command(r#"{"schemaVersion":1,"sessionId":3,"command":{"operation":"state"}}"#)?,
        )
        .map_err(|error| error.to_string())?;
        assert_eq!(state["firmwareId"], "eightbit-dragon32-monitor-0.1");
        Ok(())
    }

    #[test]
    fn development_basic_runtime_uses_guest_code_and_preserves_session_guards() -> Result<(), String>
    {
        let mut runtime = MonitorRuntime::basic_development(1)?;
        assert!(
            runtime
                .command(r#"{"schemaVersion":1,"sessionId":2,"command":{"operation":"state"}}"#)
                .is_err()
        );
        runtime.command(r#"{"schemaVersion":1,"sessionId":1,"command":{"operation":"type","text":"PRINT 9*9"}}"#)?;
        let mut frame = Value::Null;
        for _ in 0..32 {
            frame = serde_json::from_str(&runtime.command(r#"{"schemaVersion":1,"sessionId":1,"command":{"operation":"advance","cycles":32768}}"#)?)
                .map_err(|_| "Invalid development BASIC frame")?;
        }
        assert_eq!(
            frame["firmwareId"],
            serde_json::from_str::<Value>(eightbit_dragon_emulator::BASIC_FIRMWARE_MANIFEST)
                .map_err(|error| error.to_string())?["id"]
        );
        assert_eq!(frame["state"], "running");
        assert!(
            frame["text"]
                .as_str()
                .is_some_and(|text| text.contains(" 81 ") && !text.contains("?INVALID"))
        );
        runtime.command(r#"{"schemaVersion":1,"sessionId":1,"command":{"operation":"stop"}}"#)?;
        assert!(
            runtime
                .command(r#"{"schemaVersion":1,"sessionId":1,"command":{"operation":"reset"}}"#)
                .is_err()
        );
        Ok(())
    }

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
    #[test]
    fn spectrum_assembly_projects_build_and_round_trip_without_an_external_tool() {
        let mut project = project::create_for(
            "Spectrum game".into(),
            project::Kind::Assembly,
            "ORG 24576\nLD A,42\nRET\n".into(),
            "spectrum48".into(),
        )
        .unwrap();
        project
            .support_files
            .insert("AGENTS.md".into(), "User guidance".into());
        let entry = project::validate(&project).unwrap();
        assert_eq!(entry, "src/asm/main.asm");
        assert_eq!(project::source_paths(&project).unwrap(), [entry]);
        let imported = project::import(&project::export(&project).unwrap()).unwrap();
        assert_eq!(imported.kind, project::Kind::Assembly);
        assert_eq!(imported.source, project.source);
        assert_eq!(imported.manifest, project.manifest);
        assert_eq!(imported.support_files["AGENTS.md"], "User guidance");
        let response = process(
            &json!({"schemaVersion":1,"operation":"test-project","project":project}).to_string(),
            &[],
        );
        assert_eq!(response["ok"], true, "{response}");
        let built = &response["result"];
        assert_eq!(built["toolVersion"], "Eightbit Z80 assembler 1");
        assert_eq!(
            built["sourceMap"]["instructions"].as_array().unwrap().len(),
            2
        );
        let launch = json!({"schemaVersion":1,"target":"spectrum48","kind":"assembly","source":project.source,"sourcePath":"src/asm/main.asm","revision":built["assembly"]["buildId"],"sourceSha256":built["summary"]["sourceSha256"],"cassetteSha256":built["summary"]["cassetteSha256"],"breakpointLine":2});
        let mut runtime = SpectrumDevelopmentRuntime::from_source(7, &launch.to_string()).unwrap();
        let frame: Value = serde_json::from_str(
            &runtime
                .command(r#"{"schemaVersion":1,"sessionId":7,"command":{"operation":"state"}}"#)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(frame["width"], 320);
        assert_eq!(frame["height"], 256);
        assert_eq!(frame["audioSampleRate"], 44100);
        assert_eq!(frame["firmwareId"], "eightbit-spectrum48-basic-0.1-dev");
        let mut stale = launch;
        stale["source"] = json!("RET");
        assert!(SpectrumDevelopmentRuntime::from_source(8, &stale.to_string()).is_err());
        project
            .support_files
            .insert("src/undeclared.asm".into(), "RET".into());
        assert!(project::export(&project).is_err());
    }
    #[cfg(feature = "native-acceptance")]
    use std::{collections::BTreeMap, io::Read};
    #[cfg(feature = "native-acceptance")]
    #[test]
    fn every_registered_basic_target_builds_recovers_and_interchanges_with_desktop() {
        for family in [
            &eightbit_commodore_core::FAMILY,
            &eightbit_acorn_core::FAMILY,
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
            &eightbit_acorn_core::FAMILY,
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

#[cfg(test)]
mod development_host_tests {
    use super::*;
    const SOURCE: &str = "10 COUNT=41\n20 COUNT=COUNT+1:PRINT COUNT\n30 END\n";
    fn source_request(source: &str) -> Value {
        let mut author = AuthoringSession::new().unwrap();
        author.edit(1, source.into()).unwrap();
        let summary = author.build(1).unwrap();
        json!({"schemaVersion":1,"source":source,"sourceSha256":summary.source_sha256,"cassetteSha256":summary.cassette_sha256,"breakpointLine":20})
    }
    fn monitor(runtime: &mut MonitorRuntime, command: &Value) -> Value {
        serde_json::from_str(
            &runtime
                .command(&json!({"schemaVersion":1,"sessionId":19,"command":command}).to_string())
                .unwrap(),
        )
        .unwrap()
    }
    fn state(runtime: &MonitorRuntime) -> Value {
        serde_json::from_str(&runtime.debug_state().unwrap().unwrap()).unwrap()
    }
    fn debug(runtime: &mut MonitorRuntime, operation: &str) -> Value {
        serde_json::from_str(&runtime.debug_command(&json!({"schemaVersion":1,"revision":state(runtime)["revision"],"command":{"operation":operation}}).to_string()).unwrap()).unwrap()
    }
    #[test]
    fn browser_original_runtime_admission_matches_compiled_identity_before_source_build() {
        let admitted =
            eightbit_dragon_emulator::original_runtime::admit_bundled_original_runtime().is_ok();
        let valid = MonitorRuntime::from_original_source(19, &source_request(SOURCE).to_string());
        if admitted {
            assert!(valid.is_ok());
        } else {
            assert!(valid.err().unwrap().starts_with("DME1400:"));
        }
        for request in ["{}", "invalid"] {
            let error = MonitorRuntime::from_original_source(19, request)
                .err()
                .unwrap();
            assert!(error.starts_with(if admitted { "DME1401:" } else { "DME1400:" }));
        }
        // Rejection cannot mutate or replace an independent existing owner.
        let runtime = MonitorRuntime::basic_development(19).unwrap();
        assert!(runtime.debug_state().unwrap().is_none());
    }

    #[test]
    fn browser_development_host_executes_real_cload_and_revision_owned_breakpoint() {
        let mut runtime =
            MonitorRuntime::from_development_debug_source(19, &source_request(SOURCE).to_string())
                .unwrap();
        monitor(&mut runtime, &json!({"operation":"type","text":"CLOAD"}));
        for _ in 0..384 {
            monitor(&mut runtime, &json!({"operation":"advance","cycles":32768}));
        }
        monitor(&mut runtime, &json!({"operation":"type","text":"RUN"}));
        debug(&mut runtime, "continue");
        for _ in 0..512 {
            monitor(&mut runtime, &json!({"operation":"advance","cycles":32768}));
            if state(&runtime)["status"]["state"] != "running" {
                break;
            }
        }
        let stopped = state(&runtime);
        assert_eq!(stopped["status"]["state"], "stopped");
        assert_eq!(stopped["status"]["variables"][0]["name"], "COUNT");
        assert_eq!(stopped["status"]["variables"][0]["numericValue"], 41.0);
        for _ in 0..4 {
            monitor(&mut runtime, &json!({"operation":"advance","cycles":32768}));
        }
        assert_eq!(state(&runtime), stopped);
        debug(&mut runtime, "continue");
        assert_eq!(
            debug(&mut runtime, "cancel")["status"]["state"],
            "cancelled"
        );
        debug(&mut runtime, "continue");
        for _ in 0..512 {
            monitor(&mut runtime, &json!({"operation":"advance","cycles":32768}));
            if state(&runtime)["status"]["state"] != "running" {
                break;
            }
        }
        assert_eq!(
            state(&runtime)["status"]["variables"][0]["numericValue"],
            42.0
        );
        monitor(&mut runtime, &json!({"operation":"reset"}));
        assert_eq!(state(&runtime)["status"]["state"], "ready");
        assert!(runtime.debug_command(&json!({"schemaVersion":1,"revision":stopped["revision"],"command":{"operation":"continue"}}).to_string()).is_err());
        monitor(&mut runtime, &json!({"operation":"stop"}));
        assert_eq!(state(&runtime)["status"]["state"], "terminated");
    }
    #[test]
    fn browser_debug_source_never_accepts_stale_maps_failed_builds_or_external_profiles() {
        let valid = source_request(SOURCE);
        for (key, value) in [
            ("source", json!("10 PRINT 9\n")),
            ("cassetteSha256", json!("a".repeat(64))),
            ("sourceSha256", json!("b".repeat(64))),
            ("breakpointLine", json!(99)),
            ("schemaVersion", json!(2)),
            ("sourceMap", json!({})),
            ("firmware", json!("custom.rom")),
        ] {
            let mut invalid = valid.clone();
            invalid[key] = value;
            assert!(
                MonitorRuntime::from_development_debug_source(19, &invalid.to_string()).is_err()
            );
        }
        let mut invalid = valid.clone();
        invalid["source"] = json!("10 PRINT 1\n10 PRINT 2\n");
        assert!(MonitorRuntime::from_development_debug_source(19, &invalid.to_string()).is_err());
        invalid["source"] = json!(" ".repeat(eightbit_dragon_basic_core::MAX_SOURCE_BYTES + 1));
        assert!(MonitorRuntime::from_development_debug_source(19, &invalid.to_string()).is_err());
        assert!(
            MonitorRuntime::new(19)
                .unwrap()
                .debug_state()
                .unwrap()
                .is_none()
        );
        assert!(
            MonitorRuntime::basic_development(19)
                .unwrap()
                .debug_state()
                .unwrap()
                .is_none()
        );
    }
}

#[cfg(test)]
mod dragon64_tests {
    use super::*;
    #[test]
    fn browser_dragon64_projects_round_trip_their_desktop_mode_and_build() {
        for target in ["dragon64", "dragon64-native"] {
            let project = project::create_for(
                "D64 Game".into(),
                project::Kind::Basic,
                "10 PRINT 42\n20 END\n".into(),
                target.into(),
            )
            .unwrap();
            let archive = project::export(&project).unwrap();
            let recovered = project::import(&archive).unwrap();
            assert_eq!(recovered.target, target);
            let config = eightbit_dragon_project::parse_project_config(
                recovered.manifest.as_deref().unwrap(),
            )
            .unwrap();
            assert_eq!(
                config.schema_version,
                if target == "dragon64" { 4 } else { 5 }
            );
            assert_eq!(config.native_basic(), target == "dragon64-native");
            let result = process(
                &json!({"schemaVersion":1,"operation":"test-project","project":recovered})
                    .to_string(),
                &[],
            );
            assert_eq!(result["ok"], true, "{result}");
        }
    }
    #[test]
    fn browser_dragon64_source_runs_from_its_exact_mode_and_rejects_changed_builds() {
        let source = "10 A=41\n20 A=A+1:PRINT A\n30 END\n";
        for target in ["dragon64", "dragon64-native"] {
            let result = build_target(source.into(), true, target);
            assert_eq!(result["ok"], true, "{result}");
            let mut request = json!({"schemaVersion":1,"target":target,"source":source,
                "sourceSha256":result["result"]["summary"]["sourceSha256"],
                "cassetteSha256":result["result"]["summary"]["cassetteSha256"]});
            let mut runtime =
                MonitorRuntime::from_original_source(64, &request.to_string()).unwrap();
            let mut completed = false;
            for _ in 0..1024 {
                let encoded=runtime.command(&json!({"schemaVersion":1,"sessionId":64,"command":{"operation":"advance","cycles":32768}}).to_string()).unwrap();
                let frame: Value = serde_json::from_str(&encoded).unwrap();
                assert_ne!(frame["state"], "failed", "{frame}");
                if frame["text"]
                    .as_str()
                    .unwrap()
                    .lines()
                    .any(|line| line.trim() == "42")
                {
                    completed = true;
                    break;
                }
            }
            assert!(completed);
            request["source"] = json!("10 PRINT 99\n");
            assert!(MonitorRuntime::from_original_source(65, &request.to_string()).is_err());
        }
    }
}
