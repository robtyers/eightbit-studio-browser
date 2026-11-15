//! Explicit malformed/public-RAM fixtures; separate from keyboard-only entry.
use super::{advance, digest, screen, type_text};
use eightbit_commodore_machine::{Firmware, pal::Machine, sid::CLOCK_HZ};

fn require(value: bool, message: &'static str) -> Result<(), Box<dyn std::error::Error>> {
    if value { Ok(()) } else { Err(message.into()) }
}
fn command(machine: &mut Machine<'_>, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    type_text(machine, text)?;
    type_text(machine, "\r")?;
    advance(machine, CLOCK_HZ / 2)?;
    Ok(())
}
fn error_after(machine: &Machine<'_>, command: &str, message: &str) -> bool {
    let text = screen(machine);
    let rows: Vec<_> = text.lines().map(str::trim_end).collect();
    rows.iter()
        .rposition(|row| *row == command)
        .and_then(|index| rows.get(index + 1))
        == Some(&message)
}
fn install(machine: &mut Machine<'_>, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let end = 0x801u16
        .checked_add(u16::try_from(bytes.len())?)
        .ok_or("Fixture overflow")?;
    machine.bus_mut().initialise_ram(0x801, bytes)?;
    machine.bus_mut().initialise_ram(0x2b, &[1, 8])?;
    machine.bus_mut().initialise_ram(0x2d, &end.to_le_bytes())?;
    machine.bus_mut().initialise_ram(0x37, &[0, 0xa0])?;
    Ok(())
}
pub fn qualify(firmware: Firmware<'_>) -> Result<String, Box<dyn std::error::Error>> {
    let mut machine = Machine::new(firmware);
    machine.reset()?;
    advance(&mut machine, CLOCK_HZ)?;
    machine.bus_mut().initialise_ram(0xc000, &[0x5a, 0xa5])?;
    machine.bus_mut().initialise_ram(0xfb, &[1, 2, 3, 4])?;
    command(&mut machine, "10 PRINT\"KEEP\"")?;
    command(&mut machine, "20 END")?;
    let end = u16::from_le_bytes([machine.bus().ram(0x2d), machine.bus().ram(0x2e)]);
    let good: Vec<_> = (0x801..end)
        .map(|address| machine.bus().ram(address))
        .collect();
    let first_end = usize::from(u16::from_le_bytes([good[0], good[1]]) - 0x801);
    let mut corruptions = Vec::new();
    for link in [0x801u16, 0x804, 0x9fff, 0] {
        let mut bytes = good.clone();
        bytes[..2].copy_from_slice(&link.to_le_bytes());
        corruptions.push(bytes);
    }
    let mut missing = good.clone();
    missing[first_end - 1] = b'X';
    corruptions.push(missing);
    let mut early = good.clone();
    early[4] = 0;
    corruptions.push(early);
    let mut duplicate = good.clone();
    duplicate[first_end + 2..first_end + 4].copy_from_slice(&10u16.to_le_bytes());
    corruptions.push(duplicate);
    let mut invalid_number = good.clone();
    invalid_number[2..4].copy_from_slice(&64000u16.to_le_bytes());
    corruptions.push(invalid_number);
    let mut bad_end = good.clone();
    *bad_end.last_mut().ok_or("Empty fixture")? = 1;
    corruptions.push(bad_end);
    for bytes in &corruptions {
        install(&mut machine, bytes)?;
        command(&mut machine, "LIST")?;
        require(
            error_after(&machine, "LIST", "?BAD PROGRAM ERROR"),
            "Malformed linked text did not diagnose",
        )?;
        let actual: Vec<_> = (0x801..end)
            .map(|address| machine.bus().ram(address))
            .collect();
        require(
            actual == *bytes,
            "Malformed programme was partially rewritten",
        )?;
    }
    for invalid_end in [0u16, 1, 0x800, 0x802, 0xa001, 0xffff] {
        install(&mut machine, &good)?;
        machine
            .bus_mut()
            .initialise_ram(0x2d, &invalid_end.to_le_bytes())?;
        command(&mut machine, "RUN")?;
        require(
            error_after(&machine, "RUN", "?BAD PROGRAM ERROR"),
            "Invalid VARTAB did not diagnose",
        )?;
    }
    for (address, value) in [(0x37, 0x802u16), (0x2b, 0xffff), (0x2b, 0xc000)] {
        install(&mut machine, &good)?;
        machine
            .bus_mut()
            .initialise_ram(address, &value.to_le_bytes())?;
        command(&mut machine, "NEW")?;
        require(
            error_after(&machine, "NEW", "?BAD PROGRAM ERROR"),
            "Invalid bounds allowed NEW write",
        )?;
    }
    install(&mut machine, &good)?;
    machine.bus_mut().initialise_ram(0x37, &end.to_le_bytes())?;
    command(&mut machine, "30 REM NO ROOM")?;
    require(
        error_after(&machine, "30 REM NO ROOM", "?OUT OF MEMORY ERROR"),
        "Full programme edit was not rejected",
    )?;
    require(
        (0x801..end)
            .map(|address| machine.bus().ram(address))
            .eq(good.iter().copied()),
        "Full edit changed old text",
    )?;
    require(
        [machine.bus().ram(0x2d), machine.bus().ram(0x2e)] == end.to_le_bytes(),
        "Failed edit changed VARTAB",
    )?;

    // Exact fill with original canonical REM records, including 255-byte records.
    let mut full = Vec::new();
    let mut line = 0u16;
    while full.len() < 0x9800 - 3 {
        let record = (0x9800 - 3 - full.len()).min(255);
        require(record >= 6, "Invalid generated full fixture")?;
        let next = 0x801u16
            .checked_add(u16::try_from(full.len() + record)?)
            .ok_or("Full fixture overflow")?;
        full.extend(next.to_le_bytes());
        full.extend(line.to_le_bytes());
        full.push(0x8f);
        full.extend(std::iter::repeat_n(b'X', record - 6));
        full.push(0);
        line += 1;
    }
    full.extend([0, 0]);
    require(full.len() == 0x9800 - 1, "Full fixture did not reach A000")?;
    install(&mut machine, &full)?;
    command(&mut machine, "63999 REM FULL")?;
    advance(&mut machine, CLOCK_HZ)?;
    require(
        error_after(&machine, "63999 REM FULL", "?OUT OF MEMORY ERROR"),
        "Exact A000 boundary allowed overflow",
    )?;
    require(
        (0x801..0xa000)
            .map(|address| machine.bus().ram(address))
            .eq(full.iter().copied()),
        "Exact full edit changed text",
    )?;
    command(&mut machine, "0")?;
    advance(&mut machine, CLOCK_HZ * 8)?;
    require(
        u16::from_le_bytes([machine.bus().ram(0x2d), machine.bus().ram(0x2e)]) == 0xa000 - 255,
        "Max-record deletion did not compact",
    )?;
    command(&mut machine, "0 REM RESTORED")?;
    advance(&mut machine, CLOCK_HZ * 8)?;
    command(&mut machine, "RUN")?;
    for _ in 0..10 {
        advance(&mut machine, CLOCK_HZ)?;
        if error_after(&machine, "RUN", "READY.") {
            break;
        }
    }
    if !error_after(&machine, "RUN", "READY.") {
        return Err(format!(
            "Compacted full programme did not run: pc={:04x} line={}\n{}",
            machine.registers().pc,
            u16::from_le_bytes([machine.bus().ram(0x39), machine.bus().ram(0x3a)]),
            screen(&machine)
        )
        .into());
    }
    require(
        [machine.bus().ram(0xc000), machine.bus().ram(0xc001)] == [0x5a, 0xa5]
            && (0xfb..=0xfe)
                .map(|address| machine.bus().ram(address))
                .eq([1, 2, 3, 4]),
        "Guest BASIC reserved user RAM",
    )?;
    let last_end = u16::from_le_bytes([machine.bus().ram(0x2d), machine.bus().ram(0x2e)]);
    let last: Vec<_> = (0x801..last_end)
        .map(|address| machine.bus().ram(address))
        .collect();
    Ok(format!(
        "{{\"schemaVersion\":1,\"explicitPublicRamFixtures\":true,\"malformedLinks\":{},\"invalidBoundaries\":9,\"failurePreservesProgramme\":true,\"exactA000Boundary\":true,\"maximumRecordDeletion\":true,\"userRamCanaries\":true,\"compactedTextSha256\":\"{}\"}}",
        corruptions.len(),
        digest(&last)
    ))
}
