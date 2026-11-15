//! Explicit supplied-ROM guest function/strings observations.
#[path = "support/reference.rs"]
mod reference;
use reference::strings_language;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = reference::read()?;
    println!(
        "{}",
        strings_language::qualify(reference::validate(&bytes)?)?
    );
    bytes.fill(0);
    Ok(())
}
