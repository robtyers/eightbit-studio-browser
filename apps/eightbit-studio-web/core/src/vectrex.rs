//! Browser composition of the same family-owned original cartridge session.
use eightbit_vectrex_emulator::{cartridge, session::Session};
use serde::Deserialize;
use wasm_bindgen::prelude::wasm_bindgen;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Source {
    schema_version: u32,
    target: String,
    source: String,
    source_sha256: String,
    cartridge_sha256: String,
}
#[wasm_bindgen]
/// Source-bound browser owner of the original Vectrex guest.
pub struct VectrexOriginalRuntime {
    session: Session,
}
#[wasm_bindgen]
impl VectrexOriginalRuntime {
    #[wasm_bindgen(js_name=fromSource)]
    /// Reassemble the current source and verify both source and cartridge hashes.
    pub fn from_source(id: u32, json: &str) -> Result<VectrexOriginalRuntime, String> {
        if json.len() > 1024 * 1024 {
            return Err("VXO1000: Vectrex launch exceeds its limit".into());
        }
        let source: Source =
            serde_json::from_str(json).map_err(|_| "VXO1000: Invalid Vectrex source launch")?;
        if source.schema_version != 1
            || source.target != "vectrex"
            || eightbit_vectrex_emulator::digest(source.source.as_bytes()) != source.source_sha256
        {
            return Err("VXO1000: Vectrex source changed; build again".into());
        }
        let cart = cartridge::assemble(&source.source).map_err(|e| e.to_string())?;
        let session = Session::new(id, &cart.bytes, &source.cartridge_sha256, || false)
            .map_err(|e| e.to_string())?;
        Ok(Self { session })
    }
    /// Apply one bounded schema/session-bound request.
    pub fn command(&mut self, request: &str) -> Result<String, String> {
        self.session.request(request).map_err(|e| e.to_string())
    }
}
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    #[test]
    fn maximum_source_still_launches_when_json_escaping_expands_comments() {
        let mut source = cartridge::TEMPLATE.to_owned();
        while source.len() + 4096 <= 128 * 1024 {
            source.push_str("\n;");
            source.push_str(&"\u{0001}".repeat(4094));
        }
        let cart = cartridge::assemble(&source).unwrap();
        let launch = serde_json::json!({"schemaVersion":1,"target":"vectrex","source":source,"sourceSha256":eightbit_vectrex_emulator::digest(source.as_bytes()),"cartridgeSha256":eightbit_vectrex_emulator::digest(&cart.bytes)}).to_string();
        assert!(launch.len() > 512 * 1024);
        assert!(VectrexOriginalRuntime::from_source(7, &launch).is_ok());
        assert!(VectrexOriginalRuntime::from_source(7, &" ".repeat(1024 * 1024 + 1)).is_err());
    }
    #[test]
    fn browser_source_hashes_and_native_guest_frames_match() {
        let source = include_str!("../../../../families/vectrex/templates/vector-demo.asm");
        let cart = cartridge::assemble(source).unwrap().bytes;
        let mut native = Session::new(7, &cart, &eightbit_vectrex_emulator::digest(&cart), || {
            false
        })
        .unwrap();
        let mut launch = serde_json::json!({"schemaVersion":1,"target":"vectrex","source":source,"sourceSha256":eightbit_vectrex_emulator::digest(source.as_bytes()),"cartridgeSha256":eightbit_vectrex_emulator::digest(&cart)});
        let mut browser = VectrexOriginalRuntime::from_source(7, &launch.to_string()).unwrap();
        for cmd in [
            r#"{"operation":"advance","cycles":30000}"#,
            r#"{"operation":"key","code":"ArrowLeft","pressed":true}"#,
            r#"{"operation":"advance","cycles":60000}"#,
            r#"{"operation":"release"}"#,
            r#"{"operation":"reset"}"#,
        ] {
            let request = format!("{{\"schemaVersion\":1,\"sessionId\":7,\"command\":{cmd}}}");
            assert_eq!(
                native.request(&request).unwrap(),
                browser.command(&request).unwrap()
            );
        }
        for field in ["sourceSha256", "cartridgeSha256"] {
            let old = launch[field].clone();
            launch[field] = serde_json::json!("0".repeat(64));
            assert!(VectrexOriginalRuntime::from_source(8, &launch.to_string()).is_err());
            launch[field] = old;
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod project_tests {
    use crate::{browser_request, project};
    #[test]
    fn vectrex_browser_projects_preserve_stock_manifest_and_zip_source_membership() {
        let source = eightbit_vectrex_emulator::cartridge::TEMPLATE;
        let p = project::create_for(
            "Web vectors".into(),
            project::Kind::Assembly,
            source.into(),
            "vectrex".into(),
        )
        .unwrap();
        assert_eq!(project::validate(&p).unwrap(), "src/asm/main.asm");
        assert_eq!(project::source_paths(&p).unwrap(), ["src/asm/main.asm"]);
        assert!(p.support_files.contains_key("GUIDE.html"));
        let q = project::import(&project::export(&p).unwrap()).unwrap();
        assert_eq!(q.source, p.source);
        assert_eq!(q.manifest, p.manifest);
        assert_eq!(q.support_files, p.support_files);
        let result: serde_json::Value = serde_json::from_str(&browser_request(
            &serde_json::json!({"schemaVersion":1,"operation":"test-project","project":p})
                .to_string(),
            &[],
        ))
        .unwrap();
        assert_eq!(result["ok"], true, "{result}");
        assert_eq!(result["result"]["cassette"].as_array().unwrap().len(), 204);
        let mut bad = q;
        bad.manifest =
            Some("schema_version=1\nname='Bad'\ntarget='vectrex'\nsource='../outside.asm'".into());
        assert!(project::validate(&bad).is_err());
    }
}
