//! Admission and execution for the normal bundled original BASIC backend.
//!
//! A request cannot provide a qualification receipt or replace firmware. The
//! current development image deliberately fails this admission boundary.

use crate::basic_debug::BasicDebugController;
use crate::basic_debug_protocol::BasicDebugAdapter;
use crate::development_host::DevelopmentSession;
use crate::machine_debug::{
    MachineDebugCommand, MachineDebugController, MachineDebugRequest, MachineDebugTarget,
};
use crate::{
    MAX_SLICE, MonitorCommand, MonitorRequest, OriginalDevelopmentLaunch, SessionSnapshot,
};
use eightbit_dragon_basic_core::{TokenisedProgram, breakpoints::StatementMap};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const QUALIFIED_PROFILE: &str = "eightbit-dragon32-basic-extended-1";
const NOT_QUALIFIED: &str =
    "DME1400: original BASIC runtime is not qualified; use the explicit development route";

/// Immutable identity obtained only from the compiled bundled image/manifest.
/// It has no deserialiser or external-input constructor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginalRuntimeProfile {
    schema_version: u32,
    backend: &'static str,
    target: &'static str,
    profile_id: String,
    firmware_sha256: String,
    core_version: &'static str,
}

impl OriginalRuntimeProfile {
    #[must_use]
    pub fn profile_id(&self) -> &str {
        &self.profile_id
    }
    #[must_use]
    pub fn firmware_sha256(&self) -> &str {
        &self.firmware_sha256
    }
    #[must_use]
    pub fn core_version(&self) -> &str {
        self.core_version
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    schema_version: u32,
    id: String,
    status: String,
    licence: String,
    bytes: usize,
    rom_sha256: String,
    memory_map: MemoryMap,
}

#[derive(Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MemoryMap {
    ram_start: u32,
    ram_end_exclusive: u32,
    rom_start: u32,
    rom_end_exclusive: u32,
    io_start: u32,
    io_end_exclusive: u32,
    vector_alias_start: u32,
    vector_alias_end_exclusive: u32,
    vector_alias_source_start: u32,
    cartridges: bool,
    sam_all_ram: String,
}

impl MemoryMap {
    fn expanded() -> Self {
        Self {
            ram_start: 0,
            ram_end_exclusive: 0x8000,
            rom_start: 0x8000,
            rom_end_exclusive: 0xff00,
            io_start: 0xff00,
            io_end_exclusive: 0xffe0,
            vector_alias_start: 0xffe0,
            vector_alias_end_exclusive: 0x1_0000,
            vector_alias_source_start: 0xbfe0,
            cartridges: false,
            sam_all_ram: "unchanged".into(),
        }
    }
}

/// Read-only availability from the same admission used by every normal launch.
/// Null keeps the normal selector closed; it is not an external admission token.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginalRuntimeCapability {
    schema_version: u32,
    profile: Option<OriginalRuntimeProfile>,
}
/// Inspect compiled availability without allocating or executing a guest.
#[must_use]
pub fn bundled_original_runtime_capability() -> OriginalRuntimeCapability {
    bundled_original_runtime_capability_for_target("dragon32")
}
#[must_use]
pub fn bundled_original_runtime_capability_for_target(target: &str) -> OriginalRuntimeCapability {
    OriginalRuntimeCapability {
        schema_version: 1,
        profile: match target {
            "dragon32" => admit_bundled_original_runtime().ok(),
            "dragon64" => admit_bundled_dragon64_runtime(false).ok(),
            "dragon64-native" => admit_bundled_dragon64_runtime(true).ok(),
            _ => None,
        },
    }
}

/// Fail before build, evidence mutation, session allocation or guest execution.
pub fn admit_bundled_original_runtime() -> Result<OriginalRuntimeProfile, String> {
    admit(
        crate::BASIC_EXTENDED_FIRMWARE_MANIFEST,
        crate::BASIC_EXTENDED,
    )
}

fn admit(manifest: &str, image: &[u8]) -> Result<OriginalRuntimeProfile, String> {
    if manifest.len() > 32 * 1024 || image.len() != 32_768 {
        return Err(NOT_QUALIFIED.into());
    }
    let receipt: Manifest = serde_json::from_str(manifest).map_err(|_| NOT_QUALIFIED)?;
    if receipt.schema_version != 1
        || receipt.id != QUALIFIED_PROFILE
        || receipt.status != "qualified"
        || receipt.licence != "Apache-2.0"
        || receipt.bytes != image.len()
        || receipt.rom_sha256 != hex::encode(Sha256::digest(image))
        || receipt.memory_map != MemoryMap::expanded()
    {
        return Err(NOT_QUALIFIED.into());
    }
    Ok(OriginalRuntimeProfile {
        schema_version: 1,
        backend: "original",
        target: "dragon32",
        profile_id: receipt.id,
        firmware_sha256: receipt.rom_sha256,
        core_version: env!("CARGO_PKG_VERSION"),
    })
}

/// Admit only the two compiled D64 development profiles. This neither promotes
/// D32 qualification nor accepts a caller-provided image, map or receipt.
pub fn admit_bundled_dragon64_runtime(native: bool) -> Result<OriginalRuntimeProfile, String> {
    let value: serde_json::Value =
        serde_json::from_str(crate::dragon64::BASIC64_MANIFEST).map_err(|_| NOT_QUALIFIED)?;
    let mode = if native { "native" } else { "compatibility" };
    let expected = if native {
        "eightbit-dragon64-basic-64k-dev-1"
    } else {
        "eightbit-dragon64-basic-32k-dev-1"
    };
    let image: &[u8] = if native {
        crate::dragon64::BASIC64_NATIVE
    } else {
        crate::dragon64::BASIC64_COMPATIBILITY
    };
    let profile = &value["profiles"][mode];
    if value["schemaVersion"] != 1
        || value["licence"] != "Apache-2.0"
        || value["status"] != "development"
        || profile["id"] != expected
        || profile["bytes"] != image.len()
        || profile["ramBytes"] != 65536
        || profile["programEndExclusive"] != if native { 49152 } else { 32768 }
        || profile["sha256"] != hex::encode(Sha256::digest(image))
    {
        return Err(NOT_QUALIFIED.into());
    }
    Ok(OriginalRuntimeProfile {
        schema_version: 1,
        backend: "original",
        target: if native {
            "dragon64-native"
        } else {
            "dragon64"
        },
        profile_id: expected.into(),
        firmware_sha256: hex::encode(Sha256::digest(image)),
        core_version: env!("CARGO_PKG_VERSION"),
    })
}

