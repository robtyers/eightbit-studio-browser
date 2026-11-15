//! Explicit supplied-ROM guest function/vectors observations.
#[path = "support/reference.rs"]
mod reference;
use reference::basic_vectors;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = reference::read()?;
    println!("{}", basic_vectors::qualify(reference::validate(&bytes)?)?);
    bytes.fill(0);
    Ok(())
}
