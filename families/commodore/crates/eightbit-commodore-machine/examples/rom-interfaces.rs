//! Explicit exact-ROM gate for original public-interface observations.
#[path = "support/interfaces.rs"]
mod interfaces;
#[path = "support/reference.rs"]
mod reference;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut packed = reference::read()?;
    let result = interfaces::qualify(reference::validate(&packed)?);
    packed.fill(0);
    println!("{}", result?);
    Ok(())
}
