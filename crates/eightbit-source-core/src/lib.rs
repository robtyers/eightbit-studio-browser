//! Portable source-project contracts, defaults and immutable family policies.
#![allow(missing_docs)]
pub mod adapter;
use eightbit_targets::{FamilyDefinition, TargetDefinition};
use serde::{Deserialize, Serialize};
#[derive(Debug, thiserror::Error, Serialize)]
#[error("{code}: {message}")]
pub struct Error {
    pub code: &'static str,
    pub message: String,
}
pub type Result<T> = std::result::Result<T, Error>;
pub fn error(code: &'static str, message: impl Into<String>) -> Error {
    Error {
        code,
        message: message.into(),
    }
}
#[must_use]
pub fn io_error(_: std::io::Error) -> Error {
    error("DSS1001", "Cannot access the selected project file")
}
#[must_use]
pub fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(bytes))
}
pub const MAX_SOURCE: usize = 128 * 1024;
pub const GITIGNORE: &str = ".studio/\n.DS_Store\n";
pub type DebugSnapshotBuilder = fn(&[u8]) -> Result<Vec<u8>>;
#[derive(Clone, Copy)]
pub struct Profile {
    pub adapter: Option<&'static adapter::Adapter>,
    pub target: &'static TargetDefinition,
    pub boot_command: &'static str,
    pub rom_names: &'static [&'static str],
    pub debug_snapshot: Option<DebugSnapshotBuilder>,
    pub machine: &'static str,
    pub rom_size: usize,
    pub rom_sha256: &'static str,
    pub extension: &'static str,
    pub load_option: &'static str,
    pub machine_options: &'static [&'static str],
    pub assembly_language: &'static str,
    pub origin: u16,
    pub assembly_limit: usize,
    pub basic_template: &'static str,
    pub assembly_template: &'static str,
    pub basic: fn(&TargetDefinition, &str) -> Result<Vec<u8>>,
    pub assembly: fn(&TargetDefinition, &[u8]) -> Result<Vec<u8>>,
}
#[derive(Clone, Copy)]
pub struct ProjectGuidance {
    pub readme: &'static str,
    pub agents: &'static str,
    pub tests: &'static str,
}
pub struct Family {
    pub guidance: ProjectGuidance,
    pub definition: &'static FamilyDefinition,
    pub manifest: &'static str,
    pub profiles: &'static [Profile],
}
pub struct Engine {
    pub families: &'static [&'static Family],
    pub foreign_manifests: &'static [&'static str],
    pub product: &'static str,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub name: String,
    pub target: String,
    pub source: String,
}
impl Engine {
    pub fn parse_manifest(
        &self,
        family: &str,
        manifest: &str,
    ) -> Result<(Manifest, &'static Profile)> {
        if manifest.len() > 8192 {
            return Err(error("DSS1000", "Manifest exceeds 8 KiB"));
        }
        let config: Manifest = toml::from_str(manifest).map_err(|_| {
            error(
                "DSS1000",
                "Invalid project manifest; use the documented schema-1 fields",
            )
        })?;
        let profile = self.profile(family, &config.target)?;
        if config.schema_version != 1
            || config.name.trim().is_empty()
            || config.name.len() > 80
            || config.name.chars().any(char::is_control)
            || !config.source.starts_with("src/")
            || !matches!(config.source.rsplit('.').next(), Some("bas" | "asm"))
        {
            return Err(error("DSS1000", "Unsupported source-project configuration"));
        }
        if config.source.len() > 240
            || config.source.contains('\\')
            || config.source.contains('\0')
            || config
                .source
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(error("DSS1002", "Path must stay inside the project"));
        }
        Ok((config, profile))
    }

    /// Pure desktop-compatible initial files; hosts own filesystem publication.
    pub fn scaffold(
        &self,
        name: &str,
        family: &str,
        target: &str,
        template: &str,
    ) -> Result<std::collections::BTreeMap<String, String>> {
        let profile = self.profile(family, target)?;
        if !matches!(template, "basic" | "assembly")
            || !profile.target.templates.iter().any(|t| t.id == template)
        {
            return Err(error("DSS1000", "Unsupported project template"));
        }
        if name.is_empty()
            || name.len() > 64
            || !name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            || name.starts_with('-')
            || name.ends_with('-')
        {
            return Err(error(
                "DSS1000",
                "Use lowercase letters, digits and internal hyphens for the project folder",
            ));
        }
        let family = self
            .families
            .iter()
            .find(|f| f.definition.id == family)
            .ok_or_else(|| error("DSS1000", "Unsupported family"))?;
        let (source_path, source) = if template == "assembly" {
            ("src/asm/main.asm", profile.assembly_template)
        } else {
            ("src/basic/main.bas", profile.basic_template)
        };
        let config = Manifest {
            schema_version: 1,
            name: name.into(),
            target: target.into(),
            source: source_path.into(),
        };
        let manifest =
            toml::to_string(&config).map_err(|_| error("DSS1000", "Cannot serialise project"))?;
        let context = format!(
            "Product: {}. Target: **{}** (`{target}`).\n\nTemplate: `{template}`. Configured source: `{source_path}`. Manifest: `{}` (schema 1).\n\nBASIC dialect: `{}`. Assembly: `{}`, origin `${:04X}` ({}), maximum {} code bytes. Export: `.{}`.\n\n",
            self.product,
            profile.target.display_name,
            family.manifest,
            profile.target.dialect.unwrap_or("none"),
            profile.assembly_language,
            profile.origin,
            profile.origin,
            profile.assembly_limit,
            profile.extension,
        );
        let readme = format!("# {name}\n\n{context}{}", family.guidance.readme);
        let guidance = format!(
            "# Project instructions\n\n{context}{}",
            family.guidance.agents
        );
        Ok([
            (family.manifest, manifest.as_str()),
            (source_path, source),
            ("README.md", readme.as_str()),
            ("AGENTS.md", guidance.as_str()),
            (
                "CLAUDE.md",
                include_str!("../../../templates/project-CLAUDE.md"),
            ),
            (".gitignore", GITIGNORE),
            ("tests/README.md", family.guidance.tests),
        ]
        .into_iter()
        .map(|(path, text)| (path.to_owned(), text.to_owned()))
        .collect())
    }

    pub fn profile(&self, family: &str, target: &str) -> Result<&'static Profile> {
        self.families
            .iter()
            .filter(|f| f.definition.id == family)
            .flat_map(|f| f.profiles)
            .find(|p| p.target.id == target)
            .ok_or_else(|| error("DSS1000", "Unsupported machine or operating mode"))
    }
}
