//! Guest-only variable lookup/insertion against independently specified RAM records.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_spectrum_emulator::Machine;
use eightbit_z80_core::Bus;

fn word(machine: &Machine, address: u16) -> u16 {
    u16::from_le_bytes([machine.board.peek(address), machine.board.peek(address + 1)])
}
fn set_word(machine: &mut Machine, address: u16, value: u16) {
    let [lo, hi] = value.to_le_bytes();
    machine.board.write(address, lo);
    machine.board.write(address + 1, hi);
}
fn machine() -> Machine {
    let mut machine = Machine::development().unwrap();
    for (address, value) in [
        (0x5c53, 0x5ccb),
        (0x5c4b, 0x5ccb),
        (0x5c59, 0x5ccc),
        (0x5cb2, 0xff57),
    ] {
        set_word(&mut machine, address, value);
    }
    machine.board.write(0x5ccb, 0x80);
    machine
}
fn name(machine: &mut Machine, name: &str, value: Option<[u8; 5]>) {
    let line = word(machine, 0x5c59);
    for (offset, byte) in name.bytes().chain([13]).enumerate() {
        machine
            .board
            .write(line + u16::try_from(offset).unwrap(), byte);
    }
    let base = line + u16::try_from(name.len()).unwrap() + 16;
    for address in [0x5c4d, 0x5c5d] {
        set_word(machine, address, line);
    }
    for address in [0x5c61, 0x5c63, 0x5c65] {
        set_word(machine, address, base);
    }
    if let Some(value) = value {
        for (offset, byte) in value.into_iter().enumerate() {
            machine
                .board
                .write(base + u16::try_from(offset).unwrap(), byte);
        }
        set_word(machine, 0x5c65, base + 5);
    }
}
fn call(machine: &mut Machine, operation: u8) {
    call_bank(machine, operation, 0x1c80);
}
fn call_bank(machine: &mut Machine, operation: u8, address: u16) {
    let [lo, hi] = address.to_le_bytes();
    for (offset, byte) in [0x3e, operation, 0xcd, lo, hi, 0x76]
        .into_iter()
        .enumerate()
    {
        machine
            .board
            .write(0x8000 + u16::try_from(offset).unwrap(), byte);
    }
    machine.cpu.halted = false;
    machine.cpu.registers.pc = 0x8000;
    machine.cpu.registers.sp = 0xff50;
    machine.cpu.registers.iff1 = false;
    machine.cpu.registers.iff2 = false;
    let mut clocks = 0;
    while !machine.cpu.halted && clocks < 100_000 {
        clocks += machine.step().unwrap();
    }
    assert!(
        machine.cpu.halted,
        "variable operation did not finish at PC {:04x}",
        machine.cpu.registers.pc
    );
    assert_eq!(machine.board.firmware_page(), 0);
}
fn result(machine: &Machine) -> [u8; 5] {
    let end = word(machine, 0x5c65);
    let mut result = [0; 5];
    for (offset, byte) in result.iter_mut().enumerate() {
        *byte = machine.board.peek(end - 5 + u16::try_from(offset).unwrap());
    }
    result
}

#[test]
fn numeric_assignment_uses_native_records_and_relocates_the_current_command_and_stack() {
    let mut machine = machine();
    let value = [0, 0, 42, 0, 0];
    name(&mut machine, "A", Some(value));
    call(&mut machine, 1);
    assert_eq!(machine.cpu.registers.pc, 0x8006);
    let record: Vec<_> = (0x5ccb..0x5cd2).map(|a| machine.board.peek(a)).collect();
    assert_eq!(record, [0x61, 0, 0, 42, 0, 0, 0x80]);
    assert_eq!(word(&machine, 0x5c4b), 0x5ccb);
    assert_eq!(word(&machine, 0x5c53), 0x5ccb);
    assert_eq!(word(&machine, 0x5c59), 0x5cd2);
    assert_eq!(machine.board.peek(0x5cd2), b'A');
    assert_eq!(word(&machine, 0x5c63), word(&machine, 0x5c65));
    name(&mut machine, "a", None);
    call(&mut machine, 0);
    assert_eq!(result(&machine), value);
    assert_ne!(machine.board.peek(0x5c3b) & 64, 0);
}

