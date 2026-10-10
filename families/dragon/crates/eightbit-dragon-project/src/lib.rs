//! Strict loading and validation for Eightbit Studio 6809 folder projects.

#![cfg_attr(test, allow(clippy::expect_used, clippy::panic, clippy::unwrap_used))]

pub mod scaffold;

use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Read;
use std::ops::Range;
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

/// Largest accepted project configuration file.
pub const MAX_CONFIG_BYTES: u64 = 1024 * 1024;

/// A validated project and its canonical filesystem locations.
#[derive(Debug, Clone)]
pub struct LoadedProject {
    /// Canonical project root.
    pub root: PathBuf,
    /// Canonical BASIC entry file.
    pub entry_path: PathBuf,
    /// Canonical BASIC sources in explicit project order, including the entry.
    pub basic_source_paths: Vec<PathBuf>,
    /// Canonical assembly sources in configured order.
    pub assembler_source_paths: Vec<PathBuf>,
    /// Parsed configuration.
    pub config: ProjectConfig,
    /// Exact configuration bytes used by deterministic builds.
    pub config_source: String,
}

/// Versioned `dragon.toml` configuration.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    /// Project schema version. Versions 1 through 5 are supported.
    pub schema_version: u32,
    /// Project identity.
    pub project: ProjectSection,
    /// Target machine.
    pub machine: MachineSection,
    /// BASIC input settings.
    pub basic: BasicSection,
    /// Build output settings.
    pub build: BuildSection,
    /// Future run settings, validated but not executed in I1.
    pub run: RunSection,
    /// Future debug settings, validated but not executed in I1.
    pub debug: DebugSection,
    /// Assembly settings. Assembly must be disabled in I1.
    pub assembler: AssemblerSection,
}

impl ProjectConfig {
    /// Native BASIC mode is versioned separately from the compatibility contract.
    #[must_use]
    pub fn native_basic(&self) -> bool {
        self.schema_version == 5
            && self.machine.model == "dragon64"
            && self.machine.mode.as_deref() == Some("basic-64k")
    }
    /// Highest address available below the selected BASIC interpreter.
    #[must_use]
    pub fn ram_end(&self) -> u16 {
        if self.native_basic() { 0xbfff } else { 0x7fff }
    }
    /// Memory identity used by source maps; legacy values are preserved.
    #[must_use]
    pub fn address_space(&self) -> &'static str {
        if self.native_basic() {
            "dragon64-native-ram"
        } else {
            "dragon32-ram"
        }
    }
}

/// Project identity settings.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectSection {
    /// Portable project name.
    pub name: String,
}

/// Dragon hardware settings.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MachineSection {
    /// Machine identifier.
    pub model: String,
    /// RAM in KiB.
    pub ram_kib: u32,
    /// Television standard.
    pub tv: String,
    /// Explicit operating mode for schemas 4–5; omitted by legacy Dragon 32 projects.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    /// Requested ROM profile label. I1 records but does not resolve it.
    pub rom_profile: String,
}

/// Dragon BASIC source settings.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BasicSection {
    /// BASIC dialect identifier.
    pub dialect: String,
    /// Project-relative entry listing.
    pub entry: PathBuf,
    /// Explicit BASIC sources for schema 3, including the entry exactly once.
    #[serde(default)]
    pub sources: Vec<PathBuf>,
    /// First line executed by the cassette autorun path for schema 3.
    #[serde(default)]
    pub entry_line: Option<u16>,
    /// Source encoding.
    pub source_encoding: String,
    /// Line-number policy.
    pub line_numbers: String,
}

/// Build settings.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BuildSection {
    /// Portable profile name.
    pub profile: String,
    /// Project-relative generated-output directory.
    pub output_dir: PathBuf,
    /// BASIC media type.
    pub basic_media: String,
}

/// Run settings reserved for I3.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RunSection {
    /// Run adapter identifier.
    pub adapter: String,
    /// Whether run should autorun the artefact.
    pub autorun: bool,
    /// Runtime speed policy.
    pub speed: String,
    /// Explicit native cassette boot; BASIC remains the compatible default.
    #[serde(default = "default_run_boot")]
    pub boot: String,
}

fn default_run_boot() -> String {
    "basic".to_owned()
}

/// Debug settings reserved for I4.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DebugSection {
    /// Debug mode.
    pub mode: String,
    /// Whether a later runtime adapter should stop on interpreter errors.
    pub break_on_runtime_error: bool,
}

