//! Synchronous guest inspection and bounded address-breakpoint execution.
//!
//! These operations use the actual CPU and bus. They do not select a runtime,
//! replace firmware, inject BASIC or establish source-level breakpoint validity.

use crate::{MAX_SLICE, Machine};
pub use eightbit_m6809_core::Registers;
use std::collections::BTreeSet;

/// Largest side-effect-free memory read, in bytes.
pub const MAX_DEBUG_MEMORY_BYTES: usize = 4096;
/// Largest set of instruction-address breakpoints per execution slice.
pub const MAX_BREAKPOINTS: usize = 64;

/// Validated address breakpoints. No guest instruction bytes are patched.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Breakpoints(BTreeSet<u16>);

impl Breakpoints {
    /// Validate before replacing the previous set. Duplicates are coalesced.
    pub fn replace(&mut self, addresses: &[u16]) -> Result<(), String> {
        if addresses.len() > MAX_BREAKPOINTS {
            return Err("DME1300: use at most 64 breakpoint addresses".into());
        }
        self.0 = addresses.iter().copied().collect();
        Ok(())
    }

    /// Whether execution must stop before the next CPU step at this address.
    #[must_use]
    pub fn contains(&self, address: u16) -> bool {
        self.0.contains(&address)
    }
}

/// Why a bounded execution slice returned control to its caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugStopReason {
    /// The next CPU step starts at an installed address, without changing RAM.
    Breakpoint,
    /// The requested cycle budget was consumed, with at most one step's excess.
    Budget,
    /// The caller requested cancellation at a CPU-step boundary.
    Cancelled,
}

/// Real guest state at the end of one synchronous execution slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DebugStop {
    /// Cause of this stop, distinct from a verified BASIC/source breakpoint.
    pub reason: DebugStopReason,
    /// CPU cycles consumed during this slice.
    pub cycles: u32,
    /// Registers from this exact stop.
    pub registers: Registers,
}

impl Machine {
    /// Read all CPU registers without executing or accessing the bus.
    #[must_use]
    pub fn registers(&self) -> Registers {
        self.cpu.registers
    }

    /// Read mapped RAM/ROM without advancing clocks or acknowledging interrupts.
    ///
    /// Reads cannot wrap or cross the I/O/SAM aperture ($FF00–$FFDF). Device
    /// registers require a separate explicit inspection contract; returning fake
    /// memory bytes here would hide that distinction. Vector ROM remains readable.
    pub fn read_memory(&self, address: u16, length: usize) -> Result<Vec<u8>, String> {
        let start = usize::from(address);
        let end = start
            .checked_add(length)
            .ok_or("DME1301: memory range overflows")?;
        if length == 0 || length > MAX_DEBUG_MEMORY_BYTES || end > 65_536 {
            return Err("DME1301: use a non-wrapping memory range of 1–4096 bytes".into());
        }
        if start < 0xffe0 && end > 0xff00 {
            return Err("DME1301: memory inspection cannot read device or SAM registers".into());
        }
        (start..end)
            .map(|offset| {
                u16::try_from(offset)
                    .map(|address| self.peek(address))
                    .map_err(|_| "DME1301: memory range overflows".into())
            })
            .collect()
    }

    /// Perform one actual CPU step and preserve the machine's failure latch.
    ///
    /// A step is one instruction, interrupt entry or one idle cycle while the
    /// CPU waits/is halted. It is not a BASIC statement or a source-language step.
    pub fn step(&mut self) -> Result<u32, String> {
        if let Some(error) = self.failure {
            return Err(error.to_string());
        }
        if self.board.paging.fault {
            return Err("DME1601: original Dragon 64 firmware page continuation is invalid".into());
        }
        self.board.instruction_pc = self.cpu.registers.pc;
        self.board.instruction_stack = self.cpu.registers.s;
        self.cpu.step(&mut self.board).map_err(|error| {
            self.failure = Some(error);
            error.to_string()
        })
    }

