//! Original generated hardware vectors. No external ROM/fixture is used.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_commodore_machine::{
    Error, Firmware,
    cia::Cia,
    input::{Input, Joystick, Key},
    pal::{AUDIO_LIMIT, MAX_CYCLES, Machine, PalBus, Pins, State},
    sid::{CLOCK_HZ, Sid},
    vic::{FRAME_CYCLES, WIDTH},
};
use eightbit_mos6502_core::{Bus, Error as CpuError};
const BASIC: [u8; 8192] = [0; 8192];
const CHAR: [u8; 4096] = [0xaa; 4096];
fn firmware(kernal: &[u8; 8192]) -> Firmware<'_> {
    Firmware::new(&BASIC, kernal, &CHAR).unwrap()
}
fn screen_setup(bus: &mut PalBus<'_>) {
    bus.write(0xd011, 0x1b).unwrap();
    bus.write(0xd016, 8).unwrap();
    bus.write(0xd018, 0x14).unwrap();
    bus.write(0xd020, 6).unwrap();
    bus.write(0xd021, 0).unwrap();
    bus.initialise_ram(0x400, &[1; 1000]).unwrap();
    for index in 0..1000 {
        bus.write(0xd800 + index, 7).unwrap();
    }
}
fn frame(bus: &mut PalBus<'_>) {
    bus.advance_idle(FRAME_CYCLES * 2).unwrap();
}
fn pixel(bus: &PalBus<'_>, x: usize, y: usize) -> u8 {
    bus.hardware().video().frame()[(y - 14) * WIDTH + x + 8]
}

