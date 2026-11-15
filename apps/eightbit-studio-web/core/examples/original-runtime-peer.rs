//! Opt-in acceptance peer for the real native host API; no admission override.
use eightbit_browser::{
    C64OriginalRuntime, MonitorRuntime, SpectrumDevelopmentRuntime, browser_request,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{BufRead, Read, Write};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    id: u32,
    command: Command,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    source_uri: String,
    source: String,
}
#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
enum Command {
    Identity,
    Catalog,
    Build {
        source: String,
    },
    Browser {
        request: Value,
        bytes: Vec<u8>,
    },
    LinkInspection {
        sources: Vec<Source>,
        load_address: u16,
        entry_symbol: String,
    },
    Create {
        session_id: u32,
        source_request: Value,
    },
    CreateAssembly {
        session_id: u32,
        source_request: Value,
    },
    CreateMedia {
        session_id: u32,
        media_request: Value,
        bytes: Vec<u8>,
    },
    Monitor {
        request: Value,
    },
    DebugState,
    Debug {
        request: Value,
    },
    Close,
    SpectrumCreate {
        session_id: u32,
        source_request: Value,
    },
    SpectrumMonitor {
        request: Value,
    },
    SpectrumClose,
    C64Create {
        session_id: u32,
        source_request: Value,
    },
    C64Monitor {
        request: Value,
    },
    C64Close,
}
fn decode(json: &str) -> Result<Value, String> {
    serde_json::from_str(json).map_err(|_| "Invalid host response".into())
}
fn compact(mut value: Value) -> Result<Value, String> {
    if let Some(session) = value.get_mut("session") {
        *session = compact(std::mem::take(session))?;
    }
    if let Some(frame) = value.as_object_mut() {
        if let Some(pixels) = frame.remove("pixels") {
            let bytes: Vec<u8> = serde_json::from_value(pixels).map_err(|_| "Invalid pixels")?;
            frame.insert(
                "pixelsSha256".into(),
                json!(hex::encode(Sha256::digest(bytes))),
            );
        }
        if let Some(audio) = frame.remove("audio") {
            let samples: Vec<i16> = serde_json::from_value(audio).map_err(|_| "Invalid audio")?;
            let mut hash = Sha256::new();
            for sample in &samples {
                hash.update(sample.to_le_bytes());
            }
            frame.insert("audioSha256".into(), json!(hex::encode(hash.finalize())));
            frame.insert("audioSamples".into(), json!(samples.len()));
        }
    }
    Ok(value)
}
fn command(
    runtime: &mut Option<MonitorRuntime>,
    spectrum: &mut Option<SpectrumDevelopmentRuntime>,
    c64: &mut Option<C64OriginalRuntime>,
    command: Command,
) -> Result<Value, String> {
    match command {
        Command::C64Create {
            session_id,
            source_request,
        } => {
            if c64.is_some() {
                return Err("Close the owned C64 first".into());
            }
            *c64 = Some(C64OriginalRuntime::from_source(
                session_id,
                &source_request.to_string(),
            )?);
            Ok(Value::Null)
        }
        Command::C64Monitor { request } => compact(decode(
            &c64.as_mut()
                .ok_or("No owned C64")?
                .command(&request.to_string())?,
        )?),
        Command::C64Close => {
            *c64 = None;
            Ok(Value::Null)
        }
        Command::SpectrumCreate {
            session_id,
            source_request,
        } => {
            if spectrum.is_some() {
                return Err("Close the owned Spectrum runtime first".into());
            }
            *spectrum = Some(SpectrumDevelopmentRuntime::from_source(
                session_id,
                &source_request.to_string(),
            )?);
            Ok(Value::Null)
        }
        Command::SpectrumMonitor { request } => compact(decode(
            &spectrum
                .as_mut()
                .ok_or("No owned Spectrum runtime")?
                .command(&request.to_string())?,
        )?),
        Command::SpectrumClose => {
            *spectrum = None;
            Ok(Value::Null)
        }
        Command::Identity => Ok(json!({
            "manifest": decode(eightbit_dragon_emulator::BASIC_EXTENDED_FIRMWARE_MANIFEST)?,
            "firmwareSha256": hex::encode(Sha256::digest(eightbit_dragon_emulator::BASIC_EXTENDED))
        })),
        Command::Catalog => decode(&browser_request(
            r#"{"schemaVersion":1,"operation":"machines"}"#,
            &[],
        )),
        Command::Build { source } => decode(&browser_request(
            &json!({"schemaVersion":1,"operation":"build","source":source}).to_string(),
            &[],
        )),
        Command::Browser { request, bytes } => {
            decode(&browser_request(&request.to_string(), &bytes))
        }
        Command::LinkInspection {
            sources,
            load_address,
            entry_symbol,
        } => {
            let sources = sources
                .iter()
                .map(|source| eightbit_m6809_core::assembler::AssemblySource {
                    source_uri: &source.source_uri,
                    source: &source.source,
                })
                .collect::<Vec<_>>();
            let code = eightbit_m6809_core::assembler::assemble_project(
                &sources,
                load_address,
                &entry_symbol,
            )
            .map_err(|error| error.to_string())?;
            Ok(json!({
                "loadAddress": code.load_address,
                "entryAddress": code.entry_address,
                "byteCount": code.bytes.len(),
                "sha256": hex::encode(Sha256::digest(&code.bytes)),
                "symbols": code.symbols.iter().map(|symbol| json!({"sourceUri":symbol.source_uri,"name":symbol.name,"address":symbol.address})).collect::<Vec<_>>()
            }))
        }
        Command::Create {
            session_id,
            source_request,
        } => {
            if runtime.is_some() {
                return Err("Close the owned runtime first".into());
            }
            *runtime = Some(MonitorRuntime::from_original_source(
                session_id,
                &source_request.to_string(),
            )?);
            Ok(Value::Null)
        }
        Command::CreateAssembly {
            session_id,
            source_request,
        } => {
            if runtime.is_some() {
                return Err("Close the owned runtime first".into());
            }
            *runtime = Some(MonitorRuntime::from_original_assembly(
                session_id,
                &source_request.to_string(),
            )?);
            Ok(Value::Null)
        }
        Command::CreateMedia {
            session_id,
            media_request,
            bytes,
        } => {
            if runtime.is_some() {
                return Err("Close the owned runtime first".into());
            }
            *runtime = Some(MonitorRuntime::from_original_media(
                session_id,
                &media_request.to_string(),
                &bytes,
            )?);
            Ok(Value::Null)
        }
        Command::Close => {
            *runtime = None;
            Ok(Value::Null)
        }
        action => {
            let runtime = runtime.as_mut().ok_or("No owned runtime")?;
            match action {
                Command::Monitor { request } => {
                    compact(decode(&runtime.command(&request.to_string())?)?)
                }
                Command::DebugState => runtime
                    .debug_state()?
                    .map_or(Ok(Value::Null), |json| compact(decode(&json)?)),
                Command::Debug { request } => {
                    compact(decode(&runtime.debug_command(&request.to_string())?)?)
                }
                _ => Err("Invalid action".into()),
            }
        }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = std::io::stdin();
    let mut input = input.lock();
    let output = std::io::stdout();
    let mut output = output.lock();
    let mut runtime = None;
    let mut spectrum = None;
    let mut c64 = None;
    loop {
        let mut line = String::new();
        // The production API bounds aggregate source to 1 MiB; JSON escaping
        // can expand it sixfold. Real multi-file game fixtures exceed 64 KiB.
        let bytes = (&mut input)
            .take(8 * 1024 * 1024 + 1)
            .read_line(&mut line)?;
        if bytes == 0 {
            break;
        }
        if bytes > 8 * 1024 * 1024 {
            return Err("Acceptance request exceeds 8MiB".into());
        }
        let request: Request = serde_json::from_str(&line)?;
        if request.id == 0 {
            return Err("Invalid request identity".into());
        }
        let reply = match command(&mut runtime, &mut spectrum, &mut c64, request.command) {
            Ok(value) => json!({"id":request.id,"ok":true,"result":value}),
            Err(error) => {
                json!({"id":request.id,"ok":false,"error":error.chars().take(256).collect::<String>()})
            }
        };
        writeln!(output, "{reply}")?;
        output.flush()?;
    }
    Ok(())
}