    /// Run at most one bounded slice, stopping before an address breakpoint.
    ///
    /// Cancellation is checked before every CPU step. An initial breakpoint
    /// returns without executing: callers explicitly `step` to move past it.
    /// Inspection after return belongs to this stopped machine until its owner
    /// next mutates it; there is no background worker or implicit continuation.
    pub fn run_until(
        &mut self,
        cycles: u32,
        breakpoints: &Breakpoints,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<DebugStop, String> {
        if cycles == 0 || cycles > MAX_SLICE {
            return Err("DME1002: use a bounded non-zero execution slice".into());
        }
        if let Some(error) = self.failure {
            return Err(error.to_string());
        }
        let mut elapsed = 0;
        let reason = loop {
            if cancelled() {
                break DebugStopReason::Cancelled;
            }
            if breakpoints.contains(self.cpu.registers.pc) {
                break DebugStopReason::Breakpoint;
            }
            if elapsed >= cycles {
                break DebugStopReason::Budget;
            }
            elapsed += self.step()?;
        };
        Ok(DebugStop {
            reason,
            cycles: elapsed,
            registers: self.registers(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Bus, Firmware};
    use eightbit_dragon_basic_core::inspection::{ScalarWorkspace, inspect_scalars};
    use std::collections::BTreeMap;

    fn authored_machine() -> Machine {
        let mut machine = Machine::new();
        machine.board.sam.control = 0x4000;
        // Authored LDA #$42 / INCA / BRA -3, with known values in every register.
        machine.board.ram[0x6000..0x6005].copy_from_slice(&[0x86, 0x42, 0x4c, 0x20, 0xfd]);
        machine.cpu.registers = Registers {
            a: 0,
            b: 0x12,
            dp: 0x34,
            cc: 0x50,
            x: 0x5678,
            y: 0x9abc,
            u: 0x7000,
            s: 0x7f00,
            pc: 0x6000,
        };
        machine
    }

    #[test]
    fn address_breakpoints_stop_before_execution_without_modifying_guest_bytes()
    -> Result<(), String> {
        let mut machine = authored_machine();
        let mut points = Breakpoints::default();
        points.replace(&[0x6002, 0x6002])?;
        let stop = machine.run_until(MAX_SLICE, &points, || false)?;
        assert_eq!(stop.reason, DebugStopReason::Breakpoint);
        assert_eq!(stop.cycles, 2);
        assert_eq!(
            stop.registers,
            Registers {
                a: 0x42,
                pc: 0x6002,
                ..authored_machine().registers()
            }
        );
        assert_eq!(
            machine.read_memory(0x6000, 5)?,
            [0x86, 0x42, 0x4c, 0x20, 0xfd]
        );
        assert_eq!(machine.run_until(1, &points, || false)?.cycles, 0);
        assert_eq!(machine.step()?, 2);
        assert_eq!(machine.registers().a, 0x43);
        let next = machine.run_until(MAX_SLICE, &points, || false)?;
        assert_eq!(next.reason, DebugStopReason::Breakpoint);
        assert_eq!(next.registers.pc, 0x6002);
        assert_eq!(next.cycles, 3);
        let before = points.clone();
        assert!(points.replace(&[0x6003; MAX_BREAKPOINTS + 1]).is_err());
        assert_eq!(points, before);
        points.replace(&[])?;
        assert_eq!(
            machine.run_until(1, &points, || false)?.reason,
            DebugStopReason::Budget
        );
        Ok(())
    }

    #[test]
    fn bounded_execution_cancels_and_restarts_at_the_exact_guest_state() -> Result<(), String> {
        let mut machine = authored_machine();
        let points = Breakpoints::default();
        let before = machine.registers();
        for cycles in [0, MAX_SLICE + 1] {
            assert!(machine.run_until(cycles, &points, || false).is_err());
        }
        assert_eq!(machine.registers(), before);
        let stop = machine.run_until(MAX_SLICE, &points, || true)?;
        assert_eq!((stop.reason, stop.cycles), (DebugStopReason::Cancelled, 0));
        assert_eq!(stop.registers, before);
        let mut checks = 0;
        let stop = machine.run_until(MAX_SLICE, &points, || {
            checks += 1;
            checks == 3
        })?;
        assert_eq!(
            (stop.reason, stop.cycles, stop.registers.pc),
            (DebugStopReason::Cancelled, 4, 0x6003)
        );
        let resumed = machine.run_until(1, &points, || false)?;
        assert_eq!(
            (resumed.reason, resumed.cycles, resumed.registers.pc),
            (DebugStopReason::Budget, 3, 0x6002)
        );
        Ok(())
    }

    #[test]
    fn memory_inspection_is_bounded_and_never_acknowledges_device_interrupts() -> Result<(), String>
    {
        let mut machine = authored_machine();
        machine.board.write(0xff01, 5);
        machine.board.tick(1);
        assert!(machine.board.interrupts().irq);
        let clocks = (machine.board.cycles, machine.board.master_clocks);
        let registers = machine.registers();
        for (address, length) in [
            (0, 0),
            (0, MAX_DEBUG_MEMORY_BYTES + 1),
            (0xffff, 2),
            (0xff00, 1),
            (0xfeff, 2),
            (0xffdf, 2),
            (0, usize::MAX),
        ] {
            assert!(machine.read_memory(address, length).is_err());
        }
        assert_eq!(
            machine.read_memory(0, MAX_DEBUG_MEMORY_BYTES)?.len(),
            MAX_DEBUG_MEMORY_BYTES
        );
        assert_eq!(machine.read_memory(0xfffe, 2)?, crate::MONITOR[16382..]);
        assert_eq!((machine.board.cycles, machine.board.master_clocks), clocks);
        assert_eq!(machine.registers(), registers);
        assert!(machine.board.interrupts().irq);
        Ok(())
    }

    #[test]
    fn steps_interrupts_wait_and_failures_through_the_same_cpu_then_reset_recovers()
    -> Result<(), String> {
        let mut machine = authored_machine();
        machine.halt(true);
        let before = machine.registers();
        assert_eq!(machine.step()?, 1);
        assert_eq!(machine.registers(), before);
        machine.halt(false);
        machine.board.write(0xff01, 5);
        machine.cpu.registers.cc &= !eightbit_m6809_core::I;
        machine.board.tick(1);
        assert!(machine.board.interrupts().irq);
        assert_eq!(machine.step()?, 19);
        assert_eq!(
            machine.registers().pc,
            u16::from_be_bytes([machine.peek(0xfff8), machine.peek(0xfff9)])
        );
        assert_eq!(machine.registers().s, before.s - 12);
        let mut failed = authored_machine();
        failed.board.ram[0x6000] = 0x01; // Explicitly unsupported encoding.
        let error = failed
            .step()
            .err()
            .ok_or("unsupported instruction unexpectedly succeeded")?;
        let clocks = failed.board.cycles;
        assert_eq!(failed.step(), Err(error.clone()));
        assert_eq!(failed.advance(1), Err(error.clone()));
        assert_eq!(
            failed.run_until(1, &Breakpoints::default(), || false),
            Err(error)
        );
        assert_eq!(failed.board.cycles, clocks);
        failed.reset();
        assert!(failed.step().is_ok());
        Ok(())
    }

    #[test]
    fn guest_basic_values_are_inspected_at_an_actual_authored_machine_breakpoint()
    -> Result<(), String> {
        let mut machine = Machine::with_firmware(Firmware::BasicDevelopment);
        for _ in 0..10 {
            machine.advance(MAX_SLICE)?;
        }
        // Only this authored sentinel is seeded; the guest creates all BASIC state.
        machine.board.ram[0x6000..0x6002].copy_from_slice(&[0x20, 0xfe]);
        machine.type_text("10 COUNT=42:MESSAGE$=\"HELLO\":EXEC 24576")?;
        for _ in 0..100 {
            machine.advance(MAX_SLICE)?;
        }
        machine.type_text("RUN")?;
        let mut points = Breakpoints::default();
        points.replace(&[0x6000])?;
        let mut hit = false;
        for _ in 0..100 {
            let stop = machine.run_until(MAX_SLICE, &points, || false)?;
            if stop.reason == DebugStopReason::Breakpoint {
                hit = true;
                break;
            }
        }
        assert!(hit, "guest did not reach authored EXEC sentinel");
        let word = |address| u16::from_be_bytes([machine.peek(address), machine.peek(address + 1)]);
        assert_eq!(word(0x68), 10);
        let before = (machine.registers(), machine.board.cycles);
        let values = inspect_scalars(
            ScalarWorkspace {
                ram_start: 0x1e01,
                ram_end: 0x7fff,
                variables_start: word(0x1b),
                variables_end: word(0x1d),
            },
            &BTreeMap::from([
                ("CO".into(), "COUNT".into()),
                ("ME$".into(), "MESSAGE$".into()),
            ]),
            |address, length| machine.read_memory(address, length),
        )
        .map_err(|error| format!("{error:?}"))?;
        assert_eq!(values.len(), 2);
        assert_eq!(values[0].name, "COUNT");
        assert_eq!(values[0].numeric_value, Some(42.0));
        assert_eq!(values[1].name, "MESSAGE$");
        assert_eq!(values[1].string_value.as_deref(), Some("HELLO"));
        assert_eq!((machine.registers(), machine.board.cycles), before);
        Ok(())
    }
    #[test]
    fn original_guest_statement_stops_correlate_loaded_tokens_and_authored_callback()
    -> Result<(), String> {
        use eightbit_dragon_basic_core::breakpoints::{StatementMap, read_statement_cursor};
        use eightbit_dragon_basic_core::{Dialect, analyse, tokenise};
        let source = "10 COUNT=40\n20 COUNT=COUNT+1:MESSAGE$=\"HELLO\"\n30 COUNT=COUNT+1:EXEC 24576\n40 END\n";
        let dialect = Dialect::dragon32().map_err(|error| error.to_string())?;
        let programme = tokenise(&analyse(source, &dialect).program, &dialect)
            .map_err(|error| error.to_string())?;
        let map = StatementMap::new(programme.clone())?;
        let bytes = eightbit_dragon_media::create_tokenised_basic("BREAK", &programme.bytes)
            .map_err(|error| error.to_string())?;
        let tape = crate::cassette::Cassette::read(bytes)?;
        let mut machine = Machine::with_firmware(Firmware::BasicDevelopment);
        for _ in 0..8 {
            machine.advance(MAX_SLICE)?;
        }
        // Authored LDA #$5A / STA $6300 / RTS. The guest calls and returns normally.
        machine.board.ram[0x6000..0x6006].copy_from_slice(&[0x86, 0x5a, 0xb7, 0x63, 0, 0x39]);
        machine.mount_cassette_signal(tape.signal());
        machine.type_text("CLOAD\"BREAK\"")?;
        for _ in 0..256 {
            machine.advance(MAX_SLICE)?;
        }
        assert!(!machine.snapshot().text.contains("ERROR"));
        // This is the original source's fixed STATEMENT entry, not an OEM profile.
        let hook = machine.read_memory(0x84d6, 3)?;
        let mut points = Breakpoints::default();
        points.replace(&[0x84d6, 0x6000])?;
        machine.type_text("RUN")?;
        let mut callback_seen = false;
        for (line, ordinal, expected_value) in
            [(20, 1, 41.0), (30, 0, 41.0), (30, 1, 42.0), (40, 0, 42.0)]
        {
            let mut matched = None;
            for _ in 0..200 {
                let stop = machine.run_until(MAX_SLICE, &points, || false)?;
                if stop.reason != DebugStopReason::Breakpoint {
                    continue;
                }
                if stop.registers.pc == 0x6000 {
                    callback_seen = true;
                    assert_eq!(machine.peek(0x6300), 0);
                } else {
                    let cursor = read_statement_cursor(|address, length| {
                        machine.read_memory(address, length)
                    })
                    .map_err(|error| format!("{error:?}"))?;
                    let location = map
                        .locate(cursor, |address, length| {
                            machine.read_memory(address, length)
                        })
                        .map_err(|error| format!("{error:?}"))?;
                    if let Some(location) = location
                        && (location.basic_line, location.ordinal) == (line, ordinal)
                    {
                        matched = Some((cursor, location));
                        break;
                    }
                }
                machine.step()?;
            }
            let (cursor, location) =
                matched.ok_or("bounded original statement breakpoint was not reached")?;
            let word =
                |address| u16::from_be_bytes([machine.peek(address), machine.peek(address + 1)]);
            let values = inspect_scalars(
                ScalarWorkspace {
                    ram_start: 0x1e01,
                    ram_end: 0x7fff,
                    variables_start: word(0x1b),
                    variables_end: word(0x1d),
                },
                &BTreeMap::new(),
                |address, length| machine.read_memory(address, length),
            )
            .map_err(|error| format!("{error:?}"))?;
            assert_eq!(
                values
                    .iter()
                    .find(|value| value.runtime_name == "CO")
                    .and_then(|value| value.numeric_value),
                Some(expected_value)
            );
            assert_eq!(machine.read_memory(0x84d6, 3)?, hook);
            if line == 30 {
                let text = source[location.source_range.start..location.source_range.end].trim();
                assert_eq!(
                    text,
                    if ordinal == 0 {
                        "COUNT=COUNT+1"
                    } else {
                        "EXEC 24576"
                    }
                );
            }
            if line == 40 {
                assert!(callback_seen);
                assert_eq!(machine.peek(0x6300), 0x5a);
                let last = usize::from(programme.program_start) + programme.bytes.len() - 1;
                machine.board.ram[last] = 1;
                assert!(
                    map.locate(cursor, |address, length| machine
                        .read_memory(address, length))
                        .is_err()
                );
                machine.board.ram[last] = 0;
                assert_eq!(
                    map.locate(cursor, |address, length| machine
                        .read_memory(address, length))
                        .map_err(|error| format!("{error:?}"))?,
                    Some(location)
                );
                let before = machine.registers();
                let cancelled = machine.run_until(MAX_SLICE, &points, || true)?;
                assert_eq!(
                    (cancelled.reason, cancelled.cycles, cancelled.registers),
                    (DebugStopReason::Cancelled, 0, before)
                );
            }
            machine.step()?;
        }
        Ok(())
    }
}
