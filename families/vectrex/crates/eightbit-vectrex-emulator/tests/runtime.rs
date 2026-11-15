//! Original guest, boundary and device regression evidence.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_m6809_core::Bus;
use eightbit_vectrex_emulator::{
    cartridge, digest, firmware,
    machine::{Board, FRAME_CYCLES, HEIGHT, WIDTH},
    session::{Command, Session},
};
const SOURCE: &str = include_str!("../../../templates/vector-demo.asm");
fn session() -> Session {
    let cart = cartridge::assemble(SOURCE).unwrap().bytes;
    Session::new(7, &cart, &digest(&cart), || false).unwrap()
}
fn advance(s: &mut Session, frames: usize) -> eightbit_vectrex_emulator::session::Frame {
    let mut f = s.command(Command::State {}).unwrap();
    for _ in 0..frames {
        f = s
            .command(Command::Advance {
                cycles: FRAME_CYCLES,
            })
            .unwrap();
    }
    f
}
#[test]
fn original_source_boot_draws_moves_sounds_and_releases_without_external_tools() {
    let image = firmware::image().unwrap();
    assert_eq!(image.len(), 8192);
    assert_eq!(&image[8190..], &[0xe0, 0]);
    let mut s = session();
    let f = advance(&mut s, 4);
    assert_eq!(f.pixels.len(), WIDTH * HEIGHT * 4);
    assert!(f.pixels.chunks_exact(4).filter(|p| p[0] > 0).count() > 40);
    assert!(f.audio.iter().all(|n| *n == 0));
    let x = s.machine().board.peek(0xc880);
    s.command(Command::Key {
        code: "ArrowRight".into(),
        pressed: true,
    })
    .unwrap();
    advance(&mut s, 4);
    assert!(s.machine().board.peek(0xc880).wrapping_sub(x) >= 3);
    s.command(Command::Release {}).unwrap();
    advance(&mut s, 1);
    let x = s.machine().board.peek(0xc880);
    advance(&mut s, 3);
    assert_eq!(x, s.machine().board.peek(0xc880));
    s.command(Command::Key {
        code: "KeyZ".into(),
        pressed: true,
    })
    .unwrap();
    let f = advance(&mut s, 4);
    assert!(f.audio.iter().any(|n| *n > 0));
    assert!(f.audio.iter().any(|n| *n < 0));
    s.command(Command::Release {}).unwrap();
    let f = advance(&mut s, 3);
    assert!(f.audio.iter().all(|n| *n == 0));
    s.command(Command::Reset {}).unwrap();
    advance(&mut s, 4);
    assert_eq!(s.machine().board.peek(0xc880), 0);
}
#[test]
fn identity_bounds_cancellation_unsupported_services_and_reset_are_explicit() {
    let cart = cartridge::assemble(SOURCE).unwrap().bytes;
    assert!(Session::new(0, &cart, &digest(&cart), || false).is_err());
    assert!(Session::new(7, &cart, &"0".repeat(64), || false).is_err());
    assert!(Session::new(7, &cart, &digest(&cart), || true).is_err());
    let mut s = session();
    for text in [
        "{}",
        r#"{"schemaVersion":1,"sessionId":8,"command":{"operation":"state"}}"#,
        r#"{"schemaVersion":1,"sessionId":7,"command":{"operation":"state","extra":1}}"#,
    ] {
        assert!(s.request(text).is_err());
    }
    for cycles in [0, 120_001, u32::MAX] {
        assert!(s.command(Command::Advance { cycles }).is_err());
    }
    assert!(
        s.command(Command::Key {
            code: "secret".into(),
            pressed: true
        })
        .is_err()
    );
    s.command(Command::Stop {}).unwrap();
    assert!(s.command(Command::Advance { cycles: 1 }).is_err());
    s.command(Command::Reset {}).unwrap();
    advance(&mut s, 2);
    let src = SOURCE.split("start:").next().unwrap().to_owned() + "start: jsr $F687\n bra start\n";
    let cart = cartridge::assemble(&src).unwrap().bytes;
    let failure = Session::new(9, &cart, &digest(&cart), || false)
        .err()
        .unwrap();
    assert_eq!(failure.code, "VXO1001");
    assert!(!failure.message.contains("F687"));
}
#[test]
fn ram_rom_timer_irq_and_controller_bus_are_independent_of_guest_services() {
    let cart = cartridge::assemble(SOURCE).unwrap().bytes;
    let mut b = Board::new(cart, firmware::image().unwrap()).unwrap();
    b.write(0xc880, 42);
    assert_eq!(b.read(0xcc80), 42);
    b.write(0, 0);
    assert_eq!(b.read(0), b'g');
    b.write(0xe000, 0);
    assert_ne!(b.read(0xe000), 0);
    b.write(0xd00e, 0xc0);
    b.write(0xd004, 3);
    b.write(0xd005, 0);
    b.tick(4);
    assert_eq!(b.read(0xd00d) & 0xc0, 0xc0);
    assert!(b.interrupts().irq);
    b.read(0xd004);
    assert!(!b.interrupts().irq);
    b.write(0xd002, 0x9f);
    b.write(0xd003, 255);
    b.write(0xd000, 0x81);
    b.write(0xd001, 0);
    b.controllers[0].x = 127;
    assert_eq!(b.read(0xd000) & 32, 32);
    b.controllers[0].x = -127;
    assert_eq!(b.read(0xd000) & 32, 0);
}
#[test]
fn strings_and_cartridge_contracts_are_preserved() {
    let src = " org 0\n fcc \"g GCE 2026\"\n fcb $80\n fdb $FD0D\n fcb $F8,$50,$20,$C0\n fcc \"TEST\"\n fcb $80,0\nstart: bra start\n fcs \"DATA\"\n";
    assert!(
        cartridge::assemble(src).is_ok(),
        "{:?}",
        cartridge::assemble(src)
    );
    for source in [
        " org 0\n include \"outside\"\n",
        " org 1\n",
        " org 0\n fcs \"\"\n",
    ] {
        assert!(cartridge::assemble(source).is_err());
    }
    for bytes in [vec![], vec![0; 32769], vec![0; 20]] {
        assert!(cartridge::validate_cartridge(&bytes).is_err());
    }
}

