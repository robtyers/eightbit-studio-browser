//! Original bounded 6809 source assembler, sharing the CPU's documented encoding facts.
//! It has no filesystem, external-tool, firmware or family-specific authority.
use std::collections::{BTreeMap, BTreeSet};

mod project;
pub use project::{
    AssemblySource, LinkedAssembly, LinkedAssemblyError, LinkedLine, LinkedSymbol, assemble_project,
};

pub const MAX_SOURCE_BYTES: usize = 1024 * 1024;
const MAX_LINES: usize = 32_768;
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
            "DMA1001: line {}, column {}: {}",
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
    pub symbols: BTreeMap<String, u16>,
    pub lines: Vec<AssemblyLine>,
}

#[derive(Clone)]
struct Line {
    number: usize,
    label: Option<String>,
    operation: String,
    operand: String,
}
fn identifier(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || b"_.".contains(&c))
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_.".contains(&c))
}
fn without_comment(s: &str) -> &str {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b';' {
            return &s[..i];
        }
        if bytes[i] == b'"' {
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
        } else if bytes[i] == b'\'' {
            // Motorola character literals accept 'A as well as 'A'.
            i += 1;
            if bytes.get(i) == Some(&b'\\') {
                i += 1;
            }
            i += 1;
            if bytes.get(i) == Some(&b'\'') {
                i += 1;
            }
            continue;
        }
        i += 1;
    }
    s
}
fn word(s: &str) -> (&str, &str) {
    let s = s.trim();
    let end = s.find(char::is_whitespace).unwrap_or(s.len());
    (&s[..end], s[end..].trim())
}
fn directive(op: &str) -> bool {
    matches!(
        op,
        "ORG"
            | "EQU"
            | "="
            | "SETDP"
            | "FCB"
            | "DB"
            | "BYTE"
            | "FDB"
            | "DW"
            | "WORD"
            | "FCC"
            | "FCS"
            | "RMB"
            | "DS"
            | "ZMB"
            | "FILL"
            | "ALIGN"
            | "SECTION"
            | "ENDSECTION"
            | "EXPORT"
            | "EXTERN"
            | "END"
    )
}
fn parse(source: &str) -> Result<Vec<Line>, AssemblyError> {
    if source.len() > MAX_SOURCE_BYTES || !source.is_ascii() || source.as_bytes().contains(&0) {
        return Err(error(
            1,
            "Use bounded ASCII assembly source without NUL bytes",
        ));
    }
    let mut lines = Vec::new();
    let mut definitions = BTreeSet::new();
    for (index, original) in source.lines().enumerate() {
        let number = index + 1;
        if number > MAX_LINES || original.len() > 4096 {
            return Err(error(
                number,
                "Assembly source exceeds the line or line-length limit",
            ));
        }
        let text = without_comment(original).trim();
        if text.is_empty() || text.starts_with('*') {
            continue;
        }
        let (first, rest) = word(text);
        let first_upper = first.to_ascii_uppercase();
        let (label, operation, operand) = if let Some(label) = first_upper.strip_suffix(':') {
            let (op, operand) = word(rest);
            (
                Some(label.to_owned()),
                op.to_ascii_uppercase(),
                operand.to_owned(),
            )
        } else if !original.starts_with(char::is_whitespace)
            && (directive(&first_upper) || instruction(&first_upper).is_some())
            && is_label_before_opcode(&first_upper, rest, number)
        {
            let (op, operand) = word(rest);
            (
                Some(first_upper),
                op.to_ascii_uppercase(),
                operand.to_owned(),
            )
        } else if directive(&first_upper) || instruction(&first_upper).is_some() {
            (None, first_upper, rest.to_owned())
        } else if !original.starts_with(char::is_whitespace) {
            let (op, operand) = word(rest);
            (
                Some(first_upper),
                op.to_ascii_uppercase(),
                operand.to_owned(),
            )
        } else {
            (None, first_upper, rest.to_owned())
        };
        if let Some(label) = &label
            && (!identifier(label) || !definitions.insert(label.clone()))
        {
            return Err(error(number, "Invalid or duplicate label"));
        }
        if !operation.is_empty() && !directive(&operation) && instruction(&operation).is_none() {
            return Err(error(
                number,
                format!(
                    "Unsupported instruction or directive {operation}; includes and macros are not supported"
                ),
            ));
        }
        lines.push(Line {
            number,
            label,
            operation,
            operand,
        });
    }
    Ok(lines)
}

// Retain permissive unindented instructions, but allow an opcode/directive
// name as a column-zero label when the alternative is not valid operand syntax.
// Ambiguous valid expressions keep their original meaning; a colon is explicit.
fn is_label_before_opcode(first: &str, rest: &str, line: usize) -> bool {
    let (operation, _) = word(rest);
    if instruction(&operation.to_ascii_uppercase()).is_none()
        && !directive(&operation.to_ascii_uppercase())
    {
        return false;
    }
    if first == "END" {
        return false;
    }
    let candidate = Line {
        number: line,
        label: None,
        operation: first.to_owned(),
        operand: rest.to_owned(),
    };
    let empty = BTreeMap::new();
    emit(&[candidate], 0, &empty, false, "", Some(&empty)).is_err()
}

