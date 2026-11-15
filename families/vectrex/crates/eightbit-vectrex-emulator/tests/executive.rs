//! Independently authored cartridges test documented interface facts, not ROM bytes.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_m6809_core::{Bus, Registers};
use eightbit_vectrex_emulator::{cartridge, firmware, machine::Machine};
use std::collections::BTreeMap;

const HEADER: &str = " org 0\n fcc \"g GCE 2026\"\n fcb $80\n fdb 0\n fcb $F8,$50,$20,$C0\n fcc \"ABI\"\n fcb $80,0\n";
struct Guest {
    machine: Machine,
    symbols: BTreeMap<String, u16>,
}
impl Guest {
    fn new(body: &str, data: &str) -> Self {
        let a = cartridge::assemble(&format!("{HEADER}start: {body}\ndone: bra done\n{data}\n"))
            .unwrap();
        let mut machine = Machine::new(a.bytes).unwrap();
        machine.cpu.registers = Registers {
            pc: a.symbols["START"],
            s: 0xcbf0,
            dp: 0xd0,
            cc: 0x51,
            x: 0xc940,
            y: 0x1234,
            u: 0xc980,
            a: 0,
            b: 0,
        };
        for (address, value) in [
            (0xd002, 0x9f),
            (0xd003, 255),
            (0xd000, 0x81),
            (0xd00b, 0x80),
            (0xd00c, 0xce),
            (0xd004, 63),
        ] {
            machine.board.write(address, value);
        }
        Self {
            machine,
            symbols: a.symbols,
        }
    }
    fn run(&mut self) {
        self.machine.cpu.registers.pc = self.symbols["START"];
        let until = self.machine.board.cycles + 4_000_000;
        while self.machine.cpu.registers.pc != self.symbols["DONE"] {
            assert!(
                self.machine.board.cycles < until,
                "guest service failed to return"
            );
            self.machine.cpu.step(&mut self.machine.board).unwrap();
        }
        assert_eq!(
            self.machine.cpu.registers.s, 0xcbf0,
            "service leaked stack space"
        );
    }
    fn byte(&self, a: u16) -> u8 {
        self.machine.board.peek(a)
    }
    fn word(&self, a: u16) -> u16 {
        u16::from_be_bytes([self.byte(a), self.byte(a + 1)])
    }
}

#[test]
fn dp_intensity_and_zero_return_registers_and_public_mirror() {
    for (entry, a) in [(0xf1aa, 0xd0), (0xf1af, 0xc8)] {
        let mut g = Guest::new(&format!("jsr ${entry:04X}"), "");
        g.machine.cpu.registers.b = 0xab;
        g.run();
        let r = g.machine.cpu.registers;
        assert_eq!(
            (r.a, r.b, r.dp, r.x, r.y, r.u),
            (a, 0xab, a, 0xc940, 0x1234, 0xc980)
        );
        assert_eq!(
            r.cc & 0xf1,
            0x51,
            "documented instructions preserve mask/carry bits"
        );
    }
    for (entry, level) in [
        (0xf29d, 31),
        (0xf2a1, 63),
        (0xf2a5, 95),
        (0xf2a9, 127),
        (0xf2ab, 44),
    ] {
        let mut g = Guest::new(&format!("lda #44\n jsr ${entry:04X}"), "");
        g.run();
        let r = g.machine.cpu.registers;
        assert_eq!(g.byte(0xc827), level);
        assert_eq!(
            (r.d(), r.x, r.y, r.u, r.dp),
            (0x0501, 0xc940, 0x1234, 0xc980, 0xd0)
        );
        assert_eq!(r.cc & 0xf1, 0x51);
    }
    let mut g = Guest::new("jsr $F354", "");
    g.run();
    assert_eq!(g.machine.cpu.registers.d(), 0x0301);
}

#[test]
fn wait_uses_public_frame_interval_and_increments_the_big_endian_counter() {
    let mut g = Guest::new("jsr $F192", "");
    for (a, v) in [
        (0xc825, 0x12),
        (0xc826, 255),
        (0xc83d, 255),
        (0xc83e, 1),
        (0xd008, 0),
        (0xd009, 0),
    ] {
        g.machine.board.write(a, v);
    }
    g.machine.cpu.registers.dp = 0xc8;
    g.run();
    assert_eq!(g.word(0xc825), 0x1300);
    let r = g.machine.cpu.registers;
    assert_eq!(
        (r.d(), r.dp, r.x, r.y, r.u),
        (0x0301, 0xd0, 0xf9f4, 0x1234, 0xc980)
    );
    let remaining =
        u16::from_le_bytes([g.machine.board.read(0xd008), g.machine.board.read(0xd009)]);
    assert!(
        (300..=511).contains(&remaining),
        "public interval was not loaded: {remaining}"
    );
}

#[test]
fn sound_writes_each_documented_mirror_without_touching_adjacent_ram() {
    for address in 0_u8..14 {
        let mut g = Guest::new(&format!("lda #{address}\n ldb #90\n jsr $F256"), "");
        for a in 0xc800..0xc80f {
            g.machine.board.write(a, 0xa5);
        }
        g.run();
        for a in 0xc800..0xc80f {
            assert_eq!(
                g.byte(a),
                if a == 0xc800 + u16::from(address) {
                    90
                } else {
                    0xa5
                }
            );
        }
        let r = g.machine.cpu.registers;
        assert_eq!((r.x, r.b, r.y, r.u), (0xc800, 1, 0x1234, 0xc980));
    }
}

#[test]
fn button_transition_return_and_eight_key_bytes_cover_both_ports_and_repeated_reads() {
    let mut g = Guest::new("jsr $F1BA", "");
    g.machine.board.controllers[0].buttons = 5;
    g.machine.board.controllers[1].buttons = 10;
    for expected in [0xa5, 0] {
        g.run();
        assert_eq!((g.byte(0xc80f), g.byte(0xc811)), (0xa5, expected));
        for bit in 0..8 {
            assert_eq!(g.byte(0xc812 + bit), expected & (1 << bit));
        }
        let r = g.machine.cpu.registers;
        assert_eq!(
            (r.a, r.b, r.x, r.y, r.u),
            (expected, 0, 0xc81a, 0x1234, 0xc980)
        );
    }
    g.machine.board.controllers[0].buttons = 0;
    g.machine.board.controllers[1].buttons = 0;
    g.run();
    assert_eq!(g.byte(0xc810), 0xa5);
    assert_eq!(g.byte(0xc811), 0);
    g.machine.board.controllers[1].buttons = 8;
    g.run();
    assert_eq!(g.machine.cpu.registers.a, 0x80);
}