impl OriginalRuntimeProfile {
    #[must_use]
    pub fn target(&self) -> &'static str {
        self.target
    }
    #[must_use]
    pub fn firmware(&self) -> crate::Firmware {
        match self.target {
            "dragon64" => crate::Firmware::Dragon64Compatibility,
            "dragon64-native" => crate::Firmware::Dragon64Native,
            _ => crate::Firmware::BasicExtended,
        }
    }
    fn revalidate(&self) -> Result<Self, String> {
        match self.target {
            "dragon64" => admit_bundled_dragon64_runtime(false),
            "dragon64-native" => admit_bundled_dragon64_runtime(true),
            _ => admit_bundled_original_runtime(),
        }
    }
}

/// Fresh in-process build handoff. Neither token maps nor admission are accepted
/// from a serialised caller. Normal Run and Debug use the same boot sequence.
pub struct OriginalRuntimeLaunch {
    profile: OriginalRuntimeProfile,
    launch: OriginalDevelopmentLaunch,
    program: TokenisedProgram,
    machine: Option<eightbit_dragon_media::MachineCodeCassette>,
    loader: Option<crate::machine_loader::MachineLoader>,
    machine_debug: Option<MachineDebugTarget>,
    breakpoint: Option<u16>,
    names: BTreeMap<String, String>,
}
impl OriginalRuntimeLaunch {
    pub fn new(
        launch: OriginalDevelopmentLaunch,
        program: TokenisedProgram,
        breakpoint: Option<u16>,
        names: BTreeMap<String, String>,
    ) -> Result<Self, String> {
        Self::prepared(
            admit_bundled_original_runtime()?,
            launch,
            program,
            breakpoint,
            names,
        )
    }
    /// Explicit D64 development route, using the same guest cassette owner.
    pub fn new_dragon64(
        native: bool,
        launch: OriginalDevelopmentLaunch,
        program: TokenisedProgram,
        breakpoint: Option<u16>,
        names: BTreeMap<String, String>,
    ) -> Result<Self, String> {
        Self::prepared(
            admit_bundled_dragon64_runtime(native)?,
            launch,
            program,
            breakpoint,
            names,
        )
    }
    fn prepared(
        profile: OriginalRuntimeProfile,
        launch: OriginalDevelopmentLaunch,
        program: TokenisedProgram,
        breakpoint: Option<u16>,
        names: BTreeMap<String, String>,
    ) -> Result<Self, String> {
        launch.validate()?;
        let cassette = launch
            .cassette
            .as_ref()
            .ok_or("DME1401: normal original launch requires a fresh cassette")?;
        if cassette.build_id.is_none()
            || usize::from(program.program_start) + program.bytes.len()
                > usize::from(profile.firmware().ram_end()) + 1
            || eightbit_dragon_media::read_tokenised_basic(&cassette.bytes)
                .map_err(|_| "DME1401: normal original launch requires tokenised BASIC")?
                .program
                != program.bytes
        {
            return Err("DME1401: original launch does not match its current build".into());
        }
        let map = StatementMap::new(program.clone())
            .map_err(|_| "DME1401: invalid current source map")?;
        if breakpoint.is_some_and(|line| !map.contains_line(line)) {
            return Err("DME1401: breakpoint has no mapped BASIC statement".into());
        }
        Ok(Self {
            profile,
            launch,
            program,
            machine: None,
            loader: None,
            machine_debug: None,
            breakpoint,
            names,
        })
    }
    /// A validated binary cassette is loaded by guest CLOADM and entered by EXEC.
    /// A media-only owner uses the exact cassette digest as its immutable identity.
    pub fn new_machine(
        launch: OriginalDevelopmentLaunch,
        debug: Option<MachineDebugTarget>,
    ) -> Result<Self, String> {
        Self::machine_with_profile(admit_bundled_original_runtime()?, launch, debug)
    }
    pub fn new_dragon64_machine(
        native: bool,
        launch: OriginalDevelopmentLaunch,
        debug: Option<MachineDebugTarget>,
    ) -> Result<Self, String> {
        Self::machine_with_profile(admit_bundled_dragon64_runtime(native)?, launch, debug)
    }
    fn machine_with_profile(
        profile: OriginalRuntimeProfile,
        launch: OriginalDevelopmentLaunch,
        debug: Option<MachineDebugTarget>,
    ) -> Result<Self, String> {
        launch.validate()?;
        let cassette = launch
            .cassette
            .as_ref()
            .ok_or("DME1502: machine launch requires a cassette")?;
        let machine = eightbit_dragon_media::read_machine_code(&cassette.bytes)
            .map_err(|_| "DME1502: machine launch requires a complete binary cassette")?;
        let start = usize::from(machine.load_address);
        let end = start + machine.program.len();
        if start < 0x0400
            || end > usize::from(profile.firmware().ram_end()) + 1
            || usize::from(machine.execution_address) < start
            || usize::from(machine.execution_address) >= end
        {
            return Err(format!(
                "DME1502: machine code and its entry must lie in RAM $0400-${:04X}",
                profile.firmware().ram_end()
            ));
        }
        // Reserve BASIC below high loads; images spanning the existing stack
        // use an authored physical-tape bootstrap below the payload instead.
        let loader = if start < 0x2000 && end > usize::from(profile.firmware().ram_end()) - 0x1ff {
            Some(crate::machine_loader::MachineLoader::for_native(
                &machine,
                &cassette.bytes,
                profile.target == "dragon64-native",
            )?)
        } else {
            None
        };
        if let Some(target) = &debug {
            target.validate_extent(machine.load_address, machine.program.len())?;
        }
        Ok(Self {
            profile,
            launch,
            program: TokenisedProgram {
                program_start: 0x1e01,
                bytes: Vec::new(),
                lines: Vec::new(),
            },
            machine: Some(machine),
            loader,
            machine_debug: debug,
            breakpoint: None,
            names: BTreeMap::new(),
        })
    }
    /// Assembly debugging through the existing generated BASIC loader.
    pub fn new_mixed(
        launch: OriginalDevelopmentLaunch,
        program: TokenisedProgram,
        target: MachineDebugTarget,
    ) -> Result<Self, String> {
        let mut result = Self::new(launch, program, None, BTreeMap::new())?;
        result.machine_debug = Some(target);
        Ok(result)
    }
    pub fn new_dragon64_mixed(
        native: bool,
        launch: OriginalDevelopmentLaunch,
        program: TokenisedProgram,
        target: MachineDebugTarget,
    ) -> Result<Self, String> {
        let mut result = Self::new_dragon64(native, launch, program, None, BTreeMap::new())?;
        result.machine_debug = Some(target);
        Ok(result)
    }
    /// Run an explicitly selected, digest-bound BASIC or machine-code cassette.
    /// This supplies no authored source map and therefore no source debugger.
    pub fn new_media(launch: OriginalDevelopmentLaunch) -> Result<Self, String> {
        let profile = admit_bundled_original_runtime()?;
        launch.validate()?;
        let cassette = launch
            .cassette
            .as_ref()
            .ok_or("DME1502: media launch requires a cassette")?;
        if eightbit_dragon_media::read_machine_code(&cassette.bytes).is_ok() {
            return Self::new_machine(launch, None);
        }
        let dialect = eightbit_dragon_basic_core::Dialect::dragon32().map_err(|e| e.to_string())?;
        let inspection = eightbit_dragon_media::inspect_basic_media(
            &cassette.bytes,
            eightbit_dragon_media::MediaFormatHint::Auto,
        )
        .map_err(|_| "DME1502: imported BASIC cassette cannot be recovered exactly")?;
        let source = inspection.source;
        let analysis = eightbit_dragon_basic_core::analyse(&source, &dialect);
        if analysis
            .diagnostics
            .iter()
            .any(|d| d.severity == eightbit_dragon_basic_core::Severity::Error)
        {
            return Err("DME1502: imported BASIC is not a valid executable programme".into());
        }
        let program = eightbit_dragon_basic_core::tokenise(&analysis.program, &dialect)
            .map_err(|e| e.to_string())?;
        if usize::from(program.program_start) + program.bytes.len() > 0x8000 {
            return Err("DME1502: imported BASIC leaves no guest loading workspace".into());
        }
        Ok(Self {
            profile,
            launch,
            program,
            machine: None,
            loader: None,
            machine_debug: None,
            breakpoint: None,
            names: BTreeMap::new(),
        })
    }
    #[must_use]
    pub fn profile(&self) -> &OriginalRuntimeProfile {
        &self.profile
    }
    #[must_use]
    pub fn cassette_launch(&self) -> &OriginalDevelopmentLaunch {
        &self.launch
    }

