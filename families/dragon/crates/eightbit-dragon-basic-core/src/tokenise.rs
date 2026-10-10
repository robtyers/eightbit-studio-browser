use crate::{BasicLine, Dialect, Program, SourceRange, Statement};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use thiserror::Error;

const FUNCTION_PREFIX: u8 = 0xff;

/// A line and its exact position in a generated linked-line stream.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TokenisedLine {
    /// BASIC line number.
    pub number: u16,
    /// Offset of the two-byte next-line link.
    pub record_offset: usize,
    /// Runtime address of the line record.
    pub runtime_address: u16,
    /// Token bytes excluding record header and trailing zero.
    pub code: Vec<u8>,
    /// Statement mappings used to build a source map.
    pub statements: Vec<TokenisedStatement>,
}

/// Source/token correspondence for one statement.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TokenisedStatement {
    /// Zero-based ordinal within the BASIC line.
    pub ordinal: usize,
    /// Original source range.
    pub source_range: SourceRange,
    /// Offset into the line's token bytes.
    pub token_offset: usize,
    /// Length in token bytes.
    pub token_length: usize,
}

/// A complete linked-line Dragon BASIC program.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TokenisedProgram {
    /// Runtime program base.
    pub program_start: u16,
    /// Exact linked-line bytes, including the final `0x0000` marker.
    pub bytes: Vec<u8>,
    /// Per-line mapping metadata.
    pub lines: Vec<TokenisedLine>,
}

/// Warning produced while recovering a token stream.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TokenWarning {
    /// Stable warning code.
    pub code: &'static str,
    /// Byte offset in the linked stream.
    pub offset: usize,
    /// Explanation.
    pub message: String,
}

/// Canonical text recovered from linked-line bytes.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DetokenisedProgram {
    /// Canonical ASCII listing with LF endings.
    pub source: String,
    /// Lossless unknown-token notices.
    pub warnings: Vec<TokenWarning>,
}

/// Hash and compatibility evidence for an explicit conversion.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConversionReport {
    /// Report schema version.
    pub schema_version: u32,
    /// Dialect identifier.
    pub dialect: String,
    /// Explicit input format.
    pub input_format: String,
    /// Explicit output format.
    pub output_format: String,
    /// SHA-256 of exact input bytes.
    pub input_sha256: String,
    /// SHA-256 of exact output bytes.
    pub output_sha256: String,
    /// Conversion warnings.
    pub warnings: Vec<TokenWarning>,
    /// Whether a lossy decision was made.
    pub lossy: bool,
}

impl ConversionReport {
    /// Construct a deterministic conversion report.
    #[must_use]
    pub fn new(
        dialect: &Dialect,
        input_format: impl Into<String>,
        output_format: impl Into<String>,
        input: &[u8],
        output: &[u8],
        warnings: Vec<TokenWarning>,
    ) -> Self {
        Self {
            schema_version: 1,
            dialect: dialect.id.clone(),
            input_format: input_format.into(),
            output_format: output_format.into(),
            input_sha256: sha256(input),
            output_sha256: sha256(output),
            lossy: false,
            warnings,
        }
    }
}

/// Token conversion failure.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TokenError {
    /// Source contained a non-ASCII byte.
    #[error("DST1000: Dragon BASIC source must be ASCII (byte {offset})")]
    NonAscii {
        /// Offending byte offset.
        offset: usize,
    },
    /// A generated program would exceed configured BASIC memory.
    #[error("DST1001: tokenised program ends at 0x{end:04X}, beyond 0x{maximum:04X}")]
    ProgramTooLarge {
        /// Computed end address.
        end: u32,
        /// Highest permitted address.
        maximum: u16,
    },
    /// Input ended before a complete record was available.
    #[error("DST1002: truncated linked-line stream at byte {offset}")]
    Truncated {
        /// Failure offset.
        offset: usize,
    },
    /// A next-line pointer did not match the record boundary.
    #[error("DST1003: line at byte {offset} points to 0x{actual:04X}; expected 0x{expected:04X}")]
    InvalidLink {
        /// Record offset.
        offset: usize,
        /// Link read from input.
        actual: u16,
        /// Link required by this stream.
        expected: u16,
    },
    /// A line has no terminating zero.
    #[error("DST1004: line {line} has no terminating zero")]
    UnterminatedLine {
        /// BASIC line number.
        line: u16,
    },
    /// The stream has bytes after its final marker.
    #[error("DST1005: {count} trailing byte(s) follow the final program marker")]
    TrailingBytes {
        /// Unexpected byte count.
        count: usize,
    },
    /// Source used a reserved unknown-token escape incorrectly.
    #[error("DST1006: invalid unknown-token escape at source column {column}")]
    InvalidEscape {
        /// Zero-based code column.
        column: usize,
    },
}