#[test]
fn cia_timer_latches_force_load_one_shot_mask_and_ack() {
    let mut cia = Cia::default();
    cia.write(4, 1);
    cia.write(5, 0);
    assert_eq!(cia.read(4, [255; 2]), 1);
    cia.write(13, 0x81);
    cia.write(14, 0x19);
    assert_eq!(cia.read(14, [255; 2]), 9);
    cia.tick();
    assert!(!cia.irq());
    cia.tick();
    assert!(cia.irq());
    assert_eq!(cia.read(14, [255; 2]), 8);
    assert_eq!(cia.read(13, [255; 2]), 0x81);
    assert!(!cia.irq());
    assert_eq!(cia.read(4, [255; 2]), 1);
    cia.tick();
    cia.tick();
    assert_eq!(cia.read(13, [255; 2]), 0);
    cia.write(14, 0x11);
    cia.tick();
    cia.tick();
    assert!(cia.irq());
    cia.write(13, 1);
    assert!(!cia.irq());
    assert_eq!(cia.read(13, [255; 2]), 1);
}
#[test]
fn cia_timer_b_cascade_and_cnt_modes_are_independent() {
    let mut cia = Cia::default();
    for register in 4..=7 {
        cia.write(register, 0);
    }
    cia.write(13, 0x82);
    cia.write(14, 0x11);
    cia.write(15, 0x51);
    cia.tick();
    assert!(cia.irq());
    assert_eq!(cia.read(13, [255; 2]), 0x83);
    cia.write(14, 0x31);
    cia.write(15, 0);
    cia.tick();
    assert_eq!(cia.read(13, [255; 2]), 0);
    cia.pins(false, true, true);
    cia.pins(true, true, true);
    assert_eq!(cia.read(13, [255; 2]), 1);
}
#[test]
fn cia_pending_sources_survive_disabled_mask_and_flag_is_edge_triggered() {
    let mut cia = Cia::default();
    cia.pins(true, true, false);
    assert!(!cia.irq());
    cia.write(13, 0x90);
    assert!(cia.irq());
    assert_eq!(cia.read(13, [255; 2]), 0x90);
    cia.pins(true, true, false);
    assert_eq!(cia.read(13, [255; 2]), 0);
    cia.pins(true, true, true);
    cia.pins(true, true, false);
    assert_eq!(cia.read(13, [255; 2]), 0x90);
}
#[test]
fn cia_serial_input_output_and_timer_pin_modes() {
    let mut cia = Cia::default();
    cia.write(13, 0x88);
    for bit in (0..8).rev() {
        cia.pins(false, 0xa5 & (1 << bit) != 0, true);
        cia.pins(true, 0xa5 & (1 << bit) != 0, true);
    }
    assert_eq!(cia.read(12, [255; 2]), 0xa5);
    assert_eq!(cia.read(13, [255; 2]), 0x88);
    cia.write(4, 0);
    cia.write(5, 0);
    cia.write(14, 0x57);
    cia.write(12, 0x69);
    let mut bits = 0;
    let mut last = false;
    for _ in 0..16 {
        cia.tick();
        let (cnt, sp) = cia.serial_outputs();
        if cnt && !last {
            bits = (bits << 1) | u8::from(sp);
        }
        last = cnt;
    }
    assert_eq!(bits, 0x69);
    assert_eq!(cia.read(13, [255; 2]) & 0x88, 0x88);
    assert_ne!(cia.driven_port(1) & 64, 0);
}
#[test]
fn cia_tod_50hz_alarm_latching_and_noon_transition() {
    let mut cia = Cia::default();
    cia.write(14, 0x80);
    for (r, v) in [(11, 0x11), (10, 0x59), (9, 0x59), (8, 9)] {
        cia.write(r, v);
    }
    cia.write(15, 0x80);
    for (r, v) in [(11, 0x92), (10, 0), (9, 0), (8, 0)] {
        cia.write(r, v);
    }
    cia.write(15, 0);
    cia.write(13, 0x84);
    assert_eq!(cia.read(11, [255; 2]), 0x11);
    for _ in 0..5 {
        cia.tod_pulse();
    }
    assert!(cia.irq());
    assert_eq!(cia.read(9, [255; 2]), 0x59);
    assert_eq!(cia.read(8, [255; 2]), 9);
    assert_eq!(cia.read(11, [255; 2]), 0x92);
    assert_eq!(cia.read(8, [255; 2]), 0);
    cia.write(11, 1);
    for _ in 0..100 {
        cia.tod_pulse();
    }
    assert_eq!(cia.read(8, [255; 2]), 0);
}
#[test]
fn hostile_cia_register_values_remain_bounded_and_mirrored() {
    let mut cia = Cia::default();
    for register in 0..=255 {
        cia.write(register, 255);
        let _ = cia.read(register, [255; 2]);
    }
    cia.write(11, 0xff);
    cia.write(8, 15);
    for _ in 0..100_000 {
        cia.tick();
        cia.tod_pulse();
    }
    assert_eq!(cia.read(8, [255; 2]) & 0xf0, 0);
}
#[test]
fn every_keyboard_switch_scans_both_directions_and_ghosts() {
    for row in 0..8 {
        for col in 0..8 {
            let mut input = Input::default();
            let key = Key::new(row, col).unwrap();
            input.key(key, true);
            assert_eq!(input.levels(!(1 << row), 255)[1], !(1 << col));
            assert_eq!(input.levels(255, !(1 << col))[0], !(1 << row));
            input.release();
            assert_eq!(input.levels(255, 255), [255, 255]);
        }
    }
    let mut input = Input::default();
    for (r, c) in [(0, 0), (0, 1), (1, 1)] {
        input.key(Key::new(r, c).unwrap(), true);
    }
    assert_eq!(input.levels(0xfe, 255), [0xfc, 0xfc]);
    assert!(Key::new(8, 0).is_err());
    assert!(Key::new(0, 8).is_err());
    assert!(Joystick::new(32).is_err());
    assert!(input.joystick(0, Joystick::default()).is_err());
}
#[test]
fn joysticks_and_cia_output_ports_share_real_wires() {
    let mut input = Input::default();
    input.joystick(1, Joystick::new(17).unwrap()).unwrap();
    input.joystick(2, Joystick::new(10).unwrap()).unwrap();
    assert_eq!(input.levels(255, 255), [0xf5, 0xee]);
    let mut cia = Cia::default();
    cia.write(2, 255);
    cia.write(0, 0xaa);
    assert_eq!(cia.read(0, [0xfe, 255]), 0xaa);
}

