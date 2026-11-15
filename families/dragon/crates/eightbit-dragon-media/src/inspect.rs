//! Non-destructive BASIC-media detection and opaque disk-image validation.

use crate::{
    CassetteBlockInfo, CassetteMetadata, DecodedWavCassette, MAX_WAV_BYTES, WavCassetteMetadata,
    decode_wav_cassette, read_ascii_basic, read_tokenised_basic,
};
use eightbit_dragon_basic_core::{Dialect, Severity, TokenWarning, analyse, detokenise, tokenise};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

const MAX_BINARY_MEDIA_BYTES: usize = 16 * 1024 * 1024;
const MAX_BASIC_SOURCE_BYTES: usize = 1024 * 1024;

/// Explicit or automatic BASIC-media input selection.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MediaFormatHint {
    /// Attempt every supported parser and require exactly one match.
    Auto,
    /// ASCII numbered Dragon BASIC listing.
    AsciiBasic,
    /// Dragon linked-line token bytes.
    LinkedBasic,
    /// Tokenised BASIC cassette image.
    CassetteBasic,
    /// PCM WAV containing one tokenised BASIC cassette image.
    WavCassetteBasic,
    /// ASCII/gapped BASIC cassette image.
    CassetteAsciiBasic,
    /// PCM WAV containing one ASCII/gapped BASIC cassette image.
    WavCassetteAsciiBasic,
}

/// Detected supported BASIC-media format.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BasicMediaFormat {
    /// ASCII numbered Dragon BASIC listing.
    AsciiBasic,
    /// Dragon linked-line token bytes.
    LinkedBasic,
    /// Tokenised BASIC cassette image.
    CassetteBasic,
    /// PCM WAV containing one tokenised BASIC cassette image.
    WavCassetteBasic,
    /// ASCII/gapped BASIC cassette image.
    CassetteAsciiBasic,
    /// PCM WAV containing one ASCII/gapped BASIC cassette image.
    WavCassetteAsciiBasic,
}

impl BasicMediaFormat {
    /// Stable manifest/report identifier.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::AsciiBasic => "dragon-basic-listing",
            Self::LinkedBasic => "dragon-basic-linked",
            Self::CassetteBasic => "dragon-cassette-tokenised-basic",
            Self::WavCassetteBasic => "dragon-wav-cassette-tokenised-basic",
            Self::CassetteAsciiBasic => "dragon-cassette-ascii-basic",
            Self::WavCassetteAsciiBasic => "dragon-wav-cassette-ascii-basic",
        }
    }
}

/// Serializable recovery warning independent of parser lifetimes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaWarning {
    /// Stable warning code.
    pub code: String,
    /// Byte offset in the linked token stream.
    pub offset: usize,
    /// Explanation.
    pub message: String,
}

/// Read-only inspection result suitable for preview before an import.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BasicMediaInspection {
    /// Inspection schema version.
    pub schema_version: u32,
    /// Unambiguous detected/selected format.
    pub format: BasicMediaFormat,
    /// SHA-256 of exact original bytes.
    pub input_sha256: String,
    /// Exact original byte count.
    pub input_bytes: usize,
    /// Canonical editable Dragon BASIC listing.
    pub source: String,
    /// SHA-256 of the canonical recovered listing.
    pub source_sha256: String,
    /// True because inspection never opens an output or modifies the input.
    pub read_only: bool,
    /// True when the canonical source retokenises to the exact recovered token stream.
    pub canonical_round_trip: bool,
    /// Cassette filename when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cassette_filename: Option<String>,
    /// Supported cassette filename-block metadata when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cassette_metadata: Option<CassetteMetadata>,
    /// Validated cassette blocks when applicable.
    pub cassette_blocks: Vec<CassetteBlockInfo>,
    /// Bounded source-audio facts when the cassette was recovered from WAV.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wav_metadata: Option<WavCassetteMetadata>,
    /// Lossless unknown-token warnings.
    pub warnings: Vec<MediaWarning>,
}

