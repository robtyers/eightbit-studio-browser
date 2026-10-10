//! Single registration point for implemented Dragon machine/mode support.
use eightbit_targets::{
    Capability, FamilyDefinition, ProjectTemplate, TargetDefinition, TargetError, TargetRegistry,
};

/// Existing Dragon 32 PAL BASIC configuration, preserved without a schema migration.
pub const DRAGON32: TargetDefinition = TargetDefinition {
    version: 1,
    id: "dragon32-pal",
    family: "dragon",
    display_name: "Dragon 32",
    model: "dragon32",
    mode: "basic-32k",
    cpu: "m6809",
    dialect: Some(BASIC_DIALECT),
    ram_kib: 32,
    tv: Some("pal"),
    templates: &[
        ProjectTemplate {
            id: "basic",
            name: "Dragon BASIC",
            description: "A ready-to-run BASIC program. No assembler setup required.",
        },
        ProjectTemplate {
            id: "basic-assembly",
            name: "BASIC + 6809 Assembly",
            description: "Includes a BASIC launcher and assembly entry point. Requires LWTOOLS to build.",
        },
    ],
    capabilities: &[
        Capability::SourceEditing,
        Capability::Build,
        Capability::Run,
        Capability::MachineDebug,
        Capability::BasicDebug,
        Capability::CassetteExport,
        Capability::CartridgeExport,
    ],
};
/// PAL Dragon 64 hardware using the bounded 32K BASIC compatibility contract.
pub const DRAGON64: TargetDefinition = TargetDefinition {
    id: "dragon64-pal",
    display_name: "Dragon 64 (32K BASIC)",
    model: "dragon64",
    ram_kib: 64,
    capabilities: &[
        Capability::SourceEditing,
        Capability::Build,
        Capability::Run,
        Capability::MachineDebug,
        Capability::BasicDebug,
        Capability::CassetteExport,
        Capability::CartridgeExport,
    ],
    ..DRAGON32
};
/// PAL Dragon 64 with relocated BASIC and an explicit native memory contract.
pub const DRAGON64_NATIVE: TargetDefinition = TargetDefinition {
    id: "dragon64-pal-native",
    display_name: "Dragon 64 (64K BASIC)",
    mode: "basic-64k",
    dialect: Some("dragon64-basic"),
    capabilities: &[
        Capability::SourceEditing,
        Capability::Build,
        Capability::Run,
        Capability::MachineDebug,
        Capability::BasicDebug,
        Capability::CassetteExport,
    ],
    ..DRAGON64
};
/// Only actually implemented targets are registered; future variants require verified implementations.
pub static FAMILY: FamilyDefinition = FamilyDefinition {
    id: "dragon",
    display_name: "Dragon",
    targets: &[DRAGON32, DRAGON64, DRAGON64_NATIVE],
};
/// Compose Eightbit Studio 6809's current built-in family support.
pub fn registry() -> Result<TargetRegistry, TargetError> {
    TargetRegistry::new(&[&FAMILY])
}
/// Narrow target capabilities for an already validated project's boot profile.
#[must_use]
pub fn project_summary(
    target: &TargetDefinition,
    cartridge: bool,
) -> eightbit_targets::TargetSummary {
    let mut summary = target.summary();
    if cartridge {
        summary.capabilities = if target.id == DRAGON64.id {
            &[
                Capability::SourceEditing,
                Capability::Build,
                Capability::Run,
                Capability::MachineDebug,
                Capability::CartridgeExport,
            ]
        } else {
            &[
                Capability::SourceEditing,
                Capability::Build,
                Capability::Run,
                Capability::CartridgeExport,
            ]
        };
    }
    summary
}
/// Resolve the legacy manifest's exact machine configuration.
pub fn resolve_machine(
    model: &str,
    ram_kib: u32,
    tv: &str,
) -> Result<&'static TargetDefinition, TargetError> {
    resolve_legacy_target(&[DRAGON32, DRAGON64], model, ram_kib, tv)
}

/// Resolve an explicit mode without allowing registration order to select it.
pub fn resolve_mode(
    model: &str,
    ram_kib: u32,
    tv: &str,
    mode: Option<&str>,
) -> Result<&'static TargetDefinition, TargetError> {
    match mode {
        None => resolve_machine(model, ram_kib, tv),
        Some(mode) => registry()?
            .targets("dragon")?
            .iter()
            .find(|target| {
                target.model == model
                    && target.ram_kib == ram_kib
                    && target.tv == Some(tv)
                    && target.mode == mode
            })
            .ok_or(TargetError("unsupported Dragon operating mode")),
    }
}

