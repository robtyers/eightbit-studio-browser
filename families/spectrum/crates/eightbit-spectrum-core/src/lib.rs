//! Family-owned targets, firmware profiles and native BASIC/media semantics.
#![allow(missing_docs)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#[cfg(test)]
mod help_tests;
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
        description: "A bounded single-source Z80 program with a native BASIC loader; requires SjASMPlus.",
    },
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
        capabilities: &[
            Capability::SourceEditing,
            Capability::Build,
            Capability::CassetteExport,
            Capability::Run,
            Capability::MachineDebug,
        ],
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
        capabilities: CAPABILITIES,
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
        capabilities: CAPABILITIES,
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
        escaped_quote: None,
        encode,
        number_marker: 14,
        newline: 13,
        max_line: 9999,
        statements: true,
    }
}
pub fn compile_basic(target: &TargetDefinition, source: &str) -> Result<Vec<u8>> {
    let mut language = dialect();
    if target.ram_kib == 128 {
        language.extra_tokens = &[("SPECTRUM", 163), ("PLAY", 164)];
    }
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
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
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
        assert!(compile_basic(&TARGETS[1], "10 DEF FN f(x)=x\n").is_err());
        assert!(compile_basic(&TARGETS[1], "10 PLAY \"c\"\n").is_err());
        let data = compile_basic(&TARGETS[2], "10 PLAY \"c\"\n").unwrap();
        assert!(data.contains(&164));
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

#[cfg(test)]
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
                &settings,
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
    let mut language = dialect();
    if target.ram_kib == 128 {
        language.extra_tokens = &[("SPECTRUM", 163), ("PLAY", 164)];
    }
    let source = eightbit_sinclair_basic::listing(&data, &language)?;
    compile_basic(target, &source)?;
    Ok(source)
}