/// Assembly settings reserved for I6.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AssemblerSection {
    /// Assembly enablement.
    pub enabled: bool,
    /// Explicit project-relative assembly sources; wildcards are not accepted.
    pub sources: Vec<PathBuf>,
    /// I6.1 output format. Absent for disabled/schema-1 projects.
    #[serde(default)]
    pub output_format: Option<String>,
    /// Portable output filename. Absent for disabled/schema-1 projects.
    #[serde(default)]
    pub output_name: Option<String>,
    /// Base address assigned to the `code` section.
    #[serde(default)]
    pub load_address: Option<u16>,
    /// Public entry symbol expected in the linked map.
    #[serde(default)]
    pub entry_symbol: Option<String>,
}

/// A stable project-validation failure.
#[derive(Debug, Clone, Error, Serialize)]
#[error("{code}: {message}")]
pub struct ProjectError {
    /// Stable error code.
    pub code: &'static str,
    /// Actionable message.
    pub message: String,
    /// Configuration byte span when TOML supplied one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<Range<usize>>,
}

impl ProjectError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            span: None,
        }
    }
}

/// Load and strictly validate a Eightbit Studio 6809 project directory.
///
/// This performs no tool execution and writes no files.
pub fn load_project(root: impl AsRef<Path>) -> Result<LoadedProject, ProjectError> {
    let requested_root = root.as_ref();
    let root = fs::canonicalize(requested_root).map_err(|error| {
        ProjectError::new(
            "DSP1000",
            format!(
                "cannot open project directory '{}': {error}",
                requested_root.display()
            ),
        )
    })?;
    if !root.is_dir() {
        return Err(ProjectError::new(
            "DSP1000",
            format!("project root '{}' is not a directory", root.display()),
        ));
    }

    let config_path = root.join("dragon.toml");
    let invalid = || {
        ProjectError::new(
            "DSP1001",
            "dragon.toml must be a readable, regular non-symlink UTF-8 file",
        )
    };
    let oversized = || {
        ProjectError::new(
            "DSP1002",
            format!("dragon.toml exceeds the {MAX_CONFIG_BYTES}-byte limit"),
        )
    };
    let before = fs::symlink_metadata(&config_path).map_err(|_| invalid())?;
    if !before.is_file() {
        return Err(invalid());
    }
    if before.len() > MAX_CONFIG_BYTES {
        return Err(oversized());
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(0x2000_0000 | 0x0000_0004); // O_NOFOLLOW_ANY | O_NONBLOCK
    }
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(0x0002_0000 | 0x0000_0800); // O_NOFOLLOW | O_NONBLOCK
    }
    let file = options.open(&config_path).map_err(|_| invalid())?;
    let opened = file.metadata().map_err(|_| invalid())?;
    if !opened.is_file() {
        return Err(invalid());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if opened.dev() != before.dev() || opened.ino() != before.ino() {
            return Err(invalid());
        }
    }
    if opened.len() > MAX_CONFIG_BYTES {
        return Err(oversized());
    }
    let mut config_source = String::new();
    file.take(MAX_CONFIG_BYTES + 1)
        .read_to_string(&mut config_source)
        .map_err(|_| invalid())?;
    if config_source.len() as u64 > MAX_CONFIG_BYTES {
        return Err(oversized());
    }
    load_project_from_source(&root, &config_source)
}

