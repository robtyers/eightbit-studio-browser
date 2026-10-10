//! Validated built-in family/target registration. Descriptions never grant execution authority.
#![cfg_attr(test, allow(clippy::expect_used, clippy::panic, clippy::unwrap_used))]
use serde::Serialize;
use std::collections::BTreeSet;
use thiserror::Error;

/// Independently verified classes of family functionality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    /// Edit and analyse source using a registered language service.
    SourceEditing,
    /// Build a deterministic supported artefact.
    Build,
    /// Explicitly launch a fresh supported artefact.
    Run,
    /// Debug machine code using a supported CPU/backend.
    MachineDebug,
    /// Interpret BASIC only when the runtime ROM profile also permits it.
    BasicDebug,
    /// Export cassette media.
    CassetteExport,
    /// Export native program files.
    ProgramExport,
    /// Export cartridges only when the selected ROM supports startup.
    CartridgeExport,
}

/// A family-provided starting point, selected only after choosing its target.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTemplate {
    /// Family-local allow-listed template identifier.
    pub id: &'static str,
    /// User-facing name.
    pub name: &'static str,
    /// Setup and purpose explanation.
    pub description: &'static str,
}

/// One built-in machine/mode configuration. No executable paths or arbitrary flags.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetDefinition {
    /// Stable version of this definition, included in target identity.
    pub version: u32,
    /// Family-local stable identifier for the machine and operating mode.
    pub id: &'static str,
    /// Family identifier.
    pub family: &'static str,
    /// User-facing name.
    pub display_name: &'static str,
    /// Hardware model, distinct from the mode.
    pub model: &'static str,
    /// Operating mode identifier.
    pub mode: &'static str,
    /// CPU adapter identifier.
    pub cpu: &'static str,
    /// BASIC language implementation identifier.
    pub dialect: Option<&'static str>,
    /// Physical RAM configuration.
    pub ram_kib: u32,
    /// Television standard identifier.
    pub tv: Option<&'static str>,
    /// Family-owned, implemented project starting points.
    pub templates: &'static [ProjectTemplate],
    /// Implemented capability ceiling; exact runtime profiles may restrict it.
    pub capabilities: &'static [Capability],
}

/// Versioned, serialisable selected-target identity for workbench presentation.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetSummary {
    /// IPC summary version.
    pub schema_version: u32,
    /// Owning machine family.
    pub family: &'static str,
    /// Exact target identifier.
    pub id: &'static str,
    /// CPU implementation.
    pub cpu: &'static str,
    /// Includes family, mode and definition version.
    pub identity: String,
    /// User-facing target name.
    pub display_name: &'static str,
    /// Machine operating mode.
    pub mode: &'static str,
    /// Configured physical memory.
    pub ram_kib: u32,
    /// Configured video standard.
    pub tv: Option<&'static str>,
    /// Capability ceiling; exact ROM validation may further restrict operations.
    pub capabilities: &'static [Capability],
}
impl TargetDefinition {
    /// Return presentation state without exposing ROMs or executable paths.
    #[must_use]
    pub fn summary(&self) -> TargetSummary {
        TargetSummary {
            schema_version: 2,
            family: self.family,
            id: self.id,
            cpu: self.cpu,
            identity: self.identity(),
            display_name: self.display_name,
            mode: self.mode,
            ram_kib: self.ram_kib,
            tv: self.tv,
            capabilities: self.capabilities,
        }
    }
}

/// One family's registration, owned by the family package.
#[derive(Debug)]
pub struct FamilyDefinition {
    /// Stable family identifier.
    pub id: &'static str,
    /// User-facing group name.
    pub display_name: &'static str,
    /// Supported built-in targets only.
    pub targets: &'static [TargetDefinition],
}

