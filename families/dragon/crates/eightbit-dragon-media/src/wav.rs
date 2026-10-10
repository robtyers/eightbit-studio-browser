//! Bounded PCM WAV transport for validated Dragon cassette byte streams.

use crate::{read_machine_code, read_tokenised_basic, supported_cassette_prefix_len};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Maximum accepted or generated WAV size.
pub const MAX_WAV_BYTES: usize = 64 * 1024 * 1024;

const MIN_SAMPLE_RATE: u32 = 8_000;
const MAX_SAMPLE_RATE: u32 = 96_000;
const MAX_DURATION_SECONDS: u64 = 15 * 60;
const OUTPUT_SAMPLE_RATE: u32 = 44_100;
const OUTPUT_BITS_PER_SAMPLE: u16 = 16;
const OUTPUT_CHANNELS: u16 = 1;
const SILENCE_MILLISECONDS: u32 = 500;
const OUTPUT_AMPLITUDE: i16 = 24_576;
const DRAGON_MASTER_TICKS_PER_SECOND: u64 = 14_318_180;
const BIT_ZERO_TICKS: u64 = 813 * 16;
const BIT_ONE_TICKS: u64 = 435 * 16;
const MAX_TRANSITIONS: usize = 8 * 1024 * 1024;

/// Bounded WAV properties exposed during inspection and retained in provenance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WavCassetteMetadata {
    /// PCM sampling frequency.
    pub sample_rate: u32,
    /// Source channel count.
    pub channels: u16,
    /// Source PCM bit depth.
    pub bits_per_sample: u16,
    /// Complete interleaved PCM frame count.
    pub frames: usize,
    /// Rounded-down duration in milliseconds.
    pub duration_ms: u64,
    /// One-based channel used for recovery, or `None` when identical data was recovered from both.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decoded_channel: Option<u16>,
}

/// A checksum-validated cassette byte stream recovered from PCM audio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedWavCassette {
    /// Exact recovered `.cas` bytes, including leaders and checked blocks.
    pub cassette: Vec<u8>,
    /// Bounded source-audio facts.
    pub metadata: WavCassetteMetadata,
}