#[test]
fn long_names_case_spaces_and_shared_prefixes_remain_distinct_without_duplicate_records() {
    let mut machine = machine();
    for (name_text, value) in [
        ("CountDown99", 7_u16),
        ("COUNTER", 42),
        ("B", 65535),
        ("C", 999),
    ] {
        let [lo, hi] = value.to_le_bytes();
        name(&mut machine, name_text, Some([0, 0, lo, hi, 0]));
        call(&mut machine, 1);
        assert_eq!(machine.cpu.registers.pc, 0x8006);
    }
    for (name_text, value) in [
        ("count down 99", 7_u16),
        ("counter", 42),
        ("b", 65535),
        ("c", 999),
    ] {
        let [lo, hi] = value.to_le_bytes();
        name(&mut machine, name_text, None);
        call(&mut machine, 0);
        assert_eq!(result(&machine), [0, 0, lo, hi, 0]);
    }
    let end = word(&machine, 0x5c59);
    name(&mut machine, "counter", Some([0, 0, 77, 0, 0]));
    call(&mut machine, 1);
    assert_eq!(word(&machine, 0x5c59), end);
    name(&mut machine, "COUNTER", None);
    call(&mut machine, 0);
    assert_eq!(result(&machine), [0, 0, 77, 0, 0]);
    assert_eq!(machine.board.peek(0x5ccb), 0xa3);
    assert_eq!(machine.board.peek(0x5ccb + 10), b'9' | 0x80);
}

#[test]
fn lookup_skips_existing_string_array_and_for_records_without_treating_their_data_as_names() {
    let mut machine = machine();
    let mut records = vec![
        0x41, 3, 0, b'a', b'b', b'c', 0x82, 8, 0, 1, 1, 0, 0, 0, 0, 0, 0,
    ];
    records.push(0xe3);
    records.extend([0, 0, 17, 0, 0]);
    records.extend([0; 13]);
    records.extend([0x64, 0, 0, 99, 0, 0, 0x80]);
    let end = 0x5ccb + u16::try_from(records.len()).unwrap();
    for (offset, byte) in records.into_iter().enumerate() {
        machine
            .board
            .write(0x5ccb + u16::try_from(offset).unwrap(), byte);
    }
    set_word(&mut machine, 0x5c59, end);
    name(&mut machine, "D", None);
    call(&mut machine, 0);
    assert_eq!(result(&machine), [0, 0, 99, 0, 0]);
    name(&mut machine, "C", Some([0, 0, 23, 0, 0]));
    call(&mut machine, 1);
    name(&mut machine, "C", None);
    call(&mut machine, 0);
    assert_eq!(result(&machine), [0, 0, 23, 0, 0]);
}

#[test]
fn target_name_scanning_stops_at_the_assignment_without_creating_a_variable() {
    let mut machine = machine();
    name(&mut machine, "The counter 2 = 1", None);
    call(&mut machine, 2);
    assert_eq!(machine.board.peek(word(&machine, 0x5c5d)), b'=');
    assert_eq!(word(&machine, 0x5c4d), 0x5ccc);
    assert_eq!(machine.board.peek(0x5ccb), 0x80);
    assert_eq!(word(&machine, 0x5c59), 0x5ccc);
}

#[test]
fn string_reassignment_grows_and_shrinks_one_native_record_without_leaking_variables() {
    let mut machine = machine();
    for text in ["abc", "a much longer string", "", "z", "final"] {
        let line = word(&machine, 0x5c59);
        let [lo, hi] = (line + 4).to_le_bytes();
        let [length_lo, length_hi] = u16::try_from(text.len()).unwrap().to_le_bytes();
        name(
            &mut machine,
            &format!("A$=\"{text}\""),
            Some([0, lo, hi, length_lo, length_hi]),
        );
        machine.board.write(0x5c3b, 0);
        call_bank(&mut machine, 66, 0x0d80);
        assert_eq!(machine.cpu.registers.pc, 0x8006);
        assert_eq!(machine.board.peek(0x5ccb), 0x41);
        assert_eq!(word(&machine, 0x5ccc), u16::try_from(text.len()).unwrap());
        let bytes: Vec<_> = (0..u16::try_from(text.len()).unwrap())
            .map(|i| machine.board.peek(0x5cce + i))
            .collect();
        assert_eq!(bytes, text.as_bytes());
        let marker = 0x5cce + u16::try_from(text.len()).unwrap();
        assert_eq!(machine.board.peek(marker), 0x80);
        assert_eq!(word(&machine, 0x5c59), marker + 1);
        assert_eq!(word(&machine, 0x5c4b), 0x5ccb);
        assert_eq!(word(&machine, 0x5c53), 0x5ccb);
        name(&mut machine, "a$", None);
        call_bank(&mut machine, 65, 0x0d80);
        assert_eq!(result(&machine), [0, 0xce, 0x5c, length_lo, length_hi]);
        assert_eq!(machine.board.peek(0x5c3b) & 64, 0);
    }
}
