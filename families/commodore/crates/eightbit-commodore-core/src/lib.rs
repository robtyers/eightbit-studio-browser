//! Commodore-owned C64 target, native BASIC V2 PRG and VICE runtime contracts.
#![allow(missing_docs)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
pub mod basic;
pub mod firmware;
#[cfg(feature = "native")]
pub mod vice;
use eightbit_source_core::{Family, Profile, Result, error};
use eightbit_targets::{Capability, FamilyDefinition, ProjectTemplate, TargetDefinition};
pub static TARGETS: &[TargetDefinition] = &[TargetDefinition {
    version: 1,
    id: "c64-pal",
    family: "commodore",
    display_name: "Commodore 64 (PAL)",
    model: "c64",
    mode: "basic-v2",
    cpu: "mos6502",
    dialect: Some("commodore-basic-v2"),
    ram_kib: 64,
    tv: Some("pal"),
    templates: &[
        ProjectTemplate {
            id: "basic",
            name: "Commodore BASIC V2",
            description: "An original numbered BASIC program, built offline as a PRG.",
        },
        ProjectTemplate {
            id: "assembly",
            name: "6502 assembly",
            description: "A bounded ca65 program with a SYS loader; requires ca65 and ld65.",
        },
    ],
    capabilities: &[
        Capability::SourceEditing,
        Capability::Build,
        Capability::Run,
        Capability::ProgramExport,
    ],
}];
pub static DEFINITION: FamilyDefinition = FamilyDefinition {
    id: "commodore",
    display_name: "Commodore",
    targets: TARGETS,
};
pub static PROFILES: &[Profile] = &[Profile {
    #[cfg(feature = "native")]
    adapter: Some(&vice::ADAPTER),
    #[cfg(not(feature = "native"))]
    adapter: None,
    target: &TARGETS[0],
    boot_command: "",
    rom_names: &[],
    debug_snapshot: None,
    machine: "c64",
    rom_size: 20480,
    rom_sha256: "",
    extension: "prg",
    media_limit: 65536,
    load_option: "-autostart",
    machine_options: &[],
    assembly_language: "mos6502",
    origin: 2061,
    assembly_limit: 32768,
    basic_template: "10 PRINT CHR$(147)\n20 PRINT \"HELLO FROM EIGHTBIT STUDIO 6502\"\n30 POKE 53280,6\n40 POKE 53281,0\n50 FOR N=1 TO 5\n60 PRINT N\n70 NEXT N\n",
    assembly_template: "; C64 PAL: SYS loader calls $080D. Return to BASIC with RTS.\n.org $080D\n    lda #6\n    sta $D020\n    lda #0\n    sta $D021\n    rts\n",
    basic: compile_basic,
    assembly: package_assembly,
}];
pub static FAMILY: Family = Family {
    guidance: eightbit_source_core::ProjectGuidance {
        readme: include_str!("../../../templates/project-README.md"),
        agents: include_str!("../../../templates/project-AGENTS.md"),
        tests: include_str!("../../../templates/project-tests-README.md"),
    },
    definition: &DEFINITION,
    manifest: "c64.toml",
    profiles: PROFILES,
};
fn compile_basic(_: &TargetDefinition, source: &str) -> Result<Vec<u8>> {
    basic::compile(source)
}
pub fn package_assembly(_: &TargetDefinition, code: &[u8]) -> Result<Vec<u8>> {
    if code.is_empty() || code.len() > 32768 {
        return Err(error("DC64102", "Machine code must contain 1–32768 bytes"));
    }
    // Original 10 SYS2061 loader: PRG load address, next-line link, line, SYS, digits, terminators.
    let mut prg = vec![
        0x01, 0x08, 0x0b, 0x08, 10, 0, 0x9e, b'2', b'0', b'6', b'1', 0, 0, 0,
    ];
    prg.extend_from_slice(code);
    Ok(prg)
}
#[cfg(all(test, feature = "native"))]
mod tests;
