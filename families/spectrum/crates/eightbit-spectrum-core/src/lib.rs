//! Family-owned targets, firmware profiles and native BASIC/media semantics.
#![allow(missing_docs)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#[cfg(all(test, feature = "native"))]
mod help_tests;
pub mod original_session;
pub use eightbit_spectrum_emulator::Model;
/// Original runtime availability is a family-owned, closed machine list.
#[must_use]
pub fn original_target(target: &str) -> Option<&'static TargetDefinition> {
    Model::from_target(target)?;
    TARGETS.iter().find(|item| item.id == target)
}
mod tokens;
use eightbit_source_core::{Family, Profile, Result, error};
use eightbit_targets::{Capability, FamilyDefinition, ProjectTemplate, TargetDefinition};
const TEMPLATES: &[ProjectTemplate] = &[
    ProjectTemplate {
        id: "basic",
        name: "Native BASIC",
        description: "An original numbered BASIC program with deterministic media export.",
    },
    ProjectTemplate {
        id: "assembly",
        name: "Z80 assembly",
        description: "A bounded single-source Z80 programme with a native BASIC loader.",
    },
];
const ORIGINAL_CAPABILITIES: &[Capability] = &[
    Capability::SourceEditing,
    Capability::Build,
    Capability::CassetteExport,
    Capability::Run,
    Capability::MachineDebug,
];
const CAPABILITIES: &[Capability] = &[
    Capability::SourceEditing,
    Capability::Build,
    Capability::CassetteExport,
    Capability::Run,
];
pub static TARGETS: &[TargetDefinition] = &[
    TargetDefinition {
        version: 1,
        id: "spectrum16",
        family: "spectrum",
        display_name: "ZX Spectrum 16K",
        model: "spectrum16",
        mode: "native-basic",
        cpu: "z80",
        dialect: Some("spectrum-basic"),
        ram_kib: 16,
        tv: Some("pal"),
        templates: TEMPLATES,
        capabilities: CAPABILITIES,
    },
    TargetDefinition {
        version: 1,
        id: "spectrum48",
        family: "spectrum",
        display_name: "ZX Spectrum 48K",
        model: "spectrum48",
        mode: "native-basic",
        cpu: "z80",
        dialect: Some("spectrum-basic"),
        ram_kib: 48,
        tv: Some("pal"),
        templates: TEMPLATES,
        capabilities: ORIGINAL_CAPABILITIES,
    },
    TargetDefinition {
        version: 1,
        id: "spectrum128",
        family: "spectrum",
        display_name: "ZX Spectrum 128K (UK)",
        model: "spectrum128",
        mode: "native-basic",
        cpu: "z80",
        dialect: Some("spectrum-basic"),
        ram_kib: 128,
        tv: Some("pal"),
        templates: TEMPLATES,
        capabilities: ORIGINAL_CAPABILITIES,
    },
    TargetDefinition {
        version: 1,
        id: "spectrum-plus2",
        family: "spectrum",
        display_name: "ZX Spectrum +2 (UK)",
        model: "spectrum-plus2",
        mode: "native-basic",
        cpu: "z80",
        dialect: Some("spectrum-basic"),
        ram_kib: 128,
        tv: Some("pal"),
        templates: TEMPLATES,
        capabilities: ORIGINAL_CAPABILITIES,
    },
];
pub static DEFINITION: FamilyDefinition = FamilyDefinition {
    id: "spectrum",
    display_name: "ZX Spectrum",
    targets: TARGETS,
};
pub static PROFILES: &[Profile] = &[
    Profile {
        adapter: None,
        original_assembler: None,
        target: &TARGETS[0],
        machine: "spectrum",
        boot_command: "j\"\"{ENTER}",
        rom_names: &["spectrum.rom"],
        debug_snapshot: None,
        rom_size: 16384,
        rom_sha256: "d55daa439b673b0e3f5897f99ac37ecb45f974d1862b4dadb85dec34af99cb42",
        extension: "tap",
        media_limit: 65536,
        load_option: "-cassette",
        machine_options: &["-bios", "en", "-ramsize", "16K"],
        assembly_language: "z80",
        origin: 24576,
        assembly_limit: 8192,
        basic_template: "10 BORDER 1\n20 PAPER 0\n30 INK 7\n40 CLS\n50 PRINT \"HELLO FROM EIGHTBIT STUDIO Z80\"\n60 FOR N=1 TO 8\n70 BEEP 0.1,N\n80 NEXT N\n",
        assembly_template: "    ORG 24576\n    LD A,2\n    OUT (254),A\n    RET\n",
        basic: compile_basic,
        assembly: package_assembly,
    },
    Profile {
        adapter: None,
        original_assembler: Some(assemble_original),
        target: &TARGETS[1],
        machine: "spectrum",
        boot_command: "j\"\"{ENTER}",
        rom_names: &["spectrum.rom"],
        debug_snapshot: Some(debug_snapshot),
        rom_size: 16384,
        rom_sha256: "d55daa439b673b0e3f5897f99ac37ecb45f974d1862b4dadb85dec34af99cb42",
        extension: "tap",
        media_limit: 65536,
        load_option: "-cassette",
        machine_options: &["-bios", "en", "-ramsize", "48K"],
        assembly_language: "z80",
        origin: 24576,
        assembly_limit: 8192,
        basic_template: "10 BORDER 1\n20 PAPER 0\n30 INK 7\n40 CLS\n50 PRINT \"HELLO FROM EIGHTBIT STUDIO Z80\"\n60 FOR N=1 TO 8\n70 BEEP 0.1,N\n80 NEXT N\n",
        assembly_template: "    ORG 24576\n    LD A,2\n    OUT (254),A\n    RET\n",
        basic: compile_basic,
        assembly: package_assembly,
    },
    Profile {
        adapter: None,
        original_assembler: Some(assemble_original),
        target: &TARGETS[2],
        machine: "spec128",
        boot_command: "{ENTER}",
        rom_names: &["zx128_0.rom", "zx128_1.rom"],
        debug_snapshot: None,
        rom_size: 32768,
        rom_sha256: "c1ff621d7910105d4ee45c31e9fd8fd0d79a545c78b66c69a562ee1ffbae8d72",
        extension: "tap",
        media_limit: 65536,
        load_option: "-cassette",
        machine_options: &["-bios", "en", "-ramsize", "128K"],
        assembly_language: "z80",
        origin: 24576,
        assembly_limit: 8192,
        basic_template: "10 BORDER 1\n20 PAPER 0\n30 INK 7\n40 CLS\n50 PRINT \"HELLO FROM EIGHTBIT STUDIO Z80\"\n60 FOR N=1 TO 8\n70 BEEP 0.1,N\n80 NEXT N\n",
        assembly_template: "    ORG 24576\n    LD A,2\n    OUT (254),A\n    RET\n",
        basic: compile_basic,
        assembly: package_assembly,
    },
    Profile {
        adapter: None,
        original_assembler: Some(assemble_original),
        target: &TARGETS[3],
        machine: "specpls2",
        boot_command: "{ENTER}",
        rom_names: &["zxp2_0.rom", "zxp2_1.rom"],
        debug_snapshot: None,
        rom_size: 32768,
        rom_sha256: "dae0690d8b433ea22b76b62520341f784071dbde0d02f50af0e3fd20fc6bca4a",
        extension: "tap",
        media_limit: 65536,
        load_option: "-cassette",
        machine_options: &["-bios", "en", "-ramsize", "128K"],
        assembly_language: "z80",
        origin: 24576,
        assembly_limit: 8192,
        basic_template: "10 BORDER 1\n20 PAPER 0\n30 INK 7\n40 CLS\n50 PRINT \"HELLO FROM EIGHTBIT STUDIO Z80\"\n60 FOR N=1 TO 8\n70 BEEP 0.1,N\n80 NEXT N\n",
        assembly_template: "    ORG 24576\n    LD A,2\n    OUT (254),A\n    RET\n",
        basic: compile_basic,
        assembly: package_assembly,
    },
];
pub static FAMILY: Family = Family {
    guidance: eightbit_source_core::ProjectGuidance {
        readme: include_str!("../../../templates/project-README.md"),
        agents: include_str!("../../../templates/project-AGENTS.md"),
        tests: include_str!("../../../templates/project-tests-README.md"),
    },
    definition: &DEFINITION,
    manifest: "spectrum.toml",
    profiles: PROFILES,
};
fn encode(byte: u8) -> Option<u8> {
    (32..=126).contains(&byte).then_some(byte)
}
fn dialect() -> eightbit_sinclair_basic::Dialect {
    eightbit_sinclair_basic::Dialect {
        tokens: tokens::TOKENS,
        extra_tokens: &[],
        extra_statements: &[],
        escaped_quote: None,
        encode,
        number_marker: 14,
        newline: 13,
        max_line: 9999,
        statements: true,
    }
}
fn target_dialect(target: &TargetDefinition) -> eightbit_sinclair_basic::Dialect {
    let mut language = dialect();
    if target.ram_kib == 128 {
        language.extra_tokens = &[("SPECTRUM", 163), ("PLAY", 164)];
        language.extra_statements = &["CAT", "ERASE"];
    }
    language
}
pub fn compile_basic(target: &TargetDefinition, source: &str) -> Result<Vec<u8>> {
    let language = target_dialect(target);
    let program = eightbit_sinclair_basic::program(source, &language)?;
    let limit = if target.ram_kib == 16 { 7000 } else { 40000 };
    if program.len() > limit {
        return Err(error(
            "DSS1111",
            "Program exceeds the selected Spectrum BASIC memory budget",
        ));
    }
    let start = u16::from_be_bytes([program[0], program[1]]);
    tape_file(
        0,
        "ZX STUDIO",
        &program,
        start,
        u16::try_from(program.len()).map_err(|_| error("DSS1111", "Program too large"))?,
    )
}
fn block(flag: u8, bytes: &[u8], out: &mut Vec<u8>) -> Result<()> {
    let len =
        u16::try_from(bytes.len() + 2).map_err(|_| error("DSS1111", "Tape block too large"))?;
    out.extend(len.to_le_bytes());
    out.push(flag);
    out.extend(bytes);
    out.push(bytes.iter().fold(flag, |sum, b| sum ^ b));
    Ok(())
}
fn tape_file(kind: u8, name: &str, bytes: &[u8], first: u16, second: u16) -> Result<Vec<u8>> {
    let size =
        u16::try_from(bytes.len()).map_err(|_| error("DSS1111", "Tape payload too large"))?;
    let mut header = vec![kind];
    let mut filename = [b' '; 10];
    for (to, from) in filename.iter_mut().zip(name.bytes()) {
        *to = from;
    }
    header.extend(filename);
    header.extend(size.to_le_bytes());
    header.extend(first.to_le_bytes());
    header.extend(second.to_le_bytes());
    let mut out = Vec::new();
    block(0, &header, &mut out)?;
    block(255, bytes, &mut out)?;
    Ok(out)
}
pub fn package_assembly(target: &TargetDefinition, code: &[u8]) -> Result<Vec<u8>> {
    if code.is_empty() || code.len() > 8192 {
        return Err(error(
            "DSS1111",
            "Spectrum assembly supports 1–8192 bytes at address 24576",
        ));
    }
    let mut tape = compile_basic(
        target,
        "10 CLEAR 24575\n20 LOAD \"\" CODE\n30 RANDOMIZE USR 24576\n",
    )?;
    tape.extend(tape_file(3, "CODE", code, 24576, 32768)?);
    Ok(tape)
}

