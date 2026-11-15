//! Explicit original-image guest function/files development corpus.
#[allow(dead_code)]
#[path = "support/boot.rs"]
mod boot;
#[path = "support/reference.rs"]
mod reference;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (bytes, expected) = boot::read()?;
    println!("{}", boot::qualify_files(&bytes, &expected)?);
    Ok(())
}
