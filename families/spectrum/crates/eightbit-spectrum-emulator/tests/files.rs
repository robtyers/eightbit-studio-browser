//! BASIC cassette commands use real guest pulse I/O, including CLEAR/LOAD/USR.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_spectrum_emulator::{MAX_SLICE, Machine};
use eightbit_z80_core::{Bus, assembler::assemble};

fn word(machine: &Machine, address: u16) -> u16 {
    u16::from_le_bytes([machine.board.peek(address), machine.board.peek(address + 1)])
}
fn set_word(machine: &mut Machine, address: u16, value: u16) {
    let [lo, hi] = value.to_le_bytes();
    machine.board.write(address, lo);
    machine.board.write(address + 1, hi);
}
fn block(flag: u8, data: &[u8], out: &mut Vec<u8>) {
    out.extend(u16::try_from(data.len() + 2).unwrap().to_le_bytes());
    out.push(flag);
    out.extend(data);
    out.push(data.iter().fold(flag, |sum, byte| sum ^ byte));
}
fn file(kind: u8, name: &str, data: &[u8], first: u16, second: u16) -> Vec<u8> {
    let mut header = vec![kind];
    let mut filename = [32; 10];
    for (to, from) in filename.iter_mut().zip(name.bytes()) {
        *to = from;
    }
    header.extend(filename);
    header.extend(u16::try_from(data.len()).unwrap().to_le_bytes());
    header.extend(first.to_le_bytes());
    header.extend(second.to_le_bytes());
    let mut tape = Vec::new();
    block(0, &header, &mut tape);
    block(255, data, &mut tape);
    tape
}
fn programme(lines: &[(u16, Vec<u8>)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (number, body) in lines {
        bytes.extend(number.to_be_bytes());
        bytes.extend(u16::try_from(body.len() + 1).unwrap().to_le_bytes());
        bytes.extend(body);
        bytes.push(13);
    }
    bytes
}
fn boot() -> Machine {
    let mut machine = Machine::development().unwrap();
    for _ in 0..8 {
        machine.run(MAX_SLICE).unwrap();
        machine.board.drain_audio();
    }
    machine
}
fn command(machine: &mut Machine, source: &str, report: Option<u8>) {
    let line = word(machine, 0x5c59);
    for (offset, byte) in source.bytes().chain([13]).enumerate() {
        machine
            .board
            .write(line + u16::try_from(offset).unwrap(), byte);
    }
    for (offset, byte) in [0xcd, 0x40, 0x1e, 0xcd, 0, 0x1e, 0x76]
        .into_iter()
        .enumerate()
    {
        machine
            .board
            .write(0x8000 + u16::try_from(offset).unwrap(), byte);
    }
    set_word(machine, 0x5c3d, 0xff00);
    set_word(machine, 0xff00, 0x8100);
    machine.board.write(0x8100, 0x76);
    machine.cpu.halted = false;
    machine.cpu.registers.pc = 0x8000;
    machine.cpu.registers.sp = 0xff00;
    machine.cpu.registers.iff1 = false;
    machine.cpu.registers.iff2 = false;
    let start = machine.board.clocks();
    let mut drained = start;
    let mut calls = Vec::new();
    while !machine.cpu.halted && machine.board.clocks() - start < 105_000_000 {
        if machine.cpu.registers.pc == 0x1200 && calls.len() < 32 {
            calls.push((
                machine.board.firmware_page(),
                machine.cpu.registers.ix,
                machine.cpu.registers.de(),
                machine.cpu.registers.a,
                machine.cpu.registers.f,
                machine.board.tape().map(|t| (t.block(), t.elapsed())),
            ));
        }
        machine.step().unwrap();
        if machine.board.clocks() - drained >= u64::from(MAX_SLICE) {
            machine.board.drain_audio();
            drained = machine.board.clocks();
        }
    }
    assert!(
        machine.cpu.halted,
        "{source}: page {} PC {:04x} SP {:04x} IX {:04x} IY {:04x} DE {:04x} error {} tape {:?} calls {calls:?}",
        machine.board.firmware_page(),
        machine.cpu.registers.pc,
        machine.cpu.registers.sp,
        machine.cpu.registers.ix,
        machine.cpu.registers.iy,
        machine.cpu.registers.de(),
        machine.board.peek(0x5c3a),
        machine.board.tape().map(|t| (t.block(), t.elapsed()))
    );
    if let Some(report) = report {
        assert_eq!(machine.cpu.registers.pc, 0x8101);
        assert_eq!(machine.board.peek(0x5c3a), report);
    } else {
        assert_eq!(
            machine.cpu.registers.pc,
            0x8007,
            "{source}: error {}",
            machine.board.peek(0x5c3a)
        );
    }
    assert_eq!(machine.board.firmware_page(), 0);
    assert!(
        !machine.cpu.registers.iff1,
        "{source}: unexpected interrupt enable"
    );
}
fn mount(machine: &mut Machine, tape: &[u8]) {
    machine.board.mount_tap(tape).unwrap();
    machine.board.tape_mut().unwrap().play();
}

#[test]
fn basic_load_autostarts_native_programme_without_host_programme_injection() {
    let bytes = programme(&[(
        10,
        vec![244, 32, b'4', b'9', b'1', b'5', b'2', b',', b'4', b'2'],
    )]);
    let mut machine = boot();
    mount(
        &mut machine,
        &file(0, "BASIC", &bytes, 10, u16::try_from(bytes.len()).unwrap()),
    );
    command(&mut machine, "LOAD \"\"", None);
    assert_eq!(machine.board.peek(0xc000), 42);
    assert_eq!(
        word(&machine, 0x5c4b),
        0x5ccb + u16::try_from(bytes.len()).unwrap()
    );
    for (offset, expected) in bytes.iter().copied().enumerate() {
        assert_eq!(
            machine.board.peek(0x5ccb + u16::try_from(offset).unwrap()),
            expected
        );
    }
}

#[test]
fn standard_basic_loader_clears_loads_code_and_calls_our_assembled_machine_code() {
    let loader = programme(&[
        (
            10,
            [
                vec![253, 32],
                b"24575".to_vec(),
                vec![14, 143, 63, 254, 0, 0],
            ]
            .concat(),
        ),
        (20, vec![239, 32, b'"', b'"', 32, 175]),
        (
            30,
            [
                vec![249, 32, 192, 32],
                b"24576".to_vec(),
                vec![14, 143, 64, 0, 0, 0],
            ]
            .concat(),
        ),
    ]);
    let code = assemble("ORG 24576\nLD A,77\nLD (49152),A\nRET", 24576, None).unwrap();
    let mut tape = file(
        0,
        "LOADER",
        &loader,
        10,
        u16::try_from(loader.len()).unwrap(),
    );
    tape.extend(file(3, "CODE", &code.bytes, 24576, 32768));
    let mut machine = boot();
    mount(&mut machine, &tape);
    command(&mut machine, "LOAD \"\"", None);
    assert_eq!(machine.board.peek(0xc000), 77);
    assert_eq!(word(&machine, 0x5cb2), 24575);
    assert_eq!(machine.cpu.registers.sp, 24568);
    for (offset, expected) in code.bytes.iter().copied().enumerate() {
        assert_eq!(
            machine.board.peek(24576 + u16::try_from(offset).unwrap()),
            expected
        );
    }
}

#[test]
fn code_load_default_explicit_last_address_and_verify_report_actual_payload_mismatch() {
    let mut machine = boot();
    mount(&mut machine, &file(3, "BYTES", &[42, 99], 0xc000, 32768));
    command(&mut machine, "LOAD \"BYTES\" CODE", None);
    assert_eq!(machine.board.peek(0xc000), 42);
    assert_eq!(machine.board.peek(0xc001), 99);
    mount(&mut machine, &file(3, "BYTES", &[42, 99], 0xc000, 32768));
    command(&mut machine, "VERIFY \"\" CODE", None);
    mount(&mut machine, &file(3, "BYTES", &[42, 98], 0xc000, 32768));
    command(&mut machine, "VERIFY \"\" CODE", Some(26));
    assert_eq!(machine.board.peek(0xc001), 99);
    mount(&mut machine, &file(3, "LAST", &[123], 0xc000, 32768));
    command(&mut machine, "LOAD \"\" CODE 65535,1", None);
    assert_eq!(machine.board.peek(65535), 123);
}

#[test]
fn save_code_writes_two_physical_blocks_and_rejects_bad_filename_before_output() {
    let mut machine = boot();
    for (address, value) in [(0xc000, 42), (0xc001, 99), (0xc002, 123)] {
        machine.board.write(address, value);
    }
    machine.board.record_tape(true);
    command(&mut machine, "SAVE \"\" CODE 49152,3", Some(14));
    assert_eq!(machine.board.drain_tape_edges().unwrap().len(), 1);
    machine.board.record_tape(true);
    command(&mut machine, "SAVE \"BYTES\" CODE 49152,3", None);
    let edges = machine.board.drain_tape_edges().unwrap();
    assert!(edges.len() > 11000);
    assert!(
        edges
            .windows(2)
            .any(|pair| pair[1].t_state - pair[0].t_state > 3_000_000)
    );
    assert_eq!(machine.board.peek(0xc000), 42);
}

#[test]
fn cassette_header_sync_survives_different_ula_phases_at_the_last_ram_address() {
    for phase in [0, 1000, 14335, 32000] {
        let mut machine = boot();
        machine.board.tick(phase);
        mount(&mut machine, &file(3, "LAST", &[123], 0xc000, 32768));
        command(&mut machine, "LOAD \"\" CODE 65535,1", None);
        assert_eq!(machine.board.peek(65535), 123, "phase {phase}");
    }
}

#[test]
fn cassette_arrays_replace_native_records_verify_and_reject_malformed_data_without_losing_values() {
    let numeric = [
        2, 2, 0, 2, 0, 0, 0, 1, 0, 0, 0, 255, 253, 255, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0,
    ];
    let mut machine = boot();
    mount(&mut machine, &file(1, "NUM", &numeric, 0x8200, 32768));
    command(&mut machine, "LOAD \"NUM\" DATA a()", None);
    command(
        &mut machine,
        "POKE 49152,a(1,1):POKE 49153,a(1,2)+3:POKE 49154,a(2,1)",
        None,
    );
    assert_eq!(machine.board.peek(0xc000), 1);
    assert_eq!(machine.board.peek(0xc001), 0);
    assert_eq!(machine.board.peek(0xc002), 7);
    mount(&mut machine, &file(1, "NUM", &numeric, 0x8200, 32768));
    command(&mut machine, "VERIFY \"\" DATA a()", None);
    let mut changed = numeric;
    changed[7] = 9;
    mount(&mut machine, &file(1, "NUM", &changed, 0x8200, 32768));
    command(&mut machine, "VERIFY \"\" DATA a()", Some(26));
    let mut malformed = numeric;
    malformed[1] = 3;
    mount(&mut machine, &file(1, "NUM", &malformed, 0x8200, 32768));
    command(&mut machine, "LOAD \"\" DATA a()", Some(26));
    command(&mut machine, "POKE 49152,a(1,1)", None);
    assert_eq!(machine.board.peek(0xc000), 1);
    command(&mut machine, "LET b$=\"scalar\"", None);
    let characters = [2, 2, 0, 3, 0, b'a', b'b', b'c', b'd', b'e', b'f'];
    mount(&mut machine, &file(2, "CHAR", &characters, 0xc300, 32768));
    command(&mut machine, "LOAD \"\" DATA b$()", None);
    command(
        &mut machine,
        "POKE 49155,CODE b$(2):POKE 49156,LEN b$(2)",
        None,
    );
    assert_eq!(machine.board.peek(0xc003), b'd');
    assert_eq!(machine.board.peek(0xc004), 3);
}

fn saved_block(
    edges: &[eightbit_spectrum_emulator::TapeEdge],
    start: usize,
    length: usize,
) -> (Vec<u8>, usize) {
    let sync = (start..edges.len().saturating_sub(2))
        .find(|index| {
            let first = edges[index + 1].t_state - edges[*index].t_state;
            let second = edges[index + 2].t_state - edges[index + 1].t_state;
            (600..800).contains(&first) && (650..820).contains(&second)
        })
        .expect("two standard sync pulses");
    let mut bytes = Vec::new();
    let mut position = sync + 2;
    for _ in 0..length {
        let mut byte = 0;
        for _ in 0..8 {
            let first = edges[position + 1].t_state - edges[position].t_state;
            let second = edges[position + 2].t_state - edges[position + 1].t_state;
            let one = first + second > 2565;
            let expected = if one { 1710 } else { 855 };
            assert!(first.abs_diff(expected) < expected / 12);
            assert!(second.abs_diff(expected) < expected / 12);
            byte = (byte << 1) | u8::from(one);
            position += 2;
        }
        bytes.push(byte);
    }
    assert_eq!(bytes.iter().fold(0, |sum, byte| sum ^ byte), 0);
    (bytes, position)
}
#[test]
fn edited_basic_and_native_arrays_save_standard_headers_and_payload_bytes() {
    let mut machine = boot();
    command(&mut machine, "10 LET A=42:PRINT A", None);
    command(&mut machine, "20 DEF FN F(X)=X+1", None);
    let programme_length = word(&machine, 0x5c4b) - word(&machine, 0x5c53);
    let expected: Vec<_> = (0x5ccb..0x5ccb + programme_length)
        .map(|address| machine.board.peek(address))
        .collect();
    assert_eq!(expected.iter().filter(|byte| **byte == 14).count(), 3);
    machine.board.record_tape(true);
    command(&mut machine, "SAVE \"ORIGINAL\" LINE 10", None);
    let edges = machine.board.drain_tape_edges().unwrap();
    let (header, next) = saved_block(&edges, 0, 19);
    assert_eq!(&header[1..12], b"\0ORIGINAL  ");
    assert_eq!(&header[14..16], 10_u16.to_le_bytes());
    assert_eq!(&header[16..18], programme_length.to_le_bytes());
    let (data, _) = saved_block(&edges, next, usize::from(programme_length) + 2);
    assert_eq!(&data[1..data.len() - 1], expected);
    command(
        &mut machine,
        "DIM A(2):LET A(1)=7:DIM B$(2,3):LET B$(2)=\"xyz\"",
        None,
    );
    for (source, kind, name) in [
        ("SAVE \"NUM\" DATA A()", 1, 0x81_u8),
        ("SAVE \"CHAR\" DATA B$()", 2, 0xc2),
    ] {
        machine.board.record_tape(true);
        command(&mut machine, source, None);
        let edges = machine.board.drain_tape_edges().unwrap();
        let (header, next) = saved_block(&edges, 0, 19);
        assert_eq!(header[1], kind);
        assert_eq!(header[15], name);
        assert_eq!(&header[16..18], 32768_u16.to_le_bytes());
        let length = u16::from_le_bytes([header[12], header[13]]);
        let (data, _) = saved_block(&edges, next, usize::from(length) + 2);
        if kind == 2 {
            assert_eq!(&data[1..data.len() - 1], b"\x02\x02\0\x03\0   xyz");
        } else {
            assert_eq!(&data[1..4], [1, 2, 0]);
        }
    }
}

#[test]
fn merge_replaces_matching_lines_and_variable_names_and_preserves_other_records() {
    let mut machine = boot();
    for line in ["10 LET A=1", "20 POKE 49152,A", "30 POKE 49153,B"] {
        command(&mut machine, line, None);
    }
    command(
        &mut machine,
        "LET A=7:LET B=8:LET C$=\"old\":DIM D(1):LET LONGNAME=17:LET LONGER=18",
        None,
    );
    let lines = programme(&[
        (10, [vec![244, 32], b"49152,9".to_vec()].concat()),
        (15, [vec![241, 32], b"C=3".to_vec()].concat()),
        (40, [vec![245, 32], b"\"done\"".to_vec()].concat()),
    ]);
    let mut payload = lines.clone();
    payload.extend([0x61, 0, 0, 9, 0, 0, 0x43, 3, 0, b'a', b'b', b'c']);
    payload.extend([0x84, 13, 0, 1, 2, 0, 0, 0, 4, 0, 0, 0, 0, 5, 0, 0]);
    payload.extend([
        0xac,
        b'o',
        b'n',
        b'g',
        b'n',
        b'a',
        b'm',
        b'e' | 0x80,
        0,
        0,
        23,
        0,
        0,
    ]);
    mount(
        &mut machine,
        &file(0, "MERGE", &payload, 0, u16::try_from(lines.len()).unwrap()),
    );
    command(&mut machine, "MERGE \"MERGE\"", None);
    assert_eq!(machine.board.peek(0xc000), 0);
    let mut address = word(&machine, 0x5c53);
    for number in [10_u16, 15, 20, 30, 40] {
        assert_eq!(
            u16::from_be_bytes([machine.board.peek(address), machine.board.peek(address + 1)]),
            number
        );
        address += 4 + word(&machine, address + 2);
    }
    assert_eq!(address, word(&machine, 0x5c4b));
    command(
        &mut machine,
        "POKE 49152,A:POKE 49153,B:POKE 49154,CODE C$:POKE 49155,D(2):POKE 49156,LONGNAME:POKE 49157,LONGER",
        None,
    );
    for (offset, expected) in [9, 8, b'a', 5, 23, 18].into_iter().enumerate() {
        assert_eq!(
            machine.board.peek(0xc000 + u16::try_from(offset).unwrap()),
            expected
        );
    }
    command(&mut machine, "GO TO 10", None);
    assert_eq!(machine.board.peek(0xc000), 9);
    assert_eq!(machine.board.peek(0xc001), 8);
    let old_vars = word(&machine, 0x5c4b);
    let malformed = [0, 25, 255, 15, 13];
    mount(&mut machine, &file(0, "BAD", &malformed, 0, 5));
    command(&mut machine, "MERGE \"\"", Some(26));
    assert_eq!(word(&machine, 0x5c4b), old_vars);
}