#[test]
fn counted_draw_and_move_lists_advance_exactly_and_consume_the_public_count() {
    for (entry, prefix, setup) in [
        (0xf3ce, "fcb 2", ""),
        (0xf3d6, "fcb 2,23", ""),
        (0xf3d2, "", "ldb #23"),
        (0xf3d8, "", "lda #2\n ldb #23"),
        (0xf3da, "", "lda #2"),
        (0xf3dd, "", ""),
        (0xf3ad, "fcb 2", ""),
        (0xf3b5, "fcb 2,23", ""),
        (0xf3b1, "", "ldb #23"),
        (0xf3b7, "", "lda #2\n ldb #23"),
        (0xf3b9, "", "lda #2"),
        (0xf3bc, "", ""),
    ] {
        let mut g = Guest::new(
            &format!("ldx #list\n {setup}\n jsr ${entry:04X}"),
            &format!("list: {prefix}\n fcb 12,12,0,24,-12,0\nafter: fcb 0"),
        );
        g.machine.board.write(0xc823, 2);
        g.run();
        assert_eq!(
            g.machine.cpu.registers.x, g.symbols["AFTER"],
            "entry {entry:04x}"
        );
        assert_eq!(g.byte(0xc823), 0);
        assert_eq!(g.machine.cpu.registers.y, 0x1234);
        assert_eq!(g.machine.cpu.registers.u, 0xc980);
        if matches!(entry, 0xf3d6 | 0xf3d2 | 0xf3d8 | 0xf3b5 | 0xf3b1 | 0xf3b7) {
            assert_eq!(g.machine.board.read(0xd006), 23);
        }
    }
    for entry in [0xf3df, 0xf3be] {
        let mut g = Guest::new(&format!("ldd #$1020\n jsr ${entry:04X}"), "");
        g.run();
        assert_eq!(g.machine.cpu.registers.x, 0xc942);
        assert_eq!(g.byte(0xc823), 0);
    }
}

#[test]
fn text_entries_advance_u_preserve_y_and_stack_and_publish_message_and_size() {
    for (entry, prefix, setup) in [
        (0xf373, "fcb -8,80,24,-32", ""),
        (0xf378, "fcb 24,-32", ""),
        (0xf37a, "", "ldd #$18E0"),
        (0xf495, "", ""),
    ] {
        let mut g = Guest::new(
            &format!("ldu #text\n {setup}\n jsr ${entry:04X}"),
            &format!("text: {prefix}\nmessage: fcc \"A0!\"\n fcb $80\nafter: fcb 0"),
        );
        g.machine.board.write(0xc82a, 0xf8);
        g.machine.board.write(0xc82b, 80);
        g.run();
        let r = g.machine.cpu.registers;
        assert_eq!(
            (r.d(), r.x, r.y, r.u, r.dp),
            (0x0301, 0xfbb4, 0x1234, g.symbols["AFTER"], 0xd0)
        );
        assert_eq!(g.word(0xc82c), g.symbols["MESSAGE"]);
        assert_eq!(g.word(0xc82a), 0xf850);
        assert_eq!(
            g.machine.board.read(0xd006),
            if entry == 0xf495 { 63 } else { 127 }
        );
    }
}

#[test]
fn full_byte_list_counts_are_finite_and_zskip_controls_zeroing() {
    for count in [0_u8, 1, 127, 255] {
        let data = (0..=count).map(|_| " fcb 0,0\n").collect::<String>();
        let mut g = Guest::new(
            "ldx #list\n jsr $F3CE",
            &format!("list: fcb {count}\n{data}after: fcb 0"),
        );
        g.run();
        assert_eq!(g.machine.cpu.registers.x, g.symbols["AFTER"]);
        assert_eq!(g.byte(0xc823), 0);
    }
    for zero in [0, 1] {
        let mut g = Guest::new("ldd #$2020\n jsr $F3DF", "");
        g.machine.board.write(0xc824, zero);
        g.run();
        if zero == 1 {
            assert_eq!(g.machine.cpu.registers.d(), 0x0301);
        }
        assert_eq!(g.byte(0xc824), zero);
    }
}

#[test]
fn each_original_glyph_renders_and_space_is_blank_without_historical_font_data() {
    for code in 0x20_u8..=0x6f {
        let mut g = Guest::new(
            "ldu #message\n jsr $F495",
            &format!("message: fcb {code},$80"),
        );
        g.machine.board.write(0xc82a, 0xf0);
        g.machine.board.write(0xc82b, 80);
        // Intensity is a real VIA operation too, performed independently of text.
        g.machine.board.write(0xd001, 127);
        g.machine.board.write(0xd000, 0x84);
        g.machine.board.write(0xd000, 0x85);
        g.run();
        g.machine
            .board
            .tick(30_000 - u32::try_from(g.machine.board.cycles % 30_000).unwrap());
        let ink = g.machine.board.pixels().iter().filter(|&&p| p != 0).count();
        assert_eq!(ink == 0, code == 0x20, "glyph {code:02x} has {ink} pixels");
    }
}

#[test]
fn invalid_unterminated_text_fails_and_long_guest_work_can_be_cancelled() {
    for data in [
        "fcb $1F,$80".to_owned(),
        "fcb $70,$80".to_owned(),
        "fcb $FF,$80".to_owned(),
        format!("fcc \"{}\"\n fcb $80", " ".repeat(257)),
    ] {
        let mut g = Guest::new("ldu #message\n jsr $F495", &format!("message: {data}"));
        g.machine.board.write(0xc82a, 0xf0);
        g.machine.board.write(0xc82b, 80);
        let mut failure = None;
        for _ in 0..80 {
            match g.machine.advance(120_000, || false) {
                Ok(()) => {}
                Err(e) => {
                    failure = Some(e);
                    break;
                }
            }
        }
        assert_eq!(failure.unwrap().code, "VXO1001");
    }
    let mut g = Guest::new(
        "ldu #message\n jsr $F495",
        &format!("message: fcc \"{}\"\n fcb $80", "A".repeat(256)),
    );
    let mut polls = 0;
    assert_eq!(
        g.machine
            .advance(120_000, || {
                polls += 1;
                polls == 100
            })
            .unwrap_err()
            .code,
        "VXO1002"
    );
    assert!(g.machine.board.cycles < 120_000);
    // Cancellation leaves a resumable instruction boundary, rather than a host trap.
    let until = g.machine.board.cycles + 4_000_000;
    while g.machine.cpu.registers.pc != g.symbols["DONE"] {
        assert!(g.machine.board.cycles < until);
        g.machine.cpu.step(&mut g.machine.board).unwrap();
    }
    assert_eq!(g.machine.cpu.registers.s, 0xcbf0);
    assert_eq!(g.machine.cpu.registers.u, g.symbols["MESSAGE"] + 257);
}

