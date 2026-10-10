//! Display-only source ranges use the existing parser and built-in dialect table.
//! Partial colouring never validates a programme or authorises a build.

use eightbit_dragon_basic_core::{Dialect, analyse};
use serde_json::{Value, json};

pub(super) fn source_highlights(source: &str) -> Value {
    // Larger or non-ASCII documents retain native plain-text editing.
    if source.len() > 65_536 || !source.is_ascii() {
        return json!({"highlights":[]});
    }
    let Ok(dialect) = Dialect::dragon32() else {
        return json!({"highlights":[]});
    };
    let mut highlights = Vec::new();
    for line in analyse(source, &dialect).program.lines {
        highlights.push(
            json!({"start":line.number_range.start,"end":line.number_range.end,"kind":"number"}),
        );
        for statement in line.statements {
            let Some(text) = source.get(statement.range.start..statement.range.end) else {
                continue;
            };
            let trimmed = text.trim_start();
            let start = statement.range.start + text.len() - trimmed.len();
            let token = dialect
                .statement_tokens
                .iter()
                .filter(|token| {
                    trimmed
                        .get(..token.text.len())
                        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(&token.text))
                })
                .max_by_key(|token| token.text.len());
            if token.is_some_and(|token| token.text == "REM" || token.text == "'") {
                highlights.push(json!({"start":start,"end":statement.range.end,"kind":"comment"}));
                continue;
            }
            if let Some(token) = token {
                highlights
                    .push(json!({"start":start,"end":start + token.text.len(),"kind":"keyword"}));
            }
            let mut quoted = None;
            for (offset, byte) in text.bytes().enumerate() {
                if byte == b'"' {
                    if let Some(begin) = quoted.take() {
                        highlights.push(json!({"start":begin,"end":statement.range.start + offset + 1,"kind":"string"}));
                    } else {
                        quoted = Some(statement.range.start + offset);
                    }
                }
            }
            if highlights.len() > 6000 {
                return json!({"highlights":[]});
            }
        }
        if highlights.len() > 6000 {
            return json!({"highlights":[]});
        }
    }
    json!({"highlights":highlights})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presentation_uses_original_ranges_without_colouring_keywords_inside_strings_or_comments() {
        let source = "10 PRINT \"REM END\":REM \"PRINT\"\n20 END\n";
        let value = source_highlights(source);
        assert_eq!(
            value["highlights"],
            json!([
                {"start":0,"end":2,"kind":"number"},
                {"start":3,"end":8,"kind":"keyword"},
                {"start":9,"end":18,"kind":"string"},
                {"start":19,"end":30,"kind":"comment"},
                {"start":31,"end":33,"kind":"number"},
                {"start":34,"end":37,"kind":"keyword"}
            ])
        );
    }

    #[test]
    fn partial_malformed_and_large_source_remains_bounded_and_inert() {
        for source in ["", "10 PRINT \"unfinished", "invalid", "10 PRINT \"£\""] {
            assert!(source_highlights(source)["highlights"].is_array());
        }
        for source in [" ".repeat(65_537), "10 REM \"£\"".into()] {
            assert_eq!(source_highlights(&source)["highlights"], json!([]));
        }
    }
}
