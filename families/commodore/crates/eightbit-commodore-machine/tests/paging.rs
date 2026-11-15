//! Generated original images exercise the real CPU, public bus and PAL lifecycle.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[allow(dead_code)]
#[path = "../examples/support/paging.rs"]
mod fixture;
use eightbit_commodore_machine::{
    Error, Firmware,
    original::{Image, MAX_DEPTH},
    pal::{Machine, State},
};
use eightbit_mos6502_core::{Bus, C, Cpu, D, Error as CpuError, I, Registers, U};
use fixture::{hash, memory, pack, parts};

fn test_cpu() -> Cpu {
    Cpu::from_registers(Registers {
        pc: 0xe009,
        sp: 0xf8,
        y: 0x77,
        p: C | D | U,
        ..Registers::default()
    })
}
fn run_to(cpu: &mut Cpu, m: &mut eightbit_commodore_machine::Memory<'_>, pc: u16) {
    for _ in 0..400 {
        if cpu.registers().pc == pc {
            return;
        }
        let before = (cpu.registers(), m.original_page());
        cpu.step(m)
            .unwrap_or_else(|e| panic!("Gate checkpoint {before:?}: {e}"));
    }
    panic!("Original fixture did not reach bounded checkpoint {pc:04x}");
}
#[test]
fn container_rejects_hostile_lengths_versions_counts_hashes_and_trailing_bytes() {
    let bytes = fixture::fixture();
    assert_eq!(Image::parse(&bytes, &hash(&bytes)).unwrap().page_count(), 3);
    for n in [0, 1, 159, 160, bytes.len() - 1] {
        assert!(Image::parse(&bytes[..n], &hash(&bytes[..n])).is_err());
    }
    assert!(matches!(
        Image::parse(&bytes, &[0; 32]),
        Err(Error::OriginalIdentity)
    ));
    for (offset, value) in [
        (0, 0),
        (8, 2),
        (10, 0),
        (10, 17),
        (11, 129),
        (12, 255),
        (14, 1),
        (152, 1),
        (144, 0),
        (149, 0),
    ] {
        let mut changed = bytes.clone();
        changed[offset] = value;
        assert!(
            Image::parse(&changed, &hash(&changed)).is_err(),
            "accepted header byte {offset}"
        );
    }
    let mut changed = bytes.clone();
    changed.push(0);
    assert!(Image::parse(&changed, &hash(&changed)).is_err());
    for offset in [16, 48, 80, 112, 160, bytes.len() - 1] {
        let mut changed = bytes.clone();
        changed[offset] ^= 1;
        assert!(Image::parse(&changed, &hash(&bytes)).is_err());
    }
    for offset in [80, 112, 160, bytes.len() - 1] {
        let mut changed = bytes.clone();
        changed[offset] ^= 1;
        assert!(
            Image::parse(&changed, &hash(&changed)).is_err(),
            "internal binding {offset}"
        );
    }
}