/// Validate an in-memory `dragon.toml` candidate against an existing project root.
///
/// This performs no tool execution and writes no files. Desktop clients use it
/// before atomically replacing a manifest so an invalid candidate cannot make
/// the project impossible to reopen.
pub fn load_project_from_source(
    root: impl AsRef<Path>,
    config_source: &str,
) -> Result<LoadedProject, ProjectError> {
    let requested_root = root.as_ref();
    let root = fs::canonicalize(requested_root).map_err(|error| {
        ProjectError::new(
            "DSP1000",
            format!(
                "cannot open project directory '{}': {error}",
                requested_root.display()
            ),
        )
    })?;
    if !root.is_dir() {
        return Err(ProjectError::new(
            "DSP1000",
            format!("project root '{}' is not a directory", root.display()),
        ));
    }
    if config_source.len() as u64 > MAX_CONFIG_BYTES {
        return Err(ProjectError::new(
            "DSP1002",
            format!("dragon.toml exceeds the {MAX_CONFIG_BYTES}-byte limit"),
        ));
    }
    let config = parse_project_config(config_source)?;

    let entry_path = fs::canonicalize(root.join(&config.basic.entry)).map_err(|error| {
        ProjectError::new(
            "DSP1004",
            format!(
                "BASIC entry '{}' cannot be read: {error}",
                config.basic.entry.display()
            ),
        )
    })?;
    if !entry_path.starts_with(&root) {
        return Err(ProjectError::new(
            "DSP1005",
            "basic.entry resolves outside the project root",
        ));
    }
    if !entry_path.is_file() {
        return Err(ProjectError::new(
            "DSP1004",
            format!(
                "BASIC entry '{}' is not a file",
                config.basic.entry.display()
            ),
        ));
    }

    let basic_source_paths = load_basic_sources(&root, &config.basic, config.schema_version)?;
    let assembler_source_paths = load_assembler_sources(&root, &config.assembler)?;

    Ok(LoadedProject {
        root,
        entry_path,
        basic_source_paths,
        assembler_source_paths,
        config,
        config_source: config_source.to_owned(),
    })
}

/// Parse and validate the portable manifest contract without filesystem authority.
/// File existence and canonical root containment remain the loader's responsibility.
pub fn parse_project_config(source: &str) -> Result<ProjectConfig, ProjectError> {
    if source.len() as u64 > MAX_CONFIG_BYTES {
        return Err(ProjectError::new(
            "DSP1002",
            "dragon.toml exceeds its size limit",
        ));
    }
    let config: ProjectConfig = toml::from_str(source).map_err(|error| ProjectError {
        code: "DSP1003",
        message: format!("invalid dragon.toml: {error}"),
        span: error.span(),
    })?;
    validate_config(&config)?;
    validate_relative_path(&config.basic.entry, "basic.entry")?;
    validate_relative_path(&config.build.output_dir, "build.output_dir")?;
    for path in &config.basic.sources {
        validate_relative_path(path, "basic.sources")?;
    }
    for path in &config.assembler.sources {
        validate_relative_path(path, "assembler.sources")?;
    }
    Ok(config)
}

