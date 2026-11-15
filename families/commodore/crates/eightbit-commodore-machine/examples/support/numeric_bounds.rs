//! Explicit hostile heap fixtures and bounded expression recovery, original image only.
use super::{advance, screen, type_text};
use eightbit_commodore_machine::{Firmware, input::Key, pal::Machine, sid::CLOCK_HZ};
fn command(
    machine: &mut Machine<'_>,
    text: &str,
    expected: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    type_text(machine, text)?;
    type_text(machine, "\r")?;
    advance(machine, CLOCK_HZ)?;
    let display = screen(machine);
    let rows: Vec<_> = display.lines().map(str::trim_end).collect();
    // The shared text observer renders the screen's up-arrow glyph as a space.
    let echo = text.replace('^', " ");
    if rows
        .iter()
        .rposition(|row| *row == echo)
        .and_then(|at| rows.get(at + 1))
        != Some(&expected)
    {
        return Err(format!("Numeric bounds {text}: expected {expected}\n{display}").into());
    }
    super::await_public_page(machine)?;
    Ok(())
}
fn heap(machine: &mut Machine<'_>) -> Result<(), Box<dyn std::error::Error>> {
    for (at, value) in [
        (0x2b, 0x801u16),
        (0x2d, 0x803),
        (0x2f, 0x803),
        (0x31, 0x803),
        (0x33, 0xa000),
        (0x37, 0xa000),
    ] {
        machine.bus_mut().initialise_ram(at, &value.to_le_bytes())?;
    }
    machine.bus_mut().initialise_ram(0x801, &[0; 2])?;
    machine.bus_mut().initialise_ram(0x803, &[0xa5; 7])?;
    Ok(())
}
pub fn qualify(firmware: Firmware<'_>) -> Result<String, Box<dyn std::error::Error>> {
    let mut machine = Machine::new(firmware);
    machine.reset()?;
    advance(&mut machine, CLOCK_HZ)?;
    for (at, value) in [
        (0x2d, 0x802u16),
        (0x2f, 0x802),
        (0x31, 0x802),
        (0x33, 0x802),
        (0x33, 0xa001),
    ] {
        heap(&mut machine)?;
        machine.bus_mut().initialise_ram(at, &value.to_le_bytes())?;
        command(&mut machine, "A=1", "?BAD PROGRAM ERROR")?;
        if !(0x803..0x80a).all(|at| machine.bus().ram(at) == 0xa5) {
            return Err("Hostile scalar boundary partially wrote a record".into());
        }
    }
    heap(&mut machine)?;
    machine.bus_mut().initialise_ram(0x2f, &[4, 8])?;
    machine.bus_mut().initialise_ram(0x31, &[4, 8])?;
    command(&mut machine, "A=1", "?BAD PROGRAM ERROR")?;
    heap(&mut machine)?;
    machine.bus_mut().initialise_ram(0x33, &[9, 8])?;
    command(&mut machine, "A=1", "READY.")?;
    if [machine.bus().ram(0x33), machine.bus().ram(0x34)] != [0, 0xa0] {
        return Err("Collection did not reclaim an unrooted heap gap".into());
    }
    heap(&mut machine)?;
    machine.bus_mut().initialise_ram(0x33, &[9, 8])?;
    machine.bus_mut().initialise_ram(0x37, &[9, 8])?;
    command(&mut machine, "A=1", "?OUT OF MEMORY ERROR")?;
    if !(0x803..0x80a).all(|at| machine.bus().ram(at) == 0xa5)
        || [machine.bus().ram(0x2f), machine.bus().ram(0x30)] != [3, 8]
    {
        return Err("Full scalar allocation changed heap".into());
    }
    heap(&mut machine)?;
    machine.bus_mut().initialise_ram(0x33, &[10, 8])?;
    machine.bus_mut().initialise_ram(0x37, &[10, 8])?;
    command(&mut machine, "A=1", "READY.")?;
    if !(0x803..0x80a)
        .map(|at| machine.bus().ram(at))
        .eq([65, 0, 129, 0, 0, 0, 0])
        || [machine.bus().ram(0x2f), machine.bus().ram(0x30)] != [10, 8]
    {
        return Err("Exact seven-byte scalar capacity failed".into());
    }
    heap(&mut machine)?;
    for (text, expected) in [
        ("PRINT PEEK(-1)", "?ILLEGAL QUANTITY ERROR"),
        ("PRINT ABS(\"X\")", "?TYPE MISMATCH ERROR"),
        ("PRINT(-1)^.5", "?ILLEGAL QUANTITY ERROR"),
        ("PRINT A(11)", "?BAD SUBSCRIPT ERROR"),
    ] {
        command(&mut machine, text, expected)?;
    }
    let deep = format!("PRINT {}1{}", "(".repeat(25), ")".repeat(25));
    type_text(&mut machine, &deep)?;
    type_text(&mut machine, "\r")?;
    advance(&mut machine, CLOCK_HZ)?;
    if !screen(&machine).contains("?FORMULA TOO COMPLEX ERROR") {
        return Err("Deep expression did not recover within its stack bound".into());
    }
    command(&mut machine, "PRINT 6*7", " 42")?;
    // Stop at a real instruction in the allocated numeric workspace, rather
    // than guessing that a fixed clock delay landed inside decimal conversion.
    for reset in [false, true] {
        type_text(
            &mut machine,
            "PRINT 3.141592653589793238462643383279502884197",
        )?;
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
            {
                active = true;
                break;
            }
        }
        if !active {
            return Err(
                format!("Did not enter live decimal workspace\n{}", screen(&machine)).into(),
            );
        }
        machine.bus_mut().release_inputs();
        if reset {
            machine.reset()?;
            advance(&mut machine, CLOCK_HZ)?;
            command(&mut machine, "PRINT 9*9", " 81")?;
        } else {
            let registers = machine.registers();
            let page = machine.bus().original_page();
            machine.stop();
            if machine.advance(1, || false).is_ok() {
                return Err("Stopped decimal advanced".into());
            }
            machine.resume()?;
            if machine.advance(100, || true)?.cycles != 0
                || machine.registers() != registers
                || machine.bus().original_page() != page
            {
                return Err("Decimal cancellation changed context".into());
            }
            machine.resume()?;
            advance(&mut machine, CLOCK_HZ * 2)?;
            if !screen(&machine)
                .lines()
                .any(|row| row.trim_end() == " 3.14159265")
            {
                return Err("Decimal Stop/cancel/resume lost result".into());
            }
            super::await_public_page(&mut machine)?;
        }
    }
    Ok("{\"schemaVersion\":1,\"explicitPublicRamFixtures\":true,\"invalidHeapCases\":6,\"exactSevenByteCapacity\":true,\"collectionReclaimsUnrootedGap\":true,\"failedAllocationPreservesHeap\":true,\"implementedExpressionFailures\":4,\"boundedExpressionRecovery\":true,\"liveDecimalStopCancelResumeReset\":true}".into())
}
