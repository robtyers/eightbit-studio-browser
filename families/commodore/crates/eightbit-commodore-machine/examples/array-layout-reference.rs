//! Explicit public-RAM array layout observations from original keyboard inputs.
#[path = "support/reference.rs"]
mod reference;
use eightbit_commodore_machine::{pal::Machine, sid::CLOCK_HZ};
fn word(machine: &Machine<'_>, at: u16) -> u16 {
    u16::from_le_bytes([machine.bus().ram(at), machine.bus().ram(at + 1)])
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = reference::read()?;
    let mut machine = Machine::new(reference::validate(&bytes)?);
    machine.reset()?;
    reference::advance(&mut machine, CLOCK_HZ * 3)?;
    for text in [
        "DIM A(1,2),N%(1),S$(1)",
        "A(1,0)=3:A(0,1)=5",
        "N%(1)=-123:S$(1)=\"XYZ\"",
        "B=9",
    ] {
        reference::type_text(&mut machine, text)?;
        reference::type_text(&mut machine, "\r")?;
        reference::advance(&mut machine, CLOCK_HZ)?;
    }
    let start = word(&machine, 0x2f);
    let end = word(&machine, 0x31);
    if end < start || end - start > 256 {
        return Err("Unexpected array observation span".into());
    }
    let data: Vec<_> = (start..end).map(|at| machine.bus().ram(at)).collect();
    println!("start={start}, end={end}, data={data:02x?}");
    bytes.fill(0);
    Ok(())
}
