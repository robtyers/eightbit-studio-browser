use crate::cassette::{Cassette, CassetteSnapshot};
use crate::recording::{Recording, RecordingExport, RecordingSnapshot};
use crate::{Firmware, Machine, Snapshot};

pub const MAX_MONITOR_REQUEST_BYTES: usize = crate::MAX_SESSION_CASSETTE_BYTES * 4 + 4096;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
pub enum MonitorCommand {
    Advance {
        cycles: u32,
    },
    State,
    Reset,
    Stop,
    Release,
    Key {
        code: String,
        pressed: bool,
    },
    Type {
        text: String,
    },
    MountCassette {
        #[serde(deserialize_with = "cassette_bytes")]
        bytes: Vec<u8>,
    },
    MountWav {
        #[serde(deserialize_with = "cassette_bytes")]
        bytes: Vec<u8>,
    },
    RewindCassette,
    EjectCassette,
    StartRecording,
    FinishRecording,
    DiscardRecording,
    ExportRecording,
}
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MonitorRequest {
    pub schema_version: u32,
    pub session_id: u32,
    pub command: MonitorCommand,
}
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshot {
    pub session_id: u32,
    #[serde(flatten)]
    pub machine: Snapshot,
    pub audio_sample_rate: u32,
    pub audio: Vec<i16>,
    pub cassette: Option<CassetteSnapshot>,
    pub recording: RecordingSnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recording_export: Option<RecordingExport>,
}

