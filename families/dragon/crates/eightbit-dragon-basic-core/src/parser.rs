use crate::{Dialect, MAX_SOURCE_BYTES};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

/// A half-open byte range in the original ASCII source.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct SourceRange {
    /// Inclusive byte offset.
    pub start: usize,
    /// Exclusive byte offset.
    pub end: usize,
}

/// Diagnostic severity.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Build-blocking error.
    Error,
    /// Conservative warning.
    Warning,
    /// Informational observation.
    Information,
}

/// Stable Dragon BASIC diagnostic.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Diagnostic {
    /// Stable diagnostic code.
    pub code: &'static str,
    /// Severity.
    pub severity: Severity,
    /// Actionable explanation.
    pub message: String,
    /// Original source range.
    pub range: SourceRange,
    /// One-based source row.
    pub row: usize,
    /// One-based source column.
    pub column: usize,
}

/// A semantic numbered-line reference.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LineReference {
    /// Target line number.
    pub target: u16,
    /// Exact numeric source range.
    pub range: SourceRange,
}

/// A colon-separated BASIC statement with retained source position.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Statement {
    /// Zero-based ordinal within its numbered line.
    pub ordinal: usize,
    /// Statement text without its colon separator.
    pub text: String,
    /// Original source range.
    pub range: SourceRange,
}

/// One numbered BASIC line.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BasicLine {
    /// BASIC line number.
    pub number: u16,
    /// One-based physical source row.
    pub row: usize,
    /// Code after the line number and separating whitespace.
    pub code: String,
    /// Range of the line number.
    pub number_range: SourceRange,
    /// Range of the code portion.
    pub code_range: SourceRange,
    /// Colon-separated statements.
    pub statements: Vec<Statement>,
    /// Resolved semantic target candidates.
    pub references: Vec<LineReference>,
}

/// Recovered BASIC program.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Program {
    /// Numbered source lines in source order.
    pub lines: Vec<BasicLine>,
}

/// Parser and semantic-analysis result.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Analysis {
    /// Recovered program, including after local syntax errors.
    pub program: Program,
    /// Sorted diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

impl Analysis {
    /// Whether analysis contains at least one build-blocking error.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
    }
}

const MAX_DIAGNOSTICS: usize = 4096;

#[derive(Default)]
struct Diagnostics(Vec<Diagnostic>);
impl Diagnostics {
    fn push(&mut self, diagnostic: Diagnostic) {
        if self.0.len() < MAX_DIAGNOSTICS - 1 {
            self.0.push(diagnostic);
        } else if self.0.len() < MAX_DIAGNOSTICS {
            self.0.push(Diagnostic {
                code: "DSB0003",
                severity: Severity::Error,
                message: "diagnostic limit reached; correct these errors and analyse again".into(),
                ..diagnostic
            });
        }
    }
}

/// Parse and semantically analyse an ASCII Dragon BASIC listing.
#[must_use]
pub fn analyse(source: &str, dialect: &Dialect) -> Analysis {
    let mut diagnostics = Diagnostics::default();
    if source.len() > MAX_SOURCE_BYTES {
        diagnostics.push(Diagnostic {
            code: "DSB0001",
            severity: Severity::Error,
            message: format!("source exceeds the {MAX_SOURCE_BYTES}-byte limit"),
            range: SourceRange { start: 0, end: 0 },
            row: 1,
            column: 1,
        });
        return Analysis {
            program: Program { lines: Vec::new() },
            diagnostics: diagnostics.0,
        };
    }
    if !source.is_ascii() {
        let offset = source
            .bytes()
            .position(|byte| !byte.is_ascii())
            .unwrap_or(0);
        let (row, column) = row_column(source, offset);
        diagnostics.push(Diagnostic {
            code: "DSB0002",
            severity: Severity::Error,
            message: "source_encoding='ascii' does not permit this byte".to_owned(),
            range: SourceRange {
                start: offset,
                end: offset.saturating_add(1),
            },
            row,
            column,
        });
        return Analysis {
            program: Program { lines: Vec::new() },
            diagnostics: diagnostics.0,
        };
    }

    let mut lines = Vec::new();
    let mut offset = 0;
    for (row_index, physical) in source.split_inclusive('\n').enumerate() {
        let physical_without_lf = physical.strip_suffix('\n').unwrap_or(physical);
        let text = physical_without_lf
            .strip_suffix('\r')
            .unwrap_or(physical_without_lf);
        parse_physical_line(
            text,
            offset,
            row_index + 1,
            dialect,
            &mut lines,
            &mut diagnostics,
        );
        offset += physical.len();
    }
    if lines.is_empty()
        && !diagnostics
            .0
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        diagnostics.push(Diagnostic {
            code: "DSB1000",
            severity: Severity::Error,
            message: "the BASIC listing contains no numbered lines".to_owned(),
            range: SourceRange { start: 0, end: 0 },
            row: 1,
            column: 1,
        });
    }

    let line_starts: Vec<usize> = std::iter::once(0)
        .chain(
            source
                .bytes()
                .enumerate()
                .filter_map(|(index, byte)| (byte == b'\n').then_some(index + 1)),
        )
        .collect();
    add_program_diagnostics(&line_starts, &lines, &mut diagnostics);
    diagnostics
        .0
        .sort_by_key(|diagnostic| (diagnostic.range.start, diagnostic.code));
    Analysis {
        program: Program { lines },
        diagnostics: diagnostics.0,
    }
}

