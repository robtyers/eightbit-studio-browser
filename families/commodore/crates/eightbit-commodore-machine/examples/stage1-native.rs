//! Native half of the stage-1 parity check.
#[path = "support/stage1.rs"]
mod stage1;
fn main() {
    println!("{}", stage1::trace());
}