    /// Admission was obtained before preparation; recheck the immutable bundled
    /// identity before normal execution. Test-only fixture starts remain private.
    pub fn start(self, id: u32) -> Result<DevelopmentSession, String> {
        if self.profile.revalidate()? != self.profile {
            return Err(NOT_QUALIFIED.into());
        }
        self.start_admitted(id)
    }
    fn start_admitted(self, id: u32) -> Result<DevelopmentSession, String> {
        let cassette = self
            .launch
            .cassette
            .as_ref()
            .ok_or("DME1401: missing current cassette")?;
        // Two half periods per bit, eight bits per byte; use the slow bit and
        // fastest SAM clock ratio. Ten seconds of guest time covers boot/keys.
        let transport_length = self
            .loader
            .as_ref()
            .map_or(cassette.bytes.len(), |loader| loader.transport.len());
        let limit = transport_length as u64 * 13_008 + u64::from(crate::MASTER_HZ) * 10 / 8;
        let build_id = cassette
            .build_id
            .clone()
            .unwrap_or_else(|| cassette.sha256.clone());
        let transport = if let Some(loader) = &self.loader {
            OriginalDevelopmentLaunch::with_cassette(
                loader.transport.clone(),
                Some(build_id.clone()),
            )?
        } else {
            self.launch
        };
        let session = transport.start_with_firmware(id, self.profile.firmware())?;
        let inner = if let Some(target) = self.machine_debug {
            DevelopmentSession::Machine(Box::new(MachineDebugController::new(
                session,
                target,
                build_id.clone(),
            )))
        } else if let Some(line) = self.breakpoint {
            let controller = BasicDebugController::new(
                session,
                StatementMap::new(self.program.clone())
                    .map_err(|_| "DME1401: invalid current map")?,
                line,
                build_id.clone(),
                self.names,
            )?;
            let mut adapter = BasicDebugAdapter::original(controller);
            // Reserve the real search revision now so Cancel remains valid while
            // the boot owner loads. No search/inspection runs before verification.
            adapter.command(
                crate::basic_debug_protocol::DebugRequest {
                    schema_version: 1,
                    revision: adapter.snapshot().revision,
                    command: crate::basic_debug_protocol::DebugCommand::Continue,
                },
                || false,
            )?;
            DevelopmentSession::Debug {
                adapter: Box::new(adapter),
                preparing: true,
            }
        } else {
            DevelopmentSession::from_monitor(session)
        };
        Ok(DevelopmentSession::Original(Box::new(
            OriginalRuntimeSession {
                inner: Box::new(inner),
                profile: self.profile,
                build_id,
                program: self.program,
                machine: self.machine,
                loader: self.loader,
                phase: BootPhase::Preparing(LoadPhase::Prompt),
                cycles: 0,
                limit,
            },
        )))
    }
}

#[derive(Clone, Copy)]
enum LoadPhase {
    Prompt,
    Loading,
    Reserving,
    Bootstrap,
}
enum BootPhase {
    Preparing(LoadPhase),
    Paused(LoadPhase),
    Executing,
    Failed(String),
}