/// Persisted, path-free lineage added to a subsequent build manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportProvenance {
    /// Provenance schema version.
    pub schema_version: u32,
    /// Original leaf filename only; never a machine-local path.
    pub source_name: String,
    /// Input format identifier.
    pub input_format: String,
    /// SHA-256 of exact imported bytes.
    pub input_sha256: String,
    /// Exact imported byte count.
    pub input_bytes: usize,
    /// SHA-256 of the imported canonical source.
    pub imported_source_sha256: String,
    /// SHA-256 of the current project source descended from the import.
    pub current_source_sha256: String,
    /// Whether canonical source reproduced the recovered token stream.
    pub canonical_round_trip: bool,
    /// Recovery warnings retained without ROM or media bytes.
    pub warnings: Vec<MediaWarning>,
    /// Bounded WAV properties for schema 2 audio imports.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wav_metadata: Option<WavCassetteMetadata>,
}

impl BasicMediaInspection {
    /// Create path-free lineage for an explicitly approved import.
    pub fn provenance(&self, source_name: &str) -> Result<ImportProvenance, MediaInspectionError> {
        if source_name.is_empty()
            || source_name.len() > 255
            || source_name
                .chars()
                .any(|character| matches!(character, '/' | '\\' | '\0'))
        {
            return Err(MediaInspectionError::InvalidSourceName);
        }
        Ok(ImportProvenance {
            schema_version: if self.wav_metadata.is_some() { 2 } else { 1 },
            source_name: source_name.to_owned(),
            input_format: self.format.id().to_owned(),
            input_sha256: self.input_sha256.clone(),
            input_bytes: self.input_bytes,
            imported_source_sha256: self.source_sha256.clone(),
            current_source_sha256: self.source_sha256.clone(),
            canonical_round_trip: self.canonical_round_trip,
            warnings: self.warnings.clone(),
            wav_metadata: self.wav_metadata.clone(),
        })
    }
}

/// Supported opaque disk-image container.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DiskFormat {
    /// JVC/raw `.dsk`; geometry remains deliberately delegated to `XRoar`.
    Jvc,
    /// VDK image with a validated `dk` header and bounded header length.
    Vdk,
    /// DMK image with a minimally validated 16-byte header.
    Dmk,
}

/// Read-only disk attachment evidence.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiskInspection {
    /// Inspection schema version.
    pub schema_version: u32,
    /// Extension/signature-validated container.
    pub format: DiskFormat,
    /// SHA-256 of exact disk bytes.
    pub input_sha256: String,
    /// Exact disk byte count.
    pub input_bytes: usize,
    /// Always true in I5.
    pub write_protected: bool,
    /// Honest detection limitation shown to the user.
    pub detection: &'static str,
}

/// Malformed, ambiguous or unsupported media.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum MediaInspectionError {
    /// Input exceeds the bounded media limit.
    #[error("DSM2000: media exceeds the 16 MiB inspection limit")]
    TooLarge,
    /// WAV input exceeds its separate bounded media limit.
    #[error("DSM2004: WAV media exceeds the 64 MiB inspection limit")]
    WavTooLarge,
    /// No supported parser accepted the bytes.
    #[error("DSM2001: media is not a supported ASCII, linked BASIC, cassette or PCM WAV input")]
    Unsupported,
    /// More than one parser accepted the bytes.
    #[error("DSM2002: media format is ambiguous; select an explicit input format")]
    Ambiguous,
    /// An explicitly selected parser rejected the input.
    #[error("DSM2003: selected media format is invalid: {0}")]
    Invalid(String),
    /// Leaf provenance name is unsafe.
    #[error("DSM2004: imported media filename is invalid")]
    InvalidSourceName,
    /// Disk container/extension is unsupported or malformed.
    #[error("DSM2100: {0}")]
    InvalidDisk(String),
}

/// Detect or explicitly inspect a BASIC listing, linked stream or cassette.
pub fn inspect_basic_media(
    bytes: &[u8],
    hint: MediaFormatHint,
) -> Result<BasicMediaInspection, MediaInspectionError> {
    inspect_basic_media_for_dialect(bytes, hint, eightbit_dragon_targets::BASIC_DIALECT)
}