#[test]
fn pal_frame_clock_readback_raster_irq_and_ack() {
    let mut bus = PalBus::new(firmware(&BASIC));
    assert_eq!(bus.hardware().video().position(), (0, 0));
    bus.advance_idle(FRAME_CYCLES).unwrap();
    assert_eq!(bus.hardware().video().frames(), 1);
    assert_eq!(bus.hardware().video().position(), (0, 0));
    bus.write(0xd012, 10).unwrap();
    bus.write(0xd011, 0).unwrap();
    bus.write(0xd01a, 1).unwrap();
    bus.write(0xd019, 15).unwrap();
    bus.advance_idle(63 * 11).unwrap();
    assert!(bus.interrupt_lines().unwrap().irq);
    assert_eq!(bus.read(0xd019).unwrap() & 0x81, 0x81);
    bus.write(0xd019, 1).unwrap();
    assert!(!bus.interrupt_lines().unwrap().irq);
    assert!(bus.advance_idle(0).is_err());
    assert!(bus.advance_idle(MAX_CYCLES + 1).is_err());
}
#[test]
fn bad_line_cpu_hold_advances_both_cias_and_sid_clock() {
    let mut bus = PalBus::new(firmware(&BASIC));
    screen_setup(&mut bus);
    for (register, value) in [(0xdc04, 255), (0xdc05, 255), (0xdc0e, 0x11)] {
        bus.write(register, value).unwrap();
    }
    let position = bus.hardware().video().position();
    let now = u32::from(position.0) * 63 + u32::from(position.1);
    let target = 51 * 63 + 10;
    bus.advance_idle((target + FRAME_CYCLES - now) % FRAME_CYCLES)
        .unwrap();
    let counter_before = bus.read(0xdc04).unwrap();
    bus.take_audio();
    assert!(bus.hardware().video().ba_low());
    assert!(!bus.hardware().video().aec_low());
    let before = bus.cycles();
    let counter_after = bus.read(0xdc04).unwrap();
    assert_eq!(bus.cycles() - before, 44);
    assert_eq!(bus.stalls(), 43);
    assert_eq!(bus.hardware().video().position(), (51, 55));
    assert_eq!(counter_before.wrapping_sub(counter_after), 44);
    assert!((1..=2).contains(&bus.take_audio().len()));
}

#[test]
fn bad_line_ba_lead_allows_three_pending_writes_before_the_read_hold() {
    let mut bus = PalBus::new(firmware(&BASIC));
    screen_setup(&mut bus);
    let (line, cycle) = bus.hardware().video().position();
    let current = u32::from(line) * 63 + u32::from(cycle);
    bus.advance_idle((51 * 63 + 11 + FRAME_CYCLES - current) % FRAME_CYCLES)
        .unwrap();
    let before = bus.cycles();
    for address in 0x100..0x103 {
        bus.write(address, 0x42).unwrap();
    }
    assert_eq!(bus.cycles() - before, 3);
    assert_eq!(bus.stalls(), 0);
    assert!(bus.hardware().video().aec_low());
    bus.read(0x200).unwrap();
    assert_eq!(bus.stalls(), 40);
}

#[test]
fn all_eight_sprite_dma_slots_span_the_pal_line_boundary() {
    let mut bus = PalBus::new(firmware(&BASIC));
    for index in 0..8 {
        bus.write(0xd001 + index * 2, 100).unwrap();
    }
    bus.write(0xd015, 255).unwrap();
    let (line, cycle) = bus.hardware().video().position();
    let current = u32::from(line) * 63 + u32::from(cycle);
    bus.advance_idle((100 * 63 + 54 + FRAME_CYCLES - current) % FRAME_CYCLES)
        .unwrap();
    assert!(bus.hardware().video().ba_low());
    assert!(!bus.hardware().video().aec_low());
    bus.advance_idle(3).unwrap();
    assert!(bus.hardware().video().aec_low());
    let before = bus.cycles();
    bus.read(0x200).unwrap();
    assert_eq!(bus.cycles() - before, 17);
    assert_eq!(bus.stalls(), 16);
    assert_eq!(bus.hardware().video().position(), (101, 11));
}

#[test]
fn vertical_scroll_moves_character_rows_without_changing_ram() {
    let mut characters = [0; 4096];
    for row in 0..8 {
        characters[8 + row] = 0x80 >> row;
    }
    let mut bus = PalBus::new(Firmware::new(&BASIC, &BASIC, &characters).unwrap());
    screen_setup(&mut bus);
    frame(&mut bus);
    assert_eq!((pixel(&bus, 24, 51), pixel(&bus, 25, 52)), (7, 7));
    bus.write(0xd011, 0x1c).unwrap();
    frame(&mut bus);
    assert_eq!((pixel(&bus, 24, 51), pixel(&bus, 24, 52)), (0, 7));
    bus.write(0xd011, 0x18).unwrap();
    frame(&mut bus);
    assert_eq!((pixel(&bus, 24, 51), pixel(&bus, 27, 51)), (0, 7));
}

