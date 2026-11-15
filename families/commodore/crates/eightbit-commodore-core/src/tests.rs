use super::*;
use eightbit_source::Engine;
use std::{fs, sync::atomic::AtomicBool};
static ENGINE: Engine = Engine {
    product: "Eightbit Studio 6502",
    families: &[&FAMILY],
    foreign_manifests: &["dragon.toml", "vectrex.toml", "spectrum.toml", "zx81.toml"],
};

// Read the shipped first-party examples rather than maintaining test-only copies.
fn help_examples(language: &str) -> Vec<String> {
    let guide = include_str!("../../../help/GUIDE.html");
    let marker = format!("<pre data-example=\"{language}\"><code>");
    guide
        .split(&marker)
        .skip(1)
        .map(|part| {
            part.split_once("</code></pre>")
                .expect("complete Help example")
                .0
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&amp;", "&")
        })
        .collect()
}

#[test]
fn help_basic_examples_encode_linked_prgs() {
    let examples = help_examples("basic");
    assert!(!examples.is_empty());
    for (index, source) in examples.iter().enumerate() {
        let bytes = basic::compile(source)
            .unwrap_or_else(|error| panic!("Help BASIC example {}: {error:?}", index + 1));
        assert_eq!(&bytes[..2], &[1, 8]);
        assert_eq!(bytes, basic::compile(source).unwrap());
        // Walk the linked output independently of the encoder. Every source line
        // must have one in-bounds link and terminator, with exactly one final link.
        let mut offset = 2;
        let mut lines = 0;
        loop {
            let address = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            if address == 0 {
                assert_eq!(offset + 2, bytes.len());
                break;
            }
            let next = usize::from(address) - 0x0801 + 2;
            assert!(next > offset + 4 && next < bytes.len());
            assert_eq!(bytes[next - 1], 0);
            offset = next;
            lines += 1;
        }
        assert_eq!(lines, source.lines().count());
    }
}

#[test]
fn help_assembly_examples_respect_native_source_policy() {
    let examples = help_examples("assembly");
    assert!(!examples.is_empty());
    for (index, source) in examples.iter().enumerate() {
        eightbit_mos6502_toolchain::validate(source, 0x080d)
            .unwrap_or_else(|error| panic!("Help assembly example {}: {error:?}", index + 1));
    }
}

#[test]
#[ignore = "requires explicit EIGHTBIT_CA65 pointing to cc65 2.19 ca65"]
fn real_cc65_help_examples_build() {
    let ca = std::env::var_os("EIGHTBIT_CA65")
        .map(std::path::PathBuf::from)
        .expect("Set EIGHTBIT_CA65 explicitly");
    let dir = tempfile::tempdir().unwrap();
    for (index, source) in help_examples("assembly").iter().enumerate() {
        let project = ENGINE
            .create(
                dir.path(),
                &format!("help-example-{index}"),
                "commodore",
                "c64-pal",
                "assembly",
            )
            .unwrap();
        let saved = ENGINE
            .save(&project.root, &project.revision, source)
            .unwrap();
        let built = ENGINE
            .build(&saved.root, Some(&ca), &AtomicBool::new(false))
            .unwrap_or_else(|error| panic!("Help assembly example {}: {error:?}", index + 1));
        let bytes = fs::read(saved.root.join(&built.media)).unwrap();
        assert_eq!(&bytes[..2], &[1, 8]);
        assert_eq!(&bytes[6..11], &[0x9e, b'2', b'0', b'6', b'1']);
        assert!(bytes.len() > 14 && bytes.len() <= 32768 + 14);
        assert_eq!(
            ENGINE
                .fresh_media(&saved.root, &built.revision, &built.sha256)
                .unwrap(),
            bytes
        );
    }
}
#[test]
fn basic_prg_links_tokens_literals_and_data() {
    let bytes = basic::compile("10 PRINT\"HELLO\"\n20 END\n").unwrap();
    assert_eq!(
        bytes,
        vec![
            1, 8, 14, 8, 10, 0, 0x99, 34, b'H', b'E', b'L', b'L', b'O', 34, 0, 20, 8, 20, 0, 0x80,
            0, 0, 0
        ]
    );
    let b = basic::compile("0 DATA PRINT,\"FOR:REM\":PRINT 1\n10 REM PRINT+END\n").unwrap();
    assert!(b.windows(5).any(|w| w == b"PRINT"));
    assert_eq!(b.iter().filter(|b| **b == 0x99).count(), 1);
    assert!(b.windows(9).any(|w| w == b"PRINT+END"));
    assert_eq!(
        basic::compile("10 ?1\n").unwrap(),
        basic::compile("10 PRINT1\n").unwrap()
    );
}

