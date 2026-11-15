//! Public keyboard-queue observations for original physical modifier fixtures.
#[path = "support/reference.rs"]
mod reference;
use eightbit_commodore_machine::{input::Key, pal::Machine, sid::CLOCK_HZ};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = reference::read()?;
    let mut m = Machine::new(reference::validate(&bytes)?);
    m.reset()?;
    reference::advance(&mut m, CLOCK_HZ * 3)?;
    let mut codes = Vec::new();
    for letter in b'A'..=b'Z' {
        if m.bus().ram(0xc6) != 0 {
            return Err("Keyboard queue not empty before modifier probe".into());
        }
        m.bus_mut().key(Key::new(7, 5)?, true);
        m.bus_mut().key(reference::physical(letter)?.0, true);
        let mut code = None;
        for _ in 0..50_000 {
            reference::advance(&mut m, 1)?;
            if m.bus().ram(0xc6) > 0 {
                code = Some(m.bus().ram(0x277));
                break;
            }
        }
        codes.push(code.ok_or("Modifier did not produce a bounded public queue byte")?);
        m.bus_mut().release_inputs();
        reference::advance(&mut m, CLOCK_HZ / 5)?;
    }
    println!("Commodore modifier A..Z: {codes:?}");
    bytes.fill(0);
    Ok(())
}
