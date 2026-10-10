//! Bounded Dragon media inspection, cassette encoding and validation.

#![cfg_attr(test, allow(clippy::expect_used, clippy::panic, clippy::unwrap_used))]

mod cartridge;
mod native;
pub use native::{NativeImageError, create_native_run_image};
mod inspect;
pub mod programme;
pub mod reconstruct;

pub use cartridge::{
    CARTRIDGE_BASE, CartridgeAssembly, CartridgeError, MAX_CARTRIDGE_BYTES,
    create_program_cartridge, validate_cartridge,
};
mod wav;

pub use inspect::{
    BasicMediaFormat, BasicMediaInspection, DiskFormat, DiskInspection, ImportProvenance,
    MediaFormatHint, MediaInspectionError, MediaWarning, inspect_basic_media,
    inspect_basic_media_for_dialect, inspect_disk_image,
};
pub use wav::{
    DecodedWavCassette, MAX_WAV_BYTES, WavCassetteMetadata, WavError, decode_wav_cassette,
    encode_cassette_wav, encode_pcm_wav_mono,
};

use serde::Serialize;
use thiserror::Error;

const LEADER: u8 = 0x55;
const SYNC: u8 = 0x3c;
const FILENAME_BLOCK: u8 = 0x00;
const DATA_BLOCK: u8 = 0x01;
const EOF_BLOCK: u8 = 0xff;
const BASIC_FILE_TYPE: u8 = 0x00;
const MACHINE_CODE_FILE_TYPE: u8 = 0x02;
const TOKENISED_FLAG: u8 = 0x00;
const BINARY_FLAG: u8 = 0x00;
const NO_GAPS_FLAG: u8 = 0x00;
const FILENAME_BLOCK_LENGTH: usize = 15;
const LEADER_LENGTH: usize = 128;
const MAX_CASSETTE_BYTES: usize = 16 * 1024 * 1024;

/// Metadata and token bytes recovered from one I1 cassette.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TokenisedBasicCassette {
    /// Trimmed Dragon filename.
    pub filename: String,
    /// Concatenated linked-line program bytes.
    pub program: Vec<u8>,
    /// Number of data blocks.
    pub data_blocks: usize,
    /// Preserved supported filename-block fields.
    pub metadata: CassetteMetadata,
    /// Ordered, checksum-validated blocks.
    pub blocks: Vec<CassetteBlockInfo>,
}

/// Metadata and payload recovered from one binary machine-code cassette file.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MachineCodeCassette {
    /// Trimmed Dragon filename.
    pub filename: String,
    /// Concatenated machine-code bytes.
    pub program: Vec<u8>,
    /// Big-endian execution address from the filename block.
    pub execution_address: u16,
    /// Big-endian load address from the filename block.
    pub load_address: u16,
    /// Number of data blocks.
    pub data_blocks: usize,
    /// Preserved filename-block fields.
    pub metadata: CassetteMetadata,
    /// Ordered, checksum-validated blocks.
    pub blocks: Vec<CassetteBlockInfo>,
}

/// Supported cassette filename-block metadata without guessed address semantics.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CassetteMetadata {
    /// File-type byte (`0` for BASIC in the supported variant).
    pub file_type: u8,
    /// ASCII/tokenised byte (`0` for tokenised in the supported variant).
    pub ascii_flag: u8,
    /// Gap-mode byte (`0` for ungapped in the supported variant).
    pub gaps_flag: u8,
    /// Execution address followed by load address, both big-endian.
    pub address_bytes: [u8; 4],
}

/// One validated cassette block.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CassetteBlockInfo {
    /// Zero-based block ordinal.
    pub ordinal: usize,
    /// Sync-marker byte offset in the original image.
    pub offset: usize,
    /// `filename`, `data` or `eof`.
    pub kind: &'static str,
    /// Block payload length.
    pub data_length: usize,
    /// Validated checksum byte retained as metadata.
    pub checksum: u8,
}

