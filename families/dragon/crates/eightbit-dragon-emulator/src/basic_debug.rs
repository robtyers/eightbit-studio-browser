//! Portable line-breakpoint/Continue ownership for the original development ROM.
//!
//! This is a synchronous library controller, not a runtime profile or host route.
//! It executes the real guest and consumes the existing linked-token source map;
//! BASIC statements and scalar values are never evaluated on the host.

use crate::debug::Registers;
use crate::{MAX_SLICE, MonitorCommand, MonitorRequest, MonitorSession, SessionSnapshot};
use eightbit_dragon_basic_core::MAX_SOURCE_BYTES;
use eightbit_dragon_basic_core::breakpoints::{
    StatementLocation, StatementMap, read_statement_cursor,
};
use eightbit_dragon_basic_core::inspection::{
    BasicVariable, InspectionError, MAX_SCALAR_RECORDS, ScalarWorkspace, inspect_scalars,
};
use std::collections::BTreeMap;

#[cfg(test)]
const STATEMENT_HOOK: u16 = 0x84d6;
const LINE_INPUT_HOOK: u16 = 0x0182;
const MAX_SEARCH_CYCLES: u64 = MAX_SLICE as u64 * 512;
const MAX_HOOK_VISITS: u32 = 10_000;
// Even a malformed guest table repeating one name cannot amplify aliases beyond
// the existing source limit when cloning at most 128 inspected scalar records.
const MAX_SOURCE_ALIAS_BYTES: usize = MAX_SOURCE_BYTES / MAX_SCALAR_RECORDS;

/// Session, immutable build and controller revision owning a request or result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BasicDebugRevision {
    pub session_id: u32,
    pub build_id: String,
    pub revision: u64,
}

/// Inspection belongs only to this stopped revision and this exact source map.
#[derive(Clone, Debug, PartialEq)]
pub struct BasicBreakpoint {
    pub location: StatementLocation,
    pub registers: Registers,
    pub variables: Vec<BasicVariable>,
}

/// There are no retained variables/registers outside `Stopped`.
#[derive(Clone, Debug, PartialEq)]
pub enum BasicDebugState {
    Ready,
    /// Guest returned to the direct-mode line-input hook; this is not a success claim.
    Prompt,
    Running {
        cycles: u64,
        hook_visits: u32,
    },
    Stopped(BasicBreakpoint),
    Cancelled,
    Failed(String),
    Terminated,
}

/// A coherent controller/media frame; state reads do not drain audio or execute.
#[derive(Debug)]
pub struct BasicDebugFrame {
    pub revision: BasicDebugRevision,
    pub state: BasicDebugState,
    pub session: SessionSnapshot,
}

/// Sole owner of one existing media/input session and one immutable source map.
///
/// Callers explicitly queue guest input/load/run using `command`, then begin a
/// search and poll bounded slices. This API neither loads a programme nor types
/// RUN implicitly. Source/build changes require a new controller. Host adapters
/// must retain and check the returned revision before publishing an async result.
pub struct BasicDebugController {
    session: MonitorSession,
    map: StatementMap,
    line: u16,
    names: BTreeMap<String, String>,
    revision: BasicDebugRevision,
    state: BasicDebugState,
    // An interrupt may occur before the hook JMP executes. Retain its original
    // stack until that actual JMP executes, including across slices/cancellation.
    resume_stack: Option<u16>,
    programme_seen: bool,
}

impl BasicDebugController {
    pub fn new(
        session: MonitorSession,
        map: StatementMap,
        line: u16,
        build_id: String,
        names: BTreeMap<String, String>,
    ) -> Result<Self, String> {
        session.debug_machine()?;
        if !map.contains_line(line)
            || build_id.len() != 64
            || !build_id
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || names
                .values()
                .any(|name| name.len() > MAX_SOURCE_ALIAS_BYTES)
            || names.len() > MAX_SOURCE_BYTES / 2
            || names
                .iter()
                .try_fold(0usize, |total, (key, name)| {
                    total.checked_add(key.len())?.checked_add(name.len())
                })
                .is_none_or(|bytes| bytes > MAX_SOURCE_BYTES)
        {
            return Err("DME1302: invalid BASIC breakpoint, build identity or source names".into());
        }
        Ok(Self {
            revision: BasicDebugRevision {
                session_id: session.id(),
                build_id,
                revision: 0,
            },
            session,
            map,
            line,
            names,
            state: BasicDebugState::Ready,
            resume_stack: None,
            programme_seen: false,
        })
    }

    // Normal launch preparation owns execution before inspection is published.
    pub(crate) fn preparation_machine_mut(&mut self) -> &mut crate::Machine {
        self.session.debug_machine_mut()
    }
    pub(crate) fn preparation_frame(&mut self) -> SessionSnapshot {
        self.session.debug_frame()
    }

    #[must_use]
    pub fn snapshot(&self) -> BasicDebugFrame {
        self.frame(self.session.snapshot())
    }

    fn frame(&self, session: SessionSnapshot) -> BasicDebugFrame {
        BasicDebugFrame {
            revision: self.revision.clone(),
            state: self.state.clone(),
            session,
        }
    }

