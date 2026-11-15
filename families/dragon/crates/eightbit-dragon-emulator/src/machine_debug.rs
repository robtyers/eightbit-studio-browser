//! Portable machine-code inspection with one bounded execution owner.
use crate::basic_debug_protocol::{DebugFailure, DebugRegisters, DebugRevision};
use crate::debug::{Breakpoints, DebugStopReason};
use crate::{MAX_SLICE, MonitorCommand, MonitorRequest, MonitorSession, SessionSnapshot};
use serde::{Deserialize, Serialize};

const MAX_SOURCE_ENTRIES: usize = 32768;
const INVALID: &str = "DME1500: invalid machine debugger request or mapping";
const STALE: &str = "DME1501: machine debugger request belongs to a stale or stopped owner";

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MachineSourceEntry {
    pub source_uri: String,
    pub source_line: usize,
    pub runtime_address: u16,
    pub byte_length: u16,
}
#[derive(Clone, Debug)]
pub struct MachineDebugTarget {
    address: u16,
    entries: Vec<MachineSourceEntry>,
    ram_end_exclusive: u16,
}
impl MachineDebugTarget {
    pub fn new(address: u16, entries: Vec<MachineSourceEntry>) -> Result<Self, String> {
        Self::with_memory_limit(address, entries, 0x8000)
    }
    pub fn new_dragon64(
        native: bool,
        address: u16,
        entries: Vec<MachineSourceEntry>,
    ) -> Result<Self, String> {
        Self::with_memory_limit(address, entries, if native { 0xc000 } else { 0x8000 })
    }
    fn with_memory_limit(
        address: u16,
        entries: Vec<MachineSourceEntry>,
        ram_end_exclusive: u16,
    ) -> Result<Self, String> {
        if !(0x0400..ram_end_exclusive).contains(&address) || entries.len() > MAX_SOURCE_ENTRIES {
            return Err(INVALID.into());
        }
        for entry in &entries {
            if entry.source_uri.is_empty()
                || entry.source_uri.len() > 1024
                || entry.source_uri.starts_with('/')
                || entry.source_uri.contains('\\')
                || entry
                    .source_uri
                    .split('/')
                    .any(|p| matches!(p, "" | "." | ".."))
                || entry.source_line == 0
                || entry.byte_length == 0
                || entry.runtime_address < 0x0400
                || u32::from(entry.runtime_address) + u32::from(entry.byte_length)
                    > u32::from(ram_end_exclusive)
            {
                return Err(INVALID.into());
            }
        }
        let result = Self {
            address,
            entries,
            ram_end_exclusive,
        };
        result.validate_address(address)?;
        Ok(result)
    }
    fn validate_address(&self, address: u16) -> Result<(), String> {
        if !(0x0400..self.ram_end_exclusive).contains(&address)
            || (!self.entries.is_empty() && self.location(address).is_none())
        {
            return Err(INVALID.into());
        }
        Ok(())
    }
    pub(crate) fn validate_extent(&self, start: u16, length: usize) -> Result<(), String> {
        let end = usize::from(start) + length;
        if usize::from(self.address) < usize::from(start)
            || usize::from(self.address) >= end
            || self.entries.iter().any(|entry| {
                entry.runtime_address < start
                    || usize::from(entry.runtime_address) + usize::from(entry.byte_length) > end
            })
        {
            return Err(INVALID.into());
        }
        Ok(())
    }
    fn location(&self, address: u16) -> Option<MachineSourceEntry> {
        self.entries
            .iter()
            .find(|entry| {
                u32::from(address) >= u32::from(entry.runtime_address)
                    && u32::from(address)
                        < u32::from(entry.runtime_address) + u32::from(entry.byte_length)
            })
            .cloned()
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineMemorySnapshot {
    pub start_address: u16,
    pub bytes: Vec<u8>,
}
#[derive(Debug, Serialize)]
pub struct DisassembledInstruction {
    pub address: u16,
    pub bytes: Vec<u8>,
    pub mnemonic: String,
    pub operand: String,
    pub location: Option<MachineSourceEntry>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineDebugStatus {
    pub state: &'static str,
    pub message: String,
    pub stop_id: Option<String>,
    pub breakpoint_address: u16,
    pub breakpoint_verified: bool,
    pub values_valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registers: Option<DebugRegisters>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<MachineMemorySnapshot>,
    pub disassembly: Vec<DisassembledInstruction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<MachineSourceEntry>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineDebugReply {
    pub schema_version: u32,
    pub backend: &'static str,
    pub kind: &'static str,
    pub revision: DebugRevision,
    pub status: MachineDebugStatus,
    pub session: SessionSnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<DebugFailure>,
}
#[derive(Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
pub enum MachineDebugCommand {
    State,
    Continue,
    Step,
    Pause,
    Cancel,
    Breakpoint { address: u16 },
    Memory { address: u16, length: usize },
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MachineDebugRequest {
    pub schema_version: u32,
    pub revision: DebugRevision,
    pub command: MachineDebugCommand,
}
impl MachineDebugRequest {
    pub(crate) fn parse(json: &str) -> Result<Self, String> {
        if json.len() > 4096 {
            return Err(INVALID.into());
        }
        serde_json::from_str(json).map_err(|_| INVALID.into())
    }
}
#[derive(Clone, Debug)]
enum State {
    Loading,
    Ready,
    Running,
    Stopped(bool),
    Cancelled,
    Failed(String),
    Terminated,
}

pub struct MachineDebugController {
    session: MonitorSession,
    target: MachineDebugTarget,
    build_id: String,
    revision: u64,
    state: State,
    loaded: bool,
    skip_breakpoint: bool,
    memory: Option<(u16, usize)>,
}
impl MachineDebugController {
    pub(crate) fn new(
        session: MonitorSession,
        target: MachineDebugTarget,
        build_id: String,
    ) -> Self {
        Self {
            session,
            target,
            build_id,
            revision: 1,
            state: State::Loading,
            loaded: false,
            skip_breakpoint: false,
            memory: None,
        }
    }
    pub(crate) fn id(&self) -> u32 {
        self.session.id()
    }
    fn bump(&mut self) -> Result<(), String> {
        self.revision = self.revision.checked_add(1).ok_or(STALE)?;
        Ok(())
    }
    fn valid_values(&self) -> bool {
        self.loaded && matches!(self.state, State::Stopped(_) | State::Cancelled)
    }
    pub(crate) fn preparation_machine_mut(&mut self) -> &mut crate::Machine {
        self.session.debug_machine_mut()
    }
    pub(crate) fn preparation_frame(&mut self) -> SessionSnapshot {
        self.session.debug_frame()
    }
    pub(crate) fn loaded(&mut self) {
        self.loaded = true;
        self.state = State::Running;
    }
    pub(crate) fn snapshot(&self) -> MachineDebugReply {
        let (state, message) = match &self.state {
            State::Loading => ("running", "Loading the current machine-debug build".into()),
            State::Ready => (
                "ready",
                "Machine debugger reset; run a fresh build to debug again".into(),
            ),
            State::Running => ("running", "Running to the machine-code breakpoint".into()),
            State::Stopped(true) => (
                "stopped",
                format!("Machine-code breakpoint at ${:04X}", self.target.address),
            ),
            State::Stopped(false) => ("stopped", "Machine-code execution paused".into()),
            State::Cancelled => ("cancelled", "Machine-code execution paused".into()),
            State::Failed(message) => ("failed", message.clone()),
            State::Terminated => ("terminated", "Machine debugger stopped".into()),
        };
        let mut status = MachineDebugStatus {
            state,
            message,
            stop_id: None,
            breakpoint_address: self.target.address,
            breakpoint_verified: matches!(self.state, State::Stopped(true)),
            values_valid: self.valid_values(),
            registers: None,
            memory: None,
            disassembly: Vec::new(),
            location: None,
        };
        if status.values_valid
            && let Ok(machine) = self.session.debug_machine()
        {
            let registers = machine.registers();
            status.stop_id = Some(self.revision.to_string());
            status.location = self.target.location(registers.pc);
            status.registers = Some(registers.into());
            let (address, length) = self
                .memory
                .unwrap_or((registers.pc, bounded_memory_length(registers.pc)));
            if let Ok(bytes) = machine.read_memory(address, length) {
                status.disassembly = disassemble_bytes(address, &bytes, 8);
                for instruction in &mut status.disassembly {
                    instruction.location = self.target.location(instruction.address);
                }
                status.memory = Some(MachineMemorySnapshot {
                    start_address: address,
                    bytes,
                });
            }
        }
        MachineDebugReply {
            schema_version: 1,
            backend: "original",
            kind: "machine",
            revision: DebugRevision {
                session_id: self.id(),
                build_id: self.build_id.clone(),
                revision: self.revision.to_string(),
            },
            status,
            session: self.session.snapshot(),
            failure: None,
        }
    }
    pub(crate) fn snapshot_json(&self) -> Result<String, String> {
        encode(&self.snapshot())
    }
    pub(crate) fn command(&mut self, request: &MachineDebugRequest) -> Result<String, String> {
        if request.schema_version != 1 || request.revision != self.snapshot().revision {
            return Err(STALE.into());
        }
        if matches!(request.command, MachineDebugCommand::State) {
            return self.snapshot_json();
        }
        if matches!(self.state, State::Terminated | State::Failed(_)) || self.revision == u64::MAX {
            return Err(STALE.into());
        }
        match request.command {
            MachineDebugCommand::State => {}
            MachineDebugCommand::Continue => {
                if !matches!(self.state, State::Stopped(_) | State::Cancelled) {
                    return Err(INVALID.into());
                }
                self.skip_breakpoint = self.loaded
                    && self.session.debug_machine()?.registers().pc == self.target.address;
                self.state = if self.loaded {
                    State::Running
                } else {
                    State::Loading
                };
                self.memory = None;
            }
            MachineDebugCommand::Pause | MachineDebugCommand::Cancel => {
                if !matches!(self.state, State::Running | State::Loading) {
                    return Err(INVALID.into());
                }
                self.state = State::Cancelled;
            }
            MachineDebugCommand::Step => {
                if !self.valid_values() {
                    return Err(INVALID.into());
                }
                match self.session.debug_machine_mut().step() {
                    Ok(_) => self.state = State::Stopped(false),
                    Err(error) => self.state = State::Failed(error),
                }
                self.memory = None;
                self.session.debug_frame();
            }
            MachineDebugCommand::Breakpoint { address } => {
                if !self.valid_values() {
                    return Err(INVALID.into());
                }
                self.target.validate_address(address)?;
                self.target.address = address;
                self.state = State::Stopped(false);
            }
            MachineDebugCommand::Memory { address, length } => {
                if !self.valid_values() {
                    return Err(INVALID.into());
                }
                self.session.debug_machine()?.read_memory(address, length)?;
                self.memory = Some((address, length));
            }
        }
        self.bump()?;
        self.snapshot_json()
    }
    pub(crate) fn rejection(
        &self,
        request: &MachineDebugRequest,
        error: &str,
    ) -> Result<MachineDebugReply, String> {
        if request.schema_version != 1 || request.revision != self.snapshot().revision {
            return Err(STALE.into());
        }
        let mut reply = self.snapshot();
        let code = error.split_once(':').map_or("DME1500", |(code, _)| code);
        let code = if matches!(code, "DME1500" | "DME1501" | "DME1502" | "DME1301") {
            code
        } else {
            "DME1500"
        };
        reply.failure = Some(DebugFailure {
            code: code.into(),
            message: error.chars().take(256).collect(),
        });
        Ok(reply)
    }
    pub(crate) fn command_json(&mut self, json: &str) -> Result<String, String> {
        let request = MachineDebugRequest::parse(json)?;
        if request.schema_version != 1 || request.revision != self.snapshot().revision {
            return Err(STALE.into());
        }
        match self.command(&request) {
            Ok(reply) => Ok(reply),
            Err(error) => {
                let request = MachineDebugRequest::parse(json)?;
                encode(&self.rejection(&request, &error)?)
            }
        }
    }
    pub(crate) fn session_command(
        &mut self,
        request: MonitorRequest,
    ) -> Result<SessionSnapshot, String> {
        if request.schema_version != 1
            || request.session_id != self.id()
            || matches!(self.state, State::Terminated)
        {
            return Err("DME1004: machine debugger session is stale or stopped".into());
        }
        if let MonitorCommand::Advance { cycles } = request.command {
            if cycles == 0 || cycles > MAX_SLICE {
                return Err("DME1002: invalid execution slice".into());
            }
            if matches!(self.state, State::Ready) {
                return self.session.command(request);
            }
            if matches!(self.state, State::Running) {
                let result = self.poll(cycles);
                if let Err(error) = result {
                    self.state = State::Failed(error);
                    self.bump()?;
                }
            }
            return Ok(self.session.debug_frame());
        }
        let reset = matches!(request.command, MonitorCommand::Reset);
        let stop = matches!(request.command, MonitorCommand::Stop);
        let readonly = matches!(
            request.command,
            MonitorCommand::State | MonitorCommand::ExportRecording
        );
        let frame = self.session.command(request)?;
        if reset {
            self.state = State::Ready;
            self.loaded = false;
            self.memory = None;
        }
        if stop {
            self.state = State::Terminated;
            self.loaded = false;
            self.memory = None;
        }
        if !readonly {
            self.bump()?;
        }
        Ok(frame)
    }
    fn poll(&mut self, cycles: u32) -> Result<(), String> {
        let mut remaining = cycles;
        if self.skip_breakpoint {
            self.skip_breakpoint = false;
            let used = self.session.debug_machine_mut().step()?;
            remaining = remaining.saturating_sub(used);
        }
        if remaining > 0 {
            let mut breakpoints = Breakpoints::default();
            breakpoints.replace(&[self.target.address])?;
            let stop =
                self.session
                    .debug_machine_mut()
                    .run_until(remaining, &breakpoints, || false)?;
            if stop.reason == DebugStopReason::Breakpoint {
                self.state = State::Stopped(true);
                self.bump()?;
            }
        }
        Ok(())
    }
    pub(crate) fn export_recording(&mut self) -> Result<crate::RecordingExport, String> {
        self.session.export_recording()
    }
}
fn bounded_memory_length(address: u16) -> usize {
    let end = if address < 0xff00 { 0xff00 } else { 0x10000 };
    (end - usize::from(address)).min(32)
}
pub(crate) fn encode(reply: &MachineDebugReply) -> Result<String, String> {
    serde_json::to_string(reply).map_err(|_| "DME1500: cannot encode machine inspection".into())
}

#[must_use]
pub fn disassemble_bytes(
    start_address: u16,
    bytes: &[u8],
    instruction_limit: usize,
) -> Vec<DisassembledInstruction> {
    let mut result = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() && result.len() < instruction_limit.min(256) {
        let Some(address) = u16::try_from(usize::from(start_address) + offset).ok() else {
            break;
        };
        let (length, mnemonic, operand) = decode_instruction(address, &bytes[offset..]);
        let length = length.max(1).min(bytes.len() - offset);
        result.push(DisassembledInstruction {
            address,
            bytes: bytes[offset..offset + length].to_vec(),
            mnemonic: mnemonic.into(),
            operand,
            location: None,
        });
        offset += length;
    }
    result
}
fn decode_instruction(address: u16, bytes: &[u8]) -> (usize, &'static str, String) {
    let Some(opcode) = bytes.first().copied() else {
        return (1, ".FCB", String::new());
    };
    let inherent = match opcode {
        0x12 => Some("NOP"),
        0x13 => Some("SYNC"),
        0x19 => Some("DAA"),
        0x1d => Some("SEX"),
        0x39 => Some("RTS"),
        0x3a => Some("ABX"),
        0x3b => Some("RTI"),
        0x3d => Some("MUL"),
        0x3f => Some("SWI"),
        0x4f => Some("CLRA"),
        0x5f => Some("CLRB"),
        _ => None,
    };
    if let Some(mnemonic) = inherent {
        return (1, mnemonic, String::new());
    }
    if (0x20..=0x2f).contains(&opcode) {
        const BRANCHES: [&str; 16] = [
            "BRA", "BRN", "BHI", "BLS", "BCC", "BCS", "BNE", "BEQ", "BVC", "BVS", "BPL", "BMI",
            "BGE", "BLT", "BGT", "BLE",
        ];
        return relative8(address, bytes, BRANCHES[usize::from(opcode - 0x20)]);
    }
    match opcode {
        0x16 => relative16(address, bytes, "LBRA"),
        0x17 => relative16(address, bytes, "LBSR"),
        0x7e => extended(bytes, "JMP"),
        0xbd => extended(bytes, "JSR"),
        0x8d => relative8(address, bytes, "BSR"),
        0x86 => immediate8(bytes, "LDA"),
        0xc6 => immediate8(bytes, "LDB"),
        0x8e => immediate16(bytes, "LDX"),
        0xce => immediate16(bytes, "LDU"),
        0xcc => immediate16(bytes, "LDD"),
        0x96 => direct(bytes, "LDA"),
        0xd6 => direct(bytes, "LDB"),
        0x97 => direct(bytes, "STA"),
        0xd7 => direct(bytes, "STB"),
        0x9d => direct(bytes, "JSR"),
        0xb6 => extended(bytes, "LDA"),
        0xf6 => extended(bytes, "LDB"),
        0xb7 => extended(bytes, "STA"),
        0xf7 => extended(bytes, "STB"),
        _ => (1, ".FCB", format!("${opcode:02X}")),
    }
}

fn immediate8(bytes: &[u8], mnemonic: &'static str) -> (usize, &'static str, String) {
    bytes.get(1).map_or_else(
        || truncated(bytes[0]),
        |value| (2, mnemonic, format!("#${value:02X}")),
    )
}

fn immediate16(bytes: &[u8], mnemonic: &'static str) -> (usize, &'static str, String) {
    word(bytes).map_or_else(
        || truncated(bytes[0]),
        |value| (3, mnemonic, format!("#${value:04X}")),
    )
}

fn direct(bytes: &[u8], mnemonic: &'static str) -> (usize, &'static str, String) {
    bytes.get(1).map_or_else(
        || truncated(bytes[0]),
        |value| (2, mnemonic, format!("<${value:02X}")),
    )
}

fn extended(bytes: &[u8], mnemonic: &'static str) -> (usize, &'static str, String) {
    word(bytes).map_or_else(
        || truncated(bytes[0]),
        |value| (3, mnemonic, format!("${value:04X}")),
    )
}

fn relative8(address: u16, bytes: &[u8], mnemonic: &'static str) -> (usize, &'static str, String) {
    bytes.get(1).map_or_else(
        || truncated(bytes[0]),
        |offset| {
            let target = wrapping_relative(address, 2, i32::from(i8::from_ne_bytes([*offset])));
            (2, mnemonic, format!("${target:04X}"))
        },
    )
}

fn relative16(address: u16, bytes: &[u8], mnemonic: &'static str) -> (usize, &'static str, String) {
    word(bytes).map_or_else(
        || truncated(bytes[0]),
        |offset| {
            let target = wrapping_relative(
                address,
                3,
                i32::from(i16::from_be_bytes(offset.to_be_bytes())),
            );
            (3, mnemonic, format!("${target:04X}"))
        },
    )
}

fn wrapping_relative(address: u16, length: u16, offset: i32) -> u16 {
    let base = i32::from(address.wrapping_add(length));
    u16::try_from((base + offset).rem_euclid(0x1_0000)).unwrap_or(0)
}

fn word(bytes: &[u8]) -> Option<u16> {
    Some(u16::from_be_bytes([*bytes.get(1)?, *bytes.get(2)?]))
}

fn truncated(opcode: u8) -> (usize, &'static str, String) {
    (1, ".FCB", format!("${opcode:02X}"))
}
