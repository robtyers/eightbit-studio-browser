//! Generated native page/interrupt trace, shared with the compiled WASM gate.
#[path = "support/paging.rs"]
mod paging;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    print!("{}", paging::trace()?);
    Ok(())
}
