//! Shared original observations. All work/output is statically bounded.
use eightbit_commodore_machine::{Firmware, Memory};
use eightbit_mos6502_core::{Bus, C, Cpu, D, Registers};
use std::fmt::Write;

pub fn trace() -> String {
    let mut output = String::with_capacity(4096);
    let result = observations(&mut output);
    if let Err(error) = result {
        return format!("FAILED: {error}");
    }
    output
}
fn observations(output: &mut String) -> Result<(), Box<dyn std::error::Error>> {
    let basic = [0x21; 8192];
    let mut kernal = [0x42; 8192];
    kernal[0x1ffc] = 0;
    kernal[0x1ffd] = 2;
    let characters = [0x63; 4096];
    let firmware = Firmware::new(&basic, &kernal, &characters)?;
    let mut memory = Memory::new(firmware);
    // Original guest: decimal 49+51, push/pull, bank all RAM, indexed RMW, loop.
    let programme = [
        0xf8, 0x18, 0xa9, 0x49, 0x69, 0x51, 0x48, 0xd8, 0x68, 0xa2, 7, 0x86, 0, 0xa2, 0, 0x86, 1,
        0xa9, 0x80, 0x8d, 0xff, 0x12, 0xa2, 1, 0xfe, 0xfe, 0x12, 0xbd, 0xfe, 0x12, 0x4c, 0x1e, 2,
    ];
    memory.initialise_ram(0x200, &programme)?;
    let mut cpu = Cpu::new();
    writeln!(output, "reset:{}", cpu.reset(&mut memory)?)?;
    for _ in 0..24 {
        let step = cpu.step(&mut memory)?;
        let r = cpu.registers();
        writeln!(
            output,
            "{:04x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{}",
            r.pc, r.a, r.x, r.y, r.sp, r.p, step.cycles
        )?;
    }
    for pins in 0..8 {
        memory.write(0, 7)?;
        memory.write(1, pins)?;
        writeln!(
            output,
            "bank:{pins}:{:02x}:{:02x}:{:?}",
            memory.read(0xa000)?,
            memory.read(0xe000)?,
            memory.read(0xd000)
        )?;
    }
    memory.write(1, 0)?;
    // Entire byte domain for both decimal/binary operators, with a coprime second
    // operand; exhaustive arithmetic is separately checked by ordinary Rust tests.
    for opcode in [0x69, 0xe9] {
        for decimal in [0, D] {
            for a in 0..=255u8 {
                memory.initialise_ram(0x200, &[opcode, a.wrapping_mul(73)])?;
                let mut cpu = Cpu::from_registers(Registers {
                    pc: 0x200,
                    a,
                    p: decimal | C,
                    ..Registers::default()
                });
                cpu.step(&mut memory)?;
                let r = cpu.registers();
                write!(output, "{:02x}{:02x}", r.a, r.p)?;
            }
        }
    }
    writeln!(output, "\nram:{:02x}", memory.ram(0x12ff))?;
    Ok(())
}
