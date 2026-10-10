//! Process-free Dragon 32 authoring experiment for an eventual iPad host.
//!
//! This is a single-document feasibility API, not a project-format replacement,
//! emulator, firmware store or registered mobile product. All BASIC and cassette
//! rules remain in the existing authoritative family libraries.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::panic))]

use eightbit_dragon_basic_core::{
    Diagnostic, Dialect, DialectError, MAX_SOURCE_BYTES, TokenError, analyse, tokenise,
};
use eightbit_dragon_media::{CassetteError, create_tokenised_basic};
use eightbit_dragon_targets::{BASIC_ROM_SHA256, DRAGON32, EXTENDED_BASIC_ROM_BYTES};
use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Failure at the experimental authoring boundary. Messages contain no source or firmware.
#[derive(Debug, Error)]
pub enum MobileError {
    /// The selected document revision is no longer current, or has never been set.
    #[error("DSI1000: select the current document revision before building or editing")]
    Revision,
    /// An edit exceeds the family's existing source limit.
    #[error("DSI1001: source exceeds the {MAX_SOURCE_BYTES}-byte limit")]
    SourceTooLarge,
    /// Source diagnostics prevent publication of a build.
    #[error("DSI1002: correct the BASIC diagnostics before building")]
    Diagnostics {
        /// Existing stable family diagnostic codes and source ranges.
        diagnostics: Vec<Diagnostic>,
    },
    /// The built-in language contract could not be loaded.
    #[error("DSI1003: cannot load the built-in Dragon 32 BASIC dialect")]
    Dialect(#[source] DialectError),
    /// Existing token conversion or RAM-boundary failure.
    #[error(transparent)]
    Tokens(#[from] TokenError),
    /// Existing cassette-format failure.
    #[error(transparent)]
    Cassette(#[from] CassetteError),
    /// A supplied firmware image has the wrong size for this exact target.
    #[error("DSI1004: select a 16384-byte headerless Dragon 32 BASIC ROM")]
    FirmwareSize,
    /// A closed session cannot accept late client requests.
    #[error("DSI1005: create a new authoring session after closing the document")]
    Closed,
}

/// Path-free evidence for a successful, current single-document build.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildSummary {
    /// Experimental summary contract version, distinct from project/build schemas.
    pub schema_version: u32,
    /// Exact family-owned target identity; this does not advertise runtime capabilities.
    pub target: &'static str,
    /// Revision supplied explicitly by the authoring client.
    pub source_revision: u64,
    /// Digest of the exact source, including its original line endings.
    pub source_sha256: String,
    /// Digest of the linked-line token stream.
    pub tokens_sha256: String,
    /// Digest of the generated cassette.
    pub cassette_sha256: String,
    /// Linked-line stream size, including its terminator.
    pub token_bytes: usize,
    /// Generated cassette size.
    pub cassette_bytes: usize,
}

/// Current build bytes remain native data, separate from the serialisable receipt.
pub struct BuiltDocument {
    /// Revision-bound, path-free evidence.
    pub summary: BuildSummary,
    /// Token stream produced by the existing BASIC core.
    pub tokens: Vec<u8>,
    /// Cassette produced by the existing media core, with the fixed name `IPADTEST`.
    pub cassette: Vec<u8>,
    /// Non-error family diagnostics retained for presentation.
    pub diagnostics: Vec<Diagnostic>,
}

/// One bounded authoring document; edits and failed builds invalidate its artefact.
pub struct AuthoringSession {
    dialect: Dialect,
    revision: u64,
    source: String,
    built: Option<BuiltDocument>,
    closed: bool,
}

impl AuthoringSession {
    /// Create an inert session with no document, firmware or executable authority.
    pub fn new() -> Result<Self, MobileError> {
        Ok(Self {
            dialect: Dialect::dragon32().map_err(MobileError::Dialect)?,
            revision: 0,
            source: String::new(),
            built: None,
            closed: false,
        })
    }

    /// Accept a newer revision and invalidate any old successful artefact.
    ///
    /// A rejected edit leaves the current document and its matching build intact.
    pub fn edit(&mut self, revision: u64, source: String) -> Result<(), MobileError> {
        if self.closed {
            return Err(MobileError::Closed);
        }
        if revision <= self.revision {
            return Err(MobileError::Revision);
        }
        if source.len() > MAX_SOURCE_BYTES {
            return Err(MobileError::SourceTooLarge);
        }
        self.revision = revision;
        self.source = source;
        self.built = None;
        Ok(())
    }

    /// Build the explicitly selected revision without filesystem or process access.
    pub fn build(&mut self, revision: u64) -> Result<BuildSummary, MobileError> {
        if self.closed {
            return Err(MobileError::Closed);
        }
        if revision == 0 || revision != self.revision {
            return Err(MobileError::Revision);
        }
        // Clear first: even a conversion failure must not leave a launchable old success.
        self.built = None;
        let analysis = analyse(&self.source, &self.dialect);
        if analysis.has_errors() {
            return Err(MobileError::Diagnostics {
                diagnostics: analysis.diagnostics,
            });
        }
        let tokens = tokenise(&analysis.program, &self.dialect)?.bytes;
        let cassette = create_tokenised_basic("IPADTEST", &tokens)?;
        let summary = BuildSummary {
            schema_version: 1,
            target: DRAGON32.id,
            source_revision: revision,
            source_sha256: sha256(self.source.as_bytes()),
            tokens_sha256: sha256(&tokens),
            cassette_sha256: sha256(&cassette),
            token_bytes: tokens.len(),
            cassette_bytes: cassette.len(),
        };
        self.built = Some(BuiltDocument {
            summary: summary.clone(),
            tokens,
            cassette,
            diagnostics: analysis.diagnostics,
        });
        Ok(summary)
    }

    /// Return only a successful artefact for the current document revision.
    #[must_use]
    pub fn current_build(&self) -> Option<&BuiltDocument> {
        self.built.as_ref()
    }

    /// Discard the document and reject every late edit/build after session closure.
    /// Reopening requires a new session, rather than reusing a closed instance.
    pub fn close(&mut self) {
        self.source = String::new();
        self.built = None;
        self.closed = true;
    }
}

/// Inspection only: neither the receipt nor its Debug output includes ROM bytes.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareInspection {
    /// Experimental receipt contract version.
    pub schema_version: u32,
    /// Exact supplied-image digest, without retaining or forwarding the image.
    pub sha256: String,
    /// Whether the digest matches the existing family's verified Dragon 32 profile.
    /// This establishes neither redistribution permission nor mobile runtime support.
    pub matches_verified_dragon32: bool,
}

/// Inspect explicitly supplied bytes using family-owned size and exact-hash constants.
///
/// No image is read from disk, stored, logged, downloaded or executed by this API.
pub fn inspect_firmware(bytes: &[u8]) -> Result<FirmwareInspection, MobileError> {
    if u64::try_from(bytes.len()).ok() != Some(EXTENDED_BASIC_ROM_BYTES) {
        return Err(MobileError::FirmwareSize);
    }
    let sha256 = sha256(bytes);
    Ok(FirmwareInspection {
        schema_version: 1,
        matches_verified_dragon32: sha256 == BASIC_ROM_SHA256,
        sha256,
    })
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use eightbit_dragon_basic_core::detokenise;
    use eightbit_dragon_media::read_tokenised_basic;

    const SOURCE: &str = "10 PRINT \"HELLO IPAD\"\n20 GOTO 10\n";

    #[test]
    fn builds_deterministically_and_round_trips_through_authoritative_codecs() {
        let mut session = AuthoringSession::new().unwrap();
        session.edit(1, SOURCE.into()).unwrap();
        let first = session.build(1).unwrap();
        assert_eq!(first, session.build(1).unwrap());
        assert_eq!(first.schema_version, 1);
        assert_eq!(first.target, DRAGON32.id);
        let built = session.current_build().unwrap();
        let decoded = read_tokenised_basic(&built.cassette).unwrap();
        assert_eq!(decoded.filename, "IPADTEST");
        assert_eq!(decoded.program, built.tokens);
        assert_eq!(
            detokenise(&decoded.program, &session.dialect)
                .unwrap()
                .source,
            SOURCE
        );
    }

    #[test]
    fn stale_edits_and_build_requests_cannot_replace_current_state() {
        let mut session = AuthoringSession::new().unwrap();
        assert!(matches!(session.build(0), Err(MobileError::Revision)));
        session.edit(2, SOURCE.into()).unwrap();
        let built = session.build(2).unwrap();
        assert!(matches!(
            session.edit(1, "10 END\n".into()),
            Err(MobileError::Revision)
        ));
        assert!(matches!(
            session.edit(2, "10 END\n".into()),
            Err(MobileError::Revision)
        ));
        assert!(matches!(session.build(1), Err(MobileError::Revision)));
        assert_eq!(session.current_build().unwrap().summary, built);
    }

    #[test]
    fn invalid_source_discards_old_success_and_recovers_after_a_new_edit() {
        let mut session = AuthoringSession::new().unwrap();
        session.edit(1, SOURCE.into()).unwrap();
        session.build(1).unwrap();
        session.edit(2, "10 PRINT \"unterminated\n".into()).unwrap();
        assert!(session.current_build().is_none());
        let Err(MobileError::Diagnostics { diagnostics }) = session.build(2) else {
            panic!("invalid source must fail with family diagnostics");
        };
        assert!(!diagnostics.is_empty());
        assert!(session.current_build().is_none());
        session.edit(3, SOURCE.into()).unwrap();
        assert_eq!(session.build(3).unwrap().source_revision, 3);
    }

    #[test]
    fn source_limits_are_checked_before_replacing_the_document() {
        let mut session = AuthoringSession::new().unwrap();
        session.edit(1, SOURCE.into()).unwrap();
        let built = session.build(1).unwrap();
        assert!(matches!(
            session.edit(2, "x".repeat(MAX_SOURCE_BYTES + 1)),
            Err(MobileError::SourceTooLarge)
        ));
        assert_eq!(session.current_build().unwrap().summary, built);
        session.edit(2, " ".repeat(MAX_SOURCE_BYTES)).unwrap();
        assert!(session.current_build().is_none());
        assert!(session.build(2).is_err());
    }

    #[test]
    fn close_discards_builds_and_rejects_all_late_requests() {
        let mut session = AuthoringSession::new().unwrap();
        session.edit(1, SOURCE.into()).unwrap();
        session.build(1).unwrap();
        session.close();
        session.close();
        assert!(session.current_build().is_none());
        assert!(matches!(session.build(1), Err(MobileError::Closed)));
        assert!(matches!(
            session.edit(1, SOURCE.into()),
            Err(MobileError::Closed)
        ));
        assert!(matches!(
            session.edit(2, SOURCE.into()),
            Err(MobileError::Closed)
        ));
        let mut reopened = AuthoringSession::new().unwrap();
        reopened.edit(1, SOURCE.into()).unwrap();
        reopened.build(1).unwrap();
    }

    #[test]
    fn generated_firmware_is_inspected_without_becoming_a_verified_rom() {
        let generated = vec![0xa5; 16_384];
        let receipt = inspect_firmware(&generated).unwrap();
        assert_eq!(receipt.schema_version, 1);
        assert_eq!(receipt.sha256, sha256(&generated));
        assert!(!receipt.matches_verified_dragon32);
        assert!(!format!("{receipt:?}").contains("165, 165"));
        for length in [0, 1, 16_383, 16_385] {
            assert!(matches!(
                inspect_firmware(&vec![0xa5; length]),
                Err(MobileError::FirmwareSize)
            ));
        }
    }

    #[test]
    fn token_ram_limits_and_non_ascii_source_do_not_publish_a_build() {
        let mut session = AuthoringSession::new().unwrap();
        session.edit(1, "10 PRINT \"£\"\n".into()).unwrap();
        assert!(session.build(1).is_err());
        assert!(session.current_build().is_none());
        session
            .edit(2, format!("10 REM {}\n", "x".repeat(30_000)))
            .unwrap();
        assert!(matches!(
            session.build(2),
            Err(MobileError::Tokens(TokenError::ProgramTooLarge { .. }))
        ));
        assert!(session.current_build().is_none());
    }
}