/// Inspect using an explicitly selected, allow-listed BASIC memory profile.
pub fn inspect_basic_media_for_dialect(
    bytes: &[u8],
    hint: MediaFormatHint,
    dialect_id: &str,
) -> Result<BasicMediaInspection, MediaInspectionError> {
    let dialect = Dialect::for_id(dialect_id)
        .map_err(|error| MediaInspectionError::Invalid(error.to_string()))?;
    let wav_shaped = bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WAVE";
    if wav_shaped && bytes.len() > MAX_WAV_BYTES {
        return Err(MediaInspectionError::WavTooLarge);
    }
    if !wav_shaped && bytes.len() > MAX_BINARY_MEDIA_BYTES {
        return Err(MediaInspectionError::TooLarge);
    }
    if hint != MediaFormatHint::Auto {
        return inspect_as(bytes, hint, &dialect, None).map_err(MediaInspectionError::Invalid);
    }
    // The tokenised and ASCII candidates share one bounded audio decode.
    let wav = if wav_shaped {
        Some(decode_wav_cassette(bytes).map_err(|_| MediaInspectionError::Unsupported)?)
    } else {
        None
    };
    let candidates = [
        MediaFormatHint::AsciiBasic,
        MediaFormatHint::LinkedBasic,
        MediaFormatHint::CassetteBasic,
        MediaFormatHint::WavCassetteBasic,
        MediaFormatHint::CassetteAsciiBasic,
        MediaFormatHint::WavCassetteAsciiBasic,
    ]
    .into_iter()
    .filter_map(|candidate| inspect_as(bytes, candidate, &dialect, wav.as_ref()).ok())
    .collect::<Vec<_>>();
    match candidates.len() {
        0 => Err(MediaInspectionError::Unsupported),
        1 => candidates
            .into_iter()
            .next()
            .ok_or(MediaInspectionError::Unsupported),
        _ => Err(MediaInspectionError::Ambiguous),
    }
}

/// Validate a disk image for opaque, write-protected `XRoar` attachment.
pub fn inspect_disk_image(
    bytes: &[u8],
    filename: &str,
) -> Result<DiskInspection, MediaInspectionError> {
    if bytes.len() > MAX_BINARY_MEDIA_BYTES {
        return Err(MediaInspectionError::TooLarge);
    }
    if bytes.len() < 128 {
        return Err(MediaInspectionError::InvalidDisk(
            "disk image is smaller than one supported sector".to_owned(),
        ));
    }
    let extension = filename
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .ok_or_else(|| {
            MediaInspectionError::InvalidDisk(
                "disk image requires an explicit .dsk, .vdk or .dmk extension".to_owned(),
            )
        })?;
    let (format, detection) = match extension.as_str() {
        "vdk" => {
            if bytes.len() < 12 || &bytes[..2] != b"dk" {
                return Err(MediaInspectionError::InvalidDisk(
                    "VDK image has no valid 'dk' header".to_owned(),
                ));
            }
            let header_length = usize::from(u16::from_le_bytes([bytes[2], bytes[3]]));
            if !(12..bytes.len()).contains(&header_length) {
                return Err(MediaInspectionError::InvalidDisk(
                    "VDK header length is outside the image".to_owned(),
                ));
            }
            (DiskFormat::Vdk, "VDK signature and header length")
        }
        "dmk" => {
            if bytes.len() < 16 || bytes[1] == 0 {
                return Err(MediaInspectionError::InvalidDisk(
                    "DMK image has an invalid 16-byte header or zero tracks".to_owned(),
                ));
            }
            let track_length = usize::from(u16::from_le_bytes([bytes[2], bytes[3]]));
            if track_length < 128 || track_length > bytes.len() - 16 {
                return Err(MediaInspectionError::InvalidDisk(
                    "DMK track length is outside the image".to_owned(),
                ));
            }
            (DiskFormat::Dmk, "DMK extension and bounded header")
        }
        "dsk" => {
            if !bytes.len().is_multiple_of(128) {
                return Err(MediaInspectionError::InvalidDisk(
                    "raw JVC/DSK size is not sector-aligned".to_owned(),
                ));
            }
            (
                DiskFormat::Jvc,
                "JVC/raw DSK extension and sector alignment; geometry is inferred by XRoar",
            )
        }
        _ => {
            return Err(MediaInspectionError::InvalidDisk(
                "supported disk extensions are .dsk, .vdk and .dmk".to_owned(),
            ));
        }
    };
    Ok(DiskInspection {
        schema_version: 1,
        format,
        input_sha256: sha256(bytes),
        input_bytes: bytes.len(),
        write_protected: true,
        detection,
    })
}

