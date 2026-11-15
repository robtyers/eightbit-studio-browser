//! Original conditional/subroutine programmes, shared with exact-hash ROM observations.
use super::{advance, digest, screen, type_text};
use eightbit_commodore_machine::{Firmware, pal::Machine, sid::CLOCK_HZ};
fn command(machine: &mut Machine<'_>, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    type_text(machine, text)?;
    type_text(machine, "\r")?;
    advance(machine, CLOCK_HZ)?;
    Ok(())
}
fn require_line(machine: &Machine<'_>, expected: &str) -> Result<(), Box<dyn std::error::Error>> {
    if screen(machine)
        .lines()
        .map(|line| {
            if expected.starts_with('?') {
                line.split_whitespace().collect::<Vec<_>>().join(" ")
            } else {
                line.trim_end().to_owned()
            }
        })
        .filter(|row| !row.is_empty())
        .rev()
        .nth(1)
        .as_deref()
        == Some(expected)
    {
        Ok(())
    } else {
        Err(format!("Control corpus expected {expected}\n{}", screen(machine)).into())
    }
}
pub fn qualify(firmware: Firmware<'_>) -> Result<String, Box<dyn std::error::Error>> {
    let mut machine = Machine::new(firmware);
    machine.reset()?;
    advance(&mut machine, CLOCK_HZ * 3)?;
    let cases = [
        ("1=1", "-1"),
        ("1=2", "0"),
        ("1<2", "-1"),
        ("-2<-1", "-1"),
        ("-1<0", "-1"),
        ("0>-1", "-1"),
        ("0<=0", "-1"),
        ("2>=3", "0"),
        ("2<>3", "-1"),
        ("1<2<3", "-1"),
        ("NOT1=0", "-1"),
        ("1OR2AND4", "1"),
        ("1+2>2AND4<5", "-1"),
        ("NOT.9", "-1"),
        ("NOT-.1", "0"),
        ("1.9AND3", "1"),
        ("-1.9AND255", "254"),
        ("NOT32767", "-32768"),
        ("NOT-32768", "32767"),
        ("32767OR-32768", "-1"),
        ("1+NOT0", "0"),
        ("1+NOT0=0", "1"),
        ("NOT1+NOT0", "-1"),
        ("1>NOT0", "-1"),
        ("2=>1", "-1"),
        ("2=<1", "0"),
        ("2><1", "-1"),
    ];
    let mut trace = Vec::new();
    for (expression, value) in cases {
        command(&mut machine, &format!("PRINT\"R\";{expression};\"Q\""))?;
        let expected = format!(
            "R{}{value} Q",
            if value.starts_with('-') { "" } else { " " }
        );
        require_line(&machine, &expected)?;
        trace.extend_from_slice(expected.as_bytes());
        trace.push(0);
    }
    for text in [
        "NEW",
        "10 A=0",
        "20 IF 0 THEN A=99:A=98",
        "30 IF .1 THEN A=A+1",
        "40 IF A=1 THEN 60",
        "50 A=97",
        "60 IF A<>1 GOTO 90",
        "70 IF NOT0 THEN IF A=1 THEN A=A+2",
        "80 PRINT\"COND\";A:END",
        "90 PRINT\"BAD\":END",
        "RUN",
    ] {
        command(&mut machine, text)?;
    }
    require_line(&machine, "COND 3")?;
    trace.extend_from_slice(b"COND 3\0");
    for text in [
        "NEW",
        "10 A=1:GOSUB100:A=A+10",
        "20 PRINT\"SUB\";A:END",
        "100 A=A+2:GOSUB200:A=A+4:RETURN",
        "200 A=A+3:RETURN",
        "RUN",
    ] {
        command(&mut machine, text)?;
    }
    require_line(&machine, "SUB 20")?;
    trace.extend_from_slice(b"SUB 20\0");
    command(&mut machine, "GOSUB100:PRINT\"DIRECT\";A")?;
    require_line(&machine, "DIRECT 29")?;
    trace.extend_from_slice(b"DIRECT 29\0");
    // A false condition skips the whole physical line, including colon tails.
    command(&mut machine, "A=7:IF0THEN A=8:PRINT\"BAD\"")?;
    command(&mut machine, "PRINT\"SKIP\";A")?;
    require_line(&machine, "SKIP 7")?;
    trace.extend_from_slice(b"SKIP 7\0");
    for (text, expected) in [
        ("PRINT 0AND(1/0)", "?DIVISION BY ZERO ERROR"),
        ("PRINT 1OR(1/0)", "?DIVISION BY ZERO ERROR"),
        ("PRINT NOT32768", "?ILLEGAL QUANTITY ERROR"),
        ("RETURN", "?RETURN WITHOUT GOSUB ERROR"),
        ("GOSUB63998", "?UNDEF'D STATEMENT ERROR"),
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
            return Err(format!("Control error {text}: expected {expected}\n{display}").into());
        }
        trace.extend_from_slice(expected.as_bytes());
        trace.push(0);
    }
    command(&mut machine, "PRINT\"RECOVERY\";6*7")?;
    require_line(&machine, "RECOVERY 42")?;
    for text in [
        "NEW",
        "10 S=0:FOR I=1 TO2:GOSUB100:NEXT",
        "20 PRINT\"RESUME\";S;I:END",
        "100 S=S+1:IF S=1THEN STOP",
        "110 RETURN",
        "RUN",
    ] {
        command(&mut machine, text)?;
    }
    require_line(&machine, "BREAK IN 100")?;
    command(&mut machine, "PRINT\"SUSPENDED\";S")?;
    require_line(&machine, "SUSPENDED 1")?;
    command(&mut machine, "CONT")?;
    require_line(&machine, "RESUME 2  3")?;
    for text in ["NEW", "10 STOP", "RUN", "10 END", "CONT"] {
        command(&mut machine, text)?;
    }
    require_line(&machine, "?CAN'T CONTINUE ERROR")?;
    trace.extend_from_slice(b"BREAK IN 100\0SUSPENDED 1\0RESUME 2  3\0?CAN'T CONTINUE ERROR\0");
    for (programme, expected) in [
        (
            ["10 GOSUB100:END", "100 CLR:RETURN", "200 END"],
            "?RETURN WITHOUT GOSUB ERROR IN 100",
        ),
        (
            ["10 GOSUB100:END", "100 RUN200", "200 RETURN"],
            "?RETURN WITHOUT GOSUB ERROR IN 200",
        ),
    ] {
        command(&mut machine, "NEW")?;
        for text in programme {
            command(&mut machine, text)?;
        }
        command(&mut machine, "RUN")?;
        let display = screen(&machine);
        let actual = display
            .lines()
            .map(str::trim_end)
            .filter(|row| !row.is_empty())
            .rev()
            .nth(1)
            .map(|row| row.split_whitespace().collect::<Vec<_>>().join(" "));
        if actual.as_deref() != Some(expected) {
            return Err(format!(
                "CLR/RUN did not invalidate GOSUB context: expected {expected}\n{display}"
            )
            .into());
        }
        trace.extend_from_slice(expected.as_bytes());
        trace.push(0);
    }
    Ok(format!(
        "{{\"schemaVersion\":1,\"keyboardOnly\":true,\"comparisonLogicalCases\":{},\"conditionalLineSkip\":true,\"nestedSubroutineReturn\":true,\"directSubroutineReturn\":true,\"eagerLogicalEvaluation\":true,\"errorRecovery\":true,\"clearRunInvalidateSubroutines\":true,\"stopContinueNestedFrames\":true,\"editInvalidatesContinue\":true,\"traceSha256\":\"{}\",\"basicV2Complete\":false}}",
        cases.len(),
        digest(&trace)
    ))
}