#[test]
fn original_guest_front_end_fixture_preserves_the_existing_prg_contract() {
    let source = include_str!("../tests/fixtures/original-front-end.bas");
    let hex = include_str!("../tests/fixtures/original-front-end.hex").trim();
    let expected: Vec<_> = (0..hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).unwrap())
        .collect();
    assert_eq!(basic::compile(source).unwrap(), expected);
    assert_eq!(basic::decompile(&expected).unwrap(), source);
    let words = include_str!("../tests/fixtures/basic-v2-keywords.txt");
    assert_eq!(words.lines().count(), 76);
    for (index, word) in words.lines().enumerate() {
        let prg = basic::compile(&format!("10 {word}\n")).unwrap();
        assert_eq!(prg[6], 0x80 + u8::try_from(index).unwrap(), "{word}");
    }
}
#[test]
#[allow(clippy::format_collect)] // Bounded generated fixture.
fn hostile_and_boundary_basic() {
    for bad in [
        "",
        "10",
        "10 PRINT\"X",
        "10 END\n10 END",
        "20 END\n10 END",
        "64000 END",
        "PRINT 1",
        "10 PRINT\0",
        "10 PRINT £",
    ] {
        assert!(basic::compile(bad).is_err(), "{bad:?}");
    }
    assert!(basic::compile("63999 END").is_ok());
    assert!(basic::compile(&format!("10 REM{}", "X".repeat(249))).is_ok());
    assert!(basic::compile(&format!("10 REM{}", "X".repeat(250))).is_err());
    let huge = (1..400)
        .map(|n| format!("{n} REM{}\n", "X".repeat(200)))
        .collect::<String>();
    assert!(basic::compile(&huge).is_err());
}
#[test]
fn assembly_loader_address_and_limits() {
    let prg = package_assembly(&TARGETS[0], &[0x60]).unwrap();
    assert_eq!(&prg[6..11], &[0x9e, b'2', b'0', b'6', b'1']);
    assert_eq!(prg[14], 0x60);
    assert_eq!(0x0801 + prg.len() - 3, 2061);
    assert!(package_assembly(&TARGETS[0], &[]).is_err());
    assert!(package_assembly(&TARGETS[0], &vec![0; 32769]).is_err());
}
#[test]
fn project_lifecycle_is_inert_atomic_and_revision_bound() {
    let dir = tempfile::tempdir().unwrap();
    let cancel = AtomicBool::new(false);
    let p = ENGINE
        .create(dir.path(), "hello-c64", "commodore", "c64-pal", "basic")
        .unwrap();
    assert_eq!(p.target.cpu, "mos6502");
    assert!(!p.root.join(".studio").exists());
    for f in [
        "c64.toml",
        "README.md",
        "AGENTS.md",
        ".gitignore",
        "tests/README.md",
    ] {
        assert!(p.root.join(f).is_file());
    }
    let readme = fs::read_to_string(p.root.join("README.md")).unwrap();
    assert!(readme.contains("VICE"));
    assert!(!readme.contains("Z80"));
    let built = ENGINE.build(&p.root, None, &cancel).unwrap();
    let old = fs::read(p.root.join(&built.media)).unwrap();
    assert_eq!(old, basic::compile(&p.source).unwrap());
    let output = dir.path().join("export.prg");
    ENGINE
        .export(&p.root, &built.revision, &built.sha256, &output)
        .unwrap();
    assert!(
        ENGINE
            .export(&p.root, &built.revision, &built.sha256, &output)
            .is_err()
    );
    let changed = ENGINE
        .save(&p.root, &p.revision, "10 PRINT\"BROKEN")
        .unwrap();
    assert!(ENGINE.save(&p.root, &p.revision, "10 END").is_err());
    assert!(ENGINE.build(&p.root, None, &cancel).is_err());
    assert_eq!(fs::read(p.root.join(&built.media)).unwrap(), old);
    assert!(
        ENGINE
            .fresh_media(&p.root, &built.revision, &built.sha256)
            .is_err()
    );
    let fixed = ENGINE.save(&p.root, &changed.revision, "10 END").unwrap();
    assert!(
        ENGINE
            .build(&fixed.root, None, &AtomicBool::new(true))
            .is_err()
    );
    assert!(ENGINE.build(&fixed.root, None, &cancel).is_ok());
    fs::write(p.root.join("spectrum.toml"), "").unwrap();
    assert!(ENGINE.open(&p.root).is_err());
}
#[test]
fn unknown_target_manifest_and_unsafe_paths_rejected() {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        ENGINE
            .create(dir.path(), "hello", "commodore", "c128", "basic")
            .is_err()
    );
    let p = ENGINE
        .create(dir.path(), "safe", "commodore", "c64-pal", "assembly")
        .unwrap();
    assert!(
        ENGINE
            .build(&p.root, None, &AtomicBool::new(false))
            .is_err()
    );
    let config = fs::read_to_string(p.root.join("c64.toml")).unwrap();
    for source in [
        config.replace("src/asm/main.asm", "src/../../outside.asm"),
        format!("{config}\ncommand = 'evil'\n"),
        config.replace("schema_version = 1", "schema_version = 2"),
    ] {
        fs::write(p.root.join("c64.toml"), source).unwrap();
        assert!(ENGINE.open(&p.root).is_err());
    }
}
#[cfg(unix)]
#[test]
fn symlink_source_and_firmware_rejected() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let p = ENGINE
        .create(dir.path(), "safe", "commodore", "c64-pal", "basic")
        .unwrap();
    fs::remove_file(p.root.join("src/basic/main.bas")).unwrap();
    fs::write(dir.path().join("external.bas"), "10 END").unwrap();
    symlink(
        dir.path().join("external.bas"),
        p.root.join("src/basic/main.bas"),
    )
    .unwrap();
    assert!(ENGINE.open(&p.root).is_err());
    symlink(dir.path(), dir.path().join("rom-link")).unwrap();
    assert!(vice::validate_firmware(&dir.path().join("rom-link"), &PROFILES[0]).is_err());
}
#[test]
#[ignore = "requires explicit EIGHTBIT_CA65 pointing to cc65 2.19 ca65"]
fn real_cc65_template_and_rejected_build() {
    let ca = std::env::var_os("EIGHTBIT_CA65")
        .map(std::path::PathBuf::from)
        .expect("Set EIGHTBIT_CA65 explicitly");
    let dir = tempfile::tempdir().unwrap();
    let p = ENGINE
        .create(
            dir.path(),
            "real-assembly",
            "commodore",
            "c64-pal",
            "assembly",
        )
        .unwrap();
    let b = ENGINE
        .build(&p.root, Some(&ca), &AtomicBool::new(false))
        .unwrap();
    let bytes = fs::read(p.root.join(&b.media)).unwrap();
    assert_eq!(
        &bytes[14..],
        &[0xa9, 6, 0x8d, 0x20, 0xd0, 0xa9, 0, 0x8d, 0x21, 0xd0, 0x60]
    );
    ENGINE
        .save(&p.root, &p.revision, ".org $080D\nlda #999\nrts\n")
        .unwrap();
    assert!(
        ENGINE
            .build(&p.root, Some(&ca), &AtomicBool::new(false))
            .is_err()
    );
    assert_eq!(fs::read(p.root.join(&b.media)).unwrap(), bytes);
}
#[cfg(unix)]
fn fake_tool(path: &std::path::Path, script: &str) {
    use std::os::unix::fs::PermissionsExt;
    fs::write(path, format!("#!/bin/sh\nset -eu\n{script}\n")).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}