fn inspect_as(
    bytes: &[u8],
    hint: MediaFormatHint,
    dialect: &Dialect,
    wav_cache: Option<&DecodedWavCassette>,
) -> Result<BasicMediaInspection, String> {
    let (
        format,
        source,
        recovered_tokens,
        cassette_filename,
        cassette_metadata,
        blocks,
        wav_metadata,
        warnings,
    ) = match hint {
        MediaFormatHint::AsciiBasic => {
            if bytes.len() > MAX_BASIC_SOURCE_BYTES {
                return Err("ASCII BASIC input exceeds 1 MiB".to_owned());
            }
            let source = std::str::from_utf8(bytes)
                .map_err(|error| format!("ASCII BASIC input is not UTF-8: {error}"))?;
            if !source.is_ascii() {
                return Err("ASCII BASIC input contains non-ASCII bytes".to_owned());
            }
            let analysis = analyse(source, dialect);
            if analysis
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity == Severity::Error)
            {
                return Err("ASCII BASIC listing has source errors".to_owned());
            }
            let tokenised =
                tokenise(&analysis.program, dialect).map_err(|error| error.to_string())?;
            let canonical =
                detokenise(&tokenised.bytes, dialect).map_err(|error| error.to_string())?;
            (
                BasicMediaFormat::AsciiBasic,
                canonical.source,
                tokenised.bytes,
                None,
                None,
                Vec::new(),
                None,
                Vec::new(),
            )
        }
        MediaFormatHint::LinkedBasic => {
            let recovered = detokenise(bytes, dialect).map_err(|error| error.to_string())?;
            (
                BasicMediaFormat::LinkedBasic,
                recovered.source,
                bytes.to_vec(),
                None,
                None,
                Vec::new(),
                None,
                warnings(recovered.warnings),
            )
        }
        MediaFormatHint::CassetteBasic => {
            let cassette = read_tokenised_basic(bytes).map_err(|error| error.to_string())?;
            let recovered =
                detokenise(&cassette.program, dialect).map_err(|error| error.to_string())?;
            (
                BasicMediaFormat::CassetteBasic,
                recovered.source,
                cassette.program,
                Some(cassette.filename),
                Some(cassette.metadata),
                cassette.blocks,
                None,
                warnings(recovered.warnings),
            )
        }
        MediaFormatHint::WavCassetteBasic => {
            let decoded = cached_wav(bytes, wav_cache)?;
            let cassette =
                read_tokenised_basic(&decoded.cassette).map_err(|error| error.to_string())?;
            let recovered =
                detokenise(&cassette.program, dialect).map_err(|error| error.to_string())?;
            (
                BasicMediaFormat::WavCassetteBasic,
                recovered.source,
                cassette.program,
                Some(cassette.filename),
                Some(cassette.metadata),
                cassette.blocks,
                Some(decoded.metadata.clone()),
                warnings(recovered.warnings),
            )
        }
        MediaFormatHint::CassetteAsciiBasic | MediaFormatHint::WavCassetteAsciiBasic => {
            let decoded = if hint == MediaFormatHint::WavCassetteAsciiBasic {
                Some(cached_wav(bytes, wav_cache)?)
            } else {
                None
            };
            let cassette = read_ascii_basic(decoded.as_ref().map_or(bytes, |wav| &wav.cassette))
                .map_err(|error| error.to_string())?;
            let (source, recovered_tokens, warnings) =
                recover_ascii_cassette(&cassette.text, dialect)?;
            (
                if decoded.is_some() {
                    BasicMediaFormat::WavCassetteAsciiBasic
                } else {
                    BasicMediaFormat::CassetteAsciiBasic
                },
                source,
                recovered_tokens,
                Some(cassette.filename),
                Some(cassette.metadata),
                cassette.blocks,
                decoded.map(|wav| wav.metadata.clone()),
                warnings,
            )
        }
        MediaFormatHint::Auto => return Err("automatic selection is not a format".to_owned()),
    };
    let analysis = analyse(&source, dialect);
    if analysis
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        return Err("recovered BASIC source has errors".to_owned());
    }
    let retokenised = tokenise(&analysis.program, dialect).map_err(|error| error.to_string())?;
    let canonical_round_trip = retokenised.bytes == recovered_tokens;
    if !canonical_round_trip {
        return Err("canonical source did not reproduce recovered token bytes".to_owned());
    }
    let schema_version = if wav_metadata.is_some() { 2 } else { 1 };
    Ok(BasicMediaInspection {
        schema_version,
        format,
        input_sha256: sha256(bytes),
        input_bytes: bytes.len(),
        source_sha256: sha256(source.as_bytes()),
        source,
        read_only: true,
        canonical_round_trip,
        cassette_filename,
        cassette_metadata,
        cassette_blocks: blocks,
        wav_metadata,
        warnings,
    })
}