#[test]
fn original_executive_cartridge_boots_and_reset_stop_recover_the_bounded_session() {
    use eightbit_vectrex_emulator::{
        digest,
        session::{Command, Session},
    };
    let source = include_str!("../../../templates/executive-demo.asm");
    let cart = cartridge::assemble(source).unwrap().bytes;
    // Independently observed LWTOOLS 4.25 output for this authored cartridge.
    assert_eq!(cart.len(), 99);
    assert_eq!(
        digest(&cart),
        "3269f5f3913bfb3d2d319a1fb6712a5a0ddc369aa35079953ae0edfaf5195cdb"
    );
    let mut s = Session::new(23, &cart, &digest(&cart), || false).unwrap();
    let mut lit = false;
    for _ in 0..5 {
        let f = s.command(Command::Advance { cycles: 30_000 }).unwrap();
        lit |= f.pixels.chunks_exact(4).filter(|p| p[0] != 0).count() > 30;
    }
    assert!(lit);
    s.command(Command::Stop {}).unwrap();
    assert!(s.command(Command::Advance { cycles: 30_000 }).is_err());
    s.command(Command::Reset {}).unwrap();
    assert!(s.command(Command::Advance { cycles: 30_000 }).is_ok());
}

#[test]
fn a_guest_text_fault_releases_controls_and_reset_recovers() {
    use eightbit_vectrex_emulator::{
        digest,
        session::{Command, Session},
    };
    let source = format!(
        "{HEADER}start: jsr $F1BA\n lda $C80F\n bita #1\n beq start\n ldu #message\n jsr $F495\n bra start\nmessage: fcb $1F,$80\n"
    );
    let cart = cartridge::assemble(&source).unwrap().bytes;
    let mut s = Session::new(12, &cart, &digest(&cart), || false).unwrap();
    s.command(Command::Key {
        code: "KeyZ".into(),
        pressed: true,
    })
    .unwrap();
    assert_eq!(
        s.command(Command::Advance { cycles: 30_000 })
            .err()
            .unwrap()
            .code,
        "VXO1001"
    );
    assert_eq!(s.machine().board.controllers[0].buttons, 0);
    assert_eq!(s.command(Command::State {}).unwrap().state, "failed");
    s.command(Command::Reset {}).unwrap();
    assert_eq!(
        s.command(Command::Advance { cycles: 30_000 })
            .unwrap()
            .state,
        "running"
    );
}

#[test]
fn list_pixels_match_authored_line_and_move_sequences_without_extra_ram() {
    for (entry, first) in [(0xf3ce, 0xf3df), (0xf3ad, 0xf312)] {
        let mut listed = Guest::new(
            &format!("jsr $F2A9\n ldx #list\n jsr ${entry:04X}"),
            "list: fcb 2,12,12,0,24,-12,0",
        );
        let mut individual = Guest::new(
            &format!(
                "jsr $F2A9\n ldd #$0C0C\n jsr ${first:04X}\n ldd #$0018\n jsr $F3DF\n ldd #$F400\n jsr $F3DF"
            ),
            "",
        );
        listed.run();
        individual.run();
        for guest in [&mut listed, &mut individual] {
            guest
                .machine
                .board
                .tick(30_000 - u32::try_from(guest.machine.board.cycles % 30_000).unwrap());
        }
        assert!(listed.machine.board.pixels().iter().any(|&p| p != 0));
        assert_eq!(
            listed.machine.board.pixels(),
            individual.machine.board.pixels()
        );
        for address in 0xc800..0xcc00 {
            if !matches!(address, 0xc823 | 0xc827 | 0xcb80..=0xcbef) {
                assert_eq!(
                    listed.byte(address),
                    0,
                    "unexpected RAM write {address:04x}"
                );
            }
        }
    }
}

#[test]
fn digital_joysticks_preserve_caller_registers_and_honour_each_enable() {
    let mut g = Guest::new("jsr $F1F8", "");
    g.machine.board.controllers[0].x = -127;
    g.machine.board.controllers[0].y = 127;
    g.machine.board.controllers[1].x = 127;
    g.machine.board.controllers[1].y = -127;
    for (index, enable) in [1, 3, 5, 7].into_iter().enumerate() {
        g.machine
            .board
            .write(0xc81f + u16::try_from(index).unwrap(), enable);
    }
    let before = g.machine.cpu.registers;
    g.run();
    assert_eq!(
        (0xc81b..0xc81f).map(|a| g.byte(a)).collect::<Vec<_>>(),
        [255, 1, 1, 255]
    );
    let after = g.machine.cpu.registers;
    assert_eq!(
        (after.d(), after.x, after.y, after.u, after.dp),
        (before.d(), before.x, before.y, before.u, before.dp)
    );
    for a in 0xc81f..0xc823 {
        g.machine.board.write(a, 0);
    }
    g.run();
    assert!((0xc81b..0xc81f).all(|a| g.byte(a) == 0));
}

#[test]
fn text_extreme_signed_sizes_use_only_stack_and_published_text_ram() {
    for (height, width) in [(-128, -128), (-128, 127), (127, -128), (0, 0)] {
        let mut g = Guest::new(
            "ldu #message\n jsr $F373",
            &format!("message: fcb {height},{width},0,0\n fcc \"AB\"\n fcb $80\nafter: fcb 0"),
        );
        g.run();
        assert_eq!(g.machine.cpu.registers.u, g.symbols["AFTER"]);
        assert_eq!(g.machine.cpu.registers.y, 0x1234);
        for address in 0xc800..0xcc00 {
            if !matches!(address, 0xc82a..=0xc82d | 0xcb80..=0xcbef) {
                assert_eq!(
                    g.byte(address),
                    0,
                    "unexpected text RAM write {address:04x}"
                );
            }
        }
    }
}

