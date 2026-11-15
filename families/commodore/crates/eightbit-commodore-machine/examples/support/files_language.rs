//! Original keyboard/screen channels and explicit unattached-device errors.
use super::{advance, digest, screen, type_text};
use eightbit_commodore_machine::{Firmware, pal::Machine, sid::CLOCK_HZ};
fn command(machine: &mut Machine<'_>, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    type_text(machine, text)?;
    type_text(machine, "\r")?;
    advance(machine, CLOCK_HZ * 2)?;
    Ok(())
}
fn require(machine: &Machine<'_>, expected: &str) -> Result<(), Box<dyn std::error::Error>> {
    let display = screen(machine);
    let actual = display
        .lines()
        .map(str::trim_end)
        .filter(|row| !row.is_empty())
        .rev()
        .nth(1)
        .map(|row| row.split_whitespace().collect::<Vec<_>>().join(" "));
    if actual.as_deref() == Some(expected) {
        Ok(())
    } else {
        Err(format!("File corpus expected {expected}\n{display}").into())
    }
}
pub fn qualify(firmware: Firmware<'_>) -> Result<String, Box<dyn std::error::Error>> {
    let mut machine = Machine::new(firmware);
    machine.reset()?;
    advance(&mut machine, CLOCK_HZ * 3)?;
    command(&mut machine, "OPEN1,3:PRINT#1,\"CHANNEL\"")?;
    require(&machine, "CHANNEL")?;
    for text in [
        "10 OPEN2,0:INPUT#2,A,B$",
        "20 PRINT\"FILEIN\";A;B$",
        "RUN",
        "12.5,HELLO",
    ] {
        command(&mut machine, text)?;
    }
    require(&machine, "12.5,HELLO FILEIN 12.5 HELLO")?;
    command(&mut machine, "INPUT#2,A$:PRINT\"DIRECTFILE\";A$")?;
    require(&machine, "?ILLEGAL DIRECT ERROR")?;
    command(&mut machine, "OPEN1,3")?;
    let errors = [
        ("OPEN1,3", "?FILE OPEN ERROR"),
        ("PRINT#9,\"BAD\"", "?FILE NOT OPEN ERROR"),
        ("PRINT#2,\"BAD\"", "?NOT OUTPUT FILE ERROR"),
        ("INPUT#1,A", "?ILLEGAL DIRECT ERROR"),
        ("OPEN3,8,2,\"SAMPLE\"", "?DEVICE NOT PRESENT ERROR"),
        ("LOAD\"SAMPLE\",8", "?DEVICE NOT PRESENT ERROR"),
        ("SAVE\"SAMPLE\",8", "?DEVICE NOT PRESENT ERROR"),
        ("VERIFY\"SAMPLE\",8", "?DEVICE NOT PRESENT ERROR"),
    ];
    let mut trace = Vec::new();
    for (text, expected) in errors {
        command(&mut machine, text)?;
        require(&machine, expected)?;
        trace.extend_from_slice(expected.as_bytes());
        trace.push(0);
    }
    command(&mut machine, "CLOSE1:CLOSE2:PRINT\"CLOSED\";PEEK(152)")?;
    require(&machine, "CLOSED 1")?;
    command(&mut machine, "CLOSE3:PRINT\"ALLCLOSED\";PEEK(152)")?;
    require(&machine, "ALLCLOSED 0")?;
    command(&mut machine, "PRINT\"RECOVER\";6*7")?;
    require(&machine, "RECOVER 42")?;
    Ok(format!(
        "{{\"schemaVersion\":1,\"keyboardOnly\":true,\"screenChannel\":true,\"keyboardChannel\":true,\"directInputRejected\":true,\"errorCases\":{},\"unattachedDevicesFail\":true,\"closeAndRecovery\":true,\"traceSha256\":\"{}\",\"basicV2Complete\":false}}",
        errors.len(),
        digest(&trace)
    ))
}
