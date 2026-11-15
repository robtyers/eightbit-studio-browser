use super::*;

#[test]
fn dragon64_boots_both_modes_and_uses_their_public_memory_limits() -> Result<(), String> {
    for (profile, top) in [
        (Firmware::Dragon64Compatibility, 0x7ffe),
        (Firmware::Dragon64Native, 0xbffe),
    ] {
        let mut machine = Machine::with_firmware(profile);
        for _ in 0..32 {
            machine.advance(MAX_SLICE).map_err(|e| {
                format!(
                    "{profile:?}: {e}; {:?}; {:?}",
                    machine.registers(),
                    machine.snapshot().text
                )
            })?;
        }
        assert!(
            machine.snapshot().text.contains("OK>"),
            "{:?}",
            machine.snapshot()
        );
        assert_eq!(machine.board.ram.len(), 65536);
        assert_eq!(
            u16::from_be_bytes([machine.peek(0x27), machine.peek(0x28)]),
            top
        );
        machine.type_text("PRINT 2*3*4")?;
        for _ in 0..128 {
            machine.advance(MAX_SLICE).map_err(|e| {
                format!(
                    "{profile:?}: {e}; {:?}; {:?}",
                    machine.registers(),
                    machine.snapshot().text
                )
            })?;
        }
        assert!(
            machine
                .snapshot()
                .text
                .lines()
                .any(|line| line.trim() == "24"),
            "{:?}",
            machine.snapshot().text
        );
    }
    Ok(())
}

fn launch_program(
    native: bool,
    source: &str,
    breakpoint: Option<u16>,
) -> Result<development_host::DevelopmentSession, String> {
    use eightbit_dragon_basic_core::{
        Dialect, analyse, inspection::source_variable_names, tokenise,
    };
    let dialect = if native {
        Dialect::for_id("dragon64-basic")
    } else {
        Dialect::dragon32()
    }
    .map_err(|e| e.to_string())?;
    let analysis = analyse(source, &dialect);
    let program = tokenise(&analysis.program, &dialect).map_err(|e| e.to_string())?;
    let cassette = eightbit_dragon_media::create_tokenised_basic("D64TEST", &program.bytes)
        .map_err(|e| e.to_string())?;
    original_runtime::OriginalRuntimeLaunch::new_dragon64(
        native,
        OriginalDevelopmentLaunch::with_cassette(cassette, Some("a".repeat(64)))?,
        program,
        breakpoint,
        source_variable_names(source),
    )?
    .start(64)
}
fn tick64(session: &mut development_host::DevelopmentSession) -> Result<SessionSnapshot, String> {
    session.command(MonitorRequest {
        schema_version: 1,
        session_id: session.id(),
        command: MonitorCommand::Advance { cycles: MAX_SLICE },
    })
}
#[test]
fn dragon64_current_cassette_loads_and_runs_in_both_modes() -> Result<(), String> {
    for native in [false, true] {
        let mut session = launch_program(native, "10 A=41\n20 A=A+1:PRINT A\n30 END\n", None)?;
        let mut completed = false;
        for _ in 0..1024 {
            let frame = tick64(&mut session)?;
            assert_ne!(
                frame.machine.state, "failed",
                "native={native}: {} {:?}",
                frame.machine.message, frame.machine.text
            );
            if frame.machine.text.lines().any(|line| line.trim() == "42") {
                completed = true;
                break;
            }
        }
        assert!(
            completed,
            "native={native}: {:?}",
            session.snapshot().machine.text
        );
    }
    Ok(())
}