#[test]
fn documented_unimplemented_service_entries_remain_explicit_guest_faults() {
    // Public interface addresses, not ROM implementation bytes. New jump stubs
    // must not accidentally occupy one of these still-unimplemented services.
    let entries = [
        0xe98a_u16, 0xea6d, 0xeaf0, 0xf06c, 0xf14c, 0xf164, 0xf18b, 0xf1a2, 0xf1b4, 0xf1f5, 0xf259,
        0xf25b, 0xf272, 0xf27d, 0xf284, 0xf289, 0xf28c, 0xf2be, 0xf2c1, 0xf2c3, 0xf2c5, 0xf2d5,
        0xf2de, 0xf2e6, 0xf2f2, 0xf2fc, 0xf308, 0xf30c, 0xf30e, 0xf310, 0xf34a, 0xf34f, 0xf35b,
        0xf36b, 0xf38a, 0xf391, 0xf393, 0xf433, 0xf439, 0xf498, 0xf511, 0xf517, 0xf533, 0xf53f,
        0xf542, 0xf545, 0xf548, 0xf550, 0xf552, 0xf55a, 0xf55e, 0xf563, 0xf56d, 0xf571, 0xf575,
        0xf579, 0xf57a, 0xf57d, 0xf57e, 0xf584, 0xf58b, 0xf593, 0xf5d9, 0xf5db, 0xf5ef, 0xf5ff,
        0xf601, 0xf603, 0xf610, 0xf616, 0xf61f, 0xf62b, 0xf65b, 0xf65d, 0xf661, 0xf663, 0xf67f,
        0xf683, 0xf687, 0xf68d, 0xf692, 0xf742, 0xf7a9, 0xf835, 0xf84f, 0xf85e, 0xf87c, 0xf8b7,
        0xf8c7, 0xf8d8, 0xf8e5, 0xf8f3, 0xf8ff, 0xf92e, 0xf9ca, 0xff9f,
    ];
    let image = firmware::image().unwrap();
    for entry in entries {
        assert_eq!(image[usize::from(entry - 0xe000)], 2, "service {entry:04x}");
        let mut g = Guest::new(&format!("jsr ${entry:04X}"), "");
        assert_eq!(
            g.machine.advance(120_000, || false).unwrap_err().code,
            "VXO1001"
        );
    }
}

#[test]
fn packet_entries_select_scale_and_leave_x_on_the_terminator_without_private_ram() {
    for (entry, prefix, scale) in [
        (0xf404, "", 255),
        (0xf408, "", 127),
        (0xf40c, "fcb 47", 47),
        (0xf40e, "", 39),
        (0xf410, "", 63),
    ] {
        for zero in [0, 1] {
            let mut g = Guest::new(
                &format!("ldx #list\n ldb #39\n jsr ${entry:04X}"),
                &format!("list: {prefix}\n fcb 0,-12,12,$FF,12,-12\nendlist: fcb 1,$FE"),
            );
            g.machine.board.write(0xc824, zero);
            g.machine.board.write(0xc823, 0xa5);
            g.run();
            let r = g.machine.cpu.registers;
            assert_eq!(
                (r.x, r.y, r.u, r.dp),
                (g.symbols["ENDLIST"], 0x1234, 0xc980, 0xd0)
            );
            assert_eq!(g.machine.board.read(0xd006), scale);
            if zero == 1 {
                assert_eq!(r.d(), 0x0301);
            }
            for address in 0xc800..0xcc00 {
                if !(0xcb80..=0xcbef).contains(&address) {
                    assert_eq!(
                        g.byte(address),
                        match address {
                            0xc823 => 0xa5,
                            0xc824 => zero,
                            _ => 0,
                        },
                        "packet wrote unrelated RAM {address:04x}"
                    );
                }
            }
        }
    }
}

#[test]
fn packet_pixels_match_individual_signed_moves_and_strokes_with_zskip() {
    for zero in [0, 1] {
        let mut listed = Guest::new(
            "jsr $F2A9\n ldx #list\n jsr $F410\n ldd #$0808\n jsr $F3DF",
            "list: fcb 0,-12,-12,$FF,0,24,$FF,24,0,0,-8,8,1",
        );
        let mut individual = Guest::new(
            &format!(
                "jsr $F2A9\n ldd #$F4F4\n jsr $F312\n ldd #$0018\n jsr $F3DF\n ldd #$1800\n jsr $F3DF\n ldd #$F808\n jsr $F312\n {}\n ldd #$0808\n jsr $F3DF",
                if zero == 1 { "jsr $F354" } else { "" }
            ),
            "",
        );
        listed.machine.board.write(0xc824, zero);
        // Individual strokes must not reset until the list's completion.
        listed.run();
        individual.run();
        for guest in [&mut listed, &mut individual] {
            guest
                .machine
                .board
                .tick(30_000 - u32::try_from(guest.machine.board.cycles % 30_000).unwrap());
        }
        assert!(listed.machine.board.pixels().iter().any(|&p| p != 0));
        assert_eq!(
            listed.machine.board.pixels(),
            individual.machine.board.pixels()
        );
    }
}

#[test]
fn packet_empty_and_maximum_lists_return_but_extra_vectors_and_modes_fault() {
    for count in [0, 1, 255, 256] {
        let data = " fcb 0,0,0\n".repeat(count);
        let mut g = Guest::new(
            "ldx #list\n jsr $F410",
            &format!("list:\n{data}endlist: fcb 1,$FE"),
        );
        g.run();
        assert_eq!(g.machine.cpu.registers.x, g.symbols["ENDLIST"]);
    }
    for mode in [2, 127, 128, 254] {
        let mut g = Guest::new("ldx #list\n jsr $F410", &format!("list: fcb {mode},0,0,1"));
        assert_eq!(
            g.machine.advance(120_000, || false).unwrap_err().code,
            "VXO1001"
        );
    }
    for suffix in [" fcb 1", ""] {
        let mut g = Guest::new(
            "ldx #list\n jsr $F410",
            &format!("list:\n{}{suffix}", " fcb 0,0,0\n".repeat(257)),
        );
        assert_eq!(
            g.machine.advance(120_000, || false).unwrap_err().code,
            "VXO1001"
        );
    }
    for (entry, pointer) in [
        (0xf40c, 0xffff),
        (0xf410, 0xfffd),
        (0xf410, 0xfffe),
        (0xf410, 0xffff),
    ] {
        let mut g = Guest::new(&format!("ldx #${pointer:04X}\n jsr ${entry:04X}"), "");
        assert_eq!(
            g.machine.advance(120_000, || false).unwrap_err().code,
            "VXO1001"
        );
        assert!(g.machine.cpu.registers.x >= pointer, "list pointer wrapped");
    }
}

