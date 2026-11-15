//! Source correlation for a stopped Dragon BASIC statement.
//!
//! This consumes existing token/source mappings. The host must first establish
//! a real statement stop using its own validated firmware and execution owner.

use crate::inspection::{InspectionError, exact_read};
use crate::{MAX_SOURCE_BYTES, SourceRange, TokenisedProgram};

const MEMORY_CHUNK: usize = 1024;

/// Published BASIC cursor values captured from the same stopped guest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatementCursor {
    /// Linked-programme base held in $19–$1A.
    pub program_start: u16,
    /// Current BASIC line held in $68–$69; $FFFF denotes direct mode.
    pub basic_line: u16,
    /// Current token address held in $A6–$A7.
    pub statement_address: u16,
}

/// Read the published programme/line/token cursor without resuming execution.
/// This does not prove that the CPU is stopped at a statement hook.
pub fn read_statement_cursor<E>(
    mut read: impl FnMut(u16, usize) -> Result<Vec<u8>, E>,
) -> Result<StatementCursor, InspectionError<E>> {
    let mut word = |address| {
        let bytes = exact_read(&mut read, address, 2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    };
    Ok(StatementCursor {
        program_start: word(0x19)?,
        basic_line: word(0x68)?,
        statement_address: word(0xa6)?,
    })
}

/// An exact entry in the existing token/source correspondence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatementLocation {
    /// BASIC line number used by the existing source map.
    pub basic_line: u16,
    /// Zero-based statement ordinal used by the existing source map.
    pub ordinal: usize,
    /// Original source byte range, unchanged from tokenisation.
    pub source_range: SourceRange,
    /// Address of the statement's mapped token range, including leading spaces.
    pub runtime_address: u16,
    /// Length of that exact token range.
    pub token_length: usize,
}

#[derive(Debug)]
struct Entry {
    cursor: u16,
    location: StatementLocation,
}

/// Immutable source map bound to one exact linked-token programme.
#[derive(Debug)]
pub struct StatementMap {
    program_start: u16,
    bytes: Box<[u8]>,
    entries: Vec<Entry>,
}

