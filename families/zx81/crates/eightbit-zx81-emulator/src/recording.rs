//! Bounded recordings from actual guest input/output signal transitions.
use super::{CPU_HZ, Error, Model, Result, TapeEdge, image::Image};
const MAX_EDGES: usize = 2_500_000;
fn invalid() -> Error {
    Error::Tape("Recording is not a complete standard ZX80/ZX81 SAVE signal")
}
fn ordered(edges: &[TapeEdge]) -> Result<()> {
    if edges.len() < 2 || edges.len() > MAX_EDGES {
        return Err(invalid());
    }
    let start = edges[0].t_state;
    for pair in edges.windows(2) {
        if pair[1].t_state <= pair[0].t_state
            || pair[0].high == pair[1].high
            || pair[1].t_state - start > u64::from(CPU_HZ) * 600
        {
            return Err(invalid());
        }
    }
    Ok(())
}
pub struct Recording {
    pub name: Vec<u8>,
    pub image: Image,
}
pub fn decode(model: Model, edges: &[TapeEdge]) -> Result<Recording> {
    ordered(edges)?;
    let mut start = 0;
    let mut pulse_level = edges[0].high;
    // Ignore ordinary video sync preceding the five-second SAVE silence.
    for (index, pair) in edges.windows(2).enumerate() {
        if pair[1].t_state - pair[0].t_state > u64::from(CPU_HZ) * 2 {
            start = index + 1;
            pulse_level = pair[1].high;
            break;
        }
    }
    let mut bytes = Vec::new();
    let mut name = Vec::new();
    let mut named = model.zx80();
    let mut current = 0_u8;
    let mut bit = 0;
    let mut pulses = 0;
    let mut position = start;
    while position + 2 < edges.len() {
        let rise = edges[position];
        if rise.high != pulse_level {
            position += 1;
            continue;
        }
        let fall = edges[position + 1];
        let next = edges[position + 2];
        if !(250..=800).contains(&(fall.t_state - rise.t_state))
            || fall.high == pulse_level
            || next.high != pulse_level
        {
            return Err(invalid());
        }
        pulses += 1;
        let gap = next.t_state - fall.t_state;
        if gap > 2000 {
            if !matches!(pulses, 4 | 9) {
                return Err(invalid());
            }
            current = (current << 1) | u8::from(pulses == 9);
            pulses = 0;
            bit += 1;
            if bit == 8 {
                if named {
                    bytes.push(current);
                } else {
                    if current & 127 > 63 || name.len() >= 127 {
                        return Err(invalid());
                    }
                    name.push(current & 127);
                    named = current & 128 != 0;
                }
                bit = 0;
                current = 0;
                let pointer = if model.zx80() { 10 } else { 11 };
                if bytes.len() >= pointer + 2 {
                    let end = u16::from_le_bytes([bytes[pointer], bytes[pointer + 1]]);
                    let length = end
                        .checked_sub(model.image_start())
                        .map(usize::from)
                        .ok_or_else(invalid)?;
                    if length > model.ram_bytes()
                        || length < usize::from(model.programme_start() - model.image_start()) + 1
                    {
                        return Err(invalid());
                    }
                    if bytes.len() == length {
                        return Ok(Recording {
                            name,
                            image: Image::parse(model, &bytes)?,
                        });
                    }
                }
                if bytes.len() > model.ram_bytes() {
                    return Err(invalid());
                }
            }
        } else if !(250..=800).contains(&gap) {
            return Err(invalid());
        }
        position += 2;
    }
    Err(invalid())
}
/// Standard mono 8-bit PCM; no firmware/programme bytes are inferred or injected.
pub fn wave(edges: &[TapeEdge]) -> Result<Vec<u8>> {
    ordered(edges)?;
    let start = edges[0].t_state;
    let end = edges.last().ok_or_else(invalid)?.t_state;
    let samples = (end - start) * 44100 / u64::from(CPU_HZ) + 1;
    let length = u32::try_from(samples).map_err(|_| invalid())?;
    let mut out = Vec::with_capacity(44 + usize::try_from(length).map_err(|_| invalid())?);
    out.extend_from_slice(b"RIFF");
    out.extend((36 + length + (length & 1)).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend(16_u32.to_le_bytes());
    out.extend(1_u16.to_le_bytes());
    out.extend(1_u16.to_le_bytes());
    out.extend(44100_u32.to_le_bytes());
    out.extend(44100_u32.to_le_bytes());
    out.extend(1_u16.to_le_bytes());
    out.extend(8_u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend(length.to_le_bytes());
    let mut index = 0;
    for sample in 0..samples {
        let clock = start + sample * u64::from(CPU_HZ) / 44100;
        while index + 1 < edges.len() && edges[index + 1].t_state <= clock {
            index += 1;
        }
        out.push(if edges[index].high { 224 } else { 32 });
    }
    if length & 1 != 0 {
        out.push(0);
    }
    Ok(out)
}