/// Portable original Spectrum assembler, source map and stock-ROM cassette loader.
/// The external assembler path retains its existing bounded profile separately.
pub fn assemble_original(
    target: &TargetDefinition,
    source: &str,
    source_path: &str,
    revision: &str,
) -> Result<eightbit_source_core::AssemblyBuild> {
    if original_target(target.id).is_none() || source.len() > eightbit_source_core::MAX_SOURCE {
        return Err(error(
            "DSS1100",
            "Original assembly supports a single Spectrum 48K, 128K or original +2 source of at most 128 KiB",
        ));
    }
    let assembled = eightbit_z80_core::assembler::assemble(source, 24576, None).map_err(|e| {
        error(
            "S48ASM1001",
            format!("Line {}, column {}: {}", e.line, e.column, e.message),
        )
    })?;
    if assembled.bytes.len() > 40960 {
        return Err(error(
            "DSS1103",
            "Spectrum code must fit at addresses 24576–65535",
        ));
    }
    let end = usize::from(assembled.load_address) + assembled.bytes.len();
    let symbols = assembled
        .symbols
        .into_iter()
        .filter_map(|(name, value)| u16::try_from(value).ok().map(|value| (name, value)))
        .filter(|(_, value)| *value >= assembled.load_address && usize::from(*value) <= end)
        .collect();
    let map = eightbit_source_core::source_map::SourceMap::from_instructions(
        source,
        source_path,
        revision,
        assembled.load_address,
        &assembled.bytes,
        assembled
            .lines
            .into_iter()
            .map(|line| eightbit_source_core::source_map::Instruction {
                line: line.source_line,
                address: line.address,
            })
            .collect(),
        symbols,
    )?;
    let mut media = compile_basic(
        target,
        "10 CLEAR 24575\n20 LOAD \"\" CODE\n30 RANDOMIZE USR 24576\n",
    )?;
    media.extend(tape_file(
        3,
        "CODE",
        &assembled.bytes,
        assembled.load_address,
        32768,
    )?);
    verify_code_media(&media, &assembled.bytes, assembled.load_address)?;
    Ok(eightbit_source_core::AssemblyBuild {
        code: assembled.bytes,
        media,
        map,
        tool_version: "Eightbit Z80 assembler 1",
    })
}

