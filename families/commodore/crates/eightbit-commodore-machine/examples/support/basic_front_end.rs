//! Named guest-language subset. Programme entry is through physical matrix keys.
use super::{advance, digest, screen, type_text};
use eightbit_commodore_machine::{Firmware, input::Key, pal::Machine, sid::CLOCK_HZ};

fn require(value: bool, message: &'static str) -> Result<(), Box<dyn std::error::Error>> {
    if value { Ok(()) } else { Err(message.into()) }
}
fn exact_line(machine: &Machine<'_>, expected: &str) -> bool {
    screen(machine)
        .lines()
        .any(|line| line.trim_end() == expected)
}
fn programme(machine: &Machine<'_>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let end = u16::from_le_bytes([machine.bus().ram(0x2d), machine.bus().ram(0x2e)]);
    require(
        (0x803..=0xa000).contains(&end),
        "Guest VARTAB escaped BASIC RAM",
    )?;
    let mut bytes = vec![1, 8];
    bytes.extend((0x801..end).map(|address| machine.bus().ram(address)));
    Ok(bytes)
}
fn command(machine: &mut Machine<'_>, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    type_text(machine, text)?;
    type_text(machine, "\r")?;
    advance(machine, CLOCK_HZ / 5)?;
    if text == "LIST" {
        for _ in 0..4 {
            if screen(machine)
                .lines()
                .map(str::trim_end)
                .rfind(|row| !row.is_empty())
                == Some("READY.")
            {
                break;
            }
            advance(machine, CLOCK_HZ / 2)?;
        }
    }
    Ok(())
}