/// Malformed, unsupported or undecodable WAV cassette audio.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WavError {
    /// Input or deterministic output exceeds the WAV boundary.
    #[error("DSM1100: WAV exceeds the {MAX_WAV_BYTES}-byte limit")]
    TooLarge,
    /// RIFF/WAVE framing or chunk geometry is invalid.
    #[error("DSM1101: malformed RIFF/WAVE container: {0}")]
    InvalidContainer(&'static str),
    /// PCM properties are outside the deliberately supported matrix.
    #[error("DSM1102: unsupported WAV encoding: {0}")]
    UnsupportedEncoding(&'static str),
    /// Audio duration exceeds the bounded decoder limit.
    #[error("DSM1103: WAV exceeds the 15-minute duration limit")]
    TooLong,
    /// No channel yielded one checksum-valid supported cassette file.
    #[error("DSM1104: WAV contains no recoverable supported Dragon cassette programme")]
    NoSupportedCassette,
    /// Different channels yielded different valid cassette programmes.
    #[error("DSM1105: WAV channels contain different valid Dragon cassette programmes")]
    AmbiguousChannels,
    /// Encoding accepts only a cassette already validated by the media core.
    #[error("DSM1106: input is not a supported Dragon cassette image")]
    UnsupportedCassette,
    /// More than one checksum-valid programme was recovered from a channel.
    #[error("DSM1107: WAV contains more than one valid Dragon cassette programme")]
    MultipleProgrammes,
}

#[derive(Debug, Clone, Copy)]
struct PcmFormat {
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    block_align: usize,
}

/// Render one supported Dragon cassette image as deterministic mono PCM WAV.
pub fn encode_cassette_wav(cassette: &[u8]) -> Result<Vec<u8>, WavError> {
    if read_tokenised_basic(cassette).is_err() && read_machine_code(cassette).is_err() {
        return Err(WavError::UnsupportedCassette);
    }

    let silence_frames =
        usize::try_from(u64::from(OUTPUT_SAMPLE_RATE) * u64::from(SILENCE_MILLISECONDS) / 1_000)
            .map_err(|_| WavError::TooLarge)?;
    let longest_cycle = cycle_samples_upper_bound(BIT_ZERO_TICKS)?;
    let signal_frames = cassette
        .len()
        .checked_mul(8)
        .and_then(|bits| bits.checked_mul(longest_cycle))
        .ok_or(WavError::TooLarge)?;
    let estimated_frames = silence_frames
        .checked_mul(2)
        .and_then(|silence| silence.checked_add(signal_frames))
        .ok_or(WavError::TooLarge)?;
    let estimated_bytes = estimated_frames
        .checked_mul(usize::from(OUTPUT_BITS_PER_SAMPLE / 8))
        .and_then(|data| data.checked_add(44))
        .ok_or(WavError::TooLarge)?;
    if estimated_bytes > MAX_WAV_BYTES {
        return Err(WavError::TooLarge);
    }

    let mut samples = Vec::with_capacity(estimated_frames);
    samples.resize(silence_frames, 0_i16);
    let mut remainder = 0_u64;
    for byte in cassette {
        for bit_index in 0..8 {
            let ticks = if byte & (1 << bit_index) == 0 {
                BIT_ZERO_TICKS
            } else {
                BIT_ONE_TICKS
            };
            let numerator = u64::from(OUTPUT_SAMPLE_RATE)
                .checked_mul(ticks)
                .and_then(|value| value.checked_add(remainder))
                .ok_or(WavError::TooLarge)?;
            let cycle_frames = usize::try_from(numerator / DRAGON_MASTER_TICKS_PER_SECOND)
                .map_err(|_| WavError::TooLarge)?;
            remainder = numerator % DRAGON_MASTER_TICKS_PER_SECOND;
            if cycle_frames < 2 {
                return Err(WavError::UnsupportedEncoding(
                    "sample rate cannot represent Dragon cassette cycles",
                ));
            }
            let positive_frames = cycle_frames.div_ceil(2);
            samples.extend(std::iter::repeat_n(OUTPUT_AMPLITUDE, positive_frames));
            samples.extend(std::iter::repeat_n(
                -OUTPUT_AMPLITUDE,
                cycle_frames - positive_frames,
            ));
        }
    }
    samples.resize(
        samples
            .len()
            .checked_add(silence_frames)
            .ok_or(WavError::TooLarge)?,
        0,
    );

    encode_pcm_wav_mono(&samples, OUTPUT_SAMPLE_RATE)
}

/// Encode bounded signed 16-bit mono samples as a canonical PCM WAV.
pub fn encode_pcm_wav_mono(samples: &[i16], sample_rate: u32) -> Result<Vec<u8>, WavError> {
    if !(MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE).contains(&sample_rate) {
        return Err(WavError::UnsupportedEncoding(
            "sample rate must be between 8 kHz and 96 kHz",
        ));
    }
    if samples.is_empty() {
        return Err(WavError::InvalidContainer("PCM data is empty"));
    }
    let frames = u64::try_from(samples.len()).map_err(|_| WavError::TooLarge)?;
    if frames > u64::from(sample_rate) * MAX_DURATION_SECONDS {
        return Err(WavError::TooLong);
    }
    let data_bytes = samples.len().checked_mul(2).ok_or(WavError::TooLarge)?;
    let riff_size = u32::try_from(data_bytes.checked_add(36).ok_or(WavError::TooLarge)?)
        .map_err(|_| WavError::TooLarge)?;
    let data_size = u32::try_from(data_bytes).map_err(|_| WavError::TooLarge)?;
    let byte_rate = sample_rate * u32::from(OUTPUT_CHANNELS) * 2;
    let mut output = Vec::with_capacity(data_bytes + 44);
    output.extend_from_slice(b"RIFF");
    output.extend_from_slice(&riff_size.to_le_bytes());
    output.extend_from_slice(b"WAVEfmt ");
    output.extend_from_slice(&16_u32.to_le_bytes());
    output.extend_from_slice(&1_u16.to_le_bytes());
    output.extend_from_slice(&OUTPUT_CHANNELS.to_le_bytes());
    output.extend_from_slice(&sample_rate.to_le_bytes());
    output.extend_from_slice(&byte_rate.to_le_bytes());
    output.extend_from_slice(&2_u16.to_le_bytes());
    output.extend_from_slice(&OUTPUT_BITS_PER_SAMPLE.to_le_bytes());
    output.extend_from_slice(b"data");
    output.extend_from_slice(&data_size.to_le_bytes());
    for sample in samples {
        output.extend_from_slice(&sample.to_le_bytes());
    }
    if output.len() > MAX_WAV_BYTES {
        return Err(WavError::TooLarge);
    }
    Ok(output)
}

/// Recover exactly one supported Dragon cassette image from bounded PCM WAV audio.
pub fn decode_wav_cassette(bytes: &[u8]) -> Result<DecodedWavCassette, WavError> {
    let (format, data, mut metadata) = parse_wav(bytes)?;
    let mut recovered = Vec::<(u16, Vec<u8>)>::new();
    for channel in 0..format.channels {
        if let Some(cassette) = decode_channel(data, format, usize::from(channel))? {
            recovered.push((channel + 1, cassette));
        }
    }
    if recovered.is_empty() {
        return Err(WavError::NoSupportedCassette);
    }
    let first = &recovered[0].1;
    if recovered.iter().skip(1).any(|(_, bytes)| bytes != first) {
        return Err(WavError::AmbiguousChannels);
    }
    metadata.decoded_channel = if recovered.len() == usize::from(format.channels) {
        None
    } else {
        Some(recovered[0].0)
    };
    Ok(DecodedWavCassette {
        cassette: first.clone(),
        metadata,
    })
}

fn cycle_samples_upper_bound(ticks: u64) -> Result<usize, WavError> {
    let numerator = u64::from(OUTPUT_SAMPLE_RATE)
        .checked_mul(ticks)
        .and_then(|value| value.checked_add(DRAGON_MASTER_TICKS_PER_SECOND - 1))
        .ok_or(WavError::TooLarge)?;
    usize::try_from(numerator / DRAGON_MASTER_TICKS_PER_SECOND).map_err(|_| WavError::TooLarge)
}

fn parse_wav(bytes: &[u8]) -> Result<(PcmFormat, &[u8], WavCassetteMetadata), WavError> {
    if bytes.len() > MAX_WAV_BYTES {
        return Err(WavError::TooLarge);
    }
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(WavError::InvalidContainer("missing RIFF/WAVE header"));
    }
    let declared = usize::try_from(u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]))
        .map_err(|_| WavError::TooLarge)?;
    if declared.checked_add(8) != Some(bytes.len()) {
        return Err(WavError::InvalidContainer(
            "RIFF size does not match the file",
        ));
    }

    let mut cursor = 12_usize;
    let mut format = None;
    let mut data = None;
    while cursor < bytes.len() {
        if bytes.len().saturating_sub(cursor) < 8 {
            return Err(WavError::InvalidContainer("truncated chunk header"));
        }
        let id = &bytes[cursor..cursor + 4];
        let size = usize::try_from(u32::from_le_bytes([
            bytes[cursor + 4],
            bytes[cursor + 5],
            bytes[cursor + 6],
            bytes[cursor + 7],
        ]))
        .map_err(|_| WavError::TooLarge)?;
        let start = cursor.checked_add(8).ok_or(WavError::TooLarge)?;
        let end = start.checked_add(size).ok_or(WavError::TooLarge)?;
        if end > bytes.len() {
            return Err(WavError::InvalidContainer("chunk extends beyond the file"));
        }
        match id {
            b"fmt " => {
                if format.is_some() {
                    return Err(WavError::InvalidContainer("duplicate fmt chunk"));
                }
                format = Some(parse_format(&bytes[start..end])?);
            }
            b"data" => {
                if data.is_some() {
                    return Err(WavError::InvalidContainer("duplicate data chunk"));
                }
                data = Some(&bytes[start..end]);
            }
            _ => {}
        }
        cursor = end.checked_add(size & 1).ok_or(WavError::TooLarge)?;
        if cursor > bytes.len() {
            return Err(WavError::InvalidContainer("missing odd-chunk padding byte"));
        }
    }
    let format = format.ok_or(WavError::InvalidContainer("missing fmt chunk"))?;
    let data = data.ok_or(WavError::InvalidContainer("missing data chunk"))?;
    if data.is_empty() || !data.len().is_multiple_of(format.block_align) {
        return Err(WavError::InvalidContainer(
            "PCM data is empty or ends with a partial frame",
        ));
    }
    let frames = data.len() / format.block_align;
    let frames_u64 = u64::try_from(frames).map_err(|_| WavError::TooLarge)?;
    if frames_u64 > u64::from(format.sample_rate) * MAX_DURATION_SECONDS {
        return Err(WavError::TooLong);
    }
    let duration_ms =
        frames_u64.checked_mul(1_000).ok_or(WavError::TooLarge)? / u64::from(format.sample_rate);
    let metadata = WavCassetteMetadata {
        sample_rate: format.sample_rate,
        channels: format.channels,
        bits_per_sample: format.bits_per_sample,
        frames,
        duration_ms,
        decoded_channel: None,
    };
    Ok((format, data, metadata))
}

