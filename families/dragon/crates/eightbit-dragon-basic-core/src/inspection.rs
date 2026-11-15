//! Bounded scalar inspection of the published Dragon BASIC workspace.
//!
//! The caller owns a stopped target and supplies exact memory reads. This module
//! does not select firmware, install hooks, resume execution or accept profiles.

use serde::Serialize;
use std::collections::BTreeMap;

/// At most 128 seven-byte scalar records are inspected at one stop.
pub const MAX_SCALAR_RECORDS: usize = 128;

/// Published workspace bounds captured from the same stopped machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScalarWorkspace {
    /// Lowest permitted user-RAM address for tables and non-empty strings.
    pub ram_start: u16,
    /// Last permitted user-RAM address, inclusive.
    pub ram_end: u16,
    /// Address held in the simple-variable pointer.
    pub variables_start: u16,
    /// Address held in the array-variable pointer (end of scalar records).
    pub variables_end: u16,
}

/// Separate malformed guest state from a reader's own cancellation/I/O failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectionError<E> {
    /// Safe explanation containing no guest source or memory values.
    Invalid(&'static str),
    /// Original reader error, preserved for the owning transport.
    Memory(E),
}

/// One safely decoded scalar BASIC variable.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BasicVariable {
    /// Best source-level name, falling back to the significant runtime name.
    pub name: String,
    /// Dragon BASIC's one- or two-character significant runtime name.
    pub runtime_name: String,
    /// `number` or `string`.
    pub kind: &'static str,
    /// Stable user-facing value.
    pub display_value: String,
    /// Numeric value when this is a numeric scalar.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numeric_value: Option<f64>,
    /// String value when this is a string scalar.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub string_value: Option<String>,
}

/// Decode a bounded scalar table, preserving the established debugger value shape.
///
/// All reads must come from one stopped target. Each response must have the exact
/// requested length. The reader may reject a request to cancel inspection; no
/// partial values are returned. Empty tables/strings do not invoke the reader.
pub fn inspect_scalars<E>(
    workspace: ScalarWorkspace,
    source_names: &BTreeMap<String, String>,
    mut read: impl FnMut(u16, usize) -> Result<Vec<u8>, E>,
) -> Result<Vec<BasicVariable>, InspectionError<E>> {
    let ScalarWorkspace {
        ram_start,
        ram_end,
        variables_start: start,
        variables_end: end,
    } = workspace;
    if ram_start > ram_end || start < ram_start || end < start || end > ram_end {
        return Err(InspectionError::Invalid(
            "scalar variable-table bounds are invalid",
        ));
    }
    let length = usize::from(end - start);
    if length > MAX_SCALAR_RECORDS * 7 || length % 7 != 0 {
        return Err(InspectionError::Invalid(
            "scalar variable table is oversized or misaligned",
        ));
    }
    if length == 0 {
        return Ok(Vec::new());
    }
    let table = exact_read(&mut read, start, length)?;
    table
        .chunks_exact(7)
        .map(|record| {
            let first = record[0];
            let second = record[1] & 0x7f;
            if !first.is_ascii_uppercase()
                || (second != 0 && !second.is_ascii_uppercase() && !second.is_ascii_digit())
            {
                return Err(InspectionError::Invalid("scalar variable name is invalid"));
            }
            let is_string = record[1] & 0x80 != 0;
            let mut runtime_name = char::from(first).to_string();
            if second != 0 {
                runtime_name.push(char::from(second));
            }
            if is_string {
                runtime_name.push('$');
            }
            let name = source_names
                .get(&runtime_name)
                .cloned()
                .unwrap_or_else(|| runtime_name.clone());
            if is_string {
                let length = usize::from(record[2]);
                let address = u16::from_be_bytes([record[4], record[5]]);
                let bytes = if length == 0 {
                    Vec::new()
                } else {
                    let last_offset = u16::try_from(length - 1)
                        .map_err(|_| InspectionError::Invalid("BASIC string length is invalid"))?;
                    let last = address
                        .checked_add(last_offset)
                        .ok_or(InspectionError::Invalid("BASIC string address overflows"))?;
                    if address < ram_start || last > ram_end {
                        return Err(InspectionError::Invalid(
                            "BASIC string points outside user RAM",
                        ));
                    }
                    exact_read(&mut read, address, length)?
                };
                if !bytes.is_ascii() {
                    return Err(InspectionError::Invalid(
                        "BASIC string contains non-ASCII bytes",
                    ));
                }
                let value = String::from_utf8(bytes)
                    .map_err(|_| InspectionError::Invalid("BASIC string is not valid ASCII"))?;
                Ok(BasicVariable {
                    name,
                    runtime_name,
                    kind: "string",
                    display_value: format!("\"{}\"", value.replace('"', "\"\"")),
                    numeric_value: None,
                    string_value: Some(value),
                })
            } else {
                let value = decode_mbf(&record[2..7])?;
                Ok(BasicVariable {
                    name,
                    runtime_name,
                    kind: "number",
                    display_value: format_number(value),
                    numeric_value: Some(value),
                    string_value: None,
                })
            }
        })
        .collect()
}