    fn check_revision(&self, expected: &BasicDebugRevision) -> Result<(), String> {
        if expected != &self.revision {
            return Err("DME1303: BASIC debug request belongs to a stale revision".into());
        }
        if matches!(self.state, BasicDebugState::Terminated) {
            return Err("DME1303: BASIC debug session has stopped".into());
        }
        if self.revision.revision == u64::MAX {
            return Err("DME1303: BASIC debug revision limit reached; start a new session".into());
        }
        Ok(())
    }

    fn transition(&mut self, state: BasicDebugState) {
        self.revision.revision += 1;
        self.state = state;
    }

    /// Clear old inspection and create one owned continuation. No CPU work occurs.
    pub fn continue_execution(
        &mut self,
        expected: &BasicDebugRevision,
    ) -> Result<BasicDebugRevision, String> {
        self.check_revision(expected)?;
        if matches!(
            self.state,
            BasicDebugState::Running { .. } | BasicDebugState::Failed(_)
        ) {
            return Err("DME1302: BASIC debug search is active or requires reset".into());
        }
        if matches!(self.state, BasicDebugState::Ready | BasicDebugState::Prompt) {
            self.programme_seen = false;
        }
        self.transition(BasicDebugState::Running {
            cycles: 0,
            hook_visits: 0,
        });
        Ok(self.revision.clone())
    }

    /// Cancel only this owned search; real guest state remains resumable.
    pub fn cancel(&mut self, expected: &BasicDebugRevision) -> Result<BasicDebugFrame, String> {
        self.check_revision(expected)?;
        if !matches!(self.state, BasicDebugState::Running { .. }) {
            return Err("DME1302: no BASIC debug search is active".into());
        }
        self.transition(BasicDebugState::Cancelled);
        let frame = self.session.debug_frame();
        Ok(self.frame(frame))
    }

    /// Forward the existing session contract. Physical input retains a search or
    /// pause and its budget, while advancing ownership. Other successful mutations
    /// revoke continuation/inspection; State/Export remain inert. Reset recovers a
    /// failed search. Invalid requests do not revoke the debug owner.
    pub fn command(
        &mut self,
        expected: &BasicDebugRevision,
        command: MonitorCommand,
    ) -> Result<BasicDebugFrame, String> {
        self.check_revision(expected)?;
        let inert = matches!(
            &command,
            MonitorCommand::State | MonitorCommand::ExportRecording
        );
        let input = matches!(
            &command,
            MonitorCommand::Key { .. } | MonitorCommand::Type { .. } | MonitorCommand::Release
        );
        let moved = matches!(
            &command,
            MonitorCommand::Advance { .. } | MonitorCommand::Reset | MonitorCommand::Stop
        );
        let reset = matches!(&command, MonitorCommand::Reset);
        let idle_tick = matches!(&command, MonitorCommand::Advance { .. });
        let stop = matches!(&command, MonitorCommand::Stop);
        let frame = self.session.command(MonitorRequest {
            schema_version: 1,
            session_id: self.session.id(),
            command,
        })?;
        if !inert {
            if moved {
                self.resume_stack = None;
            }
            let state = if input {
                self.state.clone()
            } else if stop {
                BasicDebugState::Terminated
            } else if (idle_tick && self.state == BasicDebugState::Prompt)
                || (!reset && matches!(self.state, BasicDebugState::Failed(_)))
            {
                self.state.clone()
            } else {
                BasicDebugState::Ready
            };
            if !input {
                self.programme_seen = false;
            }
            self.transition(state);
        }
        Ok(self.frame(frame))
    }

