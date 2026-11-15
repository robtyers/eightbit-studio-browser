//! Portable source maps and original reader for single-file `SjASMPlus` SLD v1.
use crate::{Result, digest, error};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
pub const MAX_SLD_BYTES: usize = 8 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Instruction {
    pub line: usize,
    pub address: u16,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceMap {
    pub schema_version: u32,
    pub revision: String,
    pub source_path: String,
    pub code_sha256: String,
    pub origin: u16,
    pub code_bytes: usize,
    pub instructions: Vec<Instruction>,
    pub symbols: BTreeMap<String, u16>,
}
fn invalid() -> crate::Error {
    error(
        "DSS1400",
        "Assembler debug data does not match the supported single-source address model",
    )
}
impl SourceMap {
    /// Bind an original assembler's instruction starts to the exact source and image.
    pub fn from_instructions(
        source: &str,
        source_path: &str,
        revision: &str,
        origin: u16,
        code: &[u8],
        mut instructions: Vec<Instruction>,
        symbols: BTreeMap<String, u16>,
    ) -> Result<Self> {
        let end = usize::from(origin)
            .checked_add(code.len())
            .filter(|end| *end <= 65536)
            .ok_or_else(invalid)?;
        let source_lines = source.lines().count();
        if source.len() > crate::MAX_SOURCE
            || !source.is_ascii()
            || source_path.is_empty()
            || source_path.len() > 4096
            || revision.len() != 64
            || !revision.bytes().all(|b| b.is_ascii_hexdigit())
            || code.is_empty()
            || instructions.len() > code.len()
            || symbols.len() > source_lines
        {
            return Err(invalid());
        }
        instructions.sort_by_key(|i| i.address);
        if instructions.iter().any(|i| {
            i.line == 0
                || i.line > source_lines
                || i.address < origin
                || usize::from(i.address) >= end
        }) || instructions
            .windows(2)
            .any(|pair| pair[0].address == pair[1].address)
            || symbols.iter().any(|(name, address)| {
                name.is_empty()
                    || name.len() > 128
                    || !name.bytes().enumerate().all(|(i, b)| {
                        b.is_ascii_alphabetic()
                            || b"_.".contains(&b)
                            || (i > 0 && b.is_ascii_digit())
                    })
                    || *address < origin
                    || usize::from(*address) > end
            })
        {
            return Err(invalid());
        }
        Ok(Self {
            schema_version: 1,
            revision: revision.into(),
            source_path: source_path.into(),
            code_sha256: digest(code),
            origin,
            code_bytes: code.len(),
            instructions,
            symbols,
        })
    }
    pub fn parse(
        text: &str,
        source: &str,
        source_path: &str,
        revision: &str,
        origin: u16,
        code: &[u8],
    ) -> Result<Self> {
        if text.len() > MAX_SLD_BYTES
            || !text.is_ascii()
            || source.len() > 128 * 1024
            || source_path.len() > 4096
            || code.is_empty()
            || code.len() > 16384
            || revision.len() != 64
            || !revision.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(invalid());
        }
        let mut lines = text.lines();
        if lines.next() != Some("|SLD.data.version|1") {
            return Err(invalid());
        }
        let end = usize::from(origin)
            .checked_add(code.len())
            .filter(|end| *end <= 65536)
            .ok_or_else(invalid)?;
        let source_lines = source.lines().count();
        let mut instructions = Vec::new();
        let mut symbols = BTreeMap::new();
        let mut addresses = BTreeSet::new();
        let mut model = false;
        for (index, line) in lines.enumerate() {
            if index > source_lines * 3 + 2 || line.len() > 8192 {
                return Err(invalid());
            }
            if line.starts_with("||") {
                continue;
            }
            let fields = line.split('|').collect::<Vec<_>>();
            if fields.len() != 8
                || fields[0] != "main.asm"
                || !fields[2].is_empty()
                || fields[3] != "0"
            {
                return Err(invalid());
            }
            let line = fields[1].parse::<usize>().map_err(|_| invalid())?;
            if fields[6] == "Z" {
                if model
                    || line != 1
                    || fields[4] != "-1"
                    || fields[5] != "-1"
                    || fields[7] != "pages.size:65536,pages.count:32,slots.count:1,slots.adr:0"
                {
                    return Err(invalid());
                }
                model = true;
                continue;
            }
            let line = line
                .checked_sub(1)
                .filter(|n| *n > 0 && *n <= source_lines)
                .ok_or_else(invalid)?;
            if !model || !matches!(fields[6], "T" | "L" | "F") {
                return Err(invalid());
            }
            // EQU symbols do not identify emitted instructions or loaded addresses.
            if fields[6] != "T" && fields[7].split(',').any(|v| v == "+equ") {
                continue;
            }
            if fields[6] == "F" {
                continue;
            }
            let address = fields[5].parse::<u16>().map_err(|_| invalid())?;
            // Labels may precede ORG in otherwise valid existing projects. Only
            // labels in the emitted image are useful to the debugger.
            if fields[6] == "L"
                && (usize::from(address) < usize::from(origin) || usize::from(address) > end)
            {
                continue;
            }
            if fields[4] != "0"
                || usize::from(address) < usize::from(origin)
                || usize::from(address) > end
                || (fields[6] == "T" && usize::from(address) == end)
            {
                return Err(invalid());
            }
            if fields[6] == "T" {
                if !fields[7].is_empty()
                    || !addresses.insert(address)
                    || instructions.len() >= code.len()
                {
                    return Err(invalid());
                }
                instructions.push(Instruction { line, address });
            } else {
                let label = fields[7].split(',').collect::<Vec<_>>();
                if label.len() < 3
                    || !label[0].is_empty()
                    || !label[2].is_empty()
                    || label[3..].iter().any(|v| *v != "+used")
                {
                    return Err(invalid());
                }
                let name = label[1];
                if name.is_empty()
                    || name.len() > 4096
                    || !name.bytes().enumerate().all(|(i, b)| {
                        b.is_ascii_alphabetic() || b == b'_' || (i > 0 && b.is_ascii_digit())
                    })
                    || symbols.len() >= source_lines
                    || symbols.insert(name.into(), address).is_some()
                {
                    return Err(invalid());
                }
            }
        }

        if !model {
            return Err(invalid());
        }
        instructions.sort_by_key(|i| i.address);
        Ok(Self {
            schema_version: 1,
            revision: revision.into(),
            source_path: source_path.into(),
            code_sha256: digest(code),
            origin,
            code_bytes: code.len(),
            instructions,
            symbols,
        })
    }
    pub fn breakpoint(&self, line: usize) -> Result<u16> {
        self.instructions
            .iter()
            .find(|i| i.line == line)
            .map(|i| i.address)
            .ok_or_else(|| {
                error(
                    "DSS1401",
                    "Choose a source line that emits a Z80 instruction",
                )
            })
    }
    #[must_use]
    pub fn line_at(&self, address: u16) -> Option<usize> {
        self.instructions
            .iter()
            .find(|i| i.address == address)
            .map(|i| i.line)
    }
}
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    const SOURCE: &str = " ORG 24576\nstart: LD A,2\n OUT (254),A\n RET\n";
    #[test]
    fn original_instruction_maps_reject_duplicate_out_of_range_or_stale_records() {
        let revision = "a".repeat(64);
        for instructions in [
            vec![Instruction {
                line: 0,
                address: 24576,
            }],
            vec![Instruction {
                line: 2,
                address: 24577,
            }],
            vec![
                Instruction {
                    line: 2,
                    address: 24576,
                },
                Instruction {
                    line: 3,
                    address: 24576,
                },
            ],
        ] {
            assert!(
                SourceMap::from_instructions(
                    SOURCE,
                    "src/main.asm",
                    &revision,
                    24576,
                    &[0xc9],
                    instructions,
                    BTreeMap::new()
                )
                .is_err()
            );
        }
        assert!(
            SourceMap::from_instructions(
                SOURCE,
                "src/main.asm",
                "stale",
                24576,
                &[0xc9],
                vec![],
                BTreeMap::new()
            )
            .is_err()
        );
        let map = SourceMap::from_instructions(
            SOURCE,
            "src/main.asm",
            &revision,
            24576,
            &[0xc9],
            vec![Instruction {
                line: 2,
                address: 24576,
            }],
            BTreeMap::new(),
        )
        .unwrap();
        assert_eq!(map.schema_version, 1);
        assert_eq!(map.breakpoint(2).unwrap(), 24576);
    }
    const SLD: &str = "|SLD.data.version|1\nmain.asm|1||0|-1|-1|Z|pages.size:65536,pages.count:32,slots.count:1,slots.adr:0\nmain.asm|3||0|0|24576|F|start\nmain.asm|3||0|0|24576|L|,start,\nmain.asm|3||0|0|24576|T|\nmain.asm|4||0|0|24578|T|\nmain.asm|5||0|0|24580|T|\n";
    fn parse(text: &str) -> Result<SourceMap> {
        SourceMap::parse(
            text,
            SOURCE,
            "src/main.asm",
            &"a".repeat(64),
            24576,
            &[0x3e, 2, 0xd3, 254, 0xc9],
        )
    }
    #[test]
    fn exact_instruction_mapping_and_symbols() {
        let map = parse(SLD).unwrap();
        assert_eq!(map.breakpoint(2).unwrap(), 24576);
        assert_eq!(map.breakpoint(3).unwrap(), 24578);
        assert!(map.breakpoint(1).is_err());
        assert_eq!(map.line_at(24580), Some(4));
        assert_eq!(map.line_at(24579), None);
        assert_eq!(map.symbols.get("start"), Some(&24576));
        assert_eq!(
            serde_json::from_str::<SourceMap>(&serde_json::to_string(&map).unwrap()).unwrap(),
            map
        );
    }
    #[test]
    fn referenced_symbols_and_data_only_programmes_remain_supported() {
        let map = parse(&SLD.replace("|L|,start,", "|L|,start,,+used")).unwrap();
        assert_eq!(map.symbols.get("start"), Some(&24576));
        let map = parse(&format!("{SLD}main.asm|2||0|0|0|L|,before_origin,\n")).unwrap();
        assert!(!map.symbols.contains_key("before_origin"));
        let data_only = "|SLD.data.version|1\nmain.asm|1||0|-1|-1|Z|pages.size:65536,pages.count:32,slots.count:1,slots.adr:0\n";
        let map = SourceMap::parse(
            data_only,
            " ORG 24576\n DB 1\n",
            "src/main.asm",
            &"a".repeat(64),
            24576,
            &[1],
        )
        .unwrap();
        assert!(map.breakpoint(2).is_err());
        assert!(map.instructions.is_empty());
    }
    #[test]
    fn rejects_foreign_files_banks_revisions_and_malformed_records() {
        for input in [
            "|SLD.data.version|1\n".into(),
            SLD.replace("version|1", "version|2"),
            SLD.replace("main.asm", "../outside.asm"),
            SLD.replace("|0|24576|T", "|2|24576|T"),
            SLD.replace("|24576|T", "|100|T"),
            SLD.replace("main.asm|3|", "main.asm|999|"),
            SLD.replace("|T|", "|K|"),
            format!("{SLD}main.asm|3||0|0|24576|T|\n"),
            SLD.replace("|L|,start,", "|L|module,start,local"),
        ] {
            assert!(parse(&input).is_err());
        }
        assert!(SourceMap::parse(SLD, SOURCE, "src/main.asm", "bad", 24576, &[1]).is_err());
        assert!(parse(&"x".repeat(MAX_SLD_BYTES + 1)).is_err());
    }
    #[test]
    fn supports_crlf_and_does_not_map_data_as_instructions() {
        let map = parse(&SLD.replace('\n', "\r\n")).unwrap();
        assert_eq!(map.instructions.len(), 3);
        assert!(map.line_at(24577).is_none());
    }

    #[test]
    fn maps_full_size_code_without_reducing_the_existing_build_limit() {
        use std::fmt::Write;
        let source = format!(" ORG 24576\n{}", " NOP\n".repeat(8192));
        let mut sld = "|SLD.data.version|1\nmain.asm|1||0|-1|-1|Z|pages.size:65536,pages.count:32,slots.count:1,slots.adr:0\n".to_string();
        for offset in 0..8192 {
            writeln!(sld, "main.asm|{}||0|0|{}|T|", offset + 3, 24576 + offset).unwrap();
        }
        let map = SourceMap::parse(
            &sld,
            &source,
            "src/main.asm",
            &"a".repeat(64),
            24576,
            &vec![0; 8192],
        )
        .unwrap();
        assert_eq!(map.instructions.len(), 8192);
        assert_eq!(map.breakpoint(8193).unwrap(), 32767);
    }
}
