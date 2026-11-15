use super::{AssemblyError, error};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn value(
    text: &str,
    symbols: &BTreeMap<String, i64>,
    pc: u16,
    line: usize,
    final_pass: bool,
) -> Result<i64, AssemblyError> {
    let (result, _) = evaluate(text, symbols, pc, line, final_pass)?;
    Ok(result)
}
pub(super) fn references(text: &str, line: usize) -> Result<BTreeSet<String>, AssemblyError> {
    Ok(evaluate(text, &BTreeMap::new(), 0, line, false)?.1)
}
fn evaluate(
    text: &str,
    symbols: &BTreeMap<String, i64>,
    pc: u16,
    line: usize,
    final_pass: bool,
) -> Result<(i64, BTreeSet<String>), AssemblyError> {
    let mut parser = Parser {
        text: text.as_bytes(),
        cursor: 0,
        depth: 0,
        symbols,
        pc,
        line,
        final_pass,
        undefined: false,
        references: BTreeSet::new(),
    };
    let result = parser.binary(0)?;
    parser.space();
    if parser.cursor != parser.text.len() {
        return Err(error(line, "Unexpected expression text"));
    }
    Ok((result, parser.references))
}

struct Parser<'a> {
    text: &'a [u8],
    cursor: usize,
    depth: usize,
    symbols: &'a BTreeMap<String, i64>,
    pc: u16,
    line: usize,
    final_pass: bool,
    undefined: bool,
    references: BTreeSet<String>,
}
impl Parser<'_> {
    fn space(&mut self) {
        while self
            .text
            .get(self.cursor)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.cursor += 1;
        }
    }
    fn binary(&mut self, minimum: u8) -> Result<i64, AssemblyError> {
        self.depth += 1;
        if self.depth > 64 {
            return Err(error(self.line, "Expression nesting exceeds 64 levels"));
        }
        let mut left = self.atom()?;
        loop {
            self.space();
            let (operation, width, rank) = match self.text.get(self.cursor..) {
                Some([b'<', b'<', ..]) => (b'<', 2, 4),
                Some([b'>', b'>', ..]) => (b'>', 2, 4),
                Some([b'|', ..]) => (b'|', 1, 1),
                Some([b'^', ..]) => (b'^', 1, 2),
                Some([b'&', ..]) => (b'&', 1, 3),
                Some([b'+', ..]) => (b'+', 1, 5),
                Some([b'-', ..]) => (b'-', 1, 5),
                Some([b'*', ..]) => (b'*', 1, 6),
                Some([b'/', ..]) => (b'/', 1, 6),
                Some([b'%', ..]) => (b'%', 1, 6),
                _ => break,
            };
            if rank < minimum {
                break;
            }
            self.cursor += width;
            let right = self.binary(rank + 1)?;
            let shift = u32::try_from(right).ok().filter(|n| *n < 63);
            let result = match operation {
                b'+' => left.checked_add(right),
                b'-' => left.checked_sub(right),
                b'*' => left.checked_mul(right),
                b'/' => left.checked_div(right),
                b'%' => left.checked_rem(right),
                b'<' => shift.and_then(|n| left.checked_mul(1_i64 << n)),
                b'>' => shift.map(|n| left >> n),
                b'&' => Some(left & right),
                b'|' => Some(left | right),
                _ => Some(left ^ right),
            };
            left = result
                .or_else(|| (!self.final_pass && self.undefined).then_some(0))
                .ok_or_else(|| {
                    error(
                        self.line,
                        "Expression overflow, zero divisor or invalid shift",
                    )
                })?;
        }
        self.depth -= 1;
        Ok(left)
    }
    fn atom(&mut self) -> Result<i64, AssemblyError> {
        self.space();
        let current = self
            .text
            .get(self.cursor)
            .copied()
            .ok_or_else(|| error(self.line, "Expected expression"))?;
        if matches!(current, b'+' | b'-' | b'~' | b'<' | b'>') {
            self.cursor += 1;
            let value = self.binary(7)?;
            return match current {
                b'-' => value
                    .checked_neg()
                    .ok_or_else(|| error(self.line, "Expression overflow")),
                b'~' => Ok(!value),
                b'<' => Ok(value & 255),
                b'>' => Ok((value >> 8) & 255),
                _ => Ok(value),
            };
        }
        if current == b'(' {
            self.cursor += 1;
            let value = self.binary(0)?;
            self.space();
            if self.text.get(self.cursor) != Some(&b')') {
                return Err(error(self.line, "Unclosed expression"));
            }
            self.cursor += 1;
            return Ok(value);
        }
        if current == b'\'' {
            let byte = self
                .text
                .get(self.cursor + 1)
                .copied()
                .ok_or_else(|| error(self.line, "Invalid character literal"))?;
            if self.text.get(self.cursor + 2) != Some(&b'\'') {
                return Err(error(self.line, "Use one ASCII character in quotes"));
            }
            self.cursor += 3;
            return Ok(i64::from(byte));
        }
        if current == b'$'
            && !self
                .text
                .get(self.cursor + 1)
                .is_some_and(u8::is_ascii_hexdigit)
        {
            self.cursor += 1;
            return Ok(i64::from(self.pc));
        }
        if current.is_ascii_digit() || matches!(current, b'$' | b'#' | b'%') {
            let start = self.cursor;
            self.cursor += 1;
            while self
                .text
                .get(self.cursor)
                .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
            {
                self.cursor += 1;
            }
            let raw = std::str::from_utf8(&self.text[start..self.cursor])
                .map_err(|_| error(self.line, "Invalid number"))?
                .replace('_', "");
            let (digits, radix) = if raw.starts_with(['$', '#']) {
                (&raw[1..], 16)
            } else if let Some(digits) = raw.strip_prefix('%') {
                (digits, 2)
            } else if raw.starts_with("0x") || raw.starts_with("0X") {
                (&raw[2..], 16)
            } else if raw.starts_with("0b") || raw.starts_with("0B") {
                (&raw[2..], 2)
            } else if raw.ends_with(['h', 'H']) {
                (&raw[..raw.len() - 1], 16)
            } else {
                (raw.as_str(), 10)
            };
            return i64::from_str_radix(digits, radix)
                .map_err(|_| error(self.line, "Invalid or oversized integer literal"));
        }
        if current.is_ascii_alphabetic() || b"_.".contains(&current) {
            let start = self.cursor;
            self.cursor += 1;
            while self
                .text
                .get(self.cursor)
                .is_some_and(|b| b.is_ascii_alphanumeric() || b"_.".contains(b))
            {
                self.cursor += 1;
            }
            let name = std::str::from_utf8(&self.text[start..self.cursor])
                .map_err(|_| error(self.line, "Invalid symbol"))?
                .to_ascii_uppercase();
            self.references.insert(name.clone());
            self.undefined |= !self.symbols.contains_key(&name);
            return self
                .symbols
                .get(&name)
                .copied()
                .or_else(|| (!self.final_pass).then_some(0))
                .ok_or_else(|| error(self.line, format!("Unresolved symbol {name}")));
        }
        Err(error(self.line, "Unsupported expression"))
    }
}