pub struct MonitorSession {
    id: u32,
    machine: Machine,
    stopped: bool,
    cassette: Option<Cassette>,
    recording: Recording,
    recording_drops: u64,
}
impl MonitorSession {
    pub fn new(id: u32) -> Result<Self, String> {
        Self::with_firmware(id, Firmware::Monitor)
    }
    pub fn with_firmware(id: u32, firmware: Firmware) -> Result<Self, String> {
        let mut session = Self::unstarted(id, firmware)?;
        session.machine.advance(crate::MAX_SLICE)?;
        session.machine.take_audio();
        Ok(session)
    }
    // Normal launch owns every boot step so its public prompt hooks cannot pass
    // before the bounded cassette-load controller has been installed.
    pub(crate) fn unstarted(id: u32, firmware: Firmware) -> Result<Self, String> {
        if id == 0 {
            return Err("DME1004: invalid monitor session".into());
        }
        Ok(Self {
            id,
            machine: Machine::with_firmware(firmware),
            stopped: false,
            cassette: None,
            recording: Recording::default(),
            recording_drops: 0,
        })
    }
    #[must_use]
    pub fn snapshot(&self) -> SessionSnapshot {
        let mut machine = self.machine.snapshot();
        if self.stopped {
            machine.state = "stopped";
            machine.message = "Monitor stopped".into();
        }
        SessionSnapshot {
            session_id: self.id,
            machine,
            audio_sample_rate: crate::AUDIO_HZ,
            audio: Vec::new(),
            recording: self.recording.snapshot(),
            recording_export: None,
            cassette: self.cassette.as_ref().map(|cassette| {
                let mut receipt = cassette.receipt.clone();
                receipt.pending = self.machine.cassette_pending();
                receipt
            }),
        }
    }
    #[must_use]
    pub fn id(&self) -> u32 {
        self.id
    }
    pub fn export_recording(&self) -> Result<RecordingExport, String> {
        if self.stopped {
            return Err("DME1004: monitor session is stale or stopped".into());
        }
        self.recording.export()
    }
    fn drain_recording(&mut self) {
        let dropped = self.machine.board.io.dropped_tape != self.recording_drops;
        self.recording
            .append(self.machine.take_cassette_recording(), dropped);
        self.recording_drops = self.machine.board.io.dropped_tape;
    }
    pub(crate) fn debug_machine(&self) -> Result<&Machine, String> {
        if self.stopped || !self.machine.board.profile.is_basic() {
            return Err("DME1302: BASIC debugging requires an active BASIC session".into());
        }
        Ok(&self.machine)
    }
    pub(crate) fn debug_machine_mut(&mut self) -> &mut Machine {
        &mut self.machine
    }
    pub(crate) fn debug_frame(&mut self) -> SessionSnapshot {
        self.drain_recording();
        let mut frame = self.snapshot();
        frame.audio = self.machine.take_audio();
        frame
    }
    pub fn command(&mut self, request: MonitorRequest) -> Result<SessionSnapshot, String> {
        if request.schema_version != 1 || request.session_id != self.id || self.stopped {
            return Err("DME1004: monitor session is stale or stopped".into());
        }
        let advance = matches!(&request.command, MonitorCommand::Advance { .. });
        let export = matches!(&request.command, MonitorCommand::ExportRecording);
        let wav = matches!(&request.command, MonitorCommand::MountWav { .. });
        match request.command {
            MonitorCommand::Advance { cycles } => {
                if cycles == 0 || cycles > crate::MAX_SLICE {
                    return Err("DME1002: invalid execution slice".into());
                }
                let _result = self.machine.advance(cycles);
                self.drain_recording();
            }
            MonitorCommand::State | MonitorCommand::ExportRecording => {}
            MonitorCommand::Reset => {
                if self.recording.active() {
                    self.recording.failed(
                        "Reset cancelled cassette recording. Discard it before recording again.",
                    );
                }
                self.machine.reset();
            }
            MonitorCommand::Stop => {
                self.machine.cancel_input();
                self.machine.take_audio();
                self.machine.eject_cassette();
                self.cassette = None;
                self.recording = Recording::default();
                self.stopped = true;
            }
            MonitorCommand::Release => self.machine.release_keys(),
            MonitorCommand::Key { code, pressed } => {
                if code.len() > 32 {
                    return Err("DME1003: invalid key".into());
                }
                self.machine.key(&code, pressed)?;
            }
            MonitorCommand::Type { text } => self.machine.type_text(&text)?,
            MonitorCommand::MountCassette { bytes } | MonitorCommand::MountWav { bytes } => {
                if !self.machine.board.profile.is_basic() {
                    return Err(
                        "DME1005: cassette loading requires an explicit BASIC profile".into(),
                    );
                }
                let cassette = if wav {
                    Cassette::read_wav(&bytes)?
                } else {
                    Cassette::read(bytes)?
                };
                self.machine.mount_cassette_signal(cassette.signal());
                self.cassette = Some(cassette);
            }
            MonitorCommand::RewindCassette => {
                let cassette = self
                    .cassette
                    .as_ref()
                    .ok_or_else(|| "DME1005: no cassette is mounted".to_string())?;
                self.machine.mount_cassette_signal(cassette.signal());
            }
            MonitorCommand::EjectCassette => {
                self.machine.eject_cassette();
                self.cassette = None;
            }
            MonitorCommand::StartRecording => {
                if !self.machine.board.profile.is_basic() {
                    return Err(
                        "DME1005: cassette recording requires an explicit BASIC profile".into(),
                    );
                }
                self.recording.start()?;
                self.machine.take_cassette_recording();
                self.recording_drops = self.machine.board.io.dropped_tape;
            }
            MonitorCommand::FinishRecording => {
                self.drain_recording();
                self.recording.finish()?;
            }
            MonitorCommand::DiscardRecording => {
                self.recording = Recording::default();
            }
        }
        let mut snapshot = self.snapshot();
        if export {
            snapshot.recording_export = Some(self.export_recording()?);
        }
        if advance {
            snapshot.audio = self.machine.take_audio();
        }
        Ok(snapshot)
    }
    pub fn command_json(&mut self, json: &str) -> Result<SessionSnapshot, String> {
        if json.len() > MAX_MONITOR_REQUEST_BYTES {
            return Err("DME1002: monitor request is too large".into());
        }
        let request: MonitorRequest = serde_json_parse(json)?;
        self.command(request)
    }
}
pub(crate) fn cassette_bytes<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<u8>, D::Error> {
    struct Bytes;
    impl<'de> serde::de::Visitor<'de> for Bytes {
        type Value = Vec<u8>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("at most 1048576 cassette bytes")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut sequence: A,
        ) -> Result<Self::Value, A::Error> {
            let mut bytes = Vec::with_capacity(
                sequence
                    .size_hint()
                    .unwrap_or(0)
                    .min(crate::MAX_SESSION_CASSETTE_BYTES),
            );
            while let Some(byte) = sequence.next_element::<u8>()? {
                if bytes.len() == crate::MAX_SESSION_CASSETTE_BYTES {
                    return Err(serde::de::Error::custom(
                        "cassette exceeds the session limit",
                    ));
                }
                bytes.push(byte);
            }
            Ok(bytes)
        }
    }
    deserializer.deserialize_seq(Bytes)
}
fn serde_json_parse(json: &str) -> Result<MonitorRequest, String> {
    serde_json::from_str(json).map_err(|_| "DME1002: invalid monitor request".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn audio_is_delivered_once_by_bounded_advance_and_not_by_state_reads() -> Result<(), String> {
        let mut session = MonitorSession::new(5)?;
        let request = |command| MonitorRequest {
            schema_version: 1,
            session_id: 5,
            command,
        };
        let frame = session.command(request(MonitorCommand::Advance { cycles: 17920 }))?;
        assert_eq!(frame.audio_sample_rate, 48_000);
        assert!(!frame.audio.is_empty());
        assert!(frame.audio.len() <= 4096);
        assert!(
            session
                .command(request(MonitorCommand::State))?
                .audio
                .is_empty()
        );
        assert!(
            session
                .command(request(MonitorCommand::Reset))?
                .audio
                .is_empty()
        );
        assert!(
            session
                .command(request(MonitorCommand::Stop))?
                .audio
                .is_empty()
        );
        assert!(
            session
                .command(request(MonitorCommand::Advance { cycles: 17920 }))
                .is_err()
        );
        Ok(())
    }
    #[test]
    fn sessions_reject_stale_schema_and_recover_after_failure() -> Result<(), String> {
        let mut s = MonitorSession::new(4)?;
        assert!(
            s.command_json(r#"{"schemaVersion":1,"sessionId":3,"command":{"operation":"reset"}}"#)
                .is_err()
        );
        assert!(
            s.command_json(r#"{"schemaVersion":2,"sessionId":4,"command":{"operation":"reset"}}"#)
                .is_err()
        );
        assert!(s.command_json(r#"{"schemaVersion":1,"sessionId":4,"command":{"operation":"advance","cycles":40000}}"#).is_err());
        assert!(
            s.command_json(
                r#"{"schemaVersion":1,"sessionId":4,"command":{"operation":"state"},"path":"bad"}"#
            )
            .is_err()
        );
        assert!(
            s.command_json(r#"{"schemaVersion":1,"sessionId":4,"command":{"operation":"stop"}}"#)
                .is_ok()
        );
        assert!(
            s.command_json(r#"{"schemaVersion":1,"sessionId":4,"command":{"operation":"state"}}"#)
                .is_err()
        );
        Ok(())
    }
}

#[cfg(test)]
mod cassette_tests {
    use super::*;
    use crate::MAX_SLICE;
    fn request(command: MonitorCommand) -> MonitorRequest {
        MonitorRequest {
            schema_version: 1,
            session_id: 9,
            command,
        }
    }
    fn original_cassette() -> Result<Vec<u8>, String> {
        eightbit_dragon_media::create_machine_code(
            "NATIVE",
            &[0xcc, 0, 165, 0xfd, 0x63, 0, 0x39],
            0x6000,
            0x6000,
        )
        .map_err(|error| error.to_string())
    }
    #[test]
    fn cassette_deserialisation_rejects_oversized_arrays_and_non_bytes() {
        for bytes in [
            serde_json::json!([256]),
            serde_json::json!([-1]),
            serde_json::json!([0.5]),
            serde_json::json!(vec![0; crate::MAX_SESSION_CASSETTE_BYTES + 1]),
        ] {
            for operation in ["mount-cassette", "mount-wav"] {
                let input = serde_json::json!({"schemaVersion": 1, "sessionId": 9, "command": {"operation": operation, "bytes": bytes}});
                assert!(serde_json::from_value::<MonitorRequest>(input).is_err());
            }
        }
    }
    #[test]
    fn cassette_mount_is_explicit_atomic_and_owned_by_the_development_lifetime()
    -> Result<(), String> {
        let bytes = original_cassette()?;
        let mut monitor = MonitorSession::new(9)?;
        assert!(
            monitor
                .command(request(MonitorCommand::MountCassette {
                    bytes: bytes.clone()
                }))
                .is_err()
        );
        let mut session = MonitorSession::with_firmware(9, Firmware::BasicDevelopment)?;
        let before = session.machine.snapshot();
        let mounted = session.command(request(MonitorCommand::MountCassette { bytes }))?;
        assert_eq!(
            mounted.machine, before,
            "Mounting must not execute the guest"
        );
        assert!(session.machine.cassette_pending());
        let identity = mounted.cassette.ok_or("Missing cassette receipt")?.sha256;
        assert!(
            session
                .command(request(MonitorCommand::MountCassette {
                    bytes: vec![0x55; 128]
                }))
                .is_err()
        );
        assert_eq!(
            session
                .snapshot()
                .cassette
                .ok_or("Lost original cassette")?
                .sha256,
            identity
        );
        assert_eq!(session.machine.snapshot(), before);
        assert!(
            session
                .command(MonitorRequest {
                    schema_version: 1,
                    session_id: 8,
                    command: MonitorCommand::EjectCassette
                })
                .is_err()
        );
        assert!(session.snapshot().cassette.is_some());
        session.command(request(MonitorCommand::Reset))?;
        assert!(session.snapshot().cassette.is_some());
        session.command(request(MonitorCommand::EjectCassette))?;
        assert!(!session.machine.cassette_pending());
        assert!(session.snapshot().cassette.is_none());
        assert!(
            session
                .command(request(MonitorCommand::RewindCassette))
                .is_err()
        );
        session.command(request(MonitorCommand::MountCassette {
            bytes: original_cassette()?,
        }))?;
        let stopped = session.command(request(MonitorCommand::Stop))?;
        assert!(stopped.cassette.is_none());
        assert!(!session.machine.cassette_pending());
        assert!(
            session
                .command(request(MonitorCommand::RewindCassette))
                .is_err()
        );
        Ok(())
    }
    #[test]
    fn checked_project_tokens_and_ascii_load_then_run_in_the_guest() -> Result<(), String> {
        use eightbit_dragon_basic_core::{Dialect, analyse, tokenise};
        let source = "10 PRINT\"ORIGINAL LOAD\"\n20 END\n";
        let dialect = Dialect::dragon32().map_err(|error| error.to_string())?;
        let tokens = tokenise(&analyse(source, &dialect).program, &dialect)
            .map_err(|error| error.to_string())?;
        let tokenised = eightbit_dragon_media::create_tokenised_basic("ORIGIN", &tokens.bytes)
            .map_err(|error| error.to_string())?;
        let ascii = eightbit_dragon_media::create_ascii_basic("ORIGIN", source)
            .map_err(|error| error.to_string())?;
        for bytes in [tokenised, ascii] {
            let mut session = MonitorSession::with_firmware(9, Firmware::BasicDevelopment)?;
            for _ in 0..8 {
                session.command(request(MonitorCommand::Advance { cycles: MAX_SLICE }))?;
            }
            session.command(request(MonitorCommand::MountCassette { bytes }))?;
            session.command(request(MonitorCommand::Type {
                text: "CLOAD\"ORIGIN\"".into(),
            }))?;
            for _ in 0..256 {
                session.command(request(MonitorCommand::Advance { cycles: MAX_SLICE }))?;
            }
            assert!(!session.snapshot().machine.text.contains("ERROR"));
            session.command(request(MonitorCommand::Type { text: "RUN".into() }))?;
            for _ in 0..64 {
                session.command(request(MonitorCommand::Advance { cycles: MAX_SLICE }))?;
            }
            assert!(session.snapshot().machine.text.contains("ORIGINAL LOAD"));
            assert!(!session.snapshot().machine.text.contains("ERROR"));
        }
        Ok(())
    }
    #[test]
    fn cassette_json_mount_loads_through_the_guest_pia_and_rewinds() -> Result<(), String> {
        let mut session = MonitorSession::with_firmware(9, Firmware::BasicDevelopment)?;
        for _ in 0..8 {
            session.command(request(MonitorCommand::Advance { cycles: MAX_SLICE }))?;
        }
        let mount = request(MonitorCommand::MountCassette {
            bytes: original_cassette()?,
        });
        let json = serde_json::to_string(&mount).map_err(|error| error.to_string())?;
        assert!(
            json.len() > 1024,
            "Exercise the expanded, bounded media envelope"
        );
        session.command_json(&json)?;
        session.command(request(MonitorCommand::Type {
            text: "CLOADM\"NATIVE\":EXEC:PRINT\"CASSETTE READY\"".into(),
        }))?;
        for _ in 0..256 {
            session.command(request(MonitorCommand::Advance { cycles: MAX_SLICE }))?;
        }
        assert_eq!(session.machine.peek(0x6301), 165);
        assert!(session.snapshot().machine.text.contains("CASSETTE READY"));
        assert_eq!(
            (session.machine.peek(0x9d), session.machine.peek(0x9e)),
            (0x60, 0)
        );
        session.command(request(MonitorCommand::RewindCassette))?;
        assert!(
            session
                .snapshot()
                .cassette
                .ok_or("Missing rewound tape")?
                .pending
        );
        assert!(
            session
                .command_json(&" ".repeat(MAX_MONITOR_REQUEST_BYTES + 1))
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn wav_mount_is_atomic_bounded_and_loads_only_through_guest_pins() -> Result<(), String> {
        let cas = original_cassette()?;
        let wav = eightbit_dragon_media::encode_cassette_transport_wav(&cas)
            .map_err(|error| error.to_string())?;
        assert!(wav.len() <= crate::MAX_SESSION_CASSETTE_BYTES);
        let mut monitor = MonitorSession::new(9)?;
        assert!(
            monitor
                .command(request(MonitorCommand::MountWav { bytes: wav.clone() }))
                .is_err()
        );
        let mut session = MonitorSession::with_firmware(9, Firmware::BasicDevelopment)?;
        for _ in 0..8 {
            session.command(request(MonitorCommand::Advance { cycles: MAX_SLICE }))?;
        }
        let before = session.machine.snapshot();
        let json = serde_json::to_string(&request(MonitorCommand::MountWav { bytes: wav }))
            .map_err(|error| error.to_string())?;
        let mounted = session.command_json(&json)?;
        assert_eq!(mounted.machine, before);
        let identity = mounted.cassette.ok_or("Missing audio cassette")?.sha256;
        assert_eq!(identity, Cassette::read(cas.clone())?.receipt.sha256);
        assert_eq!(session.machine.peek(0x6301), 0);
        for invalid in [
            Vec::new(),
            cas,
            b"RIFFbroken".to_vec(),
            vec![0; crate::MAX_SESSION_CASSETTE_BYTES + 1],
        ] {
            assert!(
                session
                    .command(request(MonitorCommand::MountWav { bytes: invalid }))
                    .is_err()
            );
            assert_eq!(
                session
                    .snapshot()
                    .cassette
                    .ok_or("Lost audio cassette")?
                    .sha256,
                identity
            );
            assert_eq!(session.machine.snapshot(), before);
        }
        session.command(request(MonitorCommand::Type {
            text: "CLOADM\"NATIVE\":EXEC:PRINT\"WAV READY\"".into(),
        }))?;
        for _ in 0..256 {
            session.command(request(MonitorCommand::Advance { cycles: MAX_SLICE }))?;
        }
        assert_eq!(session.machine.peek(0x6301), 165);
        assert!(session.snapshot().machine.text.contains("WAV READY"));
        session.command(request(MonitorCommand::RewindCassette))?;
        assert!(
            session
                .snapshot()
                .cassette
                .ok_or("Lost rewound audio")?
                .pending
        );
        session.command(request(MonitorCommand::Stop))?;
        assert!(
            session
                .command(request(MonitorCommand::MountWav { bytes: Vec::new() }))
                .is_err()
        );
        Ok(())
    }
}

#[cfg(test)]
mod recording_tests {
    use super::*;
    use crate::MAX_SLICE;
    fn command(
        session: &mut MonitorSession,
        command: MonitorCommand,
    ) -> Result<SessionSnapshot, String> {
        session.command(MonitorRequest {
            schema_version: 1,
            session_id: session.id(),
            command,
        })
    }
    fn advance(session: &mut MonitorSession, slices: u32) -> Result<(), String> {
        for _ in 0..slices {
            command(session, MonitorCommand::Advance { cycles: MAX_SLICE })?;
        }
        Ok(())
    }
    #[test]
    fn two_guest_saves_capture_and_filename_search_loads_the_second_file() -> Result<(), String> {
        two_guest_saves(Firmware::BasicDevelopment)
    }
    #[test]
    fn expanded_two_guest_saves_reload_through_pia_with_owned_recording() -> Result<(), String> {
        two_guest_saves(Firmware::BasicExtended)
    }
    #[test]
    fn dragon64_two_guest_saves_reload_in_both_modes() -> Result<(), String> {
        two_guest_saves(Firmware::Dragon64Compatibility)?;
        two_guest_saves(Firmware::Dragon64Native)
    }
    fn two_guest_saves(profile: Firmware) -> Result<(), String> {
        let mut session = MonitorSession::with_firmware(8, profile)?;
        advance(&mut session, 8)?;
        command(
            &mut session,
            MonitorCommand::Type {
                text: "10 PRINT\"MULTITAPE READY\"".into(),
            },
        )?;
        advance(&mut session, 64)?;
        command(&mut session, MonitorCommand::StartRecording)?;
        command(
            &mut session,
            MonitorCommand::Type {
                text: "CSAVE\"FIRST\":CSAVE\"SECOND\"".into(),
            },
        )?;
        advance(&mut session, 768)?;
        let finished = command(&mut session, MonitorCommand::FinishRecording)?;
        assert_eq!(
            finished.recording.state, "ready",
            "{}",
            finished.recording.message
        );
        assert_eq!(
            finished
                .recording
                .cassette
                .ok_or("Missing cassette")?
                .format,
            "cassette-stream"
        );
        let exported = session.export_recording()?;
        let mut next = MonitorSession::with_firmware(9, profile)?;
        advance(&mut next, 8)?;
        command(
            &mut next,
            MonitorCommand::MountCassette {
                bytes: exported.bytes.clone(),
            },
        )?;
        let identity = next.snapshot().cassette.ok_or("Missing cassette")?.sha256;
        let mut malformed = exported.bytes;
        malformed.pop();
        malformed.pop();
        for bytes in [malformed, vec![0; crate::MAX_SESSION_CASSETTE_BYTES + 1]] {
            assert!(command(&mut next, MonitorCommand::MountCassette { bytes }).is_err());
            assert_eq!(
                next.snapshot().cassette.ok_or("Lost cassette")?.sha256,
                identity
            );
        }
        assert!(
            next.command(MonitorRequest {
                schema_version: 1,
                session_id: 8,
                command: MonitorCommand::MountCassette { bytes: vec![0] }
            })
            .is_err()
        );
        assert_eq!(
            next.snapshot().cassette.ok_or("Lost cassette")?.sha256,
            identity
        );
        command(
            &mut next,
            MonitorCommand::Type {
                text: "CLOAD\"SECOND\"".into(),
            },
        )?;
        advance(&mut next, 768)?;
        command(&mut next, MonitorCommand::Type { text: "RUN".into() })?;
        advance(&mut next, 64)?;
        assert!(next.snapshot().machine.text.contains("MULTITAPE READY"));
        command(&mut next, MonitorCommand::Reset)?;
        assert_eq!(
            next.snapshot()
                .cassette
                .ok_or("Lost cassette on reset")?
                .sha256,
            identity
        );
        command(&mut next, MonitorCommand::EjectCassette)?;
        assert!(next.snapshot().cassette.is_none());
        command(&mut next, MonitorCommand::Stop)?;
        assert!(next.snapshot().cassette.is_none());
        Ok(())
    }
    #[test]
    fn guest_csave_and_csavem_capture_reload_in_fresh_guest_without_memory_shortcuts()
    -> Result<(), String> {
        for (save, load) in [
            ("CSAVE\"CAPTURE\"", "CLOAD\"CAPTURE\""),
            ("CSAVE\"CAPTURE\",A", "CLOAD\"CAPTURE\""),
            (
                "POKE24576,57:CSAVEM\"CAPTURE\",24576,24576,24576",
                "CLOADM\"CAPTURE\"",
            ),
        ] {
            let mut session = MonitorSession::with_firmware(8, Firmware::BasicDevelopment)?;
            advance(&mut session, 8)?;
            command(
                &mut session,
                MonitorCommand::Type {
                    text: "10 PRINT\"CAPTURE ROUNDTRIP\"".into(),
                },
            )?;
            advance(&mut session, 64)?;
            command(&mut session, MonitorCommand::StartRecording)?;
            command(&mut session, MonitorCommand::Type { text: save.into() })?;
            advance(&mut session, 384)?;
            let finished = command(&mut session, MonitorCommand::FinishRecording)?;
            assert_eq!(
                finished.recording.state, "ready",
                "{save}: {}",
                finished.recording.message
            );
            let exported = command(&mut session, MonitorCommand::ExportRecording)?
                .recording_export
                .ok_or("Missing cassette export")?;
            assert_eq!(exported.filename, "dragon-recording.cas");
            let mut next = MonitorSession::with_firmware(9, Firmware::BasicDevelopment)?;
            advance(&mut next, 8)?;
            command(
                &mut next,
                MonitorCommand::MountCassette {
                    bytes: exported.bytes,
                },
            )?;
            command(&mut next, MonitorCommand::Type { text: load.into() })?;
            advance(&mut next, 384)?;
            assert!(!next.snapshot().machine.text.contains("ERROR"));
            let run = if load.starts_with("CLOADM") {
                "EXEC:PRINT\"CAPTURE ROUNDTRIP\""
            } else {
                "RUN"
            };
            command(&mut next, MonitorCommand::Type { text: run.into() })?;
            advance(&mut next, 64)?;
            assert!(next.snapshot().machine.text.contains("CAPTURE ROUNDTRIP"));
            assert!(!next.snapshot().machine.text.contains("ERROR"));
        }
        Ok(())
    }
    #[test]
    fn recording_reset_stop_and_stale_commands_preserve_session_ownership() -> Result<(), String> {
        let mut session = MonitorSession::with_firmware(9, Firmware::BasicDevelopment)?;
        command(&mut session, MonitorCommand::StartRecording)?;
        assert!(
            session
                .command(MonitorRequest {
                    schema_version: 1,
                    session_id: 8,
                    command: MonitorCommand::DiscardRecording
                })
                .is_err()
        );
        assert_eq!(session.snapshot().recording.state, "recording");
        command(&mut session, MonitorCommand::Reset)?;
        assert_eq!(session.snapshot().recording.state, "failed");
        assert!(session.export_recording().is_err());
        command(&mut session, MonitorCommand::DiscardRecording)?;
        command(&mut session, MonitorCommand::StartRecording)?;
        command(&mut session, MonitorCommand::DiscardRecording)?;
        assert_eq!(session.snapshot().recording.state, "idle");
        assert!(session.export_recording().is_err());
        command(&mut session, MonitorCommand::StartRecording)?;
        command(&mut session, MonitorCommand::Stop)?;
        assert_eq!(session.snapshot().recording.state, "idle");
        assert!(session.export_recording().is_err());
        Ok(())
    }
}
