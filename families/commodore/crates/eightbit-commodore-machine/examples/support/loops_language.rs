//! Original FOR/NEXT programmes, shared with separately supplied-ROM observations.
use super::{advance, digest, screen, type_text};
use eightbit_commodore_machine::{Firmware, pal::Machine, sid::CLOCK_HZ};
fn command(machine: &mut Machine<'_>, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    type_text(machine, text)?;
    type_text(machine, "\r")?;
    advance(machine, CLOCK_HZ)?;
    if text == "RUN" || text.starts_with("FOR ") {
        advance(machine, CLOCK_HZ * 12)?;
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
        Err(format!("Loop corpus expected {expected}\n{display}").into())
    }
}
pub fn qualify(firmware: Firmware<'_>) -> Result<String, Box<dyn std::error::Error>> {
    let mut machine = Machine::new(firmware);
    machine.reset()?;
    advance(&mut machine, CLOCK_HZ * 3)?;
    let mut trace = Vec::new();
    let cases: &[(&[&str], &str)] = &[
        (
            &[
                "10 S=0:ON 2.9 GOSUB100,200,300:S=S+1",
                "20 PRINT\"ONCALL\";S:END",
                "100 S=10:RETURN",
                "200 S=20:RETURN",
                "300 S=30:RETURN",
                "RUN",
            ],
            "ONCALL 21",
        ),
        (
            &[
                "10 ON0GOTO100,200:S=1:ON4GOSUB100,200",
                "20 ON2GOTO100,200:END",
                "100 PRINT\"BAD\":END",
                "200 PRINT\"ONGOTO\";S:END",
                "RUN",
            ],
            "ONGOTO 1",
        ),
        (
            &[
                "10 DATA 2,-3.5,1E2",
                "20 READ A,B,C:PRINT\"DATA\";A;B;C",
                "RUN",
            ],
            "DATA 2 -3.5 100",
        ),
        (
            &[
                "10 REM DATA 88",
                "20 PRINT\"DATA 99\";:DATA 1:DATA 2",
                "30 READ A:READ B:RESTORE:READ C",
                "40 PRINT\"RESTORE\";A;B;C",
                "RUN",
            ],
            "DATA 99RESTORE 1 2 1",
        ),
        (
            &["0 DATA 3,4", "10 READ A,B:PRINT\"ZERO\";A;B", "RUN"],
            "ZERO 3 4",
        ),
        (
            &["10 DATA ,2,", "20 READ A,B,C:PRINT\"EMPTY\";A;B;C", "RUN"],
            "EMPTY 0 2 0",
        ),
        (
            &["10 DATA 1", "20 READ A,B", "RUN"],
            "?OUT OF DATA ERROR IN 20",
        ),
        (&["10 DATA XX", "20 READ A", "RUN"], "?SYNTAX ERROR IN 10"),
        (
            &[
                "10 S=0:FOR I=1 TO 4:S=S+I:NEXT",
                "20 PRINT\"SUM\";S;I",
                "RUN",
            ],
            "SUM 10 5",
        ),
        (
            &[
                "10 S=0:FOR I=2 TO -1 STEP -.5:S=S+I:NEXT I",
                "20 PRINT\"HALF\";S;I",
                "RUN",
            ],
            "HALF 3.5 -1.5",
        ),
        (
            &[
                "10 S=0:FOR I=5 TO 1:S=S+1:NEXT",
                "20 PRINT\"ONCE\";S;I",
                "RUN",
            ],
            "ONCE 1 6",
        ),
        (
            &[
                "10 S=0:FOR I=1 TO 2:FOR J=1 TO 3:S=S+1:NEXT J,I",
                "20 PRINT\"NEST\";S;I;J",
                "RUN",
            ],
            "NEST 6 3 4",
        ),
        (
            &[
                "10 S=0:FOR I=1 TO 2",
                "20 FOR J=1 TO 5:S=S+1:NEXT I",
                "30 PRINT\"OUTER\";S;I;J",
                "RUN",
            ],
            "OUTER 2 3 1",
        ),
        (
            &[
                "10 FOR I=1 TO 2:GOSUB100:NEXT",
                "20 PRINT\"CALL\";I;J:END",
                "100 FOR J=1 TO 3:RETURN",
                "RUN",
            ],
            "CALL 3 1",
        ),
        (
            &[
                "10 S=0:FOR I=1 TO 10",
                "20 IF I=3 THEN FOR I=9 TO 9",
                "30 S=S+I:NEXT",
                "40 PRINT\"REPLACE\";S;I",
                "RUN",
            ],
            "REPLACE 12 10",
        ),
        (
            &["10 FOR I=1 TO 2:GOSUB100:NEXT:END", "100 NEXT I", "RUN"],
            "?NEXT WITHOUT FOR ERROR IN 100",
        ),
        (&["10 FOR I%=1 TO 2:NEXT", "RUN"], "?SYNTAX ERROR IN 10"),
        (
            &["10 FOR I=1 TO 2:CLR:NEXT", "RUN"],
            "?NEXT WITHOUT FOR ERROR IN 10",
        ),
    ];
    for (programme, expected) in cases {
        command(&mut machine, "NEW")?;
        for text in *programme {
            command(&mut machine, text)?;
        }
        require(&machine, expected)?;
        trace.extend_from_slice(expected.as_bytes());
        trace.push(0);
    }
    command(&mut machine, "FOR I=1 TO 2:NEXT:PRINT\"DIRECT\";I")?;
    require(&machine, "DIRECT 3")?;
    command(&mut machine, "NEXT")?;
    require(&machine, "?NEXT WITHOUT FOR ERROR")?;
    command(&mut machine, "PRINT\"RECOVER\";7*7")?;
    require(&machine, "RECOVER 49")?;
    Ok(format!(
        "{{\"schemaVersion\":1,\"keyboardOnly\":true,\"programmeCases\":{},\"directLoops\":true,\"nestedNamedAndCommaNext\":true,\"returnUnwindsLoops\":true,\"reenteredCounter\":true,\"errorsAndRecovery\":true,\"traceSha256\":\"{}\",\"basicV2Complete\":false}}",
        cases.len(),
        digest(&trace)
    ))
}
