//! Original guest sound conversion and public beeper timing on the actual pin.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_spectrum_emulator::{MAX_SLICE, Machine};
use eightbit_z80_core::Bus;

fn trace(machine: &mut Machine, end: u16) -> Vec<u64> {
    let mut edges = Vec::new();
    let mut high = machine.board.beeper_high();
    let start = machine.board.clocks();
    while machine.board.clocks() - start < 10_000_000 {
        machine.step().unwrap();
        let next = machine.board.beeper_high();
        if high != next {
            edges.push(machine.board.clocks());
            high = next;
        }
        if machine.cpu.halted && machine.cpu.registers.pc == end {
            return edges;
        }
        if machine.board.clocks().is_multiple_of(1000) {
            machine.board.drain_audio();
        }
    }
    panic!("sound did not return at {:04x}", machine.cpu.registers.pc);
}
#[test]
fn public_beeper_retains_register_owners_interrupt_state_and_quarter_delay_timing() {
    for delay in [0_u16, 1, 6, 7, 13, 1642, 30000] {
        for stack in [0x7000, 0xff00] {
            let mut machine = Machine::development().unwrap();
            for (offset, byte) in [0xcd, 0xb5, 0x03, 0x76].into_iter().enumerate() {
                machine
                    .board
                    .write(0x8000 + u16::try_from(offset).unwrap(), byte);
            }
            machine.board.write(0x5c48, 56);
            machine.cpu.registers.pc = 0x8000;
            machine.cpu.registers.sp = stack;
            machine.cpu.registers.ix = 0x1234;
            machine.cpu.registers.iy = 0x5c3a;
            machine.cpu.registers.set_hl(delay);
            machine.cpu.registers.set_de(3);
            let edges = trace(&mut machine, 0x8004);
            assert_eq!(edges.len(), 6, "delay {delay}");
            let expected = u64::from(delay) * 8 + 241;
            for cycle in [edges[2] - edges[0], edges[4] - edges[2]] {
                assert!(
                    cycle.abs_diff(expected) <= 24,
                    "delay {delay}: {cycle} vs {expected}"
                );
            }
            assert_eq!(machine.cpu.registers.ix, 0x1234);
            assert_eq!(machine.cpu.registers.iy, 0x5c3a);
            assert_eq!(machine.cpu.registers.sp, stack);
            assert!(!machine.cpu.registers.iff1);
            assert!(!machine.board.beeper_high());
        }
    }
}

#[test]
fn basic_beep_changes_octaves_and_consumes_both_guest_numeric_arguments() {
    for (pitch, cycles, period) in [(0, 5, 13377_u64), (12, 10, 6688)] {
        let mut machine = Machine::development().unwrap();
        for _ in 0..8 {
            machine.run(MAX_SLICE).unwrap();
            machine.board.drain_audio();
        }
        let line = u16::from_le_bytes([machine.board.peek(0x5c59), machine.board.peek(0x5c5a)]);
        let source = format!("BEEP .02,{pitch}:POKE 49152,123");
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
        machine.cpu.halted = false;
        machine.cpu.registers.pc = 0x8000;
        machine.cpu.registers.sp = 0xff00;
        machine.cpu.registers.iff1 = false;
        machine.cpu.registers.iff2 = false;
        let edges = trace(&mut machine, 0x8007);
        assert_eq!(edges.len(), cycles * 2);
        for pair in edges.windows(3).step_by(2) {
            assert!((pair[2] - pair[0]).abs_diff(period) < 32);
        }
        assert_eq!(machine.board.peek(0xc000), 123);
        assert_eq!(machine.board.firmware_page(), 0);
    }
}