/// Tokenise a parsed program into Dragon's linked-line memory representation.
pub fn tokenise(program: &Program, dialect: &Dialect) -> Result<TokenisedProgram, TokenError> {
    let mut bytes = Vec::new();
    let mut lines = Vec::with_capacity(program.lines.len());
    let mut address = u32::from(dialect.program_start);
    for line in &program.lines {
        let code = tokenise_code(&line.code, dialect)?;
        let record_len = 2_u32
            .checked_add(2)
            .and_then(|value| value.checked_add(u32::try_from(code.len()).ok()?))
            .and_then(|value| value.checked_add(1))
            .ok_or(TokenError::ProgramTooLarge {
                end: u32::MAX,
                maximum: dialect.maximum_program_address,
            })?;
        let next = address
            .checked_add(record_len)
            .ok_or(TokenError::ProgramTooLarge {
                end: u32::MAX,
                maximum: dialect.maximum_program_address,
            })?;
        if next + 1 > u32::from(dialect.maximum_program_address) {
            return Err(TokenError::ProgramTooLarge {
                end: next + 1,
                maximum: dialect.maximum_program_address,
            });
        }
        let next_u16 = u16::try_from(next).map_err(|_| TokenError::ProgramTooLarge {
            end: next,
            maximum: dialect.maximum_program_address,
        })?;
        let record_offset = bytes.len();
        bytes.extend_from_slice(&next_u16.to_be_bytes());
        bytes.extend_from_slice(&line.number.to_be_bytes());
        bytes.extend_from_slice(&code);
        bytes.push(0);
        lines.push(TokenisedLine {
            number: line.number,
            record_offset,
            runtime_address: u16::try_from(address).map_err(|_| TokenError::ProgramTooLarge {
                end: address,
                maximum: dialect.maximum_program_address,
            })?,
            statements: statement_mappings(line, dialect)?,
            code,
        });
        address = next;
    }
    bytes.extend_from_slice(&[0, 0]);
    Ok(TokenisedProgram {
        program_start: dialect.program_start,
        bytes,
        lines,
    })
}

fn statement_mappings(
    line: &BasicLine,
    dialect: &Dialect,
) -> Result<Vec<TokenisedStatement>, TokenError> {
    let mut mappings = Vec::with_capacity(line.statements.len());
    for Statement { ordinal, range, .. } in &line.statements {
        let local_start = range.start.saturating_sub(line.code_range.start);
        let local_end = range.end.saturating_sub(line.code_range.start);
        let before = tokenise_code(&line.code[..local_start], dialect)?;
        let through = tokenise_code(&line.code[..local_end], dialect)?;
        mappings.push(TokenisedStatement {
            ordinal: *ordinal,
            source_range: *range,
            token_offset: before.len(),
            token_length: through.len().saturating_sub(before.len()),
        });
    }
    Ok(mappings)
}