#[test]
fn packet_work_cancels_at_an_instruction_boundary_and_resumes_with_balanced_stack() {
    let mut g = Guest::new(
        "ldx #list\n jsr $F404",
        &format!("list:\n{}endlist: fcb 1", " fcb $FF,0,0\n".repeat(256)),
    );
    let mut polls = 0;
    assert_eq!(
        g.machine
            .advance(120_000, || {
                polls += 1;
                polls == 100
            })
            .unwrap_err()
            .code,
        "VXO1002"
    );
    assert!(g.machine.board.cycles < 120_000);
    let until = g.machine.board.cycles + 4_000_000;
    while g.machine.cpu.registers.pc != g.symbols["DONE"] {
        assert!(g.machine.board.cycles < until);
        g.machine.cpu.step(&mut g.machine.board).unwrap();
    }
    assert_eq!(g.machine.cpu.registers.s, 0xcbf0);
    assert_eq!(g.machine.cpu.registers.x, g.symbols["ENDLIST"]);
}

#[test]
fn packet_fault_releases_controls_and_reset_stop_recover_the_session() {
    use eightbit_vectrex_emulator::{
        digest,
        session::{Command, Session},
    };
    let source = format!(
        "{HEADER}start: jsr $F1BA\n lda $C80F\n bita #1\n beq start\n ldx #list\n jsr $F410\n bra start\nlist: fcb 2,0,0,1"
    );
    let cart = cartridge::assemble(&source).unwrap().bytes;
    let mut s = Session::new(32, &cart, &digest(&cart), || false).unwrap();
    s.command(Command::Key {
        code: "KeyZ".into(),
        pressed: true,
    })
    .unwrap();
    assert_eq!(
        s.command(Command::Advance { cycles: 30_000 })
            .err()
            .unwrap()
            .code,
        "VXO1001"
    );
    assert_eq!(s.machine().board.controllers[0].buttons, 0);
    assert_eq!(s.command(Command::State {}).unwrap().state, "failed");
    s.command(Command::Reset {}).unwrap();
    assert_eq!(
        s.command(Command::Advance { cycles: 30_000 })
            .unwrap()
            .state,
        "running"
    );
    s.command(Command::Stop {}).unwrap();
    assert!(s.command(Command::Advance { cycles: 30_000 }).is_err());
}

#[test]
fn original_packet_demo_boots_all_five_services_and_matches_real_assembler_bytes() {
    use eightbit_vectrex_emulator::{
        digest,
        session::{Command, Session},
    };
    let cart = cartridge::assemble(include_str!("../../../templates/packet-demo.asm"))
        .unwrap()
        .bytes;
    // LWTOOLS 4.25, independently assembled from this original source.
    assert_eq!(cart.len(), 88);
    assert_eq!(
        digest(&cart),
        "2989f466fd0e32ea43e094dda41a3a6e34d3122122e94c262013a19a22da428b"
    );
    let mut s = Session::new(33, &cart, &digest(&cart), || false).unwrap();
    let mut lit = false;
    for _ in 0..5 {
        let frame = s.command(Command::Advance { cycles: 30_000 }).unwrap();
        lit |= frame.pixels.chunks_exact(4).filter(|p| p[0] != 0).count() > 30;
    }
    assert!(lit);
    s.command(Command::Reset {}).unwrap();
    assert!(s.command(Command::Advance { cycles: 30_000 }).is_ok());
    s.command(Command::Stop {}).unwrap();
}

#[test]
fn text_lists_publish_final_message_and_size_and_return_after_the_list_terminator() {
    for (entry, data, size) in [
        (
            0xf385,
            "list: fcb -8,40,0,-12\n fcc \"A\"\n fcb $80,-16,64,-24,12\nlast: fcc \"B\"\n fcb $80,0\nafter: fcb $FF",
            0xf040,
        ),
        (
            0xf38c,
            "list: fcb 24,-12\n fcc \"A\"\n fcb $80,-24,12\nlast: fcc \"B\"\n fcb $80,0\nafter: fcb $FF",
            0xf850,
        ),
    ] {
        let mut g = Guest::new(&format!("ldu #list\n jsr ${entry:04X}"), data);
        g.machine.board.write(0xc82a, 0xf8);
        g.machine.board.write(0xc82b, 80);
        g.run();
        let r = g.machine.cpu.registers;
        assert_eq!(
            (r.d(), r.x, r.y, r.u, r.dp),
            (1, 0xfbb4, 0x1234, g.symbols["AFTER"], 0xd0)
        );
        assert_eq!(g.word(0xc82c), g.symbols["LAST"]);
        assert_eq!(g.word(0xc82a), size);
        assert_eq!(g.machine.board.read(0xd006), 127);
        for address in 0xc800..0xcc00 {
            if !matches!(address, 0xc82a..=0xc82d | 0xcb80..=0xcbef) {
                assert_eq!(g.byte(address), 0, "list wrote unrelated RAM {address:04x}");
            }
        }
    }
}

#[test]
fn text_lists_accept_empty_and_sixty_four_messages_but_reject_a_sixty_fifth() {
    for entry in [0xf385, 0xf38c] {
        for count in [0, 1, 64] {
            let header = if entry == 0xf385 { "-8,40,0,0" } else { "1,0" };
            let mut g = Guest::new(
                &format!("ldu #list\n jsr ${entry:04X}"),
                &format!(
                    "list:\n{} fcb 0\nafter: fcb $FF",
                    format!(" fcb {header},$80\n").repeat(count)
                ),
            );
            g.machine.board.write(0xc82a, 0xf8);
            g.machine.board.write(0xc82b, 80);
            g.machine.board.write(0xc82c, 0xa5);
            g.machine.board.write(0xc82d, 0x5a);
            g.run();
            let r = g.machine.cpu.registers;
            assert_eq!(
                (r.d(), r.x, r.y, r.u, r.dp),
                (1, 0xfbb4, 0x1234, g.symbols["AFTER"], 0xd0)
            );
            if count == 0 {
                assert_eq!((g.word(0xc82a), g.word(0xc82c)), (0xf850, 0xa55a));
                assert_eq!(g.machine.board.read(0xd006), 63, "empty list changed scale");
            }
        }
        let header = if entry == 0xf385 { "-8,40,0,0" } else { "1,0" };
        let mut g = Guest::new(
            &format!("ldu #list\n jsr ${entry:04X}"),
            &format!("list:\n{} fcb 0", format!(" fcb {header},$80\n").repeat(65)),
        );
        assert_eq!(
            g.machine.advance(120_000, || false).unwrap_err().code,
            "VXO1001"
        );
    }
}