#[test]
fn maximum_page_gate_and_public_range_counts_are_validated_without_unbounded_metadata() {
    use eightbit_commodore_machine::original::{Gate, GateKind};
    let pages = vec![0; 16 * 8192];
    let mut kernal = vec![0xea; 8192];
    let mut gates = Vec::new();
    let mut public = Vec::new();
    for index in 0..128u16 {
        let gate = Gate {
            entry: 0xe000 + index * 32,
            target: 0xa800,
            page: u8::try_from(index % 16).unwrap(),
            kind: GateKind::Call,
        };
        let offset = usize::from(gate.entry - 0xe000);
        kernal[offset..offset + 22].copy_from_slice(&gate.code());
        gates.push(gate);
        public.push((0xa000 + index * 16, 4));
    }
    let bytes = pack(&pages, &kernal, &[0; 4096], &gates, &public);
    assert_eq!(
        Image::parse(&bytes, &hash(&bytes)).unwrap().page_count(),
        16
    );
    assert_eq!(bytes.len(), 145_632);
    let oversized = vec![0; eightbit_commodore_machine::original::MAX_IMAGE + 1];
    assert!(Image::parse(&oversized, &hash(&oversized)).is_err());
}
#[test]
fn internally_rehashed_images_still_need_valid_templates_maps_and_bounded_targets() {
    let (source_pages, source_kernal, characters, source_gates) = parts();
    for broken in 0..6 {
        let (mut pages, mut kernal, mut gates) = (
            source_pages.clone(),
            source_kernal.clone(),
            source_gates.clone(),
        );
        match broken {
            0 => gates[0].page = 3,
            1 => gates[0].entry = 0xfff0,
            2 => gates[0].target = 0xc000,
            3 => gates[1] = gates[0],
            4 => kernal[0x104] = 0xea,
            _ => pages[8192] ^= 1,
        }
        let bytes = pack(&pages, &kernal, &characters, &gates, &[(0xa000, 4)]);
        assert!(Image::parse(&bytes, &hash(&bytes)).is_err());
    }
    for public in [
        vec![(0x9fff, 3)],
        vec![(0xbfff, 3)],
        vec![(0xa000, 0)],
        vec![(0xa000, 4), (0xa001, 3)],
    ] {
        let bytes = pack(
            &source_pages,
            &source_kernal,
            &characters,
            &source_gates,
            &public,
        );
        assert!(Image::parse(&bytes, &hash(&bytes)).is_err());
    }
}
#[test]
fn generated_nested_calls_restore_pages_flags_registers_and_public_stub_returns() {
    let bytes = fixture::fixture();
    for entry in [0xe100u16, 0xa010] {
        let mut m = memory(&bytes).unwrap();
        m.initialise_ram(
            0x200,
            &[0x20, entry.to_le_bytes()[0], entry.to_le_bytes()[1], 0xea],
        )
        .unwrap();
        let mut cpu = Cpu::from_registers(Registers {
            pc: 0x200,
            sp: 0xf8,
            y: 0x77,
            p: C | D | U,
            ..Registers::default()
        });
        run_to(&mut cpu, &mut m, 0x203);
        assert_eq!(m.original_page(), Some((0, 0)));
        assert_eq!((m.ram(0xc000), m.ram(0xc001)), (0x42, 0x43));
        assert_eq!(
            (cpu.registers().a, cpu.registers().y, cpu.registers().sp),
            (0x43, 0x77, 0xf8)
        );
        assert_eq!(cpu.registers().p, C | D | U);
    }
}
#[test]
fn nmi_at_every_gate_instruction_and_nested_nmi_preserve_the_interrupted_sequence() {
    let bytes = fixture::fixture();
    for cut in 0..45 {
        let mut m = memory(&bytes).unwrap();
        let mut cpu = test_cpu();
        for _ in 0..cut {
            cpu.step(&mut m).unwrap();
        }
        cpu.set_nmi(true);
        cpu.step(&mut m).unwrap();
        cpu.set_nmi(false);
        run_to(&mut cpu, &mut m, 0xe00c);
        assert_eq!(m.original_page(), Some((0, 0)), "cut {cut}");
        assert_eq!(m.ram(0xc003), 1);
    }
    let mut m = memory(&bytes).unwrap();
    let mut cpu = test_cpu();
    run_to(&mut cpu, &mut m, 0xe107);
    cpu.set_nmi(true);
    cpu.step(&mut m).unwrap();
    cpu.set_nmi(false);
    cpu.step(&mut m).unwrap();
    cpu.set_nmi(true);
    cpu.step(&mut m).unwrap();
    cpu.set_nmi(false);
    run_to(&mut cpu, &mut m, 0xe00c);
    assert_eq!(m.ram(0xc003), 2);
    assert_eq!(m.original_page(), Some((0, 0)));
    for cut in 0..16 {
        let mut memory = memory(&bytes).unwrap();
        let mut cpu = test_cpu();
        run_to(&mut cpu, &mut memory, 0xe107);
        cpu.set_nmi(true);
        cpu.step(&mut memory).unwrap();
        cpu.set_nmi(false);
        for _ in 0..cut {
            cpu.step(&mut memory).unwrap();
        }
        cpu.set_nmi(true);
        cpu.step(&mut memory).unwrap();
        cpu.set_nmi(false);
        run_to(&mut cpu, &mut memory, 0xe00c);
        assert_eq!(memory.ram(0xc003), 2, "nested NMI cut {cut}");
        assert_eq!(memory.original_page(), Some((0, 0)));
    }
}
#[test]
fn irq_and_brk_restore_page_and_normal_guest_stack() {
    let bytes = fixture::fixture();
    let mut m = memory(&bytes).unwrap();
    let mut cpu = test_cpu();
    run_to(&mut cpu, &mut m, 0xa100);
    cpu.set_irq(true);
    cpu.step(&mut m).unwrap();
    cpu.set_irq(false);
    run_to(&mut cpu, &mut m, 0xe00c);
    assert_eq!(m.ram(0xc002), 1);
    assert_eq!(cpu.registers().sp, 0xf8);
    let (mut source_pages, source_kernal, characters, source_gates) = parts();
    source_pages[8192 + 0x100..8192 + 0x103].copy_from_slice(&[0x00, 0xea, 0x60]);
    let bytes = pack(
        &source_pages,
        &source_kernal,
        &characters,
        &source_gates,
        &[(0xa000, 4), (0xa010, 3)],
    );
    let mut m = memory(&bytes).unwrap();
    let mut cpu = test_cpu();
    run_to(&mut cpu, &mut m, 0xe00c);
    assert_eq!(m.ram(0xc002), 1);
    assert_eq!(m.original_page(), Some((0, 0)));
}
#[test]
fn all_ram_and_vic_banking_remain_physical_while_private_page_is_selected() {
    let bytes = fixture::fixture();
    let mut m = memory(&bytes).unwrap();
    let mut cpu = test_cpu();
    m.initialise_ram(0xa000, &[0x71]).unwrap();
    m.initialise_ram(0xffff, &[0x72]).unwrap();
    m.initialise_ram(0x1000, &[0x73]).unwrap();
    run_to(&mut cpu, &mut m, 0xa100);
    assert_eq!(m.original_page(), Some((1, 1)));
    for pins in 0..8 {
        m.write(1, pins).unwrap();
        assert_eq!(m.read(0xbfff).unwrap(), u8::from(pins & 3 == 3));
        assert_eq!(m.ram(0xa000), 0x71);
        assert_eq!(m.ram(0xffff), 0x72);
        assert_eq!(m.vic_read(0, 0x1000), 0x5a);
    }
    m.write(1, 7).unwrap();
    run_to(&mut cpu, &mut m, 0xe00c);
    assert_eq!(m.original_page(), Some((0, 0)));
}
#[test]
fn jumping_into_a_gate_or_stale_return_faults_and_revokes_page_state() {
    let bytes = fixture::fixture();
    for target in [0xe100, 0xe104, 0xe110, 0xe115] {
        let mut m = memory(&bytes).unwrap();
        let mut cpu = Cpu::from_registers(Registers {
            pc: target,
            sp: 0xf8,
            ..Registers::default()
        });
        assert_eq!(cpu.step(&mut m), Err(CpuError::Bus(Error::OriginalGate)));
        assert!(cpu.is_faulted());
        assert_eq!(m.original_page(), Some((0, 0)));
    }
    let mut m = memory(&bytes).unwrap();
    let mut cpu = test_cpu();
    run_to(&mut cpu, &mut m, 0xa100);
    assert_eq!(m.write(0xde00, 2), Err(Error::UnsupportedIo(0xde00)));
    assert_eq!(m.original_page(), Some((1, 1)));
}