fn tokenise_code(code: &str, dialect: &Dialect) -> Result<Vec<u8>, TokenError> {
    if let Some(offset) = code.bytes().position(|byte| !byte.is_ascii()) {
        return Err(TokenError::NonAscii { offset });
    }
    let mut statement = dialect.statement_tokens.iter().collect::<Vec<_>>();
    let mut functions = dialect.function_tokens.iter().collect::<Vec<_>>();
    statement.sort_by_key(|token| std::cmp::Reverse(token.text.len()));
    functions.sort_by_key(|token| std::cmp::Reverse(token.text.len()));

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum State {
        Normal,
        String,
        Data,
        DataString,
        Comment,
    }

    let bytes = code.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut state = State::Normal;
    let mut index = 0;
    while index < bytes.len() {
        if state == State::Normal
            && code[index..].starts_with("{{TOKEN:0x")
            && let Some(close) = code[index + 10..].find("}}")
        {
            let digits_end = index + 10 + close;
            let digits = &code[index + 10..digits_end];
            let value = u16::from_str_radix(digits, 16)
                .map_err(|_| TokenError::InvalidEscape { column: index })?;
            match value {
                0x80..=0xfe => output.push(
                    u8::try_from(value).map_err(|_| TokenError::InvalidEscape { column: index })?,
                ),
                0xff80..=0xffff => {
                    output.push(FUNCTION_PREFIX);
                    output.push(value.to_be_bytes()[1]);
                }
                _ => return Err(TokenError::InvalidEscape { column: index }),
            }
            index = digits_end + 2;
            continue;
        }
        if code[index..].starts_with("{{BYTE:0x")
            && let Some(close) = code[index + 9..].find("}}")
        {
            let digits_end = index + 9 + close;
            let digits = &code[index + 9..digits_end];
            let value = u8::from_str_radix(digits, 16)
                .map_err(|_| TokenError::InvalidEscape { column: index })?;
            output.push(value);
            index = digits_end + 2;
            continue;
        }

        match state {
            State::String => {
                output.push(bytes[index]);
                if bytes[index] == b'"' {
                    state = State::Normal;
                }
                index += 1;
            }
            State::DataString => {
                output.push(bytes[index]);
                if bytes[index] == b'"' {
                    state = State::Data;
                }
                index += 1;
            }
            State::Comment => {
                output.push(bytes[index]);
                index += 1;
            }
            State::Data => {
                output.push(bytes[index]);
                if bytes[index] == b'"' {
                    state = State::DataString;
                } else if bytes[index] == b':' {
                    state = State::Normal;
                }
                index += 1;
            }
            State::Normal => {
                if bytes[index] == b'"' {
                    output.push(bytes[index]);
                    state = State::String;
                    index += 1;
                    continue;
                }
                if bytes[index] == b'\'' {
                    if let Some(token) = statement.iter().find(|token| token.text == "'") {
                        output.push(token.code);
                    } else {
                        output.push(bytes[index]);
                    }
                    state = State::Comment;
                    index += 1;
                    continue;
                }
                if at_word_boundary(bytes, index) {
                    if code[index..].len() >= 4
                        && code[index..index + 4].eq_ignore_ascii_case("GOTO")
                    {
                        emit_named(&mut output, &statement, "GO", false);
                        emit_named(&mut output, &statement, "TO", false);
                        index += 4;
                        continue;
                    }
                    if code[index..].len() >= 5
                        && code[index..index + 5].eq_ignore_ascii_case("GOSUB")
                    {
                        emit_named(&mut output, &statement, "GO", false);
                        emit_named(&mut output, &statement, "SUB", false);
                        index += 5;
                        continue;
                    }
                    if let Some(token) = find_token(code, index, &functions, true) {
                        output.push(FUNCTION_PREFIX);
                        output.push(token.code);
                        index += token.text.len();
                        continue;
                    }
                    if let Some(token) = find_token(code, index, &statement, true) {
                        output.push(token.code);
                        index += token.text.len();
                        if token.text == "REM" {
                            state = State::Comment;
                        } else if token.text == "DATA" {
                            state = State::Data;
                        }
                        continue;
                    }
                }
                if let Some(token) = find_operator(code, index, &statement) {
                    output.push(token.code);
                    index += token.text.len();
                    continue;
                }
                output.push(bytes[index]);
                index += 1;
            }
        }
    }
    Ok(output)
}