/// Malformed or unsupported cassette input.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CassetteError {
    /// Input exceeds the bounded parser limit.
    #[error("DSM1000: cassette exceeds the {MAX_CASSETTE_BYTES}-byte limit")]
    TooLarge,
    /// No valid sync marker followed a leader.
    #[error("DSM1001: no cassette block sync found at or after byte {offset}")]
    MissingSync {
        /// Search start.
        offset: usize,
    },
    /// A block was truncated.
    #[error("DSM1002: cassette block at byte {offset} is truncated")]
    Truncated {
        /// Block sync offset.
        offset: usize,
    },
    /// Additive checksum mismatch.
    #[error(
        "DSM1003: cassette block at byte {offset} has checksum 0x{actual:02X}; expected 0x{expected:02X}"
    )]
    Checksum {
        /// Block sync offset.
        offset: usize,
        /// Read checksum.
        actual: u8,
        /// Computed checksum.
        expected: u8,
    },
    /// First block was not a supported filename block.
    #[error(
        "DSM1004: expected a 15-byte filename block, found type 0x{block_type:02X} with length {length}"
    )]
    InvalidFilenameBlock {
        /// Actual block type.
        block_type: u8,
        /// Actual data length.
        length: usize,
    },
    /// Filename block describes a non-I1 cassette type.
    #[error("DSM1005: cassette is not an ungapped tokenised BASIC file")]
    UnsupportedFileType,
    /// Unexpected block ordering or type.
    #[error("DSM1006: unexpected cassette block type 0x{block_type:02X} at byte {offset}")]
    UnexpectedBlock {
        /// Block type.
        block_type: u8,
        /// Block sync offset.
        offset: usize,
    },
    /// No EOF block was found.
    #[error("DSM1007: cassette has no EOF block")]
    MissingEof,
    /// Filename could not be represented.
    #[error("DSM1008: cassette filename has no usable ASCII characters")]
    InvalidFilename,
    /// Non-leader data followed the EOF block.
    #[error("DSM1009: unexpected data follows the cassette EOF block at byte {offset}")]
    TrailingData {
        /// First unexpected byte.
        offset: usize,
    },
    /// Machine-code cassette payload was empty.
    #[error("DSM1010: machine-code cassette payload is empty")]
    EmptyMachineCode,
    /// Machine-code payload would wrap the 16-bit address space.
    #[error("DSM1011: machine-code payload does not fit from load address 0x{load_address:04X}")]
    MachineCodeAddressRange {
        /// Requested load address.
        load_address: u16,
    },
    /// Filename block does not describe the supported binary machine-code variant.
    #[error("DSM1012: cassette is not an ungapped binary machine-code file")]
    UnsupportedMachineCodeFileType,
}

/// Create a tokenised BASIC cassette with checked blocks and a 15-byte header.
pub fn create_tokenised_basic(
    filename: &str,
    linked_program: &[u8],
) -> Result<Vec<u8>, CassetteError> {
    let filename = normalise_filename(filename)?;
    let mut header = [0_u8; FILENAME_BLOCK_LENGTH];
    header[..8].copy_from_slice(&filename);
    header[8] = BASIC_FILE_TYPE;
    header[9] = TOKENISED_FLAG;
    header[10] = NO_GAPS_FLAG;
    // Bytes 11–14 are execution/load addresses and remain zero for BASIC.

    let data_block_count = linked_program.len().div_ceil(255);
    let estimated = (data_block_count + 2)
        .checked_mul(LEADER_LENGTH + 6 + 255)
        .ok_or(CassetteError::TooLarge)?;
    if estimated > MAX_CASSETTE_BYTES {
        return Err(CassetteError::TooLarge);
    }
    let mut output = Vec::with_capacity(estimated);
    append_block(&mut output, FILENAME_BLOCK, &header)?;
    for chunk in linked_program.chunks(255) {
        append_block(&mut output, DATA_BLOCK, chunk)?;
    }
    append_block(&mut output, EOF_BLOCK, &[])?;
    Ok(output)
}

/// Create an ungapped binary machine-code cassette file.
pub fn create_machine_code(
    filename: &str,
    program: &[u8],
    load_address: u16,
    execution_address: u16,
) -> Result<Vec<u8>, CassetteError> {
    if program.is_empty() {
        return Err(CassetteError::EmptyMachineCode);
    }
    let final_offset = program
        .len()
        .checked_sub(1)
        .and_then(|length| u16::try_from(length).ok())
        .and_then(|length| load_address.checked_add(length));
    if final_offset.is_none() {
        return Err(CassetteError::MachineCodeAddressRange { load_address });
    }

    let filename = normalise_filename(filename)?;
    let mut header = [0_u8; FILENAME_BLOCK_LENGTH];
    header[..8].copy_from_slice(&filename);
    header[8] = MACHINE_CODE_FILE_TYPE;
    header[9] = BINARY_FLAG;
    header[10] = NO_GAPS_FLAG;
    header[11..13].copy_from_slice(&execution_address.to_be_bytes());
    header[13..15].copy_from_slice(&load_address.to_be_bytes());
    create_cassette_blocks(&header, program)
}

