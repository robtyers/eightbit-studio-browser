//! Original WAV fixture and physical guest loading for native/WASM comparison.
use eightbit_dragon_emulator::{
    Firmware, MAX_SLICE, MonitorCommand, MonitorRequest, MonitorSession,
};
use sha2::{Digest, Sha256};

fn main() -> Result<(), String> {
    let cassette = eightbit_dragon_media::create_machine_code(
        "WAVTEST",
        &[0xcc, 0, 165, 0xfd, 0x63, 0, 0x39],
        0x6000,
        0x6000,
    )
    .map_err(|error| error.to_string())?;
    let wav = eightbit_dragon_media::encode_cassette_transport_wav(&cassette)
        .map_err(|error| error.to_string())?;
    let fixture =
        serde_json::json!({"bytes": wav, "cassetteSha256": hex::encode(Sha256::digest(&cassette))});
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments == ["--fixture"] {
        println!("{fixture}");
        return Ok(());
    }
    if !arguments.is_empty() {
        return Err("Use --fixture or no arguments".into());
    }
    let mut session = MonitorSession::with_firmware(1, Firmware::BasicDevelopment)?;
    let mut states = Sha256::new();
    let mut command = |command| {
        let snapshot = session.command(MonitorRequest {
            schema_version: 1,
            session_id: 1,
            command,
        })?;
        states.update(serde_json::to_vec(&snapshot).map_err(|_| "Cannot encode cassette frame")?);
        Ok::<_, String>(snapshot)
    };
    for _ in 0..8 {
        command(MonitorCommand::Advance { cycles: MAX_SLICE })?;
    }
    let mounted = command(MonitorCommand::MountWav { bytes: wav })?;
    if mounted.cassette.ok_or("Missing cassette receipt")?.sha256 != fixture["cassetteSha256"] {
        return Err("Recovered tape identity changed".into());
    }
    command(MonitorCommand::Type {
        text: "CLOADM\"WAVTEST\":EXEC:PRINT PEEK(&H6301)".into(),
    })?;
    for _ in 0..256 {
        command(MonitorCommand::Advance { cycles: MAX_SLICE })?;
    }
    let frame = command(MonitorCommand::RewindCassette)?;
    if !frame.machine.text.lines().any(|line| line.trim() == "165")
        || frame.machine.text.contains("ERROR")
    {
        return Err("The guest did not load and execute the authored WAV programme".into());
    }
    println!(
        "{}",
        serde_json::json!({"schemaVersion": 1, "stateTraceSha256": hex::encode(states.finalize()), "cycles": frame.machine.cycles, "pc": frame.machine.pc, "cassetteSha256": frame.cassette.ok_or("Missing rewound cassette")?.sha256})
    );
    Ok(())
}
