//! Original timed-device line vectors; existing caller-driven buses retain their API.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_mos6502_core::{B, Bus, Cpu, InterruptLines, Registers, StepKind, U};
struct DeviceBus {
    ram: Box<[u8]>,
    clock: usize,
    irq_at: usize,
    nmi_at: usize,
    nmi_delivered: bool,
}
impl Default for DeviceBus {
    fn default() -> Self {
        let mut ram = vec![0xea; 65536].into_boxed_slice();
        ram[0xfffa..0xfffc].copy_from_slice(&[0, 5]);
        ram[0xfffe..].copy_from_slice(&[0, 4]);
        Self {
            ram,
            clock: 0,
            irq_at: usize::MAX,
            nmi_at: usize::MAX,
            nmi_delivered: false,
        }
    }
}
impl Bus for DeviceBus {
    type Error = core::convert::Infallible;
    fn read(&mut self, address: u16) -> Result<u8, Self::Error> {
        self.clock += 1;
        Ok(self.ram[usize::from(address)])
    }
    fn write(&mut self, address: u16, value: u8) -> Result<(), Self::Error> {
        self.clock += 1;
        self.ram[usize::from(address)] = value;
        Ok(())
    }
    fn interrupt_lines(&mut self) -> Option<InterruptLines> {
        let edge = self.clock >= self.nmi_at && !self.nmi_delivered;
        if edge {
            self.nmi_delivered = true;
        }
        Some(InterruptLines {
            irq: self.clock >= self.irq_at,
            nmi: false,
            nmi_edge: edge,
        })
    }
}
fn cpu() -> Cpu {
    Cpu::from_registers(Registers {
        pc: 0x200,
        sp: 0xfd,
        p: U,
        ..Registers::default()
    })
}
#[test]
fn device_irq_uses_penultimate_cycle_and_nmi_pulses_are_latched() {
    for (irq_at, extra) in [(1, false), (2, true)] {
        let mut bus = DeviceBus {
            irq_at,
            ..DeviceBus::default()
        };
        let mut cpu = cpu();
        cpu.step(&mut bus).unwrap();
        if extra {
            assert_eq!(
                cpu.step(&mut bus).unwrap().kind,
                StepKind::Instruction(0xea)
            );
        }
        assert_eq!(cpu.step(&mut bus).unwrap().kind, StepKind::Irq);
    }
    let mut bus = DeviceBus {
        nmi_at: 2,
        ..DeviceBus::default()
    };
    let mut cpu = cpu();
    cpu.step(&mut bus).unwrap();
    assert_eq!(cpu.step(&mut bus).unwrap().kind, StepKind::Nmi);
    assert_eq!(cpu.registers().pc, 0x500);
    assert_eq!(
        cpu.step(&mut bus).unwrap().kind,
        StepKind::Instruction(0xea)
    );
}
#[test]
fn nmi_can_redirect_brk_vector_without_changing_software_stack_marker() {
    let mut bus = DeviceBus {
        nmi_at: 4,
        ..DeviceBus::default()
    };
    bus.ram[0x200] = 0;
    let mut cpu = cpu();
    assert_eq!(cpu.step(&mut bus).unwrap().cycles, 7);
    assert_eq!(cpu.registers().pc, 0x500);
    assert_eq!(bus.ram[0x1fb], U | B);
    assert_eq!((bus.ram[0x1fd], bus.ram[0x1fc]), (2, 2));
}
#[test]
fn external_nmi_before_first_step_is_observed_without_an_extra_instruction() {
    let mut bus = DeviceBus {
        nmi_at: 0,
        ..DeviceBus::default()
    };
    let mut cpu = cpu();
    assert_eq!(cpu.step(&mut bus).unwrap().kind, StepKind::Nmi);
    assert_eq!(bus.clock, 7);
}