#[test]
fn dragon64_native_language_control_graphics_and_high_ram() -> Result<(), String> {
    let source = "10 A$=\"ABC\":DIM A(3):FOR I=1 TO 3:A(I)=I*I:NEXT I\n20 DATA 7,HELLO\n30 READ B,B$:GOSUB 100\n40 DEF FNF(X)=X*X:PRINT USING\"###\";FNF(6)\n50 PMODE 4,1:PCLS:SCREEN 1,1:CIRCLE(128,96),20:DRAW\"BM20,20R20D20\":SCREEN 0,0\n60 POKE 40000,77:IF PEEK(40000)<>77 THEN STOP\n70 IF A(3)<>9 OR B<>8 OR B$<>\"HELLO\" THEN STOP\n80 IF MID$(A$,2,1)<>\"B\" THEN STOP\n90 PRINT\"D64PASS\":END\n100 B=B+1:RETURN\n";
    let mut session = launch_program(true, source, None)?;
    for _ in 0..2048 {
        let frame = tick64(&mut session)?;
        assert_ne!(
            frame.machine.state, "failed",
            "{} {:?}",
            frame.machine.message, frame.machine.text
        );
        if frame.machine.text.contains("D64PASS") {
            return Ok(());
        }
    }
    Err(format!(
        "Native language programme did not finish: {:?}",
        session.snapshot().machine.text
    ))
}

#[test]
fn dragon64_exec_48000_enters_native_and_reset_returns_to_selected_mode() -> Result<(), String> {
    let mut machine = Machine::with_firmware(Firmware::Dragon64Compatibility);
    for _ in 0..32 {
        machine.advance(MAX_SLICE)?;
    }
    machine.type_text("EXEC 48000")?;
    for _ in 0..128 {
        machine.advance(MAX_SLICE)?;
    }
    assert_eq!(machine.board.profile, Firmware::Dragon64Native);
    assert!(machine.snapshot().text.contains("NATIVE BASIC"));
    assert_eq!(machine.peek(0x27), 0xbf);
    machine.reset();
    assert_eq!(machine.board.profile, Firmware::Dragon64Compatibility);
    for _ in 0..32 {
        machine.advance(MAX_SLICE)?;
    }
    assert_eq!(machine.peek(0x27), 0x7f);
    Ok(())
}

#[test]
fn dragon64_native_isolated_language_cases() -> Result<(), String> {
    for statement in [
        "A$=\"ABC\"",
        "DIM A(3)",
        "FOR I=1 TO 3:A(I)=I*I:NEXT I",
        "GOSUB 100",
        "DEF FNF(X)=X*X:PRINT FNF(6)",
        "READ B,B$",
        "PMODE 4,1:PCLS:SCREEN 1,1:CIRCLE(128,96),20:DRAW\"BM20,20R20D20\":SCREEN 0,0",
        "POKE40000,77",
    ] {
        let source =
            format!("10 {statement}\n20 PRINT\"PASS64\":END\n100 RETURN\n200 DATA 7,HELLO\n");
        let mut session = launch_program(true, &source, None)?;
        let mut completed = false;
        for _ in 0..1024 {
            let frame = tick64(&mut session)?;
            assert_ne!(
                frame.machine.state, "failed",
                "{statement}: {} {:?}",
                frame.machine.message, frame.machine.text
            );
            if frame.machine.text.contains("PASS64") {
                completed = true;
                break;
            }
        }
        assert!(
            completed,
            "{statement}: {:?}",
            session.snapshot().machine.text
        );
    }
    Ok(())
}

#[test]
fn dragon64_basic_debug_is_bound_to_native_hook_loaded_bytes_and_revisions() -> Result<(), String> {
    for native in [false, true] {
        let mut session = launch_program(native, "10 A=41\n20 A=A+1:PRINT A\n30 END\n", Some(20))?;
        let mut stopped = None;
        for _ in 0..1024 {
            let frame = tick64(&mut session)?;
            assert_ne!(frame.machine.state, "failed", "{}", frame.machine.message);
            if let Some(json) = session.debug_state()? {
                let state: serde_json::Value =
                    serde_json::from_str(&json).map_err(|e| e.to_string())?;
                if state["status"]["state"] == "stopped" {
                    stopped = Some(state);
                    break;
                }
            }
        }
        let state = stopped.ok_or("D64 breakpoint was not reached")?;
        assert_eq!(
            state["status"]["variables"][0]["numericValue"].as_f64(),
            Some(41.0)
        );
        let command = serde_json::json!({"schemaVersion":1,"revision":state["revision"],"command":{"operation":"continue"}}).to_string();
        session.debug_command(&command)?;
        assert!(
            session.debug_command(&command).is_err(),
            "stale continuation is rejected"
        );
    }
    Ok(())
}