#[cfg(unix)]
#[test]
fn ca65_ld65_process_failure_and_cancellation_preserve_output() {
    let dir = tempfile::tempdir().unwrap();
    let ca = dir.path().join("ca65");
    let ld = dir.path().join("ld65");
    fake_tool(
        &ca,
        "if [ \"$1\" = --version ]; then echo 'ca65 V2.18 - fixture'; else test \"$1\" = --cpu; printf object > program.o; fi",
    );
    fake_tool(
        &ld,
        "if [ \"$1\" = --version ]; then echo 'ld65 V2.18 - fixture'; else test -f program.o; printf '\\140' > program.bin; fi",
    );
    let p = ENGINE
        .create(
            dir.path(),
            "fake-assembly",
            "commodore",
            "c64-pal",
            "assembly",
        )
        .unwrap();
    let b = ENGINE
        .build(&p.root, Some(&ca), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(b.bytes, 15);
    fake_tool(
        &ld,
        "if [ \"$1\" = --version ]; then echo 'ld65 V2.18 - fixture'; else exit 2; fi",
    );
    assert!(
        ENGINE
            .build(&p.root, Some(&ca), &AtomicBool::new(false))
            .is_err()
    );
    assert!(ENGINE.fresh_media(&p.root, &b.revision, &b.sha256).is_ok());
    assert!(
        ENGINE
            .build(&p.root, Some(&ca), &AtomicBool::new(true))
            .is_err()
    );
    fake_tool(&ca, "echo 'something else V2.18'");
    assert!(eightbit_mos6502_toolchain::probe(&ca, &AtomicBool::new(false)).is_err());
}
#[test]
fn vice_version_probe_rejects_wrong_machine_and_cancellation() {
    let dir = tempfile::tempdir().unwrap();
    let platform = if cfg!(target_os = "windows") {
        "test \"$1\" = -no-redirect-streams; shift; "
    } else {
        ""
    };
    let script = format!(
        "#!/bin/sh\nset -eu\n{platform}test \"$1\" = +logcolorize; test \"$2\" = -version; echo 'x64sc (VICE 3.10)'\n"
    );
    let exe = eightbit_test_support::shell_executable(&dir.path().join("x64sc"), &script).unwrap();
    assert_eq!(vice::probe(&exe, &AtomicBool::new(false)).unwrap(), "3.10");
    eightbit_test_support::replace_shell_script(&exe, "#!/bin/sh\necho 'x128 (VICE 3.10)'\n")
        .unwrap();
    assert!(vice::probe(&exe, &AtomicBool::new(false)).is_err());
    eightbit_test_support::replace_shell_script(&exe, "#!/bin/sh\nsleep 5\n").unwrap();
    let start = std::time::Instant::now();
    assert!(vice::probe(&exe, &AtomicBool::new(true)).is_err());
    assert!(start.elapsed() < std::time::Duration::from_secs(2));
    eightbit_test_support::replace_shell_script(&exe, &script).unwrap();
    assert_eq!(vice::probe(&exe, &AtomicBool::new(false)).unwrap(), "3.10");
}

#[cfg(windows)]
#[test]
#[ignore = "requires an explicitly selected, separately installed VICE x64sc 3.10 executable"]
fn real_vice_windows_version_probe() {
    let executable = std::path::PathBuf::from(
        std::env::var_os("EIGHTBIT_VICE").expect("explicit VICE executable path"),
    );
    let directory = tempfile::tempdir().unwrap();
    let args = [
        "-no-redirect-streams".into(),
        "+logcolorize".into(),
        "-version".into(),
    ];
    let output = eightbit_process::Process::spawn(&executable, &args, directory.path(), true)
        .unwrap()
        .wait(
            std::time::Duration::from_secs(5),
            &AtomicBool::new(false),
            16384,
        )
        .unwrap();
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "VICE version probe exited with {:?} (stdout bytes: {}, stderr bytes: {})",
        output.status.code(),
        output.stdout.len(),
        output.stderr.len()
    );
    assert!(
        text.lines().any(|line| line.trim() == "x64sc (VICE 3.10)"),
        "VICE supervisor capture omitted the required version (stdout bytes: {}, stderr bytes: {})",
        output.stdout.len(),
        output.stderr.len()
    );
    assert_eq!(
        vice::probe(&executable, &AtomicBool::new(false)).unwrap(),
        "3.10"
    );
}

