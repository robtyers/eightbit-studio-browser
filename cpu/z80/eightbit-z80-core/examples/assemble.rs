//! Local original-firmware build helper. The library itself performs no I/O.
use eightbit_z80_core::assembler::{MAX_SOURCE_BYTES, assemble};
use std::{
    fs::File,
    io::{Read, Write},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let source = arguments
        .next()
        .ok_or("Supply source path and decimal origin")?;
    let origin: u16 = arguments.next().ok_or("Supply decimal origin")?.parse()?;
    if arguments.next().is_some() {
        return Err("Unexpected argument".into());
    }
    let mut text = String::new();
    File::open(source)?
        .take(u64::try_from(MAX_SOURCE_BYTES)? + 1)
        .read_to_string(&mut text)?;
    let output = assemble(&text, origin, None)?;
    std::io::stdout().lock().write_all(&output.bytes)?;
    Ok(())
}