impl StatementMap {
    /// Validate record addresses, links, line order and statement ranges before
    /// retaining the token bytes. At most the 64 KiB address space is retained.
    pub fn new(program: TokenisedProgram) -> Result<Self, &'static str> {
        let invalid = "token/source mapping is inconsistent";
        let total = program.bytes.len();
        if !(2..=65_536).contains(&total)
            || usize::from(program.program_start) + total > 65_536
            || program.lines.len() > total / 5
        {
            return Err(invalid);
        }
        let mut offset = 0usize;
        let mut previous_line = None;
        let mut entries = Vec::new();
        for line in &program.lines {
            let record_end = offset
                .checked_add(5)
                .and_then(|value| value.checked_add(line.code.len()))
                .ok_or(invalid)?;
            let address = usize::from(program.program_start) + offset;
            let next = usize::from(program.program_start) + record_end;
            let next_address = u16::try_from(next).map_err(|_| invalid)?;
            let record = program.bytes.get(offset..record_end).ok_or(invalid)?;
            if line.record_offset != offset
                || usize::from(line.runtime_address) != address
                || previous_line.is_some_and(|previous| previous >= line.number)
                || line.statements.len() > line.code.len()
                || line.number == u16::MAX
                || line.code.contains(&0)
                || record[..2] != next_address.to_be_bytes()
                || record[2..4] != line.number.to_be_bytes()
                || record[4..record.len() - 1] != line.code
                || record.last() != Some(&0)
            {
                return Err(invalid);
            }
            let mut previous_end = 0;
            let mut previous_ordinal = None;
            for statement in &line.statements {
                let end = statement
                    .token_offset
                    .checked_add(statement.token_length)
                    .ok_or(invalid)?;
                let tokens = line.code.get(statement.token_offset..end).ok_or(invalid)?;
                if statement.token_offset < previous_end
                    || previous_ordinal.is_some_and(|previous| previous >= statement.ordinal)
                    || statement.source_range.start > statement.source_range.end
                    || statement.source_range.end > MAX_SOURCE_BYTES
                {
                    return Err(invalid);
                }
                previous_end = end;
                previous_ordinal = Some(statement.ordinal);
                if let Some(first) = tokens.iter().position(|&byte| byte != b' ') {
                    let start = address + 4 + statement.token_offset;
                    let runtime_address = u16::try_from(start).map_err(|_| invalid)?;
                    let cursor = u16::try_from(start + first).map_err(|_| invalid)?;
                    entries.push(Entry {
                        cursor,
                        location: StatementLocation {
                            basic_line: line.number,
                            ordinal: statement.ordinal,
                            source_range: statement.source_range,
                            runtime_address,
                            token_length: statement.token_length,
                        },
                    });
                }
            }
            previous_line = Some(line.number);
            offset = record_end;
        }
        if offset + 2 != total || program.bytes[offset..] != [0, 0] {
            return Err(invalid);
        }
        Ok(Self {
            program_start: program.program_start,
            bytes: program.bytes.into_boxed_slice(),
            entries,
        })
    }

    /// Whether this build has an executable mapped statement on the line.
    #[must_use]
    pub fn contains_line(&self, line: u16) -> bool {
        self.entries
            .binary_search_by_key(&line, |entry| entry.location.basic_line)
            .is_ok()
    }

    /// Correlate only an exact statement-entry cursor, then compare every loaded
    /// programme byte with its mapped build. A changed, relocated or partial
    /// load cannot reuse stale source locations. Reads are bounded to 1024 bytes;
    /// a reader error/cancellation returns no location or cached partial result.
    pub fn locate<E>(
        &self,
        cursor: StatementCursor,
        mut read: impl FnMut(u16, usize) -> Result<Vec<u8>, E>,
    ) -> Result<Option<StatementLocation>, InspectionError<E>> {
        if cursor.basic_line == u16::MAX {
            return Ok(None);
        }
        if cursor.program_start != self.program_start {
            return Err(InspectionError::Invalid(
                "BASIC programme has moved since its source map was built",
            ));
        }
        let key = (cursor.basic_line, cursor.statement_address);
        let Ok(index) = self
            .entries
            .binary_search_by_key(&key, |entry| (entry.location.basic_line, entry.cursor))
        else {
            return Ok(None);
        };
        for (index, chunk) in self.bytes.chunks(MEMORY_CHUNK).enumerate() {
            let address = u16::try_from(usize::from(self.program_start) + index * MEMORY_CHUNK)
                .map_err(|_| InspectionError::Invalid("BASIC programme address overflows"))?;
            if exact_read(&mut read, address, chunk.len())? != chunk {
                return Err(InspectionError::Invalid(
                    "BASIC programme no longer matches its source map",
                ));
            }
        }
        Ok(Some(self.entries[index].location))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Dialect, analyse, tokenise};

    fn programme(source: &str) -> TokenisedProgram {
        let dialect = Dialect::dragon32().unwrap();
        tokenise(&analyse(source, &dialect).program, &dialect).unwrap()
    }

    fn cursor(programme: &TokenisedProgram, line: usize, statement: usize) -> StatementCursor {
        let record = &programme.lines[line];
        let mapping = &record.statements[statement];
        let space = record.code[mapping.token_offset..mapping.token_offset + mapping.token_length]
            .iter()
            .position(|&byte| byte != b' ')
            .unwrap();
        StatementCursor {
            program_start: programme.program_start,
            basic_line: record.number,
            statement_address: record.runtime_address
                + 4
                + u16::try_from(mapping.token_offset + space).unwrap(),
        }
    }

    #[test]
    fn correlates_existing_line_ordinal_and_source_range_at_statement_entry() {
        let source = "10 COUNT=1\n20 PRINT \"A:B\" :  COUNT=COUNT+1:REM tail:ignored\n";
        let programme = programme(source);
        let cursor = cursor(&programme, 1, 1);
        let map = StatementMap::new(programme.clone()).unwrap();
        let location = map
            .locate(cursor, |address, length| {
                let offset = usize::from(address - programme.program_start);
                Ok::<_, ()>(programme.bytes[offset..offset + length].to_vec())
            })
            .unwrap()
            .unwrap();
        assert_eq!((location.basic_line, location.ordinal), (20, 1));
        assert_eq!(
            location.source_range,
            programme.lines[1].statements[1].source_range
        );
        assert_eq!(
            source[location.source_range.start..location.source_range.end].trim(),
            "COUNT=COUNT+1"
        );
        assert_eq!(
            location.token_length,
            programme.lines[1].statements[1].token_length
        );
    }

    #[test]
    fn rejects_inconsistent_linked_records_and_mapping_ranges() {
        let valid = programme("10 A=1\n20 A=2\n");
        for mutation in 0..10 {
            let mut invalid = valid.clone();
            match mutation {
                0 => invalid.bytes[0] ^= 1,
                1 => invalid.lines[0].runtime_address += 1,
                2 => invalid.lines[1].record_offset = 0,
                3 => invalid.lines[1].number = 10,
                4 => invalid.lines[0].statements[0].token_offset = usize::MAX,
                5 => invalid.lines[0].statements[0].token_length = usize::MAX,
                6 => invalid.lines[0].statements[0].source_range.end = MAX_SOURCE_BYTES + 1,
                7 => {
                    invalid.bytes.pop();
                }
                8 => invalid.lines[0].code[0] ^= 1,
                _ => invalid.program_start = 0xffff,
            }
            assert!(StatementMap::new(invalid).is_err(), "mutation {mutation}");
        }
        assert!(StatementMap::new(programme("")).is_ok());
    }

    #[test]
    fn never_maps_direct_mode_wrong_lines_or_operand_cursors() {
        let programme = programme("10 A=123\n");
        let cursor = cursor(&programme, 0, 0);
        let map = StatementMap::new(programme).unwrap();
        for unknown in [
            StatementCursor {
                basic_line: u16::MAX,
                ..cursor
            },
            StatementCursor {
                basic_line: 11,
                ..cursor
            },
            StatementCursor {
                statement_address: cursor.statement_address + 2,
                ..cursor
            },
        ] {
            assert_eq!(map.locate(unknown, |_, _| Err("must not read")), Ok(None));
        }
        assert!(matches!(
            map.locate(
                StatementCursor {
                    program_start: cursor.program_start + 1,
                    ..cursor
                },
                |_, _| Err("must not read")
            ),
            Err(InspectionError::Invalid(_))
        ));
    }

    #[test]
    fn complete_byte_verification_is_chunked_cancellable_and_rejects_stale_sources() {
        let mut source = String::from("10 A=1\n");
        for line in 20..120 {
            source.push_str(&format!("{line} REM {}\n", "Z".repeat(64)));
        }
        let programme = programme(&source);
        let cursor = cursor(&programme, 0, 0);
        let map = StatementMap::new(programme.clone()).unwrap();
        let mut reads = 0;
        let result = map.locate(cursor, |address, length| {
            reads += 1;
            assert!(length <= MEMORY_CHUNK);
            if reads == 2 {
                return Err("cancelled");
            }
            let offset = usize::from(address - programme.program_start);
            Ok(programme.bytes[offset..offset + length].to_vec())
        });
        assert_eq!(result, Err(InspectionError::Memory("cancelled")));
        assert_eq!(reads, 2);
        for defect in 0..3 {
            assert!(matches!(
                map.locate(cursor, |address, length| {
                    let offset = usize::from(address - programme.program_start);
                    let mut bytes = programme.bytes[offset..offset + length].to_vec();
                    match defect {
                        0 if offset + length == programme.bytes.len() => {
                            *bytes.last_mut().unwrap() = 1;
                        }
                        1 => {
                            bytes.pop();
                        }
                        2 => bytes.push(0),
                        _ => {}
                    }
                    Ok::<_, ()>(bytes)
                }),
                Err(InspectionError::Invalid(_))
            ));
        }
        assert!(
            map.locate(cursor, |address, length| {
                let offset = usize::from(address - programme.program_start);
                Ok::<_, ()>(programme.bytes[offset..offset + length].to_vec())
            })
            .unwrap()
            .is_some()
        );
    }

    #[test]
    fn published_cursor_reads_are_exact_and_preserve_reader_failure() {
        let mut reads = Vec::new();
        let cursor = read_statement_cursor(|address, length| {
            reads.push((address, length));
            Ok::<_, ()>(match address {
                0x19 => vec![0x1e, 1],
                0x68 => vec![0, 30],
                _ => vec![0x1f, 0x20],
            })
        })
        .unwrap();
        assert_eq!(reads, [(0x19, 2), (0x68, 2), (0xa6, 2)]);
        assert_eq!(
            cursor,
            StatementCursor {
                program_start: 0x1e01,
                basic_line: 30,
                statement_address: 0x1f20
            }
        );
        assert!(matches!(
            read_statement_cursor(|_, _| Ok::<_, ()>(vec![0])),
            Err(InspectionError::Invalid(_))
        ));
        assert_eq!(
            read_statement_cursor(|_, _| Err("stale session")),
            Err(InspectionError::Memory("stale session"))
        );
    }
}
