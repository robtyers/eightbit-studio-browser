use super::*;
use std::{fmt::Write, fs, sync::atomic::AtomicBool};
static ENGINE: eightbit_source::Engine = eightbit_source::Engine {
    product: "Eightbit Studio 6502",
    families: &[&FAMILY],
    foreign_manifests: &[
        "c64.toml",
        "dragon.toml",
        "vectrex.toml",
        "spectrum.toml",
        "zx81.toml",
    ],
};

#[cfg(unix)]
#[test]
#[ignore = "requires explicit BBC MAME/ROM/SHA-256/ca65 paths and a desktop display"]
fn real_bbc_managed_run_stop_and_restart() {
    use eightbit_source::runtime::{Session, Settings};
    let executable = std::path::PathBuf::from(
        std::env::var_os("EIGHTBIT_BBC_MAME").expect("Set EIGHTBIT_BBC_MAME"),
    );
    let roms = std::path::PathBuf::from(
        std::env::var_os("EIGHTBIT_BBC_ROMS").expect("Set EIGHTBIT_BBC_ROMS"),
    );
    let expected = std::env::var("EIGHTBIT_BBC_ROM_SHA256").expect("Set EIGHTBIT_BBC_ROM_SHA256");
    assert_eq!(expected.len(), 64);
    assert!(expected.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert_eq!(
        runtime::validate_firmware(&roms, &PROFILES[0]).unwrap(),
        expected.to_ascii_lowercase()
    );
    let mut settings = Settings::empty();
    settings.schema_version = 3;
    settings.emulators.insert("acorn".into(), executable);
    settings.roms.insert("bbc-model-b".into(), roms);
    settings.assembler = Some(
        std::env::var_os("EIGHTBIT_CA65")
            .expect("Set EIGHTBIT_CA65")
            .into(),
    );
    let settings = settings.validate(&ENGINE, &AtomicBool::new(false)).unwrap();
    let root = tempfile::tempdir().unwrap();
    for kind in ["basic", "assembly"] {
        let project = ENGINE
            .create(root.path(), kind, "acorn", "bbc-model-b", kind)
            .unwrap();
        for _ in 0..2 {
            let mut session = Session::build_and_start(
                &ENGINE,
                &project.root,
                &settings,
                &AtomicBool::new(false),
            )
            .unwrap();
            std::thread::sleep(std::time::Duration::from_secs(5));
            assert_eq!(session.status().unwrap().state, "running");
            session.stop();
            assert_eq!(session.status().unwrap().state, "exited");
            session.stop();
        }
        assert!(
            Session::build_and_start(&ENGINE, &project.root, &settings, &AtomicBool::new(true))
                .is_err()
        );
        if kind == "basic" {
            let changed = ENGINE
                .save(&project.root, &project.revision, "10 PRINT \"broken")
                .unwrap();
            assert!(
                Session::build_and_start(
                    &ENGINE,
                    &project.root,
                    &settings,
                    &AtomicBool::new(false)
                )
                .is_err()
            );
            ENGINE
                .save(&project.root, &changed.revision, PROFILES[0].basic_template)
                .unwrap();
            let mut recovered = Session::build_and_start(
                &ENGINE,
                &project.root,
                &settings,
                &AtomicBool::new(false),
            )
            .unwrap();
            assert_eq!(recovered.status().unwrap().state, "running");
            recovered.stop();
        }
    }
}
#[test]
fn basic_exact_line_headers_tokens_and_case() {
    assert_eq!(
        basic::compile("10 PRINT A\n").unwrap(),
        [13, 0, 10, 7, 0xf1, b' ', b'A', 13, 255]
    );
    let bytes = basic::compile("10 PRINT \"Mixed case\"\n20 END\n").unwrap();
    assert!(bytes.windows(10).any(|w| w == b"Mixed case"));
    assert_eq!(&bytes[bytes.len() - 7..], &[13, 0, 20, 5, 0xe0, 13, 255]);
    assert_eq!(
        bytes,
        basic::compile("10 PRINT \"Mixed case\"\n20 END\n").unwrap()
    );
}
#[test]
fn literal_regions_pseudo_variables_names_and_line_references() {
    let bytes = basic::compile("10 PAGE=&1900:PRINT PAGE,TIMER,score\n20 DEF PROCERROR\n30 PRINT FNsum(1)\n40 IF A THEN 100 ELSE 200\n50 DATA PRINT,REM:END\n60 REM GOTO 100\n70 *RUN PRINT\n").unwrap();
    assert_eq!(bytes[4], 0xd0);
    assert!(bytes.contains(&0x90));
    for name in [
        b"TIMER".as_slice(),
        b"score",
        b"ERROR",
        b"sum",
        b"PRINT,REM:END",
        b"GOTO 100",
        b"*RUN PRINT",
    ] {
        assert!(bytes.windows(name.len()).any(|w| w == name));
    }
    assert!(bytes.windows(4).any(|w| w == [0x8d, 0x44, 0x64, 0x40]));
    assert!(bytes.windows(4).any(|w| w == [0x8d, 0x64, 0x48, 0x40]));
    let reference = basic::compile("10 ON N GOTO 1,32767\n").unwrap();
    assert!(reference.windows(4).any(|w| w == [0x8d, 0x60, 0x7f, 0x7f]));
    // Hex literals must not turn their E/F digits into keyword tokens.
    assert!(
        basic::compile("10 A%=&FEED:PRINT A%\n")
            .unwrap()
            .windows(5)
            .any(|w| w == b"&FEED")
    );
}
#[test]
fn inline_assembler_text_and_basic_expressions_remain_distinct() {
    let bytes = basic::compile("10 DIM code 64\n20 FOR pass=0 TO 2 STEP 2\n30 P%=code\n40 [OPT pass\n50 LDA #ASC\"A\"\n60 JSR &FFEE\n70 RTS\n80 ]\n90 NEXT pass\n100 CALL code\n").unwrap();
    assert!(bytes.windows(3).any(|w| w == b"LDA"));
    assert!(bytes.contains(&0x97));
    assert!(bytes.windows(5).any(|w| w == b"&FFEE"));
}
#[test]
fn malformed_source_and_memory_boundaries_fail_with_stable_codes() {
    for source in [
        "",
        "10",
        "PRINT 1",
        "10 PRINT\0",
        "10 PRINT £",
        "10 PRINT \"open",
        "10 END\n10 END",
        "20 END\n10 END",
        "32768 END",
        "10 GOTO 32768",
    ] {
        assert_eq!(
            basic::compile(source).unwrap_err().code,
            "DBBC100",
            "{source:?}"
        );
    }
    assert!(basic::compile("32767 END").is_ok());
    assert!(basic::compile(&format!("10 REM{}", "X".repeat(250))).is_ok());
    assert!(basic::compile(&format!("10 REM{}", "X".repeat(251))).is_err());
    let mut huge = String::new();
    for number in 0..40 {
        writeln!(huge, "{number} REM{}", "X".repeat(200)).unwrap();
    }
    assert!(basic::compile(&huge).is_err());
    assert!(basic::compile(&" ".repeat(128 * 1024 + 1)).is_err());
    for length in [0, PROGRAM_LIMIT + 1] {
        assert!(package_assembly(&TARGETS[0], &vec![0; length]).is_err());
    }
    assert!(package_assembly(&TARGETS[0], &vec![0; PROGRAM_LIMIT]).is_ok());
}
#[test]
fn dfs_catalogue_boot_and_payload_are_independently_decodable() {
    for (assembly, boot) in [
        (false, b"MODE 7\rCHAIN \"PROGRAM\"\r".as_slice()),
        (true, b"MODE 7\r*RUN PROGRAM\r"),
    ] {
        let image = dfs::package(&[0x60], assembly).unwrap();
        assert_eq!(image.len(), 204_800);
        assert_eq!(&image[..8], b"EIGHTBIT");
        assert_eq!(image[261], 16);
        assert_eq!(
            (u16::from(image[262] & 3) << 8) | u16::from(image[263]),
            800
        );
        assert_eq!((image[262] >> 4) & 3, 3);
        assert_eq!(&image[8..16], b"PROGRAM$");
        assert_eq!(&image[16..24], b"!BOOT  $");
        let meta = &image[264..272];
        assert_eq!(u16::from_le_bytes([meta[0], meta[1]]), ORIGIN);
        assert_eq!(u16::from_le_bytes([meta[2], meta[3]]), ORIGIN);
        assert_eq!(u16::from_le_bytes([meta[4], meta[5]]), 1);
        assert_eq!(meta[7], 3);
        assert_eq!(image[768], 0x60);
        assert_eq!(&image[512..512 + boot.len()], boot);
        assert!(image[769..].iter().all(|b| *b == 0));
    }
}
#[test]
fn lifecycle_preserves_failed_builds_and_guards_export_revisions() {
    let directory = tempfile::tempdir().unwrap();
    let cancel = AtomicBool::new(false);
    let project = ENGINE
        .create(
            directory.path(),
            "hello-bbc",
            "acorn",
            "bbc-model-b",
            "basic",
        )
        .unwrap();
    assert!(!project.root.join(".studio").exists());
    assert!(
        fs::read_to_string(project.root.join("AGENTS.md"))
            .unwrap()
            .contains("BBC Model B")
    );
    let built = ENGINE.build(&project.root, None, &cancel).unwrap();
    assert_eq!(built.bytes, dfs::IMAGE_SIZE);
    let bytes = ENGINE
        .fresh_media(&project.root, &built.revision, &built.sha256)
        .unwrap();
    let destination = directory.path().join("export.ssd");
    ENGINE
        .export(&project.root, &built.revision, &built.sha256, &destination)
        .unwrap();
    assert!(
        ENGINE
            .export(&project.root, &built.revision, &built.sha256, &destination)
            .is_err()
    );
    let changed = ENGINE
        .save(&project.root, &project.revision, "10 PRINT \"broken")
        .unwrap();
    assert!(ENGINE.build(&project.root, None, &cancel).is_err());
    assert_eq!(fs::read(project.root.join(&built.media)).unwrap(), bytes);
    assert!(
        ENGINE
            .fresh_media(&project.root, &built.revision, &built.sha256)
            .is_err()
    );
    assert!(
        ENGINE
            .save(&project.root, &project.revision, "10 END")
            .is_err()
    );
    ENGINE
        .save(&project.root, &changed.revision, "10 END")
        .unwrap();
    assert!(
        ENGINE
            .build(&project.root, None, &AtomicBool::new(true))
            .is_err()
    );
    assert!(ENGINE.build(&project.root, None, &cancel).is_ok());
    fs::write(project.root.join("c64.toml"), "").unwrap();
    assert!(ENGINE.open(&project.root).is_err());
}
#[test]
fn unsafe_manifest_paths_and_unimplemented_targets_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    assert!(
        ENGINE
            .create(directory.path(), "master", "acorn", "bbc-master", "basic")
            .is_err()
    );
    let project = ENGINE
        .create(
            directory.path(),
            "safe-bbc",
            "acorn",
            "bbc-model-b",
            "basic",
        )
        .unwrap();
    let manifest = fs::read_to_string(project.root.join("bbc.toml")).unwrap();
    for content in [
        manifest.replace("src/basic/main.bas", "src/../../outside.bas"),
        format!("{manifest}\ncommand='evil'"),
        manifest.replace("schema_version = 1", "schema_version = 2"),
    ] {
        fs::write(project.root.join("bbc.toml"), content).unwrap();
        assert!(ENGINE.open(&project.root).is_err());
    }
}
#[test]
fn runtime_probe_cancellation_failure_and_recovery() {
    let directory = tempfile::tempdir().unwrap();
    let valid = "#!/bin/sh\nset -eu\ntest \"$1\" = -help\necho 'MAME v0.289 (fixture)'\n";
    let exe =
        eightbit_test_support::shell_executable(&directory.path().join("mame"), valid).unwrap();
    assert_eq!(
        runtime::probe(&exe, &AtomicBool::new(false)).unwrap(),
        "0.289"
    );
    eightbit_test_support::replace_shell_script(&exe, "#!/bin/sh\necho 'MAME v0.288'\n").unwrap();
    assert!(runtime::probe(&exe, &AtomicBool::new(false)).is_err());
    eightbit_test_support::replace_shell_script(&exe, "#!/bin/sh\nsleep 5\n").unwrap();
    assert!(runtime::probe(&exe, &AtomicBool::new(true)).is_err());
    eightbit_test_support::replace_shell_script(&exe, valid).unwrap();
    assert!(runtime::probe(&exe, &AtomicBool::new(false)).is_ok());
}
#[test]
fn assembler_failures_cancellation_and_recovery_preserve_output() {
    let directory = tempfile::tempdir().unwrap();
    let ca = eightbit_test_support::shell_executable(&directory.path().join("ca65"), "#!/bin/sh\nif [ \"$1\" = --version ]; then echo 'ca65 V2.18'; else printf object > program.o; fi\n").unwrap();
    let valid = "#!/bin/sh\nif [ \"$1\" = --version ]; then echo 'ld65 V2.18'; else printf '\\140' > program.bin; fi\n";
    let ld =
        eightbit_test_support::shell_executable(&directory.path().join("ld65"), valid).unwrap();
    let project = ENGINE
        .create(
            directory.path(),
            "asm-bbc",
            "acorn",
            "bbc-model-b",
            "assembly",
        )
        .unwrap();
    let built = ENGINE
        .build(&project.root, Some(&ca), &AtomicBool::new(false))
        .unwrap();
    let bytes = fs::read(project.root.join(&built.media)).unwrap();
    assert_eq!(bytes[768], 0x60);
    eightbit_test_support::replace_shell_script(
        &ld,
        "#!/bin/sh\nif [ \"$1\" = --version ]; then echo 'ld65 V2.18'; else exit 2; fi\n",
    )
    .unwrap();
    assert!(
        ENGINE
            .build(&project.root, Some(&ca), &AtomicBool::new(false))
            .is_err()
    );
    assert_eq!(fs::read(project.root.join(&built.media)).unwrap(), bytes);
    assert!(
        ENGINE
            .build(&project.root, Some(&ca), &AtomicBool::new(true))
            .is_err()
    );
    eightbit_test_support::replace_shell_script(&ld, valid).unwrap();
    assert!(
        ENGINE
            .build(&project.root, Some(&ca), &AtomicBool::new(false))
            .is_ok()
    );
    ENGINE
        .save(
            &project.root,
            &project.revision,
            ".org $1900\n.include \"outside\"\n",
        )
        .unwrap();
    assert!(
        ENGINE
            .build(&project.root, Some(&ca), &AtomicBool::new(false))
            .is_err()
    );
}
#[test]
#[ignore = "requires explicit EIGHTBIT_CA65 pointing to cc65 2.19 ca65"]
fn real_cc65_bbc_template() {
    let ca = std::env::var_os("EIGHTBIT_CA65")
        .map(std::path::PathBuf::from)
        .expect("Set EIGHTBIT_CA65");
    let directory = tempfile::tempdir().unwrap();
    let project = ENGINE
        .create(
            directory.path(),
            "real-bbc",
            "acorn",
            "bbc-model-b",
            "assembly",
        )
        .unwrap();
    let built = ENGINE
        .build(&project.root, Some(&ca), &AtomicBool::new(false))
        .unwrap();
    let image = fs::read(project.root.join(built.media)).unwrap();
    assert_eq!(
        &image[768..779],
        &[0xa9, 65, 0x20, 0xee, 0xff, 0xa9, 13, 0x20, 0xe7, 0xff, 0x60]
    );
}

