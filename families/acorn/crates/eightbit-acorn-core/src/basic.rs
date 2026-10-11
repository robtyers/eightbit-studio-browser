//! Original BASIC II encoding using published token/line format constants.
//! No interpreter implementation or ROM source is used; see ADR 0054 provenance.
use eightbit_source_core::{Result, error};
// Public BASIC II token assignments. Empty entries are not keyword tokens.
const TOKENS: &[&str] = &[
    "AND", "DIV", "EOR", "MOD", "OR", "ERROR", "LINE", "OFF", "STEP", "SPC", "TAB(", "ELSE",
    "THEN", "", "OPENIN", "PTR", "PAGE", "TIME", "LOMEM", "HIMEM", "ABS", "ACS", "ADVAL", "ASC",
    "ASN", "ATN", "BGET", "COS", "COUNT", "DEG", "ERL", "ERR", "EVAL", "EXP", "EXT", "FALSE", "FN",
    "GET", "INKEY", "INSTR(", "INT", "LEN", "LN", "LOG", "NOT", "OPENUP", "OPENOUT", "PI",
    "POINT(", "POS", "RAD", "RND", "SGN", "SIN", "SQR", "TAN", "TO", "TRUE", "USR", "VAL", "VPOS",
    "CHR$", "GET$", "INKEY$", "LEFT$(", "MID$(", "RIGHT$(", "STR$", "STRING$(", "EOF", "AUTO",
    "DELETE", "LOAD", "LIST", "NEW", "OLD", "RENUMBER", "SAVE", "", "", "", "", "", "", "SOUND",
    "BPUT", "CALL", "CHAIN", "CLEAR", "CLOSE", "CLG", "CLS", "DATA", "DEF", "DIM", "DRAW", "END",
    "ENDPROC", "ENVELOPE", "FOR", "GOSUB", "GOTO", "GCOL", "IF", "INPUT", "LET", "LOCAL", "MODE",
    "MOVE", "NEXT", "ON", "VDU", "PLOT", "PRINT", "PROC", "READ", "REM", "REPEAT", "REPORT",
    "RESTORE", "RETURN", "RUN", "STOP", "COLOUR", "TRACE", "UNTIL", "WIDTH", "OSCLI",
];
fn invalid(message: impl Into<String>) -> eightbit_source_core::Error {
    error("DBBC100", message)
}
pub fn compile(source: &str) -> Result<Vec<u8>> {
    if source.len() > eightbit_source_core::MAX_SOURCE {
        return Err(invalid("BASIC source exceeds 128 KiB"));
    }
    let mut output = Vec::new();
    let mut previous = None;
    for (index, raw) in source.lines().enumerate() {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        if raw.len() > 4096 || !raw.bytes().all(|b| (32..=126).contains(&b)) {
            return Err(invalid(format!(
                "Line {}: use bounded printable ASCII BASIC",
                index + 1
            )));
        }
        let count = raw.bytes().take_while(u8::is_ascii_digit).count();
        let number = raw[..count]
            .parse::<u16>()
            .map_err(|_| invalid("Every BASIC line needs a number from 0 to 32767"))?;
        if number > 32767 || previous.is_some_and(|n| number <= n) {
            return Err(invalid(
                "Line numbers must increase without duplicates and be at most 32767",
            ));
        }
        let body = raw[count..].trim_start();
        if body.is_empty() {
            return Err(invalid("Remove empty numbered lines from the source"));
        }
        let body = tokenise(body.as_bytes())?;
        let length = u8::try_from(body.len() + 4).map_err(|_| {
            invalid("Tokenised BASIC lines must not exceed 255 bytes including their header")
        })?;
        if output.len() + usize::from(length) + 2 > crate::PROGRAM_LIMIT {
            return Err(invalid("BASIC programme must fit between &1900 and &3000"));
        }
        output.push(13);
        output.extend(number.to_be_bytes());
        output.push(length);
        output.extend(body);
        previous = Some(number);
    }
    if previous.is_none() {
        return Err(invalid("Enter at least one numbered BASIC line"));
    }
    output.extend([13, 255]);
    Ok(output)
}
fn identifier(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Recover only the bounded, canonical BASIC II subset produced by this encoder.
pub fn decompile(bytes: &[u8]) -> Result<String> {
    let invalid = || error("DBBC103", "Unsupported or corrupt BBC BASIC II programme");
    if bytes.len() < 7 || bytes.len() > crate::PROGRAM_LIMIT {
        return Err(invalid());
    }
    let mut source = String::new();
    let mut position = 0;
    while bytes.get(position..position + 2) != Some(&[13, 255]) {
        let header = bytes.get(position..position + 4).ok_or_else(invalid)?;
        if header[0] != 13 || header[3] < 5 {
            return Err(invalid());
        }
        let number = u16::from_be_bytes([header[1], header[2]]);
        let end = position + usize::from(header[3]);
        let body = bytes.get(position + 4..end).ok_or_else(invalid)?;
        source.push_str(&format!("{number} "));
        let mut cursor = 0;
        let mut quoted = false;
        let mut literal = false;
        while let Some(&byte) = body.get(cursor) {
            if (32..=126).contains(&byte) {
                source.push(char::from(byte));
                if byte == b'"' && !literal {
                    quoted = !quoted;
                }
                cursor += 1;
            } else if quoted || literal {
                return Err(invalid());
            } else if byte == 0x8d {
                let reference = body.get(cursor..cursor + 4).ok_or_else(invalid)?;
                let low = (reference[2] & 0x3f) | (((reference[1] ^ 0x54) << 2) & 0xc0);
                let high = (reference[3] & 0x3f) | (((reference[1] ^ 0x54) << 4) & 0xc0);
                source.push_str(&u16::from_le_bytes([low, high]).to_string());
                cursor += 4;
            } else {
                let token = if (0xcf..=0xd3).contains(&byte) {
                    byte - 0x40
                } else {
                    byte
                };
                let word = TOKENS
                    .get(usize::from(token.saturating_sub(0x80)))
                    .filter(|word| !word.is_empty())
                    .ok_or_else(invalid)?;
                source.push_str(word);
                literal = matches!(byte, 0xdc | 0xf4);
                cursor += 1;
            }
        }
        source.push('\n');
        position = end;
    }
    if position + 2 != bytes.len() || compile(&source)? != bytes {
        return Err(invalid());
    }
    Ok(source)
}

fn conditional(token: u8) -> bool {
    matches!(token, 0x8f..=0x93 | 0x9a | 0x9c | 0x9e | 0x9f | 0xa2 | 0xa3 | 0xaf | 0xb1 | 0xb3 | 0xb9 | 0xbc | 0xc5 | 0xca | 0xcb | 0xd5 | 0xd8..=0xdb | 0xe0 | 0xe1 | 0xf6 | 0xf8..=0xfa)
}
fn tokenise(input: &[u8]) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut pos = 0;
    let mut statement = true;
    let mut references = false;
    while pos < input.len() {
        let byte = input[pos];
        if byte == b' ' {
            output.push(byte);
            pos += 1;
            continue;
        }
        if byte == b'"' {
            let end = input[pos + 1..]
                .iter()
                .position(|b| *b == b'"')
                .map(|n| pos + n + 2)
                .ok_or_else(|| {
                    invalid("Close the quoted BASIC string before the end of the line")
                })?;
            output.extend_from_slice(&input[pos..end]);
            pos = end;
            continue;
        }
        if statement && byte == b'*' {
            output.extend_from_slice(&input[pos..]);
            break;
        }
        if references && byte.is_ascii_digit() {
            let end = pos
                + input[pos..]
                    .iter()
                    .take_while(|b| b.is_ascii_digit())
                    .count();
            let number = std::str::from_utf8(&input[pos..end])
                .map_err(|_| invalid("Invalid line reference"))?
                .parse::<u16>()
                .map_err(|_| invalid("Line reference exceeds 32767"))?;
            if number > 32767 {
                return Err(invalid("Line reference exceeds 32767"));
            }
            let [low, high] = number.to_le_bytes();
            output.extend([
                0x8d,
                0x54 ^ ((low & 0xc0) >> 2) ^ ((high & 0xc0) >> 4),
                0x40 | (low & 0x3f),
                0x40 | (high & 0x3f),
            ]);
            pos = end;
            continue;
        }
        if byte == b'&' {
            let end = pos
                + 1
                + input[pos + 1..]
                    .iter()
                    .take_while(|b| b.is_ascii_hexdigit())
                    .count();
            output.extend_from_slice(&input[pos..end]);
            pos = end;
            statement = false;
            continue;
        }
        let found = TOKENS
            .iter()
            .enumerate()
            .filter(|(_, word)| !word.is_empty() && input[pos..].starts_with(word.as_bytes()))
            .filter(|(index, word)| {
                !conditional(0x80 + u8::try_from(*index).unwrap_or(0))
                    || !input.get(pos + word.len()).is_some_and(|b| identifier(*b))
            })
            .max_by_key(|(_, word)| word.len());
        if let Some((index, word)) = found {
            let mut token =
                0x80 + u8::try_from(index).map_err(|_| invalid("Invalid keyword token"))?;
            if statement && (0x8f..=0x93).contains(&token) {
                token += 0x40;
            }
            output.push(token);
            pos += word.len();
            if matches!(token, 0xdc | 0xf4) {
                output.extend_from_slice(&input[pos..]);
                break;
            }
            references = matches!(
                token,
                0x8b | 0x8c | 0xc6 | 0xc7 | 0xc9 | 0xcc | 0xe4 | 0xe5 | 0xf7 | 0xfc
            );
            if matches!(token, 0x85 | 0x8b | 0x8c | 0xe9) {
                statement = true;
            } else if matches!(token, 0x8f..=0x93 | 0xcf..=0xd3 | 0xc8 | 0xcd | 0xd4..=0xd7 | 0xd9 | 0xde | 0xdf | 0xe2..=0xe8 | 0xea..=0xf3 | 0xf7 | 0xfc..=0xff)
            {
                statement = false;
            }
            if matches!(token, 0xa4 | 0xf2) {
                let end = pos + input[pos..].iter().take_while(|b| identifier(**b)).count();
                output.extend_from_slice(&input[pos..end]);
                pos = end;
            }
            continue;
        }
        if byte.is_ascii_alphabetic() || byte == b'_' {
            let end = pos + input[pos..].iter().take_while(|b| identifier(**b)).count();
            output.extend_from_slice(&input[pos..end]);
            pos = end;
            statement = false;
            references = false;
            continue;
        }
        statement = byte == b':';
        if byte != b',' {
            references = false;
        }
        output.push(byte);
        pos += 1;
    }
    Ok(output)
}

