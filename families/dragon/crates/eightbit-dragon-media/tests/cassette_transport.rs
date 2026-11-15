//! Generated wire frames; records remain guest-owned opaque bytes.
#![allow(clippy::unwrap_used, clippy::panic)]
use eightbit_dragon_media::{
    CassetteError, MAX_TRANSPORT_BLOCKS, MAX_TRANSPORT_BYTES, MAX_TRANSPORT_FILES, WavError,
    create_machine_code, create_tokenised_basic, decode_wav_cassette,
    decode_wav_cassette_transport, encode_cassette_transport_wav, encode_cassette_wav,
    read_cassette_transport, read_tokenised_basic,
};

fn block(bytes: &mut Vec<u8>, kind: u8, data: &[u8]) {
    bytes.extend([0x55; 128]);
    compact_block(bytes, kind, data);
}
fn compact_block(bytes: &mut Vec<u8>, kind: u8, data: &[u8]) {
    let length = u8::try_from(data.len()).unwrap();
    bytes.extend([0x3c, kind, length]);
    bytes.extend(data);
    bytes.push(data.iter().fold(kind.wrapping_add(length), |sum, byte| {
        sum.wrapping_add(*byte)
    }));
    bytes.push(0x55);
}
fn data_file(records: &[&[u8]]) -> Vec<u8> {
    let mut bytes = Vec::new();
    block(&mut bytes, 0, b"RECORDS \x01\xff\xff\0\0\0\0");
    for record in records {
        block(&mut bytes, 1, record);
    }
    block(&mut bytes, 255, &[]);
    bytes
}

#[test]
fn mixed_files_keep_bytes_blocks_and_opaque_records_with_strict_imports() {
    let first = create_tokenised_basic("FIRST", &[0, 0]).unwrap();
    let data = data_file(&[b"\r42,\"ORIGINAL\"\r", &[], &[0, 255, b'\r']]);
    let machine = create_machine_code("LAST", &[0x39], 0x6000, 0x6000).unwrap();
    let tape = [first.clone(), data.clone(), machine].concat();
    let checked = read_cassette_transport(&tape).unwrap();
    assert_eq!(
        checked
            .files
            .iter()
            .map(|file| file.format)
            .collect::<Vec<_>>(),
        ["tokenised-basic", "ascii-data", "machine-code"]
    );
    assert_eq!(checked.files[1].metadata.file_type, 1);
    assert_eq!(checked.files[1].blocks.len(), 5);
    assert_eq!(checked.gap_offsets.len(), 6);
    for offset in checked.gap_offsets {
        assert_eq!(tape[offset - 1], 0x55);
    }
    assert!(matches!(
        read_tokenised_basic(&tape),
        Err(CassetteError::TrailingData { .. })
    ));
    assert!(matches!(
        encode_cassette_wav(&data),
        Err(WavError::UnsupportedCassette)
    ));
    let wav = encode_cassette_transport_wav(&tape).unwrap();
    assert_eq!(decode_wav_cassette_transport(&wav).unwrap().cassette, tape);
    assert!(matches!(
        decode_wav_cassette(&wav),
        Err(WavError::MultipleProgrammes)
    ));
    // Legacy single-file PCM remains byte-for-byte unchanged.
    assert_eq!(
        encode_cassette_transport_wav(&first).unwrap(),
        encode_cassette_wav(&first).unwrap()
    );
}

