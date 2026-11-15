//! Native half of original PAL-board parity.
#[path = "support/stage2.rs"]
mod stage2;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", stage2::trace()?);
    Ok(())
}