fn parse_format(bytes: &[u8]) -> Result<PcmFormat, WavError> {
    if bytes.len() < 16 {
        return Err(WavError::InvalidContainer(
            "fmt chunk is shorter than 16 bytes",
        ));
    }
    let encoding = u16::from_le_bytes([bytes[0], bytes[1]]);
    if encoding != 1 {
        return Err(WavError::UnsupportedEncoding(
            "only integer PCM format 1 is supported",
        ));
    }
    let channels = u16::from_le_bytes([bytes[2], bytes[3]]);
    if !matches!(channels, 1 | 2) {
        return Err(WavError::UnsupportedEncoding(
            "only mono or stereo PCM is supported",
        ));
    }
    let sample_rate = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    if !(MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE).contains(&sample_rate) {
        return Err(WavError::UnsupportedEncoding(
            "sample rate must be between 8 kHz and 96 kHz",
        ));
    }
    let byte_rate = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    let block_align = u16::from_le_bytes([bytes[12], bytes[13]]);
    let bits_per_sample = u16::from_le_bytes([bytes[14], bytes[15]]);
    if !matches!(bits_per_sample, 8 | 16) {
        return Err(WavError::UnsupportedEncoding(
            "only 8-bit or 16-bit PCM is supported",
        ));
    }
    let expected_align = channels
        .checked_mul(bits_per_sample / 8)
        .ok_or(WavError::InvalidContainer("PCM block alignment overflow"))?;
    let expected_rate = sample_rate
        .checked_mul(u32::from(expected_align))
        .ok_or(WavError::InvalidContainer("PCM byte rate overflow"))?;
    if block_align != expected_align || byte_rate != expected_rate {
        return Err(WavError::InvalidContainer(
            "PCM byte rate or block alignment is inconsistent",
        ));
    }
    Ok(PcmFormat {
        sample_rate,
        channels,
        bits_per_sample,
        block_align: usize::from(block_align),
    })
}