fn resolve_legacy_target(
    targets: &'static [TargetDefinition],
    model: &str,
    ram_kib: u32,
    tv: &str,
) -> Result<&'static TargetDefinition, TargetError> {
    let mut matches = targets.iter().filter(|target| {
        target.model == model && target.ram_kib == ram_kib && target.tv == Some(tv)
    });
    let target = matches
        .next()
        .ok_or(TargetError("unsupported Dragon machine configuration"))?;
    // Legacy manifests do not select a mode. Adding a mode must never silently
    // change their meaning according to registration order.
    if matches.next().is_some() {
        return Err(TargetError("ambiguous Dragon operating mode"));
    }
    Ok(target)
}
/// Built-in ROM slot and emulator mapping for the implemented Dragon 32 mode.
pub const EXTENDED_BASIC_ROM_BYTES: u64 = 16 * 1024;
/// Legacy exact-ROM profile identity; unknown ROMs never inherit interpreter support.
pub const BASIC_ROM_SHA256: &str =
    "fc0e900bfec6b52f0f80ba1e65a4712808d2a411b5b00496639ef1a2152351f1";

/// Reviewed PAL Dragon 64 32K BASIC firmware identity.
pub const D64_PRIMARY_SHA256: &str =
    "706d3865234494332afebd83ec78e8736b072b5e9fa503f977d5a343218f4347";
/// Reviewed PAL Dragon 64 alternate firmware identity.
pub const D64_ALTERNATE_SHA256: &str =
    "c3d0b8fd92020c939ec97388402d10b3890e6e26455963d50cc00b5f9cd68fa5";

/// Family-owned BASIC start address for the implemented Dragon 32 profile.
pub const BASIC_START: u16 = 0x1e01;
/// Family-owned end of Dragon 32 BASIC RAM.
pub const BASIC_END: u16 = 0x7fff;
/// Exact implemented BASIC dialect identifier.
pub const BASIC_DIALECT: &str = "dragon32-basic";

#[cfg(test)]
mod tests {
    #[test]
    fn legacy_selection_rejects_multiple_modes_for_the_same_machine() {
        static TARGETS: &[eightbit_targets::TargetDefinition] = &[
            super::DRAGON32,
            eightbit_targets::TargetDefinition {
                id: "synthetic-mode",
                mode: "synthetic-mode",
                ..super::DRAGON32
            },
        ];
        assert_eq!(
            super::resolve_legacy_target(TARGETS, "dragon32", 32, "pal").err(),
            Some(eightbit_targets::TargetError(
                "ambiguous Dragon operating mode"
            ))
        );
    }

    #[test]
    fn cartridge_project_capabilities_follow_verified_debug_support() {
        use super::*;
        let d32 = project_summary(&DRAGON32, true);
        let d64 = project_summary(&DRAGON64, true);
        assert!(!d32.capabilities.contains(&Capability::MachineDebug));
        assert!(d64.capabilities.contains(&Capability::MachineDebug));
        for cartridge in [d32, d64] {
            assert!(!cartridge.capabilities.contains(&Capability::BasicDebug));
            assert!(!cartridge.capabilities.contains(&Capability::CassetteExport));
            assert!(
                cartridge
                    .capabilities
                    .contains(&Capability::CartridgeExport)
            );
        }
        for target in [DRAGON32, DRAGON64, DRAGON64_NATIVE] {
            assert_eq!(
                project_summary(&target, false).capabilities,
                target.capabilities
            );
        }
    }

    #[test]
    fn existing_target_is_exact_and_unimplemented_variants_are_rejected() {
        assert_eq!(
            super::resolve_machine("dragon32", 32, "pal").map(|t| t.dialect),
            Ok(Some("dragon32-basic"))
        );
        assert_eq!(
            super::resolve_machine("dragon64", 64, "pal").map(|t| t.mode),
            Ok("basic-32k")
        );
        assert!(super::resolve_machine("dragon64", 32, "pal").is_err());
        assert!(super::resolve_machine("dragon32", 32, "ntsc").is_err());
    }
}
