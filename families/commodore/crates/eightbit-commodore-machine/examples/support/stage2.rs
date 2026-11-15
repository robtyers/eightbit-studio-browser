//! Original/generated full-board observation corpus; no vendor firmware is embedded.
use eightbit_commodore_machine::{
    Firmware,
    input::{Joystick, Key},
    pal::{Machine, Pins},
    vic::FRAME_CYCLES,
};
use eightbit_mos6502_core::Bus;
use sha2::{Digest, Sha256};
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn trace() -> Result<String, Box<dyn std::error::Error>> {
    let basic = [0; 8192];
    let characters = [0xaa; 4096];
    let mut kernal = [0xea; 8192];
    kernal[0x1ffc] = 0;
    kernal[0x1ffd] = 2;
    let mut machine = Machine::new(Firmware::new(&basic, &kernal, &characters)?);
    machine.bus_mut().initialise_ram(0x200, &[0x4c, 0, 2])?;
    machine.reset()?;
    let bus = machine.bus_mut();
    bus.initialise_ram(0x400, &[1; 1000])?;
    bus.initialise_ram(13 * 64, &[0xff; 63])?;
    bus.initialise_ram(0x7f8, &[13])?;
    for (address, value) in [
        (0xd011, 0x1b),
        (0xd016, 8),
        (0xd018, 0x14),
        (0xd020, 6),
        (0xd021, 0),
        (0xd800, 7),
        (0xd000, 80),
        (0xd001, 100),
        (0xd027, 2),
        (0xd015, 1),
        (0xd400, 69),
        (0xd401, 29),
        (0xd405, 0),
        (0xd406, 240),
        (0xd418, 15),
        (0xd404, 17),
        (0xdc04, 5),
        (0xdc05, 0),
        (0xdc0e, 0x11),
    ] {
        bus.write(address, value)?;
    }
    machine.advance(FRAME_CYCLES * 8, || false)?;
    let frame = hash(machine.bus().hardware().video().frame());
    let audio: Vec<u8> = machine
        .bus_mut()
        .take_audio()
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect();
    let bus = machine.bus_mut();
    bus.key(Key::new(7, 6)?, true);
    bus.joystick(1, Joystick::new(17)?)?;
    bus.write(0xdc02, 255)?;
    bus.write(0xdc00, 0x7f)?;
    let keyboard = bus.read(0xdc01)?;
    let flags = bus.read(0xdc0d)?;
    bus.pins(Pins {
        restore: true,
        ..Pins::default()
    });
    let edge = bus.interrupt_lines();
    bus.release_inputs();
    let observed = format!(
        "frame={frame};audio={};keyboard={keyboard};cia={flags};nmi={edge:?};cycles={};stalls={};cpu={:?}",
        hash(&audio),
        bus.cycles(),
        bus.stalls(),
        machine.registers()
    );
    machine.advance(1000, || false)?;
    machine.bus_mut().initialise_ram(0x200, &[2])?;
    let faulted = machine.advance(1, || false).is_err();
    let audio_cleared = machine.bus_mut().take_audio().is_empty();
    if !faulted || !audio_cleared {
        return Err("Fault did not revoke queued audio".into());
    }
    Ok(format!(
        "{observed};faulted={faulted};faultAudioCleared={audio_cleared}"
    ))
}