fn decode_channel(
    data: &[u8],
    format: PcmFormat,
    channel: usize,
) -> Result<Option<Vec<u8>>, WavError> {
    let frames = data.len() / format.block_align;
    let mut minimum = i32::MAX;
    let mut maximum = i32::MIN;
    for frame in 0..frames {
        let sample = sample(data, format, frame, channel);
        minimum = minimum.min(sample);
        maximum = maximum.max(sample);
    }
    let span = maximum.saturating_sub(minimum);
    if span < 2_048 {
        return Ok(None);
    }
    let centre = minimum + span / 2;
    let hysteresis = (span / 100).max(256);
    let mut state = 0_i8;
    let mut crossings = Vec::new();
    let mut last_active = None;
    for frame in 0..frames {
        let centred = sample(data, format, frame, channel) - centre;
        if centred.abs() >= hysteresis {
            last_active = Some(frame);
        }
        if state <= 0 && centred >= hysteresis {
            state = 1;
            crossings.push(frame);
        } else if state >= 0 && centred <= -hysteresis {
            state = -1;
            crossings.push(frame);
        }
        if crossings.len() > MAX_TRANSITIONS {
            return Err(WavError::TooLarge);
        }
    }
    let Some(last_active) = last_active else {
        return Ok(None);
    };
    let end = last_active.checked_add(1).ok_or(WavError::TooLarge)?;
    if crossings.last().copied() != Some(end) {
        crossings.push(end);
    }
    if crossings.len() < 32 {
        return Ok(None);
    }

    for phase in 0..2 {
        let widths = cycle_widths(&crossings, phase);
        let Some(threshold) = cycle_threshold(&widths, format.sample_rate) else {
            continue;
        };
        let bits = widths
            .iter()
            .map(|width| u8::from(*width < threshold))
            .collect::<Vec<_>>();
        for bit_offset in 0..8 {
            let packed = pack_lsb_bits(&bits[bit_offset.min(bits.len())..]);
            if let Some(cassette) = find_supported_cassette(&packed)? {
                return Ok(Some(cassette));
            }
        }
    }
    Ok(None)
}

