//! Shared original/supplied-ROM behavioural observations, entered by matrix keys.
use super::{advance, digest, screen, type_text};
use eightbit_commodore_machine::{Firmware, pal::Machine, sid::CLOCK_HZ};

fn command(machine: &mut Machine<'_>, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    type_text(machine, text)?;
    type_text(machine, "\r")?;
    if text.as_bytes().first().is_some_and(u8::is_ascii_digit) {
        advance(machine, CLOCK_HZ)?;
        return Ok(());
    }
    for _ in 0..5 {
        advance(machine, CLOCK_HZ)?;
        if screen(machine)
            .lines()
            .map(str::trim_end)
            .rfind(|row| !row.is_empty())
            == Some("READY.")
        {
            return Ok(());
        }
    }
    Err(format!(
        "Numeric command did not return to READY: {text}\n{}",
        screen(machine)
    )
    .into())
}
fn contains_line(machine: &Machine<'_>, expected: &str) -> bool {
    screen(machine)
        .lines()
        .map(str::trim_end)
        .filter(|row| !row.is_empty())
        .rev()
        .nth(1)
        == Some(expected)
}
fn pointer(machine: &Machine<'_>, address: u16) -> u16 {
    u16::from_le_bytes([machine.bus().ram(address), machine.bus().ram(address + 1)])
}
pub fn qualify(firmware: Firmware<'_>) -> Result<String, Box<dyn std::error::Error>> {
    let mut machine = Machine::new(firmware);
    machine.reset()?;
    advance(&mut machine, CLOCK_HZ * 3)?;
    let cases = [
        ("1+2*3", "R 7 Q"),
        ("(1+2)*3", "R 9 Q"),
        ("8/4/2", "R 1 Q"),
        ("1-2-3", "R-4 Q"),
        ("-(2+3)*-4", "R 20 Q"),
        (".1+.2", "R .3 Q"),
        ("1E-2", "R .01 Q"),
        ("1E-3", "R 1E-03 Q"),
        ("999999999", "R 999999999 Q"),
        ("1E9", "R 1E+09 Q"),
        ("2.938735877E-39", "R 2.93873588E-39 Q"),
        ("1E-50", "R 0 Q"),
    ];
    let mut trace = Vec::new();
    for (expression, expected) in cases {
        command(&mut machine, &format!("PRINT\"R\";{expression};\"Q\""))?;
        if !contains_line(&machine, expected) {
            return Err(format!(
                "Numeric keyboard {expression}: expected {expected}\n{}",
                screen(&machine)
            )
            .into());
        }
        trace.extend_from_slice(expected.as_bytes());
        trace.push(0);
    }
    command(&mut machine, "NEW")?;
    for text in [
        "A=1+1/4294967296",
        "B=1+3/4294967296",
        "C=-1-1/4294967296",
        "D=1-1/4294967296",
        "E=1-1/8589934592",
        "F=-1+1/8589934592",
        "G%=1.9",
        "H%=-1.9",
        "I%=-.1",
        "J%=-32768",
        "K%=32767.9",
    ] {
        command(&mut machine, text)?;
    }
    let start = pointer(&machine, 0x2d);
    let end = pointer(&machine, 0x2f);
    let expected: [u8; 77] = [
        65, 0, 129, 0, 0, 0, 1, 66, 0, 129, 0, 0, 0, 2, 67, 0, 129, 128, 0, 0, 1, 68, 0, 128, 127,
        255, 255, 255, 69, 0, 129, 0, 0, 0, 0, 70, 0, 129, 128, 0, 0, 0, 199, 128, 0, 1, 0, 0, 0,
        200, 128, 255, 254, 0, 0, 0, 201, 128, 255, 255, 0, 0, 0, 202, 128, 128, 0, 0, 0, 0, 203,
        128, 127, 255, 0, 0, 0,
    ];
    if end != start + 77 || !(start..end).map(|at| machine.bus().ram(at)).eq(expected) {
        let actual: Vec<_> = (start..end.min(start.saturating_add(84)))
            .map(|at| machine.bus().ram(at))
            .collect();
        return Err(format!("Public scalar records differ: {actual:02x?}").into());
    }
    trace.extend(expected);
    for (text, expected) in [
        (
            "PRINT\"R\";G%;H%;I%;J%;K%;\"Q\"",
            "R 1 -2 -1 -32768  32767 Q",
        ),
        ("ABXYZ=17:PRINT\"R\";AB;\"Q\"", "R 17 Q"),
        ("LET A=(AB+3)/2:PRINT\"R\";A;\"Q\"", "R 10 Q"),
        ("PRINT\"R\";ZZ;\"Q\"", "R 0 Q"),
    ] {
        command(&mut machine, text)?;
        if !contains_line(&machine, expected) {
            return Err(format!(
                "Scalar keyboard {text}: expected {expected}\n{}",
                screen(&machine)
            )
            .into());
        }
        trace.extend_from_slice(expected.as_bytes());
        trace.push(0);
    }
    // A failed assignment must leave the previous value usable after recovery.
    command(&mut machine, "A=10")?;
    for (text, expected) in [
        ("A=1/0", "?DIVISION BY ZERO ERROR"),
        ("A=1E39", "?OVERFLOW ERROR"),
        ("G%=32768", "?ILLEGAL QUANTITY ERROR"),
        ("H%=-32768.9", "?ILLEGAL QUANTITY ERROR"),
    ] {
        command(&mut machine, text)?;
        let display = screen(&machine);
        let rows: Vec<_> = display.lines().map(str::trim_end).collect();
        let actual = rows
            .iter()
            .rposition(|row| *row == text)
            .and_then(|at| rows[at + 1..].iter().find(|row| !row.is_empty()));
        if actual
            .map(|row| row.split_whitespace().collect::<Vec<_>>().join(" "))
            .as_deref()
            != Some(expected)
        {
            return Err(format!("Numeric error {text}: expected {expected}\n{display}").into());
        }
        trace.extend_from_slice(expected.as_bytes());
        trace.push(0);
    }
    command(&mut machine, "PRINT\"R\";A;G%;H%;\"Q\"")?;
    if !contains_line(&machine, "R 10  1 -2 Q") {
        return Err(format!("Numeric assignment recovery failed\n{}", screen(&machine)).into());
    }
    for text in ["NEW", "10 A=2", "20 A=A*3:PRINT\"RUN\";A", "30 END", "RUN"] {
        command(&mut machine, text)?;
    }
    if !contains_line(&machine, "RUN 6") {
        return Err(format!("Programme numeric state failed\n{}", screen(&machine)).into());
    }
    Ok(format!(
        "{{\"schemaVersion\":1,\"keyboardOnly\":true,\"expressions\":{},\"packedScalarRecords\":11,\"assignmentRecovery\":true,\"programmeExpressions\":true,\"traceSha256\":\"{}\",\"basicV2Complete\":false}}",
        cases.len(),
        digest(&trace)
    ))
}
