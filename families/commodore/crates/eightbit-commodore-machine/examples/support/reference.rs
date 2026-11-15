//! Exact-hash supplied-firmware gate and original keyboard-only observations.
#![allow(dead_code)] // Shared test helpers are selected independently by focused probes.
use eightbit_commodore_machine::{
    Firmware, input::Key, pal::Machine, sid::CLOCK_HZ, vic::FRAME_CYCLES,
};
use sha2::{Digest, Sha256};
#[path = "arrays_language.rs"]
pub mod arrays_language;
#[path = "basic_vectors.rs"]
pub mod basic_vectors;
#[path = "control_language.rs"]
pub mod control_language;
#[path = "editor_language.rs"]
pub mod editor_language;
#[path = "files_language.rs"]
pub mod files_language;
#[path = "functions_language.rs"]
pub mod functions_language;
#[path = "input_language.rs"]
pub mod input_language;
#[path = "loops_language.rs"]
pub mod loops_language;
#[path = "memory_language.rs"]
pub mod memory_language;
#[path = "numeric_language.rs"]
pub mod numeric_language;
#[path = "strings_language.rs"]
pub mod strings_language;

#[cfg(not(target_arch = "wasm32"))]
pub const NAMES: [&str; 3] = [
    "basic-901226-01.bin",
    "kernal-901227-03.bin",
    "chargen-901225-01.bin",
];
pub const SIZES: [usize; 3] = [8192, 8192, 4096];
pub const HASHES: [&str; 3] = [
    "89878cea0a268734696de11c4bae593eaaa506465d2029d619c0e0cbccdfa62d",
    "83c60d47047d7beab8e5b7bf6f67f80daa088b7a6a27de0d7e016f6484042721",
    "fd0d53b8480e86163ac98998976c72cc58d5dd8eb824ed7b829774e74213b420",
];
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn validate(packed: &[u8]) -> Result<Firmware<'_>, Box<dyn std::error::Error>> {
    if packed.len() != 20480 {
        return Err("Supplied firmware length rejected".into());
    }
    let mut offset = 0;
    for index in 0..3 {
        if digest(&packed[offset..offset + SIZES[index]]) != HASHES[index] {
            return Err("Supplied firmware identity rejected".into());
        }
        offset += SIZES[index];
    }
    Ok(Firmware::new(
        &packed[..8192],
        &packed[8192..16384],
        &packed[16384..],
    )?)
}
#[cfg(not(target_arch = "wasm32"))]
pub fn read() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use std::io::Read;
    let root = std::path::PathBuf::from(std::env::var("EIGHTBIT_C64_ROMS")?);
    let mut packed = Vec::with_capacity(20480);
    for index in 0..3 {
        let mut bytes = Vec::new();
        std::fs::File::open(root.join(NAMES[index]))?
            .take(u64::try_from(SIZES[index] + 1)?)
            .read_to_end(&mut bytes)?;
        if bytes.len() != SIZES[index] {
            return Err("Supplied firmware window length rejected".into());
        }
        packed.extend(bytes);
    }
    validate(&packed)?;
    Ok(packed)
}
pub fn advance(machine: &mut Machine<'_>, cycles: u32) -> Result<(), Box<dyn std::error::Error>> {
    let mut remaining = cycles;
    while remaining > 0 {
        let chunk = remaining.min(985_248);
        machine.advance(chunk, || false)?;
        remaining -= chunk;
    }
    Ok(())
}
pub fn screen(machine: &Machine<'_>) -> String {
    let mut text = String::new();
    for row in 0..25u16 {
        for col in 0..40u16 {
            let code = machine.bus().ram(0x400 + row * 40 + col) & 0x7f;
            text.push(char::from(match code {
                1..=26 => b'A' + code - 1,
                32..=63 => code,
                _ => b' ',
            }));
        }
        text.push('\n');
    }
    text
}
/// Published matrix position for an explicit fixture byte.
pub fn physical(byte: u8) -> Result<(Key, bool), Box<dyn std::error::Error>> {
    let byte = byte.to_ascii_uppercase();
    if (0xc1..=0xda).contains(&byte) {
        let (key, _) = physical(byte & 0x7f)?;
        return Ok((key, true));
    }
    if byte == 0xff {
        return Ok((Key::new(6, 6)?, true));
    }
    let rows = [
        b"\x14\r\x1d\x88\x85\x86\x87\x11",
        b"3WA4ZSE\x01",
        b"5RD6CFTX",
        b"7YG8BHUV",
        b"9IJ0MKON",
        b"+PL-.:@,",
        b"\x5c*;\x13\x02=^/",
    ];
    let special = match byte {
        148 => Some((0, 0)),
        145 => Some((0, 7)),
        157 => Some((0, 2)),
        147 => Some((6, 3)),
        b'#' => Some((1, 0)),
        b'"' => Some((7, 3)),
        b'$' => Some((1, 3)),
        b'%' => Some((2, 0)),
        b'(' => Some((3, 3)),
        b')' => Some((4, 0)),
        b'<' => Some((5, 7)),
        b'>' => Some((5, 4)),
        b'?' => Some((6, 7)),
        _ => None,
    };
    if let Some((row, column)) = special {
        return Ok((Key::new(row, column)?, true));
    }
    let last = match byte {
        b'1' => Some(0),
        b'2' => Some(3),
        b' ' => Some(4),
        b'Q' => Some(6),
        _ => None,
    };
    if let Some(column) = last {
        return Ok((Key::new(7, column)?, false));
    }
    for row in 0..7u8 {
        for col in 0..8u8 {
            if rows[usize::from(row)][usize::from(col)] == byte {
                return Ok((Key::new(row, col)?, false));
            }
        }
    }
    Err(format!("Original keyboard corpus contains an unmapped key {byte}").into())
}
pub fn type_text(machine: &mut Machine<'_>, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    type_bytes(machine, text.as_bytes())
}
/// Type explicit PETSCII bytes using only the same physical key matrix timeline.
pub fn type_bytes(
    machine: &mut Machine<'_>,
    text: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    for &byte in text {
        let (key, shift) = physical(byte)?;
        if shift {
            machine.bus_mut().key(Key::new(1, 7)?, true);
        }
        machine.bus_mut().key(key, true);
        advance(machine, FRAME_CYCLES * 2)?;
        machine.bus_mut().release_inputs();
        advance(machine, FRAME_CYCLES * 2)?;
    }
    Ok(())
}
fn require(value: bool, message: &'static str) -> Result<(), Box<dyn std::error::Error>> {
    if value { Ok(()) } else { Err(message.into()) }
}
pub fn qualify(
    firmware: Firmware<'_>,
    capture: impl FnMut(&str, &[u8], &[i16]) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<String, Box<dyn std::error::Error>> {
    qualify_runtime(firmware, false, capture)
}
pub fn qualify_runtime(
    firmware: Firmware<'_>,
    original: bool,
    mut capture: impl FnMut(&str, &[u8], &[i16]) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut machine = Machine::new(firmware);
    machine.reset()?;
    advance(&mut machine, CLOCK_HZ * 3)?;
    require(
        screen(&machine).contains(if original {
            "EIGHTBIT C64 ORIGINAL"
        } else {
            "38911 BASIC BYTES FREE"
        }) && screen(&machine).contains("READY."),
        "Stock ROM cold boot failed",
    )?;
    for line in
        include_str!("../../../eightbit-commodore-core/tests/fixtures/acceptance.bas").lines()
    {
        type_text(&mut machine, line)?;
        type_text(&mut machine, "\r")?;
    }
    type_text(&mut machine, "RUN\r")?;
    for _ in 0..30 {
        advance(&mut machine, CLOCK_HZ)?;
        if screen(&machine).contains("SPRITE + SID ACTIVE") {
            break;
        }
    }
    let text = screen(&machine);
    require(
        text.contains("C64 BASELINE READY")
            && text.contains("SPRITE + SID ACTIVE")
            && text.contains("15"),
        "Keyboard-entered BASIC/sprite/SID programme failed",
    )?;
    // Begin the observation window only after the guest reports its active state.
    // A fixed delay from RUN also contains the interpreted initialisation time.
    advance(&mut machine, CLOCK_HZ)?;
    let audio = machine.bus_mut().take_audio();
    require(
        audio.len() == 44_100,
        "Expected one-second bounded PCM tail",
    )?;
    let crossings = audio
        .windows(2)
        .filter(|pair| pair[0] <= 0 && pair[1] > 0)
        .count();
    let peak = audio
        .iter()
        .map(|sample| i32::from(*sample).abs())
        .max()
        .unwrap_or(0);
    if !(438..=442).contains(&crossings) || peak <= 500 {
        capture(
            "tone-diagnostic",
            machine.bus().hardware().video().frame(),
            &audio,
        )?;
        return Err(format!(
            "SID observation failed: crossings={crossings}, peak={peak}, frequency={:?}",
            machine.bus().hardware().sound().frequency(0)
        )
        .into());
    }
    let pixels = machine.bus().hardware().video().frame();
    require(
        pixels.iter().filter(|pixel| **pixel == 7).count() > 40,
        "Sprite frame observation failed",
    )?;
    capture("basic-active", pixels, &audio)?;
    let basic_hash = digest(pixels);
    let pcm: Vec<u8> = audio
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect();
    let pcm_hash = digest(&pcm);
    type_text(&mut machine, "Q")?;
    advance(&mut machine, CLOCK_HZ * 2)?;
    require(
        screen(&machine).contains("INPUT OK"),
        "Physical Q input did not reach the guest",
    )?;
    let silence = machine.bus_mut().take_audio();
    require(
        silence.iter().all(|sample| *sample == 0),
        "Guest SID cleanup did not silence output",
    )?;
    capture(
        "basic-after-q",
        machine.bus().hardware().video().frame(),
        &silence,
    )?;
    // Guest BASIC installs original assembly through DATA/POKE and calls SYS.
    // The host never mutates programme/workspace memory in this qualification.
    type_text(
        &mut machine,
        "NEW\r10 FOR I=0 TO 10:READ A:POKE 49152+I,A:NEXT\r20 SYS49152\r30 DATA169,7,141,32,208,169,65,32,210,255,96\r40 PRINT \"ASM RETURN OK\"\rRUN\r",
    )?;
    advance(&mut machine, CLOCK_HZ)?;
    require(
        screen(&machine).contains("ASM RETURN OK"),
        "Guest SYS/CHROUT/RTS assembly journey failed",
    )?;
    let final_frame = digest(machine.bus().hardware().video().frame());
    capture("assembly", machine.bus().hardware().video().frame(), &[])?;
    type_text(&mut machine, "LOAD\"ABSENT\",8\r")?;
    advance(&mut machine, CLOCK_HZ * 2)?;
    require(
        screen(&machine).contains("DEVICE NOT PRESENT"),
        "Unattached IEC drive did not return a guest error",
    )?;
    let frames = machine.bus().hardware().video().frames();
    let clocks = machine.bus().cycles();
    let stalls = machine.bus().stalls();
    machine.stop();
    require(
        machine.advance(1, || false).is_err(),
        "Stopped machine advanced",
    )?;
    machine.resume()?;
    require(
        machine.advance(100, || true)?.cycles == 0,
        "Cancellation advanced the guest",
    )?;
    machine.reset()?;
    advance(&mut machine, CLOCK_HZ * 3)?;
    require(
        screen(&machine).contains("READY."),
        "Reset/repeated boot failed",
    )?;
    Ok(format!(
        "{{\"schemaVersion\":1,\"coldBootReady\":true,\"keyboardBasic\":true,\"sum\":15,\"sprite\":true,\"sidCrossings\":{crossings},\"sidPeak\":{peak},\"qCleanup\":true,\"assemblySysReturn\":true,\"unattachedIecError\":true,\"stopCancelReset\":true,\"frames\":{frames},\"clocks\":{clocks},\"dmaStalls\":{stalls},\"basicFrameSha256\":\"{basic_hash}\",\"pcmSha256\":\"{pcm_hash}\",\"assemblyFrameSha256\":\"{final_frame}\"}}"
    ))
}

/// Identical public language observations for original and supplied firmware.
pub fn language(firmware: Firmware<'_>) -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!(
        "{{{}}}",
        [
            format!(
                "\"numeric_language\":{}",
                numeric_language::qualify(firmware)?
            ),
            format!(
                "\"control_language\":{}",
                control_language::qualify(firmware)?
            ),
            format!(
                "\"memory_language\":{}",
                memory_language::qualify(firmware)?
            ),
            format!("\"loops_language\":{}", loops_language::qualify(firmware)?),
            format!(
                "\"strings_language\":{}",
                strings_language::qualify(firmware)?
            ),
            format!(
                "\"arrays_language\":{}",
                arrays_language::qualify(firmware)?
            ),
            format!("\"input_language\":{}", input_language::qualify(firmware)?),
            format!(
                "\"functions_language\":{}",
                functions_language::qualify(firmware)?
            ),
            format!("\"files_language\":{}", files_language::qualify(firmware)?),
            format!(
                "\"editor_language\":{}",
                editor_language::qualify(firmware)?
            ),
            format!("\"basic_vectors\":{}", basic_vectors::qualify(firmware)?),
        ]
        .join(",")
    ))
}
