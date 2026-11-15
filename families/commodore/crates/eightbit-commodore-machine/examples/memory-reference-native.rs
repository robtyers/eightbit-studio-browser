//! Explicit supplied-ROM guest function/memory observations.
#[path = "support/reference.rs"]
mod reference;
use reference::memory_language;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = reference::read()?;
    println!(
        "{}",
        memory_language::qualify(reference::validate(&bytes)?)?
    );
    bytes.fill(0);
    Ok(())
}