fn validate_config(config: &ProjectConfig) -> Result<(), ProjectError> {
    if !matches!(config.schema_version, 1..=5) {
        return Err(ProjectError::new(
            "DSP1006",
            format!(
                "unsupported schema_version {}; supported versions are 1, 2, 3, 4 and 5",
                config.schema_version
            ),
        ));
    }
    if !is_portable_name(&config.project.name, 64) {
        return Err(ProjectError::new(
            "DSP1007",
            "project.name must be 1–64 ASCII letters, digits, underscores or hyphens and start with a letter or digit",
        ));
    }
    let target = eightbit_dragon_targets::resolve_mode(
        &config.machine.model,
        config.machine.ram_kib,
        &config.machine.tv,
        config.machine.mode.as_deref(),
    )
    .map_err(|_| {
        ProjectError::new(
            "DSP1008",
            "Supported machines are PAL Dragon 32 (32 KiB) and schema-4/5 Dragon 64 (64 KiB, explicit BASIC mode)",
        )
    })?;
    if (config.schema_version < 4
        && (config.machine.model != "dragon32" || config.machine.mode.is_some()))
        || (config.schema_version == 4
            && (config.machine.model != "dragon64"
                || config.machine.mode.as_deref() != Some("basic-32k")))
        || (config.schema_version == 5
            && (config.machine.model != "dragon64"
                || config.machine.mode.as_deref() != Some("basic-64k")))
    {
        return Err(ProjectError::new(
            "DSP1008",
            "Schemas 1–3 require Dragon 32 without a mode; schema 4 requires Dragon 64 mode='basic-32k'; schema 5 requires mode='basic-64k'",
        ));
    }
    if config.native_basic() && config.run.boot == "cartridge" {
        return Err(ProjectError::new(
            "DSP1022",
            "Native Dragon 64 BASIC occupies cartridge memory; use 32K compatibility mode for cartridge projects",
        ));
    }
    if config.machine.rom_profile.trim().is_empty() {
        return Err(ProjectError::new(
            "DSP1008",
            "machine.rom_profile cannot be empty",
        ));
    }
    if Some(config.basic.dialect.as_str()) != target.dialect
        || config.basic.source_encoding != "ascii"
        || config.basic.line_numbers != "required"
    {
        return Err(ProjectError::new(
            "DSP1009",
            "BASIC dialect must match the selected target; source_encoding='ascii' and line_numbers='required' are required",
        ));
    }
    if config.schema_version < 3
        && (!config.basic.sources.is_empty() || config.basic.entry_line.is_some())
    {
        return Err(ProjectError::new(
            "DSP1017",
            "basic.sources and basic.entry_line require schema_version=3",
        ));
    }
    if config.schema_version >= 3 {
        if config.basic.sources.is_empty() || config.basic.sources.len() > 64 {
            return Err(ProjectError::new(
                "DSP1017",
                "schema 3 requires 1–64 explicit basic.sources",
            ));
        }
        if config
            .basic
            .sources
            .iter()
            .filter(|source| *source == &config.basic.entry)
            .count()
            != 1
        {
            return Err(ProjectError::new(
                "DSP1017",
                "schema 3 basic.sources must contain basic.entry exactly once",
            ));
        }
        if !matches!(config.basic.entry_line, Some(1..=63_999)) {
            return Err(ProjectError::new(
                "DSP1017",
                "schema 3 basic.entry_line must be between 1 and 63999",
            ));
        }
    }
    if !is_portable_name(&config.build.profile, 32) {
        return Err(ProjectError::new(
            "DSP1010",
            "build.profile must be 1–32 portable name characters",
        ));
    }
    if config.build.basic_media != "cas" {
        return Err(ProjectError::new(
            "DSP1011",
            "I1 supports only build.basic_media='cas'",
        ));
    }
    if config.run.adapter != "xroar" || config.run.speed != "normal" {
        return Err(ProjectError::new(
            "DSP1012",
            "I1 recognises only run.adapter='xroar' and run.speed='normal'",
        ));
    }
    if !matches!(
        config.run.boot.as_str(),
        "basic" | "machine-code" | "cartridge"
    ) || (matches!(config.run.boot.as_str(), "machine-code" | "cartridge")
        && (config.schema_version < 3 || !config.assembler.enabled))
    {
        return Err(ProjectError::new(
            "DSP1020",
            "run.boot must be basic, or machine-code/cartridge in a schema-3 assembly-enabled project",
        ));
    }
    let cartridge = config.assembler.output_format.as_deref() == Some("cartridge");
    if cartridge != (config.run.boot == "cartridge") || (cartridge && config.schema_version < 3) {
        return Err(ProjectError::new(
            "DSP1021",
            "cartridge output requires a schema-3 project with run.boot='cartridge'",
        ));
    }
    if config.debug.mode != "basic" {
        return Err(ProjectError::new(
            "DSP1013",
            "I1 recognises only debug.mode='basic'",
        ));
    }
    if config.schema_version == 1
        && (config.assembler.enabled
            || !config.assembler.sources.is_empty()
            || config.assembler.output_format.is_some()
            || config.assembler.output_name.is_some()
            || config.assembler.load_address.is_some()
            || config.assembler.entry_symbol.is_some())
    {
        return Err(ProjectError::new(
            "DSP1014",
            "schema 1 requires disabled assembly with no assembly build fields",
        ));
    }
    validate_assembler(&config.assembler, config.ram_end())?;
    Ok(())
}

fn load_basic_sources(
    root: &Path,
    basic: &BasicSection,
    schema_version: u32,
) -> Result<Vec<PathBuf>, ProjectError> {
    let configured = if schema_version >= 3 {
        basic.sources.as_slice()
    } else {
        std::slice::from_ref(&basic.entry)
    };
    let mut sources = Vec::with_capacity(configured.len());
    for source in configured {
        validate_relative_path(source, "basic.sources")?;
        if schema_version >= 3
            && !source
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("bas"))
        {
            return Err(ProjectError::new(
                "DSP1017",
                format!("BASIC source '{}' must end in .BAS", source.display()),
            ));
        }
        let canonical = fs::canonicalize(root.join(source)).map_err(|error| {
            ProjectError::new(
                "DSP1017",
                format!(
                    "BASIC source '{}' cannot be read: {error}",
                    source.display()
                ),
            )
        })?;
        if !canonical.starts_with(root) || !canonical.is_file() {
            return Err(ProjectError::new(
                "DSP1017",
                format!("BASIC source '{}' is not a project file", source.display()),
            ));
        }
        if sources.contains(&canonical) {
            return Err(ProjectError::new(
                "DSP1017",
                format!("BASIC source '{}' is duplicated", source.display()),
            ));
        }
        sources.push(canonical);
    }
    Ok(sources)
}

