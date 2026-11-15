//! Authored, portable fixtures for the explicit-source linker contract.
#![allow(clippy::unwrap_used, clippy::panic)]
use eightbit_m6809_core::assembler::{AssemblySource, assemble, assemble_project};

fn unit<'a>(source_uri: &'a str, source: &'a str) -> AssemblySource<'a> {
    AssemblySource { source_uri, source }
}

#[test]
fn links_forward_imports_constants_and_scoped_locals_deterministically() {
    let sources = [
        unit(
            "asm/main.asm",
            " section code\n export START,CONST\n extern DATA\nSTART ldx #DATA\n bsr LOCAL\n rts\nLOCAL rts\n endsection\nCONST equ $1234\n",
        ),
        unit(
            "asm/data.asm",
            " section code\n export DATA\n extern CONST\nDATA fdb CONST\nLOCAL fcb $55\n endsection\n",
        ),
    ];
    let linked = assemble_project(&sources, 0x7000, "start").unwrap();
    assert_eq!(linked, assemble_project(&sources, 0x7000, "START").unwrap());
    assert_eq!(linked.load_address, 0x7000);
    assert_eq!(linked.entry_address, 0x7000);
    assert_eq!(
        linked.bytes,
        [0x8e, 0x70, 7, 0x8d, 1, 0x39, 0x39, 0x12, 0x34, 0x55]
    );
    let locals = linked
        .symbols
        .iter()
        .filter(|s| s.name == "LOCAL")
        .collect::<Vec<_>>();
    assert_eq!(locals.len(), 2);
    assert_eq!(
        (
            locals[0].source_uri.as_str(),
            locals[0].address,
            locals[0].exported
        ),
        ("asm/main.asm", 0x7006, false)
    );
    assert_eq!(
        (
            locals[1].source_uri.as_str(),
            locals[1].address,
            locals[1].exported
        ),
        ("asm/data.asm", 0x7009, false)
    );
    assert!(
        linked
            .symbols
            .iter()
            .any(|s| s.name == "CONST" && s.address == 0x1234 && s.exported)
    );
    assert_eq!(
        linked
            .lines
            .iter()
            .map(|l| (l.source_uri.as_str(), l.source_line, l.address, l.bytes))
            .collect::<Vec<_>>(),
        [
            ("asm/main.asm", 4, 0x7000, 3),
            ("asm/main.asm", 5, 0x7003, 2),
            ("asm/main.asm", 6, 0x7005, 1),
            ("asm/main.asm", 7, 0x7006, 1),
            ("asm/data.asm", 4, 0x7007, 2),
            ("asm/data.asm", 5, 0x7009, 1),
        ]
    );
}

#[test]
fn imports_are_explicit_and_failures_retain_source_locations() {
    for (first, second, uri, line, message) in [
        (
            "START ldx #DATA\n",
            "DATA fcb 1\n export DATA\n",
            "main.asm",
            1,
            "Undefined symbol DATA",
        ),
        (
            " extern DATA\nSTART rts\n",
            "DATA fcb 1\n",
            "main.asm",
            1,
            "no exported provider",
        ),
        (
            "START rts\n export START\n",
            "START rts\n export START\n",
            "data.asm",
            2,
            "more than one provider",
        ),
        (
            " extern START\nSTART rts\n",
            "",
            "main.asm",
            1,
            "also locally defined",
        ),
        (
            " export MISSING\nSTART rts\n",
            "",
            "main.asm",
            1,
            "not defined in this source",
        ),
        (
            "START rts\n",
            "START rts\n",
            "main.asm",
            1,
            "unique local definition",
        ),
    ] {
        let error = assemble_project(
            &[unit("main.asm", first), unit("data.asm", second)],
            0x7000,
            "START",
        )
        .unwrap_err();
        assert_eq!(
            (error.source_uri.as_str(), error.line, error.column),
            (uri, line, 1)
        );
        assert!(error.message.contains(message), "{error}");
    }
    assert!(
        assemble(" extern DATA\nSTART rts\n", 0x7000, "START")
            .unwrap_err()
            .message
            .contains("assemble_project")
    );
}

#[test]
fn constants_sections_are_non_emitting_and_forward_equates_resolve() {
    let source = " section _constants\nONE equ TWO-1\nTWO equ 2\n endsection\n section code\n export START,ONE\nSTART lda #ONE\n rts\n endsection\n";
    let linked = assemble_project(&[unit("main.asm", source)], 0x3800, "START").unwrap();
    assert_eq!(linked.bytes, [0x86, 1, 0x39]);
    assert_eq!(linked.lines.len(), 2);
    for source in [
        " section _constants\nSTART nop\n endsection\n",
        " section _constants\nSTART fcb 0\n endsection\n",
        " section _constants\nSTART\n endsection\n",
        " section _constants\nSTART endsection\n nop\n",
        " section code\n section code\nSTART rts\n endsection\n",
        " section code\nSTART rts\n",
        " endsection\nSTART rts\n",
        " section arbitrary\nSTART rts\n endsection\n",
        "A equ B\nB equ A\nSTART lda #A\n",
    ] {
        assert!(
            assemble_project(&[unit("bad.asm", source)], 0x7000, "START").is_err(),
            "{source}"
        );
    }
}

