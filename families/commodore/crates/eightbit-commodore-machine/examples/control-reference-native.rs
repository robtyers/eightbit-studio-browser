//! Explicit exact-hash supplied-ROM conditional/subroutine observations.
#[path = "support/reference.rs"]
mod reference;
use reference::control_language;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = reference::read()?;
    println!(
        "{}",
        control_language::qualify(reference::validate(&bytes)?)?
    );
    bytes.fill(0);
    Ok(())
}
