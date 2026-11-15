//! Native half of the deterministic original Vectrex native/WASM trace.
use eightbit_browser::VectrexOriginalRuntime;
use eightbit_vectrex_emulator::{cartridge, digest};
fn trace(source: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let cart = cartridge::assemble(source)?;
    let launch = serde_json::json!({"schemaVersion":1,"target":"vectrex","source":source,"sourceSha256":digest(source.as_bytes()),"cartridgeSha256":digest(&cart.bytes)});
    let mut runtime = VectrexOriginalRuntime::from_source(7, &launch.to_string())?;
    let commands = serde_json::json!([{"operation":"advance","cycles":30000},{"operation":"key","code":"ArrowRight","pressed":true},{"operation":"advance","cycles":90000},{"operation":"key","code":"KeyZ","pressed":true},{"operation":"advance","cycles":60000},{"operation":"release"},{"operation":"advance","cycles":60000},{"operation":"reset"},{"operation":"advance","cycles":90000},{"operation":"stop"}]);
    let mut trace = Vec::new();
    for command in commands.as_array().ok_or("Invalid original trace")? {
        let request = serde_json::json!({"schemaVersion":1,"sessionId":7,"command":command});
        let mut frame: serde_json::Value =
            serde_json::from_str(&runtime.command(&request.to_string())?)?;
        let pixels: Vec<u8> = serde_json::from_value(frame["pixels"].take())?;
        let audio: Vec<i16> = serde_json::from_value(frame["audio"].take())?;
        frame["pixels"] = serde_json::Value::Null;
        frame["audio"] = serde_json::Value::Null;
        frame["pixelsSha256"] = serde_json::json!(digest(&pixels));
        frame["audioSha256"] = serde_json::json!(digest(
            &audio
                .iter()
                .flat_map(|s| s.to_le_bytes())
                .collect::<Vec<_>>()
        ));
        trace.push(frame);
    }
    Ok(serde_json::json!(trace))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let header = cartridge::TEMPLATE
        .split_once("start:")
        .ok_or("Missing original starter entry")?
        .0;
    let unsupported = trace(&format!("{header}start: jsr $F687\n bra start\n"))
        .err()
        .ok_or("Unsupported executive service unexpectedly returned")?
        .to_string();
    if !unsupported.starts_with("VXO1001:") {
        return Err("Unexpected unsupported-service diagnostic".into());
    }
    let text_list_fault = trace(&format!(
        "{header}start: ldu #list\n jsr $F385\n bra start\nlist: fcb -8,40,1,0,$1F,$80,0\n"
    ))
    .err()
    .ok_or("Malformed text list unexpectedly returned")?
    .to_string();
    if !text_list_fault.starts_with("VXO1001:") {
        return Err("Unexpected text-list diagnostic".into());
    }
    let pattern_fault = trace(&format!(
        "{header}start: ldx #list\n jsr $F46E\n bra start\nlist: fcb 3,0,0,1\n"
    ))
    .err()
    .ok_or("Malformed patterned packet unexpectedly returned")?
    .to_string();
    if !pattern_fault.starts_with("VXO1001:") {
        return Err("Unexpected pattern diagnostic".into());
    }
    let traces = serde_json::json!({
        "unsupported": unsupported,
        "patternFault": pattern_fault,
        "textListFault": text_list_fault,
        "starter": trace(cartridge::TEMPLATE)?,
        "executive": trace(include_str!("../../../../families/vectrex/templates/executive-demo.asm"))?,
        "packet": trace(include_str!("../../../../families/vectrex/templates/packet-demo.asm"))?,
        "patterns": trace(include_str!("../../../../families/vectrex/templates/pattern-demo.asm"))?,
        "textLists": trace(include_str!("../../../../families/vectrex/templates/text-list-demo.asm"))?,
    });
    println!("{}", serde_json::to_string(&traces)?);
    Ok(())
}
