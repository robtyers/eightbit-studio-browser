//! Independently authored syntax vectors. The pass must never execute BASIC.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_spectrum_emulator::{MAX_SLICE, Machine};
use eightbit_z80_core::{Bus, assembler::assemble};

fn check(source: &[u8], tokenise: bool) -> (bool, u16) {
    let mut machine = Machine::development().unwrap();
    for (offset, byte) in source.iter().copied().enumerate() {
        machine
            .board
            .write(0x9000 + u16::try_from(offset).unwrap(), byte);
    }
    let end = 0x9000 + u16::try_from(source.len()).unwrap();
    for (address, value) in [(0x5c59, 0x9000_u16), (0x5c65, end)] {
        let [lo, hi] = value.to_le_bytes();
        machine.board.write(address, lo);
        machine.board.write(address + 1, hi);
    }
    machine.board.write(0xc000, 0xa5);
    let text = if tokenise {
        "di\ncall $1e40\nld hl,$9000\ncall $1a0\nhalt"
    } else {
        "di\nld hl,$9000\ncall $1a0\nhalt"
    };
    let code = assemble(text, 0x8000, None).unwrap();
    for (offset, byte) in code.bytes.into_iter().enumerate() {
        machine
            .board
            .write(0x8000 + u16::try_from(offset).unwrap(), byte);
    }
    machine.cpu.registers.pc = 0x8000;
    machine.cpu.registers.sp = 0xff00;
    for _ in 0..30 {
        machine.run(MAX_SLICE).unwrap();
        if machine.cpu.halted {
            break;
        }
    }
    assert!(
        machine.cpu.halted,
        "syntax pass exceeded its bound for {source:?}"
    );
    assert_eq!(machine.board.firmware_page(), 0);
    assert_eq!(
        machine.board.peek(0xc000),
        0xa5,
        "validation must not execute POKE"
    );
    assert!(
        !machine.board.beeper_high(),
        "validation must not execute OUT/BEEP"
    );
    assert_eq!(
        machine.board.peek(0x5c4b),
        0,
        "validation must not create variables"
    );
    let pointer = u16::from_le_bytes([machine.board.peek(0x5c5d), machine.board.peek(0x5c5e)]);
    assert!((0x9000..=end).contains(&pointer));
    (machine.cpu.registers.f & 1 == 0, pointer - 0x9000)
}
#[test]
fn syntax_accepts_documented_statement_families_without_evaluating_their_values() {
    for source in [
        "10 LET A=1/0:POKE 49152,A:OUT 254,16",
        "PRINT ABS -2^2,SIN PI/2;AT 2,4;SCREEN$(1,2);TAB 12;INKEY$",
        "LET A$=\"x\"+CHR$ 13:LET B$=A$( TO 1):LET B$(1)=\"a\"",
        "LET Q=FN F(7,\"z\"):LET Q$=FN G$()",
        "DEF FN F(X,Y$)=X+LEN Y$:DEF FN G$()=STR$ 1",
        "IF A=1 AND NOT B<2 THEN PRINT \"x:y\":LET A=A+1",
        "FOR K=1 TO 9 STEP -1:NEXT K:GO SUB 100:GO TO 20:RETURN",
        "DIM A(2,3):DIM B$(2,8):READ A(1,2),B$(1):DATA 2+3,\"x\":RESTORE 100",
        "INPUT AT 0,0;INK 2;(\"value\");TAB 10;2+2;A:INPUT LINE A$",
        "PRINT #3;\"x\":INPUT #0;A:OPEN #5,\"P\":CLOSE #5:LIST #2;10:LLIST",
        "PLOT INK 2;1,3:DRAW OVER 1;10,-4,PI/2:CIRCLE 50,60,20",
        "SAVE \"p\" LINE 20:LOAD \"p\":MERGE \"p\":VERIFY \"p\"",
        "SAVE \"c\" CODE 24576,30:LOAD \"c\" CODE:VERIFY \"c\" CODE 24576,30",
        "SAVE \"a\" DATA A():LOAD \"s\" DATA A$():SAVE \"s\" SCREEN$",
        "BEEP .05,-3:BORDER 2:INK 9:PAPER 8:FLASH 1:BRIGHT 0:INVERSE 0:OVER 1",
        "RUN 10:CLEAR 24575:RANDOMIZE:PAUSE 0:STOP:CONTINUE:CLS:COPY:NEW",
        "REM untouched \" : + := 1e- or keywords",
        "LET A=BIN 1011:LET B=VAL \"2\":LET C$=VAL$ \"CHR$ 90\"",
        "LET A=POINT(2,3)+ATTR(1,2)+USR \"a\"+PEEK 0+IN 254+RND",
        "LET A$=(\"abcd\"(2 TO 3)+\"q\") AND 1:LET A=(\"x\"<\"y\") OR 0",
    ] {
        let bytes = [source.as_bytes(), &[13]].concat();
        assert!(check(&bytes, true).0, "must accept {source}");
    }
}
#[test]
fn syntax_rejects_bad_grammar_and_static_types_before_any_execution() {
    for source in [
        "0 PRINT 1",
        "10000 PRINT 1",
        "LET A=",
        "LET A=\"x\"",
        "LET A$=2",
        "LET A=1+",
        "LET A=(2*3",
        "PRINT \"unfinished",
        "PRINT 1 2",
        "PRINT SIN \"x\"",
        "PRINT LEN 3",
        "PRINT 2+\"x\"",
        "PRINT \"x\" OR 1",
        "PRINT POINT(1)",
        "IF \"x\" THEN PRINT 1",
        "IF 1 PRINT 1",
        "FOR AA=1 TO 3",
        "FOR A=1 3",
        "NEXT A$",
        "DIM LONG(2)",
        "DIM A(1 TO 2)",
        "READ 7",
        "INPUT LINE A",
        "INPUT \"prompt\"",
        "SAVE \"c\" CODE 24576",
        "SAVE 12",
        "LOAD \"x\" LINE 20",
        "DEF FN F(X)=\"x\"",
        "DEF FN F$(X)=X",
        "LET A=FN F(,2)",
        "LET A=.",
        "LET A=1e+",
        "LET A=BIN 2",
        "OPEN #3,2",
        "POKE 4,\"x\"",
        "CIRCLE 2,3",
    ] {
        let bytes = [source.as_bytes(), &[13]].concat();
        assert!(!check(&bytes, true).0, "must reject {source}");
    }
}
#[test]
fn missing_terminators_truncated_cells_and_excess_nesting_fail_within_bounds() {
    for bytes in [
        vec![],
        vec![245, b'1'],
        vec![245, b'1', 14, 0, 0],
        vec![17],
        vec![245, 34, b'x'],
    ] {
        assert!(!check(&bytes, false).0);
    }
    let source = format!("PRINT {}1{}\r", "(".repeat(70), ")".repeat(70));
    assert!(!check(source.as_bytes(), true).0);
}