#[test]
fn authoritative_help_examples_build_and_retain_template_parity() {
    let html = include_str!("../../../help/GUIDE.html");
    for (language, attribute) in [
        ("basic", "<pre data-example=\"basic\"><code>"),
        ("assembly", "<pre data-example=\"assembly\"><code>"),
    ] {
        let examples = html.split(attribute).skip(1).map(|part| {
            part.split("</code>")
                .next()
                .unwrap()
                .replace("&amp;", "&")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
        });
        let mut count = 0;
        for source in examples {
            if language == "basic" {
                basic::compile(&source).unwrap();
            } else {
                eightbit_mos6502_toolchain::validate(&source, ORIGIN).unwrap();
            }
            count += 1;
        }
        assert!(count > 0);
    }
    assert!(html.contains(PROFILES[0].basic_template.trim()));
}

#[test]
#[ignore = "requires EIGHTBIT_BBC_MAME, EIGHTBIT_BBC_ROMS, EIGHTBIT_BBC_ROM_SHA256 and EIGHTBIT_CA65; real BBC ROM execution"]
fn real_bbc_disc_autoboot() {
    let executable = std::env::var_os("EIGHTBIT_BBC_MAME")
        .map(std::path::PathBuf::from)
        .expect("Set EIGHTBIT_BBC_MAME");
    let roms = std::env::var_os("EIGHTBIT_BBC_ROMS")
        .map(std::path::PathBuf::from)
        .expect("Set EIGHTBIT_BBC_ROMS");
    let expected = std::env::var("EIGHTBIT_BBC_ROM_SHA256").expect("Set EIGHTBIT_BBC_ROM_SHA256");
    assert_eq!(expected.len(), 64);
    assert!(expected.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert_eq!(
        runtime::validate_firmware(&roms, &PROFILES[0]).unwrap(),
        expected.to_ascii_lowercase()
    );
    let assembler = std::env::var_os("EIGHTBIT_CA65")
        .map(std::path::PathBuf::from)
        .expect("Set EIGHTBIT_CA65");
    let basic_source = "10 MODE 7\n20 PRINT \"BBC ACCEPTANCE 42\"\n30 PRINT FNdouble(21)\n40 GOTO 60\n50 PRINT \"WRONG BRANCH\"\n60 PROCgreet\n70 END\n80 DEF FNdouble(n)=n*2\n90 DEF PROCgreet\n100 PRINT \"BBC PROC OK\"\n110 ENDPROC\n";
    let mut assembly_source = ".org $1900\n".to_owned();
    for byte in b"BBC ASM OK\r\n" {
        writeln!(assembly_source, "LDA #{byte}\nJSR $FFEE").unwrap();
    }
    assembly_source.push_str("RTS\n");
    for (kind, source, markers) in [
        ("basic", basic_source, ["BBC ACCEPTANCE 42", "BBC PROC OK"]),
        (
            "assembly",
            assembly_source.as_str(),
            ["BBC ASM OK", "BBC ASM OK"],
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        let built = ENGINE
            .create(root.path(), "real-bbc", "acorn", "bbc-model-b", kind)
            .unwrap();
        ENGINE.save(&built.root, &built.revision, source).unwrap();
        let result = ENGINE
            .build(
                &built.root,
                (kind == "assembly").then_some(assembler.as_path()),
                &AtomicBool::new(false),
            )
            .unwrap();
        let directory = root.path().join("run");
        fs::create_dir(&directory).unwrap();
        let media = directory.join("program.ssd");
        fs::write(
            &media,
            ENGINE
                .fresh_media(&built.root, &result.revision, &result.sha256)
                .unwrap(),
        )
        .unwrap();
        let (mut args, staged_identity) =
            runtime::prepare_run(&PROFILES[0], &roms, &directory, &media).unwrap();
        assert_eq!(staged_identity, expected.to_ascii_lowercase());
        // Read only screen RAM and emit booleans about original test text, never ROM bytes.
        let script = directory.join("observe.lua");
        fs::write(&script, format!("{}{}", runtime::LOADER, r#"local observed_frames = 0
emu.register_frame_done(function()
 observed_frames = observed_frames + 1
 if observed_frames ~= 600 then return end
 local mem = manager.machine.devices[":maincpu"].spaces["program"]
 local chars = {}
 for address = 0x7c00,0x7fe7 do
  local byte = mem:read_u8(address) & 127
  chars[#chars+1] = string.char(byte >= 32 and byte <= 126 and byte or 32)
 end
 local text = table.concat(chars)
 local output = io.open("observed.txt", "w")
 output:write(tostring(text:find("FIRST_MARKER",1,true) ~= nil),"\n",tostring(text:find("SECOND_MARKER",1,true) ~= nil),"\n",tostring(text:find("WRONG BRANCH",1,true) == nil),"\n")
 output:close()
end)
"#.replace("FIRST_MARKER", markers[0]).replace("SECOND_MARKER", markers[1]))).unwrap();
        args.extend([
            "-video".into(),
            "none".into(),
            "-sound".into(),
            "none".into(),
            "-seconds_to_run".into(),
            "14".into(),
            "-script".into(),
            script.to_str().unwrap().into(),
        ]);
        let process =
            eightbit_process::Process::spawn_display(&executable, &args, &directory, true).unwrap();
        let output = process
            .wait(
                std::time::Duration::from_secs(25),
                &AtomicBool::new(false),
                16384,
            )
            .unwrap();
        assert!(
            output.status.success(),
            "MAME did not complete its bounded run: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::read_to_string(directory.join("observed.txt")).unwrap(),
            "true\ntrue\ntrue\n",
            "BBC did not execute the original {kind} example correctly"
        );
    }
}
