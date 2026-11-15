//! The guest allocator alone moves bytes and updates the published RAM pointers.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_spectrum_emulator::{MAX_SLICE, Machine};
use eightbit_z80_core::Bus;

fn write_word(machine: &mut Machine, address: u16, value: u16) {
    let [lo, hi] = value.to_le_bytes();
    machine.board.write(address, lo);
    machine.board.write(address + 1, hi);
}
fn read_word(machine: &Machine, address: u16) -> u16 {
    u16::from_le_bytes([machine.board.peek(address), machine.board.peek(address + 1)])
}
fn fixture() -> Machine {
    let mut machine = Machine::development().unwrap();
    machine.cpu.registers.pc = 0x8000;
    machine.cpu.registers.sp = 0xff50;
    machine.cpu.registers.set_hl(0x7005);
    machine.cpu.registers.set_bc(3);
    for (address, value) in [
        (0x5c53, 0x7000),
        (0x5c4b, 0x7004),
        (0x5c4d, 0x7006),
        (0x5c59, 0x7008),
        (0x5c5d, 0x7009),
        (0x5c63, 0x700b),
        (0x5c65, 0x7010),
        (0x5cb2, 0xff57),
        (0x5c3d, 0xff00),
    ] {
        write_word(&mut machine, address, value);
    }
    for i in 0..16_u16 {
        machine.board.write(0x7000 + i, u8::try_from(i).unwrap());
    }
    machine.board.write(0x6fff, 0xa5);
    machine.board.write(0x7013, 0x5a);
    write_word(&mut machine, 0xff00, 0x8100);
    machine.board.write(0x8100, 0x76);
    for (offset, byte) in [0xcd, 0, 0x17, 0x76].into_iter().enumerate() {
        machine
            .board
            .write(0x8000 + u16::try_from(offset).unwrap(), byte);
    }
    machine
}

#[test]
fn checked_insertion_moves_only_the_live_extent_and_adjusts_existing_public_pointers() {
    let mut machine = fixture();
    machine.run(10000).unwrap();
    assert!(machine.cpu.halted);
    for i in 0..5_u16 {
        assert_eq!(machine.board.peek(0x7000 + i), u8::try_from(i).unwrap());
    }
    for i in 5..16_u16 {
        assert_eq!(machine.board.peek(0x7003 + i), u8::try_from(i).unwrap());
    }
    assert_eq!(machine.board.peek(0x6fff), 0xa5);
    assert_eq!(machine.board.peek(0x7013), 0x5a);
    assert_eq!(read_word(&machine, 0x5c53), 0x7000);
    assert_eq!(read_word(&machine, 0x5c4b), 0x7004);
    assert_eq!(read_word(&machine, 0x5c4d), 0x7009);
    assert_eq!(read_word(&machine, 0x5c59), 0x700b);
    assert_eq!(read_word(&machine, 0x5c5d), 0x700c);
    assert_eq!(read_word(&machine, 0x5c63), 0x700e);
    assert_eq!(read_word(&machine, 0x5c65), 0x7013);
    assert_eq!(machine.cpu.registers.hl(), 0x7005);
    assert_eq!(machine.cpu.registers.bc(), 3);
}

#[test]
fn zero_size_or_end_insertion_does_not_trigger_a_65536_byte_block_move() {
    let mut machine = fixture();
    machine.cpu.registers.set_bc(0);
    machine.run(10000).unwrap();
    assert_eq!(read_word(&machine, 0x5c65), 0x7010);
    assert_eq!(machine.board.peek(0x7013), 0x5a);
    let mut machine = fixture();
    machine.cpu.registers.set_hl(0x7010);
    machine.run(10000).unwrap();
    assert_eq!(read_word(&machine, 0x5c65), 0x7013);
    assert_eq!(machine.board.peek(0x700f), 15);
    assert_eq!(machine.board.peek(0x7013), 0x5a);
}

#[test]
fn invalid_extent_or_ramtop_failure_leaves_the_old_live_data_unchanged() {
    for (point, count, ramtop) in [
        (0x6fff, 3, 0xff57),
        (0x7011, 3, 0xff57),
        (0x7005, 0xffff, 0xff57),
        (0x7005, 3, 0x7011),
    ] {
        let mut machine = fixture();
        machine.cpu.registers.set_hl(point);
        machine.cpu.registers.set_bc(count);
        write_word(&mut machine, 0x5cb2, ramtop);
        machine.run(MAX_SLICE).unwrap();
        assert_eq!(machine.board.peek(0x5c3a), 3);
        for i in 0..16_u16 {
            assert_eq!(machine.board.peek(0x7000 + i), u8::try_from(i).unwrap());
        }
        assert_eq!(read_word(&machine, 0x5c65), 0x7010);
        assert_eq!(machine.board.firmware_page(), 0);
    }
}

#[test]
fn reclamation_closes_live_gaps_and_relocates_or_clamps_public_pointers() {
    let mut machine = fixture();
    machine.board.write(0x8002, 0x1d);
    machine.run(20000).unwrap();
    assert_eq!(machine.cpu.registers.pc, 0x8004);
    for i in 0..5_u16 {
        assert_eq!(machine.board.peek(0x7000 + i), u8::try_from(i).unwrap());
    }
    for i in 8..16_u16 {
        assert_eq!(machine.board.peek(0x7000 + i - 3), u8::try_from(i).unwrap());
    }
    for (address, value) in [
        (0x5c53, 0x7000),
        (0x5c4b, 0x7004),
        (0x5c4d, 0x7005),
        (0x5c59, 0x7005),
        (0x5c5d, 0x7006),
        (0x5c63, 0x7008),
        (0x5c65, 0x700d),
    ] {
        assert_eq!(read_word(&machine, address), value, "{address:04x}");
    }
    for (point, count) in [(0x6fff, 3), (0x700f, 2), (0x7005, 0xffff)] {
        let mut machine = fixture();
        machine.board.write(0x8002, 0x1d);
        machine.cpu.registers.set_hl(point);
        machine.cpu.registers.set_bc(count);
        machine.run(20000).unwrap();
        assert_eq!(machine.cpu.registers.pc, 0x8101);
        assert_eq!(read_word(&machine, 0x5c65), 0x7010);
        for i in 0..16_u16 {
            assert_eq!(machine.board.peek(0x7000 + i), u8::try_from(i).unwrap());
        }
    }
}