#[test]
fn empty_records_files_and_noncanonical_open_flags_are_transport_only() {
    let empty = data_file(&[]);
    let file = read_cassette_transport(&empty).unwrap();
    assert_eq!(file.files[0].blocks.len(), 2);
    let mut bytes = Vec::new();
    block(&mut bytes, 0, b"RAW\x01    \xfe\x01\x02\0\0\0\0");
    block(&mut bytes, 1, &[]);
    block(&mut bytes, 255, &[]);
    let checked = read_cassette_transport(&bytes).unwrap();
    assert_eq!(checked.files[0].filename, "RAW?");
    assert_eq!(checked.files[0].format, "ascii-data");
    // OPEN can read type-zero records too. Its opaque bytes are not necessarily
    // valid BASIC source; only source import applies that narrower contract.
    let mut type_zero = data_file(&[&[0, 255, b'\r']]);
    type_zero[139] = 0;
    type_zero[146] = type_zero[146].wrapping_sub(1);
    assert!(read_cassette_transport(&type_zero).is_ok());
    assert!(eightbit_dragon_media::read_ascii_basic(&type_zero).is_err());
    assert_eq!(
        decode_wav_cassette_transport(&encode_cassette_transport_wav(&empty).unwrap())
            .unwrap()
            .cassette,
        empty
    );
}

#[test]
fn framing_checksum_order_truncation_and_gap_padding_fail_closed() {
    let good = data_file(&[b"42\r"]);
    for length in [0, 1, 128, 147, good.len() - 2] {
        assert!(read_cassette_transport(&good[..length]).is_err());
    }
    let checked = read_cassette_transport(&good).unwrap();
    for info in &checked.files[0].blocks {
        let mut corrupt = good.clone();
        corrupt[info.offset + 3 + info.data_length] ^= 1;
        assert!(matches!(
            read_cassette_transport(&corrupt),
            Err(CassetteError::Checksum { .. })
        ));
        assert!(read_cassette_transport(&[good.clone(), corrupt].concat()).is_err());
    }
    let mut bad = Vec::new();
    block(&mut bad, 0, b"RECORDS \x01\xff\xff\0\0\0\0");
    block(&mut bad, 255, &[1]);
    assert!(matches!(
        read_cassette_transport(&bad),
        Err(CassetteError::UnexpectedBlock { .. })
    ));
    let mut no_padding = Vec::new();
    compact_block(&mut no_padding, 0, b"RECORDS \x01\xff\xff\0\0\0\0");
    no_padding.pop();
    compact_block(&mut no_padding, 255, &[]);
    assert!(matches!(
        read_cassette_transport(&no_padding),
        Err(CassetteError::MissingGapPadding { .. })
    ));
}

#[test]
fn bytes_files_blocks_and_output_duration_are_bounded() {
    let empty = data_file(&[]);
    let mut maximum = empty.clone();
    maximum.resize(MAX_TRANSPORT_BYTES, 0x55);
    assert!(read_cassette_transport(&maximum).is_ok());
    maximum.push(0x55);
    assert!(matches!(
        read_cassette_transport(&maximum),
        Err(CassetteError::TransportLimit)
    ));
    let mut files = empty.repeat(MAX_TRANSPORT_FILES);
    assert_eq!(
        read_cassette_transport(&files).unwrap().files.len(),
        MAX_TRANSPORT_FILES
    );
    files.extend(&empty);
    assert!(matches!(
        read_cassette_transport(&files),
        Err(CassetteError::TransportLimit)
    ));
    let mut blocks = Vec::new();
    compact_block(&mut blocks, 0, b"RECORDS \x01\xff\xff\0\0\0\0");
    for _ in 0..MAX_TRANSPORT_BLOCKS - 2 {
        compact_block(&mut blocks, 1, &[]);
    }
    compact_block(&mut blocks, 255, &[]);
    assert_eq!(
        read_cassette_transport(&blocks).unwrap().files[0]
            .blocks
            .len(),
        MAX_TRANSPORT_BLOCKS
    );
    assert!(matches!(
        encode_cassette_transport_wav(&blocks),
        Err(WavError::TooLarge | WavError::TooLong)
    ));
    blocks.splice(20..20, [0x3c, 1, 0, 1, 0x55]);
    assert!(matches!(
        read_cassette_transport(&blocks),
        Err(CassetteError::TransportLimit)
    ));
}