    /// Run at most one CPU slice, then drain existing audio/recording queues once.
    /// Cancellation is checked before every CPU step and every inspection read.
    /// A search also has a cumulative cycle/hook budget, independent of polling.
    /// Guest failure, exhausted budgets and invalid source/variables produce a
    /// Failed frame with no inspection; invalid/stale host requests return Err
    /// without mutation. A terminal frame revokes its previous operation token.
    pub fn poll(
        &mut self,
        expected: &BasicDebugRevision,
        cycles: u32,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<BasicDebugFrame, String> {
        self.check_revision(expected)?;
        if cycles == 0
            || cycles > MAX_SLICE
            || !matches!(self.state, BasicDebugState::Running { .. })
        {
            return Err("DME1302: use a bounded slice for an active BASIC debug search".into());
        }
        match self.run_slice(cycles, &mut cancelled) {
            Ok(Some(state)) => self.transition(state),
            Ok(None) => {}
            Err(ReadFailure::Cancelled) => self.transition(BasicDebugState::Cancelled),
            Err(ReadFailure::Invalid(message)) => self.transition(BasicDebugState::Failed(message)),
        }
        let frame = self.session.debug_frame();
        Ok(self.frame(frame))
    }

    fn run_slice(
        &mut self,
        budget: u32,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<Option<BasicDebugState>, ReadFailure> {
        let mut elapsed = 0;
        loop {
            if cancelled() {
                return Err(ReadFailure::Cancelled);
            }
            let BasicDebugState::Running {
                cycles,
                hook_visits,
            } = &mut self.state
            else {
                return Err(invalid("BASIC debug search is not active"));
            };
            if *cycles >= MAX_SEARCH_CYCLES || *hook_visits >= MAX_HOOK_VISITS {
                return Err(invalid("BASIC breakpoint search limit reached"));
            }
            if elapsed >= budget {
                return Ok(None);
            }
            let machine = self.session.debug_machine().map_err(ReadFailure::Invalid)?;
            let before = machine.registers();
            // Published current-line $FFFF at the line-input hook means the guest
            // has returned to direct input. Observe it only after programme work;
            // boot/CLOAD preparation and programme INPUT are not completion.
            if self.programme_seen
                && before.pc == LINE_INPUT_HOOK
                && machine.peek(0x68) == 0xff
                && machine.peek(0x69) == 0xff
            {
                self.programme_seen = false;
                self.resume_stack = None;
                return Ok(Some(BasicDebugState::Prompt));
            }
            let statement_hook = machine.board.profile.statement_hook();
            let base = if machine.board.profile == crate::Firmware::Dragon64Native {
                0xc000
            } else {
                0x8000
            };
            let hook = usize::from(statement_hook - base);
            let firmware = machine.board.firmware;
            if before.pc == statement_hook {
                if machine
                    .read_memory(statement_hook, 3)
                    .map_err(ReadFailure::Invalid)?
                    != firmware[hook..hook + 3]
                    || firmware[hook] != 0x7e
                {
                    return Err(invalid("original BASIC statement entry is not mapped"));
                }
                if machine.peek(0x68) != 0xff || machine.peek(0x69) != 0xff {
                    self.programme_seen = true;
                }
                if self.resume_stack.is_none() {
                    *hook_visits += 1;
                    if let Some(stop) = self.inspect_stop(cancelled)? {
                        self.resume_stack = Some(before.s);
                        return Ok(Some(BasicDebugState::Stopped(stop)));
                    }
                    self.resume_stack = Some(before.s);
                }
            }
            // Cancellation may arrive during source inspection or a host reader.
            if cancelled() {
                return Err(ReadFailure::Cancelled);
            }
            let machine = self.session.debug_machine_mut();
            let consumed = machine.step().map_err(ReadFailure::Invalid)?;
            let after = machine.registers();
            if self.resume_stack == Some(before.s)
                && before.pc == statement_hook
                && after.s == before.s
                && after.pc == u16::from_be_bytes([firmware[hook + 1], firmware[hook + 2]])
            {
                self.resume_stack = None;
            }
            elapsed += consumed;
            if let BasicDebugState::Running { cycles, .. } = &mut self.state {
                *cycles += u64::from(consumed);
            }
        }
    }

    fn inspect_stop(
        &self,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<Option<BasicBreakpoint>, ReadFailure> {
        let machine = self.session.debug_machine().map_err(ReadFailure::Invalid)?;
        let mut read = |address, length| {
            if cancelled() {
                return Err(ReadFailure::Cancelled);
            }
            machine
                .read_memory(address, length)
                .map_err(ReadFailure::Invalid)
        };
        let cursor = read_statement_cursor(&mut read).map_err(inspection_error)?;
        if cursor.basic_line != self.line {
            return Ok(None);
        }
        let Some(location) = self
            .map
            .locate(cursor, &mut read)
            .map_err(inspection_error)?
        else {
            return Ok(None);
        };
        let bounds = read(0x1b, 4)?;
        let variables = inspect_scalars(
            ScalarWorkspace {
                ram_start: 0x1e01,
                ram_end: machine.board.profile.ram_end(),
                variables_start: u16::from_be_bytes([bounds[0], bounds[1]]),
                variables_end: u16::from_be_bytes([bounds[2], bounds[3]]),
            },
            &self.names,
            &mut read,
        )
        .map_err(inspection_error)?;
        if cancelled() {
            return Err(ReadFailure::Cancelled);
        }
        Ok(Some(BasicBreakpoint {
            location,
            registers: machine.registers(),
            variables,
        }))
    }
}

#[derive(Debug)]
enum ReadFailure {
    Cancelled,
    Invalid(String),
}
fn invalid(message: &str) -> ReadFailure {
    ReadFailure::Invalid(format!("DME1304: {message}"))
}
fn inspection_error(error: InspectionError<ReadFailure>) -> ReadFailure {
    match error {
        InspectionError::Invalid(message) => invalid(message),
        InspectionError::Memory(error) => error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Bus, Firmware};
    use eightbit_dragon_basic_core::{Dialect, TokenisedProgram, analyse, tokenise};

    fn programme(source: &str) -> Result<TokenisedProgram, String> {
        let dialect = Dialect::dragon32().map_err(|error| error.to_string())?;
        tokenise(&analyse(source, &dialect).program, &dialect).map_err(|error| error.to_string())
    }
    fn command(session: &mut MonitorSession, command: MonitorCommand) -> Result<(), String> {
        session.command(MonitorRequest {
            schema_version: 1,
            session_id: session.id(),
            command,
        })?;
        Ok(())
    }
    fn configured(
        session: MonitorSession,
        tokens: TokenisedProgram,
        line: u16,
    ) -> Result<BasicDebugController, String> {
        BasicDebugController::new(
            session,
            StatementMap::new(tokens)?,
            line,
            "a".repeat(64),
            BTreeMap::from([
                ("CO".into(), "COUNT".into()),
                ("ME$".into(), "MESSAGE$".into()),
            ]),
        )
    }
    // Small authored RAM fixture isolates ownership/error paths. The real guest
    // journey below instead loads the host build through CLOAD and cassette pins.
    fn at_hook() -> Result<BasicDebugController, String> {
        let tokens = programme("10 PRINT\"READY\"\n20 END\n")?;
        let mut session = MonitorSession::with_firmware(9, Firmware::BasicDevelopment)?;
        let machine = session.debug_machine_mut();
        for _ in 0..8 {
            machine.advance(MAX_SLICE)?;
        }
        let start = usize::from(tokens.program_start);
        let end = u16::try_from(start + tokens.bytes.len()).map_err(|error| error.to_string())?;
        machine.board.ram[start..start + tokens.bytes.len()].copy_from_slice(&tokens.bytes);
        machine.board.ram[0x19..0x1b].copy_from_slice(&tokens.program_start.to_be_bytes());
        machine.board.ram[0x1b..0x1d].copy_from_slice(&end.to_be_bytes());
        machine.board.ram[0x1d..0x1f].copy_from_slice(&(end + 14).to_be_bytes());
        // Two authored public scalar records, including an out-of-line string.
        let table = usize::from(end);
        machine.board.ram[table..table + 14].copy_from_slice(&[
            b'C',
            b'O',
            0x86,
            0x28,
            0,
            0,
            0,
            b'M',
            b'E' | 0x80,
            5,
            0,
            0x62,
            0,
            0,
        ]);
        machine.board.ram[0x6200..0x6205].copy_from_slice(b"HELLO");
        machine.board.ram[0x68..0x6a].copy_from_slice(&10u16.to_be_bytes());
        machine.board.ram[0xa6..0xa8].copy_from_slice(&(tokens.program_start + 4).to_be_bytes());
        machine.cpu.registers.pc = STATEMENT_HOOK;
        machine.cpu.registers.cc |= eightbit_m6809_core::I | eightbit_m6809_core::F;
        machine.take_audio();
        configured(session, tokens, 10)
    }
    fn stop(
        controller: &mut BasicDebugController,
        token: &BasicDebugRevision,
    ) -> Result<BasicDebugFrame, String> {
        for _ in 0..513 {
            let frame = controller.poll(token, MAX_SLICE, || false)?;
            if !matches!(frame.state, BasicDebugState::Running { .. }) {
                return Ok(frame);
            }
        }
        Err("controller did not return within its bounded search".into())
    }
    fn breakpoint(frame: &BasicDebugFrame) -> Result<&BasicBreakpoint, String> {
        if let BasicDebugState::Stopped(stop) = &frame.state {
            Ok(stop)
        } else {
            Err(format!("expected breakpoint, got {:?}", frame.state))
        }
    }
    fn load(source: &str, line: u16) -> Result<BasicDebugController, String> {
        let tokens = programme(source)?;
        let bytes = eightbit_dragon_media::create_tokenised_basic("CONTROL", &tokens.bytes)
            .map_err(|error| error.to_string())?;
        let mut session = MonitorSession::with_firmware(9, Firmware::BasicDevelopment)?;
        for _ in 0..8 {
            command(&mut session, MonitorCommand::Advance { cycles: MAX_SLICE })?;
        }
        command(&mut session, MonitorCommand::MountCassette { bytes })?;
        command(
            &mut session,
            MonitorCommand::Type {
                text: "CLOAD\"CONTROL\"".into(),
            },
        )?;
        for _ in 0..256 {
            command(&mut session, MonitorCommand::Advance { cycles: MAX_SLICE })?;
        }
        assert!(!session.snapshot().machine.text.contains("ERROR"));
        configured(session, tokens, line)
    }

    #[test]
    fn actual_guest_continue_tracks_same_line_subroutine_and_pending_irq() -> Result<(), String> {
        let source = "10 COUNT=0:MESSAGE$=\"HELLO\"\n20 COUNT=COUNT+1:GOSUB100:COUNT=COUNT+10\n30 IF COUNT<20 THEN20\n40 END\n100 COUNT=COUNT+2:EXEC24576:RETURN\n";
        let mut controller = load(source, 20)?;
        // Authored INC $6300 / RTS, entered and returned by guest EXEC.
        controller.session.debug_machine_mut().board.ram[0x6000..0x6004]
            .copy_from_slice(&[0x7c, 0x63, 0x00, 0x39]);
        let hook = controller
            .session
            .debug_machine()?
            .read_memory(STATEMENT_HOOK, 3)?;
        let revision = controller.snapshot().revision;
        let frame = controller.command(&revision, MonitorCommand::Type { text: "RUN".into() })?;
        let mut token = controller.continue_execution(&frame.revision)?;
        for (index, (ordinal, count, marker)) in
            [(0, 0.0, 0), (1, 1.0, 0), (2, 3.0, 1), (0, 13.0, 1)]
                .into_iter()
                .enumerate()
        {
            let frame = stop(&mut controller, &token)?;
            let stopped = breakpoint(&frame)?;
            assert_eq!(
                (stopped.location.basic_line, stopped.location.ordinal),
                (20, ordinal)
            );
            assert_eq!(stopped.registers.pc, STATEMENT_HOOK);
            assert_eq!(
                stopped
                    .variables
                    .iter()
                    .find(|value| value.name == "COUNT")
                    .and_then(|value| value.numeric_value),
                Some(count)
            );
            assert_eq!(
                stopped
                    .variables
                    .iter()
                    .find(|value| value.name == "MESSAGE$")
                    .and_then(|value| value.string_value.as_deref()),
                Some("HELLO")
            );
            assert_eq!(controller.session.debug_machine()?.peek(0x6300), marker);
            assert_eq!(
                controller
                    .session
                    .debug_machine()?
                    .read_memory(STATEMENT_HOOK, 3)?,
                hook
            );
            assert_eq!(controller.snapshot().state, frame.state);
            assert!(controller.snapshot().session.audio.is_empty());
            let origin = stopped.registers;
            token = controller.continue_execution(&frame.revision)?;
            assert!(matches!(
                controller.snapshot().state,
                BasicDebugState::Running { cycles: 0, .. }
            ));
            if index == 0 {
                let machine = controller.session.debug_machine_mut();
                machine.cpu.registers.cc &= !eightbit_m6809_core::I;
                machine.board.pia[0].control[1] |= 0x81;
                assert!(machine.board.interrupts().irq);
                let interrupted = controller.poll(&token, 1, || false)?;
                assert!(matches!(
                    interrupted.state,
                    BasicDebugState::Running { cycles: 19, .. }
                ));
                assert_ne!(controller.session.debug_machine()?.registers().s, origin.s);
                assert_eq!(controller.resume_stack, Some(origin.s));
                let cancelled = controller.cancel(&token)?;
                assert_eq!(cancelled.state, BasicDebugState::Cancelled);
                assert!(controller.poll(&token, 1, || false).is_err());
                token = controller.continue_execution(&cancelled.revision)?;
            }
        }
        Ok(())
    }

    #[test]
    fn stale_requests_and_invalid_commands_are_atomic_and_mutations_revoke_ownership()
    -> Result<(), String> {
        let mut controller = at_hook()?;
        let initial = controller.snapshot();
        for revision in [
            BasicDebugRevision {
                session_id: 8,
                ..initial.revision.clone()
            },
            BasicDebugRevision {
                build_id: "b".repeat(64),
                ..initial.revision.clone()
            },
            BasicDebugRevision {
                revision: 1,
                ..initial.revision.clone()
            },
        ] {
            assert!(controller.continue_execution(&revision).is_err());
            assert!(
                controller
                    .command(&revision, MonitorCommand::Reset)
                    .is_err()
            );
        }
        let token = controller.continue_execution(&initial.revision)?;
        assert!(controller.continue_execution(&token).is_err());
        for cycles in [0, MAX_SLICE + 1] {
            assert!(controller.poll(&token, cycles, || false).is_err());
        }
        for command in [
            MonitorCommand::Advance { cycles: 0 },
            MonitorCommand::MountCassette { bytes: vec![0] },
            MonitorCommand::Key {
                code: "invalid".repeat(6),
                pressed: true,
            },
        ] {
            assert!(controller.command(&token, command).is_err());
            assert_eq!(controller.snapshot().revision, token);
            assert_eq!(
                controller.snapshot().session.machine,
                initial.session.machine
            );
        }
        let inert = controller.command(&token, MonitorCommand::State)?;
        assert_eq!(inert.revision, token);
        let frame = controller.poll(&token, 1, || false)?;
        breakpoint(&frame)?;
        assert!(controller.poll(&token, 1, || false).is_err());
        assert!(controller.continue_execution(&token).is_err());
        let running = controller.continue_execution(&frame.revision)?;
        let reset = controller.command(&running, MonitorCommand::Reset)?;
        assert_eq!(reset.state, BasicDebugState::Ready);
        assert!(controller.poll(&running, 1, || false).is_err());
        assert_eq!(controller.resume_stack, None);
        let active = controller.continue_execution(&reset.revision)?;
        let stopped = controller.command(&active, MonitorCommand::Stop)?;
        assert_eq!(stopped.state, BasicDebugState::Terminated);
        assert_eq!(stopped.session.machine.state, "stopped");
        assert!(controller.continue_execution(&stopped.revision).is_err());
        assert!(controller.poll(&active, 1, || false).is_err());
        Ok(())
    }

    #[test]
    fn cancellation_during_source_inspection_never_publishes_partial_values_and_recovers()
    -> Result<(), String> {
        for cancel_at in [1, 3, 5, 6, 7, 8, 9] {
            let mut controller = at_hook()?;
            let before = controller.session.debug_machine()?.registers();
            let token = controller.continue_execution(&controller.snapshot().revision)?;
            let mut checks = 0;
            let cancelled = controller.poll(&token, MAX_SLICE, || {
                checks += 1;
                checks == cancel_at
            })?;
            assert_eq!(
                cancelled.state,
                BasicDebugState::Cancelled,
                "check {cancel_at}"
            );
            assert_eq!(controller.session.debug_machine()?.registers(), before);
            assert_eq!(controller.resume_stack, None);
            let resumed = controller.continue_execution(&cancelled.revision)?;
            let frame = controller.poll(&resumed, 1, || false)?;
            let stopped = breakpoint(&frame)?;
            assert_eq!(stopped.location.ordinal, 0);
            assert_eq!(stopped.variables.len(), 2);
            assert_eq!(stopped.variables[0].numeric_value, Some(42.0));
            assert_eq!(stopped.variables[1].string_value.as_deref(), Some("HELLO"));
        }
        Ok(())
    }

    #[test]
    fn source_variable_hook_and_cpu_failures_clear_inspection_and_require_reset()
    -> Result<(), String> {
        for case in 0..4 {
            let mut controller = at_hook()?;
            let machine = controller.session.debug_machine_mut();
            match case {
                0 => {
                    let end = u16::from_be_bytes([machine.peek(0x1b), machine.peek(0x1c)]);
                    machine.board.ram[usize::from(end) - 1] = 1;
                }
                1 => machine.board.ram[0x1d..0x1f].copy_from_slice(&0xffffu16.to_be_bytes()),
                2 => machine.board.sam.control |= 0x8000,
                _ => {
                    machine.cpu.registers.pc = 0x6000;
                    machine.board.ram[0x6000] = 0x01;
                }
            }
            let token = controller.continue_execution(&controller.snapshot().revision)?;
            let frame = controller.poll(&token, MAX_SLICE, || false)?;
            assert!(
                matches!(frame.state, BasicDebugState::Failed(_)),
                "case {case}: {:?}",
                frame.state
            );
            assert!(controller.continue_execution(&frame.revision).is_err());
            let reset = controller.command(&frame.revision, MonitorCommand::Reset)?;
            assert_eq!(reset.state, BasicDebugState::Ready);
            controller.continue_execution(&reset.revision)?;
        }
        Ok(())
    }

    #[test]
    fn search_budgets_and_revision_exhaustion_are_bounded() -> Result<(), String> {
        for hooks in [false, true] {
            let mut controller = at_hook()?;
            let before = controller.session.debug_machine()?.registers();
            let token = controller.continue_execution(&controller.snapshot().revision)?;
            controller.state = BasicDebugState::Running {
                cycles: if hooks { 0 } else { MAX_SEARCH_CYCLES },
                hook_visits: if hooks { MAX_HOOK_VISITS } else { 0 },
            };
            let frame = controller.poll(&token, MAX_SLICE, || false)?;
            assert_eq!(
                frame.state,
                BasicDebugState::Failed("DME1304: BASIC breakpoint search limit reached".into())
            );
            assert_eq!(controller.session.debug_machine()?.registers(), before);
        }
        let mut controller = at_hook()?;
        controller.revision.revision = u64::MAX;
        assert!(
            controller
                .continue_execution(&controller.snapshot().revision)
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn physical_input_preserves_search_budget_stop_state_and_revision_ownership()
    -> Result<(), String> {
        for input in [
            MonitorCommand::Key {
                code: "ShiftLeft".into(),
                pressed: true,
            },
            MonitorCommand::Type { text: "42".into() },
            MonitorCommand::Release,
        ] {
            let mut controller = at_hook()?;
            let token = controller.continue_execution(&controller.snapshot().revision)?;
            controller.state = BasicDebugState::Running {
                cycles: MAX_SEARCH_CYCLES,
                hook_visits: 17,
            };
            controller.programme_seen = true;
            controller.resume_stack = Some(0x7000);
            let before = controller.snapshot();
            let frame = controller.command(&token, input)?;
            assert_eq!(frame.state, before.state);
            assert!(controller.programme_seen);
            assert_eq!(controller.resume_stack, Some(0x7000));
            assert_ne!(frame.revision, token);
            assert!(controller.poll(&token, 1, || false).is_err());
            let failed = controller.poll(&frame.revision, 1, || false)?;
            assert_eq!(
                failed.state,
                BasicDebugState::Failed("DME1304: BASIC breakpoint search limit reached".into())
            );
            let released = controller.command(&failed.revision, MonitorCommand::Release)?;
            assert_eq!(released.state, failed.state);
            assert!(controller.continue_execution(&released.revision).is_err());
            let reset = controller.command(&released.revision, MonitorCommand::Reset)?;
            assert_eq!(reset.state, BasicDebugState::Ready);
            assert_eq!(controller.resume_stack, None);
            assert!(!controller.programme_seen);
        }
        let mut controller = at_hook()?;
        let token = controller.continue_execution(&controller.snapshot().revision)?;
        let frame = controller.poll(&token, 1, || false)?;
        breakpoint(&frame)?;
        let released = controller.command(&frame.revision, MonitorCommand::Release)?;
        assert_eq!(released.state, frame.state);
        assert!(controller.continue_execution(&frame.revision).is_err());
        let token = controller.continue_execution(&released.revision)?;
        assert!(controller.poll(&token, 1, || false).is_ok());
        Ok(())
    }

    #[test]
    fn an_unreachable_line_times_out_after_bounded_real_cpu_work_and_can_reset()
    -> Result<(), String> {
        let mut controller = at_hook()?;
        let machine = controller.session.debug_machine_mut();
        // Authored BRA to itself cannot reach the source breakpoint.
        machine.board.ram[0x6000..0x6002].copy_from_slice(&[0x20, 0xfe]);
        machine.cpu.registers.pc = 0x6000;
        let before = machine.board.cycles;
        let token = controller.continue_execution(&controller.snapshot().revision)?;
        let mut checks = 0;
        let cancelled = controller.poll(&token, MAX_SLICE, || {
            checks += 1;
            checks == 3
        })?;
        assert_eq!(cancelled.state, BasicDebugState::Cancelled);
        assert_eq!(controller.session.debug_machine()?.board.cycles - before, 3);
        let token = controller.continue_execution(&cancelled.revision)?;
        let before = controller.session.debug_machine()?.board.cycles;
        let frame = stop(&mut controller, &token)?;
        assert_eq!(
            frame.state,
            BasicDebugState::Failed("DME1304: BASIC breakpoint search limit reached".into())
        );
        let elapsed = controller.session.debug_machine()?.board.cycles - before;
        assert!((MAX_SEARCH_CYCLES..MAX_SEARCH_CYCLES + 3).contains(&elapsed));
        let reset = controller.command(&frame.revision, MonitorCommand::Reset)?;
        assert_eq!(reset.state, BasicDebugState::Ready);
        assert!(controller.poll(&token, MAX_SLICE, || false).is_err());
        Ok(())
    }

    #[test]
    fn controller_preserves_streaming_guest_cassette_and_audio_ownership() -> Result<(), String> {
        let mut controller = load("10 CSAVE\"CAPTURE\"\n20 END\n", 20)?;
        let frame = controller.command(
            &controller.snapshot().revision,
            MonitorCommand::StartRecording,
        )?;
        let frame =
            controller.command(&frame.revision, MonitorCommand::Type { text: "RUN".into() })?;
        let token = controller.continue_execution(&frame.revision)?;
        let mut samples = 0;
        let mut stopped = None;
        for _ in 0..513 {
            let frame = controller.poll(&token, MAX_SLICE, || false)?;
            samples += frame.session.audio.len();
            assert!(frame.session.audio.len() <= 4096);
            assert!(controller.snapshot().session.audio.is_empty());
            if !matches!(frame.state, BasicDebugState::Running { .. }) {
                stopped = Some(frame);
                break;
            }
        }
        let frame = stopped.ok_or("guest CSAVE did not finish")?;
        breakpoint(&frame)?;
        assert!(samples > 0);
        assert!(frame.session.recording.samples > 0);
        let finished = controller.command(&frame.revision, MonitorCommand::FinishRecording)?;
        assert_eq!(finished.state, BasicDebugState::Ready);
        assert_eq!(
            finished
                .session
                .recording
                .cassette
                .ok_or("missing guest recording")?
                .filename,
            "CAPTURE"
        );
        let exported = controller.command(&finished.revision, MonitorCommand::ExportRecording)?;
        assert_eq!(exported.revision, finished.revision);
        assert!(
            !exported
                .session
                .recording_export
                .ok_or("missing exported guest tape")?
                .bytes
                .is_empty()
        );
        Ok(())
    }

    #[test]
    fn actual_guest_fallthrough_and_error_return_preserve_the_authored_line_hook()
    -> Result<(), String> {
        for source in [
            "10 COUNT=41\n20 PRINT COUNT\n",
            "10 COUNT=41\n20 PRINT COUNT\n30 PRINT1/0\n",
        ] {
            let mut controller = load(source, 20)?;
            // Authored JMP to INC $6300 / RTS. Observing a public hook must not
            // patch, consume or replace the guest callback when input resumes.
            let machine = controller.session.debug_machine_mut();
            machine.board.ram[0x182..0x185].copy_from_slice(&[0x7e, 0x60, 0x00]);
            machine.board.ram[0x6000..0x6004].copy_from_slice(&[0x7c, 0x63, 0x00, 0x39]);
            machine.board.ram[0x6300] = 0;
            let frame = controller.command(
                &controller.snapshot().revision,
                MonitorCommand::Type { text: "RUN".into() },
            )?;
            let token = controller.continue_execution(&frame.revision)?;
            let frame = stop(&mut controller, &token)?;
            breakpoint(&frame)?;
            let token = controller.continue_execution(&frame.revision)?;
            let frame = stop(&mut controller, &token)?;
            assert_eq!(frame.state, BasicDebugState::Prompt);
            assert!(frame.session.machine.text.contains("DEV>"));
            assert!(controller.poll(&token, 1, || false).is_err());
            assert_eq!(controller.session.debug_machine()?.peek(0x6300), 0);
            let frame = controller.command(
                &frame.revision,
                MonitorCommand::Advance { cycles: MAX_SLICE },
            )?;
            assert_eq!(frame.state, BasicDebugState::Prompt);
            assert_eq!(controller.session.debug_machine()?.peek(0x6300), 1);
            assert_eq!(
                controller.session.debug_machine()?.read_memory(0x182, 3)?,
                [0x7e, 0x60, 0x00]
            );
            let reset = controller.command(&frame.revision, MonitorCommand::Reset)?;
            assert_eq!(reset.state, BasicDebugState::Ready);
            assert!(!controller.programme_seen);
        }
        Ok(())
    }

    #[test]
    fn boot_input_and_programme_input_are_not_reported_as_a_return_to_prompt() -> Result<(), String>
    {
        let mut boot = configured(
            MonitorSession::with_firmware(9, Firmware::BasicDevelopment)?,
            programme("10 END\n")?,
            10,
        )?;
        let token = boot.continue_execution(&boot.snapshot().revision)?;
        for _ in 0..16 {
            assert!(matches!(
                boot.poll(&token, MAX_SLICE, || false)?.state,
                BasicDebugState::Running { .. }
            ));
        }
        let mut controller = load("10 COUNT=41\n20 INPUT COUNT\n30 END\n", 20)?;
        let frame = controller.command(
            &controller.snapshot().revision,
            MonitorCommand::Type { text: "RUN".into() },
        )?;
        let token = controller.continue_execution(&frame.revision)?;
        let frame = stop(&mut controller, &token)?;
        breakpoint(&frame)?;
        let token = controller.continue_execution(&frame.revision)?;
        for _ in 0..16 {
            assert!(matches!(
                controller.poll(&token, MAX_SLICE, || false)?.state,
                BasicDebugState::Running { .. }
            ));
        }
        assert_eq!(
            controller.session.debug_machine()?.read_memory(0x68, 2)?,
            [0, 20]
        );
        let frame = controller.cancel(&token)?;
        assert_eq!(frame.state, BasicDebugState::Cancelled);
        assert!(controller.poll(&token, 1, || false).is_err());
        Ok(())
    }

    #[test]
    fn actual_guest_unanswered_input_keeps_its_limit_and_can_reset() -> Result<(), String> {
        let mut controller = load("10 COUNT=41\n20 INPUT COUNT\n30 END\n", 30)?;
        let frame = controller.command(
            &controller.snapshot().revision,
            MonitorCommand::Type { text: "RUN".into() },
        )?;
        let token = controller.continue_execution(&frame.revision)?;
        for _ in 0..16 {
            assert!(matches!(
                controller.poll(&token, MAX_SLICE, || false)?.state,
                BasicDebugState::Running { .. }
            ));
        }
        assert_eq!(
            controller.session.debug_machine()?.read_memory(0x68, 2)?,
            [0, 20]
        );
        let before = controller.snapshot().state;
        let frame = controller.command(&token, MonitorCommand::Release)?;
        assert_eq!(frame.state, before);
        assert!(controller.poll(&token, 1, || false).is_err());
        let failed = stop(&mut controller, &frame.revision)?;
        assert_eq!(
            failed.state,
            BasicDebugState::Failed("DME1304: BASIC breakpoint search limit reached".into())
        );
        assert_eq!(
            controller.session.debug_machine()?.read_memory(0x68, 2)?,
            [0, 20]
        );
        let reset = controller.command(&failed.revision, MonitorCommand::Reset)?;
        assert_eq!(reset.state, BasicDebugState::Ready);
        assert!(!controller.programme_seen);
        assert!(controller.poll(&frame.revision, 1, || false).is_err());
        Ok(())
    }

    #[test]
    fn configuration_rejects_other_firmware_stopped_sessions_unknown_lines_and_bad_builds()
    -> Result<(), String> {
        for (firmware, stopped, line, build_id) in [
            (Firmware::Monitor, false, 10, "a".repeat(64)),
            (Firmware::BasicDevelopment, true, 10, "a".repeat(64)),
            (Firmware::BasicDevelopment, false, 30, "a".repeat(64)),
            (Firmware::BasicDevelopment, false, 10, "A".repeat(64)),
            (Firmware::BasicDevelopment, false, 10, "a".repeat(65)),
        ] {
            let mut session = MonitorSession::with_firmware(9, firmware)?;
            if stopped {
                command(&mut session, MonitorCommand::Stop)?;
            }
            assert!(
                BasicDebugController::new(
                    session,
                    StatementMap::new(programme("10 END\n")?)?,
                    line,
                    build_id,
                    BTreeMap::new()
                )
                .is_err()
            );
        }
        let session = MonitorSession::with_firmware(9, Firmware::BasicDevelopment)?;
        assert!(
            BasicDebugController::new(
                session,
                StatementMap::new(programme("10 END\n")?)?,
                10,
                "a".repeat(64),
                BTreeMap::from([("A".into(), "A".repeat(MAX_SOURCE_ALIAS_BYTES + 1))])
            )
            .is_err()
        );
        Ok(())
    }
}
