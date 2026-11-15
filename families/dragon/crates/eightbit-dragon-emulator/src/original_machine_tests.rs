use crate::development_host::DevelopmentSession;
use crate::machine_debug::{MachineDebugTarget, MachineSourceEntry};
use crate::original_runtime::OriginalRuntimeLaunch;
use crate::{MAX_SLICE, MonitorCommand, MonitorRequest, OriginalDevelopmentLaunch};
use serde_json::{Value, json};

fn tick(session: &mut DevelopmentSession) -> Result<crate::SessionSnapshot, String> {
    session.command(MonitorRequest {
        schema_version: 1,
        session_id: session.id(),
        command: MonitorCommand::Advance { cycles: MAX_SLICE },
    })
}
fn state(session: &DevelopmentSession) -> Result<Value, String> {
    serde_json::from_str(&session.debug_state()?.ok_or("Missing machine debugger")?)
        .map_err(|e| e.to_string())
}
fn debug(session: &mut DevelopmentSession, command: &Value) -> Result<Value, String> {
    let request =
        json!({"schemaVersion":1,"revision":state(session)?["revision"],"command":command});
    serde_json::from_str(&session.debug_command(&request.to_string())?).map_err(|e| e.to_string())
}
fn machine(
    address: u16,
    bytes: &[u8],
    debug: Option<MachineDebugTarget>,
) -> Result<DevelopmentSession, String> {
    let cassette = eightbit_dragon_media::create_machine_code("MACHINE", bytes, address, address)
        .map_err(|e| e.to_string())?;
    let launch = OriginalDevelopmentLaunch::with_cassette(cassette, Some("a".repeat(64)))?;
    OriginalRuntimeLaunch::new_machine(launch, debug)?.start(7)
}
#[test]
fn original_machine_guest_loader_executes_low_middle_and_high_ram() -> Result<(), String> {
    for address in [0x0400, 0x0600, 0x6000, 0x7ffa] {
        let mut session = machine(address, &[0x86, 42, 0xb7, 0x63, 0x00, 0x39], None)?;
        assert_eq!(
            session.snapshot().machine.firmware_id,
            "eightbit-dragon32-basic-extended-1"
        );
        let mut done = false;
        for _ in 0..1024 {
            let frame = tick(&mut session)?;
            assert_ne!(
                frame.machine.state, "failed",
                "${address:04X}: {}",
                frame.machine.message
            );
            if !frame.machine.message.starts_with("Loading the current") {
                // Leave enough guest time for the queued command and return.
                for _ in 0..8 {
                    tick(&mut session)?;
                }
                done = true;
                break;
            }
        }
        assert!(done, "${address:04X} never executed");
        session.command(MonitorRequest {
            schema_version: 1,
            session_id: 7,
            command: MonitorCommand::Type {
                text: "PRINT PEEK(25344)".into(),
            },
        })?;
        let mut printed = false;
        for _ in 0..64 {
            if tick(&mut session)?
                .machine
                .text
                .lines()
                .any(|line| line.trim() == "42")
            {
                printed = true;
                break;
            }
        }
        assert!(
            printed,
            "${address:04X}: {}",
            session.snapshot().machine.text
        );
    }
    Ok(())
}
#[test]
fn original_machine_mapped_debug_stops_steps_resumes_and_rejects_stale_requests()
-> Result<(), String> {
    let target = MachineDebugTarget::new(
        0x6000,
        vec![MachineSourceEntry {
            source_uri: "src/asm/main.asm".into(),
            source_line: 1,
            runtime_address: 0x6000,
            byte_length: 7,
        }],
    )?;
    let mut session = machine(
        0x6000,
        &[0x86, 41, 0x4c, 0xb7, 0x63, 0x00, 0x39],
        Some(target),
    )?;
    let mut stopped = false;
    for _ in 0..1024 {
        tick(&mut session)?;
        let s = state(&session)?;
        assert_ne!(s["status"]["state"], "failed", "{s}");
        if s["status"]["state"] == "stopped" {
            stopped = true;
            break;
        }
    }
    assert!(stopped);
    let first = state(&session)?;
    assert_eq!(first["kind"], "machine");
    assert_eq!(first["backend"], "original");
    assert_eq!(first["status"]["registers"]["pc"], 0x6000);
    assert_eq!(first["status"]["location"]["sourceUri"], "src/asm/main.asm");
    assert_eq!(first["status"]["breakpointVerified"], true);
    let stale =
        json!({"schemaVersion":1,"revision":first["revision"],"command":{"operation":"step"}})
            .to_string();
    let stepped = debug(&mut session, &json!({"operation":"step"}))?;
    assert_eq!(stepped["status"]["registers"]["a"], 41);
    assert_eq!(stepped["status"]["registers"]["pc"], 0x6002);
    assert_eq!(stepped["status"]["valuesValid"], true);
    assert_eq!(stepped["status"]["breakpointVerified"], false);
    assert!(session.debug_command(&stale).is_err());
    debug(
        &mut session,
        &json!({"operation":"breakpoint","address":0x6003}),
    )?;
    debug(&mut session, &json!({"operation":"continue"}))?;
    for _ in 0..16 {
        tick(&mut session)?;
        if state(&session)?["status"]["state"] == "stopped" {
            break;
        }
    }
    assert_eq!(state(&session)?["status"]["registers"]["a"], 42);
    let read = debug(
        &mut session,
        &json!({"operation":"memory","address":0x6000,"length":7}),
    )?;
    assert_eq!(
        read["status"]["memory"]["bytes"],
        json!([0x86, 41, 0x4c, 0xb7, 0x63, 0x00, 0x39])
    );
    assert_eq!(
        debug(
            &mut session,
            &json!({"operation":"memory","address":0xff00,"length":1})
        )?["failure"]["code"],
        "DME1301"
    );
    debug(&mut session, &json!({"operation":"continue"}))?;
    debug(&mut session, &json!({"operation":"pause"}))?;
    assert_eq!(state(&session)?["status"]["valuesValid"], true);
    session.command(MonitorRequest {
        schema_version: 1,
        session_id: 7,
        command: MonitorCommand::Stop,
    })?;
    assert_eq!(state(&session)?["status"]["state"], "terminated");
    assert!(tick(&mut session).is_err());
    Ok(())
}
#[test]
fn original_machine_rejects_unsafe_extents_and_unknown_mappings_before_boot() -> Result<(), String>
{
    for (address, entry, length) in [
        (0x03ff, 0x03ff, 1),
        (0x7fff, 0x7fff, 2),
        (0x6000, 0x7000, 1),
    ] {
        let bytes =
            eightbit_dragon_media::create_machine_code("BAD", &vec![0x39; length], address, entry)
                .map_err(|e| e.to_string())?;
        assert!(
            OriginalRuntimeLaunch::new_machine(
                OriginalDevelopmentLaunch::with_cassette(bytes, None)?,
                None
            )
            .is_err()
        );
    }
    assert!(
        MachineDebugTarget::new(
            0x6001,
            vec![MachineSourceEntry {
                source_uri: "a.asm".into(),
                source_line: 1,
                runtime_address: 0x6000,
                byte_length: 1
            }]
        )
        .is_err()
    );
    Ok(())
}
#[test]
fn original_media_runs_tokenised_and_ascii_basic_without_a_source_map() -> Result<(), String> {
    let source = "10 PRINT 42\n20 END\n";
    let dialect = eightbit_dragon_basic_core::Dialect::dragon32().map_err(|e| e.to_string())?;
    let tokens = eightbit_dragon_basic_core::tokenise(
        &eightbit_dragon_basic_core::analyse(source, &dialect).program,
        &dialect,
    )
    .map_err(|e| e.to_string())?;
    for cassette in [
        eightbit_dragon_media::create_tokenised_basic("MEDIA", &tokens.bytes)
            .map_err(|e| e.to_string())?,
        eightbit_dragon_media::create_ascii_basic("MEDIA", source).map_err(|e| e.to_string())?,
    ] {
        let mut session = OriginalRuntimeLaunch::new_media(
            OriginalDevelopmentLaunch::with_cassette(cassette, None)?,
        )?
        .start(8)?;
        let mut done = false;
        for _ in 0..1024 {
            let frame = tick(&mut session)?;
            assert_ne!(frame.machine.state, "failed", "{}", frame.machine.message);
            if frame.machine.text.lines().any(|line| line.trim() == "42") {
                done = true;
                break;
            }
        }
        assert!(done);
        assert!(session.debug_state()?.is_none());
    }
    Ok(())
}

