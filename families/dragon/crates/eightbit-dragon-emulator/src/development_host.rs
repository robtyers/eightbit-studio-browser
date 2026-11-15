//! One execution owner for explicit development hosts, with no normal-runtime profile.
use crate::basic_debug::BasicDebugController;
use crate::basic_debug_protocol::{BasicDebugAdapter, DebugCommand, DebugRequest};
use crate::{
    MAX_MONITOR_REQUEST_BYTES, MAX_SLICE, MonitorCommand, MonitorRequest, MonitorSession,
    OriginalDevelopmentLaunch, RecordingExport, SessionSnapshot,
};
use eightbit_dragon_basic_core::{TokenisedProgram, breakpoints::StatementMap};
use std::collections::BTreeMap;

/// In-process handoff from a successful compiler invocation. This type has no
/// deserialiser: external JSON cannot supply a trusted source map.
pub struct DevelopmentDebugLaunch {
    launch: OriginalDevelopmentLaunch,
    map: StatementMap,
    line: u16,
    build_id: String,
    names: BTreeMap<String, String>,
}
impl DevelopmentDebugLaunch {
    pub fn new(
        launch: OriginalDevelopmentLaunch,
        program: TokenisedProgram,
        line: u16,
        names: BTreeMap<String, String>,
    ) -> Result<Self, String> {
        launch.validate()?;
        let cassette = launch
            .cassette
            .as_ref()
            .ok_or("DME1302: debug preparation requires a fresh BASIC cassette")?;
        let build_id = cassette
            .build_id
            .clone()
            .ok_or("DME1302: debug preparation requires a build identity")?;
        let recovered = eightbit_dragon_media::read_tokenised_basic(&cassette.bytes)
            .map_err(|_| "DME1302: debug preparation requires a tokenised BASIC cassette")?;
        if recovered.program != program.bytes {
            return Err("DME1302: debug programme does not match its fresh cassette".into());
        }
        let map = StatementMap::new(program).map_err(|_| "DME1302: invalid fresh statement map")?;
        if !map.contains_line(line) {
            return Err("DME1302: breakpoint line has no mapped statement".into());
        }
        Ok(Self {
            launch,
            map,
            line,
            build_id,
            names,
        })
    }
    #[must_use]
    pub fn launch(&self) -> &OriginalDevelopmentLaunch {
        &self.launch
    }
    /// Only boot/mount. Guest CLOAD/RUN and breakpoint search remain explicit.
    pub fn start(self, id: u32) -> Result<DevelopmentSession, String> {
        let session = self.launch.start(id)?;
        let controller =
            BasicDebugController::new(session, self.map, self.line, self.build_id, self.names)?;
        Ok(DevelopmentSession::Debug {
            adapter: Box::new(BasicDebugAdapter::new(controller)),
            preparing: true,
        })
    }
}