#[test]
fn source_order_and_padding_have_exact_source_maps() {
    let sources = [
        unit("first.asm", "START nop\n org $7004\n fdb $1234\n"),
        unit("second.asm", " rmb 2\n fcc \"HI\"\n"),
    ];
    let linked = assemble_project(&sources, 0x7000, "START").unwrap();
    assert_eq!(linked.bytes, [0x12, 0, 0, 0, 0x12, 0x34, 0, 0, b'H', b'I']);
    assert_eq!(
        linked
            .lines
            .iter()
            .map(|l| (l.source_uri.as_str(), l.source_line, l.address, l.bytes))
            .collect::<Vec<_>>(),
        [
            ("first.asm", 1, 0x7000, 1),
            ("first.asm", 2, 0x7001, 3),
            ("first.asm", 3, 0x7004, 2),
            ("second.asm", 1, 0x7006, 2),
            ("second.asm", 2, 0x7008, 2),
        ]
    );
    // The old single-unit map remains instruction-only.
    assert_eq!(
        assemble(sources[0].source, 0x7000, "START")
            .unwrap()
            .lines
            .len(),
        1
    );
}

#[test]
fn cross_unit_relaxation_resolves_before_strict_org_validation() {
    let sources = [
        unit("first.asm", " extern VALUE\nSTART lda VALUE\n org 2\n"),
        unit("second.asm", " export VALUE\nVALUE equ $42\n rts\n"),
    ];
    assert_eq!(
        assemble_project(&sources, 0, "START").unwrap().bytes,
        [0x96, 0x42, 0x39]
    );
}

#[test]
fn image_end_allows_constants_only_units_and_rejects_overflow() {
    let linked = assemble_project(
        &[
            unit("last.asm", "START fcb 1\n"),
            unit("constants.asm", "CONST equ $ff\n"),
        ],
        0xffff,
        "START",
    )
    .unwrap();
    assert_eq!(linked.bytes, [1]);
    assert_eq!(linked.lines[0].address, 0xffff);
    let error = assemble_project(
        &[
            unit("last.asm", "START fcb 1\n"),
            unit("overflow.asm", " fcb 2\n"),
        ],
        0xffff,
        "START",
    )
    .unwrap_err();
    assert_eq!(error.source_uri, "overflow.asm");
    assert!(error.message.contains("end of the 16-bit"));
    let full = assemble_project(&[unit("full.asm", "START rmb 65536\n")], 0, "START").unwrap();
    assert_eq!(full.bytes.len(), 65_536);
    assert_eq!(
        full.lines
            .iter()
            .map(|l| (l.address, l.bytes))
            .collect::<Vec<_>>(),
        [(0, 65_535), (65_535, 1)]
    );
}

#[test]
fn exported_entry_takes_precedence_and_must_be_inside_the_image() {
    let linked = assemble_project(
        &[
            unit("local.asm", "START nop\n"),
            unit("exported.asm", " export START\nSTART rts\n"),
        ],
        0x7000,
        "START",
    )
    .unwrap();
    assert_eq!(linked.entry_address, 0x7001);
    for source in ["START equ $6000\n nop\n", "START equ $7001\n nop\n"] {
        assert!(assemble_project(&[unit("main.asm", source)], 0x7000, "START").is_err());
    }
}

#[test]
fn project_source_count_identity_and_aggregate_limits_are_bounded() {
    assert!(assemble_project(&[], 0x7000, "START").is_err());
    let names = (0..65)
        .map(|index| format!("{index}.asm"))
        .collect::<Vec<_>>();
    let mut sources = names.iter().map(|name| unit(name, "")).collect::<Vec<_>>();
    sources[0].source = "START rts\n";
    assert!(assemble_project(&sources[..64], 0x7000, "START").is_ok());
    assert!(assemble_project(&sources, 0x7000, "START").is_err());
    for uri in ["", "bad\nidentity", "bad\0identity"] {
        assert_eq!(
            assemble_project(&[unit(uri, "START rts\n")], 0x7000, "START")
                .unwrap_err()
                .source_uri,
            ""
        );
    }
    assert_eq!(
        assemble_project(&[unit(&"x".repeat(1025), "START rts\n")], 0x7000, "START")
            .unwrap_err()
            .source_uri,
        ""
    );
    assert!(assemble_project(&[unit("main", "START rts\n")], 0x7000, &"X".repeat(129)).is_err());
    assert!(
        assemble_project(
            &[unit("same", "START rts\n"), unit("same", "")],
            0x7000,
            "START"
        )
        .is_err()
    );
    let excessive = ";1234567890".repeat(100_000);
    assert!(
        assemble_project(&[unit("large", &excessive)], 0x7000, "START")
            .unwrap_err()
            .message
            .contains("1 MiB")
    );
    let half = format!("{}\n", ";".repeat(4095)).repeat(129);
    assert!(
        assemble_project(&[unit("one", &half), unit("two", &half)], 0x7000, "START")
            .unwrap_err()
            .message
            .contains("1 MiB")
    );
    let lines = ";\n".repeat(16_385);
    assert!(
        assemble_project(&[unit("one", &lines), unit("two", &lines)], 0x7000, "START")
            .unwrap_err()
            .message
            .contains("32768")
    );
    let long_line = format!(";{}\nSTART rts\n", "x".repeat(4096));
    assert!(
        assemble_project(&[unit("long", &long_line)], 0x7000, "START")
            .unwrap_err()
            .message
            .contains("line-length")
    );
}

#[test]
fn explicit_direct_offsets_and_opcode_named_labels_remain_generic() {
    let source = "VALUE equ $0100\nSTART lda <VALUE\n setdp 1\n ldb VALUE\nFILL cmpb #4\n bra FILL\nNOP equ 2\nFCB NOP+1\n";
    let linked = assemble_project(&[unit("main.asm", source)], 0x7000, "START").unwrap();
    assert_eq!(linked.bytes, [0x96, 0, 0xd6, 0, 0xc1, 4, 0x20, 0xfc, 3]);
    assert!(
        linked
            .symbols
            .iter()
            .any(|symbol| symbol.name == "FILL" && symbol.address == 0x7004)
    );
    assert_eq!(
        assemble(source, 0x7000, "START").unwrap().bytes,
        linked.bytes
    );
}
