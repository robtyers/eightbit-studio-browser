//! Dragon 32 BASIC source analysis and linked-line token conversion.

#![cfg_attr(test, allow(clippy::expect_used, clippy::panic, clippy::unwrap_used))]

mod authoring;
mod dialect;
mod parser;
mod tokenise;

pub use authoring::{
    LineNumberChange, ProjectSourceTransform, ProjectTransformPreview, SourceEdit, TransformError,
    TransformPreview, format_source, rename_line, renumber, renumber_project,
};
pub use dialect::{Dialect, DialectError, TokenDefinition};
pub use parser::{
    Analysis, BasicLine, Diagnostic, LineReference, Program, Severity, SourceRange, Statement,
    analyse,
};
pub use tokenise::{
    ConversionReport, DetokenisedProgram, TokenError, TokenWarning, TokenisedLine,
    TokenisedProgram, TokenisedStatement, detokenise, tokenise,
};

/// Largest accepted BASIC listing in I1.
pub const MAX_SOURCE_BYTES: usize = 1024 * 1024;

/// Validate the manifest's configured BASIC entry using the same rule in every host.
#[must_use]
pub fn configured_entry_diagnostic(
    entry: &Program,
    first_line: Option<u16>,
    entry_line: u16,
) -> Option<Diagnostic> {
    let definition = entry.lines.iter().find(|line| line.number == entry_line);
    if definition.is_some() && first_line == Some(entry_line) {
        return None;
    }
    Some(Diagnostic {
        code: "DSB1005",
        severity: Severity::Error,
        message: if definition.is_none() {
            format!("basic.entry_line {entry_line} is not defined in basic.entry")
        } else {
            format!(
                "basic.entry_line {entry_line} must be the lowest configured BASIC line; found {}",
                first_line.unwrap_or(entry_line)
            )
        },
        range: definition.map_or(SourceRange { start: 0, end: 0 }, |line| line.number_range),
        row: definition.map_or(1, |line| line.row),
        column: 1,
    })
}
