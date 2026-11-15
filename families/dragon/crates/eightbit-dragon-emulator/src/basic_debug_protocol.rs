//! Shared development-only wire boundary for browser and native composition.
//! The trusted build hands over an existing controller in-process; requests never
//! accept source maps, ROM bytes, profiles, executable paths or a process ID.

use crate::basic_debug::{
    BasicDebugController, BasicDebugFrame, BasicDebugRevision, BasicDebugState,
};
use crate::{MAX_MONITOR_REQUEST_BYTES, MonitorCommand, SessionSnapshot};
use eightbit_dragon_basic_core::inspection::BasicVariable;
use serde::{Deserialize, Serialize};

pub const MAX_DEBUG_RESPONSE_BYTES: usize = 12 * 1024 * 1024;

/// u64 revisions are canonical decimal text so JavaScript cannot round them.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DebugRevision {
    pub session_id: u32,
    pub build_id: String,
    pub revision: String,
}
impl From<BasicDebugRevision> for DebugRevision {
    fn from(value: BasicDebugRevision) -> Self {
        Self {
            session_id: value.session_id,
            build_id: value.build_id,
            revision: value.revision.to_string(),
        }
    }
}
impl DebugRevision {
    fn checked(&self) -> Result<BasicDebugRevision, String> {
        let invalid = "DME1303: invalid BASIC debug revision";
        if self.revision.is_empty()
            || self.revision.len() > 20
            || self.session_id == 0
            || self.build_id.len() != 64
        {
            return Err(invalid.into());
        }
        let revision = self
            .revision
            .parse::<u64>()
            .map_err(|_| invalid.to_string())?;
        if revision.to_string() != self.revision {
            return Err(invalid.into());
        }
        Ok(BasicDebugRevision {
            session_id: self.session_id,
            build_id: self.build_id.clone(),
            revision,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
pub enum DebugCommand {
    State,
    Continue,
    Poll { cycles: u32 },
    Cancel,
    Session { command: MonitorCommand },
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DebugRequest {
    pub schema_version: u32,
    pub revision: DebugRevision,
    pub command: DebugCommand,
}

#[derive(Debug, Serialize)]
pub struct DebugRegisters {
    pub a: u8,
    pub b: u8,
    pub dp: u8,
    pub cc: u8,
    pub x: u16,
    pub y: u16,
    pub u: u16,
    pub s: u16,
    pub pc: u16,
}
impl From<crate::debug::Registers> for DebugRegisters {
    fn from(r: crate::debug::Registers) -> Self {
        Self {
            a: r.a,
            b: r.b,
            dp: r.dp,
            cc: r.cc,
            x: r.x,
            y: r.y,
            u: r.u,
            s: r.s,
            pc: r.pc,
        }
    }
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugLocation {
    pub basic_line: u16,
    pub ordinal: usize,
    pub source_start: usize,
    pub source_end: usize,
    pub runtime_address: u16,
    pub token_length: usize,
}

/// Familiar inspection fields without pretending the original owns an `XRoar` PID
/// or has passed the OEM profile checks. Absence of a stop clears all values.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugStatus {
    pub state: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_id: Option<String>,
    pub breakpoint_verified: bool,
    pub values_valid: bool,
    pub variables: Vec<BasicVariable>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registers: Option<DebugRegisters>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<DebugLocation>,
}
#[derive(Debug, Serialize)]
pub struct DebugFailure {
    pub code: String,
    pub message: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugReply {
    pub schema_version: u32,
    pub backend: &'static str,
    pub revision: DebugRevision,
    pub status: DebugStatus,
    pub session: SessionSnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<DebugFailure>,
}
impl From<BasicDebugFrame> for DebugReply {
    fn from(frame: BasicDebugFrame) -> Self {
        let mut status = DebugStatus {
            state: "ready",
            message: "Original BASIC development debugger ready".into(),
            stop_id: None,
            breakpoint_verified: false,
            values_valid: false,
            variables: Vec::new(),
            registers: None,
            location: None,
        };
        match frame.state {
            BasicDebugState::Ready => {}
            BasicDebugState::Prompt => {
                status.message = "Programme returned to BASIC prompt".into();
            }
            BasicDebugState::Running { .. } => {
                status.state = "running";
                status.message = "Searching for the original BASIC breakpoint".into();
            }
            BasicDebugState::Stopped(stop) => {
                status.state = "stopped";
                status.message = format!(
                    "Original development breakpoint at BASIC line {}",
                    stop.location.basic_line
                );
                status.stop_id = Some(frame.revision.revision.to_string());
                status.breakpoint_verified = true;
                status.values_valid = true;
                status.variables = stop.variables;
                status.registers = Some(stop.registers.into());
                status.location = Some(DebugLocation {
                    basic_line: stop.location.basic_line,
                    ordinal: stop.location.ordinal,
                    source_start: stop.location.source_range.start,
                    source_end: stop.location.source_range.end,
                    runtime_address: stop.location.runtime_address,
                    token_length: stop.location.token_length,
                });
            }
            BasicDebugState::Cancelled => {
                status.state = "cancelled";
                status.message = "Original BASIC breakpoint search cancelled".into();
            }
            BasicDebugState::Failed(message) => {
                status.state = "failed";
                status.message = message;
            }
            BasicDebugState::Terminated => {
                status.state = "terminated";
                status.message = "Original BASIC debug session stopped".into();
            }
        }
        Self {
            schema_version: 1,
            backend: "original-development",
            revision: frame.revision.into(),
            status,
            session: frame.session,
            failure: None,
        }
    }
}

/// Thin wire adapter. All guest execution and lifecycle rules remain in the
/// controller. Native hosts can pass an atomic cancellation predicate; browser
/// workers cancel between bounded polls or terminate their owned worker.
pub struct BasicDebugAdapter {
    controller: BasicDebugController,
    revision: BasicDebugRevision,
    normal_original: bool,
}
impl BasicDebugAdapter {
    #[must_use]
    pub fn new(controller: BasicDebugController) -> Self {
        let revision = controller.snapshot().revision;
        Self {
            controller,
            revision,
            normal_original: false,
        }
    }
    pub(crate) fn preparation_machine_mut(&mut self) -> &mut crate::Machine {
        self.controller.preparation_machine_mut()
    }
    pub(crate) fn preparation_frame(&mut self) -> SessionSnapshot {
        self.controller.preparation_frame()
    }
    pub(crate) fn original(controller: BasicDebugController) -> Self {
        let mut adapter = Self::new(controller);
        adapter.normal_original = true;
        adapter
    }
    fn reply(&self, frame: BasicDebugFrame) -> DebugReply {
        let mut reply: DebugReply = frame.into();
        if self.normal_original {
            reply.backend = "original";
            reply.status.message = reply.status.message.replace(" development", "");
        }
        reply
    }
    #[must_use]
    pub fn session_id(&self) -> u32 {
        self.revision.session_id
    }
    #[must_use]
    pub fn snapshot(&self) -> DebugReply {
        self.reply(self.controller.snapshot())
    }
    pub fn snapshot_json(&self) -> Result<String, String> {
        encode(&self.snapshot())
    }
    pub fn command(
        &mut self,
        request: DebugRequest,
        cancelled: impl FnMut() -> bool,
    ) -> Result<DebugReply, String> {
        if request.schema_version != 1 {
            return Err("DME1302: unsupported BASIC debug schema".into());
        }
        let revision = request.revision.checked()?;
        if revision != self.revision {
            return Err("DME1303: BASIC debug request belongs to a stale revision".into());
        }
        let frame = match request.command {
            DebugCommand::State => self.controller.snapshot(),
            DebugCommand::Continue => {
                self.controller.continue_execution(&revision)?;
                self.controller.snapshot()
            }
            DebugCommand::Poll { cycles } => self.controller.poll(&revision, cycles, cancelled)?,
            DebugCommand::Cancel => self.controller.cancel(&revision)?,
            DebugCommand::Session { command } => self.controller.command(&revision, command)?,
        };
        self.revision = frame.revision.clone();
        Ok(self.reply(frame))
    }
    pub fn command_json(
        &mut self,
        json: &str,
        cancelled: impl FnMut() -> bool,
    ) -> Result<String, String> {
        if json.len() > MAX_MONITOR_REQUEST_BYTES {
            return Err("DME1302: BASIC debug request is too large".into());
        }
        let request: DebugRequest = serde_json::from_str(json)
            .map_err(|_| "DME1302: invalid BASIC debug request".to_string())?;
        // A rejected command may still change media diagnostics (for example a
        // failed recording finish). Return that exact owned state so the caller
        // can recover without destroying a valid tape. Unowned envelopes get no
        // current inspection or recovery authority.
        if request.schema_version != 1 {
            return Err("DME1302: unsupported BASIC debug schema".into());
        }
        if request.revision.checked()? != self.revision {
            return Err("DME1303: BASIC debug request belongs to a stale revision".into());
        }
        let reply = match self.command(request, cancelled) {
            Ok(reply) => reply,
            Err(error) => {
                let mut reply = self.snapshot();
                let code = error.split_once(':').map_or("", |(code, _)| code);
                let known = matches!(
                    code,
                    "DME1001"
                        | "DME1002"
                        | "DME1003"
                        | "DME1004"
                        | "DME1005"
                        | "DME1006"
                        | "DME1007"
                        | "DME1300"
                        | "DME1301"
                        | "DME1302"
                        | "DME1303"
                        | "DME1304"
                );
                reply.failure = Some(DebugFailure {
                    code: if known { code } else { "DME1302" }.into(),
                    message: if known {
                        error.chars().take(256).collect()
                    } else {
                        "Original BASIC debug command was rejected".into()
                    },
                });
                reply
            }
        };
        encode(&reply)
    }
}
pub(crate) fn encode(reply: &DebugReply) -> Result<String, String> {
    // Session export <=4 MiB JSON, aliases <=6 MiB escaped JSON, and the fixed
    // frame/scalar/metadata allowance fit within this bounded response envelope.
    let json = serde_json::to_string(reply)
        .map_err(|_| "DME1302: cannot encode BASIC debug state".to_string())?;
    if json.len() > MAX_DEBUG_RESPONSE_BYTES {
        return Err("DME1302: BASIC debug response is too large".into());
    }
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Firmware, MAX_SLICE, MonitorRequest, MonitorSession};
    use eightbit_dragon_basic_core::breakpoints::StatementMap;
    use eightbit_dragon_basic_core::{Dialect, analyse, tokenise};
    use std::collections::BTreeMap;

    fn adapter(load: bool, invalid_opcode: bool) -> Result<BasicDebugAdapter, String> {
        let dialect = Dialect::dragon32().map_err(|error| error.to_string())?;
        let tokens = tokenise(
            &analyse("10 COUNT=41\n20 COUNT=COUNT+1:END\n", &dialect).program,
            &dialect,
        )
        .map_err(|error| error.to_string())?;
        let mut session = MonitorSession::with_firmware(9, Firmware::BasicDevelopment)?;
        for _ in 0..8 {
            session.command(MonitorRequest {
                schema_version: 1,
                session_id: 9,
                command: MonitorCommand::Advance { cycles: MAX_SLICE },
            })?;
        }
        if load {
            let bytes = eightbit_dragon_media::create_tokenised_basic("ADAPTER", &tokens.bytes)
                .map_err(|error| error.to_string())?;
            for command in [
                MonitorCommand::MountCassette { bytes },
                MonitorCommand::Type {
                    text: "CLOAD\"ADAPTER\"".into(),
                },
            ] {
                session.command(MonitorRequest {
                    schema_version: 1,
                    session_id: 9,
                    command,
                })?;
            }
            for _ in 0..256 {
                session.command(MonitorRequest {
                    schema_version: 1,
                    session_id: 9,
                    command: MonitorCommand::Advance { cycles: MAX_SLICE },
                })?;
            }
        }
        if invalid_opcode {
            // Authored invalid instruction exercises the actual CPU failure latch.
            let machine = session.debug_machine_mut();
            machine.board.ram[0x6000] = 0x01;
            machine.cpu.registers.pc = 0x6000;
            machine.cpu.registers.cc = 0x50;
        }
        Ok(BasicDebugAdapter::new(BasicDebugController::new(
            session,
            StatementMap::new(tokens)?,
            20,
            "a".repeat(64),
            BTreeMap::from([("CO".into(), "COUNT".into())]),
        )?))
    }
    fn send(adapter: &mut BasicDebugAdapter, command: DebugCommand) -> Result<DebugReply, String> {
        adapter.command(
            DebugRequest {
                schema_version: 1,
                revision: adapter.snapshot().revision,
                command,
            },
            || false,
        )
    }
    fn json_request(revision: DebugRevision, command: DebugCommand) -> Result<String, String> {
        serde_json::to_string(&DebugRequest {
            schema_version: 1,
            revision,
            command,
        })
        .map_err(|error| error.to_string())
    }
    fn wait(adapter: &mut BasicDebugAdapter) -> Result<serde_json::Value, String> {
        for _ in 0..513 {
            let request = json_request(
                adapter.snapshot().revision,
                DebugCommand::Poll { cycles: MAX_SLICE },
            )?;
            let response = adapter.command_json(&request, || false)?;
            let value: serde_json::Value =
                serde_json::from_str(&response).map_err(|error| error.to_string())?;
            if value["status"]["state"] != "running" {
                return Ok(value);
            }
        }
        Err("bounded debug wire search did not finish".into())
    }
    #[test]
    fn actual_guest_wire_stops_cancel_resume_and_stop_keep_exact_values_and_ownership()
    -> Result<(), String> {
        let mut adapter = adapter(true, false)?;
        send(
            &mut adapter,
            DebugCommand::Session {
                command: MonitorCommand::Type { text: "RUN".into() },
            },
        )?;
        send(&mut adapter, DebugCommand::Continue)?;
        let stopped = wait(&mut adapter)?;
        assert_eq!(stopped["schemaVersion"], 1);
        assert_eq!(stopped["backend"], "original-development");
        assert_eq!(stopped["status"]["state"], "stopped");
        assert_eq!(stopped["status"]["location"]["basicLine"], 20);
        assert_eq!(stopped["status"]["location"]["ordinal"], 0);
        assert_eq!(stopped["status"]["registers"]["pc"], 0x84d6);
        assert_eq!(stopped["status"]["variables"][0]["name"], "COUNT");
        assert_eq!(stopped["status"]["variables"][0]["numericValue"], 41.0);
        assert_eq!(stopped["status"]["stopId"], stopped["revision"]["revision"]);
        assert!(stopped["status"].get("processId").is_none());
        let old = adapter.snapshot().revision;
        let rejected = adapter.command_json(
            &json_request(
                old.clone(),
                DebugCommand::Session {
                    command: MonitorCommand::MountWav { bytes: vec![0] },
                },
            )?,
            || false,
        )?;
        let rejected: serde_json::Value =
            serde_json::from_str(&rejected).map_err(|error| error.to_string())?;
        assert_eq!(rejected["failure"]["code"], "DME1005");
        assert_eq!(rejected["revision"], stopped["revision"]);
        assert_eq!(rejected["status"], stopped["status"]);
        assert_eq!(
            rejected["session"]["cassette"],
            stopped["session"]["cassette"]
        );
        send(&mut adapter, DebugCommand::Continue)?;
        let request = json_request(
            adapter.snapshot().revision,
            DebugCommand::Poll { cycles: MAX_SLICE },
        )?;
        let cancelled: serde_json::Value =
            serde_json::from_str(&adapter.command_json(&request, || true)?)
                .map_err(|error| error.to_string())?;
        assert_eq!(cancelled["status"]["state"], "cancelled");
        assert_eq!(cancelled["status"]["variables"], serde_json::json!([]));
        for key in ["stopId", "registers", "location"] {
            assert!(cancelled["status"].get(key).is_none());
        }
        assert!(
            adapter
                .command_json(&json_request(old, DebugCommand::Continue)?, || false)
                .is_err()
        );
        send(&mut adapter, DebugCommand::Continue)?;
        let next = wait(&mut adapter)?;
        assert_eq!(next["status"]["location"]["ordinal"], 1);
        assert_eq!(next["status"]["variables"][0]["numericValue"], 42.0);
        let stopped = send(
            &mut adapter,
            DebugCommand::Session {
                command: MonitorCommand::Stop,
            },
        )?;
        assert_eq!(stopped.status.state, "terminated");
        assert_eq!(stopped.session.machine.state, "stopped");
        assert!(!stopped.status.values_valid);
        assert_eq!(
            send(&mut adapter, DebugCommand::State)?.status.state,
            "terminated"
        );
        assert!(send(&mut adapter, DebugCommand::Continue).is_err());
        Ok(())
    }
    #[test]
    fn malformed_stale_oversized_and_external_maps_never_mutate_the_adapter() -> Result<(), String>
    {
        let mut adapter = adapter(false, false)?;
        let good = json_request(adapter.snapshot().revision, DebugCommand::State)?;
        let value: serde_json::Value =
            serde_json::from_str(&good).map_err(|error| error.to_string())?;
        let initial = adapter.snapshot_json()?;
        for path in ["sourceMap", "firmware", "processId", "profile"] {
            let mut bad = value.clone();
            bad[path] = serde_json::json!({});
            assert!(adapter.command_json(&bad.to_string(), || false).is_err());
        }
        for bad in ["", "{", &" ".repeat(MAX_MONITOR_REQUEST_BYTES + 1)] {
            assert!(adapter.command_json(bad, || false).is_err());
        }
        for (field, bad) in [
            ("sessionId", serde_json::json!(8)),
            ("buildId", serde_json::json!("b".repeat(64))),
            ("revision", serde_json::json!("1")),
            ("revision", serde_json::json!(0)),
            ("revision", serde_json::json!("00")),
            ("revision", serde_json::json!("+0")),
            ("revision", serde_json::json!("18446744073709551616")),
        ] {
            let mut request = value.clone();
            request["revision"][field] = bad;
            assert!(
                adapter
                    .command_json(&request.to_string(), || false)
                    .is_err()
            );
        }
        let mut bad = value.clone();
        bad["schemaVersion"] = serde_json::json!(2);
        assert!(adapter.command_json(&bad.to_string(), || false).is_err());
        assert_eq!(adapter.snapshot_json()?, initial);
        send(&mut adapter, DebugCommand::Continue)?;
        for cycles in [0, MAX_SLICE + 1] {
            assert!(send(&mut adapter, DebugCommand::Poll { cycles }).is_err());
        }
        assert!(send(&mut adapter, DebugCommand::Continue).is_err());
        assert!(
            send(
                &mut adapter,
                DebugCommand::Session {
                    command: MonitorCommand::MountWav { bytes: vec![0] }
                }
            )
            .is_err()
        );
        assert_eq!(
            send(&mut adapter, DebugCommand::Cancel)?.status.state,
            "cancelled"
        );
        send(&mut adapter, DebugCommand::Continue)?;
        Ok(())
    }
    #[test]
    fn real_cpu_failure_is_a_value_invalid_frame_and_reset_recovers() -> Result<(), String> {
        let mut adapter = adapter(false, true)?;
        send(&mut adapter, DebugCommand::Continue)?;
        let failed = send(&mut adapter, DebugCommand::Poll { cycles: 1 })?;
        assert_eq!(failed.status.state, "failed");
        assert!(failed.status.message.starts_with("DME1001:"));
        assert!(!failed.status.values_valid);
        assert!(failed.status.variables.is_empty());
        assert!(failed.status.registers.is_none());
        assert!(send(&mut adapter, DebugCommand::Continue).is_err());
        assert_eq!(
            send(
                &mut adapter,
                DebugCommand::Session {
                    command: MonitorCommand::Reset
                }
            )?
            .status
            .state,
            "ready"
        );
        send(&mut adapter, DebugCommand::Continue)?;
        assert_eq!(
            send(&mut adapter, DebugCommand::Poll { cycles: 1 })?
                .status
                .state,
            "running"
        );
        Ok(())
    }
    #[test]
    fn full_u64_revision_round_trip_never_uses_json_numbers() -> Result<(), String> {
        for revision in [0, 9_007_199_254_740_993, u64::MAX] {
            let core = BasicDebugRevision {
                session_id: 9,
                build_id: "a".repeat(64),
                revision,
            };
            let wire = DebugRevision::from(core.clone());
            let encoded = serde_json::to_string(&wire).map_err(|error| error.to_string())?;
            let decoded: DebugRevision =
                serde_json::from_str(&encoded).map_err(|error| error.to_string())?;
            assert_eq!(decoded.checked()?, core);
            assert!(encoded.contains(&format!("\"revision\":\"{revision}\"")));
        }
        Ok(())
    }
    #[test]
    fn largest_inspection_and_cassette_export_fit_the_bounded_response() -> Result<(), String> {
        let mut reply = adapter(false, false)?.snapshot();
        reply.status.state = "stopped";
        reply.status.stop_id = Some(reply.revision.revision.clone());
        reply.status.values_valid = true;
        reply.status.breakpoint_verified = true;
        reply.status.registers = Some(crate::debug::Registers::default().into());
        reply.status.location = Some(DebugLocation {
            basic_line: 20,
            ordinal: 0,
            source_start: 0,
            source_end: 1,
            runtime_address: 0x1e01,
            token_length: 1,
        });
        reply.status.variables = vec![
            BasicVariable {
                name: "\0".repeat(8192),
                runtime_name: "CO$".into(),
                kind: "string",
                display_value: format!("\"{}\"", "\0".repeat(255)),
                numeric_value: None,
                string_value: Some("\0".repeat(255)),
            };
            128
        ];
        reply.session.recording_export = Some(crate::RecordingExport {
            filename: "dragon-recording.cas".into(),
            bytes: vec![255; crate::MAX_SESSION_CASSETTE_BYTES],
        });
        reply.session.machine.pixels.fill(255);
        let encoded = encode(&reply)?;
        assert!(encoded.len() > 11 * 1024 * 1024);
        assert!(encoded.len() <= MAX_DEBUG_RESPONSE_BYTES);
        reply.status.message = "x".repeat(MAX_DEBUG_RESPONSE_BYTES);
        assert!(encode(&reply).is_err());
        Ok(())
    }
}
