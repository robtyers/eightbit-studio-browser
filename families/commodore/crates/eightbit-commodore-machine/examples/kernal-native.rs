//! Explicit original firmware public KERNAL and text-entry fixtures.
#[allow(dead_code)]
#[path = "support/boot.rs"]
mod boot;
#[path = "support/reference.rs"]
mod reference;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (bytes, hash) = boot::read()?;
    boot::qualify_kernal(&bytes, &hash)?;
    println!("Original KERNAL vector/keyboard/text fixtures passed");
    Ok(())
}