fn sample(data: &[u8], format: PcmFormat, frame: usize, channel: usize) -> i32 {
    let bytes_per_sample = usize::from(format.bits_per_sample / 8);
    let offset = frame * format.block_align + channel * bytes_per_sample;
    if format.bits_per_sample == 8 {
        (i32::from(data[offset]) - 128) * 256
    } else {
        i32::from(i16::from_le_bytes([data[offset], data[offset + 1]]))
    }
}

fn cycle_widths(crossings: &[usize], phase: usize) -> Vec<usize> {
    let mut widths = Vec::with_capacity(crossings.len() / 2);
    let mut index = phase;
    while index + 2 < crossings.len() {
        widths.push(crossings[index + 2] - crossings[index]);
        index += 2;
    }
    widths
}

fn cycle_threshold(widths: &[usize], sample_rate: u32) -> Option<usize> {
    let minimum = (sample_rate / 4_000).max(2) as usize;
    let maximum = (sample_rate / 500).max(3) as usize;
    let mut training = widths
        .iter()
        .copied()
        .filter(|width| (minimum..=maximum).contains(width))
        .take(4_096)
        .collect::<Vec<_>>();
    if training.len() < 32 {
        return None;
    }
    training.sort_unstable();
    let mut short = training[training.len() / 4];
    let mut long = training[training.len() * 3 / 4];
    if short == long {
        return None;
    }
    for _ in 0..8 {
        let split = short.saturating_add(long) / 2;
        let mut short_sum = 0_usize;
        let mut short_count = 0_usize;
        let mut long_sum = 0_usize;
        let mut long_count = 0_usize;
        for width in &training {
            if *width < split {
                short_sum = short_sum.saturating_add(*width);
                short_count += 1;
            } else {
                long_sum = long_sum.saturating_add(*width);
                long_count += 1;
            }
        }
        if short_count == 0 || long_count == 0 {
            return None;
        }
        short = short_sum / short_count;
        long = long_sum / long_count;
    }
    if short.checked_mul(5)? >= long.checked_mul(4)? || long > short.checked_mul(3)? {
        return None;
    }
    Some(short.saturating_add(long) / 2)
}

fn pack_lsb_bits(bits: &[u8]) -> Vec<u8> {
    bits.chunks_exact(8)
        .map(|chunk| {
            chunk
                .iter()
                .enumerate()
                .fold(0_u8, |byte, (index, bit)| byte | (*bit << index))
        })
        .collect()
}