#[test]
fn text_list_pixels_match_original_single_string_calls() {
    for (entry, single, first, second) in [
        (0xf385, 0xf373, "-8,40,24,-20", "-8,40,-24,12"),
        (0xf38c, 0xf378, "24,-20", "-24,12"),
    ] {
        let data = format!(
            "list:\nfirst: fcb {first}\n fcc \"A\"\n fcb $80\nsecond: fcb {second}\n fcc \"B\"\n fcb $80,0"
        );
        let mut listed = Guest::new(&format!("jsr $F2A9\n ldu #list\n jsr ${entry:04X}"), &data);
        let mut individual = Guest::new(
            &format!(
                "jsr $F2A9\n ldu #first\n jsr ${single:04X}\n ldu #second\n jsr ${single:04X}"
            ),
            &data,
        );
        for guest in [&mut listed, &mut individual] {
            guest.machine.board.write(0xc82a, 0xf8);
            guest.machine.board.write(0xc82b, 40);
            guest.run();
            assert!(
                guest.machine.board.cycles < 30_000,
                "equivalence fixture crossed a display frame"
            );
            guest
                .machine
                .board
                .tick(30_000 - u32::try_from(guest.machine.board.cycles % 30_000).unwrap());
        }
        assert!(listed.machine.board.pixels().iter().any(|&p| p != 0));
        assert_eq!(
            listed.machine.board.pixels(),
            individual.machine.board.pixels()
        );
    }
}

#[test]
fn text_lists_validate_character_limits_and_reject_pointer_wrap_before_drawing() {
    for entry in [0xf385, 0xf38c] {
        let header = if entry == 0xf385 { "-8,40,1,0" } else { "1,0" };
        let mut g = Guest::new(
            &format!("ldu #list\n jsr ${entry:04X}"),
            &format!(
                "list: fcb {header}\n fcc \"{}\"\n fcb $80,0\nafter: fcb $FF",
                " ".repeat(256)
            ),
        );
        g.run();
        assert_eq!(g.machine.cpu.registers.u, g.symbols["AFTER"]);
        for data in [
            "fcb $1F,$80,0".to_owned(),
            "fcb $70,$80,0".to_owned(),
            "fcb $FF,$80,0".to_owned(),
            format!("fcc \"{}\"\n fcb $80,0", " ".repeat(257)),
            format!("fcc \"{}\"", " ".repeat(257)),
        ] {
            let mut bad = Guest::new(
                &format!("ldu #list\n jsr ${entry:04X}"),
                &format!("list: fcb {header}\n{data}"),
            );
            bad.machine.board.write(0xc82c, 0xa5);
            bad.machine.board.write(0xc82d, 0x5a);
            assert_eq!(
                bad.machine.advance(120_000, || false).unwrap_err().code,
                "VXO1001"
            );
            assert_eq!(
                bad.word(0xc82c),
                0xa55a,
                "malformed message reached renderer"
            );
        }
        for pointer in [0xfffc, 0xfffd, 0xfffe, 0xffff] {
            let mut bad = Guest::new(&format!("ldu #${pointer:04X}\n jsr ${entry:04X}"), "");
            assert_eq!(
                bad.machine.advance(120_000, || false).unwrap_err().code,
                "VXO1001"
            );
            assert_eq!(
                bad.machine.cpu.registers.u, pointer,
                "list pointer wrapped or advanced before validation"
            );
        }
    }
}

#[test]
fn text_list_validation_and_rendering_cancel_and_resume_with_balanced_stack() {
    for until_poll in [100, 2000] {
        let mut g = Guest::new(
            "ldu #list\n jsr $F38C",
            &format!(
                "list: fcb 24,-12\n fcc \"{}\"\n fcb $80,-24,12\n fcc \"B\"\n fcb $80,0\nafter: fcb $FF",
                "A".repeat(64)
            ),
        );
        g.machine.board.write(0xc82a, 0xf8);
        g.machine.board.write(0xc82b, 40);
        let mut polls = 0;
        assert_eq!(
            g.machine
                .advance(120_000, || {
                    polls += 1;
                    polls == until_poll
                })
                .unwrap_err()
                .code,
            "VXO1002"
        );
        assert!(g.machine.board.cycles < 120_000);
        let until = g.machine.board.cycles + 4_000_000;
        while g.machine.cpu.registers.pc != g.symbols["DONE"] {
            assert!(g.machine.board.cycles < until);
            g.machine.cpu.step(&mut g.machine.board).unwrap();
        }
        assert_eq!(g.machine.cpu.registers.s, 0xcbf0);
        assert_eq!(g.machine.cpu.registers.u, g.symbols["AFTER"]);
    }
}

#[test]
fn text_list_fault_releases_controls_and_reset_stop_recover_the_session() {
    use eightbit_vectrex_emulator::{
        digest,
        session::{Command, Session},
    };
    let source = format!(
        "{HEADER}start: jsr $F1BA\n lda $C80F\n bita #1\n beq start\n ldu #list\n jsr $F38C\n bra start\nlist: fcb 1,0,$1F,$80,0"
    );
    let cart = cartridge::assemble(&source).unwrap().bytes;
    let mut s = Session::new(42, &cart, &digest(&cart), || false).unwrap();
    s.command(Command::Key {
        code: "KeyZ".into(),
        pressed: true,
    })
    .unwrap();
    assert_eq!(
        s.command(Command::Advance { cycles: 30_000 })
            .err()
            .unwrap()
            .code,
        "VXO1001"
    );
    assert_eq!(s.machine().board.controllers[0].buttons, 0);
    assert_eq!(s.command(Command::State {}).unwrap().state, "failed");
    s.command(Command::Reset {}).unwrap();
    assert_eq!(
        s.command(Command::Advance { cycles: 30_000 })
            .unwrap()
            .state,
        "running"
    );
    s.command(Command::Stop {}).unwrap();
    assert!(s.command(Command::Advance { cycles: 30_000 }).is_err());
}