fn verify_code_media(mut media: &[u8], code: &[u8], origin: u16) -> Result<()> {
    let bad = || {
        error(
            "DSS1111",
            "Generated cassette failed its code round-trip check",
        )
    };
    for index in 0..4 {
        let length = media.get(..2).ok_or_else(bad)?;
        let size = usize::from(u16::from_le_bytes([length[0], length[1]]));
        let block = media
            .get(2..2 + size)
            .filter(|block| block.len() >= 2)
            .ok_or_else(bad)?;
        if block[0] != if index % 2 == 0 { 0 } else { 255 }
            || block.iter().fold(0, |sum, byte| sum ^ byte) != 0
        {
            return Err(bad());
        }
        if index % 2 == 0 && (block.len() != 19 || block[1] != if index == 0 { 0 } else { 3 }) {
            return Err(bad());
        }
        if index == 2
            && (usize::from(u16::from_le_bytes([block[12], block[13]])) != code.len()
                || u16::from_le_bytes([block[14], block[15]]) != origin)
        {
            return Err(bad());
        }
        if index == 3 && &block[1..block.len() - 1] != code {
            return Err(bad());
        }
        media = &media[2 + size..];
    }
    if !media.is_empty() {
        return Err(bad());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_assembler_maps_instructions_and_exports_large_standard_code_files() {
        let source = "ORG 24576\nstart: LD A,42\nLD (23296),A\nRET\ndata: DB 1,2\nDS 40952\n";
        let built =
            assemble_original(&TARGETS[1], source, "src/asm/main.asm", &"a".repeat(64)).unwrap();
        assert_eq!(built.code.len(), 40960);
        assert_eq!(&built.code[..8], &[0x3e, 42, 0x32, 0, 0x5b, 0xc9, 1, 2]);
        assert_eq!(built.map.breakpoint(2).unwrap(), 24576);
        assert_eq!(built.map.breakpoint(3).unwrap(), 24578);
        assert_eq!(built.map.breakpoint(4).unwrap(), 24581);
        assert!(built.map.breakpoint(5).is_err());
        assert!(built.map.breakpoint(6).is_err());
        assert_eq!(built.map.symbols["START"], 24576);
        assert_eq!(
            built.map.code_sha256,
            eightbit_source_core::digest(&built.code)
        );
        let blocks = built.media.chunks(1).count();
        assert!(blocks > 40960 && blocks < 42000);
        let mut cursor = 0;
        let mut decoded = Vec::new();
        while cursor < built.media.len() {
            let length = usize::from(u16::from_le_bytes([
                built.media[cursor],
                built.media[cursor + 1],
            ]));
            cursor += 2;
            let data = &built.media[cursor..cursor + length];
            assert_eq!(data.iter().fold(0, |a, b| a ^ b), 0);
            decoded.push(data);
            cursor += length;
        }
        assert_eq!(decoded.len(), 4);
        assert_eq!(decoded[2][1], 3);
        assert_eq!(u16::from_le_bytes([decoded[2][12], decoded[2][13]]), 40960);
        assert_eq!(u16::from_le_bytes([decoded[2][14], decoded[2][15]]), 24576);
        assert_eq!(&decoded[3][1..decoded[3].len() - 1], built.code);
        for invalid in [
            "include \"private.asm\"",
            "ORG 24576\nDS 40961",
            "ORG 0\nRET",
        ] {
            assert!(
                assemble_original(&TARGETS[1], invalid, "src/asm/main.asm", &"a".repeat(64))
                    .is_err()
            );
        }
        assert!(assemble_original(&TARGETS[0], "RET", "src/main.asm", &"a".repeat(64)).is_err());
        assert!(assemble_original(&TARGETS[1], "RET", "src/main.asm", "stale").is_err());
    }

    #[test]
    #[cfg(feature = "native")]
    fn native_original_build_needs_no_tool_and_keeps_failure_cancellation_and_freshness_guards() {
        use std::sync::atomic::{AtomicBool, Ordering};
        static ENGINE: eightbit_source::Engine = eightbit_source::Engine {
            product: "Eightbit Studio Z80",
            families: &[&FAMILY],
            foreign_manifests: &[],
        };
        for target in ["spectrum48", "spectrum128", "spectrum-plus2"] {
            let directory = tempfile::tempdir().unwrap();
            let project = ENGINE
                .create(
                    directory.path(),
                    "original-build",
                    "spectrum",
                    target,
                    "assembly",
                )
                .unwrap();
            let cancel = AtomicBool::new(false);
            let first = ENGINE.build(&project.root, None, &cancel).unwrap();
            let again = ENGINE.build(&project.root, None, &cancel).unwrap();
            assert_eq!(first.sha256, again.sha256);
            assert_eq!(first.tool_version, "Eightbit Z80 assembler 1");
            assert!(
                ENGINE
                    .fresh_media(&project.root, &first.revision, &first.sha256)
                    .is_ok()
            );
            let map = std::fs::read(project.root.join(".studio/build/debug-map.json")).unwrap();
            assert!(String::from_utf8_lossy(&map).contains(&first.revision));
            assert!(
                ENGINE
                    .build(
                        &project.root,
                        Some(std::path::Path::new("/missing/sjasmplus")),
                        &cancel
                    )
                    .is_err()
            );
            std::fs::write(
                project.root.join(&project.source_path),
                "ORG 24576\nLD A,999\n",
            )
            .unwrap();
            assert!(ENGINE.build(&project.root, None, &cancel).is_err());
            assert!(
                ENGINE
                    .fresh_media(&project.root, &first.revision, &first.sha256)
                    .is_err()
            );
            assert_eq!(
                std::fs::read(project.root.join(".studio/build/debug-map.json")).unwrap(),
                map
            );
            std::fs::write(project.root.join(&project.source_path), "ORG 24576\nRET\n").unwrap();
            cancel.store(true, Ordering::Release);
            assert!(ENGINE.build(&project.root, None, &cancel).is_err());
            assert_eq!(
                std::fs::read(project.root.join(".studio/build/debug-map.json")).unwrap(),
                map
            );
            cancel.store(false, Ordering::Release);
            assert!(ENGINE.build(&project.root, None, &cancel).is_ok());
        }
        assert!(PROFILES[0].original_assembler.is_none());
        for profile in &PROFILES[1..] {
            assert!(profile.original_assembler.is_some());
        }
    }
    #[test]
    #[cfg(feature = "native")]
    fn every_template_has_target_guidance_and_preserves_legacy_projects() {
        static TEMPLATE_ENGINE: eightbit_source::Engine = eightbit_source::Engine {
            product: "Eightbit Studio Z80",
            families: &[&FAMILY],
            foreign_manifests: &[],
        };
        let parent = tempfile::tempdir().unwrap();
        for profile in PROFILES {
            for template in profile.target.templates {
                let name = format!("{}-{}", profile.target.id, template.id);
                let project = TEMPLATE_ENGINE
                    .create(
                        parent.path(),
                        &name,
                        FAMILY.definition.id,
                        profile.target.id,
                        template.id,
                    )
                    .unwrap();
                let assembly = template.id == "assembly";
                let path = if assembly {
                    "src/asm/main.asm"
                } else {
                    "src/basic/main.bas"
                };
                assert_eq!(project.source_path, path);
                assert!(
                    !project
                        .root
                        .join(if assembly { "src/basic" } else { "src/asm" })
                        .exists()
                );
                for document in ["README.md", "AGENTS.md"] {
                    let text = TEMPLATE_ENGINE
                        .read_supporting_file(&project.root, document)
                        .unwrap();
                    assert!(text.contains(profile.target.display_name));
                    assert!(text.contains(profile.target.id));
                    assert!(text.contains(path));
                    assert!(text.contains(FAMILY.manifest));
                    assert!(text.contains("Eightbit Studio Z80"));
                }
                let claude = TEMPLATE_ENGINE
                    .read_supporting_file(&project.root, "CLAUDE.md")
                    .unwrap();
                assert_eq!(
                    claude,
                    include_str!("../../../../../templates/project-CLAUDE.md")
                );
                assert!(
                    TEMPLATE_ENGINE
                        .read_supporting_file(&project.root, "tests/README.md")
                        .is_ok()
                );
                if !assembly {
                    assert!((profile.basic)(profile.target, &project.source).is_ok());
                }
                // Previous schema-1 paths and projects without new guidance still open.
                let legacy = if assembly {
                    "src/main.asm"
                } else {
                    "src/main.bas"
                };
                std::fs::rename(project.root.join(path), project.root.join(legacy)).unwrap();
                let manifest = std::fs::read_to_string(project.root.join(FAMILY.manifest))
                    .unwrap()
                    .replace(path, legacy);
                std::fs::write(project.root.join(FAMILY.manifest), &manifest).unwrap();
                std::fs::write(
                    project.root.join("README.md"),
                    "Developer's customised guide",
                )
                .unwrap();
                std::fs::remove_file(project.root.join("CLAUDE.md")).unwrap();
                let reopened = TEMPLATE_ENGINE.open(&project.root).unwrap();
                assert_eq!(reopened.source_path, legacy);
                assert!(!reopened.supporting_files.contains(&"CLAUDE.md".into()));
                assert!(
                    TEMPLATE_ENGINE
                        .create(
                            parent.path(),
                            &name,
                            FAMILY.definition.id,
                            profile.target.id,
                            template.id
                        )
                        .is_err()
                );
                assert_eq!(
                    std::fs::read_to_string(project.root.join("README.md")).unwrap(),
                    "Developer's customised guide"
                );
                assert_eq!(
                    std::fs::read_to_string(project.root.join(FAMILY.manifest)).unwrap(),
                    manifest
                );
                assert!(!project.root.join("CLAUDE.md").exists());
            }
        }
    }

    #[test]
    fn binary_literals_and_128_keywords_preserve_native_semantics() {
        let data = compile_basic(&TARGETS[1], "10 PRINT BIN 1010\n").unwrap();
        assert!(data.windows(6).any(|w| w == [14, 132, 32, 0, 0, 0]));
        assert!(compile_basic(&TARGETS[1], "10 PRINT BIN 102\n").is_err());
        assert!(compile_basic(&TARGETS[1], "10 DEF FN f(x)=x\n").is_ok());
        assert!(compile_basic(&TARGETS[1], "10 PLAY \"c\"\n").is_err());
        let data = compile_basic(&TARGETS[2], "10 PLAY \"c\"\n").unwrap();
        assert!(data.contains(&164));
    }
    #[test]
    fn function_definitions_emit_bounded_native_parameter_records_and_recover_exactly() {
        let source =
            "10 DEF FN f(x,y$)=x+LEN y$\n20 DEF FN g$()=\"abc\"\n30 PRINT FN f(2,FN g$())\n";
        let data = compile_basic(&TARGETS[1], source).unwrap();
        assert!(
            data.windows(7)
                .any(|bytes| bytes == [b'X', 14, 0, 0, 0, 0, 0])
        );
        assert!(
            data.windows(8)
                .any(|bytes| bytes == [b'Y', b'$', 14, 0, 0, 0, 0, 0])
        );
        let recovered = decompile_basic(&TARGETS[1], &data).unwrap();
        assert_eq!(compile_basic(&TARGETS[1], &recovered).unwrap(), data);
        for source in [
            "10 DEF FN f(x,x)=x\n",
            "10 DEF FN f(long)=1\n",
            "10 DEF FN f(x y)=1\n",
            "10 DEF FN f(x,)=1\n",
            "10 DEF FN f(x) 1\n",
        ] {
            assert_eq!(
                compile_basic(&TARGETS[1], source).unwrap_err().code,
                "DSS1110"
            );
        }
        assert_eq!(
            eightbit_sinclair_basic::body("PRINT é", &dialect())
                .unwrap_err()
                .code,
            "DSS1110"
        );
    }
    #[test]
    fn tape_headers_and_checksums_are_independently_decodable() {
        let data = compile_basic(&TARGETS[1], "10 PRINT 1\n").unwrap();
        let mut i = 0;
        let mut blocks = Vec::new();
        while i < data.len() {
            let n = usize::from(u16::from_le_bytes([data[i], data[i + 1]]));
            i += 2;
            let b = &data[i..i + n];
            assert_eq!(b.iter().fold(0, |a, b| a ^ b), 0);
            blocks.push(b);
            i += n;
        }
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0][1], 0);
        assert_eq!(&blocks[0][14..16], &10u16.to_le_bytes());
        assert_eq!(&blocks[1][1..3], &10u16.to_be_bytes());
        assert_eq!(blocks[1][5], 245);
    }
    #[test]
    fn text_and_comment_keywords_are_not_tokenised() {
        let data = eightbit_sinclair_basic::program(
            "10 PRINT \"PRINT 12\"\n20 REM PRINT 34\n",
            &dialect(),
        )
        .unwrap();
        assert!(data.windows(8).any(|w| w == b"PRINT 12"));
        assert!(data.windows(8).any(|w| w == b"PRINT 34"));
    }
    #[test]
    fn malformed_and_memory_boundaries() {
        for s in [
            "",
            "10 PRINT \"x",
            "20 PRINT 1\n10 PRINT 2",
            "10 PRINT 1e999",
            "10 INCLUDE \"x\"",
        ] {
            assert!(compile_basic(&TARGETS[0], s).is_err());
        }
        assert!(
            compile_basic(
                &TARGETS[0],
                &format!("10 REM {}\n20 REM {}\n", "X".repeat(4000), "X".repeat(4000))
            )
            .is_err()
        );
        for t in TARGETS {
            assert!(compile_basic(t, PROFILES[0].basic_template).is_ok());
            assert!(package_assembly(t, &[0xc9]).is_ok());
        }
    }
}

