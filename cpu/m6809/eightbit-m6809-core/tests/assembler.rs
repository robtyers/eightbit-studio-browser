//! Original source fixtures and encoding facts; no external assembler required.
#![allow(clippy::unwrap_used, clippy::panic)]
use eightbit_m6809_core::{
    Bus, Cpu,
    assembler::{MAX_SOURCE_BYTES, assemble},
};

fn bytes(text: &str) -> Vec<u8> {
    assemble(&format!("START {text}\n"), 0x7000, "START")
        .unwrap()
        .bytes
}

#[test]
fn instruction_corpus_assembles_to_instructions_accepted_by_the_cpu() {
    struct Ram(Vec<u8>);
    impl Bus for Ram {
        fn read(&mut self, address: u16) -> u8 {
            self.0[usize::from(address)]
        }
        fn write(&mut self, address: u16, value: u8) {
            self.0[usize::from(address)] = value;
        }
        fn tick(&mut self, _: u32) {}
    }
    let mut count = 0;
    for source in include_str!("fixtures/assembler-instructions.txt")
        .lines()
        .filter(|line| !line.starts_with(';') && !line.is_empty())
    {
        let result = bytes(source);
        let mut ram = Ram(vec![0; 65_536]);
        ram.0[0x7000..0x7000 + result.len()].copy_from_slice(&result);
        let mut cpu = Cpu::default();
        cpu.registers.pc = 0x7000;
        cpu.registers.s = 0x6000;
        cpu.registers.u = 0x5000;
        cpu.registers.x = 0x4000;
        cpu.registers.y = 0x3000;
        cpu.step(&mut ram)
            .unwrap_or_else(|failure| panic!("{source}: {failure}"));
        count += 1;
    }
    assert_eq!(count, 636);
}

#[test]
fn arbitrary_ascii_operands_fail_totally_and_within_fixed_work_limits() {
    let mut seed = 0x6809_u32;
    for length in 0..128 {
        for _ in 0..16 {
            let mut source = String::from("START LDA ");
            for _ in 0..length {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                source.push(char::from(32 + u8::try_from((seed >> 16) % 95).unwrap()));
            }
            let _ = assemble(&source, 0x7000, "START");
        }
    }
}

#[test]
fn address_arithmetic_rejects_extremes_and_wraps_pc_relative_targets() {
    assert_eq!(
        assemble("START LDA TARGET\n ORG 2\nTARGET RTS\n", 0, "START")
            .unwrap()
            .bytes,
        [0x96, 2, 0x39]
    );
    assert_eq!(
        assemble("START LDA $0000,PCR\n", 0xff00, "START")
            .unwrap()
            .bytes,
        [0xa6, 0x8d, 0, 0xfc]
    );
    assert_eq!(
        assemble("START LDA $0000,PCR\n", 0xfff0, "START")
            .unwrap()
            .bytes,
        [0xa6, 0x8c, 0x0d]
    );
    for expression in [
        "-9223372036854775807-1",
        "9223372036854775807",
        "-1",
        "65536",
    ] {
        for instruction in [
            format!("LDA {expression},PCR"),
            format!("NOP\n ORG {expression}"),
        ] {
            assert!(
                assemble(&format!("START {instruction}\n"), 0x7000, "START").is_err(),
                "{instruction}"
            );
        }
    }
    assert_eq!(
        assemble("START LDA >$FFFF,PCR\n", 0, "START")
            .unwrap()
            .bytes,
        [0xa6, 0x8d, 0xff, 0xfb]
    );
    assert_eq!(
        assemble("START LDA <$0000,PCR\n", 0xfffd, "START")
            .unwrap()
            .bytes,
        [0xa6, 0x8c, 0]
    );
    assert_eq!(
        assemble(
            "START LDA FORWARD\n ORG $7002\nFORWARD EQU $42\n",
            0x7000,
            "START"
        )
        .unwrap()
        .bytes,
        [0x96, 0x42]
    );
    assert!(assemble("START LDA >$42\n ORG $7002\n", 0x7000, "START").is_err());
}

#[test]
fn assembled_programme_executes_on_the_cpu_with_forward_labels_and_indexed_writes() {
    struct Ram(Vec<u8>);
    impl Bus for Ram {
        fn read(&mut self, address: u16) -> u8 {
            self.0[usize::from(address)]
        }
        fn write(&mut self, address: u16, value: u8) {
            self.0[usize::from(address)] = value;
        }
        fn tick(&mut self, _: u32) {}
    }
    let source = "        section code\n        export START\nSTART   lds #$7f00\n        ldx #$6000\n        lda #42\n        sta ,x+\n        ldb #5\n        adda #3\n        std ,x\n        bra done\n        fcb 255\nDONE    nop\n        endsection\n";
    let result = assemble(source, 0x7000, "start").unwrap();
    let mut ram = Ram(vec![0; 65536]);
    ram.0[0x7000..0x7000 + result.bytes.len()].copy_from_slice(&result.bytes);
    let mut cpu = Cpu::default();
    cpu.registers.pc = result.entry_address;
    for _ in 0..9 {
        cpu.step(&mut ram).unwrap();
    }
    assert_eq!(&ram.0[0x6000..0x6003], &[42, 45, 5]);
    assert_eq!(cpu.registers.x, 0x6001);
    assert_eq!(cpu.registers.d(), 0x2d05);
    assert_eq!(result.lines.last().unwrap().source_line, 12);
    assert_eq!(cpu.registers.pc, result.symbols["DONE"] + 1);
}

