//! Original native image and streaming cassette boundary vectors.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use eightbit_zx81_emulator::{Model, image::Image, tape::Tape};
#[test]
fn native_image_boundaries_are_model_specific_and_failure_is_total() {
    for model in [
        Model::Zx80_1k,
        Model::Zx80_16k,
        Model::Zx81_1k,
        Model::Zx81_16k,
    ] {
        let programme = if model.zx80() {
            vec![0, 10, 244, 0, 29, 118]
        } else {
            vec![0, 10, 4, 0, 245, 0, 29, 118]
        };
        let image = Image::from_program(model, &programme).unwrap();
        assert_eq!(Image::parse(model, image.bytes()).unwrap(), image);
        assert_eq!(image.programme(), programme);
        for count in 0..image.bytes().len() {
            assert!(Image::parse(model, &image.bytes()[..count]).is_err());
        }
        for count in [0, 1, 40, 116, 16384, 65536] {
            assert!(Image::parse(model, &vec![0; count]).is_err());
        }
        for offset in if model.zx80() {
            vec![8, 10, 12, 14, 16]
        } else {
            vec![3, 7, 11]
        } {
            let mut bad = image.bytes().to_vec();
            bad[offset] = 255;
            bad[offset + 1] = 255;
            assert!(
                Image::parse(model, &bad).is_err(),
                "{model:?} pointer {offset}"
            );
        }
    }
    let blank = Image::from_program(Model::Zx81_1k, &[]).unwrap();
    assert!(Image::parse(Model::Zx80_1k, blank.bytes()).is_err());
    let blank = Image::from_program(Model::Zx80_1k, &[]).unwrap();
    assert!(Image::parse(Model::Zx81_1k, blank.bytes()).is_err());
}
#[test]
fn duplicate_descending_truncated_and_empty_lines_are_rejected() {
    for bytes in [
        &[0, 0, 244, 118][..],
        &[0, 10, 244][..],
        &[0, 10, 118, 0, 10, 118][..],
        &[0, 10, 118, 0, 9, 118][..],
    ] {
        assert!(Image::from_program(Model::Zx80_1k, bytes).is_err());
    }
    for bytes in [
        &[0, 10, 0, 0][..],
        &[0, 10, 1, 0, 0][..],
        &[0, 10, 255, 255, 118][..],
    ] {
        assert!(Image::from_program(Model::Zx81_1k, bytes).is_err());
    }
}
#[test]
fn streaming_transport_has_five_second_lead_pause_rewind_and_four_nine_cells() {
    let image = Image::from_program(Model::Zx80_1k, &[]).unwrap();
    let mut tape = Tape::from_image(&image, "").unwrap();
    tape.advance(20_000_000);
    assert!(!tape.high());
    tape.play();
    tape.advance(16_249_999);
    assert!(!tape.high());
    tape.advance(1);
    assert!(tape.high());
    // First byte is ERR_NR=255: each bit consists of nine full pulses.
    let mut rises = 1;
    let mut previous = true;
    for _ in 0..(17 * 487) {
        tape.advance(1);
        if tape.high() && !previous {
            rises += 1;
        }
        previous = tape.high();
    }
    assert_eq!(rises, 9);
    assert!(!tape.high());
    tape.stop();
    tape.advance(100_000);
    assert!(!tape.high());
    tape.rewind();
    assert!(tape.pending());
    assert!(!tape.playing());
    assert!(!tape.high());
    tape.play();
    for _ in 0..500 {
        tape.advance(325_000);
    }
    assert!(!tape.pending());
    assert!(!tape.playing());
    assert!(!tape.high());
    let image = Image::from_program(Model::Zx81_1k, &[]).unwrap();
    assert!(Tape::from_image(&image, "").is_err());
    assert!(Tape::from_image(&image, "../x").is_err());
    assert!(Tape::from_image(&image, &"A".repeat(128)).is_err());
    assert!(Tape::from_image(&image, "DEMO 1").is_ok());
}
#[test]
fn recording_accepts_both_polarities_and_rejects_truncation_and_bad_cells() {
    use eightbit_zx81_emulator::{TapeEdge, recording};
    for model in [Model::Zx80_1k, Model::Zx81_1k] {
        let image = Image::from_program(model, &[]).unwrap();
        for pulse_level in [true, false] {
            let mut bytes = Vec::new();
            if !model.zx80() {
                bytes.push(166);
            } // A, final filename character.
            bytes.extend_from_slice(image.bytes());
            let mut edges = vec![TapeEdge {
                t_state: 0,
                high: !pulse_level,
            }];
            let mut clock = 16_250_000;
            for byte in bytes {
                for bit in (0..8).rev() {
                    let count = if byte & (1 << bit) == 0 { 4 } else { 9 };
                    for pulse in 0..count {
                        edges.push(TapeEdge {
                            t_state: clock,
                            high: pulse_level,
                        });
                        clock += 487;
                        edges.push(TapeEdge {
                            t_state: clock,
                            high: !pulse_level,
                        });
                        clock += if pulse + 1 == count { 4875 } else { 487 };
                    }
                }
            }
            // A following video transition makes the final bit gap observable.
            edges.push(TapeEdge {
                t_state: clock + 10000,
                high: pulse_level,
            });
            let saved = recording::decode(model, &edges).unwrap();
            assert_eq!(saved.image, image);
            assert!(recording::decode(model, &edges[..edges.len() / 2]).is_err());
            let mut bad = edges.clone();
            bad[3].t_state = bad[2].t_state;
            assert!(recording::decode(model, &bad).is_err());
            assert!(recording::wave(&bad).is_err());
            let mut bad = edges.clone();
            bad[3].t_state = bad[2].t_state + 100;
            assert!(recording::decode(model, &bad).is_err());
            let wave = recording::wave(&edges).unwrap();
            assert!(eightbit_zx81_emulator::wave::Wave::parse(&wave).is_ok());
        }
    }
}