fn cached_wav<'a>(
    bytes: &[u8],
    cached: Option<&'a DecodedWavCassette>,
) -> Result<std::borrow::Cow<'a, DecodedWavCassette>, String> {
    match cached {
        Some(wav) => Ok(std::borrow::Cow::Borrowed(wav)),
        None => decode_wav_cassette(bytes)
            .map(std::borrow::Cow::Owned)
            .map_err(|error| error.to_string()),
    }
}

fn recover_ascii_cassette(
    text: &[u8],
    dialect: &Dialect,
) -> Result<(String, Vec<u8>, Vec<MediaWarning>), String> {
    if text.len() > MAX_BASIC_SOURCE_BYTES {
        return Err("ASCII BASIC input exceeds 1 MiB".to_owned());
    }
    // CLOAD applies complete CR-terminated lines in order. Replacements and
    // deletion lines are resolved before the existing project parser validates
    // the recovered programme. LF is ignored by the guest line input routine.
    let mut lines = std::collections::BTreeMap::new();
    let mut line = Vec::new();
    let mut last_cr = 0;
    for (offset, byte) in text.iter().copied().enumerate() {
        if byte == b'\n' {
            continue;
        }
        if byte != b'\r' {
            line.push(byte);
            continue;
        }
        last_cr = offset + 1;
        let source = std::str::from_utf8(&line).map_err(|_| "cassette text is not ASCII")?;
        let source = source.trim_start_matches(' ');
        if !source.is_empty() {
            let digits = source.bytes().take_while(u8::is_ascii_digit).count();
            let number = source[..digits]
                .parse::<u16>()
                .ok()
                .filter(|number| *number > 0 && *number <= dialect.maximum_line_number)
                .ok_or("cassette line number is outside the project dialect")?;
            let body = source[digits..].trim_start_matches(' ');
            if body.is_empty() {
                lines.remove(&number);
            } else {
                lines.insert(number, body.to_owned());
            }
        }
        line.clear();
    }
    let mut source = String::new();
    for (number, body) in lines {
        use std::fmt::Write;
        writeln!(source, "{number} {body}").map_err(|_| "cannot recover cassette listing")?;
    }
    let analysis = analyse(&source, dialect);
    if analysis
        .diagnostics
        .iter()
        .any(|item| item.severity == Severity::Error)
    {
        return Err("cassette ASCII listing has source errors".to_owned());
    }
    let tokenised = tokenise(&analysis.program, dialect).map_err(|error| error.to_string())?;
    let recovered = detokenise(&tokenised.bytes, dialect).map_err(|error| error.to_string())?;
    let warnings = if line.is_empty() {
        Vec::new()
    } else {
        vec![MediaWarning {
            code: "DSM2010".to_owned(),
            offset: last_cr,
            message: "Unterminated final cassette text was discarded, as by CLOAD.".to_owned(),
        }]
    };
    Ok((recovered.source, tokenised.bytes, warnings))
}