#[test]
fn prefixes_memory_modes_aliases_and_immediate_widths_match_documented_bytes() {
    for (source, expected) in [
        ("LDA #$ff", vec![0x86, 0xff]),
        ("LDA <$12", vec![0x96, 0x12]),
        ("LDA >$12", vec![0xb6, 0, 0x12]),
        ("LDA ,X", vec![0xa6, 0x84]),
        ("LDY #$1234", vec![0x10, 0x8e, 0x12, 0x34]),
        ("STY $1234", vec![0x10, 0xbf, 0x12, 0x34]),
        ("LDS #$7fff", vec![0x10, 0xce, 0x7f, 0xff]),
        ("STS ,S", vec![0x10, 0xef, 0xe4]),
        ("CMPU #$1234", vec![0x11, 0x83, 0x12, 0x34]),
        ("CMPS <$ff", vec![0x11, 0x9c, 0xff]),
        ("CMPD ,U", vec![0x10, 0xa3, 0xc4]),
        ("STD $6000", vec![0xfd, 0x60, 0]),
        ("JSR $8000", vec![0xbd, 0x80, 0]),
        ("NEG <$12", vec![0, 0x12]),
        ("JMP [$1234]", vec![0x6e, 0x9f, 0x12, 0x34]),
        ("LSLA", vec![0x48]),
        ("LSLB", vec![0x58]),
        ("LSL ,X", vec![0x68, 0x84]),
        ("SWI2", vec![0x10, 0x3f]),
        ("SWI3", vec![0x11, 0x3f]),
        ("ORCC #$50", vec![0x1a, 0x50]),
        ("TFR D,X", vec![0x1f, 0x01]),
        ("EXG A,B", vec![0x1e, 0x89]),
        ("PSHS CC,D,DP,X,Y,U,PC", vec![0x34, 0xff]),
        ("PULU CC,D,DP,X,Y,S,PC", vec![0x37, 0xff]),
    ] {
        assert_eq!(bytes(source), expected, "{source}");
    }
}

#[test]
fn every_documented_indexed_form_and_forced_width_is_available() {
    assert_eq!(bytes("LDA ','"), [0x96, 0x2c]);
    assert_eq!(bytes("LDA [',']"), [0xa6, 0x9f, 0, 0x2c]);
    assert_eq!(bytes("LDA ',',X"), [0xa6, 0x88, 0x2c]);
    assert_eq!(bytes("LDA [']']"), [0xa6, 0x9f, 0, 0x5d]);
    assert_eq!(bytes("LDA ']',X"), [0xa6, 0x88, 0x5d]);
    for operand in [">,X", "<,X", "<<,X", ",X,Y", "[[$1234]]", "[[,X]]"] {
        assert!(assemble(&format!("START LDA {operand}\n"), 0x7000, "START").is_err());
    }
    for (operand, expected) in [
        (",X", vec![0x84]),
        (",Y+", vec![0xa0]),
        (",U++", vec![0xc1]),
        (",-S", vec![0xe2]),
        (",--X", vec![0x83]),
        ("[,X]", vec![0x94]),
        ("[,Y++]", vec![0xb1]),
        ("[,--U]", vec![0xd3]),
        ("A,X", vec![0x86]),
        ("B,Y", vec![0xa5]),
        ("D,U", vec![0xcb]),
        ("[D,S]", vec![0xfb]),
        ("-16,X", vec![0x10]),
        ("15,S", vec![0x6f]),
        ("<15,X", vec![0x88, 15]),
        (">15,X", vec![0x89, 0, 15]),
        ("<<0,Y", vec![0x20]),
        ("128,U", vec![0xc9, 0, 128]),
        ("[-128,Y]", vec![0xb8, 128]),
        ("[>$1234]", vec![0x9f, 0x12, 0x34]),
        ("<*+3,PCR", vec![0x8c, 0]),
        (">*+4,PCR", vec![0x8d, 0, 0]),
        ("[<*+3,PCR]", vec![0x9c, 0]),
        ("[>*+4,PCR]", vec![0x9d, 0, 0]),
    ] {
        let mut expected = expected;
        expected.insert(0, 0xa6);
        assert_eq!(bytes(&format!("LDA {operand}")), expected, "{operand}");
    }
}