#[test]
fn original_machine_full_ram_uses_guest_bootstrap_and_preserves_exact_payload() -> Result<(), String>
{
    let mut bytes = vec![0x12; 0x7c00];
    bytes[..11].copy_from_slice(&[
        0x10, 0xce, 0x7f, 0xff, 0x86, 42, 0xb7, 0x63, 0x00, 0x20, 0xfe,
    ]);
    // Deliberately distinct final byte proves the inclusive $7FFF boundary.
    bytes[0x7bff] = 0xa5;
    let mut session = machine(
        0x0400,
        &bytes,
        Some(MachineDebugTarget::new(0x0400, Vec::new())?),
    )?;
    let mut reached = false;
    for _ in 0..16384 {
        let frame = tick(&mut session)?;
        assert_ne!(frame.machine.state, "failed", "{}", frame.machine.message);
        if !frame.machine.message.starts_with("Loading the current")
            && state(&session)?["status"]["state"] == "stopped"
        {
            reached = true;
            break;
        }
    }
    assert!(reached, "{}", session.snapshot().machine.message);
    assert_eq!(state(&session)?["status"]["registers"]["s"], 0x0300);
    let public_entry = debug(
        &mut session,
        &json!({"operation":"memory","address":0x9d,"length":2}),
    )?;
    assert_eq!(public_entry["status"]["memory"]["bytes"], json!([4, 0]));
    let public_line = debug(
        &mut session,
        &json!({"operation":"memory","address":0x68,"length":2}),
    )?;
    assert_eq!(public_line["status"]["memory"]["bytes"], json!([255, 255]));
    for (index, chunk) in bytes.chunks(4096).enumerate() {
        let result = debug(
            &mut session,
            &json!({"operation":"memory","address":0x0400+index*4096,"length":chunk.len()}),
        )?;
        assert_eq!(result["status"]["memory"]["bytes"], json!(chunk));
    }
    let step = debug(&mut session, &json!({"operation":"step"}))?;
    assert_eq!(step["status"]["registers"]["s"], 0x7fff);
    debug(&mut session, &json!({"operation":"continue"}))?;
    tick(&mut session)?;
    debug(&mut session, &json!({"operation":"pause"}))?;
    let result = debug(
        &mut session,
        &json!({"operation":"memory","address":0x6300,"length":1}),
    )?;
    assert_eq!(result["status"]["memory"]["bytes"], json!([42]));
    Ok(())
}