pub(crate) fn exact_read<E>(
    read: &mut impl FnMut(u16, usize) -> Result<Vec<u8>, E>,
    address: u16,
    length: usize,
) -> Result<Vec<u8>, InspectionError<E>> {
    let bytes = read(address, length).map_err(InspectionError::Memory)?;
    if bytes.len() != length {
        return Err(InspectionError::Invalid(
            "memory reader returned an unexpected length",
        ));
    }
    Ok(bytes)
}

fn decode_mbf<E>(bytes: &[u8]) -> Result<f64, InspectionError<E>> {
    if bytes.len() != 5 {
        return Err(InspectionError::Invalid(
            "numeric scalar must contain five MBF bytes",
        ));
    }
    if bytes[0] == 0 {
        return Ok(0.0);
    }
    let fraction = (u32::from(bytes[1] & 0x7f) << 24)
        | (u32::from(bytes[2]) << 16)
        | (u32::from(bytes[3]) << 8)
        | u32::from(bytes[4]);
    let magnitude =
        (1.0 + f64::from(fraction) / 2_f64.powi(31)) * 2_f64.powi(i32::from(bytes[0]) - 129);
    let value = if bytes[1] & 0x80 == 0 {
        magnitude
    } else {
        -magnitude
    };
    if value.is_finite() {
        Ok(value)
    } else {
        Err(InspectionError::Invalid("numeric scalar is not finite"))
    }
}

fn format_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value}")
    }
}