#[test]
fn private_gate_with_hidden_basic_or_io_faults_before_ram_under_io_is_written() {
    let bytes = fixture::fixture();
    for pins in [2, 3, 6] {
        let mut memory = memory(&bytes).unwrap();
        memory.write(1, pins).unwrap();
        memory.initialise_ram(0x200, &[0x20, 0, 0xe1]).unwrap();
        let mut cpu = Cpu::from_registers(Registers {
            pc: 0x200,
            sp: 0xf8,
            ..Registers::default()
        });
        for _ in 0..4 {
            cpu.step(&mut memory).unwrap();
        }
        assert_eq!(
            cpu.step(&mut memory),
            Err(CpuError::Bus(Error::OriginalGate))
        );
        assert_eq!(memory.ram(0xde00), 0);
        assert_eq!(memory.original_page(), Some((0, 0)));
    }
}
#[test]
fn recursive_depth_and_small_stack_fault_without_wrap_or_unbounded_allocation() {
    let (mut source_pages, source_kernal, characters, source_gates) = parts();
    source_pages[8192 + 0x100..8192 + 0x104].copy_from_slice(&[0x20, 0, 0xe1, 0x60]);
    let bytes = pack(
        &source_pages,
        &source_kernal,
        &characters,
        &source_gates,
        &[(0xa000, 4), (0xa010, 3)],
    );
    let mut m = memory(&bytes).unwrap();
    let mut cpu = test_cpu();
    let mut peak = 0;
    let result = (0..1000)
        .find_map(|_| {
            peak = peak.max(m.original_page().unwrap().1);
            cpu.step(&mut m).err()
        })
        .expect("Recursive gate must fault within its bounded fixture");
    assert_eq!(peak, MAX_DEPTH);
    assert_eq!(result, CpuError::Bus(Error::OriginalDepth));
    assert_eq!(m.original_page(), Some((0, 0)));
    let mut m = memory(&bytes).unwrap();
    let mut cpu = Cpu::from_registers(Registers {
        pc: 0xe009,
        sp: 9,
        ..Registers::default()
    });
    assert_eq!(cpu.step(&mut m), Err(CpuError::Bus(Error::OriginalDepth)));
}

