//! Acorn-owned BBC Model B BASIC II, DFS and supervised MAME contracts.
#![allow(missing_docs)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
pub mod basic;
pub mod dfs;
pub mod firmware;
#[cfg(feature = "native")]
pub mod runtime;
use eightbit_source_core::{Family, Profile, Result, error};
use eightbit_targets::{Capability, FamilyDefinition, ProjectTemplate, TargetDefinition};

pub const ORIGIN: u16 = 0x1900;
// Preserve the lowest screen base across the Model B's native screen modes.
pub const PROGRAM_LIMIT: usize = 0x3000 - 0x1900;
pub static TARGETS: &[TargetDefinition] = &[TargetDefinition {
    version: 1,
    id: "bbc-model-b",
    family: "acorn",
    display_name: "BBC Micro Model B (32 KB)",
    model: "bbc-model-b",
    mode: "basic-ii-dnfs-120",
    cpu: "mos6502",
    dialect: Some("bbc-basic-ii"),
    ram_kib: 32,
    tv: Some("pal"),
    templates: &[
        ProjectTemplate {
            id: "basic",
            name: "BBC BASIC II",
            description: "An original numbered BASIC II programme, built offline as a DFS disc.",
        },
        ProjectTemplate {
            id: "assembly",
            name: "6502 assembly",
            description: "A bounded ca65 programme at $1900; requires ca65 and ld65.",
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
    id: "acorn",
    display_name: "Acorn",
    targets: TARGETS,
};
pub static PROFILES: &[Profile] = &[Profile {
    #[cfg(feature = "native")]
    adapter: Some(&runtime::ADAPTER),
    #[cfg(not(feature = "native"))]
    adapter: None,
    target: &TARGETS[0],
    boot_command: "",
    rom_names: &[
        "os12.rom",
        "basic2.rom",
        "dnfs120.rom",
        "cm62024.bin",
        "saa5050",
    ],
    debug_snapshot: None,
    machine: "bbcb",
    rom_size: 65536 + 960,
    rom_sha256: "",
    extension: "ssd",
    media_limit: dfs::IMAGE_SIZE,
    load_option: "-flop1",
    machine_options: &[],
    assembly_language: "mos6502",
    origin: ORIGIN,
    assembly_limit: PROGRAM_LIMIT,
    basic_template: "10 MODE 7\n20 PRINT \"HELLO FROM EIGHTBIT STUDIO 6502\"\n30 FOR N%=1 TO 5\n40 PRINT N%\n50 NEXT N%\n60 END\n",
    assembly_template: "; BBC Model B: MODE 7, load and execute at $1900. Return with RTS.\n.org $1900\n    lda #65\n    jsr $FFEE\n    lda #13\n    jsr $FFE7\n    rts\n",
    basic: compile_basic,
    assembly: package_assembly,
}];
pub static FAMILY: Family = Family {
    definition: &DEFINITION,
    manifest: "bbc.toml",
    profiles: PROFILES,
    guidance: eightbit_source_core::ProjectGuidance {
        readme: include_str!("../../../templates/project-README.md"),
        agents: include_str!("../../../templates/project-AGENTS.md"),
        tests: include_str!("../../../templates/project-tests-README.md"),
    },
};
fn compile_basic(_: &TargetDefinition, source: &str) -> Result<Vec<u8>> {
    dfs::package(&basic::compile(source)?, false)
}
pub fn package_assembly(_: &TargetDefinition, code: &[u8]) -> Result<Vec<u8>> {
    if code.is_empty() || code.len() > PROGRAM_LIMIT {
        return Err(error(
            "DBBC102",
            "BBC machine code must fit between $1900 and $3000",
        ));
    }
    dfs::package(code, true)
}
#[cfg(all(test, feature = "native"))]
mod tests;
