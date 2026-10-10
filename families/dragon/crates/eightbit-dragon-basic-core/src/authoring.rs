use crate::{Dialect, Severity, SourceRange, analyse, detokenise, tokenise};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// One non-overlapping replacement in the original source.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SourceEdit {
    /// Half-open byte range in the original ASCII source.
    pub range: SourceRange,
    /// Replacement text.
    pub replacement: String,
}

/// One old-to-new BASIC line-number mapping.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LineNumberChange {
    /// Existing line number.
    pub from: u16,
    /// Proposed line number.
    pub to: u16,
}

/// A complete source transformation suitable for preview before applying it atomically.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TransformPreview {
    /// Exact source revision on which the transformation was calculated.
    pub source: String,
    /// Result after all edits have been applied.
    pub result: String,
    /// Ordered edits against `source`.
    pub edits: Vec<SourceEdit>,
    /// Semantic line-number changes made by this transformation.
    pub line_changes: Vec<LineNumberChange>,
}

/// One source result in a project-wide transformation.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProjectSourceTransform {
    /// Exact source used for the preview.
    pub source: String,
    /// Result after project-wide semantic edits.
    pub result: String,
    /// Non-overlapping edits for this source.
    pub edits: Vec<SourceEdit>,
}

/// A project-wide transformation preserving source-file boundaries.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProjectTransformPreview {
    /// Results in the same order as the input sources.
    pub sources: Vec<ProjectSourceTransform>,
    /// One global old-to-new line-number mapping.
    pub line_changes: Vec<LineNumberChange>,
}

/// A source transformation could not be proved safe.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum TransformError {
    /// Existing source diagnostics make semantic editing unsafe.
    #[error("DSB4000: correct source errors before applying this change")]
    SourceErrors,
    /// Renumber settings are outside the dialect's supported range.
    #[error("DSB4001: {0}")]
    InvalidSequence(String),
    /// Requested source line is absent.
    #[error("DSB4002: line {0} does not exist")]
    UnknownLine(u16),
    /// Requested destination line is already occupied.
    #[error("DSB4003: line {0} already exists")]
    LineCollision(u16),
    /// Canonical formatting failed.
    #[error("DSB4004: source cannot be formatted: {0}")]
    Format(String),
}

/// Safely renumber every program line and every recognised semantic line reference.
pub fn renumber(
    source: &str,
    dialect: &Dialect,
    start: u16,
    step: u16,
) -> Result<TransformPreview, TransformError> {
    let analysis = checked_analysis(source, dialect)?;
    if start == 0 || step == 0 {
        return Err(TransformError::InvalidSequence(
            "start and step must both be greater than zero".to_owned(),
        ));
    }

    let mut line_changes = Vec::with_capacity(analysis.program.lines.len());
    let mut next = u32::from(start);
    for line in &analysis.program.lines {
        if next > u32::from(dialect.maximum_line_number) {
            return Err(TransformError::InvalidSequence(format!(
                "sequence exceeds maximum line number {}",
                dialect.maximum_line_number
            )));
        }
        line_changes.push(LineNumberChange {
            from: line.number,
            to: u16::try_from(next)
                .map_err(|_| TransformError::InvalidSequence("line-number overflow".to_owned()))?,
        });
        next = next.checked_add(u32::from(step)).ok_or_else(|| {
            TransformError::InvalidSequence("line-number sequence overflow".to_owned())
        })?;
    }

    let mapping = line_changes
        .iter()
        .map(|change| (change.from, change.to))
        .collect::<HashMap<_, _>>();
    let mut edits = Vec::new();
    for line in &analysis.program.lines {
        push_number_edit(&mut edits, line.number_range, mapping[&line.number]);
        for reference in &line.references {
            if let Some(target) = mapping.get(&reference.target) {
                push_number_edit(&mut edits, reference.range, *target);
            }
        }
    }
    let result = apply_edits(source, &edits);
    Ok(TransformPreview {
        source: source.to_owned(),
        result,
        edits,
        line_changes,
    })
}

