//! Keyboard-only INPUT/GET success, retry and lifecycle observations.
use super::{advance, digest, screen, type_text};
use eightbit_commodore_machine::{Firmware, input::Key, pal::Machine, sid::CLOCK_HZ};
fn command(machine: &mut Machine<'_>, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    type_text(machine, text)?;
    type_text(machine, "\r")?;
    advance(machine, CLOCK_HZ * 3)?;
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
        Err(format!("Input corpus expected {expected}\n{display}").into())
    }
}
pub fn qualify(firmware: Firmware<'_>) -> Result<String, Box<dyn std::error::Error>> {
    let mut machine = Machine::new(firmware);
    machine.reset()?;
    advance(&mut machine, CLOCK_HZ * 3)?;
    for text in [
        "10 INPUT\"ENTER\";A,B$",
        "20 PRINT\"IN\";A;B$",
        "RUN",
        "12.5,HELLO",
    ] {
        command(&mut machine, text)?;
    }
    require(&machine, "IN 12.5 HELLO")?;
    command(&mut machine, "RUN")?;
    command(&mut machine, "XX")?;
    if !screen(&machine).contains("?REDO FROM START") {
        return Err("Invalid INPUT did not request retry".into());
    }
    command(&mut machine, "-2.5,OK")?;
    require(&machine, "IN-2.5 OK")?;
    for text in [
        "NEW",
        "10 INPUT A,B",
        "20 PRINT\"SUM\";A+B",
        "RUN",
        "2",
        "3",
    ] {
        command(&mut machine, text)?;
    }
    require(&machine, "SUM 5")?;
    for text in ["NEW", "10 INPUT A", "20 PRINT\"ONE\";A", "RUN", "2,3"] {
        command(&mut machine, text)?;
    }
    require(&machine, "ONE 2")?;
    if !screen(&machine).contains("?EXTRA IGNORED") {
        return Err("Excess INPUT was not diagnosed".into());
    }
    for text in ["NEW", "10 GET A$,B:PRINT\"EMPTY\";LEN(A$);B", "RUN"] {
        command(&mut machine, text)?;
    }
    require(&machine, "EMPTY 0 0")?;
    for text in [
        "NEW",
        "10 GET A$:IF A$=\"\"THEN10",
        "20 PRINT\"KEY\";A$",
        "RUN",
    ] {
        command(&mut machine, text)?;
    }
    type_text(&mut machine, "Q")?;
    advance(&mut machine, CLOCK_HZ * 2)?;
    require(&machine, "KEYQ")?;
    command(&mut machine, "INPUT A")?;
    require(&machine, "?ILLEGAL DIRECT ERROR")?;
    for text in ["NEW", "10 INPUT A", "RUN"] {
        command(&mut machine, text)?;
    }
    let registers = machine.registers();
    let page = machine.bus().original_page();
    machine.stop();
    if machine.advance(1, || false).is_ok() {
        return Err("Stopped INPUT advanced".into());
    }
    machine.resume()?;
    if machine.advance(100, || true)?.cycles != 0
        || machine.registers() != registers
        || machine.bus().original_page() != page
    {
        return Err("INPUT cancellation changed guest context".into());
    }
    machine.resume()?;
    machine.bus_mut().key(Key::new(7, 7)?, true);
    advance(&mut machine, CLOCK_HZ)?;
    machine.bus_mut().release_inputs();
    advance(&mut machine, CLOCK_HZ)?;
    if screen(&machine)
        .lines()
        .map(str::trim_end)
        .rfind(|row| !row.is_empty())
        != Some("?")
    {
        return Err("RUN/STOP unexpectedly completed INPUT".into());
    }
    machine.reset()?;
    advance(&mut machine, CLOCK_HZ * 3)?;
    command(&mut machine, "PRINT\"RECOVER\";6*7")?;
    require(&machine, "RECOVER 42")?;
    Ok(format!(
        "{{\"schemaVersion\":1,\"keyboardOnly\":true,\"numericAndStringInput\":true,\"retryAndMissingExtraFields\":true,\"emptyAndPhysicalGet\":true,\"inputStopCancelResumeReset\":true,\"lineInputIgnoresRunStop\":true,\"errorRecovery\":true,\"traceSha256\":\"{}\",\"basicV2Complete\":false}}",
        digest(b"IN 12.5 HELLO\0IN-2.5 OK\0SUM 5\0ONE 2\0EMPTY 0 0\0KEYQ\0RECOVER 42")
    ))
}