#[test]
fn changed_caller_return_and_wrapped_hardware_stack_are_not_valid_continuations() {
    let (mut pages, kernal, characters, gates) = parts();
    // Original callee changes the caller's return below its own JSR frame.
    pages[8192 + 0x100..8192 + 0x105].copy_from_slice(&[0xba, 0xfe, 3, 1, 0x60]);
    let bytes = pack(
        &pages,
        &kernal,
        &characters,
        &gates,
        &[(0xa000, 4), (0xa010, 3)],
    );
    let mut memory = memory(&bytes).unwrap();
    let mut cpu = test_cpu();
    let fault = (0..100)
        .find_map(|_| cpu.step(&mut memory).err())
        .expect("Altered return must fault");
    assert_eq!(fault, CpuError::Bus(Error::OriginalGate));
    assert_eq!(memory.original_page(), Some((0, 0)));
    for sp in [0, 1] {
        let mut memory = fixture::memory(&bytes).unwrap();
        let mut cpu = Cpu::from_registers(Registers {
            pc: 0xe009,
            sp,
            ..Registers::default()
        });
        assert_eq!(
            cpu.step(&mut memory),
            Err(CpuError::Bus(Error::OriginalDepth))
        );
    }
    for sp in [0, 1, 2] {
        let mut memory = fixture::memory(&bytes).unwrap();
        let mut cpu = Cpu::from_registers(Registers {
            pc: 0x200,
            sp,
            p: U,
            ..Registers::default()
        });
        cpu.set_irq(true);
        assert_eq!(
            cpu.step(&mut memory),
            Err(CpuError::Bus(Error::OriginalDepth))
        );
    }
}
#[test]
fn non_local_recovery_revokes_nested_continuations_and_stale_epilogues() {
    let (mut source_pages, source_kernal, characters, source_gates) = parts();
    source_pages[16384 + 0x200..16384 + 0x203].copy_from_slice(&[0x4c, 0, 0xe4]);
    let bytes = pack(
        &source_pages,
        &source_kernal,
        &characters,
        &source_gates,
        &[(0xa000, 4), (0xa010, 3)],
    );
    let mut m = memory(&bytes).unwrap();
    let mut cpu = test_cpu();
    run_to(&mut cpu, &mut m, 0xa400);
    assert_eq!(m.original_page(), Some((0, 0)));
    let mut stale = Cpu::from_registers(Registers {
        pc: 0xe110,
        sp: 0xf0,
        ..Registers::default()
    });
    assert_eq!(stale.step(&mut m), Err(CpuError::Bus(Error::OriginalGate)));
    cpu.reset(&mut m).unwrap();
    assert_eq!(m.original_page(), Some((0, 0)));
    for cut in 0..8 {
        let mut memory = memory(&bytes).unwrap();
        let mut cpu = test_cpu();
        run_to(&mut cpu, &mut memory, 0xe400);
        for _ in 0..cut {
            cpu.step(&mut memory).unwrap();
        }
        cpu.set_nmi(true);
        cpu.step(&mut memory).unwrap();
        cpu.set_nmi(false);
        run_to(&mut cpu, &mut memory, 0xa400);
        assert_eq!(memory.ram(0xc003), 1, "recovery NMI cut {cut}");
        assert_eq!(memory.original_page(), Some((0, 0)));
    }
}
#[test]
fn pal_stop_cancel_resume_reset_and_fault_have_owned_page_audio_and_input_states() {
    let bytes = fixture::fixture();
    let firmware = Firmware::original(Image::parse(&bytes, &hash(&bytes)).unwrap()).unwrap();
    let mut m = Machine::new(firmware);
    assert_eq!(m.state(), State::Stopped);
    m.reset().unwrap();
    for _ in 0..100 {
        if m.bus().original_page() == Some((1, 1)) {
            break;
        }
        m.advance(1, || false).unwrap();
    }
    assert_eq!(m.bus().original_page(), Some((1, 1)));
    m.stop();
    assert_eq!(m.bus().original_page(), Some((1, 1)));
    m.resume().unwrap();
    let registers = m.registers();
    m.bus_mut().key(
        eightbit_commodore_machine::input::Key::new(7, 6).unwrap(),
        true,
    );
    m.bus_mut().advance_idle(10_000).unwrap();
    m.advance(100, || true).unwrap();
    assert_eq!(m.registers(), registers);
    assert_eq!(m.bus().original_page(), Some((1, 1)));
    assert!(m.bus_mut().take_audio().is_empty());
    m.bus_mut().write(0xdc02, 0xff).unwrap();
    m.bus_mut().write(0xdc00, 0x7f).unwrap();
    assert_eq!(m.bus_mut().read(0xdc01).unwrap(), 0xff);
    m.advance(1000, || false).unwrap();
    assert_eq!(m.bus().original_page(), Some((0, 0)));
    m.reset().unwrap();
    assert_eq!(m.bus().original_page(), Some((0, 0)));
    assert_eq!(m.registers().p & I, I);
}
#[test]
fn generated_native_wasm_trace_has_nested_nmi_and_complete_restoration() {
    let trace = fixture::trace().unwrap();
    assert!(trace.contains("Some((2, 2))"));
    assert!(trace.contains("Some((0, 3))"));
}

#[test]
fn pal_guest_opcode_fault_revokes_page_continuations_and_requires_reset() {
    let (mut pages, kernal, characters, gates) = parts();
    pages[8192 + 0x100] = 2;
    let bytes = pack(
        &pages,
        &kernal,
        &characters,
        &gates,
        &[(0xa000, 4), (0xa010, 3)],
    );
    let mut machine =
        Machine::new(Firmware::original(Image::parse(&bytes, &hash(&bytes)).unwrap()).unwrap());
    machine.reset().unwrap();
    assert!(matches!(
        machine.advance(1000, || false),
        Err(CpuError::UnsupportedOpcode {
            address: 0xa100,
            opcode: 2
        })
    ));
    assert_eq!(machine.state(), State::Faulted);
    assert_eq!(machine.bus().original_page(), Some((0, 0)));
    assert!(machine.bus_mut().take_audio().is_empty());
    assert!(machine.resume().is_err());
    machine.reset().unwrap();
    assert_eq!(machine.state(), State::Running);
    assert_eq!(machine.bus().original_page(), Some((0, 0)));
}
