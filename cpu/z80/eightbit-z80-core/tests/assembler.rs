//! Original source and literal encoding vectors; no third-party binary fixtures.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_z80_core::assembler::assemble;

#[test]
fn documented_load_encodings_and_character_case() {
    assert_eq!(
        assemble("LD HL,-1\nLD IX,-32768\nLD BC,65535", 0x6000, None)
            .unwrap()
            .bytes,
        [0x21, 255, 255, 0xdd, 0x21, 0, 128, 1, 255, 255]
    );
    let source = "ORG 24576\nstart: ld bc,$1234\n ld de,$5678\n ld hl,$9abc\n ld sp,$def0\n ld ix,$2345\n ld iy,$6789\n ld a,'a'\n ld a,(bc)\n ld (de),a\n ld (1234),hl\n ld de,(1234)\n ld a,(1234)\n ld (1234),a\n ld a,i\n ld r,a\n ld sp,iy\n ld h,(ix-2)\n ld (iy+3),l\n ld ixh,42\n ld (ix+7),5\n";
    let result = assemble(source, 24576, Some("start")).unwrap();
    assert_eq!(
        result.bytes,
        [
            1, 0x34, 0x12, 0x11, 0x78, 0x56, 0x21, 0xbc, 0x9a, 0x31, 0xf0, 0xde, 0xdd, 0x21, 0x45,
            0x23, 0xfd, 0x21, 0x89, 0x67, 0x3e, b'a', 0x0a, 0x12, 0x22, 0xd2, 4, 0xed, 0x5b, 0xd2,
            4, 0x3a, 0xd2, 4, 0x32, 0xd2, 4, 0xed, 0x57, 0xed, 0x4f, 0xfd, 0xf9, 0xdd, 0x66, 0xfe,
            0xfd, 0x75, 3, 0xdd, 0x26, 42, 0xdd, 0x36, 7, 5
        ]
    );
    assert_eq!(result.entry_address, 24576);
    assert_eq!(result.lines.len(), 20);
}

#[test]
fn arithmetic_stack_bit_and_block_encodings() {
    let source = "add hl,bc\n add ix,ix\n adc hl,sp\n sbc hl,de\n add a,c\n adc a,(ix-128)\n sub (hl)\n sbc a,7\n and a,$ff\n xor (iy+127)\n or ixh\n cp 42\n inc iy\n dec (iy-1)\n push af\n pop ix\n ex (sp),iy\n ex de,hl\n ex af,af'\n bit 7,(ix+1)\n res 0,b\n set 3,(iy-2),c\n rlc (ix+2),a\n sra (hl)\n rld\n ldir\n cpdr\n inir\n otdr\n";
    assert_eq!(
        assemble(source, 0x6000, None).unwrap().bytes,
        [
            9, 0xdd, 0x29, 0xed, 0x7a, 0xed, 0x52, 0x81, 0xdd, 0x8e, 0x80, 0x96, 0xde, 7, 0xe6,
            255, 0xfd, 0xae, 127, 0xdd, 0xb4, 0xfe, 42, 0xfd, 0x23, 0xfd, 0x35, 255, 0xf5, 0xdd,
            0xe1, 0xfd, 0xe3, 0xeb, 8, 0xdd, 0xcb, 1, 0x7e, 0xcb, 0x80, 0xfd, 0xcb, 254, 0xd9,
            0xdd, 0xcb, 2, 7, 0xcb, 0x2e, 0xed, 0x6f, 0xed, 0xb0, 0xed, 0xb9, 0xed, 0xb2, 0xed,
            0xbb
        ]
    );
}

#[test]
fn forward_labels_expressions_and_source_maps_exclude_data_and_gaps() {
    assert_eq!(
        assemble("DB 7\nDS 40959\nEND\nignored: unknown", 24576, None)
            .unwrap()
            .bytes
            .len(),
        40960
    );
    assert_eq!(
        assemble("ALIGN boundary\nboundary EQU 16\nRET", 0x6001, None)
            .unwrap()
            .lines[0]
            .address,
        0x6010
    );
    let source = "; header\norg 24576\n length equ finish-start\n size equ length+1\n start: jr nz,finish\n ld hl,(2+3)*4\n db \"a;b\",'Z',<start,>start\n dw finish,-1\n ds size-size,0\n finish: ret\n";
    let result = assemble(source, 24576, Some("finish")).unwrap();
    assert_eq!(result.symbols["LENGTH"], 15);
    assert_eq!(result.symbols["SIZE"], 16);
    assert_eq!(
        result.bytes,
        [
            0x20, 13, 0x21, 20, 0, b'a', b';', b'b', b'Z', 0, 0x60, 15, 0x60, 255, 255, 0xc9
        ]
    );
    assert_eq!(result.entry_address, 0x600f);
    assert_eq!(
        result
            .lines
            .iter()
            .map(|l| (l.source_line, l.address, l.bytes))
            .collect::<Vec<_>>(),
        [(5, 0x6000, 2), (6, 0x6002, 3), (10, 0x600f, 1)]
    );
    assert_eq!(
        assemble("db 0\norg $6010\nentry: nop", 0x6000, Some("entry"))
            .unwrap()
            .bytes
            .len(),
        17
    );
}

