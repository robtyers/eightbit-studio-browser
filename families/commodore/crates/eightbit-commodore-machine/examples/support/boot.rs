//! Original source-bound foundation qualification. No reference firmware is read.
use super::reference::arrays_language;
use super::reference::{advance, digest, screen, type_text};
use eightbit_commodore_machine::{Error, Firmware, original::Image, pal::Machine, sid::CLOCK_HZ};
#[path = "basic_bounds.rs"]
mod basic_bounds;
#[path = "basic_front_end.rs"]
mod basic_front_end;
use super::reference::basic_vectors;
#[path = "control_bounds.rs"]
mod control_bounds;
use super::reference::control_language;
use super::reference::editor_language;
use super::reference::files_language;
use super::reference::functions_language;
use super::reference::input_language;
#[path = "kernal_vectors.rs"]
mod kernal_vectors;
use super::reference::loops_language;
use super::reference::memory_language;
#[path = "numeric.rs"]
mod numeric;
#[path = "numeric_bounds.rs"]
mod numeric_bounds;
use super::reference::numeric_language;
use super::reference::strings_language;

#[cfg(not(target_arch = "wasm32"))]
pub fn read() -> Result<(Vec<u8>, [u8; 32]), Box<dyn std::error::Error>> {
    use std::io::Read;
    let path = std::path::PathBuf::from(std::env::var("EIGHTBIT_C64_ORIGINAL")?);
    if !path.is_absolute() {
        return Err("Original image path must be absolute".into());
    }
    let expected = std::env::var("EIGHTBIT_C64_ORIGINAL_SHA256")?;
    if expected.len() != 64 || !expected.is_ascii() {
        return Err("Original image hash must have 64 hex digits".into());
    }
    let mut hash = [0; 32];
    for (index, byte) in hash.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&expected[index * 2..index * 2 + 2], 16)?;
    }
    let file = std::fs::File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err("Select a regular original firmware image".into());
    }
    let mut bytes = Vec::new();
    file.take(u64::try_from(
        eightbit_commodore_machine::original::MAX_IMAGE + 1,
    )?)
    .read_to_end(&mut bytes)?;
    Image::parse(&bytes, &hash)?;
    Ok((bytes, hash))
}
fn require(value: bool, message: &'static str) -> Result<(), Box<dyn std::error::Error>> {
    if value { Ok(()) } else { Err(message.into()) }
}
fn clock(machine: &Machine<'_>) -> u32 {
    (u32::from(machine.bus().ram(0xa0)) << 16)
        | (u32::from(machine.bus().ram(0xa1)) << 8)
        | u32::from(machine.bus().ram(0xa2))
}
fn await_public_page(machine: &mut Machine<'_>) -> Result<(), Box<dyn std::error::Error>> {
    // A clock budget may end in the middle of an ordinary IRQ. Require its
    // actual guest epilogue to return within a bounded instruction window.
    for _ in 0..10_000 {
        if machine.bus().original_page() == Some((0, 0)) {
            return Ok(());
        }
        advance(machine, 1)?;
    }
    Err("Original page continuation did not settle".into())
}
pub fn qualify_numeric(
    bytes: &[u8],
    expected: &[u8; 32],
) -> Result<String, Box<dyn std::error::Error>> {
    numeric::qualify(Firmware::original(Image::parse(bytes, expected)?)?)
}
pub fn qualify_numeric_bounds(
    bytes: &[u8],
    expected: &[u8; 32],
) -> Result<String, Box<dyn std::error::Error>> {
    numeric_bounds::qualify(Firmware::original(Image::parse(bytes, expected)?)?)
}
pub fn qualify_control(
    bytes: &[u8],
    expected: &[u8; 32],
) -> Result<String, Box<dyn std::error::Error>> {
    let firmware = Firmware::original(Image::parse(bytes, expected)?)?;
    Ok(format!(
        "{{\"language\":{},\"bounds\":{}}}",
        control_language::qualify(firmware)?,
        control_bounds::qualify(firmware)?
    ))
}
pub fn qualify_memory(
    bytes: &[u8],
    expected: &[u8; 32],
) -> Result<String, Box<dyn std::error::Error>> {
    memory_language::qualify(Firmware::original(Image::parse(bytes, expected)?)?)
}
pub fn qualify_loops(
    bytes: &[u8],
    expected: &[u8; 32],
) -> Result<String, Box<dyn std::error::Error>> {
    loops_language::qualify(Firmware::original(Image::parse(bytes, expected)?)?)
}
pub fn qualify_strings(
    bytes: &[u8],
    expected: &[u8; 32],
) -> Result<String, Box<dyn std::error::Error>> {
    strings_language::qualify(Firmware::original(Image::parse(bytes, expected)?)?)
}
pub fn qualify_arrays(
    bytes: &[u8],
    expected: &[u8; 32],
) -> Result<String, Box<dyn std::error::Error>> {
    arrays_language::qualify(Firmware::original(Image::parse(bytes, expected)?)?)
}
pub fn qualify_input(
    bytes: &[u8],
    expected: &[u8; 32],
) -> Result<String, Box<dyn std::error::Error>> {
    input_language::qualify(Firmware::original(Image::parse(bytes, expected)?)?)
}
pub fn qualify_functions(
    bytes: &[u8],
    expected: &[u8; 32],
) -> Result<String, Box<dyn std::error::Error>> {
    functions_language::qualify(Firmware::original(Image::parse(bytes, expected)?)?)
}
pub fn qualify_files(
    bytes: &[u8],
    expected: &[u8; 32],
) -> Result<String, Box<dyn std::error::Error>> {
    files_language::qualify(Firmware::original(Image::parse(bytes, expected)?)?)
}
pub fn qualify_editor(
    bytes: &[u8],
    expected: &[u8; 32],
) -> Result<String, Box<dyn std::error::Error>> {
    editor_language::qualify(Firmware::original(Image::parse(bytes, expected)?)?)
}
pub fn qualify_vectors(
    bytes: &[u8],
    expected: &[u8; 32],
) -> Result<String, Box<dyn std::error::Error>> {
    basic_vectors::qualify(Firmware::original(Image::parse(bytes, expected)?)?)
}
#[allow(dead_code)] // Focused boundary example also uses the shared boot harness.
pub fn qualify_bounds(
    bytes: &[u8],
    expected: &[u8; 32],
) -> Result<String, Box<dyn std::error::Error>> {
    basic_bounds::qualify(Firmware::original(Image::parse(bytes, expected)?)?)
}
pub fn qualify_kernal(bytes: &[u8], expected: &[u8; 32]) -> Result<(), Box<dyn std::error::Error>> {
    kernal_vectors::qualify(Firmware::original(Image::parse(bytes, expected)?)?)
}
pub fn qualify(
    bytes: &[u8],
    expected: &[u8],
    mut capture: impl FnMut(&[u8]) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<String, Box<dyn std::error::Error>> {
    let expected: &[u8; 32] = expected.try_into().map_err(|_| Error::OriginalIdentity)?;
    let firmware = Firmware::original(Image::parse(bytes, expected)?)?;
    qualify_kernal(bytes, expected)?;
    let numeric = qualify_numeric(bytes, expected)?;
    let front_end = basic_front_end::qualify(firmware)?;
    let numeric_language = numeric_language::qualify(firmware)?;
    let bounds = basic_bounds::qualify(firmware)?;
    let numeric_bounds = qualify_numeric_bounds(bytes, expected)?;
    let control = qualify_control(bytes, expected)?;
    let memory = qualify_memory(bytes, expected)?;
    let loops = qualify_loops(bytes, expected)?;
    let strings = qualify_strings(bytes, expected)?;
    let arrays = qualify_arrays(bytes, expected)?;
    let input = qualify_input(bytes, expected)?;
    let functions = qualify_functions(bytes, expected)?;
    let files = qualify_files(bytes, expected)?;
    let editor = qualify_editor(bytes, expected)?;
    let basic_vectors = qualify_vectors(bytes, expected)?;
    let combined = super::reference::qualify_runtime(firmware, true, |_, _, _| Ok(()))?;
    let mut machine = Machine::new(firmware);
    machine.reset()?;
    advance(&mut machine, CLOCK_HZ * 3)?;
    let text = screen(&machine);
    require(
        text.contains("EIGHTBIT C64 ORIGINAL") && text.contains("BASIC V2 - DEVELOPMENT"),
        "Original foundation banner failed",
    )?;
    await_public_page(&mut machine)?;
    let start = clock(&machine);
    advance(&mut machine, CLOCK_HZ)?;
    let jiffies = clock(&machine)
        .checked_sub(start)
        .ok_or("Clock regressed")?;
    require((59..=61).contains(&jiffies), "Original 60 Hz clock failed")?;
    type_text(&mut machine, "?\"HELLO 64\"\r")?;
    advance(&mut machine, CLOCK_HZ)?;
    require(
        screen(&machine)
            .lines()
            .any(|line| line.trim_end() == "HELLO 64"),
        "Original keyboard/editor echo failed",
    )?;
    await_public_page(&mut machine)?;
    require(
        machine.bus().ram(0xc000) == 0,
        "Boot reserved user programme RAM",
    )?;
    let pixels = machine.bus().hardware().video().frame();
    require(
        pixels.iter().filter(|value| **value == 14).count() > 200,
        "Original character frame failed",
    )?;
    let frame = digest(pixels);
    capture(pixels)?;
    machine.stop();
    machine.resume()?;
    require(
        machine.advance(100, || true)?.cycles == 0,
        "Original cancellation advanced",
    )?;
    machine.reset()?;
    advance(&mut machine, CLOCK_HZ * 3)?;
    require(
        screen(&machine).contains("BASIC V2 - DEVELOPMENT"),
        "Original repeated reset failed",
    )?;
    Ok(format!(
        "{{\"schemaVersion\":1,\"originalImageSha256\":\"{}\",\"boot\":true,\"keyboardEcho\":true,\"jiffiesPerSecond\":{jiffies},\"pageRestored\":true,\"stopCancelReset\":true,\"kernalAssemblyVectors\":true,\"userNmiRtiCallback\":true,\"frameSha256\":\"{frame}\",\"basicImplemented\":true,\"basicV2Complete\":false,\"frontEnd\":{front_end},\"basicBounds\":{bounds},\"numeric\":{numeric},\"numericLanguage\":{numeric_language},\"numericBounds\":{numeric_bounds},\"control\":{control},\"memory\":{memory},\"loops\":{loops},\"strings\":{strings},\"arrays\":{arrays},\"input\":{input},\"functions\":{functions},\"files\":{files},\"editor\":{editor},\"basicVectors\":{basic_vectors},\"combinedRuntime\":{combined}}}",
        digest(bytes)
    ))
}
