//! Original public BASIC-vector callbacks installed by actual guest SYS code.
//! Only RAM vector addresses and non-copyrightable error/behaviour are observed.
use super::{advance, digest, screen, type_text};
use eightbit_commodore_machine::{Firmware, pal::Machine, sid::CLOCK_HZ};
fn command(m: &mut Machine<'_>, s: &str) -> Result<(), Box<dyn std::error::Error>> {
    type_text(m, s)?;
    type_text(m, "\r")?;
    advance(m, CLOCK_HZ * 2)?;
    Ok(())
}
fn word(m: &Machine<'_>, at: u16) -> u16 {
    u16::from_le_bytes([m.bus().ram(at), m.bus().ram(at + 1)])
}
pub fn qualify(firmware: Firmware<'_>) -> Result<String, Box<dyn std::error::Error>> {
    let mut m = Machine::new(firmware);
    let mut trace = Vec::new();
    for vector in (0x300u16..=0x30a).step_by(2) {
        m.reset()?;
        advance(&mut m, CLOCK_HZ * 3)?;
        for s in [
            format!("L=PEEK({vector}):H=PEEK({})", vector + 1),
            "POKE49889,L:POKE49890,H:POKE49888,0".to_owned(),
        ] {
            command(&mut m, &s)?;
        }
        // Save flags/A, count the call, and tail-jump to the saved default.
        let hook = [
            0x8e, 0xdf, 0xc2, 0x08, 0x48, 0xee, 0xe0, 0xc2, 0x68, 0x28, 0x6c, 0xe1, 0xc2,
        ];
        for (i, byte) in hook.into_iter().enumerate() {
            command(&mut m, &format!("POKE{},{}", 49664 + i, byte))?;
        }
        let [lo, hi] = vector.to_le_bytes();
        let [next_lo, next_hi] = (vector + 1).to_le_bytes();
        let installer = [
            0xa9, 0, 0x8d, lo, hi, 0xa9, 0xc2, 0x8d, next_lo, next_hi, 0x60,
        ];
        for (i, byte) in installer.into_iter().enumerate() {
            command(&mut m, &format!("POKE{},{}", 49728 + i, byte))?;
        }
        command(&mut m, "SYS49728")?;
        if vector == 0x300 {
            command(&mut m, "PRINT 1/0")?;
            if m.bus().ram(0xc2df) != 20 {
                return Err(format!("Public division error ID was {}", m.bus().ram(0xc2df)).into());
            }
        } else if vector == 0x306 {
            command(&mut m, "10 PRINT\"HOOK\"")?;
            command(&mut m, "LIST")?;
        } else {
            command(&mut m, "PRINT\"HOOK\";6*7")?;
        }
        if m.bus().ram(0xc2e0) == 0 {
            return Err(format!("BASIC callback {vector:04x} was bypassed\n{}", screen(&m)).into());
        }
        trace.extend(vector.to_le_bytes());
        m.reset()?;
        advance(&mut m, CLOCK_HZ * 3)?;
        if word(&m, vector) == 0xc200 || word(&m, vector) == 0 {
            return Err("Reset did not restore BASIC default vector".into());
        }
    }
    Ok(format!(
        "{{\"schemaVersion\":1,\"keyboardOnly\":true,\"guestMachineCodeInstallers\":true,\"publicBasicCallbacks\":6,\"publishedErrorId\":true,\"resetRestoresVectors\":true,\"traceSha256\":\"{}\"}}",
        digest(&trace)
    ))
}