/// Assemble one bounded source into a contiguous image. ORG may add forward
/// padding; the caller's load address and entry symbol remain authoritative.
pub fn assemble(
    source: &str,
    load_address: u16,
    entry_symbol: &str,
) -> Result<Assembly, AssemblyError> {
    let entry = entry_symbol.to_ascii_uppercase();
    if !identifier(&entry) {
        return Err(error(1, "Invalid entry symbol"));
    }
    assemble_inner(source, load_address, &entry)
}
/// Assemble a raw image whose reset/entry mechanism belongs to its machine.
/// END may name a defined address, but it does not override the image load address.
pub fn assemble_image(source: &str, load_address: u16) -> Result<Assembly, AssemblyError> {
    assemble_inner(source, load_address, "")
}
fn assemble_inner(source: &str, load_address: u16, entry: &str) -> Result<Assembly, AssemblyError> {
    let lines = parse(source)?;
    let mut previous = BTreeMap::new();
    let mut sizes = Vec::new();
    for _ in 0..MAX_PASSES {
        let pass = emit(
            &lines,
            u32::from(load_address),
            &previous,
            false,
            entry,
            None,
        )?;
        if pass.symbols == previous && pass.sizes == sizes {
            let final_pass = emit(
                &lines,
                u32::from(load_address),
                &previous,
                true,
                entry,
                None,
            )?;
            let entry_value = if entry.is_empty() {
                i64::from(load_address)
            } else {
                *final_pass
                    .symbols
                    .get(entry)
                    .ok_or_else(|| error(1, format!("Entry symbol {entry} is not defined")))?
            };
            let entry_address = u16::try_from(entry_value)
                .map_err(|_| error(1, "Entry is outside the 16-bit address space"))?;
            if entry_address < load_address
                || usize::from(entry_address - load_address) >= final_pass.bytes.len()
            {
                return Err(error(1, "Entry must lie inside the assembled image"));
            }
            let symbols = final_pass
                .symbols
                .into_iter()
                .filter_map(|(name, value)| u16::try_from(value).ok().map(|value| (name, value)))
                .collect();
            return Ok(Assembly {
                load_address,
                entry_address,
                bytes: final_pass.bytes,
                symbols,
                lines: final_pass.lines,
            });
        }
        previous = pass.symbols;
        sizes = pass.sizes;
    }
    Err(error(
        1,
        "Assembly addresses did not stabilise; use explicit operand sizes or remove circular constants",
    ))
}

