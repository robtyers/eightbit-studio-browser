//! Explicit supplied-ROM guest function/arrays observations.
#[path = "support/reference.rs"]
mod reference;
use reference::arrays_language;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = reference::read()?;
    println!(
        "{}",
        arrays_language::qualify(reference::validate(&bytes)?)?
    );
    bytes.fill(0);
    Ok(())
}