fn parse_physical_line(
    text: &str,
    base: usize,
    row: usize,
    dialect: &Dialect,
    lines: &mut Vec<BasicLine>,
    diagnostics: &mut Diagnostics,
) {
    if text.trim().is_empty() {
        return;
    }
    let leading = text.len() - text.trim_start_matches([' ', '\t']).len();
    let digits = text[leading..]
        .bytes()
        .take_while(u8::is_ascii_digit)
        .count();
    if digits == 0 {
        diagnostics.push(Diagnostic {
            code: "DSB1000",
            severity: Severity::Error,
            message: "a numbered BASIC line must start with a decimal line number".to_owned(),
            range: SourceRange {
                start: base + leading,
                end: base + text.len(),
            },
            row,
            column: leading + 1,
        });
        return;
    }
    let number_text = &text[leading..leading + digits];
    let number = match number_text.parse::<u16>() {
        Ok(value) if value > 0 && value <= dialect.maximum_line_number => value,
        _ => {
            diagnostics.push(Diagnostic {
                code: "DSB1001",
                severity: Severity::Error,
                message: format!(
                    "line number must be between 1 and {}",
                    dialect.maximum_line_number
                ),
                range: SourceRange {
                    start: base + leading,
                    end: base + leading + digits,
                },
                row,
                column: leading + 1,
            });
            return;
        }
    };
    let mut code_start = leading + digits;
    while code_start < text.len() && matches!(text.as_bytes()[code_start], b' ' | b'\t') {
        code_start += 1;
    }
    let code = &text[code_start..];
    if let Some(column) = unterminated_string(code) {
        diagnostics.push(Diagnostic {
            code: "DSB1004",
            severity: Severity::Error,
            message: "unterminated string literal".to_owned(),
            range: SourceRange {
                start: base + code_start + column,
                end: base + text.len(),
            },
            row,
            column: code_start + column + 1,
        });
    }
    let statements = split_statements(code, base + code_start);
    add_statement_diagnostics(base, row, &statements, dialect, diagnostics);
    let references = statements
        .iter()
        .flat_map(|statement| find_references(&statement.text, statement.range.start))
        .collect();
    lines.push(BasicLine {
        number,
        row,
        code: code.to_owned(),
        number_range: SourceRange {
            start: base + leading,
            end: base + leading + digits,
        },
        code_range: SourceRange {
            start: base + code_start,
            end: base + text.len(),
        },
        statements,
        references,
    });
}

fn unterminated_string(code: &str) -> Option<usize> {
    let mut opening = None;
    for (index, byte) in code.bytes().enumerate() {
        if byte == b'"' {
            opening = if opening.is_some() { None } else { Some(index) };
        }
    }
    opening
}

