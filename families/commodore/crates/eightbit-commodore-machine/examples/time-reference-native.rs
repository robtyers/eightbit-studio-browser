//! Explicit public clock-variable observations, using original keyboard inputs.
#[path = "support/reference.rs"]
mod reference;
use eightbit_commodore_machine::{pal::Machine, sid::CLOCK_HZ};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = reference::read()?;
    let mut machine = Machine::new(reference::validate(&bytes)?);
    machine.reset()?;
    reference::advance(&mut machine, CLOCK_HZ * 3)?;
    for value in [
        "AB0000", "240000", "006000", "000060", "12345", "1234567", "A00000", "Z00000", "??????",
    ] {
        reference::type_text(
            &mut machine,
            &format!("TI$=\"{value}\":PRINT\"TIME\";TI$\r"),
        )?;
        reference::advance(&mut machine, CLOCK_HZ)?;
        let display = reference::screen(&machine);
        let tail: Vec<_> = display
            .lines()
            .map(str::trim_end)
            .filter(|row| !row.is_empty())
            .rev()
            .take(2)
            .collect();
        println!("{value}: {tail:?}");
    }
    for text in ["TI=1", "ST=1", "PRINT TI%", "PRINT ST%"] {
        reference::type_text(&mut machine, &format!("{text}\r"))?;
        reference::advance(&mut machine, CLOCK_HZ)?;
        let display = reference::screen(&machine);
        let tail: Vec<_> = display
            .lines()
            .map(str::trim_end)
            .filter(|row| !row.is_empty())
            .rev()
            .take(2)
            .collect();
        println!("{text}: {tail:?}");
    }
    bytes.fill(0);
    Ok(())
}