/// Strictly validate an I1 tokenised BASIC cassette and recover its program.
pub fn read_tokenised_basic(bytes: &[u8]) -> Result<TokenisedBasicCassette, CassetteError> {
    let parsed = read_cassette_file(bytes, ExpectedFile::TokenisedBasic, true)?;
    Ok(TokenisedBasicCassette {
        filename: parsed.filename,
        program: parsed.program,
        data_blocks: parsed.data_blocks,
        metadata: parsed.metadata,
        blocks: parsed.blocks,
    })
}

/// Strictly validate one standalone ungapped binary machine-code cassette file.
pub fn read_machine_code(bytes: &[u8]) -> Result<MachineCodeCassette, CassetteError> {
    let parsed = read_cassette_file(bytes, ExpectedFile::MachineCode, true)?;
    if parsed.program.is_empty() {
        return Err(CassetteError::EmptyMachineCode);
    }
    let execution_address = u16::from_be_bytes([
        parsed.metadata.address_bytes[0],
        parsed.metadata.address_bytes[1],
    ]);
    let load_address = u16::from_be_bytes([
        parsed.metadata.address_bytes[2],
        parsed.metadata.address_bytes[3],
    ]);
    let final_offset = parsed
        .program
        .len()
        .checked_sub(1)
        .and_then(|length| u16::try_from(length).ok())
        .and_then(|length| load_address.checked_add(length));
    if final_offset.is_none() {
        return Err(CassetteError::MachineCodeAddressRange { load_address });
    }
    Ok(MachineCodeCassette {
        filename: parsed.filename,
        program: parsed.program,
        execution_address,
        load_address,
        data_blocks: parsed.data_blocks,
        metadata: parsed.metadata,
        blocks: parsed.blocks,
    })
}

#[derive(Debug, Clone, Copy)]
enum ExpectedFile {
    TokenisedBasic,
    MachineCode,
}

struct ParsedCassette {
    filename: String,
    program: Vec<u8>,
    data_blocks: usize,
    metadata: CassetteMetadata,
    blocks: Vec<CassetteBlockInfo>,
    consumed: usize,
}

fn read_cassette_file(
    bytes: &[u8],
    expected_file: ExpectedFile,
    reject_trailing_data: bool,
) -> Result<ParsedCassette, CassetteError> {
    if bytes.len() > MAX_CASSETTE_BYTES {
        return Err(CassetteError::TooLarge);
    }
    let mut cursor = 0;
    let first = read_block(bytes, &mut cursor)?;
    if first.block_type != FILENAME_BLOCK || first.data.len() != FILENAME_BLOCK_LENGTH {
        return Err(CassetteError::InvalidFilenameBlock {
            block_type: first.block_type,
            length: first.data.len(),
        });
    }
    let supported = match expected_file {
        ExpectedFile::TokenisedBasic => {
            first.data[8] == BASIC_FILE_TYPE
                && first.data[9] == TOKENISED_FLAG
                && first.data[10] == NO_GAPS_FLAG
        }
        ExpectedFile::MachineCode => {
            first.data[8] == MACHINE_CODE_FILE_TYPE
                && first.data[9] == BINARY_FLAG
                && first.data[10] == NO_GAPS_FLAG
        }
    };
    if !supported {
        return Err(match expected_file {
            ExpectedFile::TokenisedBasic => CassetteError::UnsupportedFileType,
            ExpectedFile::MachineCode => CassetteError::UnsupportedMachineCodeFileType,
        });
    }
    let filename = String::from_utf8_lossy(&first.data[..8])
        .trim_end_matches(' ')
        .to_owned();
    let metadata = CassetteMetadata {
        file_type: first.data[8],
        ascii_flag: first.data[9],
        gaps_flag: first.data[10],
        address_bytes: [
            first.data[11],
            first.data[12],
            first.data[13],
            first.data[14],
        ],
    };
    let mut blocks = vec![block_info(0, &first)];
    let mut program = Vec::new();
    let mut data_blocks = 0;
    loop {
        let block = read_block(bytes, &mut cursor)?;
        let ordinal = blocks.len();
        blocks.push(block_info(ordinal, &block));
        match block.block_type {
            DATA_BLOCK => {
                program
                    .len()
                    .checked_add(block.data.len())
                    .filter(|length| *length <= MAX_CASSETTE_BYTES)
                    .ok_or(CassetteError::TooLarge)?;
                program.extend_from_slice(block.data);
                data_blocks += 1;
            }
            EOF_BLOCK if block.data.is_empty() => {
                if reject_trailing_data
                    && let Some(relative) = bytes[cursor..].iter().position(|byte| *byte != LEADER)
                {
                    return Err(CassetteError::TrailingData {
                        offset: cursor + relative,
                    });
                }
                return Ok(ParsedCassette {
                    filename,
                    program,
                    data_blocks,
                    metadata,
                    blocks,
                    consumed: cursor,
                });
            }
            block_type => {
                return Err(CassetteError::UnexpectedBlock {
                    block_type,
                    offset: block.offset,
                });
            }
        }
        if cursor >= bytes.len() {
            return Err(CassetteError::MissingEof);
        }
    }
}

