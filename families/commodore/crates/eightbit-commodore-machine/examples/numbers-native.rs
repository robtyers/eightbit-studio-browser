//! Explicit original numeric development corpus; no supplied ROM.
#[allow(dead_code)]
#[path = "support/boot.rs"]
mod boot;
#[path = "support/reference.rs"]
mod reference;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (bytes, expected) = boot::read()?;
    let mut args = std::env::args().skip(1);
    let bounds = match args.next().as_deref() {
        None => false,
        Some("--bounds") => true,
        Some(_) => return Err("Use no arguments or --bounds".into()),
    };
    if args.next().is_some() {
        return Err("Use no arguments or --bounds".into());
    }
    println!(
        "{}",
        if bounds {
            boot::qualify_numeric_bounds(&bytes, &expected)?
        } else {
            boot::qualify_numeric(&bytes, &expected)?
        }
    );
    Ok(())
}