#[test]
fn dragon64_native_error_recovery_and_page_controls_reject_ram_authority() -> Result<(), String> {
    let mut machine = Machine::with_firmware(Firmware::Dragon64Native);
    for _ in 0..32 {
        machine.advance(MAX_SLICE)?;
    }
    for text in ["PRINT 1/0", "DIM A(0)", "A(2)=1", "PRINT 12345"] {
        machine.type_text(text)?;
        for _ in 0..128 {
            machine.advance(MAX_SLICE)?;
        }
        assert!(!machine.board.paging.fault);
    }
    assert!(machine.snapshot().text.contains("12345"));
    let selected = machine.board.paging.page;
    machine.board.instruction_pc = 0x6500;
    machine.board.instruction_stack = 0xb000;
    for address in 0xff40..=0xff43 {
        machine.board.write(address, 255);
    }
    assert_eq!(machine.board.paging.page, selected);
    assert_eq!(machine.board.profile, Firmware::Dragon64Native);
    let receipt: serde_json::Value =
        serde_json::from_str(dragon64::BASIC64_MANIFEST).map_err(|e| e.to_string())?;
    let controls = receipt["profiles"]["native"]["controls"]
        .as_object()
        .ok_or("Missing native controls")?;
    let (pc, _) = controls
        .iter()
        .find(|(_, port)| **port == 0xff40)
        .ok_or("Missing native push gate")?;
    machine.board.instruction_pc = pc.parse().map_err(|_| "Invalid native gate")?;
    machine.board.sam.control |= 0x8000;
    machine.board.write(0xff40, 1);
    assert_eq!(
        machine.board.paging.page, selected,
        "RAM at a gate address owns no private ROM control"
    );
    machine.board.sam.control &= !0x8000;
    machine.board.write(0x8000, 0x57);
    machine.board.write(0x0000, 0xa5);
    assert_eq!(machine.peek(0x8000), 0x57, "upper RAM is not a 32K alias");
    machine.reset();
    assert_eq!(machine.board.paging.page, 0);
    assert!(!machine.board.paging.fault);
    Ok(())
}

#[test]
fn dragon64_serial_registers_and_printer_selection_use_the_guest_acia() -> Result<(), String> {
    for profile in [Firmware::Dragon64Compatibility, Firmware::Dragon64Native] {
        let mut machine = Machine::with_firmware(profile);
        for _ in 0..32 {
            machine.advance(MAX_SLICE)?;
        }
        assert_eq!(machine.board.read(0xff07), 0x98);
        assert_eq!(machine.board.read(0xff06), 0x0b);
        assert_eq!(machine.board.read(0xff05) & 0x18, 0x10);
        machine.type_text("POKE &H3FF,1:PRINT#-2,\"SERIAL\"")?;
        for _ in 0..128 {
            machine.advance(MAX_SLICE)?;
        }
        let bytes = machine.take_serial_output();
        assert!(bytes.windows(6).any(|w| w == b"SERIAL"), "{bytes:?}");
        assert!(machine.take_printer().is_empty());
        let bank = if profile == Firmware::Dragon64Native {
            0xc0
        } else {
            0x80
        };
        // Original RAM probe calls SERSET at the highest valid selector,
        // records invalid-selector carry, then transmits through SEROUT.
        let probe = [
            0xc6, 6, 0xbd, bank, 0x30, 0xb6, 0xff, 7, 0xb7, 0x66, 0, 0xc6, 7, 0xbd, bank, 0x30,
            0x1f, 0xa8, 0xb7, 0x66, 1, 0x86, 42, 0xbd, bank, 0x2d, 0x39,
        ];
        for (offset, byte) in probe.iter().enumerate() {
            machine.board.ram[0x6500 + offset] = *byte;
        }
        machine.type_text("EXEC 25856")?;
        for _ in 0..128 {
            machine.advance(MAX_SLICE)?;
        }
        assert_eq!(machine.peek(0x6600), 0x9e);
        assert_eq!(machine.peek(0x6601) & 1, 1);
        assert_eq!(machine.take_serial_output(), [42]);
    }
    Ok(())
}

