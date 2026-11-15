//! Explicit supplied-ROM guest function/input observations.
#[path = "support/reference.rs"]
mod reference;
use reference::input_language;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = reference::read()?;
    println!("{}", input_language::qualify(reference::validate(&bytes)?)?);
    bytes.fill(0);
    Ok(())
}
