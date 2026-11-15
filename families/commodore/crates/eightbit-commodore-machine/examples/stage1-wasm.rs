//! Test-only WebAssembly export of the same bounded trace as the native example.
#[cfg(target_arch = "wasm32")]
#[path = "support/boot.rs"]
mod boot;
#[cfg(target_arch = "wasm32")]
#[path = "support/interfaces.rs"]
mod interfaces;
#[path = "support/paging.rs"]
mod paging;
#[cfg(target_arch = "wasm32")]
#[path = "support/reference.rs"]
mod reference;
#[path = "support/stage1.rs"]
mod stage1;
#[path = "support/stage2.rs"]
mod stage2;

/// Original CPU/memory observation corpus; no UI or supplied firmware is involved.
#[wasm_bindgen::prelude::wasm_bindgen]
#[must_use]
pub fn stage1_trace() -> String {
    stage1::trace()
}

/// Generated original PAL-board observations shared with the native example.
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn stage2_trace() -> Result<String, wasm_bindgen::JsValue> {
    stage2::trace().map_err(|error| wasm_bindgen::JsValue::from_str(&error.to_string()))
}

/// Generated original page/interrupt gate trace; no supplied ROM is used.
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn stage3_paging() -> Result<String, wasm_bindgen::JsValue> {
    paging::trace().map_err(|error| wasm_bindgen::JsValue::from_str(&error.to_string()))
}

/// Explicit original guest boot, with an independently supplied trusted image hash.
#[wasm_bindgen::prelude::wasm_bindgen]
#[cfg(target_arch = "wasm32")]
pub fn stage3_original_boot(
    bytes: &[u8],
    expected: &[u8],
) -> Result<String, wasm_bindgen::JsValue> {
    boot::qualify(bytes, expected, |_| Ok(()))
        .map_err(|error| wasm_bindgen::JsValue::from_str(&error.to_string()))
}

/// Opt-in exact-hash supplied-ROM gate; returns observations, never firmware bytes.
#[wasm_bindgen::prelude::wasm_bindgen]
#[cfg(target_arch = "wasm32")]
pub fn stage2_reference(packed: &[u8]) -> Result<String, wasm_bindgen::JsValue> {
    let result = reference::validate(packed)
        .and_then(|firmware| reference::qualify(firmware, |_, _, _| Ok(())));
    result.map_err(|error| wasm_bindgen::JsValue::from_str(&error.to_string()))
}

/// Pre-firmware public RAM contracts; only original programme data is returned.
#[wasm_bindgen::prelude::wasm_bindgen]
#[cfg(target_arch = "wasm32")]
pub fn stage3_interfaces(packed: &[u8]) -> Result<String, wasm_bindgen::JsValue> {
    reference::validate(packed)
        .and_then(interfaces::qualify)
        .map_err(|error| wasm_bindgen::JsValue::from_str(&error.to_string()))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn c64_reference_language(bytes: &[u8]) -> Result<String, wasm_bindgen::JsValue> {
    reference::validate(bytes)
        .and_then(reference::language)
        .map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))
}
