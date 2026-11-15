//! Original DEF/FN programmes, including formal-value restoration and failures.
use super::{advance, digest, screen, type_text};
use eightbit_commodore_machine::{Firmware, pal::Machine, sid::CLOCK_HZ};
fn command(machine: &mut Machine<'_>, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    type_text(machine, text)?;
    type_text(machine, "\r")?;
    advance(machine, CLOCK_HZ * 5)?;
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
        Err(format!("FN corpus expected {expected}\n{display}").into())
    }
}
pub fn qualify(firmware: Firmware<'_>) -> Result<String, Box<dyn std::error::Error>> {
    let mut machine = Machine::new(firmware);
    machine.reset()?;
    advance(&mut machine, CLOCK_HZ * 3)?;
    let cases: &[(&[&str], &str)] = &[
        (
            &[
                "10 X=7:DEF FN F(X)=X*2+1",
                "20 PRINT\"FN\";FN F(3);X;FN F(4)",
                "RUN",
            ],
            "FN 7 7 9",
        ),
        (
            &[
                "NEW",
                "10 X=7:Y=5:DEF FN F(X)=X*2+1",
                "20 DEF FN G(Y)=FN F(Y)+FN F(Y+1)",
                "30 PRINT\"NESTFN\";FN G(2);X;Y",
                "RUN",
            ],
            "NESTFN 12 7 5",
        ),
        (
            &[
                "NEW",
                "10 A=10:DEF FN F(X)=X+A",
                "20 A=20:PRINT\"CAPTURE\";FN F(3)",
                "RUN",
            ],
            "CAPTURE 23",
        ),
        (&["PRINT FN Q(1)"], "?UNDEF'D FUNCTION ERROR"),
        (&["DEF FN F(X)=X"], "?ILLEGAL DIRECT ERROR"),
        (
            &["NEW", "10 DEF FN F(X)=1/0", "20 PRINT FN F(2)", "RUN"],
            "?DIVISION BY ZERO ERROR IN 20",
        ),
    ];
    let mut trace = Vec::new();
    for (index, (commands, expected)) in cases.iter().enumerate() {
        for text in *commands {
            command(&mut machine, text)?;
        }
        require(&machine, expected)?;
        if index == 0 {
            let word = |at| u16::from_le_bytes([machine.bus().ram(at), machine.bus().ram(at + 1)]);
            let variables = word(0x2d);
            let record = variables + 7;
            let bytes: Vec<_> = (record..record + 7)
                .map(|at| machine.bus().ram(at))
                .collect();
            if bytes[..2] != [0xc6, 0]
                || word(record + 4) != variables + 2
                || machine.bus().ram(word(record + 2)) != b'X'
            {
                return Err(format!(
                    "FN public record mismatch: {bytes:02x?}, variables={variables:04x}"
                )
                .into());
            }
        }
        trace.extend_from_slice(expected.as_bytes());
        trace.push(0);
    }
    command(&mut machine, "PRINT\"RECOVER\";6*7")?;
    require(&machine, "RECOVER 42")?;
    Ok(format!(
        "{{\"schemaVersion\":1,\"keyboardOnly\":true,\"programmeCases\":{},\"publicFunctionRecord\":true,\"formalRestoration\":true,\"nestedFunctions\":true,\"currentVariableCapture\":true,\"errorsAndRecovery\":true,\"traceSha256\":\"{}\",\"basicV2Complete\":false}}",
        cases.len(),
        digest(&trace)
    ))
}