/// Native managers and browser workers use the same single execution owner.
/// Their existing frame scheduler is the only source of CPU slices. Debug
/// commands cannot add a second polling loop or bypass recording-save ownership.
pub enum DevelopmentSession {
    Original(Box<crate::original_runtime::OriginalRuntimeSession>),
    Monitor(Box<MonitorSession>),
    Machine(Box<crate::machine_debug::MachineDebugController>),
    Debug {
        adapter: Box<BasicDebugAdapter>,
        preparing: bool,
    },
}
impl DevelopmentSession {
    #[must_use]
    pub fn from_monitor(session: MonitorSession) -> Self {
        Self::Monitor(Box::new(session))
    }
    #[must_use]
    pub fn id(&self) -> u32 {
        match self {
            Self::Original(session) => session.id(),
            Self::Monitor(session) => session.id(),
            Self::Machine(controller) => controller.id(),
            Self::Debug { adapter, .. } => adapter.session_id(),
        }
    }
    #[must_use]
    pub fn snapshot(&self) -> SessionSnapshot {
        match self {
            Self::Original(session) => session.snapshot(),
            Self::Monitor(session) => session.snapshot(),
            Self::Machine(controller) => controller.snapshot().session,
            Self::Debug { adapter, .. } => adapter.snapshot().session,
        }
    }
    pub fn export_recording(&mut self) -> Result<RecordingExport, String> {
        match self {
            Self::Original(session) => session.export_recording(),
            Self::Monitor(session) => session.export_recording(),
            Self::Machine(controller) => controller.export_recording(),
            Self::Debug { adapter, .. } => adapter
                .command(
                    DebugRequest {
                        schema_version: 1,
                        revision: adapter.snapshot().revision,
                        command: DebugCommand::Session {
                            command: MonitorCommand::ExportRecording,
                        },
                    },
                    || false,
                )?
                .session
                .recording_export
                .ok_or_else(|| "DME1006: no completed cassette recording".into()),
        }
    }
    pub fn command_json(&mut self, json: &str) -> Result<SessionSnapshot, String> {
        if json.len() > MAX_MONITOR_REQUEST_BYTES {
            return Err("DME1002: monitor request is too large".into());
        }
        let request = serde_json::from_str(json).map_err(|_| "DME1002: invalid monitor request")?;
        self.command(request)
    }
    pub fn command(&mut self, request: MonitorRequest) -> Result<SessionSnapshot, String> {
        if request.schema_version != 1 || request.session_id != self.id() {
            return Err("DME1004: monitor request belongs to a stale session".into());
        }
        match self {
            Self::Original(session) => session.command(request),
            Self::Monitor(session) => session.command(request),
            Self::Machine(controller) => controller.session_command(request),
            Self::Debug { adapter, preparing } => {
                let snapshot = adapter.snapshot();
                // The scheduler must retain the same stopped-session boundary
                // as MonitorSession; a terminated debug State is inspection only.
                if snapshot.session.machine.state == "stopped" {
                    return Err("DME1004: monitor session is stale or stopped".into());
                }
                let reset = matches!(&request.command, MonitorCommand::Reset);
                let command = match request.command {
                    MonitorCommand::Advance { cycles } => {
                        if cycles == 0 || cycles > MAX_SLICE {
                            return Err("DME1002: invalid execution slice".into());
                        }
                        if snapshot.status.state == "running" {
                            DebugCommand::Poll { cycles }
                        } else if *preparing && snapshot.status.state == "ready" {
                            DebugCommand::Session {
                                command: MonitorCommand::Advance { cycles },
                            }
                        } else {
                            DebugCommand::State
                        }
                    }
                    command => DebugCommand::Session { command },
                };
                let polled = matches!(command, DebugCommand::Poll { .. });
                let reply = adapter.command(
                    DebugRequest {
                        schema_version: 1,
                        revision: snapshot.revision,
                        command,
                    },
                    || false,
                )?;
                if reset || (polled && reply.status.state == "ready") {
                    // The guest is back at its direct prompt. Keep the existing
                    // scheduler servicing input; a later Debug explicitly runs again.
                    *preparing = true;
                }
                Ok(reply.session)
            }
        }
    }
    /// Read-only optional debug state; no source text, PID or OEM capability.
    pub fn debug_state(&self) -> Result<Option<String>, String> {
        match self {
            Self::Original(session) => session.debug_state(),
            Self::Monitor(_) => Ok(None),
            Self::Machine(controller) => controller.snapshot_json().map(Some),
            Self::Debug { adapter, .. } => adapter.snapshot_json().map(Some),
        }
    }
    pub fn debug_command(&mut self, json: &str) -> Result<String, String> {
        if let Self::Original(session) = self {
            return session.debug_command(json);
        }
        if let Self::Machine(controller) = self {
            return controller.command_json(json);
        }
        if json.len() > MAX_MONITOR_REQUEST_BYTES {
            return Err("DME1302: debug request is too large".into());
        }
        let request: DebugRequest =
            serde_json::from_str(json).map_err(|_| "DME1302: invalid BASIC debug request")?;
        if !matches!(
            request.command,
            DebugCommand::State | DebugCommand::Continue | DebugCommand::Cancel
        ) {
            return Err(
                "DME1302: execution slices and media belong to the existing session scheduler"
                    .into(),
            );
        }
        let Self::Debug { adapter, preparing } = self else {
            return Err("DME1302: this session has no fresh debug build".into());
        };
        let continued = matches!(request.command, DebugCommand::Continue);
        let reply = adapter.command_json(json, || false)?;
        // A valid rejected Continue leaves preparation untouched. The authoritative
        // adapter state decides whether this request actually began a search.
        if continued && adapter.snapshot().status.state == "running" {
            *preparing = false;
        }
        Ok(reply)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eightbit_dragon_basic_core::{
        Dialect, analyse, inspection::source_variable_names, tokenise,
    };
    use serde_json::{Value, json};
    const SOURCE: &str = "10 COUNT=41\n20 COUNT=COUNT+1:PRINT COUNT\n30 END\n";
    fn launch(source: &str, line: u16) -> Result<DevelopmentDebugLaunch, String> {
        let dialect = Dialect::dragon32().map_err(|e| e.to_string())?;
        let tokens =
            tokenise(&analyse(source, &dialect).program, &dialect).map_err(|e| e.to_string())?;
        let bytes = eightbit_dragon_media::create_tokenised_basic("HOST", &tokens.bytes)
            .map_err(|e| e.to_string())?;
        DevelopmentDebugLaunch::new(
            OriginalDevelopmentLaunch::with_cassette(bytes, Some("a".repeat(64)))?,
            tokens,
            line,
            source_variable_names(source),
        )
    }
    fn state(host: &DevelopmentSession) -> Result<Value, String> {
        serde_json::from_str(&host.debug_state()?.ok_or("Missing debugger")?)
            .map_err(|e| e.to_string())
    }
    fn command(
        host: &mut DevelopmentSession,
        command: MonitorCommand,
    ) -> Result<SessionSnapshot, String> {
        host.command(MonitorRequest {
            schema_version: 1,
            session_id: host.id(),
            command,
        })
    }
    fn debug(host: &mut DevelopmentSession, operation: &str) -> Result<Value, String> {
        let request = json!({"schemaVersion":1,"revision":state(host)?["revision"],"command":{"operation":operation}});
        serde_json::from_str(&host.debug_command(&request.to_string())?).map_err(|e| e.to_string())
    }
    fn tick(host: &mut DevelopmentSession) -> Result<SessionSnapshot, String> {
        command(host, MonitorCommand::Advance { cycles: MAX_SLICE })
    }
    #[test]
    fn stopped_debug_owner_rejects_scheduler_commands_without_losing_inspection()
    -> Result<(), String> {
        let mut host = launch(SOURCE, 20)?.start(7)?;
        let stopped = command(&mut host, MonitorCommand::Stop)?;
        assert_eq!(stopped.machine.state, "stopped");
        let terminated = state(&host)?;
        assert_eq!(terminated["status"]["state"], "terminated");
        for request in [
            MonitorCommand::Advance { cycles: MAX_SLICE },
            MonitorCommand::State,
            MonitorCommand::Reset,
            MonitorCommand::Release,
            MonitorCommand::Type { text: "RUN".into() },
            MonitorCommand::Stop,
        ] {
            assert!(command(&mut host, request).is_err_and(|error| error.starts_with("DME1004:")));
        }
        assert_eq!(state(&host)?, terminated);
        assert_eq!(host.snapshot().machine.cycles, stopped.machine.cycles);
        let fresh = launch(SOURCE, 20)?.start(8)?;
        assert_eq!(state(&fresh)?["status"]["state"], "ready");
        Ok(())
    }
    fn stopped(host: &mut DevelopmentSession) -> Result<Value, String> {
        for _ in 0..512 {
            tick(host)?;
            let frame = state(host)?;
            if frame["status"]["state"] != "running" {
                assert_eq!(frame["status"]["state"], "stopped");
                return Ok(frame);
            }
        }
        Err("Guest did not reach its bounded breakpoint".into())
    }
    #[test]
    fn actual_guest_load_stop_continue_cancel_reset_has_one_scheduler() -> Result<(), String> {
        let mut host = launch(SOURCE, 20)?.start(7)?;
        assert_eq!(state(&host)?["status"]["state"], "ready");
        command(
            &mut host,
            MonitorCommand::Type {
                text: "CLOAD".into(),
            },
        )?;
        for _ in 0..384 {
            tick(&mut host)?;
        }
        // The explicit Debug action queues RUN then Continue before the next frame.
        command(&mut host, MonitorCommand::Type { text: "RUN".into() })?;
        debug(&mut host, "continue")?;
        let first = stopped(&mut host)?;
        assert_eq!(first["status"]["location"]["basicLine"], 20);
        assert_eq!(first["status"]["location"]["ordinal"], 0);
        assert_eq!(first["status"]["variables"][0]["name"], "COUNT");
        assert_eq!(first["status"]["variables"][0]["numericValue"], 41.0);
        for _ in 0..8 {
            tick(&mut host)?;
        }
        assert_eq!(state(&host)?, first);
        assert!(command(&mut host, MonitorCommand::MountWav { bytes: vec![0] }).is_err());
        assert_eq!(state(&host)?, first);
        let old = json!({"schemaVersion":1,"revision":first["revision"],"command":{"operation":"continue"}}).to_string();
        debug(&mut host, "continue")?;
        assert!(host.debug_command(&old).is_err());
        let cancelled = debug(&mut host, "cancel")?;
        assert_eq!(cancelled["status"]["state"], "cancelled");
        for _ in 0..8 {
            tick(&mut host)?;
        }
        assert_eq!(state(&host)?, cancelled);
        debug(&mut host, "continue")?;
        let second = stopped(&mut host)?;
        assert_eq!(second["status"]["location"]["ordinal"], 1);
        assert_eq!(second["status"]["variables"][0]["numericValue"], 42.0);
        let revision = second["revision"].clone();
        for command in [
            json!({"operation":"poll","cycles":1}),
            json!({"operation":"session","command":{"operation":"advance","cycles":1}}),
            json!({"operation":"session","command":{"operation":"reset"}}),
        ] {
            assert!(
                host.debug_command(
                    &json!({"schemaVersion":1,"revision":revision,"command":command}).to_string()
                )
                .is_err()
            );
        }
        assert_eq!(state(&host)?, second);
        command(&mut host, MonitorCommand::Reset)?;
        assert_eq!(state(&host)?["status"]["state"], "ready");
        assert!(
            state(&host)?["status"]["variables"]
                .as_array()
                .is_some_and(Vec::is_empty)
        );
        tick(&mut host)?;
        assert!(host.debug_command(&old).is_err());
        command(&mut host, MonitorCommand::Stop)?;
        assert_eq!(state(&host)?["status"]["state"], "terminated");
        assert!(host.export_recording().is_err());
        Ok(())
    }
    #[test]
    fn actual_guest_end_returns_to_prompt_and_can_run_again() -> Result<(), String> {
        let mut host = launch(SOURCE, 20)?.start(7)?;
        command(
            &mut host,
            MonitorCommand::Type {
                text: "CLOAD".into(),
            },
        )?;
        for _ in 0..384 {
            tick(&mut host)?;
        }
        for _ in 0..2 {
            command(&mut host, MonitorCommand::Type { text: "RUN".into() })?;
            debug(&mut host, "continue")?;
            let first = stopped(&mut host)?;
            assert_eq!(first["status"]["variables"][0]["numericValue"], 41.0);
            debug(&mut host, "continue")?;
            let second = stopped(&mut host)?;
            assert_eq!(second["status"]["variables"][0]["numericValue"], 42.0);
            debug(&mut host, "continue")?;
            let mut ready = false;
            for _ in 0..64 {
                tick(&mut host)?;
                if state(&host)?["status"]["state"] == "ready" {
                    ready = true;
                    break;
                }
            }
            assert!(
                ready,
                "guest returned to prompt but debug status remained {:?}",
                state(&host)?["status"]["state"]
            );
            assert_eq!(
                state(&host)?["status"]["message"],
                "Programme returned to BASIC prompt"
            );
            assert_eq!(state(&host)?["status"]["valuesValid"], false);
            assert_eq!(state(&host)?["status"]["variables"], json!([]));
            assert!(host.snapshot().machine.text.contains("DEV>"));
            for _ in 0..8 {
                tick(&mut host)?;
            }
            assert_eq!(
                state(&host)?["status"]["message"],
                "Programme returned to BASIC prompt"
            );
        }
        Ok(())
    }

    #[test]
    fn actual_guest_input_accepts_release_reply_cancel_and_resume_without_losing_search()
    -> Result<(), String> {
        let mut host = launch("10 COUNT=41\n20 INPUT COUNT\n30 PRINT COUNT:END\n", 30)?.start(7)?;
        command(
            &mut host,
            MonitorCommand::Type {
                text: "CLOAD".into(),
            },
        )?;
        for _ in 0..384 {
            tick(&mut host)?;
        }
        command(&mut host, MonitorCommand::Type { text: "RUN".into() })?;
        debug(&mut host, "continue")?;
        // Starting Debug from the focused guest canvas queues this blur release.
        command(&mut host, MonitorCommand::Release)?;
        assert_eq!(state(&host)?["status"]["state"], "running");
        for _ in 0..16 {
            tick(&mut host)?;
        }
        assert_eq!(state(&host)?["status"]["state"], "running");
        assert!(host.snapshot().machine.text.contains('?'));
        for command_value in [
            MonitorCommand::Key {
                code: "ShiftLeft".into(),
                pressed: true,
            },
            MonitorCommand::Key {
                code: "ShiftLeft".into(),
                pressed: false,
            },
            MonitorCommand::Release,
        ] {
            let before = state(&host)?;
            command(&mut host, command_value)?;
            let after = state(&host)?;
            assert_eq!(after["status"]["state"], "running");
            assert_ne!(after["revision"], before["revision"]);
            let stale = json!({"schemaVersion":1,"revision":before["revision"],"command":{"operation":"cancel"}});
            assert!(host.debug_command(&stale.to_string()).is_err());
        }
        let cancelled = debug(&mut host, "cancel")?;
        assert_eq!(cancelled["status"]["state"], "cancelled");
        for _ in 0..4 {
            tick(&mut host)?;
        }
        assert_eq!(state(&host)?, cancelled);
        command(&mut host, MonitorCommand::Release)?;
        assert_eq!(state(&host)?["status"]["state"], "cancelled");
        debug(&mut host, "continue")?;
        command(&mut host, MonitorCommand::Type { text: "42".into() })?;
        command(&mut host, MonitorCommand::Release)?;
        assert_eq!(state(&host)?["status"]["state"], "running");
        let reached = stopped(&mut host)?;
        assert_eq!(reached["status"]["location"]["basicLine"], 30);
        assert_eq!(reached["status"]["variables"][0]["numericValue"], 42.0);
        command(&mut host, MonitorCommand::Release)?;
        let released = state(&host)?;
        for field in ["state", "location", "registers", "variables", "valuesValid"] {
            assert_eq!(released["status"][field], reached["status"][field]);
        }
        assert_ne!(released["status"]["stopId"], reached["status"]["stopId"]);
        command(&mut host, MonitorCommand::Reset)?;
        assert_eq!(state(&host)?["status"]["state"], "ready");
        Ok(())
    }

    #[test]
    fn only_exact_fresh_basic_builds_can_create_a_debug_owner() -> Result<(), String> {
        assert!(launch(SOURCE, 99).is_err());
        let dialect = Dialect::dragon32().map_err(|e| e.to_string())?;
        let program =
            tokenise(&analyse(SOURCE, &dialect).program, &dialect).map_err(|e| e.to_string())?;
        let other = tokenise(&analyse("10 PRINT 9\n", &dialect).program, &dialect)
            .map_err(|e| e.to_string())?;
        let cas = eightbit_dragon_media::create_tokenised_basic("OTHER", &other.bytes)
            .map_err(|e| e.to_string())?;
        assert!(
            DevelopmentDebugLaunch::new(
                OriginalDevelopmentLaunch::with_cassette(cas, Some("a".repeat(64)))?,
                program,
                20,
                BTreeMap::new()
            )
            .is_err()
        );
        let mut monitor = DevelopmentSession::from_monitor(MonitorSession::new(8)?);
        assert!(monitor.debug_state()?.is_none());
        assert!(monitor.debug_command("{}").is_err());
        assert!(
            monitor
                .command(MonitorRequest {
                    schema_version: 1,
                    session_id: 9,
                    command: MonitorCommand::Reset
                })
                .is_err()
        );
        Ok(())
    }
}