#[test]
fn original_text_list_demo_boots_both_services_and_matches_real_assembler_bytes() {
    use eightbit_vectrex_emulator::{
        digest,
        session::{Command, Session},
    };
    let cart = cartridge::assemble(include_str!("../../../templates/text-list-demo.asm"))
        .unwrap()
        .bytes;
    // Independently observed LWTOOLS 4.25 output for this authored cartridge.
    assert_eq!(cart.len(), 71);
    assert_eq!(
        digest(&cart),
        "ea9d0185ecf1abbf4b61b9b2baa492c1720e3cb94eabf86d3490b5ca4a3509ee"
    );
    let mut s = Session::new(43, &cart, &digest(&cart), || false).unwrap();
    let mut lit = false;
    for _ in 0..5 {
        let frame = s.command(Command::Advance { cycles: 30_000 }).unwrap();
        lit |= frame.pixels.chunks_exact(4).filter(|p| p[0] != 0).count() > 30;
    }
    assert!(lit);
    s.command(Command::Stop {}).unwrap();
    assert!(s.command(Command::Advance { cycles: 30_000 }).is_err());
    s.command(Command::Reset {}).unwrap();
    assert!(s.command(Command::Advance { cycles: 30_000 }).is_ok());
    s.command(Command::Stop {}).unwrap();
}

#[test]
fn patterned_counted_services_consume_pairs_preserve_state_and_bound_wrap() {
    for entry in [0xf434, 0xf437] {
        for count in [1, 2, 255, 256] {
            let mut g = Guest::new(
                &format!(
                    "ldx #list\n lda #{}\n sta $C823\n jsr ${entry:04X}",
                    count - 1
                ),
                &format!("list:\n{}after: fcb $A5", " fcb -1,1\n".repeat(count)),
            );
            g.machine.board.write(0xc829, 0xa5);
            g.machine.board.write(0xd00b, 0x83);
            g.machine.board.write(0xd008, 48);
            g.run();
            let r = g.machine.cpu.registers;
            assert_eq!(
                (r.x, r.y, r.u, r.dp),
                (g.symbols["AFTER"], 0x1234, 0xc980, 0xd0)
            );
            assert_eq!(g.byte(0xc823), 0);
            assert_eq!(g.byte(0xc829), 0xa5);
            assert_eq!(g.machine.board.read(0xd006), 63);
            assert_eq!(g.machine.board.read(0xd00b), 0x83);
            assert_eq!(g.machine.board.read(0xd00c), 0xce);
            for address in 0xc800..0xcc00 {
                if !matches!(address, 0xc823 | 0xc829 | 0xcb80..=0xcbef) {
                    assert_eq!(
                        g.byte(address),
                        0,
                        "pattern wrote unrelated RAM {address:04x}"
                    );
                }
            }
        }
        for pointer in [0xfffe, 0xffff] {
            let mut g = Guest::new(
                &format!("ldx #${pointer:04X}\n clra\n jsr ${entry:04X}"),
                "",
            );
            assert_eq!(
                g.machine.advance(120_000, || false).unwrap_err().code,
                "VXO1001"
            );
            assert_eq!(g.machine.cpu.registers.x, pointer);
        }
    }
}

fn pattern_pixels(body: &str, data: &str, pattern: u8, zero: u8) -> Vec<u8> {
    let mut g = Guest::new(&format!("jsr $F2A9\n{body}"), data);
    g.machine.board.write(0xc829, pattern);
    g.machine.board.write(0xc824, zero);
    g.machine.board.write(0xd004, 255);
    g.machine.board.write(0xd008, 30);
    g.run();
    assert!(g.machine.board.cycles < 30_000);
    g.machine
        .board
        .tick(30_000 - u32::try_from(g.machine.board.cycles).unwrap());
    g.machine.board.pixels().to_vec()
}

#[test]
fn pattern_bits_blank_solid_and_mixed_strokes_keep_geometry_and_zskip() {
    for zero in [0, 1] {
        let pair = "list: fcb -12,24";
        let body = "ldx #list\n clra\n jsr $F434";
        let blank = pattern_pixels(body, pair, 0, zero);
        let solid = pattern_pixels(body, pair, 255, zero);
        let mixed = pattern_pixels(body, pair, 0xaa, zero);
        let direct = pattern_pixels("ldd #$F418\n jsr $F3DF", "", 0, zero);
        let ink = |pixels: &[u8]| pixels.chunks_exact(4).filter(|p| p[0] != 0).count();
        assert_eq!(ink(&blank), 0);
        assert!(
            ink(&mixed) > 3 && ink(&mixed) < ink(&solid),
            "mixed={}, solid={}",
            ink(&mixed),
            ink(&solid)
        );
        assert_eq!(solid, direct);
        assert!(
            mixed
                .chunks_exact(4)
                .zip(solid.chunks_exact(4))
                .all(|(a, b)| a[0] == 0 || b[0] != 0)
        );
        // A following solid stroke exposes the final position, including ZSKIP.
        let suffix = "\n ldd #$0808\n jsr $F3DF";
        let patterned = pattern_pixels(&format!("{body}{suffix}"), pair, 255, zero);
        let primitive = pattern_pixels(&format!("ldd #$F418\n jsr $F3DF{suffix}"), "", 0, zero);
        assert_eq!(patterned, primitive);
    }
}

#[test]
fn patterned_packets_distinguish_modes_and_consume_the_terminator() {
    for count in [0, 1, 255, 256] {
        let mut g = Guest::new(
            "ldx #list\n jsr $F46E",
            &format!(
                "list:\n{} fcb 1\nafter: fcb $A5",
                " fcb 0,0,0\n".repeat(count)
            ),
        );
        g.run();
        let r = g.machine.cpu.registers;
        assert_eq!(
            (r.x, r.y, r.u, r.dp),
            (g.symbols["AFTER"], 0x1234, 0xc980, 0xd0)
        );
        assert_eq!(g.byte(0xc823), 0);
    }
    for zero in [0, 1] {
        let packet = "list: fcb 0,-12,-12,2,0,24,$FF,24,0,1";
        let body = "ldx #list\n jsr $F46E";
        let dashed = pattern_pixels(body, packet, 0, zero);
        let primitives = pattern_pixels(
            "ldd #$F4F4\n jsr $F312\n ldd #$0018\n jsr $F3DF\n ldd #$1800\n jsr $F312",
            "",
            0,
            0,
        );
        // Avoid an intermediate zero in the independent primitive sequence.
        let independent = if zero == 0 {
            primitives
        } else {
            pattern_pixels(
                "ldd #$F4F4\n jsr $F312\n ldd #$0018\n jsr $F3DF\n ldd #$1800\n jsr $F312\n jsr $F354",
                "",
                0,
                0,
            )
        };
        assert_eq!(dashed, independent);
        let solid = pattern_pixels(body, packet, 255, zero);
        assert!(
            solid.iter().filter(|&&p| p != 0).count() > dashed.iter().filter(|&&p| p != 0).count()
        );
    }
}