pub fn qualify(firmware: Firmware<'_>) -> Result<String, Box<dyn std::error::Error>> {
    let mut machine = Machine::new(firmware);
    machine.reset()?;
    advance(&mut machine, CLOCK_HZ)?;
    require(
        exact_line(&machine, "READY."),
        "Partial BASIC cold READY failed",
    )?;
    command(&mut machine, "?\"DIRECT\";\" OUTPUT\"")?;
    require(
        exact_line(&machine, "DIRECT OUTPUT"),
        "Guest direct PRINT did not execute",
    )?;

    for phase in 0..32 {
        advance(&mut machine, phase * 257)?;
        command(&mut machine, "?\"SHIFT\"")?;
        let text = screen(&machine);
        let rows: Vec<_> = text.lines().map(str::trim_end).collect();
        let tail: Vec<_> = rows
            .iter()
            .rev()
            .skip_while(|line| line.is_empty())
            .take(3)
            .copied()
            .collect();
        require(
            tail == ["READY.", "SHIFT", "?\"SHIFT\""],
            "Modifier arrival phase lost a quotation mark",
        )?;
    }

    // Even currently unsupported execution tokens must retain their published
    // byte assignments. This corpus never invokes their unimplemented services.
    let words =
        include_str!("../../../eightbit-commodore-core/tests/fixtures/basic-v2-keywords.txt");
    for (index, word) in words.lines().enumerate() {
        command(&mut machine, &format!("{} {word}", index * 10))?;
    }
    let mut at = 0x801u16;
    for index in 0..76u8 {
        require(
            machine.bus().ram(at + 4) == 0x80 + index,
            "Guest keyword token differs",
        )?;
        let next = u16::from_le_bytes([machine.bus().ram(at), machine.bus().ram(at + 1)]);
        require(
            next == at + 6
                && machine.bus().ram(at + 5) == 0
                && u16::from_le_bytes([machine.bus().ram(at + 2), machine.bus().ram(at + 3)])
                    == u16::from(index) * 10,
            "Guest keyword body/number/link changed",
        )?;
        at = next;
    }
    require(
        machine.bus().ram(at) == 0 && machine.bus().ram(at + 1) == 0,
        "Guest keyword links failed",
    )?;
    command(&mut machine, "NEW")?;
    require(
        programme(&machine)? == [1, 8, 0, 0],
        "Guest NEW did not clear text",
    )?;

    let source =
        include_str!("../../../eightbit-commodore-core/tests/fixtures/original-front-end.bas");
    let lines: Vec<_> = source.lines().collect();
    for index in [4, 0, 2, 1, 3] {
        command(&mut machine, lines[index])?;
    }
    let hex =
        include_str!("../../../eightbit-commodore-core/tests/fixtures/original-front-end.hex")
            .trim();
    let expected: Result<Vec<_>, _> = (0..hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&hex[index..index + 2], 16))
        .collect();
    let expected = expected?;
    require(
        programme(&machine)? == expected,
        "Guest sorted linked PRG differs from source contract",
    )?;
    command(&mut machine, "40 REM A LONGER REPLACEMENT")?;
    command(&mut machine, "40 END")?;
    command(&mut machine, "40")?;
    command(&mut machine, lines[3])?;
    command(&mut machine, "15 REM INSERT THEN DELETE")?;
    command(&mut machine, "15")?;
    require(
        programme(&machine)? == expected,
        "Guest replacement/deletion did not relink",
    )?;
    command(&mut machine, "LIST")?;
    let text = screen(&machine);
    let rows: Vec<_> = text.lines().map(str::trim_end).collect();
    let listing = rows
        .iter()
        .rposition(|line| *line == "LIST")
        .ok_or("LIST echo missing")?;
    if rows.get(listing + 1..listing + 6) != Some(lines.as_slice()) {
        return Err(format!("Guest LIST did not expand sorted tokens\n{text}").into());
    }
    command(&mut machine, "RUN")?;
    require(
        exact_line(&machine, "ONE TWO"),
        "Guest RUN/PRINT/GOTO/END failed",
    )?;
    super::await_public_page(&mut machine)?;

    command(&mut machine, "64000 PRINT\"BAD\"")?;
    require(
        exact_line(&machine, "?SYNTAX ERROR") && programme(&machine)? == expected,
        "Invalid line changed programme",
    )?;
    command(&mut machine, &format!("60 REM{}", "X".repeat(80)))?;
    require(
        exact_line(&machine, "?STRING TOO LONG ERROR") && programme(&machine)? == expected,
        "Oversized input changed programme",
    )?;
    command(&mut machine, "PRINT 1+2")?;
    advance(&mut machine, CLOCK_HZ)?;
    require(
        exact_line(&machine, " 3"),
        "Guest numeric PRINT did not execute",
    )?;
    command(&mut machine, "GOTO 49")?;
    require(
        exact_line(&machine, "?UNDEF'D STATEMENT ERROR"),
        "Missing target did not recover",
    )?;
    command(&mut machine, "?\"RECOVEX\x14RED\"")?;
    require(
        exact_line(&machine, "RECOVERED"),
        "Physical delete/error recovery failed",
    )?;
    let programme_hash = digest(&programme(&machine)?);

    command(&mut machine, "NEW")?;
    command(&mut machine, "0 DATA PRINT,\"R:M\":PRINT \"DATA OK\"")?;
    command(&mut machine, "10 REM PRINT+END")?;
    command(&mut machine, "20 PRINT \"PRINT:DATA:REM\"")?;
    command(&mut machine, "63999 END")?;
    let bytes = programme(&machine)?;
    let body = b"\x83 PRINT,\"R:M\":\x99 \"DATA OK\"\0";
    require(
        bytes.get(6..6 + body.len()) == Some(body),
        "Guest DATA/quote tokenisation failed",
    )?;
    command(&mut machine, "LIST")?;
    require(
        exact_line(&machine, "0 DATA PRINT,\"R:M\":PRINT \"DATA OK\"")
            && exact_line(&machine, "10 REM PRINT+END")
            && exact_line(&machine, "20 PRINT \"PRINT:DATA:REM\"")
            && exact_line(&machine, "63999 END"),
        "Guest LIST expanded literal DATA/REM/string bytes",
    )?;
    command(&mut machine, "RUN 0")?;
    require(
        exact_line(&machine, "DATA OK") && exact_line(&machine, "PRINT:DATA:REM"),
        "Guest DATA skip/line-zero RUN failed",
    )?;
    command(&mut machine, "?\"COMMA\",\"ZONE\"")?;
    require(
        exact_line(&machine, "COMMA     ZONE"),
        "Guest PRINT comma zone failed",
    )?;
    command(&mut machine, "10 STOP")?;
    command(&mut machine, "RUN 10")?;
    require(
        exact_line(&machine, "BREAK IN 10"),
        "Guest STOP statement failed",
    )?;

    command(&mut machine, "NEW")?;
    command(&mut machine, "10 GOTO 10")?;
    command(&mut machine, "RUN")?;
    machine.stop();
    machine.resume()?;
    require(
        machine.advance(100, || true)?.cycles == 0,
        "Guest loop cancellation advanced",
    )?;
    machine.resume()?;
    machine.bus_mut().key(Key::new(7, 7)?, true);
    advance(&mut machine, CLOCK_HZ / 10)?;
    machine.bus_mut().release_inputs();
    advance(&mut machine, CLOCK_HZ / 10)?;
    require(
        exact_line(&machine, "BREAK IN 10") && exact_line(&machine, "READY."),
        "Physical STOP did not break guest loop",
    )?;
    command(&mut machine, "?\"AFTER BREAK\"")?;
    require(
        exact_line(&machine, "AFTER BREAK"),
        "Guest loop recovery failed",
    )?;
    machine.reset()?;
    advance(&mut machine, CLOCK_HZ)?;
    require(
        programme(&machine)? == [1, 8, 0, 0] && exact_line(&machine, "READY."),
        "Guest reset did not clear programme",
    )?;
    Ok(format!(
        "{{\"schemaVersion\":1,\"keyboardOnly\":true,\"modifierPhaseCount\":32,\"keywordCount\":76,\"linkedPrgSha256\":\"{programme_hash}\",\"editListRun\":true,\"literalPrintGotoEnd\":true,\"dataRemQuotes\":true,\"lineZeroAnd63999\":true,\"errorRecovery\":true,\"physicalStop\":true,\"stopCancelReset\":true,\"numericEngineImplemented\":true,\"basicV2Complete\":false}}"
    ))
}