struct Pass {
    symbols: BTreeMap<String, i64>,
    bytes: Vec<u8>,
    lines: Vec<AssemblyLine>,
    sizes: Vec<usize>,
}
struct Context<'a> {
    symbols: &'a BTreeMap<String, i64>,
    previous: &'a BTreeMap<String, i64>,
    external: Option<&'a BTreeMap<String, i64>>,
    address: u32,
    line: usize,
    strict: bool,
    dp: u8,
}
impl Context<'_> {
    fn eval(&self, s: &str) -> Result<Option<i64>, AssemblyError> {
        Expression::evaluate(s, self)
    }
    fn value(&self, s: &str) -> Result<i64, AssemblyError> {
        Ok(self.eval(s)?.unwrap_or(0))
    }
    fn range(
        &self,
        value: i64,
        minimum: i64,
        maximum: i64,
        what: &str,
    ) -> Result<(), AssemblyError> {
        if self.strict && !(minimum..=maximum).contains(&value) {
            Err(error(self.line, format!("{what} is out of range")))
        } else {
            Ok(())
        }
    }
}
fn emit(
    lines: &[Line],
    load: u32,
    previous: &BTreeMap<String, i64>,
    strict: bool,
    entry: &str,
    external: Option<&BTreeMap<String, i64>>,
) -> Result<Pass, AssemblyError> {
    let mut out = Pass {
        symbols: BTreeMap::new(),
        bytes: Vec::new(),
        lines: Vec::new(),
        sizes: Vec::new(),
    };
    let mut section: Option<&str> = None;
    let mut ended = false;
    let mut dp = 0;
    let mut exports = Vec::new();
    for line in lines {
        if ended {
            return Err(error(line.number, "Source follows END"));
        }
        let address = load
            + u32::try_from(out.bytes.len()).map_err(|_| error(line.number, "Image too large"))?;
        if let Some(label) = &line.label
            && !matches!(line.operation.as_str(), "EQU" | "=")
        {
            out.symbols.insert(label.clone(), i64::from(address));
        }
        let context = Context {
            symbols: &out.symbols,
            previous,
            external,
            address,
            line: line.number,
            strict,
            dp,
        };
        let op = line.operation.as_str();
        let operand = line.operand.trim();
        let mut bytes = Vec::new();
        if section == Some("_constants")
            && (!matches!(op, "EQU" | "=" | "ENDSECTION")
                || (op == "ENDSECTION" && line.label.is_some()))
        {
            return Err(error(
                line.number,
                "The _constants section permits only EQU definitions",
            ));
        }
        match op {
            "" => {}
            "EQU" | "=" => {
                let name = line
                    .label
                    .as_ref()
                    .ok_or_else(|| error(line.number, "EQU requires a label"))?;
                if let Some(value) = context.eval(operand)? {
                    out.symbols.insert(name.clone(), value);
                }
            }
            "SETDP" => {
                let value = context.value(operand)?;
                context.range(value, 0, 255, "Direct page")?;
                dp = value.to_be_bytes()[7];
            }
            "SECTION" => {
                if section.is_some() {
                    return Err(error(line.number, "Sections cannot be nested"));
                }
                section = if operand.eq_ignore_ascii_case("code") {
                    Some("code")
                } else if operand.eq_ignore_ascii_case("_constants") {
                    Some("_constants")
                } else {
                    return Err(error(
                        line.number,
                        "Only code and EQU-only _constants sections are supported",
                    ));
                };
            }
            "ENDSECTION" => {
                if section.is_none() || !operand.is_empty() {
                    return Err(error(line.number, "ENDSECTION has no matching section"));
                }
                section = None;
            }
            "EXTERN" => {
                if external.is_none() {
                    return Err(error(
                        line.number,
                        "EXTERN requires assemble_project with explicit source units",
                    ));
                }
                for name in split_operands(operand, line.number)? {
                    if !identifier(name) {
                        return Err(error(line.number, "EXTERN requires symbol names"));
                    }
                }
            }
            "EXPORT" => {
                for name in split_operands(operand, line.number)? {
                    if !identifier(name) {
                        return Err(error(line.number, "EXPORT requires symbol names"));
                    }
                    exports.push((line.number, name.to_ascii_uppercase()));
                }
            }
            "END" => {
                if !entry.is_empty() && !operand.is_empty() && !operand.eq_ignore_ascii_case(entry)
                {
                    return Err(error(
                        line.number,
                        "END must use the configured entry symbol",
                    ));
                }
                if entry.is_empty() && !operand.is_empty() {
                    let _ = context.value(operand)?;
                }
                ended = true;
            }
            "ORG" | "RMB" | "DS" | "ZMB" | "ALIGN" => {
                let value = context.eval(operand)?.unwrap_or(if op == "ORG" {
                    i64::from(address)
                } else {
                    0
                });
                let count = if op == "ORG" {
                    if !(0..=65_535).contains(&value) {
                        return Err(error(line.number, "ORG requires a 16-bit address"));
                    }
                    // An unresolved direct operand can temporarily occupy one
                    // extra byte. Enforce backwards ORG only after relaxation.
                    let count = value - i64::from(address);
                    if strict { count } else { count.max(0) }
                } else if op == "ALIGN" {
                    if !(1..=65_536).contains(&value) {
                        return Err(error(
                            line.number,
                            "ALIGN requires a known positive boundary up to 65536",
                        ));
                    }
                    (value - i64::from(address) % value) % value
                } else {
                    value
                };
                if !(0..=65_536).contains(&count) {
                    return Err(error(
                        line.number,
                        "Storage/ORG must advance within the 16-bit address space",
                    ));
                }
                bytes.resize(
                    usize::try_from(count)
                        .map_err(|_| error(line.number, "Invalid storage size"))?,
                    0,
                );
            }
            "FILL" => {
                let parts = split_operands(operand, line.number)?;
                if parts.len() != 2 {
                    return Err(error(line.number, "FILL requires value,count"));
                }
                let value = context.value(parts[0])?;
                context.range(value, -128, 255, "Byte")?;
                let count = context.value(parts[1])?;
                if !(0..=65_536).contains(&count) {
                    return Err(error(line.number, "Invalid FILL count"));
                }
                bytes.resize(
                    usize::try_from(count).map_err(|_| error(line.number, "Invalid FILL count"))?,
                    value.to_be_bytes()[7],
                );
            }
            "FCB" | "DB" | "BYTE" | "FDB" | "DW" | "WORD" | "FCC" | "FCS" => {
                let wide = matches!(op, "FDB" | "DW" | "WORD");
                for part in split_operands(operand, line.number)? {
                    if part.starts_with('"') {
                        if wide {
                            return Err(error(
                                line.number,
                                "Word data requires numerical expressions",
                            ));
                        }
                        bytes.extend(string_bytes(part, line.number)?);
                    } else {
                        if matches!(op, "FCC" | "FCS") {
                            return Err(error(line.number, "FCC requires a double-quoted string"));
                        }
                        let value = context.value(part)?;
                        context.range(
                            value,
                            if wide { -32768 } else { -128 },
                            if wide { 65535 } else { 255 },
                            "Data value",
                        )?;
                        if wide {
                            bytes.extend(&value.to_be_bytes()[6..]);
                        } else {
                            bytes.push(value.to_be_bytes()[7]);
                        }
                    }
                }
            }
            _ => {
                bytes = encode(
                    instruction(op).ok_or_else(|| error(line.number, "Unsupported instruction"))?,
                    operand,
                    &context,
                )?;
            }
        }
        if op == "FCS" {
            let last = bytes
                .last_mut()
                .ok_or_else(|| error(line.number, "FCS requires non-empty string data"))?;
            *last |= 0x80;
        }
        if u64::from(load) + out.bytes.len() as u64 + bytes.len() as u64 > 65_536 {
            return Err(error(
                line.number,
                "Image crosses the end of the 16-bit address space",
            ));
        }
        if !bytes.is_empty() && (external.is_some() || instruction(op).is_some()) {
            for (offset, chunk) in bytes.chunks(usize::from(u16::MAX)).enumerate() {
                let address = u64::from(address) + (offset * usize::from(u16::MAX)) as u64;
                out.lines.push(AssemblyLine {
                    source_line: line.number,
                    address: u16::try_from(address)
                        .map_err(|_| error(line.number, "Emission outside address space"))?,
                    bytes: u16::try_from(chunk.len())
                        .map_err(|_| error(line.number, "Emission too large"))?,
                });
            }
        }
        out.sizes.push(bytes.len());
        out.bytes.extend(bytes);
    }
    if section.is_some() {
        return Err(error(
            lines.last().map_or(1, |line| line.number),
            "Missing ENDSECTION",
        ));
    }
    if strict {
        for (line, name) in exports {
            if !out.symbols.contains_key(&name) {
                return Err(error(
                    line,
                    format!("Exported symbol {name} is not defined"),
                ));
            }
        }
    }
    Ok(out)
}

