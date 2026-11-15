//! Original physical editor, LIST-range, abbreviation and screen-control fixtures.
use super::{advance, digest, screen, type_bytes, type_text};
use eightbit_commodore_machine::{Firmware, pal::Machine, sid::CLOCK_HZ};
use eightbit_mos6502_core::Bus;
fn command(m: &mut Machine<'_>, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    type_text(m, text)?;
    type_text(m, "\r")?;
    advance(m, CLOCK_HZ * 2)?;
    Ok(())
}
fn require(m: &Machine<'_>, expected: &str) -> Result<(), Box<dyn std::error::Error>> {
    let s = screen(m);
    let actual = s
        .lines()
        .map(str::trim_end)
        .filter(|row| !row.is_empty())
        .rev()
        .nth(1);
    if actual == Some(expected) {
        Ok(())
    } else {
        Err(format!("Editor expected {expected}\n{s}").into())
    }
}
pub fn qualify(firmware: Firmware<'_>) -> Result<String, Box<dyn std::error::Error>> {
    let mut m = Machine::new(firmware);
    m.reset()?;
    advance(&mut m, CLOCK_HZ * 3)?;
    for input in [
        b"PRINT\"AX\"\x9d\x9dB\r".as_slice(),
        b"PRINT\"ABXC\"\x9d\x9d\x14\r",
        b"PRINT\"AC\"\x9d\x9d\x94B\r",
    ] {
        type_bytes(&mut m, input)?;
        advance(&mut m, CLOCK_HZ * 2)?;
        require(
            &m,
            if input == b"PRINT\"AX\"\x9d\x9dB\r" {
                "AB"
            } else {
                "ABC"
            },
        )?;
    }
    command(&mut m, "10 PRINT\"OLD\"")?;
    let mut edit = vec![145];
    edit.extend([29; 9]);
    edit.extend_from_slice(b"NEW\r");
    type_bytes(&mut m, &edit)?;
    advance(&mut m, CLOCK_HZ)?;
    command(&mut m, "RUN")?;
    require(&m, "NEW")?;
    command(&mut m, "20 PRINT\"TWO\"")?;
    command(&mut m, "30 END")?;
    for (text, expected) in [
        ("LIST20", "20 PRINT\"TWO\""),
        ("LIST-10", "10 PRINT\"NEW\""),
        ("LIST30-", "30 END"),
    ] {
        command(&mut m, text)?;
        require(&m, expected)?;
    }
    command(&mut m, "NEW")?;
    let inputs = [
        b"10 P\xd2\"X\"".as_slice(),
        b"20 F\xcf I=1TO2",
        b"30 G\xcf 100",
        b"40 L\xc9",
        b"50 R\xd5",
        b"60 I\xce A",
        b"70 E\xce",
        b"80 N\xc5",
    ];
    for input in inputs {
        type_bytes(&mut m, input)?;
        type_text(&mut m, "\r")?;
        advance(&mut m, CLOCK_HZ / 2)?;
    }
    let mut at = 0x801;
    let expected = [152, 129, 137, 155, 138, 132, 128, 130];
    for token in expected {
        if m.bus().ram(at + 4) != token {
            return Err("Shifted keyword abbreviation changed its public token".into());
        }
        at = u16::from_le_bytes([m.bus().ram(at), m.bus().ram(at + 1)]);
    }
    command(&mut m, "NEW")?;
    type_bytes(&mut m, b"PRINT\xff\r")?;
    advance(&mut m, CLOCK_HZ * 2)?;
    require(&m, " 3.14159265")?;
    type_bytes(&mut m, b"PRINT\"\x93X\"\r")?;
    advance(&mut m, CLOCK_HZ * 2)?;
    require(&m, "X")?;
    command(
        &mut m,
        "PRINTCHR$(147);CHR$(18);\"R\";CHR$(146);CHR$(28);\"C\"",
    )?;
    if m.bus().ram(0x400) != 146 || m.bus().ram(0x401) != 3 || m.bus_mut().read(0xd801)? & 15 != 2 {
        return Err("Reverse/colour guest screen controls failed".into());
    }
    Ok(format!(
        "{{\"schemaVersion\":1,\"keyboardOnly\":true,\"cursorOverwrite\":true,\"deleteAndInsert\":true,\"reeditProgramme\":true,\"listRanges\":true,\"shiftedAbbreviations\":{},\"piKey\":true,\"quotedControlRoundTrip\":true,\"reverseColour\":true,\"traceSha256\":\"{}\",\"basicV2Complete\":false}}",
        expected.len(),
        digest(&expected)
    ))
}
