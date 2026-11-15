//! Opt-in source-bound original guest firmware; no supplied Commodore ROM.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../examples/support/boot.rs"]
mod boot;
#[path = "../examples/support/reference.rs"]
mod reference;

#[test]
#[ignore = "requires explicitly built original image and trusted hash; run in release mode"]
fn original_guest_front_end_and_public_ram_boundaries() {
    let (bytes, expected) = boot::read().unwrap();
    let observation = boot::qualify(&bytes, &expected, |_| Ok(())).unwrap();
    assert!(observation.contains("\"editListRun\":true"));
    assert!(observation.contains("\"exactA000Boundary\":true"));
}