fn split_operands(s: &str, line: usize) -> Result<Vec<&str>, AssemblyError> {
    let parts = operand_fields(s, line)?;
    if parts.iter().any(|part| part.is_empty()) {
        return Err(error(line, "Missing operand"));
    }
    Ok(parts)
}
fn operand_fields(s: &str, line: usize) -> Result<Vec<&str>, AssemblyError> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut quoted = false;
    let mut escaped = false;
    let input = s.as_bytes();
    let mut index = 0;
    while index < input.len() {
        let c = input[index];
        if escaped {
            escaped = false;
            index += 1;
            continue;
        }
        if quoted && c == b'\\' {
            escaped = true;
            index += 1;
            continue;
        }
        if !quoted && c == b'\'' {
            index += 1;
            if input.get(index) == Some(&b'\\') {
                index += 1;
            }
            index += 1;
            if input.get(index) == Some(&b'\'') {
                index += 1;
            }
            continue;
        }
        if c == b'"' {
            quoted = !quoted;
        }
        if c == b',' && !quoted {
            out.push(s[start..index].trim());
            start = index + 1;
        }
        index += 1;
    }
    out.push(s[start..].trim());
    if quoted {
        return Err(error(line, "Unterminated string"));
    }
    Ok(out)
}
fn string_bytes(s: &str, line: usize) -> Result<Vec<u8>, AssemblyError> {
    let inside = s
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .ok_or_else(|| error(line, "Unterminated string"))?;
    let mut out = Vec::new();
    let mut chars = inside.bytes();
    while let Some(c) = chars.next() {
        if c == b'"' {
            return Err(error(line, "Unexpected quote in string"));
        }
        out.push(if c == b'\\' {
            escaped_byte(
                chars
                    .next()
                    .ok_or_else(|| error(line, "Incomplete escape"))?,
                line,
            )?
        } else {
            c
        });
    }
    Ok(out)
}
fn escaped_byte(c: u8, line: usize) -> Result<u8, AssemblyError> {
    match c {
        b'n' => Ok(10),
        b'r' => Ok(13),
        b't' => Ok(9),
        b'0' => Ok(0),
        b'\\' | b'"' | b'\'' => Ok(c),
        _ => Err(error(line, "Unsupported character escape")),
    }
}