/// Renumber one global BASIC program split across explicit source files.
pub fn renumber_project(
    sources: &[&str],
    dialect: &Dialect,
    start: u16,
    step: u16,
) -> Result<ProjectTransformPreview, TransformError> {
    if sources.is_empty() {
        return Err(TransformError::SourceErrors);
    }
    if start == 0 || step == 0 {
        return Err(TransformError::InvalidSequence(
            "start and step must both be greater than zero".to_owned(),
        ));
    }
    let analyses = sources
        .iter()
        .map(|source| analyse(source, dialect))
        .collect::<Vec<_>>();
    let has_global_gosub = analyses.iter().any(|analysis| {
        analysis.program.lines.iter().any(|line| {
            line.statements.iter().any(|statement| {
                statement
                    .text
                    .split(|character: char| !character.is_ascii_alphabetic())
                    .any(|word| {
                        word.eq_ignore_ascii_case("GOSUB") || word.eq_ignore_ascii_case("SUB")
                    })
            })
        })
    });
    if analyses.iter().any(|analysis| {
        analysis.diagnostics.iter().any(|diagnostic| {
            diagnostic.severity == Severity::Error
                && diagnostic.code != "DSB1001"
                && diagnostic.code != "DSB1002"
                && !(diagnostic.code == "DSB2004" && has_global_gosub)
        })
    }) {
        return Err(TransformError::SourceErrors);
    }

    let mut known = HashSet::new();
    let mut definitions = Vec::new();
    for (source_index, analysis) in analyses.iter().enumerate() {
        for line in &analysis.program.lines {
            if !known.insert(line.number) {
                return Err(TransformError::SourceErrors);
            }
            definitions.push((line.number, source_index, line.number_range));
        }
    }
    if analyses
        .iter()
        .flat_map(|analysis| &analysis.program.lines)
        .any(|line| {
            line.references
                .iter()
                .any(|reference| !known.contains(&reference.target))
        })
    {
        return Err(TransformError::SourceErrors);
    }
    definitions.sort_by_key(|(number, _, _)| *number);
    let mut line_changes = Vec::with_capacity(definitions.len());
    let mut next = u32::from(start);
    for (from, _, _) in &definitions {
        if next > u32::from(dialect.maximum_line_number) {
            return Err(TransformError::InvalidSequence(format!(
                "sequence exceeds maximum line number {}",
                dialect.maximum_line_number
            )));
        }
        line_changes.push(LineNumberChange {
            from: *from,
            to: u16::try_from(next)
                .map_err(|_| TransformError::InvalidSequence("line-number overflow".to_owned()))?,
        });
        next = next.checked_add(u32::from(step)).ok_or_else(|| {
            TransformError::InvalidSequence("line-number sequence overflow".to_owned())
        })?;
    }
    let mapping = line_changes
        .iter()
        .map(|change| (change.from, change.to))
        .collect::<HashMap<_, _>>();
    let transformed = sources
        .iter()
        .zip(analyses)
        .map(|(source, analysis)| {
            let mut edits = Vec::new();
            for line in &analysis.program.lines {
                push_number_edit(&mut edits, line.number_range, mapping[&line.number]);
                for reference in &line.references {
                    if let Some(target) = mapping.get(&reference.target) {
                        push_number_edit(&mut edits, reference.range, *target);
                    }
                }
            }
            ProjectSourceTransform {
                source: (*source).to_owned(),
                result: apply_edits(source, &edits),
                edits,
            }
        })
        .collect();
    Ok(ProjectTransformPreview {
        sources: transformed,
        line_changes,
    })
}

/// Rename one numbered line and all recognised semantic references to it.
pub fn rename_line(
    source: &str,
    dialect: &Dialect,
    from: u16,
    to: u16,
) -> Result<TransformPreview, TransformError> {
    let analysis = checked_analysis(source, dialect)?;
    if to == 0 || to > dialect.maximum_line_number {
        return Err(TransformError::InvalidSequence(format!(
            "line number must be between 1 and {}",
            dialect.maximum_line_number
        )));
    }
    let definition = analysis
        .program
        .lines
        .iter()
        .find(|line| line.number == from)
        .ok_or(TransformError::UnknownLine(from))?;
    if from != to && analysis.program.lines.iter().any(|line| line.number == to) {
        return Err(TransformError::LineCollision(to));
    }

    let mut edits = Vec::new();
    push_number_edit(&mut edits, definition.number_range, to);
    for line in &analysis.program.lines {
        for reference in &line.references {
            if reference.target == from {
                push_number_edit(&mut edits, reference.range, to);
            }
        }
    }
    let result = apply_edits(source, &edits);
    Ok(TransformPreview {
        source: source.to_owned(),
        result,
        edits,
        line_changes: vec![LineNumberChange { from, to }],
    })
}