fn validate_assembler(assembler: &AssemblerSection, ram_end: u16) -> Result<(), ProjectError> {
    if !assembler.enabled {
        if !assembler.sources.is_empty()
            || assembler.output_format.is_some()
            || assembler.output_name.is_some()
            || assembler.load_address.is_some()
            || assembler.entry_symbol.is_some()
        {
            return Err(ProjectError::new(
                "DSP1014",
                "disabled assembly must not define sources or assembly build fields",
            ));
        }
        return Ok(());
    }
    if assembler.sources.is_empty() || assembler.sources.len() > 64 {
        return Err(ProjectError::new(
            "DSP1014",
            "enabled assembly requires 1–64 explicit source paths",
        ));
    }
    if !matches!(
        assembler.output_format.as_deref(),
        Some("raw" | "cartridge")
    ) {
        return Err(ProjectError::new(
            "DSP1014",
            "assembler.output_format must be raw or cartridge",
        ));
    }
    let Some(output_name) = assembler.output_name.as_deref() else {
        return Err(ProjectError::new(
            "DSP1014",
            "enabled assembly requires assembler.output_name",
        ));
    };
    if !portable_output_name(output_name) {
        return Err(ProjectError::new(
            "DSP1014",
            "assembler.output_name must be a portable .bin filename without a directory",
        ));
    }
    let cartridge = assembler.output_format.as_deref() == Some("cartridge");
    if (cartridge && assembler.load_address != Some(0xc000))
        || (!cartridge
            && !matches!(assembler.load_address, Some(address) if (0x0400..=ram_end).contains(&address)))
    {
        return Err(ProjectError::new(
            "DSP1014",
            format!(
                "assembler.load_address must be $0400–${ram_end:04X} for the selected RAM mode, or $C000 for cartridge output"
            ),
        ));
    }
    let Some(entry_symbol) = assembler.entry_symbol.as_deref() else {
        return Err(ProjectError::new(
            "DSP1014",
            "enabled assembly requires assembler.entry_symbol",
        ));
    };
    if !assembly_symbol(entry_symbol) {
        return Err(ProjectError::new(
            "DSP1014",
            "assembler.entry_symbol is not a portable 6809 symbol",
        ));
    }
    Ok(())
}

fn load_assembler_sources(
    root: &Path,
    assembler: &AssemblerSection,
) -> Result<Vec<PathBuf>, ProjectError> {
    let mut sources = Vec::with_capacity(assembler.sources.len());
    for source in &assembler.sources {
        validate_relative_path(source, "assembler.sources")?;
        if !matches!(
            source.extension().and_then(|extension| extension.to_str()),
            Some("asm" | "s")
        ) {
            return Err(ProjectError::new(
                "DSP1014",
                format!(
                    "assembly source '{}' must end in .asm or .s",
                    source.display()
                ),
            ));
        }
        let canonical = fs::canonicalize(root.join(source)).map_err(|error| {
            ProjectError::new(
                "DSP1014",
                format!(
                    "assembly source '{}' cannot be read: {error}",
                    source.display()
                ),
            )
        })?;
        if !canonical.starts_with(root) || !canonical.is_file() {
            return Err(ProjectError::new(
                "DSP1014",
                format!(
                    "assembly source '{}' is not a project file",
                    source.display()
                ),
            ));
        }
        if sources.contains(&canonical) {
            return Err(ProjectError::new(
                "DSP1014",
                format!("assembly source '{}' is duplicated", source.display()),
            ));
        }
        sources.push(canonical);
    }
    Ok(sources)
}

fn portable_output_name(value: &str) -> bool {
    let path = Path::new(value);
    let starts_portably = value
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_alphanumeric());
    path.file_name().and_then(|name| name.to_str()) == Some(value)
        && path.extension().and_then(|extension| extension.to_str()) == Some("bin")
        && value.len() <= 64
        && starts_portably
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
}

fn assembly_symbol(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    value.len() <= 64
        && (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '$' | '@' | '?')
        })
}

fn is_portable_name(value: &str, max_len: usize) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    value.len() <= max_len
        && first.is_ascii_alphanumeric()
        && chars
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
}

