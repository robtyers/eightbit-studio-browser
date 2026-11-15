//! Original guest function/memory programmes. The same keyboard corpus observes
//! the source-bound original firmware and the exact-hash supplied ROM separately.
use super::{advance, digest, screen, type_text};
use eightbit_commodore_machine::{Firmware, pal::Machine, sid::CLOCK_HZ};
fn command(machine: &mut Machine<'_>, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    type_text(machine, text)?;
    type_text(machine, "\r")?;
    advance(machine, CLOCK_HZ)?;
    if !text.starts_with("WAIT") {
        for _ in 0..20 {
            if screen(machine)
                .lines()
                .map(str::trim_end)
                .rfind(|row| !row.is_empty())
                == Some("READY.")
            {
                break;
            }
            advance(machine, CLOCK_HZ)?;
        }
    }
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
        Err(format!("Memory corpus expected {expected}\n{display}").into())
    }
}
pub fn qualify(firmware: Firmware<'_>) -> Result<String, Box<dyn std::error::Error>> {
    let mut machine = Machine::new(firmware);
    machine.reset()?;
    advance(&mut machine, CLOCK_HZ * 3)?;
    let mut trace = Vec::new();
    let cases = [
        ("-2^2", "-4"),
        ("(-2)^2", "4"),
        ("2^3^2", "64"),
        ("2^-2", ".25"),
        ("0^0", "1"),
        ("(-2)^3", "-8"),
        ("ABS(LOG(2)-.693147181)<1E-7", "-1"),
        ("ABS(LOG(10)-2.30258509)<1E-7", "-1"),
        ("ABS(EXP(1)-2.71828183)<1E-7", "-1"),
        ("ABS(EXP(-1)-.367879441)<1E-7", "-1"),
        ("SIN(0)", "0"),
        ("COS(0)", "1"),
        ("ATN(0)", "0"),
        ("ABS(SIN(1)-.841470985)<1E-7", "-1"),
        ("ABS(COS(1)-.540302306)<1E-7", "-1"),
        ("ABS(TAN(1)-1.55740772)<1E-7", "-1"),
        ("ABS(ATN(1)-.785398163)<1E-7", "-1"),
        ("ABS(ATN(10)-1.47112767)<1E-7", "-1"),
        ("SQR(4)", "2"),
        ("SQR(2)", "1.41421356"),
        ("SQR(0)", "0"),
        ("SGN(-.1)", "-1"),
        ("SGN(0)", "0"),
        ("SGN(1E30)", "1"),
        ("ABS(-12.75)", "12.75"),
        ("ABS(0)", "0"),
        ("INT(.9)", "0"),
        ("INT(-.1)", "-1"),
        ("INT(-1.1)", "-2"),
        ("INT(-32768.9)", "-32769"),
        ("INT(65535.9)", "65535"),
        ("INT(1E20)", "1E+20"),
        ("INT(-1E20)", "-1E+20"),
        ("ABS(INT(-2.3))+SGN(-8)", "2"),
        ("PEEK(49152)", "0"),
        ("PEEK(49152.9)", "0"),
    ];
    for (expression, value) in cases {
        command(&mut machine, &format!("PRINT\"R\";{expression};\"Q\""))?;
        let expected = format!(
            "R{}{value} Q",
            if value.starts_with('-') { "" } else { " " }
        );
        require(&machine, &expected)?;
        trace.extend_from_slice(expected.as_bytes());
        trace.push(0);
    }
    for text in ["POKE49152,171.9", "PRINT\"MEM\";PEEK(49152)"] {
        command(&mut machine, text)?;
    }
    require(&machine, "MEM 171")?;
    for text in ["POKE49153,0:WAIT49153,128,128", "PRINT\"WAIT\";7*7"] {
        command(&mut machine, text)?;
    }
    require(&machine, "WAIT 49")?;
    // Original subroutine: increment A/X/Y, set carry, return. SYS parameters
    // and returned registers are exercised through public guest memory only.
    for (index, byte) in [0x69, 1, 0xe8, 0xc8, 0x38, 0x60].into_iter().enumerate() {
        command(&mut machine, &format!("POKE{},{}", 49152 + index, byte))?;
    }
    for text in [
        "POKE780,40:POKE781,9:POKE782,19:POKE783,32",
        "SYS49152",
        "PRINT\"SYS\";PEEK(780);PEEK(781);PEEK(782)",
    ] {
        command(&mut machine, text)?;
    }
    require(&machine, "SYS 41 10 20")?;
    command(&mut machine, "PRINT\"FLAGS\";PEEK(783)AND1")?;
    require(&machine, "FLAGS 1")?;
    // Original USR body flips FAC's separate sign byte, preserving magnitude.
    for (index, byte) in [0xa5, 0x66, 0x49, 0xff, 0x85, 0x66, 0x60]
        .into_iter()
        .enumerate()
    {
        command(&mut machine, &format!("POKE{},{}", 49200 + index, byte))?;
    }
    command(&mut machine, "POKE785,48:POKE786,192")?;
    command(&mut machine, "PRINT\"USR\";USR(1.5);USR(-2.25)")?;
    require(&machine, "USR-1.5 2.25")?;
    // Programme RAM under BASIC is still physical RAM. Hide BASIC, inspect and
    // restore it inside guest machine code before returning to the interpreter.
    let banking = [
        0xa9, 0x36, 0x85, 1, 0xad, 0, 0xa0, 0x8d, 0x80, 0xc0, 0xa9, 0x37, 0x85, 1, 0x60,
    ];
    for (index, byte) in banking.into_iter().enumerate() {
        command(&mut machine, &format!("POKE{},{}", 49232 + index, byte))?;
    }
    for text in ["POKE40960,87:SYS49232", "PRINT\"BANK\";PEEK(49280)"] {
        command(&mut machine, text)?;
    }
    require(&machine, "BANK 87")?;
    for text in [
        "PRINT PEEK(-1)",
        "PRINT PEEK(65536)",
        "POKE49152,256",
        "POKE49152,-1",
        "SYS65536",
        "PRINT SQR(-1)",
        "PRINT LOG(0)",
        "PRINT LOG(-1)",
        "PRINT (-1)^.5",
    ] {
        command(&mut machine, text)?;
        require(&machine, "?ILLEGAL QUANTITY ERROR")?;
        trace.extend_from_slice(text.as_bytes());
        trace.push(0);
    }
    for text in [
        "A=RND(-7):B=RND(1):C=RND(-7)",
        "PRINT\"RANDOM\";A=C;A<>B;A>=0;A<1",
    ] {
        command(&mut machine, text)?;
    }
    require(&machine, "RANDOM-1 -1 -1 -1")?;
    for text in ["PRINT ABS(1", "POKE49152", "WAIT49152,1,2,3"] {
        command(&mut machine, text)?;
        require(&machine, "?SYNTAX ERROR")?;
    }
    command(&mut machine, "PRINT\"RECOVER\";6*7")?;
    require(&machine, "RECOVER 42")?;
    for text in ["POKE49153,0", "WAIT49153,1"] {
        command(&mut machine, text)?;
    }
    let registers = machine.registers();
    let page = machine.bus().original_page();
    let stack: Vec<_> = (0x100..0x200).map(|at| machine.bus().ram(at)).collect();
    machine.stop();
    if machine.advance(1, || false).is_ok() {
        return Err("Stopped WAIT advanced".into());
    }
    machine.resume()?;
    if machine.advance(100, || true)?.cycles != 0
        || machine.registers() != registers
        || machine.bus().original_page() != page
        || !(0x100..0x200).map(|at| machine.bus().ram(at)).eq(stack)
    {
        return Err("WAIT cancellation changed guest context".into());
    }
    machine.resume()?;
    machine.reset()?;
    advance(&mut machine, CLOCK_HZ * 3)?;
    command(&mut machine, "PRINT\"RESET\";6*7")?;
    require(&machine, "RESET 42")?;
    command(&mut machine, "PRINT USR(1)")?;
    require(&machine, "?ILLEGAL QUANTITY ERROR")?;
    Ok(format!(
        "{{\"schemaVersion\":1,\"keyboardOnly\":true,\"numericFunctionCases\":{},\"guestMemoryAndBanking\":true,\"sysRegistersAndFlags\":true,\"usrFac\":true,\"waitStopCancelReset\":true,\"errorsAndRecovery\":true,\"traceSha256\":\"{}\",\"basicV2Complete\":false}}",
        cases.len(),
        digest(&trace)
    ))
}
