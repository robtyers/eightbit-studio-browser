//! Original BASIC V2 tokeniser and linked-line PRG encoder from the published format.
use eightbit_source_core::{Result, error};
// Public BASIC V2 token assignments, $80–$CB; these are format constants, not ROM code.
const TOKENS: &[&str] = &[
    "END", "FOR", "NEXT", "DATA", "INPUT#", "INPUT", "DIM", "READ", "LET", "GOTO", "RUN", "IF",
    "RESTORE", "GOSUB", "RETURN", "REM", "STOP", "ON", "WAIT", "LOAD", "SAVE", "VERIFY", "DEF",
    "POKE", "PRINT#", "PRINT", "CONT", "LIST", "CLR", "CMD", "SYS", "OPEN", "CLOSE", "GET", "NEW",
    "TAB(", "TO", "FN", "SPC(", "THEN", "NOT", "STEP", "+", "-", "*", "/", "^", "AND", "OR", ">",
    "=", "<", "SGN", "INT", "ABS", "USR", "FRE", "POS", "SQR", "RND", "LOG", "EXP", "COS", "SIN",
    "TAN", "ATN", "PEEK", "LEN", "STR$", "VAL", "ASC", "CHR$", "LEFT$", "RIGHT$", "MID$", "GO",
];
fn invalid(message: &str) -> eightbit_source_core::Error {
    error("DC64100", message)
}
pub fn compile(source: &str) -> Result<Vec<u8>> {
    if source.len() > eightbit_source_core::MAX_SOURCE {
        return Err(invalid("BASIC source exceeds 128 KiB"));
    }
    let mut output = vec![1, 8];
    let mut previous = None;
    for raw in source.lines() {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        if raw.len() > 4096 || !raw.is_ascii() || raw.bytes().any(|b| !(32..=126).contains(&b)) {
            return Err(invalid(
                "Use printable ASCII BASIC; use CHR$ for control or graphics characters",
            ));
        }
        let end = raw.bytes().take_while(u8::is_ascii_digit).count();
        let number = raw[..end]
            .parse::<u16>()
            .map_err(|_| invalid("Every BASIC line needs a number from 0 to 63999"))?;
        if number > 63999 || previous.is_some_and(|n| number <= n) {
            return Err(invalid(
                "Line numbers must increase without duplicates and be at most 63999",
            ));
        }
        previous = Some(number);
        let body = raw[end..].trim_start().to_ascii_uppercase();
        if body.is_empty() {
            return Err(invalid(
                "Empty numbered lines are unavailable; remove the line instead",
            ));
        }
        let body = tokenise(body.as_bytes())?;
        if body.len() > 250 {
            return Err(invalid("Tokenised BASIC lines must not exceed 250 bytes"));
        }
        let next = 0x0801usize + (output.len() - 2) + 5 + body.len();
        if next + 2 > 0xa000 {
            return Err(invalid("BASIC exceeds the C64 memory below $A000"));
        }
        let next = u16::try_from(next).map_err(|_| invalid("BASIC address overflow"))?;
        output.extend_from_slice(&next.to_le_bytes());
        output.extend_from_slice(&number.to_le_bytes());
        output.extend(body);
        output.push(0);
    }
    if previous.is_none() {
        return Err(invalid("Enter at least one numbered BASIC line"));
    }
    output.extend([0, 0]);
    Ok(output)
}
fn tokenise(input: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut pos = 0;
    let mut quoted = false;
    let mut data = false;
    while pos < input.len() {
        let byte = input[pos];
        if byte == b'"' {
            quoted = !quoted;
            out.push(byte);
            pos += 1;
            continue;
        }
        if !quoted && data && byte == b':' {
            data = false;
        }
        if quoted || data {
            out.push(byte);
            pos += 1;
            continue;
        }
        if byte == b'?' {
            out.push(0x99);
            pos += 1;
            continue;
        }
        if let Some((index, word)) = TOKENS
            .iter()
            .enumerate()
            .filter(|(_, w)| input[pos..].starts_with(w.as_bytes()))
            .max_by_key(|(_, w)| w.len())
        {
            out.push(0x80 + u8::try_from(index).map_err(|_| invalid("Invalid BASIC token"))?);
            pos += word.len();
            if *word == "REM" {
                out.extend_from_slice(&input[pos..]);
                break;
            }
            if *word == "DATA" {
                data = true;
            }
        } else {
            out.push(byte);
            pos += 1;
        }
    }
    if quoted {
        return Err(invalid(
            "Close the quoted BASIC string before the end of the line",
        ));
    }
    Ok(out)
}

/// Recover the supported BASIC V2 PRG subset; reject machine-code tails and invalid links.
pub fn decompile(bytes: &[u8]) -> Result<String> {
    if bytes.len() < 9 || bytes.len() > 38913 || bytes[..2] != [1, 8] {
        return Err(error("DC64120", "Choose a BASIC PRG loaded at $0801"));
    }
    let mut pos = 2;
    let mut source = String::new();
    loop {
        let header = bytes
            .get(pos..pos + 2)
            .ok_or_else(|| error("DC64120", "Truncated BASIC link"))?;
        let next = u16::from_le_bytes([header[0], header[1]]);
        if next == 0 {
            if pos + 2 != bytes.len() {
                return Err(error(
                    "DC64120",
                    "PRG contains an unsupported machine-code tail",
                ));
            }
            break;
        }
        let number = bytes
            .get(pos + 2..pos + 4)
            .ok_or_else(|| error("DC64120", "Truncated line number"))?;
        let line = u16::from_le_bytes([number[0], number[1]]);
        let end = usize::from(next)
            .checked_sub(0x0801)
            .and_then(|offset| offset.checked_add(2))
            .ok_or_else(|| error("DC64120", "Invalid BASIC link"))?;
        if end <= pos + 4 || end > bytes.len() || end - pos > 255 || bytes[end - 1] != 0 {
            return Err(error("DC64120", "Invalid BASIC line link or terminator"));
        }
        let mut quoted = false;
        let mut data = false;
        let mut rem = false;
        source.push_str(&format!("{line} "));
        for byte in &bytes[pos + 4..end - 1] {
            if *byte == b'"' && !rem {
                quoted = !quoted;
            }
            if !quoted && data && *byte == b':' {
                data = false;
            }
            if !quoted && !data && !rem && *byte >= 0x80 {
                let word = TOKENS
                    .get(usize::from(*byte - 0x80))
                    .ok_or_else(|| error("DC64120", "Unsupported BASIC token"))?;
                source.push_str(word);
                rem = *word == "REM";
                data = *word == "DATA";
            } else if (32..=126).contains(byte) {
                source.push(char::from(*byte));
            } else {
                return Err(error(
                    "DC64120",
                    "Use the desktop tools for unsupported PETSCII or control codes",
                ));
            }
        }
        source.push('\n');
        pos = end;
    }
    if compile(&source)? != bytes {
        return Err(error("DC64120", "Invalid or unsupported BASIC programme"));
    }
    Ok(source)
}
