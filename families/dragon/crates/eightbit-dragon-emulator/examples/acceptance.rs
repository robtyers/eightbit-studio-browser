//! Original command corpus and compact native/WASM parity receipt.
use eightbit_dragon_emulator::{MAX_SLICE, MonitorCommand, MonitorRequest, MonitorSession};
use sha2::{Digest, Sha256};
fn main() -> Result<(), String> {
    let mut session = MonitorSession::new(1)?;
    let mut audio = Sha256::new();
    let mut states = Sha256::new();
    let mut audio_samples = 0_usize;
    let mut non_silent_audio = false;
    for text in [
        Some("M 0600 41"),
        Some("F 0610 10 42"),
        Some("C 0610 10"),
        Some("L 0620 8641B7040039"),
        Some("G 0620"),
        Some("M 0500 41"),
        Some("R"),
        Some("L 0620 86FEB7FF208634B7FF21B7FF01B7FF03863CB7FF238602B7FF208E0100301F26FC88FC20F2"),
        Some("G 0620"),
        None,
    ] {
        session.command(MonitorRequest {
            schema_version: 1,
            session_id: 1,
            command: text.map_or(MonitorCommand::Reset, |text| MonitorCommand::Type {
                text: text.into(),
            }),
        })?;
        for _ in 0..if text.is_some_and(|text| text.len() > 60) {
            50
        } else {
            30
        } {
            let step = session.command(MonitorRequest {
                schema_version: 1,
                session_id: 1,
                command: MonitorCommand::Advance { cycles: MAX_SLICE },
            })?;
            states.update(serde_json::to_vec(&step).map_err(|_| "State serialisation failed")?);
            audio_samples += step.audio.len();
            for sample in step.audio {
                audio.update(sample.to_le_bytes());
                non_silent_audio |= sample != 0;
            }
        }
    }
    let state = session.snapshot();
    if state.machine.state != "running"
        || !state.machine.text.contains("EIGHTBIT DRAGON MONITOR")
        || !non_silent_audio
    {
        return Err("Monitor acceptance failed".into());
    }
    println!(
        "{}",
        serde_json::json!({"schemaVersion":1,"firmware":state.machine.firmware_id,"cycles":state.machine.cycles,"pc":state.machine.pc,"pixelsSha256":hex::encode(Sha256::digest(&state.machine.pixels)),"textSha256":hex::encode(Sha256::digest(state.machine.text.as_bytes())),"stateTraceSha256":hex::encode(states.finalize()),"audioSha256":hex::encode(audio.finalize()),"audioSamples":audio_samples,"nonSilentAudio":non_silent_audio})
    );
    Ok(())
}
