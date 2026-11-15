//! Explicit supplied-ROM guest function/editor observations.
#[path = "support/reference.rs"]
mod reference;
use reference::editor_language;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = reference::read()?;
    println!(
        "{}",
        editor_language::qualify(reference::validate(&bytes)?)?
    );
    bytes.fill(0);
    Ok(())
}