fn warnings(warnings: Vec<TokenWarning>) -> Vec<MediaWarning> {
    warnings
        .into_iter()
        .map(|warning| MediaWarning {
            code: warning.code.to_owned(),
            offset: warning.offset,
            message: warning.message,
        })
        .collect()
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{create_tokenised_basic, encode_cassette_wav};

    fn linked(source: &str) -> Vec<u8> {
        let dialect = Dialect::dragon32().unwrap();
        tokenise(&analyse(source, &dialect).program, &dialect)
            .unwrap()
            .bytes
    }

    #[test]
    fn detects_all_four_basic_inputs_and_preserves_media_metadata() {
        let source = b"10 PRINT \"PRESERVED\"\r\n20 END\r\n";
        let ascii = inspect_basic_media(source, MediaFormatHint::Auto).unwrap();
        assert_eq!(ascii.format, BasicMediaFormat::AsciiBasic);
        assert_eq!(ascii.schema_version, 1);
        assert_eq!(ascii.source, "10 PRINT \"PRESERVED\"\n20 END\n");

        let tokens = linked(&ascii.source);
        let linked = inspect_basic_media(&tokens, MediaFormatHint::Auto).unwrap();
        assert_eq!(linked.format, BasicMediaFormat::LinkedBasic);

        let cassette = create_tokenised_basic("ARCHIVE", &tokens).unwrap();
        let inspected = inspect_basic_media(&cassette, MediaFormatHint::Auto).unwrap();
        assert_eq!(inspected.format, BasicMediaFormat::CassetteBasic);
        assert_eq!(inspected.cassette_filename.as_deref(), Some("ARCHIVE"));
        assert_eq!(inspected.cassette_blocks.len(), 3);
        assert_eq!(inspected.schema_version, 1);
        assert!(inspected.read_only && inspected.canonical_round_trip);

        let wav = encode_cassette_wav(&cassette).unwrap();
        let audio = inspect_basic_media(&wav, MediaFormatHint::Auto).unwrap();
        assert_eq!(audio.format, BasicMediaFormat::WavCassetteBasic);
        assert_eq!(audio.schema_version, 2);
        assert_eq!(audio.source, ascii.source);
        assert_eq!(
            audio
                .wav_metadata
                .as_ref()
                .map(|metadata| metadata.sample_rate),
            Some(44_100)
        );
        assert_eq!(audio.provenance("ARCHIVE.WAV").unwrap().schema_version, 2);
    }

    #[test]
    fn explicit_format_reports_corruption_without_falling_back() {
        let error = inspect_basic_media(b"not a listing", MediaFormatHint::AsciiBasic)
            .expect_err("invalid listing");
        assert!(matches!(error, MediaInspectionError::Invalid(_)));
        assert_eq!(
            inspect_basic_media(&[0xff; 32], MediaFormatHint::Auto).unwrap_err(),
            MediaInspectionError::Unsupported
        );
    }

    #[test]
    fn validates_only_bounded_read_only_disk_containers() {
        let dsk = inspect_disk_image(&[0; 256], "blank.dsk").unwrap();
        assert_eq!(dsk.format, DiskFormat::Jvc);
        assert!(dsk.write_protected);

        let mut vdk = vec![0; 256];
        vdk[..4].copy_from_slice(&[b'd', b'k', 12, 0]);
        assert_eq!(
            inspect_disk_image(&vdk, "blank.vdk").unwrap().format,
            DiskFormat::Vdk
        );
        assert!(inspect_disk_image(&[0; 255], "bad.dsk").is_err());
        assert!(inspect_disk_image(&[0; 256], "bad.img").is_err());
    }
}
