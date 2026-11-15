//! Optional provenance cannot replace guest instructions or add bus cycles.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_mos6502_core::{Bus, Cpu, Error, Execution, Registers, StepKind};

struct Probe {
    ram: Vec<u8>,
    events: Vec<Execution>,
    reads: usize,
    reject: bool,
}
impl Probe {
    fn new() -> Self {
        Self {
            ram: vec![0xea; 65536],
            events: vec![],
            reads: 0,
            reject: false,
        }
    }
}
impl Bus for Probe {
    type Error = &'static str;
    fn read(&mut self, a: u16) -> Result<u8, Self::Error> {
        self.reads += 1;
        Ok(self.ram[usize::from(a)])
    }
    fn write(&mut self, a: u16, v: u8) -> Result<(), Self::Error> {
        self.ram[usize::from(a)] = v;
        Ok(())
    }
    fn execution(&mut self, event: Execution) -> Result<(), Self::Error> {
        self.events.push(event);
        if self.reject && matches!(event, Execution::Instruction(_)) {
            Err("rejected")
        } else {
            Ok(())
        }
    }
}
#[test]
fn observer_reports_real_instruction_interrupt_reset_and_fault_without_extra_cycles() {
    let mut bus = Probe::new();
    bus.ram[0xfffc..0xfffe].copy_from_slice(&[0, 2]);
    bus.ram[0xfffe..].copy_from_slice(&[0, 4]);
    let mut cpu = Cpu::new();
    assert_eq!(cpu.reset(&mut bus), Ok(7));
    assert_eq!(bus.reads, 7);
    assert_eq!(bus.events, [Execution::Reset]);
    assert_eq!(cpu.step(&mut bus).unwrap().cycles, 2);
    assert!(matches!(
        bus.events[1],
        Execution::Instruction(Registers { pc: 0x200, .. })
    ));
    assert!(matches!(
        bus.events[2],
        Execution::Complete {
            kind: StepKind::Instruction(0xea),
            registers: Registers { pc: 0x201, .. }
        }
    ));
    let mut cpu = Cpu::from_registers(Registers {
        pc: 0x200,
        ..Registers::default()
    });
    bus.reject = true;
    let before = bus.reads;
    assert_eq!(cpu.step(&mut bus), Err(Error::Bus("rejected")));
    assert_eq!(bus.reads, before);
    assert_eq!(bus.events.last(), Some(&Execution::Fault));
    assert_eq!(cpu.step(&mut bus), Err(Error::NeedsReset));
    bus.reject = false;
    let mut cpu = Cpu::from_registers(Registers {
        pc: 0x200,
        p: 0,
        ..Registers::default()
    });
    cpu.set_irq(true);
    let before = bus.reads;
    assert_eq!(cpu.step(&mut bus).unwrap().cycles, 7);
    assert_eq!(bus.reads - before, 4);
    assert_eq!(bus.events[bus.events.len() - 2], Execution::Interrupt);
    assert!(matches!(
        bus.events.last(),
        Some(Execution::Complete {
            kind: StepKind::Irq,
            registers: Registers { pc: 0x400, .. }
        })
    ));
}