#[derive(Clone, Copy)]
enum Instruction {
    Inherent(u16),
    Immediate(u8),
    Branch(u16, bool),
    Registers(u8),
    Stack(u8),
    Indexed(u8),
    Memory {
        base: u16,
        wide: bool,
        immediate: bool,
        unary: bool,
    },
}
fn instruction(op: &str) -> Option<Instruction> {
    use Instruction::{Branch, Immediate, Indexed, Inherent, Memory, Registers, Stack};
    let inherent = match op {
        "NOP" => 0x12,
        "SYNC" => 0x13,
        "DAA" => 0x19,
        "SEX" => 0x1d,
        "RTS" => 0x39,
        "ABX" => 0x3a,
        "RTI" => 0x3b,
        "MUL" => 0x3d,
        "SWI" => 0x3f,
        "SWI2" => 0x103f,
        "SWI3" => 0x113f,
        _ => 0,
    };
    if inherent != 0 {
        return Some(Inherent(inherent));
    }
    match op {
        "ORCC" => return Some(Immediate(0x1a)),
        "ANDCC" => return Some(Immediate(0x1c)),
        "CWAI" => return Some(Immediate(0x3c)),
        "EXG" => return Some(Registers(0x1e)),
        "TFR" => return Some(Registers(0x1f)),
        "PSHS" => return Some(Stack(0x34)),
        "PULS" => return Some(Stack(0x35)),
        "PSHU" => return Some(Stack(0x36)),
        "PULU" => return Some(Stack(0x37)),
        "LEAX" => return Some(Indexed(0x30)),
        "LEAY" => return Some(Indexed(0x31)),
        "LEAS" => return Some(Indexed(0x32)),
        "LEAU" => return Some(Indexed(0x33)),
        "LBRA" => return Some(Branch(0x16, true)),
        "LBSR" => return Some(Branch(0x17, true)),
        "BSR" => return Some(Branch(0x8d, false)),
        _ => {}
    }
    let (branch, long) = op.strip_prefix('L').map_or((op, false), |s| (s, true));
    let branch = match branch {
        "BHS" => "BCC",
        "BLO" => "BCS",
        other => other,
    };
    const BRANCHES: [&str; 16] = [
        "BRA", "BRN", "BHI", "BLS", "BCC", "BCS", "BNE", "BEQ", "BVC", "BVS", "BPL", "BMI", "BGE",
        "BLT", "BGT", "BLE",
    ];
    if let Some(index) = BRANCHES.iter().position(|name| *name == branch) {
        let code = 0x20 + u16::try_from(index).ok()?;
        return Some(Branch(if long { 0x1000 | code } else { code }, long));
    }
    let unary = [
        ("NEG", 0),
        ("COM", 3),
        ("LSR", 4),
        ("ROR", 6),
        ("ASR", 7),
        ("ASL", 8),
        ("LSL", 8),
        ("ROL", 9),
        ("DEC", 10),
        ("INC", 12),
        ("TST", 13),
        ("JMP", 14),
        ("CLR", 15),
    ];
    for (name, low) in unary {
        if op == name {
            return Some(Memory {
                base: low,
                wide: false,
                immediate: false,
                unary: true,
            });
        }
        if name != "JMP" {
            if op.strip_suffix('A') == Some(name) {
                return Some(Inherent(0x40 + low));
            }
            if op.strip_suffix('B') == Some(name) {
                return Some(Inherent(0x50 + low));
            }
        }
    }
    const A: [&str; 16] = [
        "SUBA", "CMPA", "SBCA", "SUBD", "ANDA", "BITA", "LDA", "STA", "EORA", "ADCA", "ORA",
        "ADDA", "CMPX", "JSR", "LDX", "STX",
    ];
    const B: [&str; 16] = [
        "SUBB", "CMPB", "SBCB", "ADDD", "ANDB", "BITB", "LDB", "STB", "EORB", "ADCB", "ORB",
        "ADDB", "LDD", "STD", "LDU", "STU",
    ];
    for (names, base) in [(&A, 0x80), (&B, 0xc0)] {
        if let Some(low) = names.iter().position(|name| *name == op) {
            return Some(Memory {
                base: base + u16::try_from(low).ok()?,
                wide: matches!(low, 3 | 12 | 14 | 15) || (base == 0xc0 && low == 13),
                immediate: !matches!(op, "STA" | "STB" | "STD" | "STX" | "STU" | "JSR"),
                unary: false,
            });
        }
    }
    let base = match op {
        "CMPD" => 0x1083,
        "CMPY" => 0x108c,
        "LDY" => 0x108e,
        "STY" => 0x108f,
        "LDS" => 0x10ce,
        "STS" => 0x10cf,
        "CMPU" => 0x1183,
        "CMPS" => 0x118c,
        _ => return None,
    };
    Some(Memory {
        base,
        wide: true,
        immediate: !matches!(op, "STY" | "STS"),
        unary: false,
    })
}
fn opcode(code: u16) -> Vec<u8> {
    let bytes = code.to_be_bytes();
    if bytes[0] == 0 {
        vec![bytes[1]]
    } else {
        bytes.to_vec()
    }
}
fn encode(
    instruction: Instruction,
    operand: &str,
    context: &Context<'_>,
) -> Result<Vec<u8>, AssemblyError> {
    use Instruction::{Branch, Immediate, Indexed, Inherent, Memory, Registers, Stack};
    match instruction {
        Inherent(code) => {
            if !operand.is_empty() {
                return Err(error(context.line, "This instruction has no operand"));
            }
            Ok(opcode(code))
        }
        Immediate(code) => {
            let text = operand
                .strip_prefix('#')
                .ok_or_else(|| error(context.line, "Immediate operand requires #"))?;
            let value = context.value(text)?;
            context.range(value, -128, 255, "Immediate byte")?;
            Ok(vec![code, value.to_be_bytes()[7]])
        }
        Branch(code, wide) => {
            let mut out = opcode(code);
            let value = context.value(operand)?;
            context.range(value, 0, 65535, "Branch target")?;
            let next = context.address
                + u32::try_from(out.len()).map_err(|_| error(context.line, "Invalid opcode"))?
                + if wide { 2 } else { 1 };
            let target = u16::from_be_bytes([value.to_be_bytes()[6], value.to_be_bytes()[7]]);
            let next =
                u16::try_from(next & 0xffff).map_err(|_| error(context.line, "Invalid branch"))?;
            let delta = i16::from_be_bytes(target.wrapping_sub(next).to_be_bytes());
            if wide {
                out.extend(delta.to_be_bytes());
            } else {
                context.range(
                    i64::from(delta),
                    -128,
                    127,
                    "Short branch; use a long branch",
                )?;
                out.push(delta.to_be_bytes()[1]);
            }
            Ok(out)
        }
        Registers(code) => {
            let regs = split_operands(operand, context.line)?;
            if regs.len() != 2 {
                return Err(error(context.line, "TFR/EXG requires two registers"));
            }
            let from = register(regs[0]).ok_or_else(|| error(context.line, "Unknown register"))?;
            let to = register(regs[1]).ok_or_else(|| error(context.line, "Unknown register"))?;
            if (from < 8) != (to < 8) {
                return Err(error(context.line, "Register widths must match"));
            }
            Ok(vec![code, from << 4 | to])
        }
        Stack(code) => {
            let mut mask = 0;
            for reg in split_operands(operand, context.line)? {
                mask |= match reg.to_ascii_uppercase().as_str() {
                    "CC" => 1,
                    "A" => 2,
                    "B" => 4,
                    "D" => 6,
                    "DP" => 8,
                    "X" => 16,
                    "Y" => 32,
                    "U" if code < 0x36 => 64,
                    "S" if code >= 0x36 => 64,
                    "PC" => 128,
                    _ => {
                        return Err(error(
                            context.line,
                            "Invalid register for this stack instruction",
                        ));
                    }
                };
            }
            Ok(vec![code, mask])
        }
        Indexed(code) => {
            let mut out = vec![code];
            out.extend(indexed(operand, context, 1)?);
            Ok(out)
        }
        Memory {
            base,
            wide,
            immediate,
            unary,
        } => {
            if let Some(text) = operand.strip_prefix('#') {
                if !immediate {
                    return Err(error(
                        context.line,
                        "This instruction has no immediate form",
                    ));
                }
                let mut out = opcode(base);
                let value = context.value(text)?;
                context.range(
                    value,
                    if wide { -32768 } else { -128 },
                    if wide { 65535 } else { 255 },
                    "Immediate value",
                )?;
                if wide {
                    out.extend(&value.to_be_bytes()[6..]);
                } else {
                    out.push(value.to_be_bytes()[7]);
                }
                return Ok(out);
            }
            if operand.starts_with('[') || operand_fields(operand, context.line)?.len() > 1 {
                let mut out = opcode(base + if unary { 0x60 } else { 0x20 });
                out.extend(indexed(operand, context, out.len())?);
                return Ok(out);
            }
            let (forced, text) = if let Some(s) = operand.strip_prefix('<') {
                (Some(true), s)
            } else if let Some(s) = operand.strip_prefix('>') {
                (Some(false), s)
            } else {
                (None, operand)
            };
            let value = context.eval(text)?;
            let direct = forced.unwrap_or(
                value.is_some_and(|v| (0..=65535).contains(&v) && v >> 8 == i64::from(context.dp)),
            );
            let value = value.unwrap_or(0);
            context.range(value, 0, 65535, "Memory address")?;
            let mut out = opcode(
                base + if unary {
                    if direct { 0 } else { 0x70 }
                } else if direct {
                    0x10
                } else {
                    0x30
                },
            );
            if direct {
                out.push(value.to_be_bytes()[7]);
            } else {
                out.extend(&value.to_be_bytes()[6..]);
            }
            Ok(out)
        }
    }
}
fn register(s: &str) -> Option<u8> {
    match s.to_ascii_uppercase().as_str() {
        "D" => Some(0),
        "X" => Some(1),
        "Y" => Some(2),
        "U" => Some(3),
        "S" => Some(4),
        "PC" => Some(5),
        "A" => Some(8),
        "B" => Some(9),
        "CC" => Some(10),
        "DP" => Some(11),
        _ => None,
    }
}