#[test]
fn branch_targets_use_instruction_end_and_support_signed_boundaries() {
    assert_eq!(bytes("BRA *"), vec![0x20, 0xfe]);
    assert_eq!(bytes("BHS *+129"), vec![0x24, 127]);
    assert_eq!(bytes("BLO *-126"), vec![0x25, 128]);
    assert_eq!(bytes("LBRA *"), vec![0x16, 0xff, 0xfd]);
    assert_eq!(bytes("LBEQ *"), vec![0x10, 0x27, 0xff, 0xfc]);
    assert_eq!(bytes("LBSR *+300"), vec![0x17, 1, 41]);
    let error = assemble("START BRA FAR\n RMB 128\nFAR RTS\n", 0x7000, "START").unwrap_err();
    assert_eq!(error.line, 1);
    assert!(error.message.contains("Short branch"));
}

#[test]
fn directives_expressions_case_and_forward_constants_are_deterministic() {
    assert_eq!(
        bytes("FCB ',', ';', 'A, \"A,B;C\""),
        vec![44, 59, 65, 65, 44, 66, 59, 67]
    );
    let source = "PAGE equ $12\nVALUE equ FINAL+1\nSTART lda #VALUE\n setdp PAGE\n sta $1234\n fcb %1010,@17,'A',<($1234+2),>($1234+2)\n fdb $1234,-1\n fcc \"A;B\\n\" ; comment\n rmb 2\n fill $a5,2\n align 16\nFINAL equ 41\n end START\n";
    let result = assemble(source, 0x7000, "START").unwrap();
    assert_eq!(
        &result.bytes[..18],
        &[
            0x86, 42, 0x97, 0x34, 10, 15, 65, 0x36, 0x12, 0x12, 0x34, 0xff, 0xff, 65, 59, 66, 10, 0
        ]
    );
    assert_eq!(result.bytes.len() % 16, 0);
    assert_eq!(result, assemble(source, 0x7000, "START").unwrap());
    assert_eq!(bytes("LDA #((2+3)*4|1)"), vec![0x86, 21]);
    assert_eq!(bytes("LDA #(~0&255)"), vec![0x86, 255]);
    assert_eq!(bytes("LDA #(1<<7)"), vec![0x86, 128]);
}

#[test]
fn malformed_sources_have_actionable_errors_instead_of_partial_output() {
    for source in [
        "START LDA #256",
        "START LDX #65536",
        "START TFR A,X",
        "START PSHS S",
        "START STA #1",
        "START LDA [,X+]",
        "START LDA [<<1,X]",
        "START LDA 1,X++",
        "START LEAX #1",
        "START NOP 1",
        "START LDA MISSING",
        "START EQU OTHER\nOTHER EQU START",
        "START NOP\nSTART RTS",
        "START NOP\n org $6000",
        "START NOP\n fill 0,-1",
        "START LDA #(1/0)",
        "START LDA #(1+)",
        "START LDA #$GG",
        "START LDA #9223372036854775808",
        "START LDA #((1)",
        "START LDA #1 trailing",
        "START FCB \"unfinished",
        "START INCLUDE \"anything\"",
        "START MACRO",
        "START NOP\n endsection",
        " section code\nSTART NOP",
        "START NOP\n end\n RTS",
        "START NOP\n export MISSING",
        "START LDA \0",
        "START FCC \"é\"",
    ] {
        assert!(assemble(source, 0x7000, "START").is_err(), "{source}");
    }
    assert!(assemble("START NOP", 0x7000, "ABSENT").is_err());
    assert!(assemble("START RMB 2", 0xffff, "START").is_err());
    assert_eq!(
        assemble("START FCB 1", 0xffff, "START").unwrap().bytes,
        vec![1]
    );
    assert!(assemble(&" ".repeat(MAX_SOURCE_BYTES + 1), 0x7000, "START").is_err());
    assert!(
        assemble(
            &format!("START LDA #{}1{}", "(".repeat(40), ")".repeat(40)),
            0x7000,
            "START"
        )
        .is_err()
    );
}

#[test]
fn raw_images_need_no_synthetic_entry_label_and_fcs_marks_the_final_byte() {
    use eightbit_m6809_core::assembler::assemble_image;
    let image = assemble_image(
        " org 0\n__VXO_CARTRIDGE_START: fcs \"ABC\"\n nop\n end __VXO_CARTRIDGE_START\n",
        0,
    )
    .unwrap();
    assert_eq!(image.bytes, [65, 66, 195, 0x12]);
    assert_eq!(image.lines[0].source_line, 3);
    assert!(assemble_image(" fcs \"\"\n", 0).is_err());
    assert!(assemble_image(" nop\n end MISSING\n", 0).is_err());
}
