//! Original bounded token-stream encoder. Machine dialect tables are injected by families.
#![allow(missing_docs)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
use eightbit_source_core::{Result, error};
pub struct Dialect {
    pub tokens: &'static [(&'static str, u8)],
    pub extra_tokens: &'static [(&'static str, u8)],
    pub escaped_quote: Option<u8>,
    pub encode: fn(u8) -> Option<u8>,
    pub number_marker: u8,
    pub newline: u8,
    pub max_line: u16,
    pub statements: bool,
}
/// Sinclair five-byte floating point; zero is common to both native BASIC dialects.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
pub fn number(value: f64) -> Result<[u8; 5]> {
    if !value.is_finite() || value < 0.0 {
        return Err(error(
            "DSS1110",
            "Numeric literal is outside the native BASIC range",
        ));
    }
    if value == 0.0 {
        return Ok([0; 5]);
    }
    let mut exponent = value.log2().floor() as i32;
    let fraction = value / 2.0_f64.powi(exponent);
    let mut mantissa = ((fraction - 1.0) * 2_147_483_648.0).round() as u64;
    if mantissa == 2_147_483_648 {
        mantissa = 0;
        exponent += 1;
    }
    let exponent = u8::try_from(exponent + 129)
        .ok()
        .filter(|v| *v != 0)
        .ok_or_else(|| {
            error(
                "DSS1110",
                "Numeric literal is outside the native BASIC range",
            )
        })?;
    let m = u32::try_from(mantissa)
        .map_err(|_| {
            error(
                "DSS1110",
                "Numeric literal is outside the native BASIC range",
            )
        })?
        .to_be_bytes();
    Ok([exponent, m[0], m[1], m[2], m[3]])
}
fn encode(d: &Dialect, byte: u8) -> Result<u8> {
    (d.encode)(byte).ok_or_else(|| {
        error(
            "DSS1110",
            "Character is unavailable in the selected BASIC character set",
        )
    })
}
/// Each source line has an explicit ascending native BASIC line number.
pub fn program(source: &str, d: &Dialect) -> Result<Vec<u8>> {
    if source.len() > 128 * 1024 || !source.is_ascii() {
        return Err(error("DSS1110", "Use ASCII BASIC source up to 128 KiB"));
    }
    let mut out = Vec::new();
    let mut previous = 0;
    for (index, line) in source.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let bad = || {
            error(
                "DSS1110",
                format!(
                    "Source line {}: use an ascending BASIC line number followed by a statement",
                    index + 1
                ),
            )
        };
        if line.len() > 4096 {
            return Err(bad());
        }
        let split = line.find(char::is_whitespace).ok_or_else(bad)?;
        let line_number = line[..split].parse::<u16>().map_err(|_| bad())?;
        if line_number <= previous || line_number > d.max_line {
            return Err(bad());
        }
        previous = line_number;
        let body = body(line[split..].trim(), d)
            .map_err(|e| error(e.code, format!("Source line {}: {}", index + 1, e.message)))?;
        out.extend(line_number.to_be_bytes());
        out.extend(u16::try_from(body.len()).map_err(|_| bad())?.to_le_bytes());
        out.extend(body);
        if out.len() > 60000 {
            return Err(error("DSS1111", "BASIC program exceeds supported memory"));
        }
    }
    if out.is_empty() {
        return Err(error(
            "DSS1110",
            "Enter at least one numbered BASIC statement",
        ));
    }
    Ok(out)
}
pub fn body(source: &str, d: &Dialect) -> Result<Vec<u8>> {
    let text = source.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    let mut quoted = false;
    let mut rem = false;
    let mut first = true;
    while i < text.len() {
        let b = text[i];
        if rem {
            out.push(encode(d, b)?);
            i += 1;
            continue;
        }
        if quoted
            && b == b'"'
            && text.get(i + 1) == Some(&b'"')
            && let Some(code) = d.escaped_quote
        {
            out.push(code);
            i += 2;
            continue;
        }
        if b == b'"' {
            quoted = !quoted;
            out.push(encode(d, b)?);
            i += 1;
            continue;
        }
        if quoted {
            out.push(encode(d, b)?);
            i += 1;
            continue;
        }
        if b == b':' && !d.statements {
            return Err(error("DSS1110", "ZX81 permits one statement per line"));
        }
        if b == b':' {
            first = true;
            out.push(encode(d, b)?);
            i += 1;
            continue;
        }
        if b.is_ascii_whitespace() {
            out.push(encode(d, b' ')?);
            i += 1;
            continue;
        }
        let token = d
            .tokens
            .iter()
            .chain(d.extra_tokens.iter())
            .filter(|(word, _)| {
                source[i..]
                    .get(..word.len())
                    .is_some_and(|v| v.eq_ignore_ascii_case(word))
                    && (!word.as_bytes().last().is_some_and(u8::is_ascii_alphabetic)
                        || text
                            .get(i + word.len())
                            .is_none_or(|c| !c.is_ascii_alphabetic() && *c != b'$'))
            })
            .max_by_key(|(word, _)| word.len());
        if let Some((word, code)) = token {
            if first
                && !matches!(
                    *word,
                    "REM"
                        | "PRINT"
                        | "LET"
                        | "IF"
                        | "FOR"
                        | "NEXT"
                        | "GOTO"
                        | "GO TO"
                        | "GOSUB"
                        | "GO SUB"
                        | "RETURN"
                        | "STOP"
                        | "END"
                        | "CLS"
                        | "RUN"
                        | "DIM"
                        | "INPUT"
                        | "POKE"
                        | "RAND"
                        | "RANDOMIZE"
                        | "RANDOMISE"
                        | "PAUSE"
                        | "PLOT"
                        | "UNPLOT"
                        | "SCROLL"
                        | "FAST"
                        | "SLOW"
                        | "CLEAR"
                        | "LOAD"
                        | "SAVE"
                        | "BORDER"
                        | "INK"
                        | "PAPER"
                        | "FLASH"
                        | "BRIGHT"
                        | "INVERSE"
                        | "OVER"
                        | "BEEP"
                        | "DRAW"
                        | "CIRCLE"
                        | "DATA"
                        | "READ"
                        | "RESTORE"
                        | "OUT"
                        | "LPRINT"
                        | "LLIST"
                        | "LIST"
                        | "NEW"
                        | "CONT"
                        | "CONTINUE"
                        | "COPY"
                        | "PLAY"
                        | "SPECTRUM"
                        | "MERGE"
                        | "VERIFY"
                        | "DEF FN"
                )
            {
                return Err(error("DSS1110", "Expected a supported BASIC statement"));
            }
            if *word == "DEF FN" {
                return Err(error(
                    "DSS1110",
                    "DEF FN parameter encoding is not supported yet",
                ));
            }
            out.push(*code);
            i += word.len();
            first = false;
            rem = *word == "REM";
            if *word == "BIN" {
                while text.get(i) == Some(&b' ') {
                    out.push(encode(d, b' ')?);
                    i += 1;
                }
                let start = i;
                while text.get(i).is_some_and(u8::is_ascii_digit) {
                    i += 1;
                }
                let literal = &source[start..i];
                let value = u16::from_str_radix(literal, 2)
                    .map_err(|_| error("DSS1110", "BIN requires 1 to 16 binary digits"))?;
                if literal.len() > 16 {
                    return Err(error("DSS1110", "BIN requires 1 to 16 binary digits"));
                }
                for byte in literal.bytes() {
                    out.push(encode(d, byte)?);
                }
                out.push(d.number_marker);
                out.extend(number(f64::from(value))?);
            }
            continue;
        }
        if first {
            return Err(error("DSS1110", "Expected a supported BASIC statement"));
        }
        if b.is_ascii_digit() || (b == b'.' && text.get(i + 1).is_some_and(u8::is_ascii_digit)) {
            let start = i;
            while text
                .get(i)
                .is_some_and(|c| c.is_ascii_digit() || *c == b'.')
            {
                i += 1;
            }
            if text.get(i).is_some_and(|c| *c == b'e' || *c == b'E') {
                i += 1;
                if text.get(i).is_some_and(|c| *c == b'+' || *c == b'-') {
                    i += 1;
                }
                while text.get(i).is_some_and(u8::is_ascii_digit) {
                    i += 1;
                }
            }
            let literal = &source[start..i];
            let value = literal
                .parse::<f64>()
                .map_err(|_| error("DSS1110", "Invalid numeric literal"))?;
            for b in literal.bytes() {
                out.push(encode(d, b.to_ascii_uppercase())?);
            }
            out.push(d.number_marker);
            out.extend(number(value)?);
            continue;
        }
        if b.is_ascii_alphabetic() {
            while text
                .get(i)
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'$')
            {
                out.push(encode(d, text[i].to_ascii_uppercase())?);
                i += 1;
            }
            continue;
        }
        out.push(encode(d, b)?);
        i += 1;
    }
    if quoted {
        return Err(error("DSS1110", "Unterminated string literal"));
    }
    if first {
        return Err(error("DSS1110", "Missing BASIC statement"));
    }
    out.push(d.newline);
    Ok(out)
}
/// Recover only the representable native BASIC subset, retaining numeric and token integrity.
pub fn listing(bytes: &[u8], dialect: &Dialect) -> Result<String> {
    if bytes.is_empty() || bytes.len() > 60000 {
        return Err(error("DSS1120", "Invalid BASIC programme size"));
    }
    let mut source = String::new();
    let mut pos = 0usize;
    let mut previous = 0;
    while pos < bytes.len() {
        let header = bytes
            .get(pos..pos + 4)
            .ok_or_else(|| error("DSS1120", "Truncated BASIC line"))?;
        let line = u16::from_be_bytes([header[0], header[1]]);
        let length = usize::from(u16::from_le_bytes([header[2], header[3]]));
        if line <= previous || line > dialect.max_line || length == 0 || length > 4096 {
            return Err(error("DSS1120", "Invalid BASIC line number or length"));
        }
        previous = line;
        pos += 4;
        let body = bytes
            .get(pos..pos + length)
            .ok_or_else(|| error("DSS1120", "Truncated BASIC body"))?;
        if body.last() != Some(&dialect.newline) {
            return Err(error("DSS1120", "Missing BASIC line terminator"));
        }
        let mut text = String::new();
        let mut quoted = false;
        let mut rem = false;
        let mut index = 0;
        while index + 1 < body.len() {
            let byte = body[index];
            if !quoted && !rem && byte == dialect.number_marker {
                if index + 6 >= body.len() {
                    return Err(error("DSS1120", "Truncated numeric literal"));
                }
                index += 6;
                continue;
            }
            if quoted && dialect.escaped_quote == Some(byte) {
                text.push_str("\"\"");
                index += 1;
                continue;
            }
            if !quoted
                && !rem
                && let Some((word, _)) = dialect
                    .tokens
                    .iter()
                    .chain(dialect.extra_tokens.iter())
                    .find(|(_, code)| *code == byte)
            {
                text.push_str(word);
                rem = *word == "REM";
            } else {
                let character = (32..=126)
                    .find(|candidate| (dialect.encode)(*candidate) == Some(byte))
                    .ok_or_else(|| {
                        error("DSS1120", "Unsupported BASIC character or control code")
                    })?;
                text.push(char::from(character));
                if character == b'"' {
                    quoted = !quoted;
                }
            }
            index += 1;
        }
        if text.len() > 4096 {
            return Err(error("DSS1120", "Recovered BASIC line is too large"));
        }
        source.push_str(&format!("{line} {text}\n"));
        if source.len() > 128 * 1024 {
            return Err(error("DSS1120", "Recovered source is too large"));
        }
        pos += length;
    }
    if program(&source, dialect)? != bytes {
        return Err(error(
            "DSS1120",
            "Programme uses unsupported tokens or noncanonical numeric data",
        ));
    }
    Ok(source)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn floating_point_reference_values() {
        assert_eq!(number(0.0).unwrap(), [0; 5]);
        assert_eq!(number(1.0).unwrap(), [129, 0, 0, 0, 0]);
        assert_eq!(number(0.25).unwrap(), [127, 0, 0, 0, 0]);
        assert_eq!(number(10.0).unwrap(), [132, 32, 0, 0, 0]);
        assert!(number(f64::INFINITY).is_err());
        assert!(number(1e100).is_err());
    }
}
