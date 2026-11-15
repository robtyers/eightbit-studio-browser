//! Stage-3 public data contracts, measured with external exact-hash ROMs only.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../examples/support/interfaces.rs"]
mod interfaces;
#[path = "../examples/support/reference.rs"]
mod reference;

#[test]
#[ignore = "requires exact-hash external C64 firmware; use release mode for physical typing"]
fn real_c64_public_workspace_numeric_strings_arrays_usr() {
    let mut packed = reference::read().unwrap();
    let result = interfaces::qualify(reference::validate(&packed).unwrap());
    packed.fill(0);
    assert!(result.unwrap().contains("\"keyboardOnly\":true"));
}