fn emit_named(
    output: &mut Vec<u8>,
    tokens: &[&crate::TokenDefinition],
    name: &str,
    function: bool,
) {
    if let Some(token) = tokens.iter().find(|token| token.text == name) {
        if function {
            output.push(FUNCTION_PREFIX);
        }
        output.push(token.code);
    }
}

fn at_word_boundary(bytes: &[u8], index: usize) -> bool {
    index == 0 || !is_name_byte(bytes[index - 1])
}

fn find_token<'a>(
    code: &str,
    index: usize,
    tokens: &'a [&crate::TokenDefinition],
    require_end_boundary: bool,
) -> Option<&'a crate::TokenDefinition> {
    tokens.iter().copied().find(|token| {
        let end = index + token.text.len();
        end <= code.len()
            && code[index..end].eq_ignore_ascii_case(&token.text)
            && (!require_end_boundary
                || token.text.ends_with('(')
                || token
                    .text
                    .chars()
                    .all(|character| !character.is_ascii_alphabetic())
                || end == code.len()
                || !is_keyword_continuation(code.as_bytes()[end]))
    })
}

fn find_operator<'a>(
    code: &str,
    index: usize,
    tokens: &'a [&crate::TokenDefinition],
) -> Option<&'a crate::TokenDefinition> {
    tokens.iter().copied().find(|token| {
        !token
            .text
            .chars()
            .any(|character| character.is_ascii_alphabetic())
            && code[index..].starts_with(&token.text)
    })
}

fn is_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$' | b'%')
}

fn is_keyword_continuation(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || matches!(byte, b'_' | b'$' | b'%')
}

/// Detokenise and validate a Dragon linked-line stream.
pub fn detokenise(bytes: &[u8], dialect: &Dialect) -> Result<DetokenisedProgram, TokenError> {
    let statements: HashMap<u8, &str> = dialect
        .statement_tokens
        .iter()
        .map(|token| (token.code, token.text.as_str()))
        .collect();
    let functions: HashMap<u8, &str> = dialect
        .function_tokens
        .iter()
        .map(|token| (token.code, token.text.as_str()))
        .collect();
    let mut source = String::new();
    let mut warnings = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if cursor + 2 > bytes.len() {
            return Err(TokenError::Truncated { offset: cursor });
        }
        let record_offset = cursor;
        let link = u16::from_be_bytes([bytes[cursor], bytes[cursor + 1]]);
        cursor += 2;
        if link == 0 {
            if cursor != bytes.len() {
                return Err(TokenError::TrailingBytes {
                    count: bytes.len() - cursor,
                });
            }
            return Ok(DetokenisedProgram { source, warnings });
        }
        if cursor + 2 > bytes.len() {
            return Err(TokenError::Truncated { offset: cursor });
        }
        let line_number = u16::from_be_bytes([bytes[cursor], bytes[cursor + 1]]);
        cursor += 2;
        let code_start = cursor;
        let Some(relative_end) = bytes[cursor..].iter().position(|byte| *byte == 0) else {
            return Err(TokenError::UnterminatedLine { line: line_number });
        };
        let code_end = cursor + relative_end;
        cursor = code_end + 1;
        let expected_u32 = u32::from(dialect.program_start)
            + u32::try_from(cursor).map_err(|_| TokenError::ProgramTooLarge {
                end: u32::MAX,
                maximum: dialect.maximum_program_address,
            })?;
        let expected = u16::try_from(expected_u32).map_err(|_| TokenError::ProgramTooLarge {
            end: expected_u32,
            maximum: dialect.maximum_program_address,
        })?;
        if link != expected {
            return Err(TokenError::InvalidLink {
                offset: record_offset,
                actual: link,
                expected,
            });
        }
        source.push_str(&line_number.to_string());
        source.push(' ');
        source.push_str(&detokenise_code(
            &bytes[code_start..code_end],
            code_start,
            &statements,
            &functions,
            &mut warnings,
        ));
        source.push('\n');
    }
    Err(TokenError::Truncated { offset: cursor })
}

