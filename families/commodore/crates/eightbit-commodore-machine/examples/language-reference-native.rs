//! Shared public language corpus using explicitly supplied exact-hash ROMs.
#[path = "support/reference.rs"]
mod reference;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = reference::read()?;
    println!("{}", reference::language(reference::validate(&bytes)?)?);
    bytes.fill(0);
    Ok(())
}