fn find_supported_cassette(bytes: &[u8]) -> Result<Option<Vec<u8>>, WavError> {
    let mut recovered = None;
    let mut offset = 0_usize;
    while offset < bytes.len() {
        let Some(relative) = bytes[offset..]
            .windows(2)
            .position(|pair| pair == [0x55, 0x3c])
        else {
            break;
        };
        let sync_leader = offset + relative;
        let leader_start = bytes[..sync_leader]
            .iter()
            .rposition(|byte| *byte != 0x55)
            .map_or(0, |position| position + 1);
        let candidate = &bytes[leader_start..];
        if let Some(consumed) = supported_cassette_prefix_len(candidate) {
            if recovered.is_some() {
                return Err(WavError::MultipleProgrammes);
            }
            // Canonical cassette writers terminate the EOF block with one leader byte.
            // Preserve that byte when present, but leave any subsequent leader run for
            // another independently validated filename block.
            let end = consumed
                .checked_add(usize::from(candidate.get(consumed) == Some(&0x55)))
                .ok_or(WavError::TooLarge)?;
            recovered = Some(candidate[..end].to_vec());
            offset = leader_start.checked_add(end).ok_or(WavError::TooLarge)?;
        } else {
            offset = sync_leader.checked_add(2).ok_or(WavError::TooLarge)?;
        }
    }
    Ok(recovered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{create_machine_code, create_tokenised_basic};

    #[test]
    fn deterministic_pcm_round_trip_preserves_basic_cassette() {
        let cassette = create_tokenised_basic("AUDIO", &[0x1e, 0x08, 0x00, 0x0a, 0x80, 0x00, 0, 0])
            .unwrap_or_else(|error| panic!("cassette: {error}"));
        let first = encode_cassette_wav(&cassette).unwrap_or_else(|error| panic!("wav: {error}"));
        let second = encode_cassette_wav(&cassette).unwrap_or_else(|error| panic!("wav: {error}"));
        assert_eq!(first, second);
        assert_eq!(&first[..4], b"RIFF");
        assert_eq!(&first[8..12], b"WAVE");
        let decoded = decode_wav_cassette(&first).unwrap_or_else(|error| panic!("decode: {error}"));
        assert_eq!(decoded.cassette, cassette);
        assert_eq!(decoded.metadata.sample_rate, OUTPUT_SAMPLE_RATE);
        assert_eq!(decoded.metadata.channels, 1);
        assert_eq!(decoded.metadata.bits_per_sample, 16);
    }

    #[test]
    fn machine_code_cassette_round_trips_through_wav() {
        let cassette = create_machine_code("MACHINE", &[0x86, 0x2a, 0x39], 0x7000, 0x7000)
            .unwrap_or_else(|error| panic!("cassette: {error}"));
        let wav = encode_cassette_wav(&cassette).unwrap_or_else(|error| panic!("wav: {error}"));
        assert_eq!(
            decode_wav_cassette(&wav)
                .unwrap_or_else(|error| panic!("decode: {error}"))
                .cassette,
            cassette
        );
    }

    #[test]
    fn rejects_malformed_and_unsupported_wav() {
        assert_eq!(
            decode_wav_cassette(b"not wave"),
            Err(WavError::InvalidContainer("missing RIFF/WAVE header"))
        );
        let cassette = create_tokenised_basic("AUDIO", &[0, 0]).unwrap();
        let mut wav = encode_cassette_wav(&cassette).unwrap();
        wav[20..22].copy_from_slice(&3_u16.to_le_bytes());
        assert_eq!(
            decode_wav_cassette(&wav),
            Err(WavError::UnsupportedEncoding(
                "only integer PCM format 1 is supported"
            ))
        );
    }

    #[test]
    fn selects_the_valid_stereo_channel_without_mixing() {
        let cassette = create_tokenised_basic("STEREO", &[0, 0]).unwrap();
        let mono = encode_cassette_wav(&cassette).unwrap();
        let data = &mono[44..];
        let frames = data.len() / 2;
        let stereo_data_size = u32::try_from(frames * 4).unwrap();
        let mut stereo = mono[..44].to_vec();
        stereo[22..24].copy_from_slice(&2_u16.to_le_bytes());
        stereo[28..32].copy_from_slice(&(OUTPUT_SAMPLE_RATE * 4).to_le_bytes());
        stereo[32..34].copy_from_slice(&4_u16.to_le_bytes());
        stereo[4..8].copy_from_slice(&(stereo_data_size + 36).to_le_bytes());
        stereo[40..44].copy_from_slice(&stereo_data_size.to_le_bytes());
        stereo.truncate(44);
        for sample in data.chunks_exact(2) {
            stereo.extend_from_slice(sample);
            stereo.extend_from_slice(&0_i16.to_le_bytes());
        }
        let decoded = decode_wav_cassette(&stereo).unwrap();
        assert_eq!(decoded.cassette, cassette);
        assert_eq!(decoded.metadata.decoded_channel, Some(1));
    }

    #[test]
    fn tolerates_inversion_dc_offset_and_stable_speed_change() {
        let cassette = create_tokenised_basic("ROBUST", &[0, 0]).unwrap();
        let wav = encode_cassette_wav(&cassette).unwrap();
        let transformed = rewrite_mono_16(
            &wav,
            pcm16(&wav)
                .into_iter()
                .map(|sample| {
                    i16::try_from((-i32::from(sample) + 1_500).clamp(-32_768, 32_767)).unwrap()
                })
                .collect(),
        );
        let slowed_samples = {
            let samples = pcm16(&transformed);
            let output_frames = samples.len() * 11 / 10;
            (0..output_frames)
                .map(|index| samples[index * 10 / 11])
                .collect::<Vec<_>>()
        };
        let slowed = rewrite_mono_16(&transformed, slowed_samples);
        assert_eq!(decode_wav_cassette(&slowed).unwrap().cassette, cassette);
    }

    #[test]
    fn accepts_unsigned_eight_bit_pcm() {
        let cassette = create_tokenised_basic("EIGHTBIT", &[0, 0]).unwrap();
        let wav = encode_cassette_wav(&cassette).unwrap();
        let samples = pcm16(&wav)
            .into_iter()
            .map(|sample| u8::try_from((i32::from(sample) + 32_768) / 256).unwrap())
            .collect::<Vec<_>>();
        let data_size = u32::try_from(samples.len()).unwrap();
        let mut eight_bit = wav[..44].to_vec();
        eight_bit[4..8].copy_from_slice(&(data_size + 36).to_le_bytes());
        eight_bit[28..32].copy_from_slice(&OUTPUT_SAMPLE_RATE.to_le_bytes());
        eight_bit[32..34].copy_from_slice(&1_u16.to_le_bytes());
        eight_bit[34..36].copy_from_slice(&8_u16.to_le_bytes());
        eight_bit[40..44].copy_from_slice(&data_size.to_le_bytes());
        eight_bit.extend_from_slice(&samples);
        if samples.len() % 2 == 1 {
            eight_bit.push(0);
            let riff_size = u32::try_from(eight_bit.len() - 8).unwrap();
            eight_bit[4..8].copy_from_slice(&riff_size.to_le_bytes());
        }
        assert_eq!(decode_wav_cassette(&eight_bit).unwrap().cassette, cassette);
    }

    #[test]
    fn rejects_stereo_channels_with_different_valid_programmes() {
        let left_cassette = create_tokenised_basic("LEFT", &[0, 0]).unwrap();
        let right_cassette = create_machine_code("RIGHT", &[0x39], 0x7000, 0x7000).unwrap();
        let left = encode_cassette_wav(&left_cassette).unwrap();
        let right = encode_cassette_wav(&right_cassette).unwrap();
        let stereo = stereo_pcm16(&left, &right);
        assert_eq!(
            decode_wav_cassette(&stereo),
            Err(WavError::AmbiguousChannels)
        );
    }

    #[test]
    fn rejects_multiple_valid_programmes_in_one_channel() {
        let first = create_tokenised_basic("FIRST", &[0, 0]).unwrap();
        let second = create_machine_code("SECOND", &[0x39], 0x7000, 0x7000).unwrap();
        let mut samples = pcm16(&encode_cassette_wav(&first).unwrap());
        samples.extend(pcm16(&encode_cassette_wav(&second).unwrap()));
        let combined = rewrite_mono_16(&encode_cassette_wav(&first).unwrap(), samples);
        assert_eq!(
            decode_wav_cassette(&combined),
            Err(WavError::MultipleProgrammes)
        );
    }

    fn pcm16(wav: &[u8]) -> Vec<i16> {
        wav[44..]
            .chunks_exact(2)
            .map(|sample| i16::from_le_bytes([sample[0], sample[1]]))
            .collect()
    }

    fn rewrite_mono_16(wav: &[u8], samples: Vec<i16>) -> Vec<u8> {
        let data_size = u32::try_from(samples.len() * 2).unwrap();
        let mut rewritten = wav[..44].to_vec();
        rewritten[4..8].copy_from_slice(&(data_size + 36).to_le_bytes());
        rewritten[40..44].copy_from_slice(&data_size.to_le_bytes());
        for sample in samples {
            rewritten.extend_from_slice(&sample.to_le_bytes());
        }
        rewritten
    }

    fn stereo_pcm16(left: &[u8], right: &[u8]) -> Vec<u8> {
        let left = pcm16(left);
        let right = pcm16(right);
        let frames = left.len().max(right.len());
        let data_size = u32::try_from(frames * 4).unwrap();
        let mut stereo = Vec::with_capacity(44 + frames * 4);
        stereo.extend_from_slice(b"RIFF");
        stereo.extend_from_slice(&(data_size + 36).to_le_bytes());
        stereo.extend_from_slice(b"WAVEfmt ");
        stereo.extend_from_slice(&16_u32.to_le_bytes());
        stereo.extend_from_slice(&1_u16.to_le_bytes());
        stereo.extend_from_slice(&2_u16.to_le_bytes());
        stereo.extend_from_slice(&OUTPUT_SAMPLE_RATE.to_le_bytes());
        stereo.extend_from_slice(&(OUTPUT_SAMPLE_RATE * 4).to_le_bytes());
        stereo.extend_from_slice(&4_u16.to_le_bytes());
        stereo.extend_from_slice(&16_u16.to_le_bytes());
        stereo.extend_from_slice(b"data");
        stereo.extend_from_slice(&data_size.to_le_bytes());
        for frame in 0..frames {
            stereo.extend_from_slice(&left.get(frame).copied().unwrap_or(0).to_le_bytes());
            stereo.extend_from_slice(&right.get(frame).copied().unwrap_or(0).to_le_bytes());
        }
        stereo
    }
}