fn detokenise_code(
    bytes: &[u8],
    base: usize,
    statements: &HashMap<u8, &str>,
    functions: &HashMap<u8, &str>,
    warnings: &mut Vec<TokenWarning>,
) -> String {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum State {
        Normal,
        String,
        Data,
        DataString,
        Comment,
    }
    let mut state = State::Normal;
    let mut output = String::new();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        match state {
            State::String => {
                push_literal_byte(&mut output, warnings, base + index, byte);
                if byte == b'"' {
                    state = State::Normal;
                }
            }
            State::Data => {
                push_literal_byte(&mut output, warnings, base + index, byte);
                if byte == b'"' {
                    state = State::DataString;
                } else if byte == b':' {
                    state = State::Normal;
                }
            }
            State::DataString => {
                push_literal_byte(&mut output, warnings, base + index, byte);
                if byte == b'"' {
                    state = State::Data;
                }
            }
            State::Comment => push_literal_byte(&mut output, warnings, base + index, byte),
            State::Normal => {
                if byte == b'"' {
                    output.push('"');
                    state = State::String;
                } else if byte == FUNCTION_PREFIX {
                    if let Some(function) = bytes.get(index + 1) {
                        index += 1;
                        if let Some(text) = functions.get(function) {
                            output.push_str(text);
                        } else {
                            let value = 0xff00_u16 | u16::from(*function);
                            output.push_str(&format!("{{{{TOKEN:0x{value:04X}}}}}"));
                            warnings.push(TokenWarning {
                                code: "DST2001",
                                offset: base + index - 1,
                                message: format!("unknown function token 0x{value:04X} retained"),
                            });
                        }
                    } else {
                        output.push_str("{{BYTE:0xFF}}");
                        warnings.push(TokenWarning {
                            code: "DST2002",
                            offset: base + index,
                            message: "dangling function-token prefix retained".to_owned(),
                        });
                    }
                } else if byte >= 0x80 {
                    if let Some(text) = statements.get(&byte) {
                        output.push_str(text);
                        if *text == "REM" || *text == "'" {
                            state = State::Comment;
                        } else if *text == "DATA" {
                            state = State::Data;
                        }
                    } else {
                        output.push_str(&format!("{{{{TOKEN:0x{byte:02X}}}}}"));
                        warnings.push(TokenWarning {
                            code: "DST2000",
                            offset: base + index,
                            message: format!("unknown token 0x{byte:02X} retained"),
                        });
                    }
                } else {
                    output.push(char::from(byte));
                }
            }
        }
        index += 1;
    }
    output
}

