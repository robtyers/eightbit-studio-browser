//! Explicit public token observations for physically shifted original inputs.
#[path = "support/reference.rs"]
mod reference;
use eightbit_commodore_machine::{pal::Machine, sid::CLOCK_HZ};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = reference::read()?;
    let mut m = Machine::new(reference::validate(&bytes)?);
    m.reset()?;
    reference::advance(&mut m, CLOCK_HZ * 3)?;
    for input in [
        b"10 P\xd2\"X\"".as_slice(),
        b"20 F\xcf I=1TO2",
        b"30 G\xcf 100",
        b"40 L\xc9",
        b"50 R\xd5",
        b"60 I\xce A",
        b"70 E\xce",
        b"80 N\xc5",
    ] {
        reference::type_bytes(&mut m, input)?;
        reference::type_text(&mut m, "\r")?;
        reference::advance(&mut m, CLOCK_HZ)?;
    }
    let mut at = 0x801;
    for _ in 0..8 {
        println!(
            "{}: {}",
            u16::from_le_bytes([m.bus().ram(at + 2), m.bus().ram(at + 3)]),
            m.bus().ram(at + 4)
        );
        at = u16::from_le_bytes([m.bus().ram(at), m.bus().ram(at + 1)]);
    }
    bytes.fill(0);
    Ok(())
}
