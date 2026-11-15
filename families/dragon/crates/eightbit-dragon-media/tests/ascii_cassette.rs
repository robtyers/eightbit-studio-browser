//! Original text and independent cassette frames; no third-party programme data.
#![allow(clippy::unwrap_used, clippy::panic)]
use eightbit_dragon_media::{
    BasicMediaFormat, CassetteError, MediaFormatHint, WavError, create_ascii_basic,
    create_tokenised_basic, decode_wav_cassette, encode_cassette_wav, inspect_basic_media,
    read_ascii_basic, read_machine_code, read_tokenised_basic,
};

fn frame(output: &mut Vec<u8>, kind: u8, data: &[u8]) {
    let length = u8::try_from(data.len()).unwrap();
    output.extend([0x55; 128]);
    output.extend([0x3c, kind, length]);
    output.extend(data);
    output.push(data.iter().fold(kind.wrapping_add(length), |sum, byte| {
        sum.wrapping_add(*byte)
    }));
    output.push(0x55);
}
fn fixture(text: &[u8], chunk: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    frame(&mut bytes, 0, b"ORIGINAL\0\xff\xff\0\0\0\0");
    for data in text.chunks(chunk) {
        frame(&mut bytes, 1, data);
    }
    frame(&mut bytes, 0xff, &[]);
    bytes
}

#[test]
fn ascii_transport_retains_metadata_and_payload_across_block_boundaries() {
    let source = format!("10 REM {}\r\n20 END\n", "ORIGINAL".repeat(29));
    let encoded = create_ascii_basic("TEXT", &source).unwrap();
    let text = format!("\r10 REM {}\r20 END\r", "ORIGINAL".repeat(29));
    let file = read_ascii_basic(&encoded).unwrap();
    assert_eq!(file.filename, "TEXT");
    assert_eq!(file.text, text.as_bytes());
    assert_eq!(file.metadata.file_type, 0);
    assert_eq!(file.metadata.ascii_flag, 255);
    assert_eq!(file.metadata.gaps_flag, 255);
    assert_eq!(file.metadata.address_bytes, [0; 4]);
    assert!(read_tokenised_basic(&encoded).is_err());
    assert!(read_machine_code(&encoded).is_err());
    for chunk in [1, 7, 128, 255] {
        let bytes = fixture(text.as_bytes(), chunk);
        assert_eq!(read_ascii_basic(&bytes).unwrap().text, text.as_bytes());
    }
}

#[test]
fn ascii_import_applies_replacement_deletion_crlf_and_eof_tail_rules() {
    let text = b"\r20 END\r\n10 A=1\r10 A=2\r30 REM DELETE\r30\r99 INVALID TAIL";
    let bytes = fixture(text, 7);
    let inspection = inspect_basic_media(&bytes, MediaFormatHint::Auto).unwrap();
    assert_eq!(inspection.format, BasicMediaFormat::CassetteAsciiBasic);
    assert_eq!(inspection.format.id(), "dragon-cassette-ascii-basic");
    assert_eq!(inspection.source, "10 A=2\n20 END\n");
    assert!(inspection.read_only && inspection.canonical_round_trip);
    assert_eq!(inspection.warnings.len(), 1);
    assert_eq!(inspection.warnings[0].code, "DSM2010");
    assert_eq!(read_ascii_basic(&bytes).unwrap().text, text);
    let provenance = inspection.provenance("TEXT.cas").unwrap();
    assert_eq!(provenance.schema_version, 1);
    assert_eq!(provenance.input_format, "dragon-cassette-ascii-basic");
    assert_eq!(provenance.input_bytes, bytes.len());
    assert_eq!(provenance.input_sha256, inspection.input_sha256);
    assert!(inspect_basic_media(&bytes, MediaFormatHint::CassetteBasic).is_err());
}