/// Preserve the debugger's established source-name aliases for significant runtime names.
#[must_use]
pub fn source_variable_names(source: &str) -> BTreeMap<String, String> {
    const KEYWORDS: &[&str] = &[
        "AND", "DATA", "ELSE", "FOR", "GOSUB", "GOTO", "IF", "LET", "NEXT", "NOT", "OR", "PRINT",
        "REM", "RETURN", "STEP", "THEN", "TO",
    ];
    let mut names = BTreeMap::new();
    for line in source.lines() {
        let mut in_string = false;
        let mut identifier = String::new();
        for character in line.chars().chain(std::iter::once(' ')) {
            if character == '"' {
                in_string = !in_string;
            }
            if in_string {
                continue;
            }
            if character.is_ascii_alphanumeric() || character == '$' {
                identifier.push(character.to_ascii_uppercase());
                continue;
            }
            if identifier
                .chars()
                .next()
                .is_some_and(|first| first.is_ascii_alphabetic())
                && !KEYWORDS.contains(&identifier.as_str())
            {
                let is_string = identifier.ends_with('$');
                let stem = identifier.trim_end_matches('$');
                let mut key = stem.chars().take(2).collect::<String>();
                if is_string {
                    key.push('$');
                }
                names.entry(key).or_insert_with(|| identifier.clone());
            }
            identifier.clear();
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace(records: u16) -> ScalarWorkspace {
        ScalarWorkspace {
            ram_start: 0x1e01,
            ram_end: 0x7fff,
            variables_start: 0x3000,
            variables_end: 0x3000 + records * 7,
        }
    }

    #[test]
    fn decodes_published_numeric_representation_and_source_names() {
        for (bytes, expected) in [
            ([0, 0, 0, 0, 0], 0.0),
            ([0x81, 0, 0, 0, 0], 1.0),
            ([0x87, 0x7e, 0, 0, 0], 127.0),
            ([0x82, 0xc0, 0, 0, 0], -3.0),
        ] {
            let mut table = vec![b'C', b'O'];
            table.extend_from_slice(&bytes);
            let values = inspect_scalars(
                workspace(1),
                &BTreeMap::from([("CO".into(), "COUNT".into())]),
                |address, length| {
                    assert_eq!((address, length), (0x3000, 7));
                    Ok::<_, ()>(table.clone())
                },
            )
            .unwrap();
            assert!((values[0].numeric_value.unwrap() - expected).abs() < f64::EPSILON);
            assert_eq!(values[0].name, "COUNT");
            assert_eq!(values[0].runtime_name, "CO");
            assert_eq!(values[0].kind, "number");
            assert_eq!(values[0].string_value, None);
        }
    }

    #[test]
    fn decodes_strings_and_never_dereferences_empty_descriptors() {
        let table = [
            b'A', 0x80, 3, 0, 0x7f, 0xfd, 0, b'B', 0x80, 0, 0, 0xff, 0xff, 0,
        ];
        let mut requests = Vec::new();
        let values = inspect_scalars(workspace(2), &BTreeMap::new(), |address, length| {
            requests.push((address, length));
            Ok::<_, ()>(if address == 0x3000 {
                table.to_vec()
            } else {
                b"A\"B".to_vec()
            })
        })
        .unwrap();
        assert_eq!(requests, [(0x3000, 14), (0x7ffd, 3)]);
        assert_eq!(values[0].display_value, "\"A\"\"B\"");
        assert_eq!(values[0].string_value.as_deref(), Some("A\"B"));
        assert_eq!(values[1].string_value.as_deref(), Some(""));
        assert_eq!(values[1].runtime_name, "B$");
    }

    #[test]
    fn validates_empty_maximum_and_invalid_table_bounds_before_reads() {
        let empty =
            inspect_scalars(workspace(0), &BTreeMap::new(), |_, _| Err("must not read")).unwrap();
        assert!(empty.is_empty());
        let maximum = inspect_scalars(workspace(128), &BTreeMap::new(), |_, length| {
            assert_eq!(length, MAX_SCALAR_RECORDS * 7);
            Ok::<_, ()>([b'A', 0, 0, 0, 0, 0, 0].repeat(MAX_SCALAR_RECORDS))
        })
        .unwrap();
        assert_eq!(maximum.len(), MAX_SCALAR_RECORDS);
        let base = workspace(1);
        for bounds in [
            workspace(129),
            ScalarWorkspace {
                variables_start: 0x1e00,
                ..base
            },
            ScalarWorkspace {
                variables_end: 0x2fff,
                ..base
            },
            ScalarWorkspace {
                variables_end: 0x3008,
                ..base
            },
            ScalarWorkspace {
                variables_end: 0x8000,
                ..base
            },
            ScalarWorkspace {
                ram_start: 0x8000,
                ..base
            },
        ] {
            assert!(matches!(
                inspect_scalars(bounds, &BTreeMap::new(), |_, _| Err("must not read")),
                Err(InspectionError::Invalid(_))
            ));
        }
    }

    #[test]
    fn rejects_short_or_long_responses_and_propagates_reader_cancellation() {
        for length in [0, 6, 8] {
            assert!(matches!(
                inspect_scalars(workspace(1), &BTreeMap::new(), |_, _| Ok::<_, ()>(
                    vec![0; length]
                )),
                Err(InspectionError::Invalid(_))
            ));
        }
        let string = vec![b'A', 0x80, 2, 0, 0x40, 0, 0];
        for length in [0, 1, 3] {
            assert!(matches!(
                inspect_scalars(workspace(1), &BTreeMap::new(), |address, _| Ok::<_, ()>(
                    if address == 0x3000 {
                        string.clone()
                    } else {
                        vec![0; length]
                    }
                )),
                Err(InspectionError::Invalid(_))
            ));
        }
        let mut reads = 0;
        assert_eq!(
            inspect_scalars(workspace(1), &BTreeMap::new(), |_, _| {
                reads += 1;
                if reads == 1 {
                    Ok(string.clone())
                } else {
                    Err("cancelled")
                }
            }),
            Err(InspectionError::Memory("cancelled"))
        );
        // A later stopped snapshot can recover; no partial result/cache survived.
        assert!(
            inspect_scalars(workspace(0), &BTreeMap::new(), |_, _| Err("cancelled"))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn rejects_names_non_ascii_and_out_of_ram_strings_without_partial_output() {
        for record in [
            [b'@', 0, 0, 0, 0, 0, 0],
            [b'A', b'@', 0, 0, 0, 0, 0],
            [b'A', 0x80, 1, 0, 0x1e, 0, 0],
            [b'A', 0x80, 2, 0, 0x7f, 0xff, 0],
            [b'A', 0x80, 255, 0, 0xff, 0xff, 0],
        ] {
            let mut reads = 0;
            assert!(matches!(
                inspect_scalars(workspace(1), &BTreeMap::new(), |_, _| {
                    reads += 1;
                    Ok::<_, ()>(record.to_vec())
                }),
                Err(InspectionError::Invalid(_))
            ));
            assert_eq!(reads, 1);
        }
        assert!(matches!(
            inspect_scalars(workspace(1), &BTreeMap::new(), |address, _| Ok::<_, ()>(
                if address == 0x3000 {
                    vec![b'A', 0x80, 1, 0, 0x40, 0, 0]
                } else {
                    vec![0x80]
                }
            )),
            Err(InspectionError::Invalid(_))
        ));
    }
}
