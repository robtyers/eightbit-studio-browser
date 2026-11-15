//! Print reproducible original firmware identity without external tools or inputs.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use eightbit_vectrex_emulator::{digest, firmware};
    let image = firmware::image()?;
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"schemaVersion":1,"id":firmware::ID,"licence":"Apache-2.0","bytes":image.len(),"sourceSha256":digest(firmware::SOURCE.as_bytes()),"imageSha256":digest(&image)})
        )?
    );
    Ok(())
}