#[test]
fn dragon64_native_machine_code_loads_executes_and_debugs_above_32k() -> Result<(), String> {
    let bytes = eightbit_dragon_media::create_machine_code(
        "HIGH",
        &[0x86, 0x57, 0xb7, 0x65, 0x00, 0x20, 0xfe],
        0xa000,
        0xa000,
    )
    .map_err(|e| e.to_string())?;
    let launch = original_runtime::OriginalRuntimeLaunch::new_dragon64_machine(
        true,
        OriginalDevelopmentLaunch::with_cassette(bytes, Some("b".repeat(64)))?,
        Some(machine_debug::MachineDebugTarget::new_dragon64(
            true,
            0xa000,
            Vec::new(),
        )?),
    )?;
    let mut session = launch.start(65)?;
    for _ in 0..1024 {
        let frame = tick64(&mut session)?;
        assert_ne!(
            frame.machine.state, "failed",
            "{} {:?}",
            frame.machine.message, frame.machine.text
        );
        if let Some(json) = session.debug_state()? {
            let value: serde_json::Value =
                serde_json::from_str(&json).map_err(|e| e.to_string())?;
            if value["status"]["state"] == "stopped" {
                assert_eq!(value["status"]["registers"]["pc"], 0xa000);
                return Ok(());
            }
        }
    }
    Err("Native high-RAM source debugger did not stop".into())
}

#[test]
fn dragon64_native_transition_preserves_the_existing_programme() -> Result<(), String> {
    let mut machine = Machine::with_firmware(Firmware::Dragon64Compatibility);
    for _ in 0..32 {
        machine.advance(MAX_SLICE)?;
    }
    for text in ["PCLEAR 2", "10 PRINT 31415"] {
        machine.type_text(text)?;
        for _ in 0..128 {
            machine.advance(MAX_SLICE)?;
        }
    }
    let start = u16::from_be_bytes([machine.peek(0x19), machine.peek(0x1a)]);
    let end = u16::from_be_bytes([machine.peek(0x1b), machine.peek(0x1c)]);
    let bytes = machine.read_memory(start, usize::from(end - start))?;
    machine.type_text("EXEC 48000")?;
    for _ in 0..128 {
        machine.advance(MAX_SLICE)?;
    }
    assert_eq!(machine.board.profile, Firmware::Dragon64Native);
    assert_eq!(
        u16::from_be_bytes([machine.peek(0x19), machine.peek(0x1a)]),
        start
    );
    assert_eq!(machine.read_memory(start, bytes.len())?, bytes);
    machine.type_text("RUN")?;
    for _ in 0..128 {
        machine.advance(MAX_SLICE)?;
    }
    assert!(machine.snapshot().text.contains("31415"));
    Ok(())
}

#[test]
fn dragon64_native_keyboard_repeats_but_compatibility_preserves_single_presses()
-> Result<(), String> {
    for (profile, repeat) in [
        (Firmware::Dragon64Compatibility, false),
        (Firmware::Dragon64Native, true),
    ] {
        let mut machine = Machine::with_firmware(profile);
        for _ in 0..32 {
            machine.advance(MAX_SLICE)?;
        }
        assert_eq!(machine.peek(0x11f), 5);
        machine.key("KeyA", true)?;
        for _ in 0..50 {
            machine.advance(MAX_SLICE)?;
        }
        machine.key("KeyA", false)?;
        for _ in 0..8 {
            machine.advance(MAX_SLICE)?;
        }
        let text = machine.snapshot().text;
        let line = text
            .lines()
            .find(|line| line.starts_with("OK> "))
            .ok_or("Missing typed line")?;
        let count = line.bytes().filter(|byte| *byte == b'A').count();
        assert_eq!(count > 1, repeat, "{profile:?}: {text:?}");
        assert!(count >= 1);
    }
    Ok(())
}

#[test]
fn dragon64_native_mem_reports_unsigned_free_space() -> Result<(), String> {
    let mut session = launch_program(
        true,
        "10 IF MEM>32768 THEN PRINT\"MEMPASS\"\n20 END\n",
        None,
    )?;
    for _ in 0..1024 {
        let frame = tick64(&mut session)?;
        assert_ne!(frame.machine.state, "failed", "{}", frame.machine.message);
        if frame.machine.text.contains("MEMPASS") {
            return Ok(());
        }
    }
    Err("Native MEM did not report its larger positive free space".into())
}
