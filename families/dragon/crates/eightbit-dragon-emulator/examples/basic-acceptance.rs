//! Original development BASIC command corpus; native/WASM snapshot/audio parity.
use eightbit_dragon_emulator::{
    Firmware, MAX_SLICE, MonitorCommand, MonitorRequest, MonitorSession,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Corpus {
    schema_version: u32,
    commands: Vec<Option<String>>,
}
fn main() -> Result<(), String> {
    let corpus: Corpus = serde_json::from_str(include_str!("basic-acceptance.json"))
        .map_err(|_| "Invalid original BASIC corpus")?;
    if corpus.schema_version != 1 || corpus.commands.len() > 32 {
        return Err("Invalid original BASIC corpus limits".into());
    }
    let mut session = MonitorSession::with_firmware(1, Firmware::BasicDevelopment)?;
    let mut audio = Sha256::new();
    let mut states = Sha256::new();
    let mut audio_samples = 0_usize;
    let mut non_silent_audio = false;
    for (index, text) in corpus.commands.into_iter().enumerate() {
        let length = text.as_ref().map_or(0, String::len);
        let curve_ready = text
            .as_ref()
            .is_some_and(|text| text.contains("CURVE READY"));
        let paint_ready = text
            .as_ref()
            .is_some_and(|text| text.contains("PAINT READY"));
        let mid_ready = text.as_ref().is_some_and(|text| text.contains("MID READY"));
        let timer_ready = text
            .as_ref()
            .is_some_and(|text| text.contains("TIMER READY"));
        let buffer_ready = text
            .as_ref()
            .is_some_and(|text| text.contains("BUFFER READY"));
        let completion_marker = [
            "RADIX READY",
            "NUMBER READY",
            "SAVE READY",
            "ASCII READY",
            "RND READY",
        ]
        .into_iter()
        .find(|marker| text.as_ref().is_some_and(|text| text.contains(marker)));
        let settle_frames = if text.as_ref().is_some_and(|text| text.starts_with("CSAVE")) {
            128
        } else {
            48
        };
        session.command(MonitorRequest {
            schema_version: 1,
            session_id: 1,
            command: text.map_or(MonitorCommand::Reset, |text| MonitorCommand::Type { text }),
        })?;
        for _ in 0..length.saturating_mul(24_000).div_ceil(32_768) + settle_frames {
            let step = session.command(MonitorRequest {
                schema_version: 1,
                session_id: 1,
                command: MonitorCommand::Advance { cycles: MAX_SLICE },
            })?;
            if step.machine.state != "running" || step.machine.text.contains(" ERROR") {
                return Err(format!(
                    "Development BASIC corpus failed at command {}: {}: {}",
                    index + 1,
                    step.machine.message,
                    step.machine.text
                ));
            }
            states.update(serde_json::to_vec(&step).map_err(|_| "State serialisation failed")?);
            audio_samples += step.audio.len();
            for sample in step.audio {
                audio.update(sample.to_le_bytes());
                non_silent_audio |= sample != 0;
            }
        }
        if curve_ready && !session.snapshot().machine.text.contains("CURVE READY") {
            return Err("The guest curve did not return to the command loop".into());
        }
        if paint_ready && !session.snapshot().machine.text.contains("PAINT READY") {
            return Err("The guest PAINT did not return to the command loop".into());
        }
        if mid_ready && !session.snapshot().machine.text.contains("MID READY") {
            return Err("The guest MID$ commands did not return to the command loop".into());
        }
        if timer_ready && !session.snapshot().machine.text.contains("TIMER READY") {
            return Err("The guest unsigned TIMER command did not return to the prompt".into());
        }
        if let Some(marker) = completion_marker
            && !session.snapshot().machine.text.contains(marker)
        {
            return Err(format!("The guest command did not complete: {marker}"));
        }
        if buffer_ready {
            let text = session.snapshot().machine.text;
            if !text.contains("BUFFER READY") || !text.lines().any(|line| line.trim() == "85") {
                return Err("Direct tokenisation changed the cassette buffer".into());
            }
        }
    }
    let frame = session.snapshot();
    if !frame.machine.text.contains("DEV>") || !non_silent_audio {
        return Err("Development BASIC output/audio acceptance failed".into());
    }
    println!(
        "{}",
        serde_json::json!({"schemaVersion":1,"firmware":frame.machine.firmware_id,"cycles":frame.machine.cycles,"pc":frame.machine.pc,"pixelsSha256":hex::encode(Sha256::digest(&frame.machine.pixels)),"textSha256":hex::encode(Sha256::digest(frame.machine.text.as_bytes())),"stateTraceSha256":hex::encode(states.finalize()),"audioSha256":hex::encode(audio.finalize()),"audioSamples":audio_samples,"nonSilentAudio":non_silent_audio})
    );
    Ok(())
}
