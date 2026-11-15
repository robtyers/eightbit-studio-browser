//! Original no-I/O source assembler. Input, expressions, passes and output are bounded.
use std::collections::{BTreeMap, BTreeSet};
mod encode;
mod expression;

pub const MAX_SOURCE_BYTES: usize = 1024 * 1024;
const MAX_LINES: usize = 32768;
const MAX_PASSES: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssemblyError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}
impl std::fmt::Display for AssemblyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "S48ASM1001: line {}, column {}: {}",
            self.line, self.column, self.message
        )
    }
}
impl std::error::Error for AssemblyError {}
fn error(line: usize, message: impl Into<String>) -> AssemblyError {
    AssemblyError {
        line,
        column: 1,
        message: message.into(),
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssemblyLine {
    pub source_line: usize,
    pub address: u16,
    pub bytes: u16,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assembly {
    pub load_address: u16,
    pub entry_address: u16,
    pub bytes: Vec<u8>,
    pub symbols: BTreeMap<String, i64>,
    pub lines: Vec<AssemblyLine>,
}
struct Line {
    number: usize,
    label: Option<String>,
    op: String,
    operand: String,
}
fn identifier(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .as_bytes()
            .first()
            .is_some_and(|b| b.is_ascii_alphabetic() || b"_.".contains(b))
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.".contains(&b))
}
fn word(text: &str) -> (&str, &str) {
    let text = text.trim();
    let end = text.find(char::is_whitespace).unwrap_or(text.len());
    (&text[..end], text[end..].trim())
}
fn uncomment(text: &str) -> &str {
    let bytes = text.as_bytes();
    let mut cursor = 0;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b';' => return &text[..cursor],
            b'"' => {
                cursor += 1;
                while cursor < bytes.len() && bytes[cursor] != b'"' {
                    cursor += 1;
                }
            }
            b'\'' if bytes.get(cursor + 2) == Some(&b'\'') => cursor += 2,
            _ => {}
        }
        cursor += 1;
    }
    text
}
fn parse(source: &str) -> Result<Vec<Line>, AssemblyError> {
    if source.len() > MAX_SOURCE_BYTES
        || !source.is_ascii()
        || source
            .bytes()
            .any(|b| b.is_ascii_control() && !b"\r\n\t".contains(&b))
    {
        return Err(error(
            1,
            "Use at most 1 MiB of ASCII assembly without control characters",
        ));
    }
    let mut lines = Vec::new();
    let mut labels = BTreeSet::new();
    for (index, raw) in source.lines().enumerate() {
        if index >= MAX_LINES || raw.len() > 4096 {
            return Err(error(index + 1, "Source line/count limit exceeded"));
        }
        let code = uncomment(raw).trim();
        if code.is_empty() {
            continue;
        }
        let (first, rest) = word(code);
        let (next, remaining) = word(rest);
        let (label, op, operand) = if let Some((name, tail)) = first.split_once(':') {
            let after = format!("{tail} {rest}");
            let (op, operand) = word(&after);
            (
                Some(name.to_ascii_uppercase()),
                op.to_ascii_uppercase(),
                operand.to_owned(),
            )
        } else if next.eq_ignore_ascii_case("equ") || next == "=" {
            (
                Some(first.to_ascii_uppercase()),
                "EQU".into(),
                remaining.to_owned(),
            )
        } else {
            (
                None,
                first.trim_start_matches('.').to_ascii_uppercase(),
                rest.to_owned(),
            )
        };
        if let Some(name) = &label
            && (!identifier(name) || !labels.insert(name.clone()))
        {
            return Err(error(index + 1, "Invalid or duplicate symbol"));
        }
        lines.push(Line {
            number: index + 1,
            label,
            op: op.trim_start_matches('.').to_owned(),
            operand,
        });
        if lines.last().is_some_and(|line| line.op == "END") {
            break;
        }
    }
    Ok(lines)
}

/// Assembles a flat image at the supplied origin; no source can open a file or run a tool.
pub fn assemble(source: &str, origin: u16, entry: Option<&str>) -> Result<Assembly, AssemblyError> {
    let parsed = parse(source)?;
    validate_equations(&parsed)?;
    let mut previous = BTreeMap::new();
    let mut previous_size = usize::MAX;
    let mut converged = false;
    for _ in 0..MAX_PASSES {
        let result = pass(&parsed, origin, &previous, false)?;
        if result.symbols == previous && result.bytes.len() == previous_size {
            converged = true;
            previous = result.symbols;
            break;
        }
        previous_size = result.bytes.len();
        previous = result.symbols;
    }
    if !converged {
        return Err(error(1, "Assembly layout did not converge in 32 passes"));
    }
    let mut result = pass(&parsed, origin, &previous, true)?;
    if result.symbols != previous {
        return Err(error(1, "Unstable symbol definitions"));
    }
    if result.bytes.is_empty() {
        return Err(error(1, "Assembly contains no output"));
    }
    if let Some(name) = entry {
        let address = result
            .symbols
            .get(&name.to_ascii_uppercase())
            .copied()
            .and_then(|v| u16::try_from(v).ok())
            .ok_or_else(|| error(1, "Entry symbol is unresolved or outside the address space"))?;
        if !result.lines.iter().any(|line| line.address == address) {
            return Err(error(
                1,
                "Entry symbol must identify an emitted instruction",
            ));
        }
        result.entry_address = address;
    }
    Ok(result)
}

fn validate_equations(lines: &[Line]) -> Result<(), AssemblyError> {
    let mut dependencies = BTreeMap::new();
    let names: BTreeSet<_> = lines
        .iter()
        .filter(|l| l.op == "EQU")
        .filter_map(|l| l.label.as_ref())
        .collect();
    let mut dependents: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for line in lines.iter().filter(|l| l.op == "EQU") {
        let name = line
            .label
            .as_ref()
            .ok_or_else(|| error(line.number, "EQU requires a symbol"))?;
        let refs = expression::references(&line.operand, line.number)?;
        let refs: Vec<_> = refs.into_iter().filter(|r| names.contains(r)).collect();
        dependencies.insert(name.clone(), refs.len());
        for reference in refs {
            dependents.entry(reference).or_default().push(name.clone());
        }
    }
    let mut ready: Vec<String> = dependencies
        .iter()
        .filter(|(_, n)| **n == 0)
        .map(|(name, _)| name.clone())
        .collect();
    let mut visited = 0;
    while let Some(name) = ready.pop() {
        visited += 1;
        for dependent in dependents.get(&name).into_iter().flatten() {
            if let Some(count) = dependencies.get_mut(dependent) {
                *count -= 1;
                if *count == 0 {
                    ready.push(dependent.clone());
                }
            }
        }
    }
    if visited != dependencies.len() {
        return Err(error(1, "Cyclic EQU definitions"));
    }
    Ok(())
}

fn pass(
    parsed: &[Line],
    origin: u16,
    prior: &BTreeMap<String, i64>,
    final_pass: bool,
) -> Result<Assembly, AssemblyError> {
    let mut symbols = prior.clone();
    let mut result = Assembly {
        load_address: origin,
        entry_address: origin,
        bytes: vec![],
        symbols: BTreeMap::new(),
        lines: vec![],
    };
    for line in parsed {
        if line.op == "END" {
            if !line.operand.trim().is_empty() {
                return Err(error(
                    line.number,
                    "END takes no operands; select an entry symbol through the build API",
                ));
            }
            break;
        }
        let address = usize::from(origin) + result.bytes.len();
        let pc = u16::try_from(address)
            .map_err(|_| error(line.number, "Output exceeds the 64 KiB address space"))?;
        if let Some(label) = &line.label {
            let value = if line.op == "EQU" {
                expression::value(&line.operand, &symbols, pc, line.number, final_pass)?
            } else {
                i64::from(pc)
            };
            symbols.insert(label.clone(), value);
        }
        if line.op.is_empty() || line.op == "EQU" {
            continue;
        }
        let context = Context {
            pc,
            symbols: &symbols,
            line: line.number,
            final_pass,
        };
        let data = matches!(
            line.op.as_str(),
            "DB" | "DEFB"
                | "BYTE"
                | "DW"
                | "DEFW"
                | "WORD"
                | "DS"
                | "DEFS"
                | "BLOCK"
                | "ORG"
                | "ALIGN"
                | "END"
        );
        let bytes = if data {
            context.data(&line.op, &line.operand)?
        } else {
            context.instruction(&line.op, &line.operand)?
        };
        if bytes.len() > 65536_usize.saturating_sub(address) {
            return Err(error(
                line.number,
                "Output exceeds the 64 KiB address space",
            ));
        }
        if !data && !bytes.is_empty() {
            result.lines.push(AssemblyLine {
                source_line: line.number,
                address: pc,
                bytes: u16::try_from(bytes.len())
                    .map_err(|_| error(line.number, "Instruction too large"))?,
            });
        }
        result.bytes.extend(bytes);
    }
    result.symbols = symbols;
    Ok(result)
}

struct Context<'a> {
    pc: u16,
    symbols: &'a BTreeMap<String, i64>,
    line: usize,
    final_pass: bool,
}
impl Context<'_> {
    fn fail(&self, message: impl Into<String>) -> AssemblyError {
        error(self.line, message)
    }
    fn value(&self, text: &str) -> Result<i64, AssemblyError> {
        expression::value(text, self.symbols, self.pc, self.line, self.final_pass)
    }
    fn byte(&self, text: &str) -> Result<u8, AssemblyError> {
        let value = self.value(text)?;
        if self.final_pass && !(-128..=255).contains(&value) {
            return Err(self.fail("Byte value is outside -128..255"));
        }
        Ok(value as u8)
    }
    fn address(&self, text: &str) -> Result<u16, AssemblyError> {
        let value = self.value(text)?;
        if self.final_pass && !(0..=65535).contains(&value) {
            return Err(self.fail("Address is outside 0..65535"));
        }
        Ok(value as u16)
    }
    fn word_value(&self, text: &str) -> Result<u16, AssemblyError> {
        let value = self.value(text)?;
        if self.final_pass && !(-32768..=65535).contains(&value) {
            return Err(self.fail("Word value is outside -32768..65535"));
        }
        Ok(value as u16)
    }
    fn relative(&self, text: &str) -> Result<u8, AssemblyError> {
        let target = self.address(text)?;
        let difference = target.wrapping_sub(self.pc.wrapping_add(2)).cast_signed();
        if self.final_pass && !(-128..=127).contains(&difference) {
            return Err(self.fail("Relative branch exceeds -128..127 bytes"));
        }
        Ok(difference as u8)
    }
    fn operands<'a>(&self, text: &'a str) -> Result<Vec<&'a str>, AssemblyError> {
        if text.trim().is_empty() {
            return Ok(vec![]);
        }
        let bytes = text.as_bytes();
        let mut depth = 0_usize;
        let mut quoted = false;
        let mut start = 0;
        let mut result = Vec::new();
        let mut cursor = 0;
        while cursor < bytes.len() {
            match bytes[cursor] {
                b'"' => quoted = !quoted,
                b'\'' if !quoted && bytes.get(cursor + 2) == Some(&b'\'') => cursor += 2,
                b'(' if !quoted => {
                    depth += 1;
                    if depth > 64 {
                        return Err(self.fail("Operand nesting exceeds 64 levels"));
                    }
                }
                b')' if !quoted => {
                    depth = depth
                        .checked_sub(1)
                        .ok_or_else(|| self.fail("Unbalanced operand"))?;
                }
                b',' if !quoted && depth == 0 => {
                    result.push(text[start..cursor].trim());
                    start = cursor + 1;
                    if result.len() >= 4096 {
                        return Err(self.fail("Too many operands"));
                    }
                }
                _ => {}
            }
            cursor += 1;
        }
        if depth != 0 || quoted {
            return Err(self.fail("Unclosed operand or string"));
        }
        result.push(text[start..].trim());
        if result.contains(&"") {
            return Err(self.fail("Empty operand"));
        }
        Ok(result)
    }
    fn data(&self, op: &str, text: &str) -> Result<Vec<u8>, AssemblyError> {
        let values = self.operands(text)?;
        match op {
            "END" if values.is_empty() => Ok(vec![]),
            "ORG" if values.len() == 1 => {
                let mut address = self.address(values[0])?;
                if !self.final_pass && address < self.pc {
                    address = self.pc;
                }
                let count = address
                    .checked_sub(self.pc)
                    .ok_or_else(|| self.fail("ORG cannot move before the current output"))?;
                Ok(vec![0; usize::from(count)])
            }
            "DS" | "DEFS" | "BLOCK" | "ALIGN" if (1..=2).contains(&values.len()) => {
                let mut count = self.value(values[0])?;
                let count = if op == "ALIGN" {
                    if !self.final_pass && count == 0 {
                        count = 1;
                    }
                    if count <= 0 || count > 65536 {
                        return Err(self.fail("ALIGN requires 1..65536"));
                    }
                    (count - i64::from(self.pc) % count) % count
                } else {
                    count
                };
                if !(0..=65536).contains(&count) {
                    return Err(self.fail("Reservation exceeds the 64 KiB limit"));
                }
                let fill = if values.len() == 2 {
                    self.byte(values[1])?
                } else {
                    0
                };
                Ok(vec![
                    fill;
                    usize::try_from(count)
                        .map_err(|_| self.fail("Invalid reservation"))?
                ])
            }
            "DB" | "DEFB" | "BYTE" | "DW" | "DEFW" | "WORD" if !values.is_empty() => {
                let words = matches!(op, "DW" | "DEFW" | "WORD");
                let mut result = Vec::new();
                for value in values {
                    if !words && value.starts_with('"') && value.ends_with('"') && value.len() >= 2
                    {
                        let text = &value[1..value.len() - 1];
                        if text.contains('"') {
                            return Err(
                                self.fail("Embedded quotes in data strings are unsupported")
                            );
                        }
                        result.extend(text.as_bytes());
                    } else if words {
                        result.extend(self.word_value(value)?.to_le_bytes());
                    } else {
                        result.push(self.byte(value)?);
                    }
                }
                Ok(result)
            }
            _ => Err(self.fail("Invalid data directive or operand count")),
        }
    }
}
