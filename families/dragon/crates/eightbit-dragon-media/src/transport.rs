//! Physical transport validation, separate from single-programme source import.
use crate::{
    CassetteBlockInfo, CassetteError, CassetteMetadata, DATA_BLOCK, EOF_BLOCK, FILENAME_BLOCK,
    FILENAME_BLOCK_LENGTH, LEADER, block_info, read_block,
};

/// Volatile browser/native tape capacity, including leaders and block framing.
pub const MAX_TRANSPORT_BYTES: usize = 1024 * 1024;
/// Bound metadata independently of compressed or very short file contents.
pub const MAX_TRANSPORT_FILES: usize = 256;
/// Bound empty-block metadata and generated gap durations independently of bytes.
pub const MAX_TRANSPORT_BLOCKS: usize = 8192;

/// Checked original byte offsets; no payload is interpreted or rewritten.
#[derive(Debug)]
pub struct CassetteTransport {
    /// Complete files in original tape order.
    pub files: Vec<CassetteTransportFile>,
    /// Insert a half-second low interval before these byte offsets.
    pub gap_offsets: Vec<usize>,
}

/// One complete, checksum-valid file in a physical tape stream.
#[derive(Debug)]
pub struct CassetteTransportFile {
    /// Printable display label; the original eight wire bytes remain untouched.
    pub filename: String,
    /// Existing programme labels or `ascii-data` for opaque gapped records.
    pub format: &'static str,
    /// Exact type, flags and address bytes from the filename block.
    pub metadata: CassetteMetadata,
    /// Checked block offsets in the original image.
    pub blocks: Vec<CassetteBlockInfo>,
}

/// Check all files before replacing a mounted tape. Existing single-programme
/// readers deliberately retain their trailing-data and source-import contracts.
/// Gapped ASCII data is opaque: record parsing, EOF and errors belong to the guest.
pub fn read_cassette_transport(bytes: &[u8]) -> Result<CassetteTransport, CassetteError> {
    if bytes.len() > MAX_TRANSPORT_BYTES {
        return Err(CassetteError::TransportLimit);
    }
    let mut cursor = 0;
    let mut files = Vec::new();
    let mut gap_offsets = Vec::new();
    let mut block_count = 0;
    loop {
        if files.len() == MAX_TRANSPORT_FILES {
            return Err(CassetteError::TransportLimit);
        }
        let file = read_file(bytes, &mut cursor, &mut block_count)?;
        if let Some(previous) = files.last() {
            let previous: &CassetteTransportFile = previous;
            gap_offsets.push(gap_offset(
                bytes,
                previous.blocks.last().ok_or(CassetteError::MissingEof)?,
                file.blocks[0].offset,
            )?);
        }
        if file.metadata.gaps_flag != 0 {
            for blocks in file.blocks.windows(2) {
                gap_offsets.push(gap_offset(bytes, &blocks[0], blocks[1].offset)?);
            }
        }
        files.push(file);
        if bytes[cursor..].iter().all(|byte| *byte == LEADER) {
            return Ok(CassetteTransport { files, gap_offsets });
        }
    }
}

/// WAV recovery needs a complete file boundary before accepting another file.
pub(crate) fn transport_prefix_len(bytes: &[u8]) -> Result<usize, CassetteError> {
    if bytes.len() > MAX_TRANSPORT_BYTES {
        return Err(CassetteError::TransportLimit);
    }
    let mut cursor = 0;
    read_file(bytes, &mut cursor, &mut 0)?;
    Ok(cursor)
}

fn gap_offset(
    bytes: &[u8],
    previous: &CassetteBlockInfo,
    next: usize,
) -> Result<usize, CassetteError> {
    // Retain the entire checksum cycle and final zero bit of its padding byte.
    let end = previous.offset + 4 + previous.data_length;
    if end >= next || bytes.get(end) != Some(&LEADER) {
        return Err(CassetteError::MissingGapPadding { offset: end });
    }
    Ok(end + 1)
}

fn read_file(
    bytes: &[u8],
    cursor: &mut usize,
    block_count: &mut usize,
) -> Result<CassetteTransportFile, CassetteError> {
    if *block_count >= MAX_TRANSPORT_BLOCKS {
        return Err(CassetteError::TransportLimit);
    }
    let first = read_block(bytes, cursor)?;
    if first.block_type != FILENAME_BLOCK || first.data.len() != FILENAME_BLOCK_LENGTH {
        return Err(CassetteError::InvalidFilenameBlock {
            block_type: first.block_type,
            length: first.data.len(),
        });
    }
    let format = match (first.data[8], first.data[9], first.data[10]) {
        (0, 0, 0) => "tokenised-basic",
        (0, 255, 255) => "ascii-basic",
        (2, 0, 0) => "machine-code",
        // The observed OPEN input contract accepts any type with both flags
        // non-zero. Do not treat its records as BASIC source or normalise bytes.
        (_, ascii, gaps) if ascii != 0 && gaps != 0 => "ascii-data",
        _ => return Err(CassetteError::UnsupportedTransport),
    };
    let filename = first.data[..8]
        .iter()
        .map(|byte| {
            if (32..=126).contains(byte) {
                char::from(*byte)
            } else {
                '?'
            }
        })
        .collect::<String>()
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
    *block_count += 1;
    let mut payload_length = 0_usize;
    loop {
        if *block_count >= MAX_TRANSPORT_BLOCKS {
            return Err(CassetteError::TransportLimit);
        }
        let block = read_block(bytes, cursor)?;
        *block_count += 1;
        blocks.push(block_info(blocks.len(), &block));
        match block.block_type {
            DATA_BLOCK => {
                payload_length += block.data.len();
            }
            EOF_BLOCK if block.data.is_empty() => break,
            block_type => {
                return Err(CassetteError::UnexpectedBlock {
                    block_type,
                    offset: block.offset,
                });
            }
        }
        if *cursor >= bytes.len() {
            return Err(CassetteError::MissingEof);
        }
    }
    if format == "machine-code" {
        if payload_length == 0 {
            return Err(CassetteError::EmptyMachineCode);
        }
        let load_address =
            u16::from_be_bytes([metadata.address_bytes[2], metadata.address_bytes[3]]);
        if payload_length
            .checked_sub(1)
            .and_then(|value| u16::try_from(value).ok())
            .and_then(|value| load_address.checked_add(value))
            .is_none()
        {
            return Err(CassetteError::MachineCodeAddressRange { load_address });
        }
    }
    Ok(CassetteTransportFile {
        filename,
        format,
        metadata,
        blocks,
    })
}
