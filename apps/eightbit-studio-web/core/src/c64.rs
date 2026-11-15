//! Explicit C64 composition; shared family code owns all guest behaviour.
use eightbit_commodore_core::{
    basic,
    original_session::{BUNDLED_FIRMWARE, BUNDLED_SHA256, Session},
};
use serde::Deserialize;
use wasm_bindgen::prelude::wasm_bindgen;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Source {
    schema_version: u32,
    target: String,
    source: String,
    source_sha256: String,
    prg_sha256: String,
}
#[wasm_bindgen]
/// Disposable C64 worker owner; construction requires an explicit current build.
pub struct C64OriginalRuntime {
    session: Session<'static>,
}
#[wasm_bindgen]
impl C64OriginalRuntime {
    /// Recompile BASIC and verify both source and PRG before boot or RAM loading.
    #[wasm_bindgen(js_name=fromSource)]
    pub fn from_source(session_id: u32, encoded: &str) -> Result<C64OriginalRuntime, String> {
        if encoded.len() > 256 * 1024 {
            return Err("C64RUN1000: Source launch exceeds its limit".into());
        }
        let source: Source =
            serde_json::from_str(encoded).map_err(|_| "C64RUN1000: Invalid source launch")?;
        if source.schema_version != 1
            || source.target != "c64-pal"
            || source.source.len() > 128 * 1024
            || eightbit_source::digest(source.source.as_bytes()) != source.source_sha256
        {
            return Err("C64RUN1000: C64 source identity changed; build again".into());
        }
        let prg = basic::compile(&source.source).map_err(|e| e.to_string())?;
        let session = Session::from_prg(
            session_id,
            BUNDLED_FIRMWARE,
            &BUNDLED_SHA256,
            &prg,
            &source.prg_sha256,
            || false,
        )
        .map_err(|e| e.to_string())?;
        Ok(Self { session })
    }
    /// Apply one bounded, identity-bound command to this exact guest.
    pub fn command(&mut self, request: &str) -> Result<String, String> {
        self.session.request(request).map_err(|e| e.to_string())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_and_prg_identities_are_checked_before_guest_launch() {
        let source = "10 PRINT\"BROWSER C64\"\n";
        let mut launch = serde_json::json!({"schemaVersion":1,"target":"c64-pal","source":source,"sourceSha256":eightbit_source::digest(source.as_bytes()),"prgSha256":eightbit_source::digest(&basic::compile(source).unwrap())});
        let mut session = C64OriginalRuntime::from_source(7, &launch.to_string()).unwrap();
        for _ in 0..150 {
            session.command(r#"{"schemaVersion":1,"sessionId":7,"command":{"operation":"advance","cycles":19656}}"#).unwrap();
        }
        let frame: serde_json::Value = serde_json::from_str(
            &session
                .command(r#"{"schemaVersion":1,"sessionId":7,"command":{"operation":"state"}}"#)
                .unwrap(),
        )
        .unwrap();
        assert!(frame["text"].as_str().unwrap().contains("BROWSER C64"));
        for field in ["sourceSha256", "prgSha256"] {
            let previous = launch[field].clone();
            launch[field] = serde_json::json!("0".repeat(64));
            assert!(C64OriginalRuntime::from_source(8, &launch.to_string()).is_err());
            launch[field] = previous;
        }
        launch["target"] = serde_json::json!("bbc-model-b");
        assert!(C64OriginalRuntime::from_source(8, &launch.to_string()).is_err());
    }
}
