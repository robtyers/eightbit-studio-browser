//! Bounded original-machine session shared by native and browser hosts.
//! Source is rebuilt before launch; the guest ROM loads the standard cassette.
use crate::{Model, Result, assemble_original, compile_basic, error, original_target};
use eightbit_source_core::{digest, source_map::SourceMap};
use eightbit_spectrum_emulator::{
    AUDIO_HZ, HEIGHT, Key, MAX_SLICE, MAX_TAP_BYTES, Machine, TapeCapture, WIDTH,
};
#[cfg(test)]
use eightbit_spectrum_emulator::{CPU_HZ, FRAME_T_STATES};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const MAX_REQUEST_BYTES: usize = 5 * 1024 * 1024;
pub const FIRMWARE_ID: &str = "eightbit-spectrum48-basic-0.1-dev";

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Basic,
    Assembly,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceLaunch {
    pub schema_version: u32,
    pub target: String,
    pub kind: Language,
    pub source: String,
    pub source_path: String,
    pub revision: String,
    pub source_sha256: String,
    pub cassette_sha256: String,
    pub breakpoint_line: Option<usize>,
}

#[derive(Deserialize)]
#[serde(
    tag = "operation",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Command {
    Advance {
        cycles: u32,
    },
    State {},
    Reset {},
    Stop {},
    Release {},
    Type {
        text: String,
    },
    Key {
        code: String,
        pressed: bool,
    },
    MountCassette {
        #[serde(deserialize_with = "cassette_bytes")]
        bytes: Vec<u8>,
    },
    MountWav {
        #[serde(deserialize_with = "cassette_bytes")]
        bytes: Vec<u8>,
    },
    StartRecording {},
    FinishRecording {},
    DiscardRecording {},
    ExportRecording {},
    RewindCassette {},
    EjectCassette {},
    Pause {},
    Continue {
        stop_id: u32,
    },
    Step {
        stop_id: u32,
    },
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub schema_version: u32,
    pub session_id: u32,
    pub command: Command,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugState {
    pub state: &'static str,
    pub stop_id: u32,
    pub values_valid: bool,
    pub breakpoint_verified: bool,
    pub source_line: Option<usize>,
    pub revision: String,
    pub registers: Option<BTreeMap<&'static str, u16>>,
    pub breakpoint_address: u16,
    pub memory: Option<DebugMemory>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugMemory {
    pub start_address: u16,
    pub bytes: Vec<u8>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CassetteReceipt {
    pub filename: &'static str,
    pub format: &'static str,
    pub bytes: usize,
    pub sha256: String,
    pub pending: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    pub schema_version: u32,
    pub session_id: u32,
    pub firmware_id: &'static str,
    pub width: usize,
    pub height: usize,
    pub state: &'static str,
    pub message: String,
    pub text: String,
    pub pixels: Vec<u8>,
    pub audio_sample_rate: u32,
    pub frame_t_states: u32,
    pub audio: Vec<i16>,
    pub cassette: Option<CassetteReceipt>,
    pub debug: Option<DebugState>,
    pub recording: RecordingStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recording_export: Option<RecordingExport>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingStatus {
    pub state: &'static str,
    pub samples: u64,
    pub sample_rate: u32,
    pub message: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cassette: Option<CassetteReceipt>,
}
#[derive(Serialize)]
pub struct RecordingExport {
    pub filename: &'static str,
    pub bytes: Vec<u8>,
}
enum CaptureState {
    Idle,
    Active(TapeCapture),
    Ready(Vec<u8>),
    Failed,
}
struct Debugger {
    program: DebugProgram,
    breakpoint: u16,
    stopped: bool,
    stop_id: u32,
    skip_once: bool,
    stepping: bool,
}
enum DebugProgram {
    Assembly {
        code: Vec<u8>,
        map: SourceMap,
    },
    Basic {
        code: Vec<u8>,
        lines: BTreeMap<u16, usize>,
        revision: String,
    },
}
impl Debugger {
    fn revision(&self) -> &str {
        match &self.program {
            DebugProgram::Assembly { map, .. } => &map.revision,
            DebugProgram::Basic { revision, .. } => revision,
        }
    }
    fn source_line(&self, machine: &Machine) -> Option<usize> {
        match &self.program {
            DebugProgram::Assembly { map, .. } => map.line_at(machine.cpu.registers.pc),
            DebugProgram::Basic { lines, .. } => at_statement(machine)
                .then(|| lines.get(&guest_word(machine, 0x5c45)).copied())
                .flatten(),
        }
    }
    fn at_breakpoint(&self, machine: &Machine) -> bool {
        match self.program {
            DebugProgram::Assembly { .. } => machine.cpu.registers.pc == self.breakpoint,
            DebugProgram::Basic { .. } => {
                at_statement(machine)
                    && (self.stepping || guest_word(machine, 0x5c45) == self.breakpoint)
            }
        }
    }
}
pub struct Session {
    id: u32,
    machine: Machine,
    stopped: bool,
    failure: Option<String>,
    input: VecDeque<Vec<Key>>,
    held: Vec<Key>,
    input_remaining: u32,
    manual_keys: BTreeSet<String>,
    debugger: Option<Debugger>,
    cassette: Option<(usize, String)>,
    capture: CaptureState,
    capture_start: u64,
    capture_samples: u64,
}
fn invalid(message: &str) -> eightbit_source_core::Error {
    error("S48RUN1000", message)
}
#[allow(clippy::needless_pass_by_value)] // Result::map_err transfers the owned board error.
fn host(e: eightbit_spectrum_emulator::Error) -> eightbit_source_core::Error {
    error("S48RUN1001", e.to_string())
}

impl Session {
    pub fn new(id: u32) -> Result<Self> {
        Self::for_target(id, "spectrum48")
    }
    pub fn for_target(id: u32, target: &str) -> Result<Self> {
        let model = Model::from_target(target)
            .ok_or_else(|| invalid("Unsupported original Spectrum target"))?;
        if id == 0 {
            return Err(invalid("Invalid Spectrum session identity"));
        }
        let mut session = Self {
            id,
            machine: Machine::development_model(model).map_err(host)?,
            stopped: false,
            failure: None,
            input: VecDeque::new(),
            held: vec![],
            input_remaining: 0,
            manual_keys: BTreeSet::new(),
            debugger: None,
            cassette: None,
            capture: CaptureState::Idle,
            capture_start: 0,
            capture_samples: 0,
        };
        session.boot()?;
        Ok(session)
    }
    fn firmware_id(&self) -> &'static str {
        match self.machine.board.model() {
            Model::Spectrum48 => FIRMWARE_ID,
            Model::Spectrum128 => "eightbit-spectrum128-basic-0.1-dev",
            Model::Plus2 => "eightbit-spectrum-plus2-basic-0.1-dev",
        }
    }
    fn boot(&mut self) -> Result<()> {
        for _ in 0..8 {
            self.machine.run(MAX_SLICE).map_err(host)?;
            self.machine.board.drain_audio();
        }
        Ok(())
    }
    pub fn from_source(id: u32, launch: &SourceLaunch) -> Result<Self> {
        if launch.schema_version != 1
            || original_target(&launch.target).is_none()
            || launch.source.len() > eightbit_source_core::MAX_SOURCE
            || digest(launch.source.as_bytes()) != launch.source_sha256
            || launch.revision.len() != 64
            || !launch.revision.bytes().all(|b| b.is_ascii_hexdigit())
            || !launch.source_path.starts_with("src/")
            || launch.source_path.len() > 240
            || launch.source_path.contains('\\')
            || launch
                .source_path
                .split('/')
                .any(|p| p.is_empty() || p == "." || p == "..")
            || !launch
                .source_path
                .ends_with(if launch.kind == Language::Basic {
                    ".bas"
                } else {
                    ".asm"
                })
        {
            return Err(invalid(
                "Source changed or original Spectrum launch is invalid; build again",
            ));
        }
        let target = original_target(&launch.target)
            .ok_or_else(|| invalid("Unsupported original Spectrum target"))?;
        let (media, debug) = if launch.kind == Language::Assembly {
            let built = assemble_original(
                target,
                &launch.source,
                &launch.source_path,
                &launch.revision,
            )?;
            let debug = if let Some(line) = launch.breakpoint_line {
                Some(Debugger {
                    breakpoint: built.map.breakpoint(line)?,
                    program: DebugProgram::Assembly {
                        code: built.code,
                        map: built.map,
                    },
                    stopped: false,
                    stop_id: 0,
                    skip_once: false,
                    stepping: false,
                })
            } else {
                None
            };
            (built.media, debug)
        } else {
            let media = compile_basic(target, &launch.source)?;
            let debug = if let Some(selected) = launch.breakpoint_line {
                let lines: BTreeMap<_, _> = launch
                    .source
                    .lines()
                    .enumerate()
                    .filter_map(|(i, line)| {
                        line.trim()
                            .split_once(char::is_whitespace)
                            .and_then(|(n, _)| n.parse::<u16>().ok())
                            .map(|n| (n, i + 1))
                    })
                    .collect();
                let breakpoint = lines
                    .iter()
                    .find(|(_, line)| **line == selected)
                    .map(|(number, _)| *number)
                    .ok_or_else(|| invalid("Choose a numbered BASIC source line"))?;
                let code = media
                    .get(24..media.len().saturating_sub(1))
                    .ok_or_else(|| invalid("Invalid generated BASIC cassette"))?
                    .to_vec();
                Some(Debugger {
                    program: DebugProgram::Basic {
                        code,
                        lines,
                        revision: launch.revision.clone(),
                    },
                    breakpoint,
                    stopped: false,
                    stop_id: 0,
                    skip_once: false,
                    stepping: false,
                })
            } else {
                None
            };
            (media, debug)
        };
        if digest(&media) != launch.cassette_sha256 {
            return Err(invalid("Cassette changed; build again"));
        }
        let mut session = Self::for_target(id, &launch.target)?;
        session.machine.board.mount_tap(&media).map_err(host)?;
        session.cassette = Some((media.len(), digest(&media)));
        if let Some(tape) = session.machine.board.tape_mut() {
            tape.play();
        }
        session.type_text("LOAD \"\"\n")?;
        session.debugger = debug;
        Ok(session)
    }
    pub fn from_source_json(id: u32, request: &str) -> Result<Self> {
        if request.len() > 512 * 1024 {
            return Err(invalid("Spectrum source request is too large"));
        }
        let request: SourceLaunch = serde_json::from_str(request)
            .map_err(|_| invalid("Invalid original Spectrum source request"))?;
        Self::from_source(id, &request)
    }
    pub fn command_json(&mut self, request: &str) -> Result<String> {
        if request.len() > MAX_REQUEST_BYTES {
            return Err(invalid("Spectrum request is too large"));
        }
        let request =
            serde_json::from_str(request).map_err(|_| invalid("Invalid Spectrum command"))?;
        let frame = self.command(request)?;
        serde_json::to_string(&frame).map_err(|_| invalid("Cannot encode Spectrum frame"))
    }
    pub fn command(&mut self, request: Request) -> Result<Frame> {
        if request.schema_version != 1 || request.session_id != self.id || self.stopped {
            return Err(invalid("Spectrum session is stale or stopped"));
        }
        match request.command {
            Command::Advance { cycles } => {
                if cycles == 0 || cycles > MAX_SLICE {
                    return Err(invalid("Invalid Spectrum execution slice"));
                }
                if self.failure.is_none()
                    && !self.debugger.as_ref().is_some_and(|d| d.stopped)
                    && let Err(e) = self.advance(cycles)
                {
                    self.failure = Some(e.to_string());
                    self.release();
                }
            }
            Command::State {} => {}
            Command::Stop {} => {
                self.discard_recording();
                self.release();
                self.machine.board.eject_tape();
                self.cassette = None;
                self.machine.board.drain_audio();
                self.debugger = None;
                self.stopped = true;
            }
            Command::Reset {} => {
                self.discard_recording();
                self.release();
                self.debugger = None;
                self.failure = None;
                self.machine.reset();
                self.machine.board.rewind_cassette();
                self.boot()?;
            }
            Command::Release {} => {
                // Canvas/window blur releases physical keys. Submitted text is
                // an owned operation and keeps its timed pulse until completion.
                self.manual_keys.clear();
                self.machine.board.release_keys();
                for key in &self.held {
                    self.machine.board.key(*key, true);
                }
            }
            Command::Type { mut text } => {
                if text.len() > 4095 {
                    return Err(invalid("Spectrum command exceeds 4095 bytes"));
                }
                if !text.ends_with(['\n', '\r']) {
                    text.push('\n');
                }
                self.type_text(&text)?;
            }
            Command::Key { code, pressed } => self.key(code, pressed)?,
            Command::MountCassette { bytes } => {
                self.machine.board.mount_tap(&bytes).map_err(host)?;
                self.cassette = Some((bytes.len(), digest(&bytes)));
                if let Some(tape) = self.machine.board.tape_mut() {
                    tape.play();
                }
            }
            Command::MountWav { bytes } => {
                self.machine.board.mount_wav(&bytes).map_err(host)?;
                self.cassette = Some((bytes.len(), digest(&bytes)));
                self.machine.board.play_cassette();
            }
            Command::RewindCassette {} => {
                self.machine.board.rewind_cassette();
                self.machine.board.play_cassette();
            }
            Command::EjectCassette {} => {
                self.machine.board.eject_tape();
                self.cassette = None;
            }
            Command::Pause {} => {
                let debug = self
                    .debugger
                    .as_mut()
                    .ok_or_else(|| invalid("This session is not debugging assembly"))?;
                if !debug.stopped {
                    debug.stop_id = debug
                        .stop_id
                        .checked_add(1)
                        .ok_or_else(|| invalid("Debug stop limit reached"))?;
                    debug.stopped = true;
                }
            }
            Command::Continue { stop_id } => self.resume(stop_id, false)?,
            Command::Step { stop_id } => self.resume(stop_id, true)?,
            Command::StartRecording {} => {
                if !matches!(self.capture, CaptureState::Idle) {
                    return Err(invalid("Save or discard the previous recording first"));
                }
                self.capture_start = self.machine.board.clocks();
                self.capture_samples = 0;
                self.capture = CaptureState::Active(TapeCapture::for_model(
                    self.capture_start,
                    self.machine.board.model(),
                ));
                self.machine.board.record_tape(true);
            }
            Command::FinishRecording {} => {
                if !matches!(self.capture, CaptureState::Active(_)) {
                    return Err(invalid("No active cassette recording"));
                }
                self.collect_recording();
                self.machine.board.record_tape(false);
                if let CaptureState::Active(capture) = &self.capture {
                    self.capture = match capture.finish() {
                        Ok(bytes) => CaptureState::Ready(bytes),
                        Err(_) => CaptureState::Failed,
                    };
                }
            }
            Command::DiscardRecording {} => self.discard_recording(),
            Command::ExportRecording {} => {
                let bytes = self.recording_bytes()?.to_vec();
                let mut frame = self.frame();
                frame.recording_export = Some(RecordingExport {
                    filename: "spectrum-recording.tap",
                    bytes,
                });
                return Ok(frame);
            }
        }
        self.collect_recording();
        Ok(self.frame())
    }
    pub fn recording_bytes(&self) -> Result<&[u8]> {
        if let CaptureState::Ready(bytes) = &self.capture {
            Ok(bytes)
        } else {
            Err(invalid("Finish a valid cassette recording before saving"))
        }
    }
    fn discard_recording(&mut self) {
        self.machine.board.record_tape(false);
        self.capture = CaptureState::Idle;
        self.capture_samples = 0;
    }
    fn collect_recording(&mut self) {
        if let CaptureState::Active(capture) = &mut self.capture {
            let now = self.machine.board.clocks();
            self.capture_samples = now.saturating_sub(self.capture_start) * 9600
                / u64::from(self.machine.board.model().cpu_hz());
            let result = self
                .machine
                .board
                .drain_tape_edges()
                .and_then(|edges| capture.append(&edges, now));
            if result.is_err() {
                self.capture = CaptureState::Failed;
                self.machine.board.record_tape(false);
            }
        }
    }
    fn resume(&mut self, stop_id: u32, step: bool) -> Result<()> {
        let debug = self
            .debugger
            .as_mut()
            .ok_or_else(|| invalid("This session is not debugging assembly"))?;
        if !debug.stopped || debug.stop_id != stop_id {
            return Err(invalid("Debug stop is stale"));
        }
        debug.stopped = false;
        debug.skip_once = true;
        if step {
            if matches!(debug.program, DebugProgram::Basic { .. }) {
                debug.stepping = true;
                return self.advance(MAX_SLICE);
            }
            self.machine.step().map_err(host)?;
            debug.stop_id = debug
                .stop_id
                .checked_add(1)
                .ok_or_else(|| invalid("Debug stop limit reached"))?;
            debug.stopped = true;
            debug.skip_once = false;
        }
        self.machine.board.drain_audio();
        Ok(())
    }
    fn advance(&mut self, budget: u32) -> Result<()> {
        let mut elapsed = 0;
        while elapsed < budget {
            if let Some(debug) = self.debugger.as_mut() {
                if !debug.skip_once
                    && debug.at_breakpoint(&self.machine)
                    && code_matches(&self.machine, debug)
                {
                    debug.stopped = true;
                    debug.stepping = false;
                    debug.stop_id = debug
                        .stop_id
                        .checked_add(1)
                        .ok_or_else(|| invalid("Debug stop limit reached"))?;
                    self.machine.board.drain_audio();
                    break;
                }
                debug.skip_once = false;
            }
            if self.input_remaining == 0 {
                self.input_event();
            }
            let clocks = self.machine.step().map_err(host)?;
            elapsed += clocks;
            self.input_remaining = self.input_remaining.saturating_sub(clocks);
        }
        Ok(())
    }
    fn input_event(&mut self) {
        if !self.held.is_empty() {
            for key in self.held.drain(..) {
                self.machine.board.key(key, false);
            }
            if self.input.is_empty() {
                self.machine.board.set_literal_typing(false);
            }
            self.input_remaining = self.machine.board.model().frame_t_states() * 2;
        } else if !self.input.is_empty() {
            if !self.machine.board.editor_ready() {
                self.input_remaining = self.machine.board.model().frame_t_states() / 4;
                return;
            }
            let Some(chord) = self.input.pop_front() else {
                return;
            };
            for key in &chord {
                self.machine.board.key(*key, true);
            }
            self.held = chord;
            self.input_remaining = self.machine.board.model().frame_t_states() * 2;
        } else {
            self.machine.board.set_literal_typing(false);
        }
    }
    fn type_text(&mut self, text: &str) -> Result<()> {
        if text.len() > 4096 || self.input.len() + text.len() > 4096 || !self.manual_keys.is_empty()
        {
            return Err(invalid("Spectrum input queue is full or keys are held"));
        }
        let chords = text.bytes().map(chord).collect::<Result<Vec<_>>>()?;
        self.input.extend(chords);
        self.machine.board.set_literal_typing(true);
        Ok(())
    }
    fn release(&mut self) {
        self.input.clear();
        self.held.clear();
        self.manual_keys.clear();
        self.input_remaining = 0;
        self.machine.board.release_keys();
        self.machine.board.set_literal_typing(false);
    }
    fn key(&mut self, code: String, pressed: bool) -> Result<()> {
        if !self.input.is_empty() || !self.held.is_empty() {
            return Err(invalid("Wait for queued Spectrum text to finish"));
        }
        key_code(&code)?;
        if pressed {
            if self.manual_keys.len() >= 64 {
                return Err(invalid("Too many held Spectrum keys"));
            }
            self.manual_keys.insert(code);
        } else {
            self.manual_keys.remove(&code);
        }
        self.machine.board.release_keys();
        for code in &self.manual_keys {
            for key in key_code(code)? {
                self.machine.board.key(key, true);
            }
        }
        Ok(())
    }
    /// Inspection never consumes audio or advances the guest.
    #[must_use]
    pub fn debug_state(&self) -> Option<DebugState> {
        self.debugger.as_ref().map(|d| {
            let r = self.machine.cpu.registers;
            let valid = d.stopped && self.failure.is_none();
            let mapped = valid && code_matches(&self.machine, d);
            DebugState {
                state: if d.stopped { "stopped" } else { "running" },
                stop_id: d.stop_id,
                values_valid: valid,
                breakpoint_verified: mapped,
                source_line: if mapped {
                    d.source_line(&self.machine)
                } else {
                    None
                },
                revision: d.revision().into(),
                registers: valid.then(|| {
                    [
                        ("af", r.af()),
                        ("bc", r.bc()),
                        ("de", r.de()),
                        ("hl", r.hl()),
                        ("af'", u16::from_be_bytes([r.alternate[0], r.alternate[1]])),
                        ("bc'", u16::from_be_bytes([r.alternate[2], r.alternate[3]])),
                        ("de'", u16::from_be_bytes([r.alternate[4], r.alternate[5]])),
                        ("hl'", u16::from_be_bytes([r.alternate[6], r.alternate[7]])),
                        ("ix", r.ix),
                        ("iy", r.iy),
                        ("sp", r.sp),
                        ("pc", r.pc),
                    ]
                    .into()
                }),
                breakpoint_address: match d.program {
                    DebugProgram::Assembly { .. } => d.breakpoint,
                    DebugProgram::Basic { .. } => eightbit_spectrum_emulator::STATEMENT_ENTRY,
                },
                memory: valid.then(|| {
                    let start_address = r.pc.min(65472);
                    DebugMemory {
                        start_address,
                        bytes: (0..64)
                            .map(|offset| self.machine.board.peek(start_address + offset))
                            .collect(),
                    }
                }),
            }
        })
    }
    pub fn frame(&mut self) -> Frame {
        let debug = self.debug_state();
        let state = if self.stopped {
            "stopped"
        } else if self.failure.is_some() {
            "failed"
        } else {
            "running"
        };
        let message = self.failure.clone().unwrap_or_else(|| {
            if self.stopped {
                "Spectrum stopped"
            } else if debug.as_ref().is_some_and(|d| d.state == "stopped") {
                "Spectrum paused"
            } else {
                match self.machine.board.model() {
                    Model::Spectrum48 => "Original Spectrum 48K development runtime",
                    Model::Spectrum128 => "Original Spectrum 128K development runtime",
                    Model::Plus2 => "Original Spectrum +2 development runtime",
                }
            }
            .into()
        });
        Frame {
            schema_version: 1,
            session_id: self.id,
            firmware_id: self.firmware_id(),
            width: WIDTH,
            height: HEIGHT,
            state,
            message,
            text: screen_text(&self.machine),
            pixels: self.machine.board.frame().to_vec(),
            audio_sample_rate: AUDIO_HZ,
            frame_t_states: self.machine.board.model().frame_t_states(),
            audio: self.machine.board.drain_audio(),
            cassette: self
                .cassette
                .as_ref()
                .map(|(bytes, sha256)| CassetteReceipt {
                    filename: "CASSETTE",
                    format: "cassette-stream",
                    bytes: *bytes,
                    sha256: sha256.clone(),
                    pending: self.machine.board.cassette_pending(),
                }),
            debug,
            recording: RecordingStatus {
                state: match self.capture {
                    CaptureState::Idle => "idle",
                    CaptureState::Active(_) => "recording",
                    CaptureState::Ready(_) => "ready",
                    CaptureState::Failed => "failed",
                },
                samples: self.capture_samples,
                sample_rate: 9600,
                message: match self.capture {
                    CaptureState::Idle => "Record the guest MIC output while SAVE runs.",
                    CaptureState::Active(_) => "Recording cassette output…",
                    CaptureState::Ready(_) => "Checked standard-speed TAP is ready to save.",
                    CaptureState::Failed => {
                        "Incomplete, unsupported or oversized signal. Discard and record again."
                    }
                },
                cassette: if let CaptureState::Ready(bytes) = &self.capture {
                    Some(CassetteReceipt {
                        filename: "CASSETTE",
                        format: "cassette-stream",
                        bytes: bytes.len(),
                        sha256: digest(bytes),
                        pending: false,
                    })
                } else {
                    None
                },
            },
            recording_export: None,
        }
    }
}
fn code_matches(machine: &Machine, debug: &Debugger) -> bool {
    let (code, origin) = match &debug.program {
        DebugProgram::Assembly { code, map } => (code, map.origin),
        DebugProgram::Basic { code, .. } => (code, guest_word(machine, 0x5c53)),
    };
    origin >= 0x4000
        && code.iter().enumerate().all(|(i, byte)| {
            u16::try_from(usize::from(origin) + i)
                .is_ok_and(|address| machine.board.peek(address) == *byte)
        })
}
fn guest_word(machine: &Machine, address: u16) -> u16 {
    u16::from_le_bytes([machine.board.peek(address), machine.board.peek(address + 1)])
}
fn at_statement(machine: &Machine) -> bool {
    machine.board.firmware_page() == 0
        && machine.cpu.registers.pc == eightbit_spectrum_emulator::STATEMENT_ENTRY
        && (1..=9999).contains(&guest_word(machine, 0x5c45))
}
fn chord(byte: u8) -> Result<Vec<Key>> {
    let (modifier, key): (Option<&str>, String) = match byte {
        b'a'..=b'z' | b'0'..=b'9' => (None, char::from(byte).to_string()),
        b'A'..=b'Z' => (
            Some("caps-shift"),
            char::from(byte.to_ascii_lowercase()).to_string(),
        ),
        b' ' => (None, "space".into()),
        b'\n' | b'\r' => (None, "enter".into()),
        8 | 127 => (Some("caps-shift"), "0".into()),
        _ => {
            let symbols = [
                (b'!', "1"),
                (b'@', "2"),
                (b'#', "3"),
                (b'$', "4"),
                (b'%', "5"),
                (b'&', "6"),
                (b'\'', "7"),
                (b'(', "8"),
                (b')', "9"),
                (b'_', "0"),
                (b'"', "p"),
                (b';', "o"),
                (b'=', "l"),
                (b'+', "k"),
                (b'-', "j"),
                (b'^', "h"),
                (b'.', "m"),
                (b',', "n"),
                (b'*', "b"),
                (b':', "z"),
                (b'?', "c"),
                (b'/', "v"),
                (b'<', "r"),
                (b'>', "t"),
                (b'|', "s"),
                (b'\\', "d"),
                (b'{', "f"),
                (b'}', "g"),
                (b'~', "a"),
                (b'`', "x"),
            ];
            let key = symbols
                .iter()
                .find(|(symbol, _)| *symbol == byte)
                .ok_or_else(|| invalid("Unsupported Spectrum text character"))?
                .1;
            (Some("symbol-shift"), key.into())
        }
    };
    let mut result = vec![Key::named(&key).map_err(host)?];
    if let Some(modifier) = modifier {
        result.push(Key::named(modifier).map_err(host)?);
    }
    Ok(result)
}
fn key_code(code: &str) -> Result<Vec<Key>> {
    if code.len() > 32 {
        return Err(invalid("Unsupported Spectrum keyboard key"));
    }
    if let Some(letter) = code
        .strip_prefix("Key")
        .filter(|s| s.len() == 1 && s.bytes().all(|b| b.is_ascii_uppercase()))
    {
        return chord(letter.as_bytes()[0].to_ascii_lowercase());
    }
    if let Some(digit) = code
        .strip_prefix("Digit")
        .filter(|s| s.len() == 1 && s.bytes().all(|b| b.is_ascii_digit()))
    {
        return chord(digit.as_bytes()[0]);
    }
    match code {
        "Enter" => chord(13),
        "Space" => chord(b' '),
        "Backspace" | "Delete" => chord(8),
        "ArrowLeft" => Ok(vec![
            Key::named("caps-shift").map_err(host)?,
            Key::named("5").map_err(host)?,
        ]),
        "ArrowRight" => Ok(vec![
            Key::named("caps-shift").map_err(host)?,
            Key::named("8").map_err(host)?,
        ]),
        "ArrowUp" => Ok(vec![
            Key::named("caps-shift").map_err(host)?,
            Key::named("7").map_err(host)?,
        ]),
        "ArrowDown" => Ok(vec![
            Key::named("caps-shift").map_err(host)?,
            Key::named("6").map_err(host)?,
        ]),
        "ShiftLeft" | "ShiftRight" => Ok(vec![Key::named("caps-shift").map_err(host)?]),
        "ControlLeft" | "ControlRight" | "AltRight" => {
            Ok(vec![Key::named("symbol-shift").map_err(host)?])
        }
        _ => Key::named(code).map(|key| vec![key]).map_err(host),
    }
}
fn screen_text(machine: &Machine) -> String {
    let chars = u16::from_le_bytes([machine.board.peek(0x5c36), machine.board.peek(0x5c37)]);
    let glyphs: BTreeMap<[u8; 8], char> = (32_u8..127)
        .map(|code| {
            (
                std::array::from_fn(|row| {
                    machine.board.peek(
                        chars
                            .wrapping_add(u16::from(code) * 8)
                            .wrapping_add(u16::try_from(row).unwrap_or(0)),
                    )
                }),
                char::from(code),
            )
        })
        .collect();
    let mut result = String::with_capacity(768);
    for y in 0..24_u16 {
        for x in 0..32_u16 {
            let glyph = std::array::from_fn(|row| {
                machine.board.peek(
                    0x4000
                        | ((y & 24) << 8)
                        | (u16::try_from(row).unwrap_or(0) << 8)
                        | ((y & 7) << 5)
                        | x,
                )
            });
            result.push(glyphs.get(&glyph).copied().unwrap_or(' '));
        }
    }
    result
}
fn cassette_bytes<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Vec<u8>, D::Error> {
    struct Bytes;
    impl<'de> serde::de::Visitor<'de> for Bytes {
        type Value = Vec<u8>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a bounded TAP byte array")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> std::result::Result<Vec<u8>, A::Error> {
            let mut bytes = Vec::new();
            while let Some(byte) = seq.next_element::<u8>()? {
                if bytes.len() == MAX_TAP_BYTES {
                    return Err(serde::de::Error::custom("TAP exceeds 1 MiB"));
                }
                bytes.push(byte);
            }
            Ok(bytes)
        }
    }
    deserializer.deserialize_seq(Bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TARGETS;
    use eightbit_z80_core::Bus;
    fn command(session: &mut Session, command: Command) -> Result<Frame> {
        session.command(Request {
            schema_version: 1,
            session_id: session.id,
            command,
        })
    }
    fn launch(source: &str, kind: Language, breakpoint_line: Option<usize>) -> SourceLaunch {
        launch_for("spectrum48", source, kind, breakpoint_line)
    }
    fn launch_for(
        target: &str,
        source: &str,
        kind: Language,
        breakpoint_line: Option<usize>,
    ) -> SourceLaunch {
        let profile = original_target(target).unwrap();
        let source_path = if kind == Language::Basic {
            "src/basic/main.bas"
        } else {
            "src/asm/main.asm"
        };
        let media = if kind == Language::Basic {
            compile_basic(profile, source).unwrap()
        } else {
            assemble_original(profile, source, source_path, &"a".repeat(64))
                .unwrap()
                .media
        };
        SourceLaunch {
            schema_version: 1,
            target: target.into(),
            kind,
            source: source.into(),
            source_path: source_path.into(),
            revision: "a".repeat(64),
            source_sha256: digest(source.as_bytes()),
            cassette_sha256: digest(&media),
            breakpoint_line,
        }
    }
    #[test]
    fn banked_targets_load_music_disc_files_and_debug_source_through_the_shared_session() {
        for target in ["spectrum128", "spectrum-plus2"] {
            let source = "10 POKE 49152,42:SAVE! \"zeta\" CODE 49152,1\n20 POKE 49152,43:SAVE! \"alpha\" CODE 49152,1:CAT!\n30 LOAD! \"zeta\" CODE:PLAY \"T240O5N1C\":PRINT \"BANKED READY\"\n40 STOP\n";
            let request = launch_for(target, source, Language::Basic, Some(4));
            let mut session = Session::from_source(41, &request).unwrap();
            let mut frame = command(&mut session, Command::State {}).unwrap();
            for _ in 0..800 {
                frame = command(&mut session, Command::Advance { cycles: MAX_SLICE }).unwrap();
                if frame.debug.as_ref().is_some_and(|d| d.values_valid) {
                    break;
                }
            }
            let stopped = frame.debug.as_ref().expect("mapped BASIC stop");
            assert_eq!(stopped.source_line, Some(4));
            assert!(stopped.breakpoint_verified);
            assert!(frame.text.contains("BANKED READY"), "{}", frame.text);
            assert!(frame.text.find("alpha").unwrap() < frame.text.find("zeta").unwrap());
            assert_eq!(session.machine.board.peek(0xc000), 42);
            assert_eq!(
                session.machine.board.model(),
                Model::from_target(target).unwrap()
            );
            assert_ne!(frame.firmware_id, FIRMWARE_ID);
            command(
                &mut session,
                Command::Step {
                    stop_id: stopped.stop_id,
                },
            )
            .unwrap();
            command(&mut session, Command::Stop {}).unwrap();
            assert!(command(&mut session, Command::State {}).is_err());
        }
    }
    #[test]
    fn basic_breakpoints_and_steps_follow_only_the_loaded_numbered_programme() {
        let source = "\n10 LET A=1\n20 POKE 49152,A\n30 LET A=A+1:IF A<3 THEN GO TO 20\n40 STOP\n";
        let mut session =
            Session::from_source(8, &launch(source, Language::Basic, Some(3))).unwrap();
        let mut stopped = None;
        for _ in 0..800 {
            let frame = command(&mut session, Command::Advance { cycles: MAX_SLICE }).unwrap();
            if frame.debug.as_ref().is_some_and(|d| d.values_valid) {
                stopped = frame.debug;
                break;
            }
        }
        let first = stopped.expect("BASIC must stop before its mapped statement");
        assert!(first.breakpoint_verified);
        assert_eq!(first.source_line, Some(3));
        assert_eq!(session.machine.board.peek(0xc000), 0);
        let next = command(
            &mut session,
            Command::Step {
                stop_id: first.stop_id,
            },
        )
        .unwrap()
        .debug
        .unwrap();
        assert!(next.values_valid && next.breakpoint_verified);
        assert_eq!(next.source_line, Some(4));
        assert_eq!(session.machine.board.peek(0xc000), 1);
        assert!(
            command(
                &mut session,
                Command::Continue {
                    stop_id: first.stop_id
                }
            )
            .is_err()
        );
        command(
            &mut session,
            Command::Continue {
                stop_id: next.stop_id,
            },
        )
        .unwrap();
        for _ in 0..100 {
            let frame = command(&mut session, Command::Advance { cycles: MAX_SLICE }).unwrap();
            if frame.debug.as_ref().is_some_and(|d| d.values_valid) {
                break;
            }
        }
        let repeated = session.debug_state().unwrap();
        assert!(repeated.values_valid);
        assert_eq!(repeated.source_line, Some(3));
        let origin = guest_word(&session.machine, 0x5c53);
        session.machine.board.write(origin + 4, 0);
        let stale = session.debug_state().unwrap();
        assert!(!stale.breakpoint_verified);
        assert_eq!(stale.source_line, None);
        assert!(Session::from_source(9, &launch(source, Language::Basic, Some(1))).is_err());
    }
    #[test]
    fn guest_save_recording_exports_checked_tap_and_failed_capture_recovers() {
        let source = "10 POKE 49152,42\n20 PAUSE 0\n30 SAVE \"CAP\" CODE 49152,1\n40 STOP\n";
        for target in ["spectrum48", "spectrum128", "spectrum-plus2"] {
            let mut session =
                Session::from_source(11, &launch_for(target, source, Language::Basic, None))
                    .unwrap();
            for _ in 0..800 {
                command(&mut session, Command::Advance { cycles: MAX_SLICE }).unwrap();
                if session.machine.board.peek(0xc000) == 42 {
                    break;
                }
            }
            assert_eq!(session.machine.board.peek(0xc000), 42);
            command(&mut session, Command::StartRecording {}).unwrap();
            assert!(command(&mut session, Command::ExportRecording {}).is_err());
            command(
                &mut session,
                Command::Key {
                    code: "Space".into(),
                    pressed: true,
                },
            )
            .unwrap();
            command(
                &mut session,
                Command::Advance {
                    cycles: Model::from_target(target).unwrap().frame_t_states() * 2,
                },
            )
            .unwrap();
            command(
                &mut session,
                Command::Key {
                    code: "Space".into(),
                    pressed: false,
                },
            )
            .unwrap();
            let mut finished = false;
            for _ in 0..800 {
                let frame = command(&mut session, Command::Advance { cycles: MAX_SLICE }).unwrap();
                if frame.text.contains("STOP statement") {
                    finished = true;
                    break;
                }
            }
            assert!(finished);
            let frame = command(&mut session, Command::FinishRecording {}).unwrap();
            assert_eq!(frame.recording.state, "ready");
            let bytes = session.recording_bytes().unwrap();
            assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 19);
            assert_eq!(bytes[3], 3);
            assert_eq!(&bytes[4..7], b"CAP");
            assert_eq!(&bytes[21..], [3, 0, 255, 42, 213]);
            let frame = command(&mut session, Command::ExportRecording {}).unwrap();
            assert_eq!(
                frame.recording_export.unwrap().filename,
                "spectrum-recording.tap"
            );
            assert!(command(&mut session, Command::StartRecording {}).is_err());
            command(&mut session, Command::DiscardRecording {}).unwrap();
            command(&mut session, Command::StartRecording {}).unwrap();
            let frame = command(&mut session, Command::FinishRecording {}).unwrap();
            assert_eq!(frame.recording.state, "failed");
            assert!(session.recording_bytes().is_err());
            command(&mut session, Command::Reset {}).unwrap();
            assert_eq!(session.frame().recording.state, "idle");
        }
    }
    #[test]
    fn continue_retains_nested_subroutine_returns_across_stop_and_expression_errors() {
        for (source, correction) in [
            (
                "10 GO SUB 100:POKE 49152,42:STOP\n100 GO SUB 200:RETURN\n200 STOP:RETURN\n",
                "CONTINUE",
            ),
            (
                "10 LET B=0:GO SUB 100:POKE 49152,42:STOP\n100 GO SUB 200:RETURN\n200 LET A=1/B:RETURN\n",
                "LET B=2:CONTINUE",
            ),
        ] {
            let mut session =
                Session::from_source(13, &launch(source, Language::Basic, None)).unwrap();
            let mut stopped = false;
            for _ in 0..800 {
                command(&mut session, Command::Advance { cycles: MAX_SLICE }).unwrap();
                if guest_word(&session.machine, 0x5c6e) == 200 {
                    stopped = true;
                    break;
                }
            }
            assert!(stopped);
            command(
                &mut session,
                Command::Type {
                    text: correction.into(),
                },
            )
            .unwrap();
            for _ in 0..200 {
                command(&mut session, Command::Advance { cycles: MAX_SLICE }).unwrap();
                if session.machine.board.peek(0xc000) == 42 {
                    break;
                }
            }
            assert_eq!(
                session.machine.board.peek(0xc000),
                42,
                "{source}: {}",
                session.frame().text
            );
        }
    }
    #[test]
    fn guest_edit_recalls_native_lines_and_moves_across_missing_line_numbers() {
        fn advance(session: &mut Session, slices: usize) {
            for _ in 0..slices {
                command(session, Command::Advance { cycles: MAX_SLICE }).unwrap();
                if session.input.is_empty()
                    && session.held.is_empty()
                    && !session.machine.board.cassette_pending()
                    && session.machine.cpu.halted
                    && session.machine.board.firmware_page() == 0
                {
                    return;
                }
            }
            panic!("guest editor did not settle: {}", session.frame().text);
        }
        fn chord(session: &mut Session, keys: &[&str]) {
            for key in keys {
                command(
                    session,
                    Command::Key {
                        code: (*key).into(),
                        pressed: true,
                    },
                )
                .unwrap();
            }
            command(
                session,
                Command::Advance {
                    cycles: FRAME_T_STATES * 2,
                },
            )
            .unwrap();
            command(session, Command::Release {}).unwrap();
            command(
                session,
                Command::Advance {
                    cycles: FRAME_T_STATES * 2,
                },
            )
            .unwrap();
        }
        fn edit_line(session: &Session) -> Vec<u8> {
            let start = guest_word(&session.machine, 0x5c59);
            (0..4096)
                .map(|i| session.machine.board.peek(start + i))
                .take_while(|byte| *byte != 13)
                .collect()
        }
        let source = "10 LET A=1.25\n20 REM edit\n30 STOP\n";
        let mut session = Session::from_source(17, &launch(source, Language::Basic, None)).unwrap();
        advance(&mut session, 800);
        command(
            &mut session,
            Command::Type {
                text: "LIST 15".into(),
            },
        )
        .unwrap();
        advance(&mut session, 100);
        chord(&mut session, &["caps-shift", "1"]);
        assert_eq!(guest_word(&session.machine, 0x5c49), 20);
        assert_eq!(
            edit_line(&session),
            [b"20 ".as_slice(), &[234], b" edit"].concat()
        );
        chord(&mut session, &["caps-shift", "7"]);
        assert_eq!(guest_word(&session.machine, 0x5c49), 10);
        assert_eq!(guest_word(&session.machine, 0x5c6c), 10);
        for scan in 0..8_u16 {
            assert_eq!(
                session.machine.board.peek(0x4002 + scan * 256),
                session
                    .machine
                    .board
                    .peek(0x3c00 + u16::from(b'>') * 8 + scan)
            );
        }
        chord(&mut session, &["caps-shift", "1"]);
        assert_eq!(
            edit_line(&session),
            [b"10 ".as_slice(), &[241], b" A=1.25"].concat()
        );
        assert!(!edit_line(&session).contains(&14));
        chord(&mut session, &["enter"]);
        advance(&mut session, 10);
        assert_eq!(guest_word(&session.machine, 0x5c49), 10);
        chord(&mut session, &["caps-shift", "6"]);
        assert_eq!(guest_word(&session.machine, 0x5c49), 20);
        command(
            &mut session,
            Command::Type {
                text: "LIST 9999".into(),
            },
        )
        .unwrap();
        advance(&mut session, 100);
        chord(&mut session, &["caps-shift", "1"]);
        assert_eq!(guest_word(&session.machine, 0x5c49), 30);
        assert_eq!(edit_line(&session), [b"30 ".as_slice(), &[226]].concat());
        chord(&mut session, &["enter"]);
        advance(&mut session, 10);
        command(&mut session, Command::Type { text: "RUN".into() }).unwrap();
        advance(&mut session, 100);
        assert!(session.frame().text.contains("STOP statement,30:1"));
        assert_eq!(guest_word(&session.machine, 0x5c6e), 30);
    }
    #[test]
    fn syntax_rejection_preserves_programme_and_defers_every_statement_until_corrected() {
        fn idle(session: &mut Session) {
            for _ in 0..800 {
                command(session, Command::Advance { cycles: MAX_SLICE }).unwrap();
                if session.input.is_empty()
                    && session.held.is_empty()
                    && !session.machine.board.cassette_pending()
                    && session.machine.cpu.halted
                    && session.machine.board.firmware_page() == 0
                {
                    return;
                }
            }
            panic!("editor did not settle: {}", session.frame().text);
        }
        fn enter(session: &mut Session, text: &str) {
            command(session, Command::Type { text: text.into() }).unwrap();
            idle(session);
        }
        let mut session =
            Session::from_source(18, &launch("10 STOP\n", Language::Basic, None)).unwrap();
        idle(&mut session);
        let start = guest_word(&session.machine, 0x5c53);
        let end = guest_word(&session.machine, 0x5c4b);
        let old_programme: Vec<_> = (start..end)
            .map(|a| session.machine.board.peek(a))
            .collect();
        enter(&mut session, "POKE 49152,42:LET A=");
        assert_eq!(session.machine.board.peek(0xc000), 0);
        assert_eq!(
            (start..end)
                .map(|a| session.machine.board.peek(a))
                .collect::<Vec<_>>(),
            old_programme
        );
        enter(&mut session, "7");
        assert_eq!(session.machine.board.peek(0xc000), 42);
        enter(&mut session, "10 LET A=");
        assert_eq!(
            (start..end)
                .map(|a| session.machine.board.peek(a))
                .collect::<Vec<_>>(),
            old_programme
        );
        enter(&mut session, "9");
        assert_ne!(
            (start..end)
                .map(|a| session.machine.board.peek(a))
                .collect::<Vec<_>>(),
            old_programme
        );
        enter(&mut session, "RUN:POKE 49153,A");
        // RUN replaces the direct continuation; inspect A in a separate command.
        enter(&mut session, "POKE 49153,A");
        assert_eq!(session.machine.board.peek(0xc001), 9);
    }
    #[test]
    fn queued_literal_text_waits_for_guest_editor_work_without_losing_characters() {
        let mut session = Session::new(19).unwrap();
        let text = format!("REM {}", "x".repeat(900));
        session.type_text(&text).unwrap();
        let mut completed = false;
        for _ in 0..1600 {
            session.advance(MAX_SLICE).unwrap();
            session.machine.board.drain_audio();
            if session.input.is_empty()
                && session.held.is_empty()
                && session.machine.board.editor_ready()
                && session.machine.cpu.halted
            {
                completed = true;
                break;
            }
        }
        assert!(completed, "literal text must finish within its bound");
        let line = guest_word(&session.machine, 0x5c59);
        assert_eq!(
            (0..text.len())
                .map(|offset| session
                    .machine
                    .board
                    .peek(line + u16::try_from(offset).unwrap()))
                .collect::<Vec<_>>(),
            text.as_bytes()
        );
        assert_eq!(
            session
                .machine
                .board
                .peek(line + u16::try_from(text.len()).unwrap()),
            13
        );
        session.type_text("\nPOKE 49152,42\n").unwrap();
        for _ in 0..100 {
            session.advance(MAX_SLICE).unwrap();
            session.machine.board.drain_audio();
            if session.machine.board.peek(0xc000) == 42 {
                break;
            }
        }
        assert_eq!(session.machine.board.peek(0xc000), 42);
    }
    #[test]
    fn near_full_basic_heap_can_report_allocation_failure_and_accept_recovery_commands() {
        fn enter(session: &mut Session, text: &str) {
            command(session, Command::Type { text: text.into() }).unwrap();
            for _ in 0..400 {
                session.advance(MAX_SLICE).unwrap();
                session.machine.board.drain_audio();
                if session.input.is_empty()
                    && session.held.is_empty()
                    && session.machine.board.editor_ready()
                    && session.machine.cpu.halted
                {
                    return;
                }
            }
            panic!(
                "allocation recovery must return to an editable prompt: {}",
                session.frame().text
            );
        }
        let mut session = Session::new(20).unwrap();
        enter(&mut session, "LET B=77:DIM A(8180)");
        assert!(
            guest_word(&session.machine, 0x5c59) > 64000,
            "fixture must nearly fill the BASIC heap"
        );
        enter(&mut session, "DIM A(8250)");
        assert!(session.frame().text.contains("Out of memory"));
        enter(&mut session, "POKE 49152,B");
        assert_eq!(session.machine.board.peek(0xc000), 77);
        enter(&mut session, "CLEAR:POKE 49153,42");
        assert_eq!(session.machine.board.peek(0xc001), 42);
        assert!(guest_word(&session.machine, 0x5c59) < 24000);
    }
    #[test]
    fn input_after_a_debug_pause_accepts_the_whole_queued_number() {
        for (source, breakpoint, input_line) in [
            (
                "10 PRINT \"READY\"\n20 INPUT \"NUMBER? \";N\n30 POKE 49152,N\n40 STOP\n",
                1,
                20,
            ),
            (
                "10 BORDER 1:PAPER 0:INK 7:CLS\n20 PRINT \"SPECTRUM OWN ROM OK\"\n30 FOR N=1 TO 5:PLOT N*10,N*10:NEXT N\n40 DRAW 50,20:CIRCLE 120,80,20\n50 BEEP 0.1,0\n60 LET A$=\"TAPE\":LET V=3*7\n70 PRINT \"RESULT \";V;\" \";A$\n80 INPUT \"NUMBER? \";N\n90 PRINT \"INPUT OK \";N\n95 POKE 49152,N\n100 STOP\n",
                2,
                80,
            ),
        ] {
            let mut session =
                Session::from_source(14, &launch(source, Language::Basic, Some(breakpoint)))
                    .unwrap();
            for _ in 0..800 {
                command(&mut session, Command::Advance { cycles: MAX_SLICE }).unwrap();
                if session.debug_state().is_some_and(|d| d.values_valid) {
                    break;
                }
            }
            let stop = session.debug_state().unwrap();
            assert!(stop.values_valid);
            command(
                &mut session,
                Command::Step {
                    stop_id: stop.stop_id,
                },
            )
            .unwrap();
            let stop = session.debug_state().unwrap();
            command(
                &mut session,
                Command::Continue {
                    stop_id: stop.stop_id,
                },
            )
            .unwrap();
            for _ in 0..400 {
                command(
                    &mut session,
                    Command::Advance {
                        cycles: FRAME_T_STATES,
                    },
                )
                .unwrap();
                if guest_word(&session.machine, 0x5c45) == input_line {
                    break;
                }
            }
            for _ in 0..30 {
                command(
                    &mut session,
                    Command::Advance {
                        cycles: FRAME_T_STATES,
                    },
                )
                .unwrap();
            }
            command(&mut session, Command::Pause {}).unwrap();
            let stop = session.debug_state().unwrap();
            command(
                &mut session,
                Command::Continue {
                    stop_id: stop.stop_id,
                },
            )
            .unwrap();
            command(&mut session, Command::Type { text: "42".into() }).unwrap();
            command(
                &mut session,
                Command::Advance {
                    cycles: FRAME_T_STATES,
                },
            )
            .unwrap();
            command(&mut session, Command::Release {}).unwrap();
            for _ in 0..150 {
                command(
                    &mut session,
                    Command::Advance {
                        cycles: FRAME_T_STATES,
                    },
                )
                .unwrap();
                if session.machine.board.peek(0xc000) == 42 {
                    break;
                }
            }
            assert_eq!(
                session.machine.board.peek(0xc000),
                42,
                "{}",
                session.frame().text
            );
        }
    }
    #[test]
    fn holding_break_in_input_reports_once_and_allows_a_fresh_command() {
        let source = "10 INPUT \"NUMBER? \";N\n20 STOP\n";
        let mut session = Session::from_source(15, &launch(source, Language::Basic, None)).unwrap();
        for _ in 0..800 {
            command(&mut session, Command::Advance { cycles: MAX_SLICE }).unwrap();
            if guest_word(&session.machine, 0x5c45) == 10
                && session.machine.cpu.halted
                && session.machine.board.firmware_page() == 0
            {
                break;
            }
        }
        for _ in 0..30 {
            command(
                &mut session,
                Command::Advance {
                    cycles: FRAME_T_STATES,
                },
            )
            .unwrap();
        }
        command(
            &mut session,
            Command::Key {
                code: "ShiftLeft".into(),
                pressed: true,
            },
        )
        .unwrap();
        command(
            &mut session,
            Command::Key {
                code: "Space".into(),
                pressed: true,
            },
        )
        .unwrap();
        for _ in 0..12 {
            command(
                &mut session,
                Command::Advance {
                    cycles: FRAME_T_STATES,
                },
            )
            .unwrap();
        }
        command(&mut session, Command::Release {}).unwrap();
        let text = session.frame().text;
        assert_eq!(text.matches("STOP in INPUT").count(), 1, "{text}");
        command(
            &mut session,
            Command::Type {
                text: "POKE 49152,42".into(),
            },
        )
        .unwrap();
        for _ in 0..150 {
            command(
                &mut session,
                Command::Advance {
                    cycles: FRAME_T_STATES,
                },
            )
            .unwrap();
            if session.machine.board.peek(0xc000) == 42 {
                break;
            }
        }
        assert_eq!(
            session.machine.board.peek(0xc000),
            42,
            "{}",
            session.frame().text
        );
    }
    #[test]
    fn pcm_wave_cassette_loads_through_the_guest_ear_and_rejects_bad_replacement() {
        let tap = compile_basic(&TARGETS[1], "10 POKE 49152,42\n20 STOP\n").unwrap();
        let mut tape = eightbit_spectrum_emulator::Tape::parse(&tap).unwrap();
        tape.play();
        let mut pcm = Vec::new();
        let mut fraction = 0_u32;
        while tape.playing() {
            pcm.push(if tape.high() { 224 } else { 32 });
            fraction += CPU_HZ;
            let clocks = fraction / 11025;
            fraction %= 11025;
            tape.advance(clocks);
            assert!(pcm.len() < MAX_TAP_BYTES - 44);
        }
        if pcm.len() % 2 == 1 {
            pcm.push(128);
        }
        let len = u32::try_from(pcm.len()).unwrap();
        let mut wav = b"RIFF".to_vec();
        wav.extend((len + 36).to_le_bytes());
        wav.extend(b"WAVEfmt ");
        wav.extend(16_u32.to_le_bytes());
        wav.extend([1, 0, 1, 0]);
        wav.extend(11025_u32.to_le_bytes());
        wav.extend(11025_u32.to_le_bytes());
        wav.extend([1, 0, 8, 0]);
        wav.extend(b"data");
        wav.extend(len.to_le_bytes());
        wav.extend(pcm);
        let mut session = Session::new(12).unwrap();
        command(&mut session, Command::MountWav { bytes: wav }).unwrap();
        let identity = session.frame().cassette.unwrap().sha256;
        assert!(command(&mut session, Command::MountWav { bytes: vec![0] }).is_err());
        assert_eq!(session.frame().cassette.unwrap().sha256, identity);
        command(
            &mut session,
            Command::Type {
                text: "LOAD \"\"".into(),
            },
        )
        .unwrap();
        for _ in 0..800 {
            command(&mut session, Command::Advance { cycles: MAX_SLICE }).unwrap();
            if session.machine.board.peek(0xc000) == 42 {
                break;
            }
        }
        assert_eq!(session.machine.board.peek(0xc000), 42);
        command(&mut session, Command::RewindCassette {}).unwrap();
        assert!(session.frame().cassette.unwrap().pending);
        command(&mut session, Command::EjectCassette {}).unwrap();
        assert!(session.frame().cassette.is_none());
    }
    #[test]
    fn source_launch_loads_basic_through_guest_keyboard_and_cassette_pins() {
        let mut session =
            Session::from_source(1, &launch("10 POKE 49152,42\n", Language::Basic, None)).unwrap();
        for _ in 0..220 {
            let frame = command(&mut session, Command::Advance { cycles: MAX_SLICE }).unwrap();
            assert_eq!(frame.state, "running");
            assert_eq!(frame.pixels.len(), WIDTH * HEIGHT * 4);
            if session.machine.board.peek(0xc000) == 42 {
                return;
            }
        }
        panic!("Guest BASIC cassette did not run");
    }
    #[test]
    fn assembly_debugging_binds_exact_bytes_and_stops_with_stale_command_rejection() {
        let source = "ORG 24576\nLD A,42\nLD (49152),A\nRET\n";
        let mut session =
            Session::from_source(2, &launch(source, Language::Assembly, Some(3))).unwrap();
        let mut stopped = None;
        for _ in 0..350 {
            let frame = command(&mut session, Command::Advance { cycles: MAX_SLICE }).unwrap();
            assert_eq!(frame.state, "running");
            if frame.debug.as_ref().is_some_and(|d| d.state == "stopped") {
                stopped = frame.debug;
                break;
            }
        }
        let stopped = stopped.expect("Assembly must reach its mapped breakpoint");
        assert!(stopped.values_valid && stopped.breakpoint_verified);
        assert_eq!(stopped.source_line, Some(3));
        assert_eq!(stopped.registers.as_ref().unwrap()["af"] >> 8, 42);
        let frame = command(
            &mut session,
            Command::Step {
                stop_id: stopped.stop_id,
            },
        )
        .unwrap();
        let next = frame.debug.unwrap();
        assert_eq!(next.source_line, Some(4));
        assert_eq!(session.machine.board.peek(0xc000), 42);
        assert!(
            command(
                &mut session,
                Command::Continue {
                    stop_id: stopped.stop_id
                }
            )
            .is_err()
        );
        command(
            &mut session,
            Command::Continue {
                stop_id: next.stop_id,
            },
        )
        .unwrap();
        command(&mut session, Command::Advance { cycles: MAX_SLICE }).unwrap();
        command(&mut session, Command::Pause {}).unwrap();
        session.machine.board.write(0x6000, 0);
        let frame = command(&mut session, Command::State {}).unwrap();
        assert!(!frame.debug.unwrap().breakpoint_verified);
        command(&mut session, Command::Reset {}).unwrap();
        assert!(session.frame().debug.is_none());
        assert_eq!(
            command(&mut session, Command::Stop {}).unwrap().state,
            "stopped"
        );
        assert!(command(&mut session, Command::Advance { cycles: 1 }).is_err());
    }
    #[test]
    fn failed_guest_execution_resets_and_input_validation_does_not_partly_queue() {
        let mut session = Session::new(3).unwrap();
        assert!(session.frame().text.contains("EIGHTBIT"));
        assert!(
            session
                .command_json(
                    r#"{"schemaVersion":1,"sessionId":4,"command":{"operation":"reset"}}"#
                )
                .is_err()
        );
        assert!(
            session
                .command_json(
                    r#"{"schemaVersion":1,"sessionId":3,"command":{"operation":"state","extra":1}}"#
                )
                .is_err()
        );
        assert!(
            command(
                &mut session,
                Command::Advance {
                    cycles: MAX_SLICE + 1
                }
            )
            .is_err()
        );
        assert!(
            command(
                &mut session,
                Command::Type {
                    text: "abc\0".into()
                }
            )
            .is_err()
        );
        assert!(session.input.is_empty());
        for code in ["ShiftLeft", "ShiftRight"] {
            command(
                &mut session,
                Command::Key {
                    code: code.into(),
                    pressed: true,
                },
            )
            .unwrap();
        }
        command(
            &mut session,
            Command::Key {
                code: "ShiftLeft".into(),
                pressed: false,
            },
        )
        .unwrap();
        assert_eq!(session.machine.board.input(0xfefe) & 1, 0);
        command(&mut session, Command::Release {}).unwrap();
        assert_eq!(session.machine.board.input(0xfefe) & 1, 1);
        for address in 0x8000..0x8050 {
            session.machine.board.write(address, 0xdd);
        }
        session.machine.cpu.halted = false;
        session.machine.cpu.registers.pc = 0x8000;
        assert_eq!(
            command(&mut session, Command::Advance { cycles: 1000 })
                .unwrap()
                .state,
            "failed"
        );
        assert_eq!(
            command(&mut session, Command::Reset {}).unwrap().state,
            "running"
        );
        command(
            &mut session,
            Command::Type {
                text: "POKE 49152,77\n".into(),
            },
        )
        .unwrap();
        for _ in 0..20 {
            command(&mut session, Command::Advance { cycles: MAX_SLICE }).unwrap();
        }
        assert_eq!(session.machine.board.peek(0xc000), 77);
    }
    #[test]
    fn launch_and_media_boundaries_reject_mismatches_without_starting() {
        for field in ["target", "source", "cassette", "path", "revision"] {
            let mut source = launch("10 STOP\n", Language::Basic, None);
            match field {
                "target" => source.target = "spectrum16".into(),
                "source" => source.source.push(' '),
                "cassette" => source.cassette_sha256 = "0".repeat(64),
                "path" => source.source_path = "src/../private.bas".into(),
                _ => source.revision.clear(),
            }
            assert!(Session::from_source(1, &source).is_err());
        }
        let mut session = Session::new(1).unwrap();
        assert!(command(&mut session, Command::MountCassette { bytes: vec![1, 0] }).is_err());
        assert_eq!(
            command(&mut session, Command::State {}).unwrap().state,
            "running"
        );
        assert!(Session::new(0).is_err());
    }
}
