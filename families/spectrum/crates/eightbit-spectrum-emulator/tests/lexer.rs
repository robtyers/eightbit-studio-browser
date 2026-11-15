//! The editor lexer executes in original guest code, independently of host builds.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_spectrum_emulator::Machine;
use eightbit_z80_core::Bus;

#[test]
fn editor_keywords_preserve_strings_comments_identifiers_and_native_tokens() {
    for (source, expected) in [
        (
            b"print 1+2".as_slice(),
            [vec![245], b" 1+2".to_vec()].concat(),
        ),
        (
            b"LET Score=ABS (-2):PRINT Score",
            [
                vec![241],
                b" Score=".to_vec(),
                vec![189],
                b" (-2):".to_vec(),
                vec![245],
                b" Score".to_vec(),
            ]
            .concat(),
        ),
        (
            b"PRINT \"LET PRINT\":REM print a$ <= 2",
            [
                vec![245],
                b" \"LET PRINT\":".to_vec(),
                vec![234],
                b" print a$ <= 2".to_vec(),
            ]
            .concat(),
        ),
        (
            b"GO TO 10:LET A$=CHR$ 65",
            [
                vec![236],
                b" 10:".to_vec(),
                vec![241],
                b" A$=".to_vec(),
                vec![194],
                b" 65".to_vec(),
            ]
            .concat(),
        ),
        (
            b"IF X<=2 THEN PRINT LEN A$",
            [
                vec![250],
                b" X".to_vec(),
                vec![199],
                b"2 ".to_vec(),
                vec![203],
                b" ".to_vec(),
                vec![245],
                b" ".to_vec(),
                vec![177],
                b" A$".to_vec(),
            ]
            .concat(),
        ),
        (&[245, 32, 34, 65, 34], vec![245, 32, 34, 65, 34]),
    ] {
        let mut machine = Machine::development().unwrap();
        machine.board.write(0x5c59, 0);
        machine.board.write(0x5c5a, 0x90);
        for (offset, byte) in source.iter().copied().chain([13]).enumerate() {
            machine
                .board
                .write(0x9000 + u16::try_from(offset).unwrap(), byte);
        }
        for (offset, byte) in [0xcd, 0x40, 0x1e, 0x76].into_iter().enumerate() {
            machine
                .board
                .write(0x8000 + u16::try_from(offset).unwrap(), byte);
        }
        machine.cpu.registers.pc = 0x8000;
        machine.cpu.registers.sp = 0xff00;
        for _ in 0..10 {
            machine.run(350_000).unwrap();
            if machine.cpu.halted {
                break;
            }
        }
        assert!(
            machine.cpu.halted,
            "{source:?} at {:04x}",
            machine.cpu.registers.pc
        );
        let result: Vec<_> = (0..=expected.len())
            .map(|i| machine.board.peek(0x9000 + u16::try_from(i).unwrap()))
            .collect();
        assert_eq!(result, [expected, vec![13]].concat(), "{source:?}");
        assert_eq!(machine.board.firmware_page(), 0);
    }
}
