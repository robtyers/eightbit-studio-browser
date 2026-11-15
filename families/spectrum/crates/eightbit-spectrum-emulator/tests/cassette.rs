//! Standard TAP pulses are consumed by original guest code at the public loader.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_spectrum_emulator::{MAX_SLICE, Machine};
use eightbit_z80_core::Bus;

fn load(flag: u8, data: &[u8], requested: u8, verify: bool, corrupt: bool, old: &[u8]) -> Machine {
    let mut block = vec![flag];
    block.extend(data);
    let parity = block.iter().fold(0, |value, byte| value ^ byte);
    block.push(parity ^ u8::from(corrupt));
    let mut tap = u16::try_from(block.len()).unwrap().to_le_bytes().to_vec();
    tap.extend(block);
    let mut machine = Machine::development().unwrap();
    machine.board.mount_tap(&tap).unwrap();
    machine.board.tape_mut().unwrap().play();
    for (offset, byte) in [0xcd, 0x56, 0x05, 0x76].into_iter().enumerate() {
        machine
            .board
            .write(0x8000 + u16::try_from(offset).unwrap(), byte);
    }
    for (offset, byte) in old.iter().copied().enumerate() {
        machine
            .board
            .write(0x9000 + u16::try_from(offset).unwrap(), byte);
    }
    machine.cpu.registers.pc = 0x8000;
    machine.cpu.registers.sp = 0xff00;
    machine.cpu.registers.ix = 0x9000;
    machine.cpu.registers.iy = 0x5c3a;
    machine.cpu.registers.a = requested;
    machine.cpu.registers.f = u8::from(!verify);
    machine
        .cpu
        .registers
        .set_de(u16::try_from(data.len()).unwrap());
    for _ in 0..100 {
        machine.run(MAX_SLICE).unwrap();
        machine.board.drain_audio();
        if machine.cpu.halted {
            break;
        }
    }
    assert!(
        machine.cpu.halted,
        "loader stuck at {:04x}",
        machine.cpu.registers.pc
    );
    assert_eq!(machine.cpu.registers.pc, 0x8004);
    assert_eq!(machine.cpu.registers.iy, 0x5c3a);
    assert_eq!(machine.board.firmware_page(), 0);
    assert!(!machine.cpu.registers.iff1);
    machine
}

#[test]
fn public_loader_decodes_header_and_data_pulses_without_host_memory_injection() {
    for flag in [0, 255] {
        let data = [0, 255, 0x55, 0xaa, 1, 128, 13, 14, 0x76];
        let machine = load(flag, &data, flag, false, false, &[]);
        assert_ne!(machine.cpu.registers.f & 1, 0, "flag {flag}");
        for (offset, expected) in data.into_iter().enumerate() {
            assert_eq!(
                machine.board.peek(0x9000 + u16::try_from(offset).unwrap()),
                expected
            );
        }
        assert_eq!(machine.cpu.registers.ix, 0x9009);
        assert_eq!(machine.cpu.registers.de(), 0);
    }
}

#[test]
fn verification_flag_mismatch_and_checksum_failures_return_carry_clear() {
    let data = [1, 2, 3, 4];
    let success = load(255, &data, 255, true, false, &data);
    assert_ne!(success.cpu.registers.f & 1, 0);
    let mismatch = load(255, &data, 255, true, false, &[1, 2, 9, 4]);
    assert_eq!(mismatch.cpu.registers.f & 1, 0);
    assert_eq!(mismatch.board.peek(0x9002), 9);
    let bad_flag = load(255, &data, 0, false, false, &[99; 4]);
    assert_eq!(bad_flag.cpu.registers.f & 1, 0);
    assert_eq!(bad_flag.board.peek(0x9000), 99);
    let bad_checksum = load(255, &data, 255, false, true, &[]);
    assert_eq!(bad_checksum.cpu.registers.f & 1, 0);
}

#[test]
fn public_save_emits_standard_pilot_sync_data_and_checksum_pulses() {
    for (flag, stack) in [(0_u8, 0xff00), (255, 0x7000)] {
        let data: Vec<u8> = (0..=255).collect();
        let mut machine = Machine::development().unwrap();
        for (offset, byte) in data.iter().copied().enumerate() {
            machine
                .board
                .write(0x9000 + u16::try_from(offset).unwrap(), byte);
        }
        for (offset, byte) in [0xcd, 0xc2, 0x04, 0x76].into_iter().enumerate() {
            machine
                .board
                .write(0x8000 + u16::try_from(offset).unwrap(), byte);
        }
        machine.board.write(0x5c48, 56);
        machine.board.record_tape(true);
        machine.cpu.registers.pc = 0x8000;
        machine.cpu.registers.sp = stack;
        machine.cpu.registers.ix = 0x9000;
        machine.cpu.registers.iy = 0x5c3a;
        machine.cpu.registers.a = flag;
        machine.cpu.registers.set_de(256);
        for _ in 0..120 {
            machine.run(MAX_SLICE).unwrap();
            machine.board.drain_audio();
            if machine.cpu.halted {
                break;
            }
        }
        assert!(
            machine.cpu.halted,
            "save at {:04x}",
            machine.cpu.registers.pc
        );
        assert_eq!(machine.cpu.registers.pc, 0x8004);
        assert_ne!(machine.cpu.registers.f & 1, 0);
        assert_eq!(machine.cpu.registers.ix, 0x9100);
        assert_eq!(machine.cpu.registers.de(), 0);
        assert_eq!(machine.cpu.registers.sp, stack);
        assert_eq!(machine.cpu.registers.iy, 0x5c3a);
        let edges = machine.board.drain_tape_edges().unwrap();
        let pilot = if flag == 0 { 8063 } else { 3223 };
        assert_eq!(edges.len(), pilot + 16 * (data.len() + 2) + 4);
        for pulse in edges[1..=pilot + 1].windows(2) {
            let duration = pulse[1].t_state - pulse[0].t_state;
            assert!(duration.abs_diff(2168) < 180, "pilot {duration}");
        }
        for (position, expected) in [(pilot + 1, 667_u64), (pilot + 2, 735)] {
            let duration = edges[position + 1].t_state - edges[position].t_state;
            assert!(
                duration.abs_diff(expected) < expected / 12,
                "sync {position}: {duration}"
            );
        }
        let mut decoded = Vec::new();
        let mut byte = 0_u8;
        for (bit, pulses) in edges[pilot + 3..].windows(3).step_by(2).enumerate() {
            let a = pulses[1].t_state - pulses[0].t_state;
            let b = pulses[2].t_state - pulses[1].t_state;
            let one = a + b > 2565;
            let expected = if one { 1710 } else { 855 };
            assert!(a.abs_diff(expected) < expected / 12, "bit {bit} first {a}");
            assert!(b.abs_diff(expected) < expected / 12, "bit {bit} second {b}");
            byte = (byte << 1) | u8::from(one);
            if bit % 8 == 7 {
                decoded.push(byte);
                byte = 0;
            }
        }
        let mut expected = vec![flag];
        expected.extend(&data);
        expected.push(expected.iter().fold(0, |sum, value| sum ^ value));
        assert_eq!(decoded, expected);
    }
}