#[test]
fn patterned_packets_reject_bad_modes_excess_vectors_list_state_and_wrap() {
    for mode in [3, 127, 128, 254] {
        let mut g = Guest::new("ldx #list\n jsr $F46E", &format!("list: fcb {mode},0,0,1"));
        assert_eq!(
            g.machine.advance(120_000, || false).unwrap_err().code,
            "VXO1001"
        );
    }
    let mut g = Guest::new(
        "ldx #list\n jsr $F46E",
        &format!("list:\n{} fcb 1", " fcb 0,0,0\n".repeat(257)),
    );
    assert_eq!(
        g.machine.advance(120_000, || false).unwrap_err().code,
        "VXO1001"
    );
    let mut g = Guest::new("ldx #list\n jsr $F46E", "list: fcb 1");
    g.machine.board.write(0xc823, 1);
    assert_eq!(
        g.machine.advance(120_000, || false).unwrap_err().code,
        "VXO1001"
    );
    for pointer in [0xfffd, 0xfffe, 0xffff] {
        let mut g = Guest::new(&format!("ldx #${pointer:04X}\n jsr $F46E"), "");
        assert_eq!(
            g.machine.advance(120_000, || false).unwrap_err().code,
            "VXO1001"
        );
        assert!(g.machine.cpu.registers.x >= pointer);
    }
}

#[test]
fn patterned_work_cancels_and_resumes_with_stack_and_acr_restored() {
    for entry in [0xf434, 0xf437, 0xf46e] {
        let data = if entry == 0xf46e {
            format!(
                "list:\n{} fcb 1\nafter: fcb 0",
                " fcb $FF,0,1\n".repeat(256)
            )
        } else {
            format!("list:\n{}after: fcb 0", " fcb 0,1\n".repeat(256))
        };
        let mut g = Guest::new(
            &format!(
                "ldx #list\n lda #255\n{} jsr ${entry:04X}",
                if entry == 0xf437 { " sta $C823\n" } else { "" }
            ),
            &data,
        );
        g.machine.board.write(0xc829, 0xaa);
        while g.machine.board.read(0xd00c) != 0x0e {
            assert!(g.machine.board.cycles < 2000);
            g.machine.cpu.step(&mut g.machine.board).unwrap();
        }
        let mut polls = 0;
        assert_eq!(
            g.machine
                .advance(120_000, || {
                    polls += 1;
                    polls == 3
                })
                .unwrap_err()
                .code,
            "VXO1002"
        );
        assert_eq!(g.machine.board.read(0xd00b) & 0x1c, 0x10);
        assert!(g.machine.board.cycles < 120_000);
        let until = g.machine.board.cycles + 4_000_000;
        while g.machine.cpu.registers.pc != g.symbols["DONE"] {
            assert!(g.machine.board.cycles < until);
            g.machine.cpu.step(&mut g.machine.board).unwrap();
        }
        assert_eq!(g.machine.cpu.registers.s, 0xcbf0);
        assert_eq!(g.machine.cpu.registers.x, g.symbols["AFTER"]);
        assert_eq!(g.machine.board.read(0xd00b), 0x80);
    }
}

#[test]
fn pattern_fault_releases_controls_and_reset_stop_recover_the_session() {
    use eightbit_vectrex_emulator::{
        digest,
        session::{Command, Session},
    };
    let source = format!(
        "{HEADER}start: jsr $F1BA\n lda $C80F\n bita #1\n beq start\n ldx #list\n jsr $F46E\n bra start\nlist: fcb 3,0,0,1"
    );
    let cart = cartridge::assemble(&source).unwrap().bytes;
    let mut s = Session::new(51, &cart, &digest(&cart), || false).unwrap();
    s.command(Command::Key {
        code: "KeyZ".into(),
        pressed: true,
    })
    .unwrap();
    assert_eq!(
        s.command(Command::Advance { cycles: 30_000 })
            .err()
            .unwrap()
            .code,
        "VXO1001"
    );
    assert_eq!(s.machine().board.controllers[0].buttons, 0);
    assert_eq!(s.command(Command::State {}).unwrap().state, "failed");
    s.command(Command::Reset {}).unwrap();
    assert_eq!(
        s.command(Command::Advance { cycles: 30_000 })
            .unwrap()
            .state,
        "running"
    );
    s.command(Command::Stop {}).unwrap();
    assert!(s.command(Command::Advance { cycles: 30_000 }).is_err());
}

#[test]
fn original_pattern_demo_boots_all_three_services_and_matches_real_assembler_bytes() {
    use eightbit_vectrex_emulator::{
        digest,
        session::{Command, Session},
    };
    let cart = cartridge::assemble(include_str!("../../../templates/pattern-demo.asm"))
        .unwrap()
        .bytes;
    // LWTOOLS 4.25, independently assembled from this original source.
    assert_eq!(cart.len(), 95);
    assert_eq!(
        digest(&cart),
        "4fb5bb10b1f25cd7e726dedc9386479c4f06ec3e7ea06c5022395e972ba7e048"
    );
    let mut s = Session::new(52, &cart, &digest(&cart), || false).unwrap();
    let mut lit = false;
    for _ in 0..5 {
        let frame = s.command(Command::Advance { cycles: 30_000 }).unwrap();
        lit |= frame.pixels.chunks_exact(4).filter(|p| p[0] != 0).count() > 30;
    }
    assert!(lit);
    s.command(Command::Reset {}).unwrap();
    assert!(s.command(Command::Advance { cycles: 30_000 }).is_ok());
    s.command(Command::Stop {}).unwrap();
}