fn validate_relative_path(path: &Path, field: &str) -> Result<(), ProjectError> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(ProjectError::new(
            "DSP1015",
            format!("{field} must be a non-empty project-relative path"),
        ));
    }
    for component in path.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(ProjectError::new(
                "DSP1015",
                format!("{field} must not contain '.', '..', a root or a platform prefix"),
            ));
        }
    }
    Ok(())
}

/// Render a path relative to a project root using portable `/` separators.
pub fn portable_relative_path(root: &Path, path: &Path) -> Result<String, ProjectError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| ProjectError::new("DSP1016", "path is not contained by the project root"))?;
    Ok(relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/"))
}

impl fmt::Display for ProjectConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} (schema {})",
            self.project.name, self.schema_version
        )
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn manifest_read_rejects_special_files_and_bounds_regular_utf8() {
        let (_directory, root) = project(VALID);
        let path = root.join("dragon.toml");
        let mut maximum = VALID.to_owned();
        maximum.push('\n');
        maximum.push_str(&" ".repeat(usize::try_from(MAX_CONFIG_BYTES).unwrap() - maximum.len()));
        fs::write(&path, &maximum).unwrap();
        assert!(load_project(&root).is_ok());
        fs::write(&path, format!("{maximum} ")).unwrap();
        assert_eq!(load_project(&root).unwrap_err().code, "DSP1002");
        fs::write(&path, [0xff]).unwrap();
        assert_eq!(load_project(&root).unwrap_err().code, "DSP1001");
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        assert_eq!(load_project(&root).unwrap_err().code, "DSP1001");
        fs::remove_dir(&path).unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.join("src/MAIN.BAS"), &path).unwrap();
            assert_eq!(load_project(&root).unwrap_err().code, "DSP1001");
            fs::remove_file(&path).unwrap();
            assert!(
                std::process::Command::new("/usr/bin/mkfifo")
                    .arg(&path)
                    .status()
                    .unwrap()
                    .success()
            );
            assert_eq!(load_project(&root).unwrap_err().code, "DSP1001");
        }
    }

    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    const VALID: &str = r#"