#[test]
fn every_template_has_target_guidance_and_preserves_legacy_projects() {
    static TEMPLATE_ENGINE: eightbit_source::Engine = eightbit_source::Engine {
        product: "Eightbit Studio 6502",
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
                assert!(text.contains("Eightbit Studio 6502"));
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

#[cfg(unix)]
#[test]
#[ignore = "Requires EIGHTBIT_VICE, EIGHTBIT_C64_ROMS, EIGHTBIT_C64_ROM_SHA256 and a display"]
fn real_vice_managed_run_stop_and_restart() {
    use eightbit_source::runtime::{Session, Settings};
    let executable = std::env::var_os("EIGHTBIT_VICE").expect("explicit VICE path");
    let roms = std::path::PathBuf::from(
        std::env::var_os("EIGHTBIT_C64_ROMS").expect("explicit firmware folder"),
    );
    let expected =
        std::env::var("EIGHTBIT_C64_ROM_SHA256").expect("explicit framed firmware SHA-256");
    assert_eq!(expected.len(), 64, "SHA-256 required");
    assert!(expected.bytes().all(|byte| byte.is_ascii_hexdigit()));
    let actual =
        vice::validate_firmware(&roms, &PROFILES[0]).expect("supported firmware revisions");
    assert!(
        actual == expected.to_ascii_lowercase(),
        "Firmware SHA-256 mismatch"
    );
    let parent = tempfile::tempdir().expect("private project parent");
    let project = ENGINE
        .create(
            parent.path(),
            "linux-smoke",
            "commodore",
            "c64-pal",
            "basic",
        )
        .expect("original project");
    fs::write(
        project.root.join(&project.source_path),
        "10 PRINT \"C64 PAL OK\"\n20 GOTO 20\n",
    )
    .expect("original BASIC smoke");
    let mut settings = Settings::empty();
    settings.emulator = Some(executable.into());
    settings.roms.insert("c64-pal".into(), roms);
    for _ in 0..2 {
        let mut session =
            Session::build_and_start(&ENGINE, &project.root, &settings, &AtomicBool::new(false))
                .expect("managed VICE start");
        std::thread::sleep(std::time::Duration::from_secs(3));
        assert_eq!(
            session.status().expect("live managed status").state,
            "running"
        );
        session.stop();
        assert_eq!(
            session.status().expect("terminated managed status").state,
            "exited"
        );
    }
    assert!(
        Session::build_and_start(&ENGINE, &project.root, &settings, &AtomicBool::new(true))
            .is_err()
    );
}

const ACCEPTANCE_BASIC: &str = include_str!("../tests/fixtures/acceptance.bas");
const ACCEPTANCE_ASSEMBLY: &str = include_str!("../tests/fixtures/acceptance.asm");

#[test]
fn original_acceptance_sources_use_supported_project_contracts() {
    let bytes = basic::compile(ACCEPTANCE_BASIC).unwrap();
    assert_eq!(&bytes[..2], &[1, 8]);
    assert!(bytes.windows(18).any(|part| part == b"C64 BASELINE READY"));
    assert_eq!(bytes, basic::compile(ACCEPTANCE_BASIC).unwrap());
    eightbit_mos6502_toolchain::validate(ACCEPTANCE_ASSEMBLY, 0x080d).unwrap();
}

#[cfg(unix)]
#[test]
#[ignore = "Requires EIGHTBIT_CA65, EIGHTBIT_VICE, EIGHTBIT_C64_ROMS, EIGHTBIT_C64_ROM_SHA256 and a display"]
fn real_c64_acceptance_build_export_and_recovery() {
    use eightbit_source::runtime::{Session, Settings};
    let roms = std::path::PathBuf::from(
        std::env::var_os("EIGHTBIT_C64_ROMS").expect("explicit firmware folder"),
    );
    let expected = std::env::var("EIGHTBIT_C64_ROM_SHA256").expect("explicit firmware SHA-256");
    assert_eq!(expected.len(), 64, "SHA-256 required");
    assert!(expected.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert_eq!(
        vice::validate_firmware(&roms, &PROFILES[0]).unwrap(),
        expected.to_ascii_lowercase(),
        "Firmware identity must be checked before execution"
    );
    let mut settings = Settings::empty();
    settings.emulator = Some(
        std::env::var_os("EIGHTBIT_VICE")
            .expect("explicit VICE path")
            .into(),
    );
    settings.assembler = Some(
        std::env::var_os("EIGHTBIT_CA65")
            .expect("explicit ca65 path")
            .into(),
    );
    settings.roms.insert("c64-pal".into(), roms);
    let settings = settings.validate(&ENGINE, &AtomicBool::new(false)).unwrap();
    let parent = tempfile::tempdir().unwrap();
    for (template, source, invalid) in [
        ("basic", ACCEPTANCE_BASIC, "10 PRINT\"UNCLOSED\n"),
        ("assembly", ACCEPTANCE_ASSEMBLY, ".org $080D\nlda #999\n"),
    ] {
        let project = ENGINE
            .create(parent.path(), template, "commodore", "c64-pal", template)
            .unwrap();
        let project = ENGINE
            .save(&project.root, &project.revision, source)
            .unwrap();
        let built = ENGINE
            .build(
                &project.root,
                settings.assembler.as_deref(),
                &AtomicBool::new(false),
            )
            .unwrap();
        let bytes = ENGINE
            .fresh_media(&project.root, &built.revision, &built.sha256)
            .unwrap();
        let exported = parent.path().join(format!("{template}.prg"));
        ENGINE
            .export(&project.root, &built.revision, &built.sha256, &exported)
            .unwrap();
        assert_eq!(fs::read(&exported).unwrap(), bytes);
        assert!(
            ENGINE
                .export(&project.root, &built.revision, &built.sha256, &exported)
                .is_err()
        );
        let repeated = ENGINE
            .build(
                &project.root,
                settings.assembler.as_deref(),
                &AtomicBool::new(false),
            )
            .unwrap();
        assert_eq!(repeated.sha256, built.sha256);
        assert!(
            Session::build_and_start(&ENGINE, &project.root, &settings, &AtomicBool::new(true))
                .is_err()
        );

        let bad = ENGINE
            .save(&project.root, &project.revision, invalid)
            .unwrap();
        assert!(
            Session::build_and_start(&ENGINE, &bad.root, &settings, &AtomicBool::new(false))
                .is_err()
        );
        assert_eq!(fs::read(project.root.join(&built.media)).unwrap(), bytes);
        assert!(
            ENGINE
                .fresh_media(&project.root, &built.revision, &built.sha256)
                .is_err()
        );
        assert!(
            ENGINE
                .export(
                    &project.root,
                    &built.revision,
                    &built.sha256,
                    &parent.path().join(format!("stale-{template}.prg"))
                )
                .is_err()
        );

        let corrected = ENGINE.save(&bad.root, &bad.revision, source).unwrap();
        let mut session =
            Session::build_and_start(&ENGINE, &corrected.root, &settings, &AtomicBool::new(false))
                .unwrap();
        std::thread::sleep(std::time::Duration::from_secs(3));
        assert_eq!(session.status().unwrap().state, "running");
        session.stop();
        assert_eq!(session.status().unwrap().state, "exited");
    }
}

#[cfg(unix)]
#[test]
#[ignore = "Requires the explicit C64 real-tool variables and a new absolute EIGHTBIT_C64_CAPTURE_DIR"]
fn real_c64_exported_video_keyboard_and_pcm() {
    use std::path::PathBuf;
    let executable = PathBuf::from(std::env::var_os("EIGHTBIT_VICE").expect("explicit VICE path"));
    let assembler = PathBuf::from(std::env::var_os("EIGHTBIT_CA65").expect("explicit ca65 path"));
    let roms = PathBuf::from(std::env::var_os("EIGHTBIT_C64_ROMS").expect("explicit ROM folder"));
    let expected = std::env::var("EIGHTBIT_C64_ROM_SHA256").expect("explicit firmware SHA-256");
    assert_eq!(expected.len(), 64);
    assert!(expected.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert_eq!(
        vice::validate_firmware(&roms, &PROFILES[0]).unwrap(),
        expected.to_ascii_lowercase()
    );
    vice::probe(&executable, &AtomicBool::new(false)).unwrap();
    eightbit_mos6502_toolchain::probe(&assembler, &AtomicBool::new(false)).unwrap();
    let captures = PathBuf::from(
        std::env::var_os("EIGHTBIT_C64_CAPTURE_DIR").expect("explicit new capture folder"),
    );
    assert!(captures.is_absolute());
    // An explicit new folder retains only original PRGs, framebuffer images and PCM.
    // Reference firmware is staged privately by the existing adapter and then removed.
    fs::create_dir(&captures).expect("capture folder must not already exist");
    let parent = tempfile::tempdir().unwrap();
    for (name, template, source, cycles, key) in [
        ("basic-active", "basic", ACCEPTANCE_BASIC, "7000000", false),
        ("basic-after-q", "basic", ACCEPTANCE_BASIC, "14000000", true),
        (
            "assembly",
            "assembly",
            ACCEPTANCE_ASSEMBLY,
            "7000000",
            false,
        ),
    ] {
        let project = ENGINE
            .create(parent.path(), name, "commodore", "c64-pal", template)
            .unwrap();
        let project = ENGINE
            .save(&project.root, &project.revision, source)
            .unwrap();
        let built = ENGINE
            .build(&project.root, Some(&assembler), &AtomicBool::new(false))
            .unwrap();
        let exported = captures.join(format!("{name}.prg"));
        ENGINE
            .export(&project.root, &built.revision, &built.sha256, &exported)
            .unwrap();
        let staging = tempfile::tempdir().unwrap();
        let (mut arguments, _) =
            vice::prepare_run(&PROFILES[0], &roms, staging.path(), &exported).unwrap();
        let image = captures.join(format!("{name}.png"));
        let audio = captures.join(format!("{name}.wav"));
        arguments.extend([
            "-limitcycles".into(),
            cycles.into(),
            "-exitscreenshot".into(),
            image.to_str().unwrap().into(),
            "-sounddev".into(),
            "wav".into(),
            "-soundarg".into(),
            audio.to_str().unwrap().into(),
            "-soundrate".into(),
            "44100".into(),
        ]);
        if key {
            arguments.extend([
                "-keybuf".into(),
                "q".into(),
                "-keybuf-delay".into(),
                "8000000".into(),
            ]);
        }
        let output =
            eightbit_process::Process::spawn_display(&executable, &arguments, staging.path(), true)
                .unwrap()
                .wait(
                    std::time::Duration::from_secs(40),
                    &AtomicBool::new(false),
                    256 * 1024,
                )
                .unwrap();
        let log = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        // VICE deliberately returns 1 for its documented cycle-limit exit.
        assert_eq!(
            output.status.code(),
            Some(1),
            "{name}: unexpected capture exit"
        );
        assert!(
            log.contains("cycle limit reached"),
            "{name}: no bounded-cycle completion"
        );
        assert!(
            log.contains("AUTOSTART: Done."),
            "{name}: exported PRG did not autostart"
        );
        let png = eightbit_source::read_file(&image, 1024 * 1024).unwrap();
        assert!(png.len() >= 24, "{name}: complete PNG header required");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), 384);
        assert_eq!(u32::from_be_bytes(png[20..24].try_into().unwrap()), 272);
        let wav = eightbit_source::read_file(&audio, 4 * 1024 * 1024).unwrap();
        assert!(
            wav.len() > 88_244,
            "{name}: at least one second of PCM required"
        );
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(u16::from_le_bytes(wav[20..22].try_into().unwrap()), 1);
        assert_eq!(u16::from_le_bytes(wav[22..24].try_into().unwrap()), 1);
        assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), 44100);
        assert_eq!(u16::from_le_bytes(wav[34..36].try_into().unwrap()), 16);
        assert_eq!(&wav[36..40], b"data");
        let peak = |bytes: &[u8]| {
            bytes
                .chunks_exact(2)
                .map(|sample| i32::from(i16::from_le_bytes([sample[0], sample[1]])).unsigned_abs())
                .max()
                .unwrap_or(0)
        };
        if name == "basic-active" {
            // Ignore startup transients: the final second must contain the requested
            // PAL triangle tone, not merely some non-zero samples during boot.
            let tail = &wav[wav.len() - 88_200..];
            assert!(peak(tail) > 1000, "SID exercise must sustain PCM");
            let samples = tail
                .chunks_exact(2)
                .map(|sample| i16::from_le_bytes([sample[0], sample[1]]))
                .collect::<Vec<_>>();
            let crossings = samples
                .windows(2)
                .filter(|pair| pair[0] <= 0 && pair[1] > 0)
                .count();
            assert!(
                (430..=450).contains(&crossings),
                "Expected approximately 440 Hz SID output, observed {crossings} positive crossings in one second"
            );
        } else if name == "basic-after-q" {
            assert!(
                peak(&wav[44..]) > 1000,
                "input case must first play the tone"
            );
            assert!(
                peak(&wav[wav.len() - 88_200..]) < 100,
                "Q must silence SID for the final second"
            );
        }
    }
}