#[test]
fn ascii_bounds_and_corruption_are_rejected_without_interpreting_payload() {
    let maximum = format!("10 REM {}\r", "X".repeat(244));
    assert_eq!(maximum.len(), 252);
    assert!(read_ascii_basic(&fixture(maximum.as_bytes(), 255)).is_ok());
    for text in [
        format!("10 REM {}\r", "X".repeat(245)).into_bytes(),
        b"10 REM \0\r".to_vec(),
        b"10 REM \t\r".to_vec(),
        b"10 REM \x80\r".to_vec(),
    ] {
        assert!(matches!(
            read_ascii_basic(&fixture(&text, 255)),
            Err(CassetteError::InvalidAsciiBasic { .. })
        ));
    }
    assert!(create_ascii_basic("BAD", "10 REM \u{00e9}\n").is_err());
    let good = fixture(b"10 END\r", 7);
    for length in [0, 1, 127, good.len() - 2] {
        assert!(read_ascii_basic(&good[..length]).is_err());
    }
    let file = read_ascii_basic(&good).unwrap();
    let mut corrupt = good.clone();
    corrupt[file.blocks[1].offset + 3] ^= 1;
    assert!(matches!(
        read_ascii_basic(&corrupt),
        Err(CassetteError::Checksum { .. })
    ));
    let mut trailing = good.clone();
    trailing.push(0);
    assert!(matches!(
        read_ascii_basic(&trailing),
        Err(CassetteError::TrailingData { .. })
    ));
    let mut flags = good;
    flags[128 + 3 + 10] = 0; // Recompute the header checksum independently.
    flags[128 + 3 + 15] = flags[129..146].iter().fold(0_u8, |a, b| a.wrapping_add(*b));
    assert_eq!(
        read_ascii_basic(&flags),
        Err(CassetteError::UnsupportedAsciiBasicFileType)
    );
    for text in [b"0 END\r".as_slice(), b"65535 END\r", b"BAD\r", b"10 END"] {
        assert!(
            inspect_basic_media(&fixture(text, 255), MediaFormatHint::CassetteAsciiBasic).is_err()
        );
    }
}

#[test]
fn ascii_wav_is_deterministic_has_gaps_and_retains_explicit_format_provenance() {
    let cas = fixture(b"\r10 PRINT\"ASCII\"\r20 END\r", 7);
    let wav = encode_cassette_wav(&cas).unwrap();
    assert_eq!(wav, encode_cassette_wav(&cas).unwrap());
    let decoded = decode_wav_cassette(&wav).unwrap();
    assert_eq!(decoded.cassette, cas);
    let file = read_ascii_basic(&cas).unwrap();
    let silent_frames = wav[44..]
        .chunks_exact(2)
        .filter(|sample| *sample == [0, 0])
        .count();
    assert_eq!(silent_frames, (file.blocks.len() + 1) * 22_050);
    let inspected = inspect_basic_media(&wav, MediaFormatHint::Auto).unwrap();
    assert_eq!(inspected.format, BasicMediaFormat::WavCassetteAsciiBasic);
    assert_eq!(inspected.source, "10 PRINT\"ASCII\"\n20 END\n");
    assert_eq!(inspected.provenance("TEXT.wav").unwrap().schema_version, 2);
    assert_eq!(inspected.format.id(), "dragon-wav-cassette-ascii-basic");
    assert!(inspect_basic_media(&wav, MediaFormatHint::WavCassetteBasic).is_err());
}

#[test]
fn mixed_ascii_and_tokenised_recordings_remain_ambiguous() {
    let ascii = fixture(b"10 END\r", 255);
    let tokenised = create_tokenised_basic("LINKED", &[0x1e, 9, 0, 10, 0x8a, 0, 0, 0]).unwrap();
    let first = encode_cassette_wav(&ascii).unwrap();
    let second = encode_cassette_wav(&tokenised).unwrap();
    let mut mixed = first.clone();
    mixed.extend_from_slice(&second[44..]);
    let riff_length = u32::try_from(mixed.len() - 8).unwrap();
    let data_length = u32::try_from(mixed.len() - 44).unwrap();
    mixed[4..8].copy_from_slice(&riff_length.to_le_bytes());
    mixed[40..44].copy_from_slice(&data_length.to_le_bytes());
    assert_eq!(
        decode_wav_cassette(&mixed),
        Err(WavError::MultipleProgrammes)
    );
}