/// Invalid application composition or unsupported target selection.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("{0}")]
pub struct TargetError(pub &'static str);

/// Immutable registry created by native application composition.
#[derive(Debug)]
pub struct TargetRegistry {
    families: Vec<&'static FamilyDefinition>,
}
impl TargetRegistry {
    /// Reject conflicting, malformed or unbounded built-in registrations.
    pub fn new(families: &[&'static FamilyDefinition]) -> Result<Self, TargetError> {
        if families.is_empty() || families.len() > 32 {
            return Err(TargetError("invalid family count"));
        }
        let mut ids = BTreeSet::new();
        for family in families {
            if !identifier(family.id)
                || !label(family.display_name, 128)
                || !ids.insert(family.id)
                || family.targets.is_empty()
                || family.targets.len() > 64
            {
                return Err(TargetError("invalid or duplicate family registration"));
            }
            let mut targets = BTreeSet::new();
            let mut configurations = BTreeSet::new();
            for target in family.targets {
                if target.family != family.id
                    || target.version == 0
                    || !identifier(target.id)
                    || !identifier(target.model)
                    || !identifier(target.mode)
                    || !identifier(target.cpu)
                    || target.dialect.is_some_and(|value| !identifier(value))
                    || target.tv.is_some_and(|value| !identifier(value))
                    || target.display_name.is_empty()
                    || target.display_name.len() > 128
                    || target.display_name.chars().any(char::is_control)
                    || target.ram_kib == 0
                    || target.ram_kib > 65_536
                    || target.templates.len() > 16
                    || target
                        .templates
                        .iter()
                        .enumerate()
                        .any(|(index, template)| {
                            !identifier(template.id)
                                || !label(template.name, 128)
                                || !label(template.description, 512)
                                || target.templates[..index]
                                    .iter()
                                    .any(|prior| prior.id == template.id)
                        })
                    || (target.supports(Capability::BasicDebug) && target.dialect.is_none())
                    || target.capabilities.len() > 7
                    || target
                        .capabilities
                        .iter()
                        .enumerate()
                        .any(|(index, value)| target.capabilities[..index].contains(value))
                    || !targets.insert(target.id)
                    || !configurations.insert((
                        target.model,
                        target.mode,
                        target.ram_kib,
                        target.tv,
                        target.dialect,
                    ))
                {
                    return Err(TargetError("invalid or ambiguous target registration"));
                }
            }
        }
        Ok(Self {
            families: families.to_vec(),
        })
    }

    /// Resolve an exact family and target. Never choose a nearest compatible target.
    pub fn resolve(
        &self,
        family: &str,
        target: &str,
    ) -> Result<&'static TargetDefinition, TargetError> {
        self.families
            .iter()
            .find(|entry| entry.id == family)
            .and_then(|entry| entry.targets.iter().find(|entry| entry.id == target))
            .ok_or(TargetError("unsupported family or target"))
    }

    /// List the application's compiled support for a family.
    pub fn targets(&self, family: &str) -> Result<&'static [TargetDefinition], TargetError> {
        self.families
            .iter()
            .find(|entry| entry.id == family)
            .map(|entry| entry.targets)
            .ok_or(TargetError("unsupported family"))
    }
}
impl TargetDefinition {
    /// Stable target/mode/version identity for stale-state and artefact boundaries.
    #[must_use]
    pub fn identity(&self) -> String {
        format!("{}:{}:{}@{}", self.family, self.id, self.mode, self.version)
    }
    /// Check the built-in capability ceiling; runtime validation remains mandatory.
    #[must_use]
    pub fn supports(&self, capability: Capability) -> bool {
        self.capabilities.contains(&capability)
    }
}
fn label(value: &str, limit: usize) -> bool {
    !value.is_empty() && value.len() <= limit && !value.chars().any(char::is_control)
}
fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIRST: TargetDefinition = TargetDefinition {
        version: 1,
        id: "small",
        family: "example",
        display_name: "Small machine",
        model: "small",
        mode: "basic",
        cpu: "cpu-a",
        dialect: Some("basic-a"),
        ram_kib: 32,
        tv: Some("pal"),
        templates: &[],
        capabilities: &[Capability::SourceEditing, Capability::Build],
    };
    static FAMILY: FamilyDefinition = FamilyDefinition {
        id: "example",
        display_name: "Example",
        targets: &[
            FIRST,
            TargetDefinition {
                id: "large",
                model: "large",
                ram_kib: 64,
                ..FIRST
            },
        ],
    };
    static OTHER: FamilyDefinition = FamilyDefinition {
        id: "other",
        display_name: "Other",
        targets: &[TargetDefinition {
            family: "other",
            cpu: "cpu-b",
            dialect: None,
            tv: None,
            ..FIRST
        }],
    };
    #[test]
    fn multiple_variants_and_cpu_families_share_registry_without_fallback() {
        let registry = TargetRegistry::new(&[&FAMILY, &OTHER]).unwrap();
        let small = registry.resolve("example", "small").unwrap();
        let large = registry.resolve("example", "large").unwrap();
        assert_ne!(small.identity(), large.identity());
        assert_eq!(registry.resolve("other", "small").unwrap().cpu, "cpu-b");
        assert!(small.supports(Capability::Build));
        assert!(!small.supports(Capability::Run));
        assert!(registry.resolve("example", "missing").is_err());
        assert!(registry.targets("missing").is_err());
        assert!(TargetRegistry::new(&[&FAMILY, &FAMILY]).is_err());
    }
    #[test]
    fn malformed_and_ambiguous_definitions_fail_closed() {
        static DUPLICATE: FamilyDefinition = FamilyDefinition {
            id: "example",
            display_name: "Example",
            targets: &[FIRST, FIRST],
        };
        static MISMATCH: FamilyDefinition = FamilyDefinition {
            id: "wrong",
            display_name: "Wrong",
            targets: &[FIRST],
        };
        static MEMORY: FamilyDefinition = FamilyDefinition {
            id: "example",
            display_name: "Example",
            targets: &[TargetDefinition {
                capabilities: &[Capability::BasicDebug],
                dialect: None,
                ..FIRST
            }],
        };
        static AMBIGUOUS: FamilyDefinition = FamilyDefinition {
            id: "example",
            display_name: "Example",
            targets: &[
                FIRST,
                TargetDefinition {
                    id: "another",
                    ..FIRST
                },
            ],
        };
        for families in [
            &[&DUPLICATE][..],
            &[&MISMATCH],
            &[&MEMORY],
            &[&AMBIGUOUS],
            &[],
        ] {
            assert!(TargetRegistry::new(families).is_err());
        }
    }
}