#[test]
fn io_conditions_and_interrupt_encodings() {
    let source = "jp nz,$1234\n call pe,$5678\n ret c\n jp (ix)\n djnz $\n rst $38\n in a,($fe)\n out ($fe),a\n in b,(c)\n in (c)\n out (c),0\n im 0\n im 1\n im 2\n reti\n retn\n";
    assert_eq!(
        assemble(source, 0x6000, None).unwrap().bytes,
        [
            0xc2, 0x34, 0x12, 0xec, 0x78, 0x56, 0xd8, 0xdd, 0xe9, 0x10, 0xfe, 0xff, 0xdb, 0xfe,
            0xd3, 0xfe, 0xed, 0x40, 0xed, 0x70, 0xed, 0x71, 0xed, 0x46, 0xed, 0x56, 0xed, 0x5e,
            0xed, 0x4d, 0xed, 0x45
        ]
    );
}

#[test]
fn malformed_unsafe_and_out_of_range_source_is_rejected_without_output() {
    for source in [
        "",
        "include \"file.asm\"",
        "device zxspectrum48",
        "lua\n",
        "macro x",
        "jp missing",
        "A equ B\nB equ A\nnop",
        "A equ A+1\nnop",
        "x: nop\nX: nop",
        "db 256",
        "dw 65536",
        "ds 65537",
        "ds -1",
        "org 0\nnop",
        "ld a,(ix+128)",
        "ld (ix-129),1",
        "ld ix,iy",
        "ld ixh,iyl",
        "ld h,ixl",
        "ld ixh,(hl)",
        "ld (hl),(hl)",
        "ld (ix),ixh",
        "bit 8,a",
        "bit -1,a",
        "ld a,1/0",
        "ld a,1<<64",
        "ld a,9223372036854775807+1",
        "jp (bc)",
        "out (7),b",
        "rst 7",
        "im 3",
        "push sp",
        "jr po,$",
        "jr $+130",
        "jr $-127",
        "nop 1",
        "db \"bad",
        "ld a,(ix+(1)",
        "db 1,,2",
        "db \"a\"b\"",
        "ld a,1\0",
        "ld a,λ",
    ] {
        assert!(
            assemble(source, 0x6000, None).is_err(),
            "accepted {source:?}"
        );
    }
    assert!(assemble("data: db 1", 0x6000, Some("data")).is_err());
    assert!(
        assemble(
            &format!("ld a,{}1{}", "(".repeat(65), ")".repeat(65)),
            0x6000,
            None
        )
        .is_err()
    );
    assert!(assemble(&"nop\n".repeat(32769), 0x6000, None).is_err());
    assert!(assemble("ds 40961", 0x6000, None).is_err());
}

#[test]
fn address_boundaries_and_relative_wrap_are_checked() {
    assert_eq!(
        assemble("last: nop", 0xffff, Some("last")).unwrap().bytes,
        [0]
    );
    assert!(assemble("nop\nnop", 0xffff, None).is_err());
    assert_eq!(assemble("jr 0", 0xfffe, None).unwrap().bytes, [0x18, 0]);
    assert_eq!(
        assemble("jr $+129", 0x6000, None).unwrap().bytes,
        [0x18, 127]
    );
    assert_eq!(
        assemble("jr $-126", 0x6000, None).unwrap().bytes,
        [0x18, 128]
    );
    assert_eq!(
        assemble("value equ 4\nld a,12/value", 0x6000, None)
            .unwrap()
            .bytes,
        [0x3e, 3]
    );
    assert_eq!(
        assemble("ld a,12/value\nvalue equ 4", 0x6000, None)
            .unwrap()
            .bytes,
        [0x3e, 3]
    );
}