fn split_statements(code: &str, base: usize) -> Vec<Statement> {
    let bytes = code.as_bytes();
    let mut starts = vec![0];
    let mut in_string = false;
    let mut in_comment = false;
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'"' if !in_comment => in_string = !in_string,
            b'\'' if !in_string => in_comment = true,
            b':' if !in_string && !in_comment => starts.push(index + 1),
            _ if !in_string && !in_comment && starts_with_word(code, index, "REM") => {
                in_comment = true;
                index += 2;
            }
            _ => {}
        }
        index += 1;
    }
    let mut statements = Vec::new();
    for (ordinal, start) in starts.iter().copied().enumerate() {
        let end = starts
            .get(ordinal + 1)
            .map_or(code.len(), |next| next.saturating_sub(1));
        statements.push(Statement {
            ordinal,
            text: code[start..end].to_owned(),
            range: SourceRange {
                start: base + start,
                end: base + end,
            },
        });
    }
    statements
}

fn add_statement_diagnostics(
    line_base: usize,
    row: usize,
    statements: &[Statement],
    dialect: &Dialect,
    diagnostics: &mut Diagnostics,
) {
    for statement in statements {
        if let Some(argument) = unsupported_restore_argument(&statement.text) {
            let range = SourceRange {
                start: statement.range.start + argument.start,
                end: statement.range.start + argument.end,
            };
            diagnostics.push(Diagnostic {
                code: "DSB1007",
                severity: Severity::Error,
                message: "RESTORE takes no line-number argument in standard Dragon 32 BASIC"
                    .to_owned(),
                range,
                row,
                column: range.start.saturating_sub(line_base) + 1,
            });
            continue;
        }
        let leading = statement
            .text
            .len()
            .saturating_sub(statement.text.trim_start_matches([' ', '\t']).len());
        let candidate = &statement.text[leading..];
        if candidate.is_empty()
            || candidate.starts_with('\'')
            || candidate.starts_with("{{TOKEN:0x")
            || candidate.starts_with("{{BYTE:0x")
        {
            continue;
        }
        let word_length = candidate
            .bytes()
            .take_while(u8::is_ascii_alphabetic)
            .count();
        if word_length == 0 {
            continue;
        }
        let word = &candidate[..word_length];
        if is_statement_keyword(word, dialect) || is_implicit_assignment(candidate, word_length) {
            continue;
        }

        let range = SourceRange {
            start: statement.range.start + leading,
            end: statement.range.start + leading + word_length,
        };
        let suggestion = closest_statement_keyword(word, dialect)
            .map_or_else(String::new, |keyword| format!("; did you mean {keyword}?"));
        diagnostics.push(Diagnostic {
            code: "DSB1006",
            severity: Severity::Error,
            message: format!(
                "unknown statement '{word}'{suggestion}; use a Dragon 32 BASIC keyword or an assignment"
            ),
            range,
            row,
            column: range.start.saturating_sub(line_base) + 1,
        });
    }
}

fn unsupported_restore_argument(statement: &str) -> Option<SourceRange> {
    let lexemes = semantic_lexemes(statement);
    for (index, lexeme) in lexemes.iter().enumerate() {
        if !matches!(&lexeme.kind, LexemeKind::Word(word) if word == "RESTORE") {
            continue;
        }
        let argument = lexemes.get(index + 1)?;
        if matches!(&argument.kind, LexemeKind::Word(word) if word == "ELSE") {
            continue;
        }
        return Some(SourceRange {
            start: argument.start,
            end: argument.end,
        });
    }
    None
}

fn is_statement_keyword(word: &str, dialect: &Dialect) -> bool {
    matches!(word.to_ascii_uppercase().as_str(), "GOTO" | "GOSUB")
        || dialect
            .statement_tokens
            .iter()
            .any(|token| token.text.trim_end_matches('(').eq_ignore_ascii_case(word))
}

fn is_implicit_assignment(statement: &str, word_end: usize) -> bool {
    let bytes = statement.as_bytes();
    let mut index = word_end;
    while index < bytes.len() && bytes[index].is_ascii_alphanumeric() {
        index += 1;
    }
    if index < bytes.len() && matches!(bytes[index], b'$' | b'%') {
        index += 1;
    }
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    if index < bytes.len() && bytes[index] == b'(' {
        let mut depth = 0_usize;
        let mut in_string = false;
        while index < bytes.len() {
            match bytes[index] {
                b'"' => in_string = !in_string,
                b'(' if !in_string => depth += 1,
                b')' if !in_string => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        index += 1;
                        break;
                    }
                }
                _ => {}
            }
            index += 1;
        }
        if depth != 0 {
            return false;
        }
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
    }
    bytes.get(index) == Some(&b'=')
}

