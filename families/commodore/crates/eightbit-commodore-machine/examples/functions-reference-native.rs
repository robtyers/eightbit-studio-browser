//! Explicit supplied-ROM guest function/functions observations.
#[path = "support/reference.rs"]
mod reference;
use reference::functions_language;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = reference::read()?;
    println!(
        "{}",
        functions_language::qualify(reference::validate(&bytes)?)?
    );
    bytes.fill(0);
    Ok(())
}