/// Return the checked cassette-file boundary for a supported BASIC or machine-code prefix.
///
/// WAV recovery uses this narrower parser entry point to detect multiple files without
/// weakening the strict public cassette readers' trailing-data rule.
pub(crate) fn supported_cassette_prefix_len(bytes: &[u8]) -> Option<usize> {
    if let Ok(parsed) = read_cassette_file(bytes, ExpectedFile::TokenisedBasic, false) {
        return Some(parsed.consumed);
    }
    let parsed = read_cassette_file(bytes, ExpectedFile::MachineCode, false).ok()?;
    if parsed.program.is_empty() {
        return None;
    }
    let load_address = u16::from_be_bytes([
        parsed.metadata.address_bytes[2],
        parsed.metadata.address_bytes[3],
    ]);
    parsed
        .program
        .len()
        .checked_sub(1)
        .and_then(|length| u16::try_from(length).ok())
        .and_then(|length| load_address.checked_add(length))?;
    Some(parsed.consumed)
}

fn create_cassette_blocks(
    header: &[u8; FILENAME_BLOCK_LENGTH],
    program: &[u8],
) -> Result<Vec<u8>, CassetteError> {
    let data_block_count = program.len().div_ceil(255);
    let estimated = (data_block_count + 2)
        .checked_mul(LEADER_LENGTH + 6 + 255)
        .ok_or(CassetteError::TooLarge)?;
    if estimated > MAX_CASSETTE_BYTES {
        return Err(CassetteError::TooLarge);
    }
    let mut output = Vec::with_capacity(estimated);
    append_block(&mut output, FILENAME_BLOCK, header)?;
    for chunk in program.chunks(255) {
        append_block(&mut output, DATA_BLOCK, chunk)?;
    }
    append_block(&mut output, EOF_BLOCK, &[])?;
    Ok(output)
}

struct Block<'a> {
    offset: usize,
    block_type: u8,
    data: &'a [u8],
    checksum: u8,
}

fn read_block<'a>(bytes: &'a [u8], cursor: &mut usize) -> Result<Block<'a>, CassetteError> {
    while *cursor < bytes.len() && bytes[*cursor] == LEADER {
        *cursor += 1;
    }
    if *cursor >= bytes.len() || bytes[*cursor] != SYNC {
        return Err(CassetteError::MissingSync { offset: *cursor });
    }
    let offset = *cursor;
    *cursor += 1;
    if bytes.len().saturating_sub(*cursor) < 3 {
        return Err(CassetteError::Truncated { offset });
    }
    let block_type = bytes[*cursor];
    let length_byte = bytes[*cursor + 1];
    let length = usize::from(length_byte);
    *cursor += 2;
    let data_end = cursor
        .checked_add(length)
        .filter(|end| end.saturating_add(1) <= bytes.len())
        .ok_or(CassetteError::Truncated { offset })?;
    let data = &bytes[*cursor..data_end];
    *cursor = data_end;
    let actual = bytes[*cursor];
    *cursor += 1;
    let expected = checksum(block_type, length_byte, data);
    if actual != expected {
        return Err(CassetteError::Checksum {
            offset,
            actual,
            expected,
        });
    }
    Ok(Block {
        offset,
        block_type,
        data,
        checksum: actual,
    })
}

fn block_info(ordinal: usize, block: &Block<'_>) -> CassetteBlockInfo {
    CassetteBlockInfo {
        ordinal,
        offset: block.offset,
        kind: match block.block_type {
            FILENAME_BLOCK => "filename",
            DATA_BLOCK => "data",
            EOF_BLOCK => "eof",
            _ => "unsupported",
        },
        data_length: block.data.len(),
        checksum: block.checksum,
    }
}