#[test]
fn cia_cnt_gated_cascade_and_parallel_strobe_are_clocked() {
    let mut cia = Cia::default();
    for register in 4..=7 {
        cia.write(register, 0);
    }
    cia.write(14, 0x31);
    cia.write(15, 0x71);
    cia.pins(false, true, true);
    cia.pins(true, true, true);
    assert_eq!(cia.read(13, [255; 2]) & 3, 3);
    assert!(cia.parallel_strobe());
    cia.write(1, 0);
    assert!(!cia.parallel_strobe());
    cia.tick();
    assert!(cia.parallel_strobe());
    cia.read(1, [255; 2]);
    assert!(!cia.parallel_strobe());
}

#[test]
fn vic_port_ram_bus_and_cycle_boundary_reads_are_observable() {
    let mut bus = PalBus::new(firmware(&BASIC));
    bus.initialise_ram(0x3fff, &[0xa5]).unwrap();
    // Cycles 16/17 are idle reads while the display is blank; earlier slots
    // fetch sprite pointers even when their sprites are disabled.
    bus.advance_idle(15).unwrap();
    bus.write(0, 7).unwrap();
    assert_eq!(bus.ram(0), 0xa5);
    bus.write(1, 0).unwrap();
    assert_eq!(bus.ram(1), 0xa5);
    // Bank I/O back in, then read the last and first cycles of a raster line.
    bus.write(1, 7).unwrap();
    let (line, cycle) = bus.hardware().video().position();
    assert_eq!(line, 0);
    bus.advance_idle(u32::from(62 - cycle)).unwrap();
    assert_eq!(bus.read(0xd012).unwrap(), 0);
    assert_eq!(bus.read(0xd012).unwrap(), 1);
}
#[test]
fn hires_text_bitmap_multicolour_and_extended_background_pixels() {
    let mut bus = PalBus::new(firmware(&BASIC));
    screen_setup(&mut bus);
    frame(&mut bus);
    assert_eq!(
        (pixel(&bus, 24, 51), pixel(&bus, 25, 51), pixel(&bus, 0, 51)),
        (7, 0, 6)
    );
    bus.write(0xd011, 0x3b).unwrap();
    bus.write(0xd018, 0x18).unwrap();
    bus.initialise_ram(0x2000, &[0x80; 8000]).unwrap();
    bus.initialise_ram(0x400, &[0x25; 1000]).unwrap();
    frame(&mut bus);
    assert_eq!((pixel(&bus, 24, 51), pixel(&bus, 25, 51)), (2, 5));
    bus.write(0xd016, 0x18).unwrap();
    bus.initialise_ram(0x2000, &[0x1b; 8000]).unwrap();
    frame(&mut bus);
    assert_eq!(
        (
            pixel(&bus, 24, 51),
            pixel(&bus, 26, 51),
            pixel(&bus, 28, 51),
            pixel(&bus, 30, 51)
        ),
        (0, 2, 5, 7)
    );
    bus.write(0xd011, 0x5b).unwrap();
    bus.write(0xd016, 8).unwrap();
    bus.write(0xd018, 0x14).unwrap();
    bus.write(0xd022, 3).unwrap();
    bus.initialise_ram(0x400, &[0x41; 1000]).unwrap();
    frame(&mut bus);
    assert_eq!((pixel(&bus, 24, 51), pixel(&bus, 25, 51)), (7, 3));
    bus.write(0xd016, 0x18).unwrap();
    frame(&mut bus);
    assert!(bus.hardware().video().invalid_mode_seen());
    assert_eq!(pixel(&bus, 24, 51), 0);
}
#[test]
fn multicolour_text_and_horizontal_scroll_use_colour_high_bit() {
    let mut bus = PalBus::new(firmware(&BASIC));
    screen_setup(&mut bus);
    bus.write(0xd016, 0x18).unwrap();
    bus.write(0xd022, 2).unwrap();
    bus.write(0xd023, 3).unwrap();
    for index in 0..1000 {
        bus.write(0xd800 + index, 15).unwrap();
    }
    frame(&mut bus);
    assert_eq!((pixel(&bus, 24, 51), pixel(&bus, 25, 51)), (3, 3));
    bus.write(0xd016, 0x19).unwrap();
    frame(&mut bus);
    assert_eq!((pixel(&bus, 24, 51), pixel(&bus, 25, 51)), (0, 3));
}
#[test]
fn sprite_priority_collision_expansion_and_multicolour() {
    let mut bus = PalBus::new(firmware(&BASIC));
    screen_setup(&mut bus);
    bus.initialise_ram(0x7f8, &[13, 13]).unwrap();
    bus.initialise_ram(13 * 64, &[0xff; 63]).unwrap();
    for (r, v) in [
        (0xd000, 80),
        (0xd001, 100),
        (0xd002, 80),
        (0xd003, 100),
        (0xd027, 2),
        (0xd028, 3),
        (0xd015, 3),
        (0xd01a, 6),
    ] {
        bus.write(r, v).unwrap();
    }
    frame(&mut bus);
    assert_eq!(pixel(&bus, 80, 101), 2);
    assert_eq!(bus.read(0xd01e).unwrap(), 3);
    assert_eq!(bus.read(0xd01e).unwrap(), 0);
    assert_eq!(bus.read(0xd01f).unwrap(), 3);
    assert!(bus.interrupt_lines().unwrap().irq);
    bus.write(0xd01b, 1).unwrap();
    frame(&mut bus);
    assert_eq!(pixel(&bus, 80, 101), 7);
    assert_eq!(pixel(&bus, 81, 101), 2);
    bus.write(0xd01b, 0).unwrap();
    bus.write(0xd01d, 1).unwrap();
    bus.write(0xd017, 1).unwrap();
    frame(&mut bus);
    assert_eq!(pixel(&bus, 120, 135), 2);
    bus.write(0xd01c, 1).unwrap();
    bus.write(0xd026, 5).unwrap();
    frame(&mut bus);
    assert_eq!(pixel(&bus, 80, 101), 5);
}
#[test]
fn light_pen_latches_once_per_frame_and_vic_bank_pins_are_inverted() {
    let mut bus = PalBus::new(firmware(&BASIC));
    bus.write(0xd01a, 8).unwrap();
    bus.light_pen();
    let first = bus.read(0xd013).unwrap();
    bus.advance_idle(50).unwrap();
    bus.light_pen();
    assert_eq!(bus.read(0xd013).unwrap(), first);
    assert!(bus.interrupt_lines().unwrap().irq);
    bus.advance_idle(FRAME_CYCLES).unwrap();
    bus.light_pen();
    assert_ne!(bus.read(0xd013).unwrap(), first);
    screen_setup(&mut bus);
    bus.initialise_ram(0x4400, &[0xff; 1000]).unwrap();
    bus.initialise_ram(0x5000, &[0; 4096]).unwrap();
    bus.write(0xdd02, 3).unwrap();
    bus.write(0xdd00, 2).unwrap();
    frame(&mut bus);
    assert_eq!(pixel(&bus, 24, 51), 0);
}
#[test]
fn machine_side_cassette_iec_user_pins_and_restore_are_explicit() {
    let mut bus = PalBus::new(firmware(&BASIC));
    bus.write(0, 0x2f).unwrap();
    bus.write(1, 7).unwrap();
    let outputs = bus.outputs();
    assert!(outputs.cassette_motor);
    assert!(!outputs.cassette_write);
    bus.write(1, 0x2f).unwrap();
    assert!(!bus.outputs().cassette_motor);
    assert!(bus.outputs().cassette_write);
    bus.pins(Pins {
        cassette_sense: false,
        cassette_read: false,
        user_parallel: 0x5a,
        user_flag: false,
        restore: true,
        ..Pins::default()
    });
    assert_eq!(bus.read(1).unwrap() & 16, 0);
    assert_eq!(bus.read(0xdc0d).unwrap() & 16, 16);
    assert_eq!(bus.read(0xdd01).unwrap(), 0x5a);
    assert!(bus.interrupt_lines().unwrap().nmi);
    bus.write(0xdd02, 0xff).unwrap();
    bus.write(0xdd00, 0x1f).unwrap();
    assert!(!bus.outputs().iec_clock);
    assert!(bus.outputs().iec_data);
    bus.release_inputs();
    assert!(!bus.interrupt_lines().unwrap().nmi);
}
#[test]
fn machine_lifecycle_budget_cancellation_fault_and_reset_recover() {
    let mut kernal = [0xea; 8192];
    kernal[0x1ffc] = 0;
    kernal[0x1ffd] = 2;
    let mut machine = Machine::new(firmware(&kernal));
    machine
        .bus_mut()
        .initialise_ram(0x200, &[0x4c, 0, 2])
        .unwrap();
    assert_eq!(machine.state(), State::Stopped);
    assert!(machine.advance(1, || false).is_err());
    machine.reset().unwrap();
    assert_eq!(machine.advance(100, || true).unwrap().cycles, 0);
    assert!(machine.advance(0, || false).is_err());
    assert!(machine.advance(1000, || false).unwrap().cycles >= 1000);
    machine.stop();
    machine.resume().unwrap();
    machine.bus_mut().initialise_ram(0x200, &[2]).unwrap();
    assert!(matches!(
        machine.advance(100, || false),
        Err(CpuError::UnsupportedOpcode { .. })
    ));
    assert_eq!(machine.state(), State::Faulted);
    assert!(machine.bus_mut().take_audio().is_empty());
    assert_eq!(machine.resume(), Err(Error::MachineState));
    machine
        .bus_mut()
        .initialise_ram(0x200, &[0x4c, 0, 2])
        .unwrap();
    machine.reset().unwrap();
    machine.advance(100, || false).unwrap();
    machine.bus_mut().advance_idle(MAX_CYCLES).unwrap();
    assert_eq!(machine.bus_mut().take_audio().len(), AUDIO_LIMIT);
    assert!(machine.bus_mut().take_audio().is_empty());
}

