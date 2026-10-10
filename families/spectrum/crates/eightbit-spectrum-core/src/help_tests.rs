use super::*;
use std::sync::atomic::AtomicBool;

// Examples remain in the authoritative family/CPU guides, never test-only copies.
const GUIDES: &[&str] = &[
    include_str!("../../../help/GUIDE.html"),
    include_str!("../../../../../cpu/z80/help/GUIDE.html"),
];
fn examples(language: &str) -> Vec<String> {
    let marker = format!("<pre data-example=\"{language}\"><code>");
    GUIDES
        .iter()
        .flat_map(|guide| guide.split(&marker).skip(1))
        .map(|part| {
            part.split_once("</code></pre>")
                .expect("complete Help listing")
                .0
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&amp;", "&")
        })
        .collect()
}

fn tape_blocks(tape: &[u8]) -> Vec<&[u8]> {
    let mut remaining = tape;
    let mut blocks = Vec::new();
    while !remaining.is_empty() {
        assert!(remaining.len() >= 2);
        let size = usize::from(u16::from_le_bytes([remaining[0], remaining[1]]));
        assert!(size >= 2 && size + 2 <= remaining.len());
        let block = &remaining[2..size + 2];
        assert_eq!(block.iter().fold(0, |sum, byte| sum ^ byte), 0);
        blocks.push(block);
        remaining = &remaining[size + 2..];
    }
    blocks
}

#[test]
fn help_basic_examples_encode_for_every_spectrum_target() {
    let listings = examples("basic");
    assert!(!listings.is_empty());
    assert!(listings.iter().any(|s| s.contains("COLLECT FIVE STARS")));
    for target in TARGETS {
        for (index, source) in listings.iter().enumerate() {
            let tape = compile_basic(target, source).unwrap_or_else(|error| {
                panic!("{} Help BASIC listing {}: {error:?}", target.id, index + 1)
            });
            assert_eq!(tape, compile_basic(target, source).unwrap());
            let blocks = tape_blocks(&tape);
            assert_eq!(blocks.len(), 2);
            assert_eq!(blocks[0].len(), 19);
            assert_eq!(&blocks[0][..2], &[0, 0]);
            assert_eq!(blocks[1][0], 255);
            let size = usize::from(u16::from_le_bytes([blocks[0][12], blocks[0][13]]));
            assert_eq!(blocks[1].len(), size + 2);
            let mut lines = &blocks[1][1..=size];
            let mut numbers = Vec::new();
            while !lines.is_empty() {
                assert!(lines.len() >= 4);
                numbers.push(u16::from_be_bytes([lines[0], lines[1]]));
                let length = usize::from(u16::from_le_bytes([lines[2], lines[3]]));
                assert!(length > 0 && 4 + length <= lines.len());
                assert_eq!(lines[3 + length], 13);
                lines = &lines[4 + length..];
            }
            let source_numbers: Vec<u16> = source
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| line.split_whitespace().next().unwrap().parse().unwrap())
                .collect();
            assert_eq!(numbers, source_numbers);
            assert_eq!(
                u16::from_le_bytes([blocks[0][14], blocks[0][15]]),
                source_numbers[0]
            );
        }
    }
}

#[test]
fn help_assembly_examples_obey_native_source_contract() {
    let listings = examples("assembly");
    assert!(!listings.is_empty());
    assert!(listings.iter().any(|s| s.contains("LD DE,22688")));
    for (index, source) in listings.iter().enumerate() {
        eightbit_source::build::validate_assembly(source, 24576)
            .unwrap_or_else(|error| panic!("Help assembly listing {}: {error:?}", index + 1));
    }
}

#[test]
#[ignore = "requires explicit STUDIO_SJASMPLUS pointing to SjASMPlus 1.24.0"]
fn real_sjasmplus_help_examples_build() {
    let assembler = std::env::var_os("STUDIO_SJASMPLUS")
        .map(std::path::PathBuf::from)
        .expect("Set STUDIO_SJASMPLUS explicitly");
    static ENGINE: eightbit_source::Engine = eightbit_source::Engine {
        product: "Eightbit Studio Z80",
        families: &[&FAMILY],
        foreign_manifests: &[],
    };
    let parent = tempfile::tempdir().unwrap();
    for target in TARGETS {
        for (index, source) in examples("assembly").iter().enumerate() {
            let project = ENGINE
                .create(
                    parent.path(),
                    &format!("help-{}-{index}", target.id),
                    "spectrum",
                    target.id,
                    "assembly",
                )
                .unwrap();
            let saved = ENGINE
                .save(&project.root, &project.revision, source)
                .unwrap();
            let built = ENGINE
                .build(&saved.root, Some(&assembler), &AtomicBool::new(false))
                .unwrap_or_else(|error| {
                    panic!(
                        "{} Help assembly listing {}: {error:?}",
                        target.id,
                        index + 1
                    )
                });
            let tape = ENGINE
                .fresh_media(&saved.root, &built.revision, &built.sha256)
                .unwrap();
            let blocks = tape_blocks(&tape);
            assert_eq!(blocks.len(), 4);
            assert_eq!(blocks[2][1], 3);
            assert_eq!(u16::from_le_bytes([blocks[2][14], blocks[2][15]]), 24576);
            assert!(blocks[3].len() > 2 && blocks[3].len() <= 8194);
        }
    }
}
