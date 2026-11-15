//! Original exact-rational fixtures, evaluated by ordinary guest 6510 instructions.
use super::digest;
use eightbit_commodore_machine::{
    Firmware,
    pal::{PalBus, Pins},
};
use eightbit_mos6502_core::{Cpu, I, Registers, U};

fn packed(text: &str) -> Result<[u8; 5], Box<dyn std::error::Error>> {
    if text.len() != 10 {
        return Err("Numeric fixture length".into());
    }
    let mut result = [0; 5];
    for (index, byte) in result.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16)?;
    }
    Ok(result)
}
fn unpack(bytes: [u8; 5]) -> [u8; 6] {
    if bytes[0] == 0 {
        return [0; 6];
    }
    [
        bytes[0],
        bytes[1] | 128,
        bytes[2],
        bytes[3],
        bytes[4],
        if bytes[1] & 128 == 0 { 0 } else { 255 },
    ]
}
fn result(bus: &PalBus<'_>) -> [u8; 5] {
    let exponent = bus.ram(0x61);
    if exponent == 0 {
        return [0; 5];
    }
    [
        exponent,
        (bus.ram(0x62) & 127) | (bus.ram(0x66) & 128),
        bus.ram(0x63),
        bus.ram(0x64),
        bus.ram(0x65),
    ]
}
fn call(bus: &mut PalBus<'_>, operation: u8) -> Result<Registers, Box<dyn std::error::Error>> {
    let mut cpu = Cpu::from_registers(Registers {
        pc: 0xc000,
        sp: 0xc0,
        a: operation,
        p: I | U,
        ..Registers::default()
    });
    let mut interrupted = false;
    for _ in 0..500_000 {
        let interrupt = !interrupted
            && matches!(operation, 4 | 5)
            && bus.ram(0x23) == 1
            && bus.ram(0x27) == 1
            && cpu.registers().sp < bus.ram(0x22)
            && bus.original_page() == Some((2, 1));
        bus.pins(Pins {
            restore: interrupt,
            ..Pins::default()
        });
        interrupted |= interrupt;
        cpu.step(bus)?;
        if cpu.registers().pc == 0xc003 {
            if cpu.registers().sp != 0xc0 || bus.original_page() != Some((0, 0)) {
                return Err("Guest numeric gate did not restore caller".into());
            }
            if matches!(operation, 4 | 5) && !interrupted {
                return Err("Numeric stack fixture did not receive its physical NMI".into());
            }
            if [bus.ram(0x24), bus.ram(0x25), bus.ram(0x47), bus.ram(0x48)] != [0x5a, 0xa5, 0x81, 8]
            {
                return Err("Numeric helper overwrote programme/scalar pointers".into());
            }
            return Ok(cpu.registers());
        }
    }
    Err("Guest numeric instruction bound".into())
}
pub fn qualify(firmware: Firmware<'_>) -> Result<String, Box<dyn std::error::Error>> {
    let mut bus = PalBus::new(firmware);
    bus.initialise_ram(0xc000, &[0x20, 0x80, 0xf8, 0xea])?;
    bus.initialise_ram(0x318, &[0x80, 0xc1])?;
    // Original public callback: restore the saved registers, then the real RTI.
    bus.initialise_ram(0xc180, &[0x68, 0xa8, 0x68, 0xaa, 0x68, 0x40])?;
    bus.initialise_ram(0x24, &[0x5a, 0xa5])?;
    bus.initialise_ram(0x47, &[0x81, 8])?;
    let mut trace = Vec::new();
    let mut vectors = 0u32;
    for line in include_str!("../../tests/fixtures/numeric-binary.txt")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() != 5 {
            return Err("Numeric fixture columns".into());
        }
        let operation = fields[0].parse::<u8>()?;
        bus.initialise_ram(0x61, &unpack(packed(fields[1])?))?;
        bus.initialise_ram(0x69, &unpack(packed(fields[2])?))?;
        let expected = packed(fields[3])?;
        let status = fields[4].parse::<u8>()?;
        let registers = call(&mut bus, operation)?;
        if registers.x != status || (status == 0 && result(&bus) != expected) {
            return Err(format!("Guest numeric vector {vectors} op {operation}: status={}, expected={status}, actual={:02x?}, expected={expected:02x?}",registers.x,result(&bus)).into());
        }
        trace.extend(result(&bus));
        trace.push(status);
        vectors += 1;
    }
    let mut decimals = 0u32;
    for line in include_str!("../../tests/fixtures/numeric-decimal.txt")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() != 3 {
            return Err("Decimal fixture columns".into());
        }
        let mut input = fields[0].as_bytes().to_vec();
        input.push(0);
        bus.initialise_ram(0xc200, &input)?;
        bus.initialise_ram(0x7a, &[0, 0xc2])?;
        let registers = call(&mut bus, 4)?;
        let expected = packed(fields[1])?;
        let status = fields[2].parse::<u8>()?;
        if registers.x != status || (status == 0 && result(&bus) != expected) {
            return Err(format!("Decimal vector {decimals} {}: status={}, expected={status}, actual={:02x?}, expected={expected:02x?}",fields[0],registers.x,result(&bus)).into());
        }
        trace.extend(result(&bus));
        trace.push(status);
        decimals += 1;
    }
    let mut displays = 0u32;
    for line in include_str!("../../tests/fixtures/numeric-display.txt")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() != 2 {
            return Err("Display fixture columns".into());
        }
        let input = packed(fields[0])?;
        bus.initialise_ram(0x61, &unpack(input))?;
        let registers = call(&mut bus, 5)?;
        let mut output = Vec::new();
        for address in 0x100..0x120 {
            let byte = bus.ram(address);
            if byte == 0 {
                break;
            }
            output.push(byte);
        }
        let expected: Result<Vec<_>, _> = (0..fields[1].len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&fields[1][index..index + 2], 16))
            .collect();
        if registers.x != 0 || output != expected? || result(&bus) != input {
            return Err(format!(
                "Display vector {displays} {}: status={}, actual={:?}",
                fields[0],
                registers.x,
                String::from_utf8_lossy(&output)
            )
            .into());
        }
        trace.extend(output);
        trace.push(0);
        displays += 1;
    }
    let mut integers = 0u32;
    for line in include_str!("../../tests/fixtures/numeric-integer.txt")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() != 4 {
            return Err("Integer fixture columns".into());
        }
        let operation = fields[0].parse::<u8>()?;
        let status = fields[3].parse::<u8>()?;
        if operation == 6 {
            bus.initialise_ram(0x14, &u16::from_str_radix(fields[1], 16)?.to_le_bytes())?;
        } else {
            bus.initialise_ram(0x61, &unpack(packed(fields[1])?))?;
        }
        let registers = call(&mut bus, operation)?;
        let actual_integer = u16::from_le_bytes([bus.ram(0x14), bus.ram(0x15)]);
        if registers.x != status
            || (status == 0
                && if operation == 6 {
                    result(&bus) != packed(fields[2])?
                } else {
                    actual_integer != u16::from_str_radix(fields[2], 16)?
                })
        {
            return Err(format!("Integer vector {integers}: {line}, status={}, integer={actual_integer:04x}, FAC={:02x?}", registers.x,result(&bus)).into());
        }
        trace.push(operation);
        trace.push(status);
        if status == 0 {
            if operation == 6 {
                trace.extend(result(&bus));
            } else {
                trace.extend(actual_integer.to_le_bytes());
            }
        }
        integers += 1;
    }
    let mut control = 0u32;
    for line in include_str!("../../tests/fixtures/numeric-control.txt")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() != 6 {
            return Err("Control numeric fixture columns".into());
        }
        let operation = fields[0].parse::<u8>()?;
        let left = packed(fields[1])?;
        let right = packed(fields[2])?;
        let expected = packed(fields[3])?;
        let status = fields[4].parse::<u8>()?;
        let order = fields[5].parse::<u8>()?;
        bus.initialise_ram(0x61, &unpack(left))?;
        bus.initialise_ram(0x69, &unpack(right))?;
        let registers = call(&mut bus, operation)?;
        if registers.x != status
            || (status == 0 && result(&bus) != expected)
            || (operation == 8
                && (registers.a != order
                    || (0..6u16).any(|at| bus.ram(0x69 + at) != unpack(right)[usize::from(at)])))
        {
            return Err(format!(
                "Control numeric vector {control}: {line}, status={}, A={}, FAC={:02x?}",
                registers.x,
                registers.a,
                result(&bus)
            )
            .into());
        }
        trace.extend([
            operation,
            status,
            if operation == 8 { registers.a } else { 0 },
        ]);
        if status == 0 {
            trace.extend(result(&bus));
        }
        control += 1;
    }
    Ok(format!(
        "{{\"schemaVersion\":1,\"explicitAssemblyFixtures\":true,\"binaryVectors\":{vectors},\"decimalVectors\":{decimals},\"displayVectors\":{displays},\"integerVectors\":{integers},\"controlVectors\":{control},\"decimalAndDisplayNmi\":true,\"publicPointersPreserved\":true,\"traceSha256\":\"{}\",\"hostBasicEvaluator\":false}}",
        digest(&trace)
    ))
}
