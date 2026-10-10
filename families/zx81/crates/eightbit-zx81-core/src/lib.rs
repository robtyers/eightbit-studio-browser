//! Family-owned targets, firmware profiles and native BASIC/media semantics.
#![allow(missing_docs)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
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
        id: "zx81-1k",
        family: "zx81",
        display_name: "ZX81 1K",
        model: "zx81-1k",
        mode: "native-basic",
        cpu: "z80",
        dialect: Some("zx81-basic"),
        ram_kib: 1,
        tv: Some("pal"),
        templates: TEMPLATES,
        capabilities: CAPABILITIES,
    },
    TargetDefinition {
        version: 1,
        id: "zx81-16k",
        family: "zx81",
        display_name: "ZX81 16K",
        model: "zx81-16k",
        mode: "native-basic",
        cpu: "z80",
        dialect: Some("zx81-basic"),
        ram_kib: 16,
        tv: Some("pal"),
        templates: TEMPLATES,
        capabilities: CAPABILITIES,
    },
];
pub static DEFINITION: FamilyDefinition = FamilyDefinition {
    id: "zx81",
    display_name: "ZX81",
    targets: TARGETS,
};
pub static PROFILES: &[Profile] = &[
    Profile {
        adapter: None,
        target: &TARGETS[0],
        machine: "zx81",
        boot_command: "j\"\"{ENTER}",
        rom_names: &["zx81a.rom"],
        debug_snapshot: None,
        load_option: "-cassette",
        machine_options: &["-bios", "2nd", "-ramsize", "1K"],
        assembly_language: "z80",
        rom_size: 8192,
        rom_sha256: "14ad84f4243efcd41587ff46ab932d11087043e8d455a1ed2a227b9657828dfa",
        extension: "p",
        origin: 16514,
        assembly_limit: 256,
        basic_template: "10 CLS\n20 PRINT \"HELLO FROM EIGHTBIT STUDIO Z80\"\n30 FOR N=1 TO 5\n40 PRINT N\n50 NEXT N\n",
        assembly_template: "    ORG 16514\n    LD BC,42\n    RET\n",
        basic: compile_basic,
        assembly: package_assembly,
    },
    Profile {
        adapter: None,
        target: &TARGETS[1],
        machine: "zx81",
        boot_command: "j\"\"{ENTER}",
        rom_names: &["zx81a.rom"],
        debug_snapshot: None,
        load_option: "-cassette",
        machine_options: &["-bios", "2nd", "-ramsize", "16K"],
        assembly_language: "z80",
        rom_size: 8192,
        rom_sha256: "14ad84f4243efcd41587ff46ab932d11087043e8d455a1ed2a227b9657828dfa",
        extension: "p",
        origin: 16514,
        assembly_limit: 12000,
        basic_template: "10 CLS\n20 PRINT \"HELLO FROM EIGHTBIT STUDIO Z80\"\n30 FOR N=1 TO 5\n40 PRINT N\n50 NEXT N\n",
        assembly_template: "    ORG 16514\n    LD BC,42\n    RET\n",
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
    manifest: "zx81.toml",
    profiles: PROFILES,
};
fn encode(byte: u8) -> Option<u8> {
    let b = byte.to_ascii_uppercase();
    match b {
        b' ' => Some(0),
        b'"' => Some(11),
        b'$' => Some(13),
        b':' => Some(14),
        b'?' => Some(15),
        b'(' => Some(16),
        b')' => Some(17),
        b'>' => Some(18),
        b'<' => Some(19),
        b'=' => Some(20),
        b'+' => Some(21),
        b'-' => Some(22),
        b'*' => Some(23),
        b'/' => Some(24),
        b';' => Some(25),
        b',' => Some(26),
        b'.' => Some(27),
        b'0'..=b'9' => Some(b - b'0' + 28),
        b'A'..=b'Z' => Some(b - b'A' + 38),
        _ => None,
    }
}
fn dialect() -> eightbit_sinclair_basic::Dialect {
    eightbit_sinclair_basic::Dialect {
        tokens: tokens::TOKENS,
        extra_tokens: &[],
        escaped_quote: Some(192),
        encode,
        number_marker: 126,
        newline: 118,
        max_line: 9999,
        statements: false,
    }
}
pub fn compile_basic(target: &TargetDefinition, source: &str) -> Result<Vec<u8>> {
    image(
        target,
        &eightbit_sinclair_basic::program(source, &dialect())?,
    )
}
fn image(target: &TargetDefinition, program: &[u8]) -> Result<Vec<u8>> {
    let display = if target.ram_kib == 1 {
        vec![118; 25]
    } else {
        let mut d = vec![118];
        for _ in 0..24 {
            d.extend([0; 32]);
            d.push(118);
        }
        d
    };
    let dfile = 16509 + program.len();
    let vars = dfile + display.len();
    let end = vars + 1;
    let limit = 16384
        + usize::try_from(target.ram_kib).map_err(|_| error("DSS1111", "Invalid RAM size"))? * 1024;
    if end + 256 > limit {
        return Err(error(
            "DSS1111",
            "Program and display exceed the selected ZX81 memory budget (256 bytes reserved for stack/workspace)",
        ));
    }
    let mut out = vec![0; 116];
    fn word(out: &mut [u8], address: usize, value: usize) -> Result<()> {
        let value = u16::try_from(value).map_err(|_| error("DSS1111", "ZX81 address overflow"))?;
        out[address - 16393..address - 16393 + 2].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }
    for (address, value) in [
        (16394, 10),
        (16396, dfile),
        (16398, dfile + 1),
        (16400, vars),
        (16404, end),
        (16406, end),
        (16410, end + 1),
        (16412, end + 1),
        (16415, 16477),
        (16419, 1),
        (16421, 65535),
        (16425, 16509),
        (16436, 32768),
    ] {
        word(&mut out, address, value)?;
    }
    out[16418 - 16393] = 2;
    out[16423 - 16393] = 15;
    out[16424 - 16393] = 55;
    out[16440 - 16393] = 188;
    out[16441 - 16393] = 33;
    out[16442 - 16393] = 24;
    out[16443 - 16393] = 128;
    out[16476 - 16393] = 118;
    out.extend(program);
    out.extend(display);
    out.push(128);
    Ok(out)
}
pub fn package_assembly(target: &TargetDefinition, code: &[u8]) -> Result<Vec<u8>> {
    let limit = if target.ram_kib == 1 { 256 } else { 12000 };
    if code.is_empty() || code.len() > limit {
        return Err(error(
            "DSS1111",
            "Machine code exceeds the ZX81 target budget",
        ));
    }
    let mut program = vec![0, 10];
    program.extend(
        u16::try_from(code.len() + 2)
            .map_err(|_| error("DSS1111", "Machine code too large"))?
            .to_le_bytes(),
    );
    program.push(234);
    program.extend(code);
    program.push(118);
    program.extend(eightbit_sinclair_basic::program(
        "20 PRINT USR 16514\n",
        &dialect(),
    )?);
    image(target, &program)
}
/// Recover the representable BASIC area from a bounded ZX81 P image.
pub fn decompile_basic(target: &TargetDefinition, bytes: &[u8]) -> Result<String> {
    if bytes.len() < 142 || bytes.len() > 16384 || bytes.last() != Some(&128) {
        return Err(error("DSS1120", "Choose a bounded ZX81 BASIC P image"));
    }
    let dfile = usize::from(u16::from_le_bytes([bytes[3], bytes[4]]));
    let end = dfile
        .checked_sub(16393)
        .filter(|end| *end > 116 && *end < bytes.len())
        .ok_or_else(|| error("DSS1120", "Invalid ZX81 display pointer"))?;
    let source = eightbit_sinclair_basic::listing(&bytes[116..end], &dialect())?;
    compile_basic(target, &source)?;
    Ok(source)
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
    fn p_image_pointers_and_display_match_target_ram() {
        for t in TARGETS {
            let data = compile_basic(t, "10 PRINT 1\n").unwrap();
            let word =
                |a: usize| usize::from(u16::from_le_bytes([data[a - 16393], data[a - 16393 + 1]]));
            let d = word(16396);
            let v = word(16400);
            assert_eq!(v - d, if t.ram_kib == 1 { 25 } else { 793 });
            assert_eq!(word(16404), data.len() + 16393);
            assert_eq!(data[d - 16393], 118);
            assert_eq!(data[data.len() - 1], 128);
            assert_eq!(data[120], 245);
        }
    }
    #[test]
    fn dialect_and_memory_rejections() {
        for s in [
            "10 PRINT 1: PRINT 2",
            "10 BEEP 1,2",
            "10 PRINT \"@\"",
            "10 PRINT \"oops",
        ] {
            assert!(compile_basic(&TARGETS[0], s).is_err());
        }
        let s = format!("10 REM {}\n", "A".repeat(1000));
        assert!(compile_basic(&TARGETS[0], &s).is_err());
        assert!(compile_basic(&TARGETS[1], &s).is_ok());
    }
    #[test]
    fn assembly_lives_at_documented_rem_address() {
        let data = package_assembly(&TARGETS[1], &[0x01, 42, 0, 0xc9]).unwrap();
        assert_eq!(&data[16514 - 16393..16518 - 16393], &[0x01, 42, 0, 0xc9]);
    }
}
