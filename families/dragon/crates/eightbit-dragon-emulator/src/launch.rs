//! Versioned, explicit original-development launch; never a project default.
use crate::{Firmware, MAX_MONITOR_REQUEST_BYTES, MonitorCommand, MonitorRequest, MonitorSession};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum OriginalBackend {
    OriginalDevelopment,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum OriginalTarget {
    Dragon32,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OriginalCassette {
    #[serde(deserialize_with = "crate::session::cassette_bytes")]
    pub bytes: Vec<u8>,
    pub sha256: String,
    /// Omitted for an explicitly selected cassette rather than a project build.
    pub build_id: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OriginalDevelopmentLaunch {
    pub schema_version: u32,
    pub backend: OriginalBackend,
    pub target: OriginalTarget,
    pub cassette: Option<OriginalCassette>,
}
impl OriginalDevelopmentLaunch {
    #[must_use]
    pub fn prompt() -> Self {
        Self {
            schema_version: 1,
            backend: OriginalBackend::OriginalDevelopment,
            target: OriginalTarget::Dragon32,
            cassette: None,
        }
    }
    pub fn with_cassette(bytes: Vec<u8>, build_id: Option<String>) -> Result<Self, String> {
        let request = Self {
            cassette: Some(OriginalCassette {
                sha256: hex::encode(Sha256::digest(&bytes)),
                bytes,
                build_id,
            }),
            ..Self::prompt()
        };
        request.validate()?;
        Ok(request)
    }
    pub fn from_json(json: &str) -> Result<Self, String> {
        if json.len() > MAX_MONITOR_REQUEST_BYTES {
            return Err("DME1002: original launch request is too large".into());
        }
        let request: Self = serde_json::from_str(json)
            .map_err(|_| "DME1002: invalid original development launch".to_string())?;
        request.validate()?;
        Ok(request)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("DME1002: unsupported original launch schema".into());
        }
        if let Some(media) = &self.cassette {
            if media.bytes.is_empty() || media.bytes.len() > crate::MAX_SESSION_CASSETTE_BYTES {
                return Err("DME1005: original launch cassette exceeds its size limit".into());
            }
            let digest = |value: &str| {
                value.len() == 64
                    && value
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            };
            if !digest(&media.sha256)
                || media.build_id.as_ref().is_some_and(|value| !digest(value))
                || hex::encode(Sha256::digest(&media.bytes)) != media.sha256
            {
                return Err("DME1005: original launch cassette identity changed".into());
            }
            crate::cassette::Cassette::read(media.bytes.clone())?;
        }
        Ok(())
    }
    /// Validate before boot; mounting never types CLOAD/RUN or injects guest RAM.
    pub fn start(self, session_id: u32) -> Result<MonitorSession, String> {
        self.start_with_firmware(session_id, Firmware::BasicDevelopment)
    }
    // Only the admitted normal-runtime owner selects the expanded profile.
    // Serialised development launches cannot supply or override this selection.
    pub(crate) fn start_with_firmware(
        self,
        session_id: u32,
        firmware: Firmware,
    ) -> Result<MonitorSession, String> {
        self.validate()?;
        let mut session = if firmware == Firmware::BasicExtended || firmware.is_dragon64() {
            MonitorSession::unstarted(session_id, firmware)?
        } else {
            MonitorSession::with_firmware(session_id, firmware)?
        };
        if let Some(media) = self.cassette {
            session.command(MonitorRequest {
                schema_version: 1,
                session_id,
                command: MonitorCommand::MountCassette { bytes: media.bytes },
            })?;
        }
        Ok(session)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_schema_rejects_other_backends_targets_firmware_and_changed_bytes()
    -> Result<(), String> {
        let value =
            serde_json::to_value(OriginalDevelopmentLaunch::prompt()).map_err(|e| e.to_string())?;
        for (key, replacement) in [
            ("schemaVersion", serde_json::json!(2)),
            ("backend", serde_json::json!("xroar")),
            ("target", serde_json::json!("dragon64")),
            ("firmware", serde_json::json!("custom.rom")),
        ] {
            let mut invalid = value.clone();
            invalid[key] = replacement;
            assert!(OriginalDevelopmentLaunch::from_json(&invalid.to_string()).is_err());
        }
        let bytes = eightbit_dragon_media::create_machine_code("LAUNCH", &[0x39], 0x6000, 0x6000)
            .map_err(|e| e.to_string())?;
        let mut launch = OriginalDevelopmentLaunch::with_cassette(bytes, Some("a".repeat(64)))?;
        if let Some(media) = launch.cassette.as_mut() {
            media.bytes[0] ^= 1;
        }
        assert!(launch.start(1).is_err());
        Ok(())
    }
    #[test]
    fn validated_launch_mounts_without_executing_and_keeps_monitor_default() -> Result<(), String> {
        let bytes = eightbit_dragon_media::create_machine_code("LAUNCH", &[0x39], 0x6000, 0x6000)
            .map_err(|e| e.to_string())?;
        let before = MonitorSession::with_firmware(1, Firmware::BasicDevelopment)?.snapshot();
        let launch = OriginalDevelopmentLaunch::with_cassette(bytes, Some("a".repeat(64)))?;
        let serialised = serde_json::to_string(&launch).map_err(|e| e.to_string())?;
        let started = OriginalDevelopmentLaunch::from_json(&serialised)?
            .start(1)?
            .snapshot();
        assert_eq!(started.machine, before.machine);
        assert_eq!(
            started.cassette.ok_or("Missing cassette")?.filename,
            "LAUNCH"
        );
        assert_eq!(
            MonitorSession::new(2)?.snapshot().machine.firmware_id,
            "eightbit-dragon32-monitor-0.1"
        );
        Ok(())
    }
}