fn indexed(operand: &str, c: &Context<'_>, opcode_len: usize) -> Result<Vec<u8>, AssemblyError> {
    let (indirect, text) = if operand.starts_with('[') {
        (
            true,
            operand
                .strip_prefix('[')
                .and_then(|s| s.strip_suffix(']'))
                .ok_or_else(|| error(c.line, "Unmatched indirect brackets"))?
                .trim(),
        )
    } else {
        (false, operand)
    };
    // Any remaining syntax brackets are rejected by expression/register
    // parsing; brackets inside character literals are ordinary byte values.
    let fields = operand_fields(text, c.line)?;
    if fields.len() == 1 {
        if !indirect {
            return Err(error(c.line, "Indexed operand requires a register"));
        }
        if text.starts_with('<') {
            return Err(error(
                c.line,
                "Extended indirect addressing requires a 16-bit address",
            ));
        }
        let value = c.value(text.strip_prefix('>').unwrap_or(text))?;
        c.range(value, 0, 65535, "Indirect address")?;
        return Ok(vec![0x9f, value.to_be_bytes()[6], value.to_be_bytes()[7]]);
    }
    let [offset, register] = fields.as_slice() else {
        return Err(error(
            c.line,
            "Indexed operand requires exactly one register",
        ));
    };
    let mut offset = offset.trim();
    let register = register.trim().to_ascii_uppercase();
    let forced = if let Some(s) = offset.strip_prefix("<<") {
        offset = s;
        Some(5)
    } else if let Some(s) = offset.strip_prefix('<') {
        offset = s;
        Some(8)
    } else if let Some(s) = offset.strip_prefix('>') {
        offset = s;
        Some(16)
    } else {
        None
    };
    if register == "PCR" || register == "PC" {
        if offset.is_empty() || forced == Some(5) {
            return Err(error(
                c.line,
                "PC relative indexing requires an expression and 8- or 16-bit displacement",
            ));
        }
        let target = c.eval(offset)?;
        if register == "PCR" {
            c.range(target.unwrap_or(0), 0, 65535, "PC relative target")?;
        }
        let prefix =
            u32::try_from(opcode_len).map_err(|_| error(c.line, "Invalid instruction length"))?;
        let delta = |extra: u32| -> i64 {
            if register == "PC" {
                target.unwrap_or(0)
            } else {
                // The CPU adds the signed displacement modulo 65536, just
                // like a branch. Convert only after strict target validation.
                let target = target.unwrap_or(i64::from(c.address)).to_be_bytes();
                let next = (c.address + prefix + extra).to_be_bytes();
                let target = u16::from_be_bytes([target[6], target[7]]);
                let next = u16::from_be_bytes([next[2], next[3]]);
                i64::from(i16::from_be_bytes(target.wrapping_sub(next).to_be_bytes()))
            }
        };
        let short = forced.map_or(
            target.is_some() && (-128..=127).contains(&delta(2)),
            |width| width == 8,
        );
        let displacement = delta(if short { 2 } else { 3 });
        c.range(
            displacement,
            if short { -128 } else { -32768 },
            if short { 127 } else { 32767 },
            "PC relative displacement",
        )?;
        let mut out = vec![if short { 0x8c } else { 0x8d } | if indirect { 0x10 } else { 0 }];
        if short {
            out.push(displacement.to_be_bytes()[7]);
        } else {
            out.extend(&displacement.to_be_bytes()[6..]);
        }
        return Ok(out);
    }
    let core = register.trim_start_matches('-').trim_end_matches('+');
    let reg = match core {
        "X" => 0,
        "Y" => 0x20,
        "U" => 0x40,
        "S" => 0x60,
        _ => return Err(error(c.line, "Indexed register must be X, Y, U, S or PCR")),
    };
    let auto = match register.as_str() {
        s if s == format!("{core}+") => Some(0),
        s if s == format!("{core}++") => Some(1),
        s if s == format!("-{core}") => Some(2),
        s if s == format!("--{core}") => Some(3),
        s if s == core => None,
        _ => return Err(error(c.line, "Invalid indexed auto increment/decrement")),
    };
    if let Some(mode) = auto {
        if !offset.is_empty() || forced.is_some() || (indirect && mode % 2 == 0) {
            return Err(error(
                c.line,
                "Invalid offset or indirect auto increment/decrement",
            ));
        }
        return Ok(vec![0x80 | reg | mode | if indirect { 0x10 } else { 0 }]);
    }
    let accumulator = match offset.to_ascii_uppercase().as_str() {
        "A" => Some(6),
        "B" => Some(5),
        "D" => Some(11),
        _ => None,
    };
    if let Some(mode) = accumulator {
        if forced.is_some() {
            return Err(error(c.line, "Accumulator indexing has no size prefix"));
        }
        return Ok(vec![0x80 | reg | mode | if indirect { 0x10 } else { 0 }]);
    }
    if offset.is_empty() {
        if forced.is_some() {
            return Err(error(c.line, "Indexed size prefix requires an offset"));
        }
        return Ok(vec![0x84 | reg | if indirect { 0x10 } else { 0 }]);
    }
    let value = c.eval(offset)?;
    let width = forced.unwrap_or_else(|| {
        if !indirect && value.is_some_and(|v| (-16..=15).contains(&v)) {
            5
        } else if value.is_some_and(|v| (-128..=127).contains(&v)) {
            8
        } else {
            16
        }
    });
    let value = value.unwrap_or(0);
    if width == 5 {
        if indirect {
            return Err(error(c.line, "Indirect indexing has no 5-bit form"));
        }
        c.range(value, -16, 15, "5-bit indexed offset")?;
        return Ok(vec![reg | (value.to_be_bytes()[7] & 31)]);
    }
    c.range(
        value,
        if width == 8 { -128 } else { -32768 },
        if width == 8 { 127 } else { 65535 },
        "Indexed offset",
    )?;
    let mut out =
        vec![0x80 | reg | if width == 8 { 8 } else { 9 } | if indirect { 0x10 } else { 0 }];
    if width == 8 {
        out.push(value.to_be_bytes()[7]);
    } else {
        out.extend(&value.to_be_bytes()[6..]);
    }
    Ok(out)
}