schema_version = 1
[project]
name = "test-project"
[machine]
model = "dragon32"
ram_kib = 32
tv = "pal"
rom_profile = "dragon32-standard"
[basic]
dialect = "dragon32-basic"
entry = "src/MAIN.BAS"
source_encoding = "ascii"
line_numbers = "required"
[build]
profile = "debug"
output_dir = ".dragon/build"
basic_media = "cas"
[run]
adapter = "xroar"
autorun = true
speed = "normal"
[debug]
mode = "basic"
break_on_runtime_error = false
[assembler]
enabled = false
sources = []
"#;

    fn project(config: &str) -> (TempDir, PathBuf) {
        let directory = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
        fs::create_dir(directory.path().join("src"))
            .unwrap_or_else(|error| panic!("create src: {error}"));
        fs::write(directory.path().join("dragon.toml"), config)
            .unwrap_or_else(|error| panic!("write config: {error}"));
        let mut source = fs::File::create(directory.path().join("src/MAIN.BAS"))
            .unwrap_or_else(|error| panic!("create source: {error}"));
        writeln!(source, "10 PRINT \"OK\"").unwrap_or_else(|error| panic!("write source: {error}"));
        let root = directory.path().to_path_buf();
        (directory, root)
    }

    #[test]
    fn native_dragon64_requires_matching_version_mode_and_dialect() {
        let valid = VALID
            .replace("schema_version = 1", "schema_version = 5")
            .replace(
                "model = \"dragon32\"",
                "model = \"dragon64\"\nmode = \"basic-64k\"",
            )
            .replace("ram_kib = 32", "ram_kib = 64")
            .replace("dragon32-basic", "dragon64-basic")
            .replace(
                "entry = \"src/MAIN.BAS\"",
                "entry = \"src/MAIN.BAS\"\nsources = [\"src/MAIN.BAS\"]\nentry_line = 10",
            );
        let (_directory, root) = project(&valid);
        let config = load_project(&root).unwrap().config;
        assert!(config.native_basic());
        assert_eq!(config.ram_end(), 0xbfff);
        for invalid in [
            valid.replace("schema_version = 5", "schema_version = 4"),
            valid.replace("basic-64k", "basic-32k"),
            valid.replace("dragon64-basic", "dragon32-basic"),
            valid.replace("ram_kib = 64", "ram_kib = 32"),
        ] {
            fs::write(root.join("dragon.toml"), invalid).unwrap();
            assert!(load_project(&root).is_err());
        }
    }

    #[test]
    fn dragon64_requires_explicit_schema_four_compatibility_mode() {
        let valid = VALID
            .replace("schema_version = 1", "schema_version = 4")
            .replace(
                "model = \"dragon32\"",
                "model = \"dragon64\"\nmode = \"basic-32k\"",
            )
            .replace("ram_kib = 32", "ram_kib = 64")
            .replace(
                "entry = \"src/MAIN.BAS\"",
                "entry = \"src/MAIN.BAS\"\nsources = [\"src/MAIN.BAS\"]\nentry_line = 10",
            );
        let (_directory, root) = project(&valid);
        assert_eq!(
            load_project(root).unwrap().config.machine.mode.as_deref(),
            Some("basic-32k")
        );
        for invalid in [
            valid.replace("schema_version = 4", "schema_version = 3"),
            valid.replace("mode = \"basic-32k\"", ""),
            valid.replace("basic-32k", "basic-64k"),
            valid.replace("ram_kib = 64", "ram_kib = 32"),
            valid.replace("tv = \"pal\"", "tv = \"ntsc\""),
            valid.replace("sources = [\"src/MAIN.BAS\"]", "sources = []"),
            valid.replace(
                "speed = \"normal\"",
                "speed = \"normal\"\nboot = \"cartridge\"",
            ),
        ] {
            let (_directory, root) = project(&invalid);
            assert!(load_project(root).is_err());
        }
        let (_directory, root) = project(VALID);
        let legacy = load_project(root).unwrap();
        assert!(legacy.config.machine.mode.is_none());
        assert!(
            !toml::to_string(&legacy.config.machine)
                .unwrap()
                .contains("mode =")
        );
    }

    #[test]
    fn loads_valid_project() {
        let (_directory, root) = project(VALID);
        let loaded = load_project(root).unwrap_or_else(|error| panic!("load: {error}"));
        assert_eq!(loaded.config.project.name, "test-project");
        assert!(loaded.entry_path.ends_with("src/MAIN.BAS"));
    }

    #[test]
    fn rejects_unknown_fields_with_span() {
        let invalid = VALID.replace(
            "name = \"test-project\"",
            "name = \"test-project\"\ntyop = true",
        );
        let (_directory, root) = project(&invalid);
        let error = load_project(root).expect_err("unknown field must fail");
        assert_eq!(error.code, "DSP1003");
        assert!(error.span.is_some());
    }

    #[test]
    fn loads_schema_two_explicit_assembly_sources() {
        let assembly = VALID
            .replace("schema_version = 1", "schema_version = 2")
            .replace(
                "enabled = false\nsources = []",
                "enabled = true\nsources = [\"asm/MAIN.asm\", \"asm/HELPER.s\"]\noutput_format = \"raw\"\noutput_name = \"machine.bin\"\nload_address = 28672\nentry_symbol = \"START\"",
            );
        let (directory, root) = project(&assembly);
        fs::create_dir(directory.path().join("asm")).unwrap_or_else(|error| panic!("asm: {error}"));
        fs::write(
            directory.path().join("asm/MAIN.asm"),
            " section code\nSTART rts\n",
        )
        .unwrap_or_else(|error| panic!("main: {error}"));
        fs::write(
            directory.path().join("asm/HELPER.s"),
            " section code\nHELPER rts\n",
        )
        .unwrap_or_else(|error| panic!("helper: {error}"));

        let loaded = load_project(root).unwrap_or_else(|error| panic!("load: {error}"));
        assert_eq!(loaded.assembler_source_paths.len(), 2);
        assert_eq!(loaded.config.assembler.load_address, Some(0x7000));
    }

    #[test]
    fn loads_schema_three_explicit_basic_sources() {
        let multi = VALID
            .replace("schema_version = 1", "schema_version = 3")
            .replace(
                "entry = \"src/MAIN.BAS\"",
                "entry = \"src/MAIN.BAS\"\nsources = [\"src/MAIN.BAS\", \"src/SCREEN.BAS\"]\nentry_line = 10",
            );
        let (directory, root) = project(&multi);
        fs::write(directory.path().join("src/SCREEN.BAS"), "1000 RETURN\n")
            .unwrap_or_else(|error| panic!("screen: {error}"));

        let loaded = load_project(root).unwrap_or_else(|error| panic!("load: {error}"));
        assert_eq!(loaded.basic_source_paths.len(), 2);
        assert!(loaded.basic_source_paths[0].ends_with("src/MAIN.BAS"));
        assert!(loaded.basic_source_paths[1].ends_with("src/SCREEN.BAS"));
        assert_eq!(loaded.config.basic.entry_line, Some(10));
    }

    #[test]
    fn rejects_schema_three_without_entry_in_sources() {
        let multi = VALID
            .replace("schema_version = 1", "schema_version = 3")
            .replace(
                "entry = \"src/MAIN.BAS\"",
                "entry = \"src/MAIN.BAS\"\nsources = [\"src/SCREEN.BAS\"]\nentry_line = 10",
            );
        let (directory, root) = project(&multi);
        fs::write(directory.path().join("src/SCREEN.BAS"), "1000 RETURN\n")
            .unwrap_or_else(|error| panic!("screen: {error}"));

        let error = load_project(root).expect_err("entry membership must fail");
        assert_eq!(error.code, "DSP1017");
    }

    #[test]
    fn native_boot_is_explicit_and_requires_schema_three_assembly() {
        let (_directory, root) = project(VALID);
        assert_eq!(
            load_project(root)
                .unwrap_or_else(|error| panic!("load: {error}"))
                .config
                .run
                .boot,
            "basic"
        );
        for boot in ["machine-code", "shell", ""] {
            let config = VALID.replace(
                "speed = \"normal\"",
                &format!("speed = \"normal\"\nboot = {boot:?}"),
            );
            let (_directory, root) = project(&config);
            assert_eq!(
                load_project(root).expect_err("invalid boot must fail").code,
                "DSP1020"
            );
        }
    }

    #[test]
    fn rejects_parent_path() {
        let invalid = VALID.replace("src/MAIN.BAS", "../MAIN.BAS");
        let (_directory, root) = project(&invalid);
        let error = load_project(root).expect_err("parent path must fail");
        assert_eq!(error.code, "DSP1015");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_entry_symlink_outside_root() {
        use std::os::unix::fs::symlink;

        let (directory, root) = project(VALID);
        let outside = TempDir::new().unwrap_or_else(|error| panic!("outside tempdir: {error}"));
        fs::write(outside.path().join("OUT.BAS"), "10 END\n")
            .unwrap_or_else(|error| panic!("write outside: {error}"));
        fs::remove_file(directory.path().join("src/MAIN.BAS"))
            .unwrap_or_else(|error| panic!("remove fixture: {error}"));
        symlink(
            outside.path().join("OUT.BAS"),
            directory.path().join("src/MAIN.BAS"),
        )
        .unwrap_or_else(|error| panic!("symlink: {error}"));

        let error = load_project(root).expect_err("escaping symlink must fail");
        assert_eq!(error.code, "DSP1005");
    }
    #[test]
    fn cartridge_profile_requires_schema_three_fixed_mapping_and_matching_boot() {
        let config = VALID.replace("schema_version = 1", "schema_version = 3")
        .replace("entry = \"src/MAIN.BAS\"", "entry = \"src/MAIN.BAS\"\nsources = [\"src/MAIN.BAS\"]\nentry_line = 10")
            .replace("speed = \"normal\"", "speed = \"normal\"\nboot = \"cartridge\"")
            .replace("enabled = false\nsources = []", "enabled = true\nsources = [\"asm/MAIN.asm\"]\noutput_format = \"cartridge\"\noutput_name = \"machine.bin\"\nload_address = 49152\nentry_symbol = \"START\"");
        let (directory, root) = project(&config);
        fs::create_dir(directory.path().join("asm")).unwrap();
        fs::write(
            directory.path().join("asm/MAIN.asm"),
            " section code\nSTART rts\n endsection\n",
        )
        .unwrap();
        assert!(load_project(&root).is_ok());
        for invalid in [
            config.replace("schema_version = 3", "schema_version = 2"),
            config.replace("load_address = 49152", "load_address = 28672"),
            config.replace("boot = \"cartridge\"", "boot = \"basic\""),
        ] {
            fs::write(directory.path().join("dragon.toml"), invalid).unwrap();
            assert!(load_project(&root).is_err());
        }
    }
}