fn tone(voice: u8, wave: u8) -> Sid {
    let mut sid = Sid::default();
    let offset = voice * 7;
    for (r, v) in [
        (offset, 69),
        (offset + 1, 29),
        (offset + 2, 0),
        (offset + 3, 8),
        (offset + 5, 0),
        (offset + 6, 240),
        (24, 15),
        (offset + 4, wave | 1),
    ] {
        sid.write(r, v);
    }
    sid
}
#[test]
fn three_sid_voices_waveforms_and_pitch_follow_guest_registers() {
    for voice in 0..3 {
        for waveform in [16, 32, 64, 128] {
            let mut sid = tone(voice, waveform);
            let mut audio = Vec::new();
            for _ in 0..CLOCK_HZ {
                if let Some(v) = sid.tick() {
                    audio.push(v);
                }
            }
            assert_eq!(audio.len(), 44100);
            assert!(audio.iter().any(|v| *v > 500));
            assert!(audio.iter().any(|v| *v < -500));
            if waveform != 128 {
                let crossings = audio[22050..]
                    .windows(2)
                    .filter(|p| p[0] <= 0 && p[1] > 0)
                    .count();
                assert!((219..=221).contains(&crossings));
            }
        }
    }
}
#[test]
fn sid_gate_envelope_test_readback_filter_and_volume_cleanup() {
    let mut sid = tone(2, 32);
    for _ in 0..3000 {
        sid.tick();
    }
    assert_eq!(sid.read(28), 255);
    assert_ne!(sid.read(27), 0);
    sid.write(18, 0x29);
    assert_eq!(sid.read(27), 0);
    sid.write(18, 0x20);
    for _ in 0..30_000 {
        sid.tick();
    }
    assert_eq!(sid.read(28), 0);
    sid.inputs([12, 34], 0);
    for _ in 0..512 {
        sid.tick();
    }
    assert_eq!((sid.read(25), sid.read(26)), (12, 34));
    let mut direct = tone(0, 32);
    let mut filtered = tone(0, 32);
    filtered.write(21, 7);
    filtered.write(22, 15);
    filtered.write(23, 1);
    filtered.write(24, 0x1f);
    let mut difference = 0;
    for _ in 0..50_000 {
        if direct.tick() != filtered.tick() {
            difference += 1;
        }
    }
    assert!(difference > 1000);
    filtered.write(24, 0);
    for _ in 0..1000 {
        assert!(filtered.tick().is_none_or(|v| v == 0));
    }
}