fn append_block(output: &mut Vec<u8>, block_type: u8, data: &[u8]) -> Result<(), CassetteError> {
    let length = u8::try_from(data.len()).map_err(|_| CassetteError::TooLarge)?;
    output.extend(std::iter::repeat_n(LEADER, LEADER_LENGTH));
    output.push(SYNC);
    output.push(block_type);
    output.push(length);
    output.extend_from_slice(data);
    output.push(checksum(block_type, length, data));
    output.push(LEADER);
    Ok(())
}

fn checksum(block_type: u8, length: u8, data: &[u8]) -> u8 {
    data.iter()
        .fold(block_type.wrapping_add(length), |sum, value| {
            sum.wrapping_add(*value)
        })
}

fn normalise_filename(filename: &str) -> Result<[u8; 8], CassetteError> {
    let stem = std::path::Path::new(filename)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(filename);
    let mut result = [b' '; 8];
    let mut count = 0;
    for byte in stem.bytes() {
        if count == 8 {
            break;
        }
        let normalised = if byte.is_ascii_alphanumeric() {
            byte.to_ascii_uppercase()
        } else if matches!(byte, b'_' | b'-' | b' ') {
            b'_'
        } else {
            continue;
        };
        result[count] = normalised;
        count += 1;
    }
    if count == 0 {
        return Err(CassetteError::InvalidFilename);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn emits_exact_filename_block_contract() {
        let cassette = create_tokenised_basic("hello.bas", &[0, 0])
            .unwrap_or_else(|error| panic!("cassette: {error}"));
        assert_eq!(cassette[..LEADER_LENGTH], [LEADER; LEADER_LENGTH]);
        assert_eq!(
            &cassette[LEADER_LENGTH..LEADER_LENGTH + 18],
            &[
                SYNC, 0x00, 0x0f, b'H', b'E', b'L', b'L', b'O', b' ', b' ', b' ', 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00
            ]
        );
        let header = &cassette[LEADER_LENGTH + 3..LEADER_LENGTH + 18];
        assert_eq!(
            cassette[LEADER_LENGTH + 18],
            checksum(FILENAME_BLOCK, 15, header)
        );
    }

    #[test]
    fn rejects_checksum_and_truncation() {
        let mut cassette = create_tokenised_basic("HELLO", &[0, 0])
            .unwrap_or_else(|error| panic!("cassette: {error}"));
        cassette[LEADER_LENGTH + 18] ^= 1;
        assert!(matches!(
            read_tokenised_basic(&cassette),
            Err(CassetteError::Checksum { .. })
        ));

        let valid = create_tokenised_basic("HELLO", &[0, 0])
            .unwrap_or_else(|error| panic!("cassette: {error}"));
        assert!(read_tokenised_basic(&valid[..valid.len() - 2]).is_err());

        let mut trailing = valid;
        trailing.push(0x99);
        assert!(matches!(
            read_tokenised_basic(&trailing),
            Err(CassetteError::TrailingData { .. })
        ));
    }

    #[test]
    fn machine_code_header_round_trips_addresses_and_payload() {
        let program = [0x86, 0x2a, 0x39];
        let cassette = create_machine_code("machine.bin", &program, 0x7000, 0x7002)
            .unwrap_or_else(|error| panic!("machine cassette: {error}"));
        let header = &cassette[LEADER_LENGTH + 3..LEADER_LENGTH + 18];
        assert_eq!(&header[..8], b"MACHINE ");
        assert_eq!(&header[8..], &[0x02, 0x00, 0x00, 0x70, 0x02, 0x70, 0x00]);

        let recovered = read_machine_code(&cassette)
            .unwrap_or_else(|error| panic!("read machine cassette: {error}"));
        assert_eq!(recovered.program, program);
        assert_eq!(recovered.load_address, 0x7000);
        assert_eq!(recovered.execution_address, 0x7002);
        assert_eq!(recovered.data_blocks, 1);
    }

    #[test]
    fn machine_code_rejects_empty_and_wrapping_payloads() {
        assert_eq!(
            create_machine_code("EMPTY", &[], 0x7000, 0x7000),
            Err(CassetteError::EmptyMachineCode)
        );
        assert_eq!(
            create_machine_code("WRAPS", &[0; 2], 0xffff, 0xffff),
            Err(CassetteError::MachineCodeAddressRange {
                load_address: 0xffff
            })
        );
    }

    proptest! {
        #[test]
        fn cassette_round_trip(program in proptest::collection::vec(any::<u8>(), 0..4096)) {
            let cassette = create_tokenised_basic("PROPERTY", &program)?;
            let recovered = read_tokenised_basic(&cassette)?;
            prop_assert_eq!(recovered.program, program);
            prop_assert_eq!(recovered.filename, "PROPERTY");
        }
    }
}
