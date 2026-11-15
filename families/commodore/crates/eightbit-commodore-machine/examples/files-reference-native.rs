//! Explicit supplied-ROM guest function/files observations.
#[path = "support/reference.rs"]
mod reference;
use reference::files_language;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = reference::read()?;
    println!("{}", files_language::qualify(reference::validate(&bytes)?)?);
    bytes.fill(0);
    Ok(())
}
