//! Native cassette RAM images. No container can add physical guest memory.
use super::{Error, Model, Result};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub model: Model,
    bytes: Vec<u8>,
    pub programme_end: u16,
    pub variables: u16,
    pub end: u16,
}
fn invalid() -> Error {
    Error::Tape("Invalid native O/P image or memory pointers")
}
fn word(bytes: &[u8], offset: usize) -> Result<u16> {
    let value = bytes.get(offset..offset + 2).ok_or_else(invalid)?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}
fn put(bytes: &mut [u8], start: u16, address: u16, value: u16) {
    let offset = usize::from(address - start);
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}
fn lines(model: Model, bytes: &[u8]) -> Result<()> {
    let mut position = 0;
    let mut previous = 0;
    while position < bytes.len() {
        let header_len = if model.zx80() { 2 } else { 4 };
        let header = bytes
            .get(position..position + header_len)
            .ok_or_else(invalid)?;
        let number = u16::from_be_bytes([header[0], header[1]]);
        if number <= previous || number > 9999 {
            return Err(invalid());
        }
        previous = number;
        position += header_len;
        let length = if model.zx80() {
            bytes[position..]
                .iter()
                .position(|byte| *byte == 118)
                .map(|n| n + 1)
                .ok_or_else(invalid)?
        } else {
            usize::from(u16::from_le_bytes([header[2], header[3]]))
        };
        if length == 0
            || bytes
                .get(position..position + length)
                .and_then(|body| body.last())
                != Some(&118)
        {
            return Err(invalid());
        }
        position += length;
    }
    Ok(())
}
fn variables(model: Model, bytes: &[u8]) -> Result<()> {
    let mut p = 0;
    while let Some(tag) = bytes.get(p).copied() {
        if tag == 128 {
            return if p + 1 == bytes.len() {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        if tag & 31 < 6 {
            return Err(invalid());
        }
        let kind = tag & 0xe0;
        let size = if model.zx80() {
            match kind {
                0x60 => 3,
                0xe0 => 7,
                0xa0 => 2 + 2 * (usize::from(*bytes.get(p + 1).ok_or_else(invalid)?) + 1),
                0x80 => bytes
                    .get(p + 1..)
                    .and_then(|body| body.iter().position(|byte| *byte == 1))
                    .map(|n| n + 2)
                    .ok_or_else(invalid)?,
                0x40 => named_length(bytes, p, 2)?,
                _ => return Err(invalid()),
            }
        } else {
            match kind {
                0x60 => 6,
                0xe0 => 18,
                0xa0 => named_length(bytes, p, 5)?,
                0x40 => 3 + usize::from(word(bytes, p + 1)?),
                0x80 | 0xc0 => {
                    let length = usize::from(word(bytes, p + 1)?);
                    let dims = usize::from(*bytes.get(p + 3).ok_or_else(invalid)?);
                    if dims == 0 || length < 1 + 2 * dims {
                        return Err(invalid());
                    }
                    let mut elements = 1_usize;
                    for index in 0..dims {
                        let dim = usize::from(word(bytes, p + 4 + 2 * index)?);
                        if dim == 0 {
                            return Err(invalid());
                        }
                        elements = elements
                            .checked_mul(dim)
                            .filter(|n| *n <= 16384)
                            .ok_or_else(invalid)?;
                    }
                    if length != 1 + 2 * dims + elements * if kind == 0x80 { 5 } else { 1 } {
                        return Err(invalid());
                    }
                    3 + length
                }
                _ => return Err(invalid()),
            }
        };
        p = p
            .checked_add(size)
            .filter(|end| *end < bytes.len())
            .ok_or_else(invalid)?;
    }
    Err(invalid())
}
fn named_length(bytes: &[u8], start: usize, value_bytes: usize) -> Result<usize> {
    let mut p = start + 1;
    loop {
        let byte = *bytes.get(p).ok_or_else(invalid)?;
        if !(28..=63).contains(&(byte & 127)) {
            return Err(invalid());
        }
        p += 1;
        if byte & 128 != 0 {
            return Ok(p - start + value_bytes);
        }
    }
}
impl Image {
    pub fn from_program(model: Model, programme: &[u8]) -> Result<Self> {
        if programme.len() > model.ram_bytes() {
            return Err(invalid());
        }
        lines(model, programme)?;
        let start = model.image_start();
        let programme_start = model.programme_start();
        let programme_end = programme_start
            .checked_add(u16::try_from(programme.len()).map_err(|_| invalid())?)
            .ok_or_else(invalid)?;
        let mut bytes = vec![0; usize::from(programme_start - start)];
        let end;
        let variables;
        if model.zx80() {
            variables = programme_end;
            end = variables.checked_add(1).ok_or_else(invalid)?;
            let display = end.checked_add(1).ok_or_else(invalid)?;
            if u32::from(display) + 25 + 256 > u32::from(model.ram_top()) {
                return Err(invalid());
            }
            bytes[0] = 255;
            bytes[1] = 0x40;
            for (address, value) in [
                (0x4004, end),
                (0x4006, 10),
                (0x4008, variables),
                (0x400a, end),
                (0x400c, display),
                (0x400e, display + 23),
                (0x4010, display + 25),
                (0x4013, 1),
            ] {
                put(&mut bytes, start, address, value);
            }
            bytes[0x12] = 2;
            bytes[0x24] = 33;
            bytes[0x25] = 23;
            bytes.extend_from_slice(programme);
        } else {
            let mut display = vec![118];
            for _ in 0..24 {
                if model.ram_bytes() != 1024 {
                    display.extend([0; 32]);
                }
                display.push(118);
            }
            variables = programme_end
                .checked_add(u16::try_from(display.len()).map_err(|_| invalid())?)
                .ok_or_else(invalid)?;
            end = variables.checked_add(1).ok_or_else(invalid)?;
            if u32::from(end) + 256 > u32::from(model.ram_top()) {
                return Err(invalid());
            }
            for (address, value) in [
                (16394, 10),
                (16396, programme_end),
                (16398, programme_end + 1),
                (16400, variables),
                (16404, end),
                (16406, end),
                (16410, end + 1),
                (16412, end + 1),
                (16415, 16477),
                (16419, 1),
                (16421, 65535),
                (16425, 16509),
                (16436, 32768),
            ] {
                put(&mut bytes, start, address, value);
            }
            for (address, value) in [
                (16418, 2),
                (16423, 15),
                (16424, 55),
                (16440, 188),
                (16441, 33),
                (16442, 24),
                (16443, 128),
                (16476, 118),
            ] {
                bytes[usize::from(address - start)] = value;
            }
            bytes.extend_from_slice(programme);
            bytes.extend(display);
        }
        bytes.push(128);
        Self::parse(model, &bytes)
    }
    pub fn parse(model: Model, bytes: &[u8]) -> Result<Self> {
        if bytes.len() > model.ram_bytes()
            || bytes.len() < usize::from(model.programme_start() - model.image_start()) + 1
        {
            return Err(invalid());
        }
        let (programme_end, variables, end) = if model.zx80() {
            let vars = word(bytes, 8)?;
            let end = word(bytes, 10)?;
            let display = word(bytes, 12)?;
            let lower = word(bytes, 14)?;
            let display_end = word(bytes, 16)?;
            if display <= end
                || display_end < display
                || display_end >= model.ram_top()
                || !(display..=display_end).contains(&lower)
            {
                return Err(invalid());
            }
            (vars, vars, end)
        } else {
            if bytes[0] != 0 {
                return Err(invalid());
            }
            (word(bytes, 3)?, word(bytes, 7)?, word(bytes, 11)?)
        };
        if programme_end < model.programme_start()
            || variables < programme_end
            || end <= variables
            || end >= model.ram_top()
            || usize::from(end - model.image_start()) != bytes.len()
            || bytes.last() != Some(&128)
        {
            return Err(invalid());
        }
        let offset = |address: u16| usize::from(address - model.image_start());
        lines(
            model,
            &bytes[offset(model.programme_start())..offset(programme_end)],
        )?;
        self::variables(model, &bytes[offset(variables)..])?;
        if !model.zx80() {
            let display = &bytes[offset(programme_end)..offset(variables)];
            if display.first() != Some(&118) || display.last() != Some(&118) {
                return Err(invalid());
            }
            let mut row = 0;
            let mut column = 0;
            for byte in &display[1..] {
                if *byte == 118 {
                    row += 1;
                    column = 0;
                } else {
                    column += 1;
                    if column > 32 {
                        return Err(invalid());
                    }
                }
            }
            if row != 24 {
                return Err(invalid());
            }
        }
        Ok(Self {
            model,
            bytes: bytes.into(),
            programme_end,
            variables,
            end,
        })
    }
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    #[must_use]
    pub fn programme(&self) -> &[u8] {
        &self.bytes[usize::from(self.model.programme_start() - self.model.image_start())
            ..usize::from(self.programme_end - self.model.image_start())]
    }
}