/// A normal launch delegates to the existing session after bounded guest loading.
/// There is still only one host scheduler and one recording/save owner.
pub struct OriginalRuntimeSession {
    inner: Box<DevelopmentSession>,
    profile: OriginalRuntimeProfile,
    build_id: String,
    program: TokenisedProgram,
    machine: Option<eightbit_dragon_media::MachineCodeCassette>,
    loader: Option<crate::machine_loader::MachineLoader>,
    phase: BootPhase,
    cycles: u64,
    limit: u64,
}
impl OriginalRuntimeSession {
    fn inner(&self) -> &DevelopmentSession {
        &self.inner
    }
    fn inner_mut(&mut self) -> &mut DevelopmentSession {
        &mut self.inner
    }
    #[must_use]
    pub fn profile(&self) -> &OriginalRuntimeProfile {
        &self.profile
    }
    /// Immutable fresh build identity, also available for Run without a debugger.
    #[must_use]
    pub fn build_id(&self) -> &str {
        &self.build_id
    }
    pub(crate) fn id(&self) -> u32 {
        self.inner().id()
    }
    pub(crate) fn snapshot(&self) -> SessionSnapshot {
        self.describe(self.inner().snapshot())
    }
    fn describe(&self, mut frame: SessionSnapshot) -> SessionSnapshot {
        match &self.phase {
            BootPhase::Preparing(_) => {
                frame.machine.message = "Loading the current build".into();
            }
            BootPhase::Paused(_) => {
                frame.machine.message = "Original launch cancelled".into();
            }
            BootPhase::Failed(message) => {
                frame.machine.state = "failed";
                frame.machine.message.clone_from(message);
            }
            BootPhase::Executing => {}
        }
        frame
    }
    pub(crate) fn command(&mut self, request: MonitorRequest) -> Result<SessionSnapshot, String> {
        if request.schema_version != 1 || request.session_id != self.id() {
            return Err("DME1004: original request belongs to a stale session".into());
        }
        if matches!(
            request.command,
            MonitorCommand::Stop | MonitorCommand::Reset
        ) {
            // Explicit reset cancels automatic loading; it never silently RUNs
            // an old build again. A new Run prepares a fresh immutable build.
            self.phase = BootPhase::Executing;
            return self.inner_mut().command(request);
        }
        if matches!(self.phase, BootPhase::Executing) {
            return self.inner_mut().command(request);
        }
        if let MonitorCommand::Advance { cycles } = request.command {
            if cycles == 0 || cycles > MAX_SLICE {
                return Err("DME1002: invalid execution slice".into());
            }
            if matches!(self.phase, BootPhase::Preparing(_))
                && let Err(error) = self.advance(cycles)
            {
                self.phase = BootPhase::Failed(error);
            }
            let frame = match self.inner_mut() {
                DevelopmentSession::Monitor(session) => session.debug_frame(),
                DevelopmentSession::Debug { adapter, .. } => adapter.preparation_frame(),
                DevelopmentSession::Machine(controller) => controller.preparation_frame(),
                DevelopmentSession::Original(_) => {
                    return Err("DME1401: invalid nested original owner".into());
                }
            };
            return Ok(self.describe(frame));
        }
        if matches!(
            request.command,
            MonitorCommand::State | MonitorCommand::Release
        ) {
            let frame = self.inner_mut().command(request)?;
            return Ok(self.describe(frame));
        }
        Err(
            "DME1401: wait for the current build to load, or reset/stop the original session"
                .into(),
        )
    }
    fn advance(&mut self, budget: u32) -> Result<(), String> {
        let mut elapsed = 0;
        while elapsed < budget {
            if self.cycles >= self.limit {
                return Err("DME1401: original build load timed out; reset or run again".into());
            }
            let machine = match self.inner.as_mut() {
                DevelopmentSession::Monitor(session) => session.debug_machine_mut(),
                DevelopmentSession::Debug { adapter, .. } => adapter.preparation_machine_mut(),
                DevelopmentSession::Machine(controller) => controller.preparation_machine_mut(),
                DevelopmentSession::Original(_) => {
                    return Err("DME1401: invalid nested original owner".into());
                }
            };
            let pc = machine.registers().pc;
            let direct = machine.peek(0x68) == 0xff && machine.peek(0x69) == 0xff;
            if matches!(self.phase, BootPhase::Preparing(LoadPhase::Prompt))
                && pc
                    == if self.profile.firmware().is_dragon64() {
                        0x0182
                    } else {
                        0x0167
                    }
                && direct
                && !machine.input_pending()
            {
                if let Some(binary) = &self.machine {
                    if self.loader.is_some() {
                        machine.type_text("CLOADM:EXEC")?;
                        self.phase = BootPhase::Preparing(LoadPhase::Loading);
                    } else if binary.load_address >= 0x2000 {
                        machine.type_text(&format!("CLEAR 0,{}", binary.load_address))?;
                        self.phase = BootPhase::Preparing(LoadPhase::Reserving);
                    } else {
                        let command = if binary.load_address == 0x0400 {
                            // The published block reader toggles the screen marker at
                            // $0400, including EOF. Restore this one payload byte in
                            // guest BASIC before entering the validated machine code.
                            format!("CLOADM:POKE1024,{}:EXEC", binary.program[0])
                        } else {
                            "CLOADM:EXEC".into()
                        };
                        machine.type_text(&command)?;
                        self.phase = BootPhase::Preparing(LoadPhase::Loading);
                    }
                } else {
                    machine.type_text("CLOAD")?;
                    self.phase = BootPhase::Preparing(LoadPhase::Loading);
                }
            } else if matches!(self.phase, BootPhase::Preparing(LoadPhase::Reserving))
                && pc == 0x0182
                && direct
                && !machine.input_pending()
            {
                let binary = self
                    .machine
                    .as_ref()
                    .ok_or("DME1502: missing machine load owner")?;
                if u16::from_be_bytes([machine.peek(0x21), machine.peek(0x22)])
                    != binary.load_address - 1
                {
                    return Err(
                        "DME1502: guest could not reserve machine-code loading workspace".into(),
                    );
                }
                machine.type_text("CLOADM:EXEC")?;
                self.phase = BootPhase::Preparing(LoadPhase::Loading);
            } else if matches!(self.phase, BootPhase::Preparing(LoadPhase::Loading))
                && ((self.machine.is_none() && pc == 0x0182)
                    || (self.machine.is_some()
                        && pc == self.profile.firmware().statement_hook()
                        && machine.peek(machine.registers().x) == 0xa4))
                && direct
                && !machine.input_pending()
            {
                let (address, bytes) = if let Some(loader) = &self.loader {
                    if u16::from_be_bytes([machine.peek(0x9d), machine.peek(0x9e)])
                        != crate::machine_loader::ADDRESS
                    {
                        return Err(
                            "DME1502: guest bootstrap entry differs from its cassette".into()
                        );
                    }
                    (crate::machine_loader::ADDRESS, &loader.bytes)
                } else if let Some(binary) = &self.machine {
                    if u16::from_be_bytes([machine.peek(0x9d), machine.peek(0x9e)])
                        != binary.execution_address
                    {
                        return Err("DME1502: guest machine-code entry does not match the selected cassette".into());
                    }
                    (binary.load_address, &binary.program)
                } else {
                    if u16::from_be_bytes([machine.peek(0x19), machine.peek(0x1a)])
                        != self.program.program_start
                    {
                        return Err(
                            "DME1401: loaded BASIC base does not match the current build".into(),
                        );
                    }
                    (self.program.program_start, &self.program.bytes)
                };
                for (index, chunk) in bytes.chunks(1024).enumerate() {
                    let address = u16::try_from(usize::from(address) + index * 1024)
                        .map_err(|_| "DME1502: loaded build exceeds memory")?;
                    if machine.read_memory(address, chunk.len())? != chunk {
                        return Err("DME1502: guest load did not produce the selected build".into());
                    }
                }
                if self.loader.is_some() {
                    self.phase = BootPhase::Preparing(LoadPhase::Bootstrap);
                } else {
                    if self.machine.is_none() {
                        machine.type_text("RUN")?;
                    }
                    self.loaded();
                    return Ok(());
                }
            }
            if self.machine.is_some()
                && matches!(self.phase, BootPhase::Preparing(LoadPhase::Loading))
                && pc == 0x0182
                && direct
                && !machine.input_pending()
            {
                return Err("DME1502: guest rejected the machine-code cassette before EXEC".into());
            }
            if matches!(self.phase, BootPhase::Preparing(LoadPhase::Bootstrap)) {
                let loader = self
                    .loader
                    .as_ref()
                    .ok_or("DME1502: missing guest bootstrap")?;
                if pc == loader.failed {
                    return Err("DME1502: guest bootstrap rejected the selected cassette".into());
                }
                if pc == loader.ready {
                    let binary = self
                        .machine
                        .as_ref()
                        .ok_or("DME1502: missing machine payload")?;
                    for (index, chunk) in binary.program.chunks(1024).enumerate() {
                        let address =
                            u16::try_from(usize::from(binary.load_address) + index * 1024)
                                .map_err(|_| "DME1502: loaded build exceeds memory")?;
                        if machine.read_memory(address, chunk.len())? != chunk {
                            return Err(
                                "DME1502: guest bootstrap did not produce the selected build"
                                    .into(),
                            );
                        }
                    }
                    self.loaded();
                    return Ok(());
                }
            }
            let consumed = machine.step()?;
            self.cycles += u64::from(consumed);
            elapsed += consumed;
        }
        Ok(())
    }
    fn loaded(&mut self) {
        if let DevelopmentSession::Debug { preparing, .. } = self.inner.as_mut() {
            *preparing = false;
        }
        if let DevelopmentSession::Machine(controller) = self.inner.as_mut() {
            controller.loaded();
        }
        self.phase = BootPhase::Executing;
    }
    fn describe_machine_reply(&self, reply: &mut crate::machine_debug::MachineDebugReply) {
        // All preparation replies, including rejected inspection requests, retain
        // the boot owner's authoritative failed/cancelled/loading state.
        reply.session = self.describe(self.inner().snapshot());
        reply.status.state = match self.phase {
            BootPhase::Failed(_) => "failed",
            BootPhase::Paused(_) => "cancelled",
            _ => "running",
        };
        reply.status.message = reply.session.machine.message.clone();
    }
    pub(crate) fn debug_state(&self) -> Result<Option<String>, String> {
        if matches!(self.phase, BootPhase::Executing) {
            return self.inner().debug_state();
        }
        if let DevelopmentSession::Machine(controller) = self.inner() {
            let mut reply = controller.snapshot();
            self.describe_machine_reply(&mut reply);
            return crate::machine_debug::encode(&reply).map(Some);
        }
        let DevelopmentSession::Debug { adapter, .. } = self.inner() else {
            return Ok(None);
        };
        // Loading exposes no stale variables, source location or CPU inspection.
        // It supplies the immutable owner needed by the existing window bridge.
        let mut reply = adapter.snapshot();
        reply.session = self.describe(reply.session);
        reply.status.state = match self.phase {
            BootPhase::Failed(_) => "failed",
            BootPhase::Paused(_) => "cancelled",
            _ => "running",
        };
        reply.status.message = reply.session.machine.message.clone();
        crate::basic_debug_protocol::encode(&reply).map(Some)
    }
    pub(crate) fn debug_command(&mut self, json: &str) -> Result<String, String> {
        if matches!(self.phase, BootPhase::Executing) {
            return self.inner_mut().debug_command(json);
        }
        if let DevelopmentSession::Machine(controller) = self.inner.as_mut() {
            let request = MachineDebugRequest::parse(json)?;
            let cancel = matches!(
                request.command,
                MachineDebugCommand::Cancel | MachineDebugCommand::Pause
            );
            let resume = matches!(request.command, MachineDebugCommand::Continue);
            let allowed = matches!(request.command, MachineDebugCommand::State)
                || (cancel && matches!(self.phase, BootPhase::Preparing(_)))
                || (resume && matches!(self.phase, BootPhase::Paused(_)));
            if !allowed {
                let mut reply = controller.rejection(
                    &request,
                    "DME1502: wait for guest loading before machine inspection",
                )?;
                self.describe_machine_reply(&mut reply);
                return crate::machine_debug::encode(&reply);
            }
            controller.command(&request)?;
            if cancel {
                if let BootPhase::Preparing(phase) = self.phase {
                    self.phase = BootPhase::Paused(phase);
                }
            } else if resume && let BootPhase::Paused(phase) = self.phase {
                self.phase = BootPhase::Preparing(phase);
            }
            return self
                .debug_state()?
                .ok_or_else(|| "DME1500: missing machine debugger".into());
        }
        use crate::basic_debug_protocol::{DebugCommand, DebugRequest};
        if json.len() > crate::MAX_MONITOR_REQUEST_BYTES {
            return Err("DME1302: BASIC debug request is too large".into());
        }
        let request: DebugRequest =
            serde_json::from_str(json).map_err(|_| "DME1302: invalid BASIC debug request")?;
        let DevelopmentSession::Debug { adapter, preparing } = self.inner.as_mut() else {
            return Err("DME1302: this original session has no debugger".into());
        };
        let cancel = matches!(request.command, DebugCommand::Cancel);
        let resume = matches!(request.command, DebugCommand::Continue);
        let allowed = matches!(request.command, DebugCommand::State)
            || (cancel && matches!(self.phase, BootPhase::Preparing(_)))
            || (resume && matches!(self.phase, BootPhase::Paused(_)));
        if !allowed {
            return Err(
                "DME1401: wait for the current BASIC build to load, or reset the session".into(),
            );
        }
        adapter.command(request, || false)?;
        if cancel {
            *preparing = false;
            if let BootPhase::Preparing(phase) = self.phase {
                self.phase = BootPhase::Paused(phase);
            }
        } else if resume && let BootPhase::Paused(phase) = self.phase {
            *preparing = true;
            self.phase = BootPhase::Preparing(phase);
        }
        self.debug_state()?
            .ok_or_else(|| "DME1302: missing original debugger".into())
    }
    pub(crate) fn export_recording(&mut self) -> Result<crate::RecordingExport, String> {
        self.inner_mut().export_recording()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn guest_bootstrap_bounds_stack_with_nmi_and_rejects_changed_header() -> Result<(), String> {
        let mut normal_minimum = 0;
        for scenario in [None, Some(false), Some(true)] {
            let changed_header = scenario == Some(true);
            let payload = vec![0x12; 300];
            let cassette =
                eightbit_dragon_media::create_machine_code("PAYLOAD", &payload, 0x0400, 0x0400)
                    .map_err(|e| e.to_string())?;
            let mut prepared = OriginalRuntimeLaunch::new_machine(
                OriginalDevelopmentLaunch::with_cassette(cassette.clone(), None)?,
                None,
            )?;
            let binary = prepared.machine.as_ref().ok_or("missing binary")?;
            let second = if changed_header {
                eightbit_dragon_media::create_machine_code("PAYLOAD", &payload, 0x0400, 0x0401)
                    .map_err(|e| e.to_string())?
            } else {
                cassette
            };
            // Force the same generated bootstrap for a short payload so every
            // instruction can be observed without a second full-RAM tape run.
            prepared.loader = Some(crate::machine_loader::MachineLoader::new(binary, &second)?);
            let DevelopmentSession::Original(mut owner) = prepared.start(1)? else {
                return Err("wrong runtime owner".into());
            };
            let mut minimum = 0x0300;
            let mut nmi_sent = false;
            let mut nmi_clear = false;
            let mut completed = false;
            for _ in 0..8_000_000 {
                let bootstrap = matches!(owner.phase, BootPhase::Preparing(LoadPhase::Bootstrap));
                if bootstrap {
                    let DevelopmentSession::Monitor(session) = owner.inner.as_mut() else {
                        return Err("wrong inner owner".into());
                    };
                    let machine = session.debug_machine_mut();
                    let stack = machine.registers().s;
                    if stack <= 0x0300 {
                        minimum = minimum.min(stack);
                    }
                    if nmi_clear {
                        machine.nmi(false);
                        nmi_clear = false;
                    }
                    if scenario.is_some() && !nmi_sent && stack == normal_minimum {
                        machine.nmi(true);
                        nmi_sent = true;
                        nmi_clear = true;
                    }
                }
                match owner.advance(if bootstrap { 1 } else { MAX_SLICE }) {
                    Ok(()) => {}
                    Err(error) => {
                        assert!(changed_header, "{error}");
                        assert!(error.contains("bootstrap rejected"), "{error}");
                        completed = true;
                        break;
                    }
                }
                if matches!(owner.phase, BootPhase::Executing) {
                    assert!(!changed_header);
                    completed = true;
                    break;
                }
            }
            assert!(completed);
            if scenario.is_none() {
                normal_minimum = minimum;
            } else {
                assert!(
                    nmi_sent,
                    "test must exercise NMI at the deepest normal stack"
                );
                assert_eq!(minimum, normal_minimum - 12);
            }
            assert!(minimum >= 0x01e9, "stack ${minimum:04X} overlaps header");
            assert!(minimum < 0x02f0, "stack depth was not measured");
            eprintln!("bootstrap minimum S=${minimum:04X}, changed_header={changed_header}");
        }
        Ok(())
    }

    #[test]
    fn guest_bootstrap_cancel_failure_and_reset_preserve_runtime_ownership() -> Result<(), String> {
        let payload = vec![0x12; 300];
        let cassette =
            eightbit_dragon_media::create_machine_code("PAYLOAD", &payload, 0x0400, 0x0400)
                .map_err(|e| e.to_string())?;
        let changed =
            eightbit_dragon_media::create_machine_code("PAYLOAD", &payload, 0x0400, 0x0401)
                .map_err(|e| e.to_string())?;
        let mut prepared = OriginalRuntimeLaunch::new_machine(
            OriginalDevelopmentLaunch::with_cassette(cassette, None)?,
            Some(MachineDebugTarget::new(0x0400, Vec::new())?),
        )?;
        prepared.loader = Some(crate::machine_loader::MachineLoader::new(
            prepared.machine.as_ref().ok_or("missing binary")?,
            &changed,
        )?);
        let DevelopmentSession::Original(mut owner) = prepared.start(19)? else {
            return Err("wrong runtime owner".into());
        };
        for _ in 0..256 {
            owner.advance(MAX_SLICE)?;
            if matches!(owner.phase, BootPhase::Preparing(LoadPhase::Bootstrap)) {
                break;
            }
        }
        assert!(matches!(
            owner.phase,
            BootPhase::Preparing(LoadPhase::Bootstrap)
        ));
        let state = |owner: &OriginalRuntimeSession| -> Result<serde_json::Value, String> {
            serde_json::from_str(&owner.debug_state()?.ok_or("missing debugger")?)
                .map_err(|e| e.to_string())
        };
        let command = |owner: &mut OriginalRuntimeSession, operation: &str| -> Result<(), String> {
            owner.debug_command(&json!({"schemaVersion":1,"revision":state(owner)?["revision"],"command":{"operation":operation}}).to_string())?;
            Ok(())
        };
        command(&mut owner, "cancel")?;
        let cancelled = state(&owner)?;
        assert_eq!(cancelled["status"]["state"], "cancelled");
        assert_eq!(cancelled["status"]["valuesValid"], false);
        let advance = || MonitorRequest {
            schema_version: 1,
            session_id: 19,
            command: MonitorCommand::Advance { cycles: MAX_SLICE },
        };
        for _ in 0..3 {
            owner.command(advance())?;
        }
        assert_eq!(state(&owner)?["revision"], cancelled["revision"]);
        command(&mut owner, "continue")?;
        let mut failed = false;
        for _ in 0..256 {
            if owner.command(advance())?.machine.state == "failed" {
                failed = true;
                break;
            }
        }
        assert!(failed);
        assert_eq!(state(&owner)?["status"]["state"], "failed");
        assert_eq!(state(&owner)?["status"]["valuesValid"], false);
        let failed_reply = state(&owner)?;
        let denied: serde_json::Value = serde_json::from_str(&owner.debug_command(&json!({"schemaVersion":1,"revision":failed_reply["revision"],"command":{"operation":"memory","address":1024,"length":1}}).to_string())?).map_err(|e|e.to_string())?;
        assert_eq!(denied["status"]["state"], "failed");
        assert_eq!(denied["revision"], failed_reply["revision"]);
        assert_eq!(denied["failure"]["code"], "DME1502");
        owner.command(MonitorRequest {
            schema_version: 1,
            session_id: 19,
            command: MonitorCommand::Reset,
        })?;
        for _ in 0..64 {
            owner.command(advance())?;
        }
        assert_eq!(state(&owner)?["status"]["state"], "ready");
        assert!(owner.snapshot().machine.text.contains("OK>"));
        Ok(())
    }

    #[test]
    fn compiled_capability_is_read_only_and_unqualified_firmware_stays_closed()
    -> Result<(), Box<dyn std::error::Error>> {
        let value = serde_json::to_value(bundled_original_runtime_capability())?;
        let compiled: serde_json::Value =
            serde_json::from_str(crate::BASIC_EXTENDED_FIRMWARE_MANIFEST)?;
        assert_eq!(value["schemaVersion"], 1);
        if compiled["status"] == "development" {
            assert_eq!(value["profile"], serde_json::Value::Null);
        } else {
            assert_eq!(compiled["status"], "qualified");
            assert_eq!(value["profile"]["profileId"], QUALIFIED_PROFILE);
            assert_eq!(
                value["profile"]["firmwareSha256"],
                hex::encode(Sha256::digest(crate::BASIC_EXTENDED))
            );
            assert_eq!(value["profile"]["target"], "dragon32");
        }
        let image = vec![0x12; 32_768];
        let profile = admit(&receipt(&image).to_string(), &image)?;
        let qualified = OriginalRuntimeCapability {
            schema_version: 1,
            profile: Some(profile),
        };
        assert_eq!(
            serde_json::to_value(qualified)?["profile"]["profileId"],
            QUALIFIED_PROFILE
        );
        Ok(())
    }

    fn receipt(image: &[u8]) -> serde_json::Value {
        json!({"schemaVersion":1,"id":QUALIFIED_PROFILE,"status":"qualified",
            "licence":"Apache-2.0","bytes":image.len(),
            "romSha256":hex::encode(Sha256::digest(image)),
            "memoryMap": MemoryMap::expanded()})
    }

    fn fixture(breakpoint: Option<u16>) -> Result<DevelopmentSession, String> {
        use eightbit_dragon_basic_core::{
            Dialect, analyse, inspection::source_variable_names, tokenise,
        };
        let source = "10 COUNT=41\n20 COUNT=COUNT+1:PRINT COUNT\n30 END\n";
        let dialect = Dialect::dragon32().map_err(|e| e.to_string())?;
        let program =
            tokenise(&analyse(source, &dialect).program, &dialect).map_err(|e| e.to_string())?;
        let cassette = eightbit_dragon_media::create_tokenised_basic("NORMAL", &program.bytes)
            .map_err(|e| e.to_string())?;
        // Private test admission exercises the real expanded Board boot sequence.
        // It does not promote the compiled receipt or open public admission.
        let profile = admit(
            &receipt(crate::BASIC_EXTENDED).to_string(),
            crate::BASIC_EXTENDED,
        )?;
        OriginalRuntimeLaunch::prepared(
            profile,
            OriginalDevelopmentLaunch::with_cassette(cassette, Some("a".repeat(64)))?,
            program,
            breakpoint,
            source_variable_names(source),
        )?
        .start_admitted(7)
    }
    fn tick(session: &mut DevelopmentSession) -> Result<SessionSnapshot, String> {
        session.command(MonitorRequest {
            schema_version: 1,
            session_id: session.id(),
            command: MonitorCommand::Advance { cycles: MAX_SLICE },
        })
    }
    #[test]
    fn fixture_admission_runs_real_cload_and_run_without_host_memory_injection()
    -> Result<(), String> {
        let mut session = fixture(None)?;
        assert_eq!(session.snapshot().machine.firmware_id, QUALIFIED_PROFILE);
        assert_eq!(
            session.snapshot().machine.pc,
            0xb3b4,
            "boot is owned from reset"
        );
        let mut finished = false;
        for _ in 0..768 {
            let frame = tick(&mut session)?;
            assert_ne!(
                frame.machine.state, "failed",
                "{}; PC={:04x}; screen={:?}",
                frame.machine.message, frame.machine.pc, frame.machine.text
            );
            if frame.machine.text.contains("42") {
                finished = true;
                break;
            }
        }
        assert!(finished, "normal boot did not run the fresh programme");
        assert!(session.debug_state()?.is_none());
        Ok(())
    }
    #[test]
    fn fixture_admission_hands_loaded_guest_to_existing_debugger_and_keeps_revisions()
    -> Result<(), String> {
        let mut session = fixture(Some(20))?;
        assert_eq!(session.snapshot().machine.firmware_id, QUALIFIED_PROFILE);
        let mut stopped = None;
        for _ in 0..768 {
            let frame = tick(&mut session)?;
            assert_ne!(frame.machine.state, "failed", "{}", frame.machine.message);
            if let Some(json) = session.debug_state()? {
                let state: serde_json::Value =
                    serde_json::from_str(&json).map_err(|e| e.to_string())?;
                if state["status"]["state"] == "stopped" {
                    stopped = Some(state);
                    break;
                }
            }
        }
        let first = stopped.ok_or("normal boot never reached the source breakpoint")?;
        assert_eq!(first["status"]["location"]["basicLine"], 20);
        assert_eq!(first["status"]["variables"][0]["numericValue"], 41.0);
        let request = json!({"schemaVersion":1,"revision":first["revision"],"command":{"operation":"continue"}}).to_string();
        session.debug_command(&request)?;
        assert!(session.debug_command(&request).is_err());
        for _ in 0..64 {
            tick(&mut session)?;
            let state: serde_json::Value =
                serde_json::from_str(&session.debug_state()?.ok_or("lost debug owner")?)
                    .map_err(|e| e.to_string())?;
            if state["status"]["state"] == "stopped" {
                assert_eq!(state["status"]["variables"][0]["numericValue"], 42.0);
                return Ok(());
            }
        }
        Err("normal original Continue did not stop".into())
    }
    #[test]
    fn normal_loading_debug_owner_cancels_without_running_or_accepting_stale_revisions()
    -> Result<(), String> {
        let mut session = fixture(Some(20))?;
        let initial: serde_json::Value =
            serde_json::from_str(&session.debug_state()?.ok_or("missing debugger")?)
                .map_err(|e| e.to_string())?;
        assert_eq!(initial["backend"], "original");
        assert_eq!(initial["status"]["state"], "running");
        assert_eq!(initial["status"]["variables"], serde_json::json!([]));
        assert!(initial["status"].get("registers").is_none());
        session.command(MonitorRequest {
            schema_version: 1,
            session_id: 7,
            command: MonitorCommand::Release,
        })?;
        let request = |revision: &serde_json::Value| {
            serde_json::json!({"schemaVersion":1,"revision":revision,"command":{"operation":"cancel"}}).to_string()
        };
        assert!(
            session
                .debug_command(&request(&initial["revision"]))
                .is_err()
        );
        let current: serde_json::Value =
            serde_json::from_str(&session.debug_state()?.ok_or("missing debugger")?)
                .map_err(|e| e.to_string())?;
        let cancelled: serde_json::Value =
            serde_json::from_str(&session.debug_command(&request(&current["revision"]))?)
                .map_err(|e| e.to_string())?;
        assert_eq!(cancelled["backend"], "original");
        assert_eq!(cancelled["status"]["state"], "cancelled");
        for _ in 0..8 {
            session.command(MonitorRequest {
                schema_version: 1,
                session_id: 7,
                command: MonitorCommand::Advance { cycles: MAX_SLICE },
            })?;
        }
        let retained: serde_json::Value =
            serde_json::from_str(&session.debug_state()?.ok_or("missing debugger")?)
                .map_err(|e| e.to_string())?;
        assert_eq!(retained["status"], cancelled["status"]);
        session.debug_command(&serde_json::json!({"schemaVersion":1,"revision":retained["revision"],"command":{"operation":"continue"}}).to_string())?;
        for _ in 0..768 {
            session.command(MonitorRequest {
                schema_version: 1,
                session_id: 7,
                command: MonitorCommand::Advance { cycles: MAX_SLICE },
            })?;
            let state: serde_json::Value =
                serde_json::from_str(&session.debug_state()?.ok_or("missing debugger")?)
                    .map_err(|e| e.to_string())?;
            if state["status"]["state"] == "stopped" {
                assert_eq!(state["status"]["variables"][0]["numericValue"], 41.0);
                return Ok(());
            }
        }
        Err("Resumed normal boot did not reach the current build breakpoint".into())
    }

    #[test]
    fn executed_warm_reset_retains_screen_and_variables_without_rerunning_the_build()
    -> Result<(), String> {
        let mut session = fixture(None)?;
        let output42 = |text: &str| text.lines().any(|line| line.trim() == "42");
        for _ in 0..768 {
            if output42(&tick(&mut session)?.machine.text) {
                break;
            }
        }
        assert!(output42(&session.snapshot().machine.text));
        for _ in 0..32 {
            tick(&mut session)?;
        }
        session.command(MonitorRequest {
            schema_version: 1,
            session_id: session.id(),
            command: MonitorCommand::Type {
                text: "COUNT=87".into(),
            },
        })?;
        for _ in 0..32 {
            tick(&mut session)?;
        }
        session.command(MonitorRequest {
            schema_version: 1,
            session_id: session.id(),
            command: MonitorCommand::Reset,
        })?;
        for _ in 0..96 {
            tick(&mut session)?;
        }
        assert!(
            output42(&session.snapshot().machine.text),
            "Warm reset retains the old screen"
        );
        session.command(MonitorRequest {
            schema_version: 1,
            session_id: session.id(),
            command: MonitorCommand::Type {
                text: "PRINT \"RESETVALUE\";COUNT".into(),
            },
        })?;
        for _ in 0..128 {
            let frame = tick(&mut session)?;
            if frame
                .machine
                .text
                .lines()
                .any(|line| line.split_whitespace().collect::<Vec<_>>() == ["RESETVALUE", "87"])
            {
                return Ok(());
            }
        }
        Err("Warm reset lost COUNT87 or automatically reran the previous COUNT41 programme".into())
    }

    #[test]
    fn loading_is_bounded_rejects_interference_and_reset_cancels_automatic_execution()
    -> Result<(), String> {
        let mut session = fixture(None)?;
        let id = session.id();
        for (session_id, command) in [
            (id + 1, MonitorCommand::Advance { cycles: MAX_SLICE }),
            (id, MonitorCommand::Type { text: "RUN".into() }),
            (id, MonitorCommand::RewindCassette),
        ] {
            assert!(
                session
                    .command(MonitorRequest {
                        schema_version: 1,
                        session_id,
                        command
                    })
                    .is_err()
            );
        }
        if let DevelopmentSession::Original(owner) = &mut session {
            owner.limit = 0;
        }
        let failed = tick(&mut session)?;
        assert_eq!(failed.machine.state, "failed");
        assert!(failed.machine.message.contains("timed out"));
        let reset = session.command(MonitorRequest {
            schema_version: 1,
            session_id: id,
            command: MonitorCommand::Reset,
        })?;
        assert_ne!(reset.machine.state, "failed");
        for _ in 0..8 {
            tick(&mut session)?;
        }
        assert!(!session.snapshot().machine.text.contains("42"));
        let stopped = session.command(MonitorRequest {
            schema_version: 1,
            session_id: id,
            command: MonitorCommand::Stop,
        })?;
        assert_eq!(stopped.machine.state, "stopped");
        Ok(())
    }

    #[test]
    fn development_identity_cannot_admit_normal_execution() {
        // Retain this unconditional negative case after the bundled image qualifies.
        // Downgrading a fixture receipt never executes or admits its bytes.
        let mut development = receipt(crate::BASIC_EXTENDED);
        development["id"] = json!("eightbit-dragon32-basic-0.1-dev");
        development["status"] = json!("development");
        assert_eq!(
            admit(&development.to_string(), crate::BASIC_EXTENDED),
            Err(NOT_QUALIFIED.into())
        );
    }

    #[test]
    fn authored_qualification_receipt_requires_exact_identity_size_status_and_digest()
    -> Result<(), String> {
        // Generated data tests admission only; it is never executed or described
        // as qualified firmware. Production admission accepts no caller inputs.
        let image = vec![0x12; 32_768];
        let valid = receipt(&image);
        let profile = admit(&valid.to_string(), &image)?;
        assert_eq!(profile.profile_id(), QUALIFIED_PROFILE);
        assert_eq!(profile.firmware_sha256(), valid["romSha256"]);
        for (field, value) in [
            ("schemaVersion", json!(2)),
            ("id", json!("eightbit-dragon32-basic-0.1-dev")),
            ("status", json!("development")),
            ("licence", json!("unknown")),
            ("bytes", json!(32_767)),
            ("romSha256", json!("0".repeat(64))),
        ] {
            let mut malformed = valid.clone();
            malformed[field] = value;
            assert!(admit(&malformed.to_string(), &image).is_err(), "{field}");
        }
        for length in [0, 16_384, 20_107, 32_767, 32_769] {
            let invalid = vec![0; length];
            assert!(admit(&receipt(&invalid).to_string(), &invalid).is_err());
        }
        for (field, value) in [
            ("ramStart", json!(1)),
            ("ramEndExclusive", json!(65_536)),
            ("romStart", json!(0xc000)),
            ("romEndExclusive", json!(65_536)),
            ("ioStart", json!(0xff20)),
            ("ioEndExclusive", json!(0xffff)),
            ("vectorAliasStart", json!(0xfff0)),
            ("vectorAliasEndExclusive", json!(0xffff)),
            ("vectorAliasSourceStart", json!(0xffe0)),
            ("cartridges", json!(true)),
            ("samAllRam", json!("expanded")),
            ("unknown", json!(0)),
        ] {
            let mut malformed = valid.clone();
            malformed["memoryMap"][field] = value;
            assert!(admit(&malformed.to_string(), &image).is_err(), "{field}");
        }
        let mut missing_map = valid.clone();
        missing_map["memoryMap"] = json!(null);
        assert!(admit(&missing_map.to_string(), &image).is_err());
        let mut old_profile = valid.clone();
        old_profile["id"] = json!("eightbit-dragon32-basic-1");
        assert!(admit(&old_profile.to_string(), &image).is_err());
        assert!(admit("{}", &image).is_err());
        assert!(admit(&" ".repeat(32 * 1024 + 1), &image).is_err());
        Ok(())
    }
}