fn closest_statement_keyword<'a>(word: &str, dialect: &'a Dialect) -> Option<&'a str> {
    if word.len() > 32 {
        return None;
    }
    let upper = word.to_ascii_uppercase();
    dialect
        .statement_tokens
        .iter()
        .filter_map(|token| {
            let keyword = token.text.trim_end_matches('(');
            keyword
                .bytes()
                .all(|byte| byte.is_ascii_alphabetic())
                .then(|| (edit_distance(&upper, keyword), keyword))
        })
        .filter(|(distance, keyword)| *distance <= 1 || (*distance == 2 && keyword.len() >= 6))
        .min_by(std::cmp::Ord::cmp)
        .map(|(_, keyword)| keyword)
}

fn edit_distance(left: &str, right: &str) -> usize {
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    let mut current = vec![0; right.len() + 1];
    for (left_index, left_byte) in left.bytes().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_byte) in right.bytes().enumerate() {
            current[right_index + 1] = (previous[right_index + 1] + 1)
                .min(current[right_index] + 1)
                .min(previous[right_index] + usize::from(left_byte != right_byte));
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right.len()]
}

#[derive(Debug)]
enum LexemeKind {
    Word(String),
    Number(u16),
    Comma,
    Other,
}

#[derive(Debug)]
struct Lexeme {
    kind: LexemeKind,
    start: usize,
    end: usize,
}

fn find_references(statement: &str, base: usize) -> Vec<LineReference> {
    let lexemes = semantic_lexemes(statement);
    let mut references = Vec::new();
    let mut index = 0;
    while index < lexemes.len() {
        let word = if let LexemeKind::Word(word) = &lexemes[index].kind {
            word.as_str()
        } else {
            index += 1;
            continue;
        };
        let mut target_start = None;
        let mut list = false;
        match word {
            "GOTO" | "GOSUB" => {
                target_start = Some(index + 1);
                list = has_word_before(&lexemes, index, "ON");
            }
            "GO" if next_word(&lexemes, index) == Some("TO")
                || next_word(&lexemes, index) == Some("SUB") =>
            {
                target_start = Some(index + 2);
                list = has_word_before(&lexemes, index, "ON");
            }
            "THEN" | "ELSE" | "RUN" => target_start = Some(index + 1),
            _ => {}
        }
        if let Some(start) = target_start {
            if list {
                let mut cursor = start;
                while cursor < lexemes.len() {
                    if let LexemeKind::Number(target) = lexemes[cursor].kind {
                        references.push(reference(base, &lexemes[cursor], target));
                    } else if !matches!(lexemes[cursor].kind, LexemeKind::Comma) {
                        break;
                    }
                    cursor += 1;
                }
            } else if let Some(lexeme) = lexemes.get(start)
                && let LexemeKind::Number(target) = lexeme.kind
            {
                references.push(reference(base, lexeme, target));
            }
        }
        index += 1;
    }
    references
}

fn reference(base: usize, lexeme: &Lexeme, target: u16) -> LineReference {
    LineReference {
        target,
        range: SourceRange {
            start: base + lexeme.start,
            end: base + lexeme.end,
        },
    }
}

fn has_word_before(lexemes: &[Lexeme], index: usize, expected: &str) -> bool {
    lexemes[..index]
        .iter()
        .any(|lexeme| matches!(&lexeme.kind, LexemeKind::Word(word) if word == expected))
}

fn next_word(lexemes: &[Lexeme], index: usize) -> Option<&str> {
    lexemes[index + 1..]
        .iter()
        .find_map(|lexeme| match &lexeme.kind {
            LexemeKind::Word(word) => Some(word.as_str()),
            _ => None,
        })
}