struct Expression<'a, 'b> {
    text: &'a [u8],
    position: usize,
    context: &'b Context<'b>,
    depth: usize,
    operations: usize,
}
impl Expression<'_, '_> {
    fn evaluate(s: &str, c: &Context<'_>) -> Result<Option<i64>, AssemblyError> {
        if s.len() > 512 || s.trim().is_empty() {
            return Err(error(c.line, "Missing or oversized expression"));
        }
        let mut parser = Expression {
            text: s.as_bytes(),
            position: 0,
            context: c,
            depth: 0,
            operations: 0,
        };
        let value = parser.binary(0)?;
        parser.space();
        if parser.position != parser.text.len() {
            return Err(error(c.line, "Unexpected expression text"));
        }
        Ok(value)
    }
    fn space(&mut self) {
        while self
            .text
            .get(self.position)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.position += 1;
        }
    }
    fn binary(&mut self, minimum: u8) -> Result<Option<i64>, AssemblyError> {
        self.depth += 1;
        if self.depth > 32 {
            return Err(error(self.context.line, "Expression nesting exceeds 32"));
        }
        let mut left = self.atom()?;
        loop {
            self.space();
            let tail = &self.text[self.position..];
            let (op, precedence, length) = if tail.starts_with(b"<<") {
                ("<<", 4, 2)
            } else if tail.starts_with(b">>") {
                (">>", 4, 2)
            } else {
                match tail.first() {
                    Some(b'|') => ("|", 1, 1),
                    Some(b'^') => ("^", 2, 1),
                    Some(b'&') => ("&", 3, 1),
                    Some(b'+') => ("+", 5, 1),
                    Some(b'-') => ("-", 5, 1),
                    Some(b'*') => ("*", 6, 1),
                    Some(b'/') => ("/", 6, 1),
                    Some(b'%') => ("%", 6, 1),
                    _ => break,
                }
            };
            if precedence < minimum {
                break;
            }
            self.position += length;
            self.operations += 1;
            if self.operations > 128 {
                return Err(error(
                    self.context.line,
                    "Expression has too many operations",
                ));
            }
            let right = self.binary(precedence + 1)?;
            left = match (left, right) {
                (Some(a), Some(b)) => Some(
                    match op {
                        "+" => a.checked_add(b),
                        "-" => a.checked_sub(b),
                        "*" => a.checked_mul(b),
                        "/" => a.checked_div(b),
                        "%" => a.checked_rem(b),
                        "&" => Some(a & b),
                        "|" => Some(a | b),
                        "^" => Some(a ^ b),
                        "<<" => u32::try_from(b).ok().and_then(|n| a.checked_shl(n)),
                        ">>" => u32::try_from(b).ok().and_then(|n| a.checked_shr(n)),
                        _ => None,
                    }
                    .ok_or_else(|| {
                        error(self.context.line, "Expression overflows or divides by zero")
                    })?,
                ),
                _ => None,
            };
        }
        self.depth -= 1;
        Ok(left)
    }
    fn atom(&mut self) -> Result<Option<i64>, AssemblyError> {
        self.space();
        let start = self.position;
        let first = *self
            .text
            .get(start)
            .ok_or_else(|| error(self.context.line, "Missing expression value"))?;
        if matches!(first, b'+' | b'-' | b'~' | b'<' | b'>') {
            self.position += 1;
            self.depth += 1;
            if self.depth > 32 {
                return Err(error(self.context.line, "Expression nesting exceeds 32"));
            }
            let value = self.atom()?;
            self.depth -= 1;
            return value
                .map(|value| match first {
                    b'-' => value
                        .checked_neg()
                        .ok_or_else(|| error(self.context.line, "Expression overflow")),
                    b'~' => Ok(!value),
                    b'<' => Ok(value & 255),
                    b'>' => Ok((value >> 8) & 255),
                    _ => Ok(value),
                })
                .transpose();
        }
        if first == b'(' {
            self.position += 1;
            let value = self.binary(0)?;
            self.space();
            if self.text.get(self.position) != Some(&b')') {
                return Err(error(self.context.line, "Missing closing parenthesis"));
            }
            self.position += 1;
            return Ok(value);
        }
        if first == b'*' {
            self.position += 1;
            return Ok(Some(i64::from(self.context.address)));
        }
        if first == b'\'' {
            self.position += 1;
            let mut value = *self
                .text
                .get(self.position)
                .ok_or_else(|| error(self.context.line, "Missing character literal"))?;
            self.position += 1;
            if value == b'\\' {
                value = escaped_byte(
                    *self
                        .text
                        .get(self.position)
                        .ok_or_else(|| error(self.context.line, "Missing character escape"))?,
                    self.context.line,
                )?;
                self.position += 1;
            }
            if self.text.get(self.position) == Some(&b'\'') {
                self.position += 1;
            }
            return Ok(Some(i64::from(value)));
        }
        let (radix, prefix) = if first == b'$' {
            (16, 1)
        } else if first == b'%' {
            (2, 1)
        } else if first == b'@' {
            (8, 1)
        } else if self.text[start..].starts_with(b"0x") || self.text[start..].starts_with(b"0X") {
            (16, 2)
        } else {
            (10, 0)
        };
        if prefix > 0 || first.is_ascii_digit() {
            self.position += prefix;
            let digits = self.position;
            while self
                .text
                .get(self.position)
                .is_some_and(u8::is_ascii_hexdigit)
            {
                self.position += 1;
            }
            let text = std::str::from_utf8(&self.text[digits..self.position])
                .map_err(|_| error(self.context.line, "Invalid numeric literal"))?;
            let value = i64::from_str_radix(text, radix)
                .map_err(|_| error(self.context.line, "Invalid or overflowing numeric literal"))?;
            return Ok(Some(value));
        }
        while self
            .text
            .get(self.position)
            .is_some_and(|c| c.is_ascii_alphanumeric() || b"_.".contains(c))
        {
            self.position += 1;
        }
        let name = std::str::from_utf8(&self.text[start..self.position])
            .map_err(|_| error(self.context.line, "Invalid symbol"))?
            .to_ascii_uppercase();
        if !identifier(&name) {
            return Err(error(self.context.line, "Invalid expression value"));
        }
        let value = self
            .context
            .symbols
            .get(&name)
            .or_else(|| self.context.previous.get(&name))
            .or_else(|| self.context.external.and_then(|symbols| symbols.get(&name)))
            .copied();
        if self.context.strict && value.is_none() {
            return Err(error(self.context.line, format!("Undefined symbol {name}")));
        }
        Ok(value)
    }
}
