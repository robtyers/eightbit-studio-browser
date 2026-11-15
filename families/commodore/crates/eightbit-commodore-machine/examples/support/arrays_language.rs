//! Original array programmes and public layout checks, shared with reference observations.
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
        Err(format!("Array corpus expected {expected}\n{display}").into())
    }
}
fn word(machine: &Machine<'_>, at: u16) -> u16 {
    u16::from_le_bytes([machine.bus().ram(at), machine.bus().ram(at + 1)])
}
pub fn qualify(firmware: Firmware<'_>) -> Result<String, Box<dyn std::error::Error>> {
    let mut machine = Machine::new(firmware);
    machine.reset()?;
    advance(&mut machine, CLOCK_HZ * 3)?;
    for text in [
        "DIM A(1,2),N%(1),S$(1)",
        "A(1,0)=3:A(0,1)=5",
        "N%(1)=-123:S$(1)=\"XYZ\"",
        "B=9",
    ] {
        command(&mut machine, text)?;
    }
    let at = word(&machine, 0x2f);
    let end = word(&machine, 0x31);
    let expected = [
        0x41, 0, 39, 0, 2, 0, 3, 0, 2, 0, 0, 0, 0, 0, 0x82, 0x40, 0, 0, 0, 0x83, 0x20, 0, 0, 0,
    ];
    if end.checked_sub(at) != Some(63)
        || !(at..at + 24).map(|a| machine.bus().ram(a)).eq(expected)
        || !(at + 39..at + 50)
            .map(|a| machine.bus().ram(a))
            .eq([0xce, 0x80, 11, 0, 1, 0, 2, 0, 0, 0xff, 0x85])
        || !(at + 50..at + 61)
            .map(|a| machine.bus().ram(a))
            .eq([0x53, 0x80, 13, 0, 1, 0, 2, 0, 0, 0, 3])
    {
        return Err(format!(
            "Public array header/element layout mismatch\n{}",
            screen(&machine)
        )
        .into());
    }
    let pointer = word(&machine, at + 61);
    if !(pointer..pointer + 3)
        .map(|a| machine.bus().ram(a))
        .eq(*b"XYZ")
    {
        return Err("Array string descriptor mismatch".into());
    }
    let cases: &[(&[&str], &str)] = &[
        (
            &["PRINT\"VALUES\";A(1,0);A(0,1);N%(1);S$(1)"],
            "VALUES 3 5 -123 XYZ",
        ),
        (&["NEW", "A(1)=Z+7", "PRINT\"MOVE\";A(1);Z"], "MOVE 7 0"),
        (
            &[
                "NEW",
                "DIM X(1,2,3)",
                "X(1,2,3)=9:X(0,2,3)=7",
                "PRINT\"THREE\";X(1,2,3);X(0,2,3)",
            ],
            "THREE 9 7",
        ),
        (
            &["NEW", "A(10)=11", "PRINT\"DEFAULT\";A(0);A(10)"],
            "DEFAULT 0 11",
        ),
        (&["PRINT A(11)"], "?BAD SUBSCRIPT ERROR"),
        (&["DIM A(2)"], "?REDIM'D ARRAY ERROR"),
        (&["PRINT A(1,1)"], "?BAD SUBSCRIPT ERROR"),
        (
            &[
                "NEW",
                "10 DATA 2,-3,ABC,\"D,E:F\"",
                "20 DIM N%(1),S$(1)",
                "30 READ N%(0),N%(1),S$(0),S$(1)",
                "40 PRINT\"READ\";N%(0);N%(1);S$(0);S$(1)",
                "RUN",
            ],
            "READ 2 -3 ABCD,E:F",
        ),
        (
            &[
                "NEW",
                "10 POKE55,0:POKE56,16:CLR",
                "20 DIM S$(1,1):S$(1,1)=\"ROOT\"",
                "30 FOR I=1 TO 25:A$=\"ABCDEFGHIJKLMNOP\"",
                "40 A$=A$+A$:A$=A$+A$:NEXT",
                "50 PRINT\"ARRAYGC\";S$(1,1);I;LEN(A$)",
                "RUN",
            ],
            "ARRAYGCROOT 26 64",
        ),
    ];
    let mut trace = Vec::new();
    for (commands, expected) in cases {
        for text in *commands {
            command(&mut machine, text)?;
        }
        require(&machine, expected)?;
        trace.extend_from_slice(expected.as_bytes());
        trace.push(0);
    }
    command(&mut machine, "PRINT\"RECOVER\";6*7")?;
    require(&machine, "RECOVER 42")?;
    Ok(format!(
        "{{\"schemaVersion\":1,\"keyboardOnly\":true,\"publicPackedLayout\":true,\"programmeCases\":{},\"floatIntegerStringArrays\":true,\"multidimensional\":true,\"implicitDimensions\":true,\"scalarInsertion\":true,\"arrayStringCollectionRoots\":true,\"errorsAndRecovery\":true,\"traceSha256\":\"{}\",\"basicV2Complete\":false}}",
        cases.len(),
        digest(&trace)
    ))
}
