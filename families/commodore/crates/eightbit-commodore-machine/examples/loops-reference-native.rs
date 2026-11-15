//! Explicit supplied-ROM guest function/loops observations.
#[path = "support/reference.rs"]
mod reference;
use reference::loops_language;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = reference::read()?;
    println!("{}", loops_language::qualify(reference::validate(&bytes)?)?);
    bytes.fill(0);
    Ok(())
}