#[test]
fn original_machine_cancel_and_reset_do_not_reuse_a_loaded_debug_build() -> Result<(), String> {
    let mut session = machine(
        0x7000,
        &[0x20, 0xfe],
        Some(MachineDebugTarget::new(0x7000, Vec::new())?),
    )?;
    let cancelled = debug(&mut session, &json!({"operation":"cancel"}))?;
    assert_eq!(cancelled["status"]["state"], "cancelled");
    assert_eq!(cancelled["status"]["valuesValid"], false);
    let old_revision = cancelled["revision"].clone();
    for _ in 0..3 {
        tick(&mut session)?;
    }
    assert_eq!(state(&session)?["revision"], old_revision);
    debug(&mut session, &json!({"operation":"continue"}))?;
    for _ in 0..256 {
        tick(&mut session)?;
        if state(&session)?["status"]["state"] == "stopped" {
            break;
        }
    }
    assert_eq!(state(&session)?["status"]["state"], "stopped");
    let old = json!({"schemaVersion":1,"revision":state(&session)?["revision"],"command":{"operation":"step"}});
    session.command(MonitorRequest {
        schema_version: 1,
        session_id: 7,
        command: MonitorCommand::Reset,
    })?;
    assert!(session.debug_command(&old.to_string()).is_err());
    for _ in 0..64 {
        tick(&mut session)?;
    }
    let reset = state(&session)?;
    assert_eq!(reset["status"]["state"], "ready");
    assert_eq!(reset["status"]["valuesValid"], false);
    assert!(reset["status"].get("registers").is_none());
    assert!(session.snapshot().machine.text.contains("OK>"));
    let rejected = debug(&mut session, &json!({"operation":"continue"}))?;
    assert_eq!(rejected["failure"]["code"], "DME1500");
    assert_eq!(rejected["revision"], reset["revision"]);
    Ok(())
}

#[test]
fn original_mixed_guest_basic_loader_reaches_mapped_assembly_and_returns() -> Result<(), String> {
    let dialect = eightbit_dragon_basic_core::Dialect::dragon32().map_err(|e| e.to_string())?;
    let source = "10 CLEAR 200,24575\n20 FOR I=0 TO 6:READ B:POKE 24576+I,B:NEXT I\n30 EXEC 24576\n40 PRINT 42:END\n50 DATA 134,41,76,183,99,0,57\n";
    let analysis = eightbit_dragon_basic_core::analyse(source, &dialect);
    let tokens = eightbit_dragon_basic_core::tokenise(&analysis.program, &dialect)
        .map_err(|e| e.to_string())?;
    let cassette = eightbit_dragon_media::create_tokenised_basic("MIXED", &tokens.bytes)
        .map_err(|e| e.to_string())?;
    let target = MachineDebugTarget::new(
        0x6000,
        vec![MachineSourceEntry {
            source_uri: "src/asm/main.asm".into(),
            source_line: 4,
            runtime_address: 0x6000,
            byte_length: 7,
        }],
    )?;
    let mut session = OriginalRuntimeLaunch::new_mixed(
        OriginalDevelopmentLaunch::with_cassette(cassette, Some("a".repeat(64)))?,
        tokens,
        target,
    )?
    .start(7)?;
    for _ in 0..512 {
        tick(&mut session)?;
        if state(&session)?["status"]["state"] == "stopped" {
            break;
        }
    }
    let stopped = state(&session)?;
    assert_eq!(stopped["status"]["registers"]["pc"], 0x6000);
    assert_eq!(stopped["status"]["location"]["sourceLine"], 4);
    debug(&mut session, &json!({"operation":"continue"}))?;
    let mut printed = false;
    for _ in 0..64 {
        if tick(&mut session)?
            .machine
            .text
            .lines()
            .any(|line| line.trim() == "42")
        {
            printed = true;
            break;
        }
    }
    assert!(printed, "{}", session.snapshot().machine.text);
    Ok(())
}
