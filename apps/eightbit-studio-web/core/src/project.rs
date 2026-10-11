//! Browser snapshots wrap, rather than replace, the desktop project manifest.
use eightbit_dragon_project::{parse_project_config, scaffold};
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read, Write};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

const LIMIT: usize = 1024 * 1024;
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
#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Kind {
    Basic,
    Assembly,
}

fn support_path(name: &str) -> bool {
    [
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
    let mut project = if target == "dragon32" {
        create_dragon(name, kind, source)?
    } else {
        if kind != Kind::Basic {
            return Err("DSW1012: browser assembly projects require the desktop edition".into());
        }
        let portable = slug(&name)?.to_ascii_lowercase().replace('_', "-");
        let engine = crate::machines::engine(&target)?;
        let family = engine.families[0];
        let mut files = engine
            .scaffold(&portable, family.definition.id, &target, "basic")
            .map_err(|e| e.to_string())?;
        let manifest = files
            .remove(family.manifest)
            .ok_or("DSW1014: missing generated manifest")?;
        files.remove("src/basic/main.bas");
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
    let manifest = if kind == Kind::Basic {
        Some(scaffold::manifest(&portable, false).map_err(|error| error.to_string())?)
    } else {
        None
    };
    let support_files = if kind == Kind::Basic {
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
pub(crate) fn validate(project: &Snapshot) -> Result<String, String> {
    let portable = slug(&project.name)?;
    if !matches!(project.schema_version, 1 | 2)
        || (project.schema_version == 1 && project.target != "dragon32")
        || project.source.contains('\0')
        || project.source.len() > LIMIT
    {
        return Err("DSW1011: unsupported or oversized browser project".into());
    }
    if project.support_files.len() > 32
        || project
            .support_files
            .iter()
            .any(|(name, content)| !support_path(name) || content.len() > 65_536)
        || project
            .support_files
            .values()
            .map(String::len)
            .sum::<usize>()
            > 512 * 1024
    {
        return Err("DSW1011: unsupported or oversized project support files".into());
    }
    if project.target != "dragon32" {
        let engine = crate::machines::engine(&project.target)?;
        if project.kind != Kind::Basic || project.source.len() > eightbit_source::MAX_SOURCE {
            return Err("DSW1012: use a single BASIC source up to 128 KiB".into());
        }
        let manifest = project
            .manifest
            .as_deref()
            .ok_or("DSW1011: BASIC manifest is missing")?;
        let (config, _) = engine
            .parse_manifest(engine.families[0].definition.id, manifest)
            .map_err(|e| e.to_string())?;
        if config.target != project.target
            || config.source.rsplit('.').next() != Some("bas")
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
            if !matches!(config.schema_version, 1..=3)
                || (config.schema_version == 3
                    && (config.basic.sources.len() != 1
                        || config.basic.sources[0] != config.basic.entry))
                || config.machine.model != "dragon32"
                || config.assembler.enabled
                || config.run.boot != "basic"
                || config.project.name != portable
            {
                return Err("DSW1012: this browser supports single-source Dragon 32 BASIC projects with schema 1, 2 or 3 and assembly disabled; use the desktop edition for this project".into());
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
    if archive.len() > 64 {
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
        let (portable, entry, target) = if *manifest_name == "dragon.toml" {
            let config = parse_project_config(manifest).map_err(|e| e.to_string())?;
            (
                config.project.name,
                config.basic.entry.to_string_lossy().replace('\\', "/"),
                default_target(),
            )
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
            (config.name, config.source, config.target)
        };
        let name = metadata
            .as_ref()
            .filter(|data| {
                data.target == target
                    && slug(&data.name)
                        .ok()
                        .map(|name| {
                            if target == "dragon32" {
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
            kind: Kind::Basic,
            source: files
                .get(&entry)
                .ok_or("DSW1013: BASIC entry file is missing")?
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
    snapshot.support_files = files
        .iter()
        .filter(|(name, _)| support_path(name))
        .map(|(name, content)| (name.clone(), content.clone()))
        .collect();
    let entry = validate(&snapshot)?;
    for name in files.keys() {
        if ![
            entry.as_str(),
            manifest_name(&snapshot)?,
            "eightbit-project.json",
        ]
        .contains(&name.as_str())
            && !support_path(name)
        {
            return Err("DSW1013: ZIP contains additional files this browser cannot preserve; use the desktop edition".into());
        }
    }
    Ok(snapshot)
}
pub(crate) fn manifest_name(project: &Snapshot) -> Result<&'static str, String> {
    if project.target == "dragon32" {
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