/// Original 48K SNA for standalone assembly: PC is popped from the saved stack.
/// Format provenance: worldofspectrum.net/zx-modules/fileformats/snaformat.html.
fn debug_snapshot(code: &[u8]) -> Result<Vec<u8>> {
    if code.is_empty() || code.len() > 8192 {
        return Err(error("DSS1400", "Invalid debug programme size"));
    }
    let mut image = vec![0; 27 + 49152];
    image[0] = 0x3f;
    image[23..25].copy_from_slice(&0xfffcu16.to_le_bytes());
    image[25] = 1;
    image[26] = 7;
    image[27 + 8192..27 + 8192 + code.len()].copy_from_slice(code);
    image[27 + 0xbffc..27 + 0xbffe].copy_from_slice(&0x6000u16.to_le_bytes());
    Ok(image)
}

#[cfg(all(test, feature = "native"))]
mod mame_tests {
    use super::*;
    use std::{path::PathBuf, sync::atomic::AtomicBool, time::Duration};
    static ENGINE: eightbit_source::Engine = eightbit_source::Engine {
        product: "Eightbit Studio Z80",
        families: &[&FAMILY],
        foreign_manifests: &[],
    };
    #[test]
    fn debug_snapshot_has_original_ram_and_stack_contract() {
        let image = debug_snapshot(&[0x3e, 2, 0xc9]).unwrap();
        assert_eq!(image.len(), 49179);
        assert_eq!(&image[23..25], &0xfffcu16.to_le_bytes());
        assert_eq!(&image[27 + 0xbffc..27 + 0xbffe], &0x6000u16.to_le_bytes());
        assert_eq!(&image[27 + 8192..27 + 8195], &[0x3e, 2, 0xc9]);
        assert!(debug_snapshot(&[]).is_err());
        assert!(debug_snapshot(&vec![0; 8193]).is_err());
    }
    #[test]
    #[ignore = "Requires explicitly supplied MAME 0.289, SjASMPlus 1.24.0 and exact UK Spectrum 48K ROM"]
    fn real_mame_debug_lifecycle() {
        let settings = eightbit_source::runtime::Settings {
            emulators: std::collections::BTreeMap::default(),
            schema_version: 2,
            emulator: Some(PathBuf::from(std::env::var("STUDIO_MAME").unwrap())),
            assembler: Some(PathBuf::from(std::env::var("STUDIO_SJASMPLUS").unwrap())),
            roms: [(
                "spectrum48".into(),
                PathBuf::from(std::env::var("STUDIO_SPECTRUM48_ROM").unwrap()),
            )]
            .into(),
        };
        mame_debug_lifecycle(&settings);
    }
    #[test]
    #[ignore = "requires explicitly supplied MAME 0.289 and exact UK Spectrum 48K ROM"]
    fn real_mame_original_assembler_debug_lifecycle() {
        let settings = eightbit_source::runtime::Settings {
            emulators: std::collections::BTreeMap::default(),
            schema_version: 2,
            emulator: Some(PathBuf::from(std::env::var("STUDIO_MAME").unwrap())),
            assembler: None,
            roms: [(
                "spectrum48".into(),
                PathBuf::from(std::env::var("STUDIO_SPECTRUM48_ROM").unwrap()),
            )]
            .into(),
        };
        mame_debug_lifecycle(&settings);
    }
    #[test]
    #[ignore = "requires explicitly supplied MAME 0.289 and exact UK Spectrum 48K ROM"]
    fn real_mame_stock_rom_loads_original_small_and_full_size_cassettes() {
        mame_stock_cassettes(
            &PROFILES[1],
            &PathBuf::from(std::env::var("STUDIO_SPECTRUM48_ROM").unwrap()),
        );
    }
    #[test]
    #[ignore = "requires supplied MAME and exact UK 128K/original UK +2 ROMs"]
    fn real_mame_banked_models_load_original_small_and_full_size_cassettes() {
        for (profile, key) in [
            (&PROFILES[2], "STUDIO_SPECTRUM128_ROM"),
            (&PROFILES[3], "STUDIO_SPECTRUM_PLUS2_ROM"),
        ] {
            mame_stock_cassettes(profile, &PathBuf::from(std::env::var(key).unwrap()));
        }
    }
    fn mame_stock_cassettes(profile: &Profile, rom: &std::path::Path) {
        let executable = PathBuf::from(std::env::var("STUDIO_MAME").unwrap());
        let cancel = AtomicBool::new(false);
        eightbit_source::runtime::probe(&executable, &cancel).unwrap();
        for full in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            eightbit_source::runtime::stage_rom(profile, rom, directory.path()).unwrap();
            let source = format!(
                "ORG 24576\nLD A,42\nLD (23296),A\nRET\n{}",
                if full { "DS 40954,165\n" } else { "" }
            );
            let built =
                assemble_original(profile.target, &source, "src/asm/main.asm", &"a".repeat(64))
                    .unwrap();
            std::fs::write(directory.path().join("programme.tap"), built.media).unwrap();
            // Only fixed adapter actions and numeric observations; no ROM reads,
            // snapshots or source from the reference implementation are retained.
            let script = format!(
                r#"local frames = 0
emu.register_frame_done(function()
 frames = frames + 1
 if frames == 250 then manager.machine.natkeyboard:post_coded('{boot}') end
 if frames == 500 then
  local cassette = manager.machine.cassettes[":cassette"]
  cassette:seek(0,"set"); cassette.motor_state = true; cassette:play()
 end
 local space = manager.machine.devices[":maincpu"].spaces["program"]
 if frames > 500 and space:read_u8(23296) == 42 and ({} or space:read_u8(65535) == 165) then
  local receipt = assert(io.open("receipt.txt","w")); receipt:write("cassette-loaded\n"); receipt:close()
  manager.machine:exit()
 end
 if frames == 15000 then manager.machine:exit() end
end)
"#,
                if full { "false" } else { "true" },
                boot = if profile.target.id == "spectrum48" {
                    "j\"\"{ENTER}"
                } else {
                    "{ENTER}"
                }
            );
            std::fs::write(directory.path().join("loader.lua"), script).unwrap();
            let args: Vec<String> = [
                profile.machine,
                "-noreadconfig",
                "-noplugins",
                "-nohttp",
                "-skip_gameinfo",
                "-rompath",
                ".",
                "-bios",
                "en",
                "-ramsize",
                if profile.target.ram_kib == 48 {
                    "48K"
                } else {
                    "128K"
                },
                "-cassette",
                "programme.tap",
                "-natural",
                "-script",
                "loader.lua",
                "-video",
                "none",
                "-sound",
                "none",
                "-nothrottle",
                "-seconds_to_run",
                "310",
            ]
            .into_iter()
            .map(String::from)
            .collect();
            let output =
                eightbit_process::Process::spawn(&executable, &args, directory.path(), true)
                    .unwrap()
                    .wait(Duration::from_secs(60), &cancel, 8192)
                    .unwrap();
            assert!(
                output.status.success(),
                "stock-ROM cassette process failed: {} {}",
                String::from_utf8_lossy(&output.stderr),
                String::from_utf8_lossy(&output.stdout)
            );
            assert_eq!(
                std::fs::read_to_string(directory.path().join("receipt.txt")).unwrap_or_default(),
                "cassette-loaded\n",
                "stock-ROM cassette journey, full size {full}"
            );
        }
    }
    fn mame_debug_lifecycle(settings: &eightbit_source::runtime::Settings) {
        let directory = tempfile::tempdir().unwrap();
        let p = ENGINE
            .create(
                directory.path(),
                "original",
                "spectrum",
                "spectrum48",
                "assembly",
            )
            .unwrap();
        let cancel = AtomicBool::new(false);
        for _ in 0..2 {
            let mut debug = eightbit_source::debug::DebugSession::start_with_presentation(
                &ENGINE,
                &p.root,
                settings,
                &eightbit_source::debug::Breakpoint {
                    line: Some(3),
                    address: None,
                },
                &cancel,
                true,
            )
            .unwrap();
            assert_eq!(debug.snapshot.registers.as_ref().unwrap()["pc"], 0x6002);
            assert_eq!(debug.snapshot.registers.as_ref().unwrap()["af"] >> 8, 2);
            assert_eq!(debug.snapshot.source_line, Some(3));
            let stepped = debug
                .advance(
                    &ENGINE,
                    debug.snapshot.session_id,
                    debug.snapshot.stop_id,
                    true,
                    &cancel,
                )
                .unwrap();
            assert_eq!(stepped.registers.as_ref().unwrap()["pc"], 0x6004);
            let interrupt = debug.interrupt_handle().unwrap();
            let pause = std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(150));
                interrupt.interrupt().unwrap();
            });
            debug
                .advance(
                    &ENGINE,
                    debug.snapshot.session_id,
                    debug.snapshot.stop_id,
                    false,
                    &cancel,
                )
                .unwrap();
            pause.join().unwrap();
            debug.runtime.stop();
            assert_eq!(debug.runtime.status().unwrap().state, "exited");
        }
    }
}

