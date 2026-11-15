//! Explicit exact-hash supplied-ROM numeric observations; no implementation reads.
#[path = "support/reference.rs"]
mod reference;
use reference::numeric_language;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = reference::read()?;
    let firmware = reference::validate(&bytes)?;
    println!("{}", numeric_language::qualify(firmware)?);
    bytes.fill(0);
    Ok(())
}