#[test]
fn bundled_executive_and_original_cartridge_have_reproducible_identities() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../../firmware/original/manifest.json")).unwrap();
    assert_eq!(manifest["schemaVersion"], 1);
    assert_eq!(manifest["id"], firmware::ID);
    assert_eq!(manifest["bytes"], 8192);
    assert_eq!(
        manifest["sourceSha256"],
        digest(firmware::SOURCE.as_bytes())
    );
    assert_eq!(manifest["imageSha256"], digest(&firmware::image().unwrap()));
    let cart = cartridge::assemble(SOURCE).unwrap();
    assert_eq!(cart.bytes.len(), 204);
    // Independently observed LWTOOLS 4.25 output from this original source.
    assert_eq!(
        digest(&cart.bytes),
        "36acc57e60eedb3dfbd37656740265d97792bbdafb06bcec9faa4f1cddfe14f5"
    );
}

#[test]
fn both_controller_ports_bound_buttons_opposing_directions_and_release() {
    let mut s = session();
    for (port, keys) in [
        [
            "ArrowLeft",
            "ArrowRight",
            "ArrowUp",
            "ArrowDown",
            "KeyZ",
            "KeyX",
            "KeyC",
            "KeyV",
        ],
        [
            "KeyA", "KeyD", "KeyW", "KeyS", "KeyF", "KeyG", "KeyH", "KeyJ",
        ],
    ]
    .iter()
    .enumerate()
    {
        for (i, key) in keys.iter().enumerate() {
            s.command(Command::Key {
                code: (*key).into(),
                pressed: true,
            })
            .unwrap();
            let c = &s.machine().board.controllers[port];
            assert_eq!(
                (c.x, c.y, c.buttons),
                match i {
                    0 => (-127, 0, 0),
                    1 => (127, 0, 0),
                    2 => (0, 127, 0),
                    3 => (0, -127, 0),
                    _ => (0, 0, 1 << (i - 4)),
                }
            );
            s.command(Command::Release {}).unwrap();
        }
        for key in &keys[..4] {
            s.command(Command::Key {
                code: (*key).into(),
                pressed: true,
            })
            .unwrap();
        }
        let c = &s.machine().board.controllers[port];
        assert_eq!((c.x, c.y), (0, 0));
        s.command(Command::Stop {}).unwrap();
        let c = &s.machine().board.controllers[port];
        assert_eq!((c.x, c.y, c.buttons), (0, 0, 0));
        s.command(Command::Reset {}).unwrap();
    }
}

#[test]
fn manifest_paths_reject_nul_traversal_and_foreign_separators() {
    for path in [
        "../outside.asm",
        "src/a/../bad.asm",
        "src\\bad.asm",
        "src/\0.asm",
    ] {
        let m = cartridge::Manifest {
            schema_version: 1,
            name: "Paths".into(),
            target: "vectrex".into(),
            source: path.into(),
        };
        assert!(cartridge::parse_manifest(&toml::to_string(&m).unwrap()).is_err());
    }
    // The native path contract already normalises these separators/components.
    for path in [
        "src/asm/main.asm",
        "src/asm/./main.asm",
        "src/asm//main.asm",
    ] {
        let m = cartridge::Manifest {
            schema_version: 1,
            name: "Paths".into(),
            target: "vectrex".into(),
            source: path.into(),
        };
        assert!(cartridge::parse_manifest(&toml::to_string(&m).unwrap()).is_ok());
    }
}

#[test]
fn original_shift_model_rotates_msb_first_at_the_selected_t2_latch_period() {
    let cart = cartridge::assemble(SOURCE).unwrap().bytes;
    for low in [0, 6, 30, 255] {
        let mut b = Board::new(cart.clone(), firmware::image().unwrap()).unwrap();
        b.write(0xd008, low);
        b.write(0xd00b, 0x90);
        b.write(0xd00a, 0xa5);
        let period = u32::from(low) + 2;
        b.tick(period - 1);
        assert_eq!(b.read(0xd00a), 0xa5);
        for bit in 1..=24 {
            b.tick(if bit == 1 { 1 } else { period });
            assert_eq!(b.read(0xd00a), 0xa5_u8.rotate_left(bit % 8));
            // Reading SR acknowledges completion without restarting this model.
            assert_eq!(b.read(0xd00d) & 4, 0);
        }
        b.tick(period * 8);
        assert_eq!(b.read(0xd00d) & 4, 4);
        b.write(0xd00b, 0x80);
        let held = b.read(0xd00a);
        b.tick(period * 8);
        assert_eq!(b.read(0xd00a), held);
    }
}