fn semantic_lexemes(statement: &str) -> Vec<Lexeme> {
    let bytes = statement.as_bytes();
    let mut output = Vec::new();
    let mut index = 0;
    let mut in_string = false;
    while index < bytes.len() {
        if bytes[index].is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if bytes[index] == b'"' {
            in_string = !in_string;
            index += 1;
            continue;
        }
        if in_string {
            index += 1;
            continue;
        }
        if bytes[index] == b'\'' || starts_with_word(statement, index, "REM") {
            break;
        }
        if starts_with_word(statement, index, "DATA") {
            break;
        }
        if bytes[index].is_ascii_alphabetic() {
            let start = index;
            index += 1;
            while index < bytes.len() && bytes[index].is_ascii_alphabetic() {
                index += 1;
            }
            output.push(Lexeme {
                kind: LexemeKind::Word(statement[start..index].to_ascii_uppercase()),
                start,
                end: index,
            });
        } else if bytes[index].is_ascii_digit() {
            let start = index;
            index += 1;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }
            let kind = statement[start..index]
                .parse::<u16>()
                .map_or(LexemeKind::Other, LexemeKind::Number);
            output.push(Lexeme {
                kind,
                start,
                end: index,
            });
        } else {
            output.push(Lexeme {
                kind: if bytes[index] == b',' {
                    LexemeKind::Comma
                } else {
                    LexemeKind::Other
                },
                start: index,
                end: index + 1,
            });
            index += 1;
        }
    }
    output
}

fn add_program_diagnostics(
    line_starts: &[usize],
    lines: &[BasicLine],
    diagnostics: &mut Diagnostics,
) {
    let mut known = HashSet::new();
    let mut first_definition = HashMap::new();
    let mut previous = None;
    for line in lines {
        if let Some(first) = first_definition.insert(line.number, line.number_range) {
            diagnostics.push(diagnostic_at(
                line_starts,
                "DSB1001",
                Severity::Error,
                format!(
                    "duplicate line {}; first defined at byte {}",
                    line.number, first.start
                ),
                line.number_range,
            ));
        }
        if previous.is_some_and(|number| line.number <= number) {
            diagnostics.push(diagnostic_at(
                line_starts,
                "DSB1003",
                Severity::Error,
                format!(
                    "line {} is not greater than the preceding line",
                    line.number
                ),
                line.number_range,
            ));
        }
        previous = Some(line.number);
        known.insert(line.number);
    }
    for line in lines {
        for reference in &line.references {
            if !known.contains(&reference.target) {
                diagnostics.push(diagnostic_at(
                    line_starts,
                    "DSB1002",
                    Severity::Error,
                    format!("target line {} does not exist", reference.target),
                    reference.range,
                ));
            }
        }
    }
    add_loop_diagnostics(line_starts, lines, diagnostics);
}

fn add_loop_diagnostics(line_starts: &[usize], lines: &[BasicLine], diagnostics: &mut Diagnostics) {
    let mut stack: Vec<(String, SourceRange)> = Vec::new();
    let mut any_gosub = false;
    let mut returns = Vec::new();
    for line in lines {
        for statement in &line.statements {
            let lexemes = semantic_lexemes(&statement.text);
            let first = lexemes.iter().find_map(|lexeme| match &lexeme.kind {
                LexemeKind::Word(word) => Some((word.as_str(), lexeme)),
                _ => None,
            });
            match first {
                Some(("FOR", _)) => {
                    if let Some(variable) = lexemes.iter().skip(1).find_map(|lexeme| match &lexeme
                        .kind
                    {
                        LexemeKind::Word(word) => Some((word.clone(), lexeme)),
                        _ => None,
                    }) {
                        stack.push((
                            variable.0,
                            SourceRange {
                                start: statement.range.start + variable.1.start,
                                end: statement.range.start + variable.1.end,
                            },
                        ));
                    }
                }
                Some(("NEXT", first_lexeme)) => {
                    let next_variable =
                        lexemes
                            .iter()
                            .skip(1)
                            .find_map(|lexeme| match &lexeme.kind {
                                LexemeKind::Word(word) => Some(word.as_str()),
                                _ => None,
                            });
                    match stack.pop() {
                        None => diagnostics.push(diagnostic_at(
                            line_starts,
                            "DSB2001",
                            Severity::Error,
                            "NEXT has no statically visible FOR".to_owned(),
                            SourceRange {
                                start: statement.range.start + first_lexeme.start,
                                end: statement.range.start + first_lexeme.end,
                            },
                        )),
                        Some((for_variable, _))
                            if next_variable
                                .is_some_and(|next| !next.eq_ignore_ascii_case(&for_variable)) =>
                        {
                            diagnostics.push(diagnostic_at(
                                line_starts,
                                "DSB2002",
                                Severity::Warning,
                                format!(
                                    "NEXT variable '{}' does not match active FOR variable '{for_variable}'",
                                    next_variable.unwrap_or_default()
                                ),
                                statement.range,
                            ));
                        }
                        Some(_) => {}
                    }
                }
                Some(("RETURN", lexeme)) => returns.push(SourceRange {
                    start: statement.range.start + lexeme.start,
                    end: statement.range.start + lexeme.end,
                }),
                _ => {}
            }
            any_gosub |= lexemes.iter().any(|lexeme| {
                matches!(&lexeme.kind, LexemeKind::Word(word) if word == "GOSUB" || word == "SUB")
            });
        }
    }
    for (_, range) in stack {
        diagnostics.push(diagnostic_at(
            line_starts,
            "DSB2003",
            Severity::Warning,
            "FOR has no statically visible NEXT".to_owned(),
            range,
        ));
    }
    if !any_gosub {
        for range in returns {
            diagnostics.push(diagnostic_at(
                line_starts,
                "DSB2004",
                Severity::Warning,
                "RETURN appears in a program with no GOSUB".to_owned(),
                range,
            ));
        }
    }
}