/// Return the canonical Dragon BASIC representation of a valid listing.
pub fn format_source(source: &str, dialect: &Dialect) -> Result<String, TransformError> {
    let analysis = checked_analysis(source, dialect)?;
    let tokenised = tokenise(&analysis.program, dialect)
        .map_err(|error| TransformError::Format(error.to_string()))?;
    detokenise(&tokenised.bytes, dialect)
        .map(|program| program.source)
        .map_err(|error| TransformError::Format(error.to_string()))
}

fn checked_analysis(source: &str, dialect: &Dialect) -> Result<crate::Analysis, TransformError> {
    let analysis = analyse(source, dialect);
    if analysis
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        return Err(TransformError::SourceErrors);
    }
    Ok(analysis)
}

fn push_number_edit(edits: &mut Vec<SourceEdit>, range: SourceRange, number: u16) {
    edits.push(SourceEdit {
        range,
        replacement: number.to_string(),
    });
}

fn apply_edits(source: &str, edits: &[SourceEdit]) -> String {
    let mut ordered = edits.to_vec();
    ordered.sort_by_key(|edit| std::cmp::Reverse(edit.range.start));
    let mut output = source.to_owned();
    for edit in ordered {
        output.replace_range(edit.range.start..edit.range.end, &edit.replacement);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dialect() -> Dialect {
        Dialect::dragon32().unwrap_or_else(|error| panic!("dialect: {error}"))
    }

    #[test]
    fn renumber_changes_definitions_and_semantic_references_only() {
        let source = concat!(
            "100 PRINT \"GOTO 300\"\n",
            "200 DATA 300:GOTO 300:REM GOTO 100\n",
            "300 IF X THEN 100\n"
        );
        let preview = renumber(source, &dialect(), 10, 10)
            .unwrap_or_else(|error| panic!("renumber: {error}"));
        assert_eq!(
            preview.result,
            concat!(
                "10 PRINT \"GOTO 300\"\n",
                "20 DATA 300:GOTO 30:REM GOTO 100\n",
                "30 IF X THEN 10\n"
            )
        );
        assert_eq!(preview.line_changes.len(), 3);
    }

    #[test]
    fn renumbers_a_cross_file_program_in_global_line_order() {
        let preview = renumber_project(
            &[
                "100 GOSUB 1000\n200 END\n",
                "1000 PRINT \"SUB\"\n1010 RETURN\n",
            ],
            &dialect(),
            10,
            10,
        )
        .unwrap_or_else(|error| panic!("project renumber: {error}"));
        assert_eq!(preview.sources[0].result, "10 GOSUB 30\n20 END\n");
        assert_eq!(preview.sources[1].result, "30 PRINT \"SUB\"\n40 RETURN\n");
        assert_eq!(
            preview.line_changes[2],
            LineNumberChange { from: 1000, to: 30 }
        );
    }

    #[test]
    fn rejects_overflow_and_source_errors() {
        let overflow =
            renumber("10 END\n20 END\n", &dialect(), 63_990, 10).expect_err("overflow must fail");
        assert!(matches!(overflow, TransformError::InvalidSequence(_)));
        assert_eq!(
            renumber("10 GOTO 999\n", &dialect(), 10, 10),
            Err(TransformError::SourceErrors)
        );
    }

    #[test]
    fn rename_updates_references_and_rejects_collisions() {
        let source = "10 GOTO 30\n20 END\n30 GOTO 10\n";
        let preview = rename_line(source, &dialect(), 30, 40)
            .unwrap_or_else(|error| panic!("rename: {error}"));
        assert_eq!(preview.result, "10 GOTO 40\n20 END\n40 GOTO 10\n");
        assert_eq!(
            rename_line(source, &dialect(), 30, 20),
            Err(TransformError::LineCollision(20))
        );
    }

    #[test]
    fn formatting_is_canonical_and_preserves_protected_text() {
        let formatted = format_source("10 print \"hello\"\n20 goto 10\n", &dialect())
            .unwrap_or_else(|error| panic!("format: {error}"));
        assert_eq!(formatted, "10 PRINT \"hello\"\n20 GOTO 10\n");
    }
}
