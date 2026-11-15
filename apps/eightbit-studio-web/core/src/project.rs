//! Browser snapshots wrap, rather than replace, the desktop project manifest.
use eightbit_dragon_project::{parse_project_config, scaffold};
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read, Write};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

const LIMIT: usize = 1024 * 1024;
const MAX_PROJECT_FILES: usize = 128;
pub(crate) fn is_dragon(target: &str) -> bool {
    matches!(target, "dragon32" | "dragon64" | "dragon64-native")
}
fn dragon_target(config: &eightbit_dragon_project::ProjectConfig) -> &'static str {
    if config.machine.model == "dragon64" {
        if config.native_basic() {
            "dragon64-native"
        } else {
            "dragon64"
        }
    } else {
        "dragon32"
    }
}
fn default_target() -> String {
    "dragon32".into()
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Snapshot {
    pub schema_version: u32,
    #[serde(default = "default_target")]
    pub target: String,
    pub name: String,
    pub kind: Kind,
    pub source: String,
    pub manifest: Option<String>,
    #[serde(default)]
    pub support_files: std::collections::BTreeMap<String, String>,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Kind {
    Basic,
    Assembly,
}

fn support_path(name: &str) -> bool {
    [
        "GUIDE.html",
        "AGENTS.md",
        "CLAUDE.md",
        "README.md",
        "tests/README.md",
        ".gitignore",
    ]
    .contains(&name)
        || name.strip_prefix("tests/").is_some_and(|file| {
            file.ends_with(".dragon-test.toml")
                && file.len() < 128
                && file
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        })
}
fn slug(name: &str) -> Result<String, String> {
    if name.is_empty()
        || name.len() > 64
        || !name.as_bytes()[0].is_ascii_alphanumeric()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b" _-".contains(&byte))
    {
        return Err("DSW1010: use 1–64 letters, numbers, spaces, underscores or hyphens; start with a letter or number".into());
    }
    Ok(name.replace(' ', "-"))
}
#[cfg(test)]
pub(crate) fn create(name: String, kind: Kind, source: String) -> Result<Snapshot, String> {
    create_for(name, kind, source, default_target())
}
pub(crate) fn create_for(
    name: String,
    kind: Kind,
    source: String,
    target: String,
) -> Result<Snapshot, String> {
    let mut project = if target == "vectrex" {
        if kind != Kind::Assembly {
            return Err("DSW1012: Vectrex requires assembly".into());
        }
        slug(&name)?;
        let m = eightbit_vectrex_emulator::cartridge::Manifest {
            schema_version: 1,
            name: name.clone(),
            target: target.clone(),
            source: "src/asm/main.asm".into(),
        };
        Snapshot {
            schema_version: 2,
            target: target.clone(),
            name,
            kind,
            source,
            manifest: Some(
                toml::to_string(&m).map_err(|_| "DSW1010: cannot encode Vectrex manifest")?,
            ),
            support_files: eightbit_vectrex_emulator::cartridge::SUPPORT
                .iter()
                .map(|(p, t)| (p.to_string(), t.to_string()))
                .collect(),
        }
    } else if is_dragon(&target) {
        if target != "dragon32" && kind != Kind::Basic {
            return Err(
                "DSW1012: Dragon 64 browser projects currently require BASIC source".into(),
            );
        }
        let mut project = create_dragon(name, kind, source)?;
        if target != "dragon32" {
            project.schema_version = 2;
            project.manifest = Some(
                scaffold::dragon64_manifest(&slug(&project.name)?, target == "dragon64-native")
                    .map_err(|e| e.to_string())?,
            );
        }
        project
    } else {
        if kind != Kind::Basic && eightbit_spectrum_core::original_target(&target).is_none() {
            return Err("DSW1012: browser assembly projects require the desktop edition".into());
        }
        let portable = slug(&name)?.to_ascii_lowercase().replace('_', "-");
        let engine = crate::machines::engine(&target)?;
        let family = engine.families[0];
        let mut files = engine
            .scaffold(
                &portable,
                family.definition.id,
                &target,
                if kind == Kind::Basic {
                    "basic"
                } else {
                    "assembly"
                },
            )
            .map_err(|e| e.to_string())?;
        let manifest = files
            .remove(family.manifest)
            .ok_or("DSW1014: missing generated manifest")?;
        files.remove(if kind == Kind::Basic {
            "src/basic/main.bas"
        } else {
            "src/asm/main.asm"
        });
        Snapshot {
            schema_version: 2,
            target: target.clone(),
            name,
            kind,
            source,
            manifest: Some(manifest),
            support_files: files,
        }
    };
    project.target = target;
    validate(&project)?;
    Ok(project)
}
fn create_dragon(name: String, kind: Kind, source: String) -> Result<Snapshot, String> {
    let portable = slug(&name)?;
    let manifest = Some(if kind == Kind::Basic {
        scaffold::manifest(&portable, false).map_err(|error| error.to_string())?
    } else {
        assembly_manifest(&portable, &AssemblyOptions::default())?
    });
    let mut support_files = if kind == Kind::Basic {
        [
            ("AGENTS.md", scaffold::AGENTS),
            ("CLAUDE.md", scaffold::CLAUDE),
            ("README.md", scaffold::README),
            ("tests/README.md", scaffold::TESTS_README),
            (".gitignore", scaffold::GITIGNORE),
        ]
        .into_iter()
        .map(|(name, content)| (name.into(), content.into()))
        .collect()
    } else {
        std::collections::BTreeMap::new()
    };
    if kind == Kind::Assembly {
        support_files.insert("src/basic/main.bas".into(), "10 END\n".into());
    }
    let project = Snapshot {
        schema_version: 2,
        target: default_target(),
        name,
        kind,
        source,
        manifest,
        support_files,
    };
    validate(&project)?;
    Ok(project)
}
/// Source settings use the existing desktop manifest, not a browser-only schema.
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AssemblyOptions {
    pub load_address: u16,
    pub entry_symbol: String,
}
impl Default for AssemblyOptions {
    fn default() -> Self {
        Self {
            load_address: 0x7000,
            entry_symbol: "START".into(),
        }
    }
}
fn assembly_manifest(name: &str, options: &AssemblyOptions) -> Result<String, String> {
    let original = scaffold::manifest(name, true).map_err(|error| error.to_string())?;
    let mut value: toml::Value =
        toml::from_str(&original).map_err(|_| "DSW1011: invalid assembly scaffold")?;
    value
        .get_mut("run")
        .and_then(toml::Value::as_table_mut)
        .ok_or("DSW1011: missing run settings")?
        .insert("boot".into(), toml::Value::String("machine-code".into()));
    let assembler = value
        .get_mut("assembler")
        .and_then(toml::Value::as_table_mut)
        .ok_or("DSW1011: missing assembly settings")?;
    assembler.insert(
        "load_address".into(),
        toml::Value::Integer(i64::from(options.load_address)),
    );
    assembler.insert(
        "entry_symbol".into(),
        toml::Value::String(options.entry_symbol.clone()),
    );
    let manifest =
        toml::to_string(&value).map_err(|_| "DSW1011: cannot encode assembly settings")?;
    parse_project_config(&manifest).map_err(|error| error.to_string())?;
    Ok(manifest)
}
pub(crate) fn configure_assembly(
    project: &mut Snapshot,
    options: &AssemblyOptions,
) -> Result<(), String> {
    if project.kind != Kind::Assembly || project.target != "dragon32" {
        return Err("DSW1012: assembly settings require a Dragon 32 assembly project".into());
    }
    // Only change settings: an imported manifest owns its source order and paths.
    let mut candidate = project.clone();
    let manifest = project
        .manifest
        .clone()
        .unwrap_or(assembly_manifest(&slug(&project.name)?, options)?);
    let mut value: toml::Value =
        toml::from_str(&manifest).map_err(|_| "DSW1011: invalid assembly manifest")?;
    let assembler = value
        .get_mut("assembler")
        .and_then(toml::Value::as_table_mut)
        .ok_or("DSW1011: missing assembly settings")?;
    assembler.insert(
        "load_address".into(),
        toml::Value::Integer(i64::from(options.load_address)),
    );
    assembler.insert(
        "entry_symbol".into(),
        toml::Value::String(options.entry_symbol.clone()),
    );
    candidate.manifest =
        Some(toml::to_string(&value).map_err(|_| "DSW1011: cannot encode assembly settings")?);
    validate(&candidate)?;
    *project = candidate;
    Ok(())
}
pub(crate) fn assembly_options(project: &Snapshot) -> Result<AssemblyOptions, String> {
    validate(project)?;
    if project.kind != Kind::Assembly || project.target != "dragon32" {
        return Err("DSW1012: not a Dragon 32 assembly project".into());
    }
    let Some(manifest) = project.manifest.as_deref() else {
        return Ok(AssemblyOptions::default());
    };
    let config = parse_project_config(manifest).map_err(|error| error.to_string())?;
    Ok(AssemblyOptions {
        load_address: config
            .assembler
            .load_address
            .ok_or("DSW1012: assembly load address is missing")?,
        entry_symbol: config
            .assembler
            .entry_symbol
            .ok_or("DSW1012: assembly entry symbol is missing")?,
    })
}
/// Only explicit manifest paths are admitted as source; there is no filesystem lookup.
fn assembly_file_paths(project: &Snapshot) -> Result<(Vec<String>, Option<String>), String> {
    if project.kind != Kind::Assembly {
        return Ok((Vec::new(), None));
    }
    if project.target == "vectrex" {
        let m = eightbit_vectrex_emulator::cartridge::parse_manifest(
            project
                .manifest
                .as_deref()
                .ok_or("DSW1011: Vectrex manifest is missing")?,
        )
        .map_err(|e| e.to_string())?;
        return Ok((vec![m.source], None));
    }
    if eightbit_spectrum_core::original_target(&project.target).is_some() {
        let engine = crate::machines::engine(&project.target)?;
        let manifest = project
            .manifest
            .as_deref()
            .ok_or("DSW1011: assembly manifest is missing")?;
        let (config, _) = engine
            .parse_manifest("spectrum", manifest)
            .map_err(|e| e.to_string())?;
        if config.target != project.target || config.source.rsplit('.').next() != Some("asm") {
            return Err("DSW1012: Spectrum assembly source does not match its manifest".into());
        }
        return Ok((vec![config.source], None));
    }
    if project.target != "dragon32" {
        return Err("DSW1012: assembly requires Dragon 32".into());
    }
    let Some(manifest) = project.manifest.as_deref() else {
        return Ok((
            vec!["src/recovered.asm".into()],
            Some("src/basic/main.bas".into()),
        ));
    };
    if manifest.len() > 65_536 {
        return Err("DSW1011: browser manifest exceeds 64 KiB".into());
    }
    let config = parse_project_config(manifest).map_err(|error| error.to_string())?;
    let mut seen = std::collections::BTreeSet::new();
    let paths = config
        .assembler
        .sources
        .iter()
        .map(|path| {
            let path = path.to_string_lossy().into_owned();
            if !source_path(&path, "asm") || !seen.insert(path.to_ascii_lowercase()) {
                return Err(
                    "DSW1012: assembly paths must be distinct portable relative .asm files".into(),
                );
            }
            Ok(path)
        })
        .collect::<Result<Vec<_>, String>>()?;
    let basic = config.basic.entry.to_string_lossy().into_owned();
    if !source_path(&basic, "bas") || paths.is_empty() || paths.len() > 64 {
        return Err("DSW1012: expected 1–64 assembly sources and a portable BASIC entry".into());
    }
    Ok((paths, Some(basic)))
}
fn source_path(path: &str, extension: &str) -> bool {
    path.len() <= 256
        && path.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        })
        && path
            .rsplit_once('.')
            .is_some_and(|(_, ext)| ext.eq_ignore_ascii_case(extension))
}
pub(crate) fn source_paths(project: &Snapshot) -> Result<Vec<String>, String> {
    let entry = validate(project)?;
    if project.kind == Kind::Assembly {
        Ok(assembly_file_paths(project)?.0)
    } else {
        Ok(vec![entry])
    }
}
pub(crate) fn validate(project: &Snapshot) -> Result<String, String> {
    let portable = slug(&project.name)?;
    if !matches!(project.schema_version, 1 | 2)
        || (project.schema_version == 1 && project.target != "dragon32")
        || project.source.contains('\0')
        || project.source.len() > LIMIT
    {
        return Err("DSW1011: unsupported or oversized browser project".into());
    }
    if project.support_files.len() > 96 {
        return Err("DSW1011: too many project support files".into());
    }
    let (assembly_sources, basic_support) = assembly_file_paths(project)?;
    let mut guidance_bytes = 0;
    let mut source_bytes = project.source.len();
    let mut guidance_count = 0;
    for (name, content) in &project.support_files {
        if content.contains('\0') {
            return Err("DSW1011: project text must not contain NUL".into());
        }
        if support_path(name) {
            guidance_count += 1;
            guidance_bytes += content.len();
            if content.len() > 65_536 {
                return Err("DSW1011: oversized project guidance".into());
            }
        } else if assembly_sources.iter().skip(1).any(|path| path == name)
            || basic_support.as_ref() == Some(name)
        {
            source_bytes += content.len();
        } else {
            return Err("DSW1011: project support file is not declared by its manifest".into());
        }
    }
    if guidance_count > 32 || guidance_bytes > 512 * 1024 || source_bytes > LIMIT {
        return Err("DSW1011: unsupported or oversized project source/support files".into());
    }
    if project.target == "vectrex" {
        if project.kind != Kind::Assembly || project.source.len() > 128 * 1024 {
            return Err("DSW1012: Vectrex requires one assembly source up to 128 KiB".into());
        }
        return assembly_sources
            .first()
            .cloned()
            .ok_or_else(|| "DSW1012: missing Vectrex source".into());
    }
    if !is_dragon(&project.target) {
        let engine = crate::machines::engine(&project.target)?;
        if (project.kind != Kind::Basic
            && eightbit_spectrum_core::original_target(&project.target).is_none())
            || project.source.len() > eightbit_source::MAX_SOURCE
        {
            return Err("DSW1012: use a supported single source up to 128 KiB".into());
        }
        let manifest = project
            .manifest
            .as_deref()
            .ok_or("DSW1011: BASIC manifest is missing")?;
        let (config, _) = engine
            .parse_manifest(engine.families[0].definition.id, manifest)
            .map_err(|e| e.to_string())?;
        if config.target != project.target
            || config.source.rsplit('.').next()
                != Some(if project.kind == Kind::Basic {
                    "bas"
                } else {
                    "asm"
                })
            || support_path(&config.source)
        {
            return Err(
                "DSW1012: project target or source does not match its BASIC manifest".into(),
            );
        }
        return Ok(config.source);
    }
    match (project.kind, project.manifest.as_deref()) {
        (Kind::Basic, Some(manifest)) => {
            if manifest.len() > 65_536 {
                return Err("DSW1011: browser manifest exceeds 64 KiB".into());
            }
            let config = parse_project_config(manifest).map_err(|error| error.to_string())?;
            if !matches!(config.schema_version, 1..=5)
                || (config.schema_version >= 3
                    && (config.basic.sources.len() != 1
                        || config.basic.sources[0] != config.basic.entry))
                || dragon_target(&config) != project.target
                || config.assembler.enabled
                || config.run.boot != "basic"
                || config.project.name != portable
            {
                return Err("DSW1012: this browser supports single-source Dragon BASIC projects with assembly disabled; use the desktop edition for this project".into());
            }
            let entry = config.basic.entry.to_string_lossy().replace('\\', "/");
            if ["dragon.toml", "eightbit-project.json"].contains(&entry.as_str())
                || support_path(&entry)
            {
                return Err(
                    "DSW1012: BASIC source cannot replace a project settings or guidance file"
                        .into(),
                );
            }
            Ok(entry)
        }
        (Kind::Assembly, Some(manifest)) => {
            if manifest.len() > 65_536 {
                return Err("DSW1011: browser manifest exceeds 64 KiB".into());
            }
            let config = parse_project_config(manifest).map_err(|error| error.to_string())?;
            if config.run.boot == "cartridge" {
                return Err("DSW1023: cartridges are unsupported on the website; use the desktop edition with XRoar".into());
            }
            if config.schema_version != 3
                || config.machine.model != "dragon32"
                || config.machine.ram_kib != 32
                || !config.assembler.enabled
                || config.basic.sources.len() != 1
                || config.basic.sources[0] != config.basic.entry
                || !basic_support
                    .as_ref()
                    .is_some_and(|path| project.support_files.contains_key(path))
                || assembly_sources
                    .iter()
                    .skip(1)
                    .any(|path| !project.support_files.contains_key(path))
                || config.assembler.output_format.as_deref() != Some("raw")
                || config.run.boot != "machine-code"
                || config.project.name != portable
            {
                return Err("DSW1012: browser assembly requires 1–64 declared Dragon 32 RAM sources, its BASIC entry and machine-code boot".into());
            }
            let entry = assembly_sources
                .first()
                .ok_or("DSW1012: assembly source is missing")?
                .clone();
            Ok(entry)
        }
        (Kind::Assembly, None) => Ok("src/recovered.asm".into()),
        _ => Err("DSW1011: project language and manifest do not match".into()),
    }
}
pub(crate) fn import(bytes: &[u8]) -> Result<Snapshot, String> {
    if !bytes.starts_with(b"PK") {
        let mut snapshot: Snapshot = serde_json::from_slice(bytes)
            .map_err(|_| "DSW1011: invalid browser project file".to_owned())?;
        validate(&snapshot)?;
        snapshot.schema_version = 2;
        return Ok(snapshot);
    }
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| "DSW1013: invalid project ZIP".to_owned())?;
    if archive.len() > MAX_PROJECT_FILES {
        return Err("DSW1013: project ZIP contains too many files".into());
    }
    let mut files = std::collections::BTreeMap::new();
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|_| "DSW1013: unreadable project ZIP".to_owned())?;
        let name = file.name().to_owned();
        if file.enclosed_name().is_none()
            || name.contains('\\')
            || file
                .unix_mode()
                .is_some_and(|mode| mode & 0o170_000 == 0o120_000)
        {
            return Err("DSW1013: project ZIP contains an unsafe path or symlink".into());
        }
        if file.is_dir() {
            continue;
        }
        let file_limit = if name == "eightbit-project.json" {
            7 * LIMIT
        } else {
            LIMIT + 65_536
        };
        if file.size() > u64::try_from(file_limit).map_err(|_| "DSW1013: size conversion failed")?
            || files.contains_key(&name)
        {
            return Err("DSW1013: oversized or duplicate project file".into());
        }
        let mut data = Vec::new();
        file.by_ref()
            .take(u64::try_from(file_limit + 1).map_err(|_| "DSW1013: size conversion failed")?)
            .read_to_end(&mut data)
            .map_err(|_| "DSW1013: corrupt project file".to_owned())?;
        if data.len() > file_limit {
            return Err("DSW1013: oversized project file".into());
        }
        let text = String::from_utf8(data)
            .map_err(|_| "DSW1013: project files must be UTF-8 text".to_owned())?;
        files.insert(name, text);
        if files.values().map(String::len).sum::<usize>() > 9 * LIMIT {
            return Err("DSW1013: expanded project ZIP exceeds its limit".into());
        }
    }
    let metadata: Option<Snapshot> = files
        .get("eightbit-project.json")
        .map(|json| {
            serde_json::from_str(json).map_err(|_| "DSW1011: invalid project snapshot".to_owned())
        })
        .transpose()?;
    // Desktop files are authoritative: desktop edits do not rewrite browser metadata.
    let manifests = [
        "dragon.toml",
        "c64.toml",
        "bbc.toml",
        "spectrum.toml",
        "zx81.toml",
        "vectrex.toml",
    ]
    .into_iter()
    .filter(|name| files.contains_key(*name))
    .collect::<Vec<_>>();
    if manifests.len() > 1 {
        return Err("DSW1013: project ZIP contains ambiguous computer manifests".into());
    }
    let mut snapshot = if let Some(manifest_name) = manifests.first() {
        let manifest = files
            .get(*manifest_name)
            .ok_or("DSW1013: missing manifest")?;
        let (portable, entry, target, kind) = if *manifest_name == "dragon.toml" {
            let config = parse_project_config(manifest).map_err(|e| e.to_string())?;
            let target = dragon_target(&config).to_owned();
            let assembly = config.run.boot != "basic";
            let entry = if assembly {
                config
                    .assembler
                    .sources
                    .first()
                    .ok_or("DSW1012: assembly source is missing")?
            } else {
                &config.basic.entry
            };
            (
                config.project.name,
                entry.to_string_lossy().replace('\\', "/"),
                target,
                if assembly {
                    Kind::Assembly
                } else {
                    Kind::Basic
                },
            )
        } else if *manifest_name == "vectrex.toml" {
            let config = eightbit_vectrex_emulator::cartridge::parse_manifest(manifest)
                .map_err(|e| e.to_string())?;
            (config.name, config.source, config.target, Kind::Assembly)
        } else {
            let config: eightbit_source::Manifest =
                toml::from_str(manifest).map_err(|_| "DSW1013: invalid source manifest")?;
            let engine = crate::machines::engine(&config.target)?;
            if engine.families[0].manifest != *manifest_name {
                return Err("DSW1013: manifest belongs to a different computer".into());
            }
            engine
                .parse_manifest(engine.families[0].definition.id, manifest)
                .map_err(|e| e.to_string())?;
            let kind = if config.source.rsplit('.').next() == Some("asm")
                && eightbit_spectrum_core::original_target(&config.target).is_some()
            {
                Kind::Assembly
            } else {
                Kind::Basic
            };
            (config.name, config.source, config.target, kind)
        };
        let name = metadata
            .as_ref()
            .filter(|data| {
                data.target == target
                    && slug(&data.name)
                        .ok()
                        .map(|name| {
                            if is_dragon(&target) {
                                name
                            } else {
                                name.to_ascii_lowercase().replace('_', "-")
                            }
                        })
                        .as_deref()
                        == Some(portable.as_str())
            })
            .map_or_else(|| portable.clone(), |data| data.name.clone());
        Snapshot {
            schema_version: 2,
            target,
            name,
            kind,
            source: files
                .get(&entry)
                .ok_or("DSW1013: project entry file is missing")?
                .clone(),
            manifest: Some(manifest.clone()),
            support_files: std::collections::BTreeMap::new(),
        }
    } else {
        let mut project = metadata.ok_or("DSW1013: ZIP needs a supported manifest at its root")?;
        if project.kind != Kind::Assembly || project.target != "dragon32" {
            return Err("DSW1013: BASIC project ZIP needs its desktop manifest".into());
        }
        let entry = validate(&project)?;
        project.source.clone_from(
            files
                .get(&entry)
                .ok_or("DSW1013: assembly source is missing")?,
        );
        project
    };
    let (assembly_sources, basic_support) = assembly_file_paths(&snapshot)?;
    snapshot.support_files = files
        .iter()
        .filter(|(name, _)| {
            support_path(name)
                || assembly_sources.iter().skip(1).any(|path| path == *name)
                || basic_support.as_ref() == Some(*name)
        })
        .map(|(name, content)| (name.clone(), content.clone()))
        .collect();
    let entry = validate(&snapshot)?;
    for name in files.keys() {
        if !([
            entry.as_str(),
            manifest_name(&snapshot)?,
            "eightbit-project.json",
        ]
        .contains(&name.as_str())
            || snapshot.support_files.contains_key(name))
        {
            return Err("DSW1013: ZIP contains additional files this browser cannot preserve; use a source-only project ZIP".into());
        }
    }
    Ok(snapshot)
}
pub(crate) fn manifest_name(project: &Snapshot) -> Result<&'static str, String> {
    if project.target == "vectrex" {
        return Ok("vectrex.toml");
    }

    if is_dragon(&project.target) {
        Ok("dragon.toml")
    } else {
        Ok(crate::machines::engine(&project.target)?.families[0].manifest)
    }
}
pub(crate) fn export(project: &Snapshot) -> Result<Vec<u8>, String> {
    let entry = validate(project)?;
    let mut current = project.clone();
    current.schema_version = 2;
    let snapshot =
        serde_json::to_string_pretty(&current).map_err(|_| "DSW1014: cannot encode project")?;
    let mut files = vec![
        ("eightbit-project.json", snapshot.as_str()),
        (entry.as_str(), project.source.as_str()),
    ];
    if let Some(manifest) = project.manifest.as_deref() {
        files.push((manifest_name(project)?, manifest));
    }
    for (name, content) in &project.support_files {
        files.push((name.as_str(), content.as_str()));
    }
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(0o644);
    for (name, content) in files {
        writer
            .start_file(name, options)
            .map_err(|_| "DSW1014: cannot create ZIP entry")?;
        writer
            .write_all(content.as_bytes())
            .map_err(|_| "DSW1014: cannot write ZIP entry")?;
    }
    Ok(writer
        .finish()
        .map_err(|_| "DSW1014: cannot finish ZIP")?
        .into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn project_round_trip_and_desktop_contract() {
        let project = create("My first game".into(), Kind::Basic, "10 END\n".into()).unwrap();
        assert_eq!(
            parse_project_config(project.manifest.as_deref().unwrap())
                .unwrap()
                .project
                .name,
            "My-first-game"
        );
        let zip = export(&project).unwrap();
        assert_eq!(zip, export(&project).unwrap());
        let restored = import(&zip).unwrap();
        assert_eq!(restored.source, project.source);
        assert_eq!(restored.name, project.name);
        let assembly = create("Recovered".into(), Kind::Assembly, "ORG $400".into()).unwrap();
        assert_eq!(
            import(&export(&assembly).unwrap()).unwrap().kind as u8,
            Kind::Assembly as u8
        );
    }
    #[test]
    fn desktop_and_browser_share_defaults_and_desktop_edits_return_without_loss() {
        let original = create("Portable game".into(), Kind::Basic, "10 END\n".into()).unwrap();
        assert_eq!(
            original.manifest.as_deref(),
            Some(scaffold::manifest("Portable-game", false).unwrap().as_str())
        );
        for (name, content) in [
            ("AGENTS.md", scaffold::AGENTS),
            ("CLAUDE.md", scaffold::CLAUDE),
            ("README.md", scaffold::README),
            ("tests/README.md", scaffold::TESTS_README),
            (".gitignore", scaffold::GITIGNORE),
        ] {
            assert_eq!(
                original.support_files.get(name).map(String::as_str),
                Some(content)
            );
        }
        let root = tempfile::tempdir().unwrap();
        let bytes = export(&original).unwrap();
        let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
        for index in 0..zip.len() {
            let mut file = zip.by_index(index).unwrap();
            let path = root.path().join(file.enclosed_name().unwrap());
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut contents = Vec::new();
            file.read_to_end(&mut contents).unwrap();
            std::fs::write(path, contents).unwrap();
        }
        let desktop = eightbit_dragon_project::load_project(root.path()).unwrap();
        assert_eq!(desktop.config.schema_version, 3);
        assert_eq!(
            desktop.config.basic.entry.to_str(),
            Some("src/basic/main.bas")
        );
        std::fs::write(
            root.path().join("src/basic/main.bas"),
            "10 PRINT 42\n20 END\n",
        )
        .unwrap();
        std::fs::write(root.path().join("AGENTS.md"), "# Custom project guidance\n").unwrap();
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        for name in [
            "dragon.toml",
            "eightbit-project.json",
            "src/basic/main.bas",
            "AGENTS.md",
            "CLAUDE.md",
            "README.md",
            "tests/README.md",
            ".gitignore",
        ] {
            writer
                .start_file(
                    name,
                    SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
                )
                .unwrap();
            writer
                .write_all(&std::fs::read(root.path().join(name)).unwrap())
                .unwrap();
        }
        let back = import(&writer.finish().unwrap().into_inner()).unwrap();
        assert_eq!(back.source, "10 PRINT 42\n20 END\n");
        assert_eq!(
            back.support_files["AGENTS.md"],
            "# Custom project guidance\n"
        );
        assert_eq!(back.support_files["CLAUDE.md"], scaffold::CLAUDE);
    }
    fn multi_source_project(count: usize) -> Snapshot {
        let mut project = create("Multi".into(), Kind::Assembly, "START rts\n".into()).unwrap();
        let mut manifest: toml::Value =
            toml::from_str(project.manifest.as_deref().unwrap()).unwrap();
        manifest["assembler"]["sources"] = toml::Value::Array(
            (0..count)
                .map(|index| toml::Value::String(format!("asm/source{index}.asm")))
                .collect(),
        );
        manifest["assembler"]["load_address"] = toml::Value::Integer(0x2000);
        manifest["basic"]["entry"] = toml::Value::String("src/MAIN.BAS".into());
        manifest["basic"]["sources"] =
            toml::Value::Array(vec![toml::Value::String("src/MAIN.BAS".into())]);
        project.manifest = Some(toml::to_string(&manifest).unwrap());
        project.support_files.clear();
        project.support_files.insert(
            "src/MAIN.BAS".into(),
            "10 REM historical loader is retained, not executed\n20 END\n".into(),
        );
        for index in 1..count {
            project.support_files.insert(
                format!("asm/source{index}.asm"),
                "; secondary source\n".into(),
            );
        }
        project
    }
    #[test]
    fn declared_sources_and_native_settings_round_trip_without_rewriting_the_basic_stub() {
        let mut project = multi_source_project(64);
        project
            .support_files
            .insert("AGENTS.md".into(), "User-owned guidance".into());
        assert_eq!(source_paths(&project).unwrap().len(), 64);
        let zip = export(&project).unwrap();
        let restored = import(&zip).unwrap();
        assert_eq!(restored.support_files, project.support_files);
        assert_eq!(restored.manifest, project.manifest);
        assert_eq!(export(&restored).unwrap(), zip);
        configure_assembly(
            &mut project,
            &AssemblyOptions {
                load_address: 0x3800,
                entry_symbol: "START".into(),
            },
        )
        .unwrap();
        let config = parse_project_config(project.manifest.as_deref().unwrap()).unwrap();
        assert_eq!(config.assembler.sources.len(), 64);
        assert_eq!(config.basic.entry.to_str(), Some("src/MAIN.BAS"));
        assert_eq!(config.assembler.load_address, Some(0x3800));
        assert_eq!(project.support_files, restored.support_files);
        let before = project.manifest.clone();
        assert!(
            configure_assembly(
                &mut project,
                &AssemblyOptions {
                    load_address: 0xffff,
                    entry_symbol: "START".into()
                }
            )
            .is_err()
        );
        assert_eq!(project.manifest, before);
    }
    #[test]
    fn assembly_project_membership_bounds_and_paths_are_checked_before_export_or_import() {
        assert!(validate(&multi_source_project(65)).is_err());
        let base = multi_source_project(2);
        for path in [
            "../bad.asm",
            "/bad.asm",
            "asm/../bad.asm",
            "asm/./bad.asm",
            "asm//bad.asm",
            "asm\\bad.asm",
            "asm/second.bin",
            "asm/source0.asm",
            "ASM/SOURCE0.ASM",
        ] {
            let mut project = base.clone();
            project.manifest = project
                .manifest
                .map(|text| text.replace("asm/source1.asm", path));
            assert!(validate(&project).is_err(), "{path}");
        }
        for (path, content) in [
            ("other.asm", "rts".into()),
            ("assets/picture.bin", "not admitted".into()),
            ("asm/source1.asm", "nul\0source".into()),
            ("asm/source1.asm", "x".repeat(LIMIT)),
            ("AGENTS.md", "x".repeat(65_537)),
        ] {
            let mut project = base.clone();
            project.support_files.insert(path.into(), content);
            assert!(export(&project).is_err(), "{path}");
            assert!(import(&serde_json::to_vec(&project).unwrap()).is_err());
        }
        let mut missing = base.clone();
        missing.support_files.remove("asm/source1.asm");
        assert!(validate(&missing).is_err());
        let mut mixed = base.clone();
        mixed.manifest = mixed
            .manifest
            .map(|text| text.replace("boot = \"machine-code\"", "boot = \"basic\""));
        assert!(validate(&mixed).is_err());
        let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, text) in [
            ("dragon.toml", base.manifest.as_deref().unwrap()),
            ("asm/source0.asm", &base.source),
            ("asm/source1.asm", &base.support_files["asm/source1.asm"]),
            ("src/MAIN.BAS", &base.support_files["src/MAIN.BAS"]),
            ("extra.asm", "rts"),
        ] {
            archive
                .start_file(name, SimpleFileOptions::default())
                .unwrap();
            archive.write_all(text.as_bytes()).unwrap();
        }
        assert!(import(&archive.finish().unwrap().into_inner()).is_err());
    }
    #[test]
    fn hostile_future_and_unsupported_projects_are_rejected() {
        let mut project = create("Test".into(), Kind::Basic, String::new()).unwrap();
        assert!(create("../bad".into(), Kind::Basic, String::new()).is_err());
        project.schema_version = 3;
        assert!(validate(&project).is_err());
        project.schema_version = 1;
        project.source = "x".repeat(LIMIT + 1);
        assert!(validate(&project).is_err());
        project.source.clear();
        let manifest = project.manifest.clone();
        project.manifest = manifest
            .as_ref()
            .map(|value| value.replace("src/basic/main.bas", "AGENTS.md"));
        assert!(validate(&project).is_err());
        project.manifest = manifest;
        project.manifest = project
            .manifest
            .map(|value| value.replace("src/basic/main.bas", "../escape.bas"));
        assert!(validate(&project).is_err());
        assert!(import(br#"{"schemaVersion":1,"name":"T","kind":"assembly","source":"","manifest":null,"rom":[1]}"#).is_err());
        assert!(import(b"PKbroken").is_err());
    }
}