fn diagnostic_at(
    line_starts: &[usize],
    code: &'static str,
    severity: Severity,
    message: String,
    range: SourceRange,
) -> Diagnostic {
    let (row, column) = indexed_position(line_starts, range.start);
    Diagnostic {
        code,
        severity,
        message,
        range,
        row,
        column,
    }
}

fn indexed_position(line_starts: &[usize], offset: usize) -> (usize, usize) {
    let row = line_starts.partition_point(|start| *start <= offset);
    (row, offset - line_starts[row.saturating_sub(1)] + 1)
}

fn row_column(source: &str, offset: usize) -> (usize, usize) {
    let prefix = &source.as_bytes()[..offset.min(source.len())];
    let row = prefix.iter().filter(|byte| **byte == b'\n').count() + 1;
    let column = prefix
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(prefix.len() + 1, |position| prefix.len() - position);
    (row, column)
}

fn starts_with_word(text: &str, index: usize, word: &str) -> bool {
    let bytes = text.as_bytes();
    if index + word.len() > bytes.len()
        || !text[index..index + word.len()].eq_ignore_ascii_case(word)
    {
        return false;
    }
    let before_ok = index == 0 || !is_name_byte(bytes[index - 1]);
    let after = index + word.len();
    let after_ok = after == bytes.len() || !is_name_byte(bytes[after]);
    before_ok && after_ok
}

fn is_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$' | b'%')
}

#[cfg(test)]
mod tests {

