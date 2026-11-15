//! Explicit supplied-ROM gate, outside ordinary CI.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../examples/support/reference.rs"]
mod reference;
#[test]
fn historical_firmware_gate_rejects_generated_wrong_and_truncated_bytes() {
    for bytes in [vec![], vec![0; 20479], vec![0; 20480], vec![0; 20481]] {
        assert!(reference::validate(&bytes).is_err());
    }
}
#[test]
#[ignore = "requires exact-hash external C64 firmware; use release mode for the keyboard corpus"]
fn real_c64_original_pal_keyboard_video_audio_and_recovery() {
    let packed = reference::read().unwrap();
    let firmware = reference::validate(&packed).unwrap();
    let result = reference::qualify(firmware, |_, _, _| Ok(())).unwrap();
    assert!(result.contains("\"qCleanup\":true"));
}