/// Bounded BASIC TAP recovery with exact block and numerical round-trip checks.
pub fn decompile_basic(target: &TargetDefinition, bytes: &[u8]) -> Result<String> {
    fn block_at(bytes: &[u8], pos: &mut usize, flag: u8) -> Result<Vec<u8>> {
        let length = bytes
            .get(*pos..*pos + 2)
            .ok_or_else(|| error("DSS1120", "Truncated TAP block"))?;
        let length = usize::from(u16::from_le_bytes([length[0], length[1]]));
        *pos += 2;
        let data = bytes
            .get(*pos..*pos + length)
            .ok_or_else(|| error("DSS1120", "Truncated TAP payload"))?;
        if data.len() < 2 || data[0] != flag || data.iter().fold(0u8, |sum, b| sum ^ b) != 0 {
            return Err(error("DSS1120", "Invalid TAP flag or checksum"));
        }
        *pos += length;
        Ok(data[1..data.len() - 1].to_vec())
    }
    if bytes.len() > 65536 {
        return Err(error("DSS1120", "TAP exceeds the supported limit"));
    }
    let mut pos = 0;
    let header = block_at(bytes, &mut pos, 0)?;
    if header.len() != 17 || header[0] != 0 {
        return Err(error("DSS1120", "Choose a single BASIC programme TAP"));
    }
    let data = block_at(bytes, &mut pos, 255)?;
    if pos != bytes.len()
        || usize::from(u16::from_le_bytes([header[11], header[12]])) != data.len()
        || usize::from(u16::from_le_bytes([header[15], header[16]])) != data.len()
    {
        return Err(error(
            "DSS1120",
            "Additional tape files or BASIC variables are unsupported",
        ));
    }
    let language = target_dialect(target);
    let source = eightbit_sinclair_basic::listing(&data, &language)?;
    compile_basic(target, &source)?;
    Ok(source)
}
