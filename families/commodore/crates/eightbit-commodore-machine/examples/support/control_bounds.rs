//! Explicit guest return-stack corruption and lifecycle fixtures, original image only.
use super::{advance, screen, type_text};
use eightbit_commodore_machine::{Firmware, input::Key, pal::Machine, sid::CLOCK_HZ};
fn command(machine: &mut Machine<'_>, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    type_text(machine, text)?;
    type_text(machine, "\r")?;
    advance(machine, CLOCK_HZ)?;
    Ok(())
}
fn require(machine: &Machine<'_>, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    if screen(machine)
        .lines()
        .map(str::trim_end)
        .filter(|row| !row.is_empty())
        .rev()
        .nth(1)
        == Some(text)
    {
        Ok(())
    } else {
        Err(format!("Control boundary expected {text}\n{}", screen(machine)).into())
    }
}
fn word(machine: &Machine<'_>, at: u16) -> u16 {
    u16::from_le_bytes([machine.bus().ram(at), machine.bus().ram(at + 1)])
}
pub fn qualify(firmware: Firmware<'_>) -> Result<String, Box<dyn std::error::Error>> {
    let mut machine = Machine::new(firmware);
    machine.reset()?;
    advance(&mut machine, CLOCK_HZ)?;
    for text in ["10 GOSUB10", "RUN"] {
        command(&mut machine, text)?;
    }
    require(&machine, "?OUT OF MEMORY ERROR IN 10")?;
    command(&mut machine, "PRINT\"DEPTH\";6*7")?;
    require(&machine, "DEPTH 42")?;
    for (at, value, error) in [
        (0x1f2, 0x8cu8, "?RETURN WITHOUT GOSUB ERROR IN 100"),
        (0x1f4, 0xff, "?BAD PROGRAM ERROR IN 10"),
        (0x1f5, 0xff, "?BAD PROGRAM ERROR IN 10"),
        (0x1f7, 11, "?BAD PROGRAM ERROR IN 11"),
    ] {
        for text in ["NEW", "10 GOSUB100:END", "100 GOTO100", "RUN"] {
            command(&mut machine, text)?;
        }
        let second = word(&machine, 0x801);
        let terminator = word(&machine, second) - 1;
        let mut active = false;
        for _ in 0..100_000 {
            if machine.registers().sp == 0xf1
                && machine.bus().ram(0x1f2) == 0x8d
                && machine.bus().original_page() == Some((0, 0))
                // The current GOTO has consumed its target; the following
                // iteration will fetch our replacement token afresh. Stopping
                // after CHRGET returned can otherwise retain the previous A.
                && word(&machine, 0x7a) == terminator
            {
                active = true;
                break;
            }
            advance(&mut machine, 1)?;
        }
        if !active {
            return Err("Did not find real suspended GOSUB frame".into());
        }
        let end = word(&machine, 0x2d);
        // Turn the existing same-size loop body into RETURN followed by spaces;
        // preserve its line header, link and NUL so return validation stays real.
        machine.bus_mut().initialise_ram(second + 4, &[0x8e])?;
        for address in second + 5..terminator {
            machine.bus_mut().initialise_ram(address, &[32])?;
        }
        let before: Vec<_> = (0x801..end)
            .map(|address| machine.bus().ram(address))
            .collect();
        machine.bus_mut().initialise_ram(at, &[value])?;
        advance(&mut machine, CLOCK_HZ)?;
        require(&machine, error)?;
        if !(0x801..end)
            .map(|address| machine.bus().ram(address))
            .eq(before)
        {
            return Err("Bad return frame changed programme text".into());
        }
        super::await_public_page(&mut machine)?;
        command(&mut machine, "PRINT\"FRAME\";7*7")?;
        require(&machine, "FRAME 49")?;
    }
    for reset in [false, true] {
        for text in [
            "NEW",
            "100 A=3.141592653589793238462643383279502884197:RETURN",
        ] {
            command(&mut machine, text)?;
        }
        type_text(&mut machine, "GOSUB100:PRINT\"RESUME\";A")?;
        machine.bus_mut().key(Key::new(0, 1)?, true);
        let mut active = false;
        for _ in 0..100_000 {
            advance(&mut machine, 1)?;
            if machine
                .bus()
                .original_page()
                .is_some_and(|(page, _)| page == 2)
                && machine.registers().sp < 0xb0
                && machine.bus().ram(0x23) == 1
                && machine.bus().ram(0x27) == 1
                && machine.bus().ram(0x26).checked_sub(machine.bus().ram(0x22)) == Some(40)
                && machine.bus().ram(0x1f2) == 0x8d
            {
                active = true;
                break;
            }
        }
        if !active {
            return Err(format!(
                "No allocated numeric work inside GOSUB\n{}",
                screen(&machine)
            )
            .into());
        }
        machine.bus_mut().release_inputs();
        if reset {
            machine.reset()?;
            advance(&mut machine, CLOCK_HZ)?;
            command(&mut machine, "RETURN")?;
            require(&machine, "?RETURN WITHOUT GOSUB ERROR")?;
        } else {
            let registers = machine.registers();
            let page = machine.bus().original_page();
            let stack: Vec<_> = (0x100..0x200).map(|at| machine.bus().ram(at)).collect();
            machine.stop();
            if machine.advance(1, || false).is_ok() {
                return Err("Stopped subroutine advanced".into());
            }
            machine.resume()?;
            if machine.advance(100, || true)?.cycles != 0
                || machine.registers() != registers
                || machine.bus().original_page() != page
                || !(0x100..0x200).map(|at| machine.bus().ram(at)).eq(stack)
            {
                return Err("Subroutine cancellation changed stack/context".into());
            }
            machine.resume()?;
            advance(&mut machine, CLOCK_HZ * 2)?;
            require(&machine, "RESUME 3.14159265")?;
            super::await_public_page(&mut machine)?;
        }
    }
    Ok("{\"schemaVersion\":1,\"explicitStackFixtures\":true,\"corruptReturnFrames\":4,\"failurePreservesProgramme\":true,\"boundedSubroutineDepth\":6,\"numericSubroutineStopCancelResumeReset\":true}".into())
}
