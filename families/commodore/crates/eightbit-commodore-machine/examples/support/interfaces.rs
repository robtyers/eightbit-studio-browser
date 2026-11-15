//! Original, keyboard-entered public RAM observations, never firmware extraction.
use super::reference::{advance, screen, type_text};
use eightbit_commodore_machine::{Firmware, pal::Machine, sid::CLOCK_HZ};

fn word(machine: &Machine<'_>, address: u16) -> u16 {
    u16::from_le_bytes([machine.bus().ram(address), machine.bus().ram(address + 1)])
}
fn bytes(machine: &Machine<'_>, address: u16, count: u16) -> Vec<u8> {
    (0..count)
        .map(|offset| machine.bus().ram(address + offset))
        .collect()
}
fn require(value: bool, message: &'static str) -> Result<(), Box<dyn std::error::Error>> {
    if value { Ok(()) } else { Err(message.into()) }
}
fn returned_value(machine: &Machine<'_>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let start = word(machine, 0x2d);
    let end = word(machine, 0x2f);
    require(
        (0x803..0x900).contains(&start) && end >= start && end - start <= 16 * 7,
        "USR variable table outside the original probe's bound",
    )?;
    for index in 0..16 {
        let address = start + index * 7;
        if address + 7 <= end && bytes(machine, address, 2) == b"U\0" {
            return Ok(bytes(machine, address + 2, 5));
        }
    }
    Err("Guest USR did not assign its return value".into())
}
pub fn qualify(firmware: Firmware<'_>) -> Result<String, Box<dyn std::error::Error>> {
    let mut machine = Machine::new(firmware);
    machine.reset()?;
    advance(&mut machine, CLOCK_HZ * 3)?;
    require(
        screen(&machine).contains("READY."),
        "Interface probe boot failed",
    )?;
    type_text(
        &mut machine,
        "NEW\rA=0:B=1:C=-1:D=.5:E=1.5:F=2\rA%=32767:B%=-32768:S$=\"ABC\"\rDIM X(1):X(1)=1.5\rPRINT\"ABI READY\"\r",
    )?;
    advance(&mut machine, CLOCK_HZ)?;
    require(
        screen(&machine).contains("ABI READY"),
        "Interface probe input failed",
    )?;
    require(word(&machine, 0x2b) == 0x801, "BASIC text start changed")?;
    let variables = word(&machine, 0x2d);
    let arrays = word(&machine, 0x2f);
    let end = word(&machine, 0x31);
    require(
        (0x803..0x900).contains(&variables) && arrays == variables + 9 * 7 && end == arrays + 17,
        "Unexpected public variable/array layout",
    )?;
    // Only the bounded public RAM records produced by our own values are read.
    // These are observations of a data format, not copied ROM tables or code.
    let records = bytes(&machine, variables, 9 * 7);
    let array = bytes(&machine, arrays, 17);
    require(
        records[..56]
            == [
                65, 0, 0, 0, 0, 0, 0, 66, 0, 129, 0, 0, 0, 0, 67, 0, 129, 128, 0, 0, 0, 68, 0, 128,
                0, 0, 0, 0, 69, 0, 129, 64, 0, 0, 0, 70, 0, 130, 0, 0, 0, 0, 193, 128, 127, 255, 0,
                0, 0, 194, 128, 128, 0, 0, 0, 0,
            ]
            && records[56..59] == [83, 128, 3],
        "Original numeric/type-name vectors changed",
    )?;
    require(
        array == [88, 0, 17, 0, 1, 0, 2, 0, 0, 0, 0, 0, 129, 64, 0, 0, 0],
        "Original dimension/element vectors changed",
    )?;
    let descriptor = variables + 8 * 7 + 2;
    let string_length = machine.bus().ram(descriptor);
    let string_address = word(&machine, descriptor + 1);
    if string_length != 3 || !(0x200..=0xa000 - 3).contains(&string_address) {
        return Err(format!("Unexpected public string descriptor: length={string_length}, address={string_address}, records={records:?}").into());
    }
    let string = bytes(&machine, string_address, 3);
    require(
        string == b"ABC",
        "String descriptor did not resolve our original text",
    )?;
    // The guest alone installs this original 6510 subroutine through DATA/POKE.
    // USR passes its value in FAC1; copy six documented accumulator bytes to RAM
    // and leave them unchanged for the return. No host programme injection.
    type_text(
        &mut machine,
        "10 FOR I=0 TO 12:READ Z:POKE 49152+I,Z:NEXT\r20 DATA162,5,181,97,157,0,193,202,16,248,96,234,234\rRUN\rPOKE785,0:POKE786,192\rU=USR(1.5)\r",
    )?;
    advance(&mut machine, CLOCK_HZ)?;
    let positive_fac = bytes(&machine, 0xc100, 6);
    require(
        positive_fac == [129, 192, 0, 0, 0, 0] && returned_value(&machine)? == [129, 64, 0, 0, 0],
        "Positive guest USR argument/return vector changed",
    )?;
    type_text(&mut machine, "U=USR(-1.5)\r")?;
    advance(&mut machine, CLOCK_HZ)?;
    let negative_fac = bytes(&machine, 0xc100, 6);
    require(
        negative_fac == [129, 192, 0, 0, 0, 255]
            && returned_value(&machine)? == [129, 192, 0, 0, 0],
        "Negative guest USR argument/return vector changed",
    )?;
    require(
        !screen(&machine).contains("ERROR"),
        "Guest USR argument/return probe failed",
    )?;
    // Public clock parameter order is misstated in some period prose. Observe
    // actual guest calls, not ROM instructions. Pause/ack the CIA timer while
    // collecting register and RAM bytes, then restore its control register.
    let clock_code = [
        0x08, 0xad, 0x0e, 0xdc, 0x48, 0x78, 0xa9, 0, 0x8d, 0x0e, 0xdc, 0xad, 0x0d, 0xdc, 0xa9,
        0x56, 0xa2, 0x34, 0xa0, 0x12, 0x20, 0xdb, 0xff, 0x20, 0xde, 0xff, 0x8d, 0x10, 0xc1, 0x8e,
        0x11, 0xc1, 0x8c, 0x12, 0xc1, 0xa5, 0xa0, 0x8d, 0x13, 0xc1, 0xa5, 0xa1, 0x8d, 0x14, 0xc1,
        0xa5, 0xa2, 0x8d, 0x15, 0xc1, 0x68, 0x8d, 0x0e, 0xdc, 0x28, 0x60,
    ];
    type_text(
        &mut machine,
        &format!(
            "NEW\r10 FOR I=0 TO {}:READ Z:POKE49152+I,Z:NEXT\r15 SYS49152\r",
            clock_code.len() - 1
        ),
    )?;
    for (index, chunk) in clock_code.chunks(12).enumerate() {
        let data = chunk
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(",");
        type_text(&mut machine, &format!("{} DATA{data}\r", 20 + index * 10))?;
    }
    type_text(&mut machine, "RUN\r")?;
    advance(&mut machine, CLOCK_HZ)?;
    let clock_layout = bytes(&machine, 0xc110, 6);
    require(
        clock_layout == [0x56, 0x34, 0x12, 0x12, 0x34, 0x56],
        "Public SETTIM/RDTIM register/RAM order changed",
    )?;
    machine.stop();
    Ok(format!(
        "{{\"schemaVersion\":1,\"keyboardOnly\":true,\"textStart\":2049,\"variableStart\":{variables},\"arrayStart\":{arrays},\"arrayEnd\":{end},\"records\":{records:?},\"array\":{array:?},\"string\":{string:?},\"positiveFac\":{positive_fac:?},\"negativeFac\":{negative_fac:?},\"clockRegistersThenRam\":{clock_layout:?}}}"
    ))
}
