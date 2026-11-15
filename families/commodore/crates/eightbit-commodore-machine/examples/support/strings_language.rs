//! Independently authored strings/collection programmes, shared with reference observations.
use super::{advance, digest, screen, type_text};
use eightbit_commodore_machine::{Firmware, pal::Machine, sid::CLOCK_HZ};
fn command(machine: &mut Machine<'_>, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    type_text(machine, text)?;
    type_text(machine, "\r")?;
    advance(machine, CLOCK_HZ)?;
    if text == "RUN" {
        advance(machine, CLOCK_HZ * 30)?;
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
        Err(format!("String corpus expected {expected}\n{display}").into())
    }
}
pub fn qualify(firmware: Firmware<'_>) -> Result<String, Box<dyn std::error::Error>> {
    let mut machine = Machine::new(firmware);
    machine.reset()?;
    advance(&mut machine, CLOCK_HZ * 3)?;
    let mut trace = Vec::new();
    let cases = [
        ("\"A\"+\"BC\"", "RABCQ"),
        ("LEN(\"\")", "R 0 Q"),
        ("LEN(\"ABC\")", "R 3 Q"),
        ("ASC(\"A\")", "R 65 Q"),
        ("ASC(CHR$(255))", "R 255 Q"),
        ("CHR$(65)", "RAQ"),
        ("STR$(1.25)", "R 1.25Q"),
        ("STR$(-1.25)", "R-1.25Q"),
        ("STR$(12)+STR$(34)", "R 12 34Q"),
        ("VAL(\" -1.25\")", "R-1.25 Q"),
        ("VAL(\"12X\")", "R 12 Q"),
        ("VAL(\"\")", "R 0 Q"),
        ("VAL(\"Z\")", "R 0 Q"),
        ("LEFT$(\"ABCDE\",2)", "RABQ"),
        ("RIGHT$(\"ABCDE\",2)", "RDEQ"),
        ("LEFT$(\"ABC\",9)", "RABCQ"),
        ("RIGHT$(\"ABC\",9)", "RABCQ"),
        ("MID$(\"ABCDE\",2,3)", "RBCDQ"),
        ("MID$(\"ABCDE\",3)", "RCDEQ"),
        ("MID$(\"ABCDE\",8)", "RQ"),
        ("MID$(\"ABCDE\",2,0)", "RQ"),
        ("\"A\"<\"B\"", "R-1 Q"),
        ("\"AB\"<\"ABC\"", "R-1 Q"),
        ("\"B\">\"A\"", "R-1 Q"),
        ("\"\"=\"\"", "R-1 Q"),
        ("\"A\"<>\"A\"", "R 0 Q"),
    ];
    for (expression, expected) in cases {
        command(&mut machine, &format!("PRINT\"R\";{expression};\"Q\""))?;
        require(&machine, expected)?;
        trace.extend_from_slice(expected.as_bytes());
        trace.push(0);
    }
    for text in ["A$=\"ALPHA\":B$=A$", "A$=\"BETA\"", "PRINT\"COPY\";A$;B$"] {
        command(&mut machine, text)?;
    }
    require(&machine, "COPYBETAALPHA")?;
    for text in [
        "PRINT 1+\"A\"",
        "PRINT \"A\"-\"B\"",
        "A$=1",
        "A=\"A\"",
        "PRINT ABS(\"A\")",
    ] {
        command(&mut machine, text)?;
        require(&machine, "?TYPE MISMATCH ERROR")?;
    }
    for text in ["PRINT ASC(\"\")", "PRINT CHR$(256)", "PRINT MID$(\"A\",0)"] {
        command(&mut machine, text)?;
        require(&machine, "?ILLEGAL QUANTITY ERROR")?;
    }
    for (text, expected) in [
        ("TI$=\"123456\":PRINT\"TIME\";TI$", "TIME123456"),
        ("PRINT\"CLOCK\";TI>0;TI<5184000", "CLOCK-1 -1"),
        ("PRINT\"STATUS\";ST=PEEK(144)", "STATUS-1"),
    ] {
        command(&mut machine, text)?;
        require(&machine, expected)?;
    }
    for text in ["TI$=\"00000\"", "TI$=\"1234567\""] {
        command(&mut machine, text)?;
        require(&machine, "?ILLEGAL QUANTITY ERROR")?;
    }
    for text in [
        "NEW",
        "10 POKE55,0:POKE56,16:CLR",
        "20 A$=\"ABCDEFGHIJKLMNOPQRSTUVWXYZ\"",
        "30 A$=A$+A$:B$=A$",
        "40 FOR I=1 TO 20:C$=A$+B$:NEXT",
        "50 PRINT\"GC\";LEN(A$);LEN(B$);LEN(C$);A$=B$",
        "RUN",
    ] {
        command(&mut machine, text)?;
    }
    require(&machine, "GC 52 52 104 -1")?;
    command(&mut machine, "PRINT\"KEEP\";LEFT$(C$,5);RIGHT$(B$,5)")?;
    require(&machine, "KEEPABCDEVWXYZ")?;
    command(&mut machine, "PRINT\"RECOVER\";6*7")?;
    require(&machine, "RECOVER 42")?;
    Ok(format!(
        "{{\"schemaVersion\":1,\"keyboardOnly\":true,\"expressionCases\":{},\"immutableScalarCopies\":true,\"movingCollection\":true,\"temporaryRoots\":true,\"typeQuantityErrorsAndRecovery\":true,\"traceSha256\":\"{}\",\"basicV2Complete\":false}}",
        cases.len(),
        digest(&trace)
    ))
}