#[cfg(test)]
mod portable_tests {
    use super::*;
    #[test]
    fn recovery_preserves_literal_regions_names_lvalues_and_line_references() {
        let source = "0 MODE 7\n10 PAGE=&1900:PTR#1=0\n20 DEF FNanswer=42\n30 PROCPrintName\n40 IF FNanswer=42 THEN 100 ELSE 110\n50 DATA GOTO,\"PRINT\"\n60 REM ENDPROC PRINT\n70 *FX 4,1\n100 PRINT \"GOTO 110\":END\n110 DEF PROCPrintName:PRINT \"Hello\":ENDPROC\n";
        let bytes = compile(source).unwrap();
        assert_eq!(decompile(&bytes).unwrap(), source);
        for size in 0..bytes.len() {
            assert!(decompile(&bytes[..size]).is_err());
        }
    }
    #[test]
    fn recovery_rejects_noncanonical_tokens_lengths_and_trailing_data() {
        let good = compile("10 GOTO 20\n20 END\n").unwrap();
        for (offset, byte) in [(0, 0), (3, 4), (4, 0x01), (5, 0x8d), (7, 0), (8, 0xff)] {
            let mut bad = good.clone();
            bad[offset] = byte;
            assert!(decompile(&bad).is_err());
        }
        let mut bad = good;
        bad.push(0);
        assert!(decompile(&bad).is_err());
    }
}
