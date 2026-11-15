//! Original assembly-call vectors, explicitly initialised in user RAM for tests.
use eightbit_commodore_machine::{
    Firmware,
    pal::{PalBus, Pins},
};
use eightbit_mos6502_core::{C, Cpu, I, Registers, U};

fn require(value: bool, message: &'static str) -> Result<(), Box<dyn std::error::Error>> {
    if value { Ok(()) } else { Err(message.into()) }
}
fn call(
    bus: &mut PalBus<'_>,
    address: u16,
    registers: Registers,
) -> Result<Registers, Box<dyn std::error::Error>> {
    let [low, high] = address.to_le_bytes();
    bus.initialise_ram(0xc000, &[0x20, low, high, 0xea])?;
    let mut cpu = Cpu::from_registers(Registers {
        pc: 0xc000,
        sp: 0x80,
        p: registers.p | I | U,
        ..registers
    });
    // A full 960-cell text+colour scroll alone needs over 6,000 instructions.
    for _ in 0..10_000 {
        cpu.step(bus)?;
        if cpu.registers().pc == 0xc003 {
            return Ok(cpu.registers());
        }
    }
    Err("Public KERNAL vector exceeded its instruction bound".into())
}
pub fn qualify(firmware: Firmware<'_>) -> Result<(), Box<dyn std::error::Error>> {
    let mut bus = PalBus::new(firmware);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus)?;
    let mut booted = false;
    for _ in 0..100_000 {
        cpu.step(&mut bus)?;
        if cpu.registers().pc == 0xa100 && bus.original_page() == Some((0, 0)) {
            booted = true;
            break;
        }
    }
    require(booted, "KERNAL contract fixture boot failed")?;
    let input = Registers::default();
    for address in [0xff87, 0xff8a, 0xff84, 0xff81] {
        call(&mut bus, address, input)?;
    }
    let result = call(&mut bus, 0xffed, input)?;
    require(
        (result.x, result.y) == (40, 25),
        "SCREEN public dimensions failed",
    )?;
    let result = call(&mut bus, 0xfff3, input)?;
    require(
        (result.x, result.y) == (0, 0xdc),
        "IOBASE public pointer failed",
    )?;
    let result = call(&mut bus, 0xff99, Registers { p: C, ..input })?;
    require(
        (result.x, result.y) == (0, 0xa0),
        "MEMTOP public pointer failed",
    )?;
    let result = call(&mut bus, 0xff9c, Registers { p: C, ..input })?;
    require(
        (result.x, result.y) == (0, 8),
        "MEMBOT public pointer failed",
    )?;
    for (address, x, y) in [(0xff99, 0x34, 0x92), (0xff9c, 0x56, 0x08)] {
        call(
            &mut bus,
            address,
            Registers {
                x,
                y,
                p: 0,
                ..input
            },
        )?;
        let changed = call(&mut bus, address, Registers { p: C, ..input })?;
        require(
            (changed.x, changed.y) == (x, y),
            "Memory bound write/read failed",
        )?;
    }
    // SCNKEY and GETIN operate on the real matrix and public queue.
    bus.key(eightbit_commodore_machine::input::Key::new(1, 2)?, true);
    call(&mut bus, 0xff9f, input)?;
    require(
        call(&mut bus, 0xffe4, input)?.a == b'A',
        "SCNKEY/GETIN failed",
    )?;
    bus.release_inputs();
    call(&mut bus, 0xff9f, input)?;
    require(
        call(&mut bus, 0xffe1, input)?.p & eightbit_mos6502_core::Z == 0,
        "Unpressed STOP failed",
    )?;
    bus.key(eightbit_commodore_machine::input::Key::new(7, 7)?, true);
    call(&mut bus, 0xff9f, input)?;
    require(
        call(&mut bus, 0xffe1, input)?.p & eightbit_mos6502_core::Z != 0,
        "Pressed STOP failed",
    )?;
    require(
        call(&mut bus, 0xffe4, input)?.a == 3,
        "STOP matrix character failed",
    )?;
    bus.release_inputs();
    call(&mut bus, 0xff9f, input)?;
    call(
        &mut bus,
        0xffba,
        Registers {
            a: 4,
            x: 8,
            y: 2,
            ..input
        },
    )?;
    call(
        &mut bus,
        0xffbd,
        Registers {
            a: 3,
            x: 0x80,
            y: 0xc1,
            ..input
        },
    )?;
    require(
        [
            bus.ram(0xb8),
            bus.ram(0xba),
            bus.ram(0xb9),
            bus.ram(0xb7),
            bus.ram(0xbb),
            bus.ram(0xbc),
        ] == [4, 8, 2, 3, 0x80, 0xc1],
        "File parameter workspace failed",
    )?;
    let result = call(&mut bus, 0xffd5, input)?;
    require(
        result.a == 5 && result.p & C != 0,
        "Unattached LOAD did not report device absent",
    )?;
    require(call(&mut bus, 0xffb7, input)?.a == 0x80, "READST failed")?;
    call(
        &mut bus,
        0xfff0,
        Registers {
            x: 10,
            y: 12,
            p: 0,
            ..input
        },
    )?;
    let result = call(&mut bus, 0xfff0, Registers { p: C, ..input })?;
    require(
        (result.x, result.y) == (10, 12),
        "PLOT public cursor failed",
    )?;
    call(&mut bus, 0xffd2, Registers { a: 65, ..input })?;
    require(
        bus.ram(0x400 + 10 * 40 + 12) == 1,
        "CHROUT public cursor output failed",
    )?;
    require(
        call(&mut bus, 0xffe4, input)?.a == 0,
        "Empty GETIN queue failed",
    )?;
    // Original RAM entry facts and a real page-boundary text-pointer wrap.
    bus.initialise_ram(0xc200, b" 9\0")?;
    bus.initialise_ram(0x7a, &[0, 0xc2])?;
    let character = call(
        &mut bus,
        0x79,
        Registers {
            x: 0x17,
            y: 0x23,
            ..input
        },
    )?;
    require(
        character.a == b'9'
            && character.p & C == 0
            && character.x == 0x17
            && character.y == 0x23
            && bus.ram(0x7a) == 1,
        "CHRGOT public character/flags failed",
    )?;
    bus.initialise_ram(0x7a, &[0xff, 0xc2])?;
    bus.initialise_ram(0xc300, &[0])?;
    let character = call(&mut bus, 0x73, input)?;
    require(
        character.a == 0 && character.p & C != 0 && bus.ram(0x7a) == 0 && bus.ram(0x7b) == 0xc3,
        "CHRGET public pointer wrap failed",
    )?;
    call(&mut bus, 0xff90, Registers { a: 0x80, ..input })?;
    call(&mut bus, 0xffa2, Registers { a: 1, ..input })?;
    require(
        bus.ram(0x9d) == 0x80 && bus.ram(0x285) == 1,
        "Message/timeout flags failed",
    )?;
    call(
        &mut bus,
        0xffba,
        Registers {
            a: 1,
            x: 3,
            y: 0,
            ..input
        },
    )?;
    call(&mut bus, 0xffbd, input)?;
    require(
        call(&mut bus, 0xffc0, input)?.p & C == 0,
        "Screen OPEN failed",
    )?;
    require(
        call(&mut bus, 0xffc9, Registers { x: 1, ..input })?.p & C == 0,
        "Screen CHKOUT failed",
    )?;
    let denied = call(&mut bus, 0xffc6, Registers { x: 1, ..input })?;
    require(
        denied.a == 6 && denied.p & C != 0,
        "Output-only channel accepted CHKIN",
    )?;
    call(&mut bus, 0xffc3, Registers { a: 1, ..input })?;
    let missing = call(&mut bus, 0xffc9, Registers { x: 1, ..input })?;
    require(
        missing.a == 3 && missing.p & C != 0,
        "Closed channel accepted CHKOUT",
    )?;
    call(
        &mut bus,
        0xffba,
        Registers {
            a: 2,
            x: 0,
            y: 0,
            ..input
        },
    )?;
    require(
        call(&mut bus, 0xffc0, input)?.p & C == 0,
        "Keyboard OPEN failed",
    )?;
    require(
        call(&mut bus, 0xffc6, Registers { x: 2, ..input })?.p & C == 0,
        "Keyboard CHKIN failed",
    )?;
    bus.initialise_ram(0x277, &[b'Z', 13])?;
    bus.initialise_ram(0xc6, &[2])?;
    require(
        call(
            &mut bus,
            0xffcf,
            Registers {
                x: 0x17,
                y: 0x23,
                ..input
            },
        )?
        .a == b'Z',
        "CHRIN did not return edited keyboard data",
    )?;
    require(
        call(&mut bus, 0xffcf, input)?.a == 13,
        "CHRIN did not terminate its public line",
    )?;
    call(&mut bus, 0xffcc, input)?;
    call(&mut bus, 0xffe7, input)?;
    require(
        bus.ram(0x98) == 0 && bus.ram(0x99) == 0 && bus.ram(0x9a) == 3,
        "CLALL/CLRCHN failed",
    )?;
    for address in [
        0xff93, 0xff96, 0xffa5, 0xffa8, 0xffab, 0xffae, 0xffb1, 0xffb4, 0xffd8,
    ] {
        let absent = call(&mut bus, address, input)?;
        require(
            absent.a == 5 && absent.p & C != 0 && bus.ram(0x90) == 0x80,
            "Unattached service fabricated success",
        )?;
    }
    call(
        &mut bus,
        0xffdb,
        Registers {
            a: 0x56,
            x: 0x34,
            y: 0x12,
            ..input
        },
    )?;
    let result = call(&mut bus, 0xffde, input)?;
    require(
        (result.a, result.x, result.y) == (0x56, 0x34, 0x12),
        "Clock public register order failed",
    )?;
    call(&mut bus, 0xffea, input)?;
    let ticked = call(&mut bus, 0xffde, input)?;
    require(
        (ticked.a, ticked.x, ticked.y) == (0x57, 0x34, 0x12),
        "UDTIM failed",
    )?;
    call(
        &mut bus,
        0xff8d,
        Registers {
            p: C,
            x: 0,
            y: 0xc1,
            ..input
        },
    )?;
    for index in 0..32 {
        require(
            bus.ram(0xc100 + index) == bus.ram(0x314 + index),
            "VECTOR read failed",
        )?;
    }
    // Assembly fixtures use explicit user RAM initialisation. The separate boot
    // and physical keyboard journey never injects programme/workspace bytes.
    call(
        &mut bus,
        0xfff0,
        Registers {
            x: 10,
            y: 13,
            p: 0,
            ..input
        },
    )?;
    bus.initialise_ram(0xc112, &[0x80, 0xc1])?;
    bus.initialise_ram(0xc180, &[0xee, 0x90, 0xc1, 0x60])?;
    call(
        &mut bus,
        0xff8d,
        Registers {
            p: 0,
            x: 0,
            y: 0xc1,
            ..input
        },
    )?;
    call(&mut bus, 0xffd2, Registers { a: 66, ..input })?;
    require(bus.ram(0xc190) == 1, "Patchable CHROUT vector failed")?;
    call(&mut bus, 0xff8a, input)?;
    call(&mut bus, 0xffd2, Registers { a: 66, ..input })?;
    require(
        bus.ram(0x400 + 10 * 40 + 13) == 2,
        "RESTOR vector recovery failed",
    )?;
    bus.initialise_ram(
        0xc1a0,
        &[
            0xba, 0xbd, 4, 1, 0x8d, 0x92, 0xc1, 0xee, 0x91, 0xc1, 0x68, 0xa8, 0x68, 0xaa, 0x68,
            0x40,
        ],
    )?;
    bus.initialise_ram(0x318, &[0xa0, 0xc1])?;
    bus.initialise_ram(0xc000, &[0x20, 0, 0xf8, 0xea])?;
    let mut cpu = Cpu::from_registers(Registers {
        pc: 0xc000,
        sp: 0x80,
        p: C | U,
        ..input
    });
    let (mut triggered, mut returned) = (false, false);
    for _ in 0..100_000 {
        if !triggered && bus.original_page() == Some((1, 1)) {
            bus.pins(Pins {
                restore: true,
                ..Pins::default()
            });
            triggered = true;
        } else {
            bus.pins(Pins::default());
        }
        cpu.step(&mut bus)?;
        if cpu.registers().pc == 0xc003 {
            returned = true;
            break;
        }
    }
    require(
        triggered && returned && bus.ram(0xc191) == 1 && bus.original_page() == Some((0, 0)),
        "User NMI RTI callback did not restore private page",
    )?;
    require(
        bus.ram(0xc192) & (I | C) == C,
        "User interrupt frame lost original status",
    )?;
    // Original screen/colour patterns cross every 256-byte copy boundary.
    // Snapshot the source before scrolling so overlap cannot mask a bad copy.
    let cells: Vec<_> = (0..1000)
        .map(|index| u8::try_from(index % 251))
        .collect::<Result<_, _>>()?;
    let colours: Vec<_> = (0..1000)
        .map(|index| u8::try_from(index % 16))
        .collect::<Result<_, _>>()?;
    bus.initialise_ram(0x400, &cells)?;
    bus.initialise_ram(0x7e8, &[0x5a; 24])?;
    use eightbit_mos6502_core::Bus;
    for (index, colour) in colours.iter().enumerate() {
        bus.write(0xd800 + u16::try_from(index)?, *colour)?;
    }
    call(
        &mut bus,
        0xfff0,
        Registers {
            x: 24,
            y: 0,
            p: 0,
            ..input
        },
    )?;
    call(&mut bus, 0xffd2, Registers { a: 13, ..input })?;
    for index in 0..960u16 {
        require(
            bus.ram(0x400 + index) == cells[usize::from(index) + 40],
            "Screen scroll overlapped unread source",
        )?;
        require(
            bus.read(0xd800 + index)? & 15 == colours[usize::from(index) + 40],
            "Colour scroll overlapped unread source",
        )?;
    }
    require(
        (0x7c0..0x7e8).all(|address| bus.ram(address) == 32)
            && (0x7e8..0x800).all(|address| bus.ram(address) == 0x5a),
        "Scroll damaged bottom blank or sprite pointers",
    )?;
    call(&mut bus, 0xffd2, Registers { a: 20, ..input })?;
    require(
        bus.ram(0xd6) == 23 && bus.ram(0xd3) == 39 && bus.ram(0x7bf) == 32,
        "Wrapped physical-line delete failed",
    )?;
    Ok(())
}