fn push_literal_byte(
    output: &mut String,
    warnings: &mut Vec<TokenWarning>,
    offset: usize,
    byte: u8,
) {
    if byte.is_ascii() {
        output.push(char::from(byte));
    } else {
        output.push_str(&format!("{{{{BYTE:0x{byte:02X}}}}}"));
        warnings.push(TokenWarning {
            code: "DST2003",
            offset,
            message: format!("non-ASCII literal byte 0x{byte:02X} retained"),
        });
    }
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyse;
    use proptest::prelude::*;

    fn dialect() -> Dialect {
        Dialect::dragon32().unwrap_or_else(|error| panic!("dialect: {error}"))
    }

    fn tokenise_source(source: &str) -> TokenisedProgram {
        let dialect = dialect();
        let analysis = analyse(source, &dialect);
        assert!(!analysis.has_errors(), "{:?}", analysis.diagnostics);
        tokenise(&analysis.program, &dialect).unwrap_or_else(|error| panic!("tokenise: {error}"))
    }

    #[test]
    fn hello_has_exact_linked_line_prefix() {
        let tokenised = tokenise_source("10 FOR I = 1 TO 10\n20 PRINT \"HELLO\"\n");
        assert_eq!(
            &tokenised.bytes[..17],
            &[
                0x1e, 0x12, 0x00, 0x0a, 0x80, 0x20, b'I', 0x20, 0xcb, 0x20, b'1', 0x20, 0xbc, 0x20,
                b'1', b'0', 0x00
            ]
        );
    }

    #[test]
    fn protects_strings_data_and_comments() {
        let tokenised =
            tokenise_source("10 PRINT \"GOTO DATA SIN\":DATA GOTO,SIN:REM PRINT GOTO\n");
        let code = &tokenised.lines[0].code;
        assert!(code.windows(9).any(|window| window == b"GOTO DATA"));
        let recovered = detokenise(&tokenised.bytes, &dialect())
            .unwrap_or_else(|error| panic!("detokenise: {error}"));
        assert_eq!(
            recovered.source,
            "10 PRINT \"GOTO DATA SIN\":DATA GOTO,SIN:REM PRINT GOTO\n"
        );
    }

    #[test]
    fn rejects_bad_link_and_trailing_bytes() {
        let mut tokenised = tokenise_source("10 END\n").bytes;
        tokenised[1] ^= 1;
        assert!(matches!(
            detokenise(&tokenised, &dialect()),
            Err(TokenError::InvalidLink { .. })
        ));

        let mut valid = tokenise_source("10 END\n").bytes;
        valid.push(1);
        assert!(matches!(
            detokenise(&valid, &dialect()),
            Err(TokenError::TrailingBytes { count: 1 })
        ));
    }

    #[test]
    fn unknown_tokens_round_trip_losslessly() {
        let mut tokenised = tokenise_source("10 END\n").bytes;
        tokenised[4] = 0xfe;
        let recovered = detokenise(&tokenised, &dialect())
            .unwrap_or_else(|error| panic!("detokenise: {error}"));
        assert_eq!(recovered.warnings.len(), 1);
        assert!(recovered.source.contains("{{TOKEN:0xFE}}"));
        let round_trip = tokenise_source(&recovered.source);
        assert_eq!(round_trip.bytes, tokenised);
    }

    #[test]
    fn protects_quoted_data_and_retains_dangling_prefix() {
        let tokenised = tokenise_source("10 DATA \"PRINT:GOTO\",SIN:GOTO20\n20 END\n");
        let recovered = detokenise(&tokenised.bytes, &dialect())
            .unwrap_or_else(|error| panic!("detokenise: {error}"));
        assert_eq!(
            recovered.source,
            "10 DATA \"PRINT:GOTO\",SIN:GOTO20\n20 END\n"
        );

        let mut malformed = tokenise_source("10 END\n").bytes;
        malformed[4] = 0xff;
        let recovered = detokenise(&malformed, &dialect())
            .unwrap_or_else(|error| panic!("detokenise malformed: {error}"));
        assert!(recovered.source.contains("{{BYTE:0xFF}}"));
        let round_trip = tokenise_source(&recovered.source);
        assert_eq!(round_trip.bytes, malformed);

        let mut literal = tokenise_source("10 REM X\n").bytes;
        literal[6] = 0xfe;
        let recovered = detokenise(&literal, &dialect())
            .unwrap_or_else(|error| panic!("detokenise literal: {error}"));
        assert!(recovered.source.contains("REM {{BYTE:0xFE}}"));
        assert_eq!(tokenise_source(&recovered.source).bytes, literal);
    }

    proptest! {
        #[test]
        fn canonical_round_trip_preserves_tokens(
            line in 1_u16..=63999,
            value in 0_u16..=9999,
            message in "[A-Z0-9 ]{0,24}"
        ) {
            let source = format!("{line} PRINT \"{message}\";{value}\n");
            let first = tokenise_source(&source);
            let recovered = detokenise(&first.bytes, &dialect())?;
            let second = tokenise_source(&recovered.source);
            prop_assert_eq!(second.bytes, first.bytes);
        }

        #[test]
        fn parser_does_not_panic_on_ascii(input in "[ -~\\r\\n]{0,2048}") {
            let _ = analyse(&input, &dialect());
        }
    }
}