    #[test]
    fn dense_malformed_listings_keep_bounded_diagnostics_and_remain_errors() {
        let result = analyse(&"1 END\n".repeat(40_000), &dialect());
        assert_eq!(result.diagnostics.len(), MAX_DIAGNOSTICS);
        assert!(result.has_errors());
        assert_eq!(
            result
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == "DSB0003")
                .count(),
            1
        );
        let invalid = analyse(&"invalid\n".repeat(40_000), &dialect());
        assert_eq!(invalid.diagnostics.len(), MAX_DIAGNOSTICS);
        assert!(invalid.has_errors());
    }
    #[test]
    fn indexed_semantic_positions_preserve_blank_lines_crlf_and_eof() {
        let source = "\r\n10 END\r\n\r\n10 GOTO 999";
        let result = analyse(source, &dialect());
        for diagnostic in &result.diagnostics {
            assert_eq!(
                (diagnostic.row, diagnostic.column),
                row_column(source, diagnostic.range.start)
            );
        }
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "DSB1002" && diagnostic.row == 4)
        );
    }

    use super::*;

    fn dialect() -> Dialect {
        Dialect::dragon32().unwrap_or_else(|error| panic!("dialect: {error}"))
    }

    #[test]
    fn reports_hello_undefined_target_at_exact_number() {
        let source = concat!(
            "10 COUNT=0\n",
            "20 MESSAGE$=\"HELLO\"\n",
            "30 COUNT=COUNT+1:PRINT MESSAGE$;COUNT\n",
            "40 IF COUNT<3 THEN 30\n",
            "50 GOTO 999\n"
        );
        let analysis = analyse(source, &dialect());
        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "DSB1002")
            .unwrap_or_else(|| panic!("missing DSB1002: {:?}", analysis.diagnostics));
        assert_eq!(&source[diagnostic.range.start..diagnostic.range.end], "999");
        assert_eq!((diagnostic.row, diagnostic.column), (5, 9));
    }

    #[test]
    fn finds_direct_and_on_target_forms_but_not_literals() {
        let source = concat!(
            "10 ON X GOTO 100,200\n",
            "20 GOSUB 300:IF X THEN 100 ELSE 200\n",
            "30 DATA 100,200:GOTO 400\n",
            "40 PRINT \"GOTO 500\":REM GOTO 600\n",
            "100 END\n200 END\n300 RETURN\n400 END\n"
        );
        let analysis = analyse(source, &dialect());
        assert!(!analysis.has_errors(), "{:?}", analysis.diagnostics);
        let targets = analysis
            .program
            .lines
            .iter()
            .flat_map(|line| line.references.iter().map(|reference| reference.target))
            .collect::<Vec<_>>();
        assert_eq!(targets, vec![100, 200, 300, 100, 200, 400]);
    }

    #[test]
    fn rejects_restore_line_arguments_but_accepts_bare_restore() {
        let source = "10 RESTORE 30\n20 IF X THEN RESTORE 30\n30 RESTORE\n";
        let analysis = analyse(source, &dialect());
        let diagnostics = analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "DSB1007")
            .collect::<Vec<_>>();

        assert_eq!(diagnostics.len(), 2, "{:?}", analysis.diagnostics);
        assert_eq!(
            &source[diagnostics[0].range.start..diagnostics[0].range.end],
            "30"
        );
        assert_eq!(
            &source[diagnostics[1].range.start..diagnostics[1].range.end],
            "30"
        );
        assert!(
            analysis
                .program
                .lines
                .iter()
                .all(|line| line.references.is_empty())
        );
    }

    #[test]
    fn preserves_statements_inside_strings_and_comments() {
        let source = "10 PRINT \"A:B\":DATA \"C:D\",1:REM E:F\n";
        let analysis = analyse(source, &dialect());
        assert_eq!(analysis.program.lines[0].statements.len(), 3);
    }

    #[test]
    fn recovers_after_malformed_line() {
        let source = "PRINT \"BAD\"\n20 GOTO 99\n30 END\n";
        let analysis = analyse(source, &dialect());
        assert_eq!(analysis.program.lines.len(), 2);
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "DSB1000")
        );
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "DSB1002")
        );
    }

    #[test]
    fn recognises_compact_line_targets_and_rejects_blank_listing() {
        let analysis = analyse("10 IF X THEN30\n30 GOTO10\n", &dialect());
        assert!(!analysis.has_errors(), "{:?}", analysis.diagnostics);
        let targets = analysis
            .program
            .lines
            .iter()
            .flat_map(|line| line.references.iter().map(|reference| reference.target))
            .collect::<Vec<_>>();
        assert_eq!(targets, vec![30, 10]);

        let blank = analyse(" \n\t\n", &dialect());
        assert!(blank.has_errors());
        assert_eq!(blank.diagnostics[0].code, "DSB1000");
    }

    #[test]
    fn rejects_unknown_statements_without_rejecting_implicit_assignments() {
        let source = concat!(
            "10 PRONT \"HELLO\"\n",
            "20 PRONT=1\n",
            "30 NAME$ = \"DRAGON\"\n",
            "40 VALUES(1,2)=3\n",
            "50 END\n",
        );
        let analysis = analyse(source, &dialect());
        let diagnostics = analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "DSB1006")
            .collect::<Vec<_>>();

        assert_eq!(diagnostics.len(), 1, "{:?}", analysis.diagnostics);
        assert_eq!(
            &source[diagnostics[0].range.start..diagnostics[0].range.end],
            "PRONT"
        );
        assert_eq!((diagnostics[0].row, diagnostics[0].column), (1, 4));
        assert!(diagnostics[0].message.contains("did you mean PRINT?"));
    }
}
