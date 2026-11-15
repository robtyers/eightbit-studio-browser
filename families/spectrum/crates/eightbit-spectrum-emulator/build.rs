//! Build only independently authored guest firmware using our original assembler.
use eightbit_z80_core::assembler::assemble;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fmt::Write,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

fn digest(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(text, "{byte:02x}");
    }
    text
}
fn original_source(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.len() > 1024 * 1024 {
        return Err("Firmware source must be a bounded regular file".into());
    }
    let mut text = String::new();
    fs::File::open(path)?
        .take(1024 * 1024 + 1)
        .read_to_string(&mut text)?;
    if text.len() > 1024 * 1024 {
        return Err("Firmware source grew beyond its limit".into());
    }
    Ok(text)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let source = root.join("../../firmware/original");
    let pages = [
        "page-00.asm",
        "page-01.asm",
        "page-02.asm",
        "page-03.asm",
        "page-04.asm",
        "page-05.asm",
        "page-06.asm",
        "page-07.asm",
        "page-08.asm",
        "page-09.asm",
        "page-10.asm",
        "page-11.asm",
        "page-12.asm",
        "page-13.asm",
        "page-14.asm",
        "page-15.asm",
        "page-16.asm",
        "page-17.asm",
        "page-18.asm",
        "page-19.asm",
        "page-20.asm",
        "page-21.asm",
        "page-22.asm",
        "page-23.asm",
        "page-24.asm",
        "page-25.asm",
    ];
    let mut image = Vec::new();
    let mut source_identity = String::new();
    let mut sources = BTreeMap::new();
    for name in ["common.asm", "tables.asm", "font.asm"]
        .into_iter()
        .chain(pages)
    {
        println!("cargo:rerun-if-changed={}", source.join(name).display());
        let text = original_source(&source.join(name))?;
        writeln!(source_identity, "{name}:{}", digest(text.as_bytes()))?;
        sources.insert(name, text);
    }
    for (page, kernel) in [
        ("page-01.asm", "floating.asm"),
        ("page-06.asm", "format.asm"),
        ("page-09.asm", "math.asm"),
    ] {
        let path = root
            .join("../../../../cpu/z80/firmware/original")
            .join(kernel);
        println!("cargo:rerun-if-changed={}", path.display());
        let text = original_source(&path)?;
        writeln!(
            source_identity,
            "shared/{kernel}:{}",
            digest(text.as_bytes())
        )?;
        let wrapper = sources.get(page).ok_or("Missing kernel composition page")?;
        sources.insert(page, format!("{wrapper}\n{text}"));
    }
    let builder_hash = digest(original_source(&root.join("build.rs"))?.as_bytes());
    let mut assembler_identity = String::new();
    let cpu = root.join("../../../../cpu/z80/eightbit-z80-core/src");
    for file in [
        "lib.rs",
        "assembler.rs",
        "assembler/encode.rs",
        "assembler/expression.rs",
    ] {
        writeln!(
            assembler_identity,
            "{file}:{}",
            digest(original_source(&cpu.join(file))?.as_bytes())
        )?;
    }
    let common = sources
        .get("common.asm")
        .ok_or("Missing original common firmware")?;
    let tables = sources
        .get("tables.asm")
        .ok_or("Missing original firmware tables")?;
    let font = sources
        .get("font.asm")
        .ok_or("Missing original font designs")?;
    let mut common_bytes = Vec::new();
    let mut shared_tail = Vec::new();
    let mut symbols = String::new();
    for (index, page) in pages.iter().enumerate() {
        let text = format!(
            "{common}\nORG $2000\n{}\n{tables}\n{font}",
            sources.get(page).ok_or("Missing original firmware page")?
        );
        let assembled = assemble(&text, 0, None)
            .map_err(|error| format!("Original firmware {page}: {error}"))?;
        if index == 0 {
            let entry = assembled
                .symbols
                .get("STMT_LOOP")
                .ok_or("Missing original statement entry")?;
            fs::write(
                PathBuf::from(std::env::var("OUT_DIR")?).join("statement-entry.rs"),
                format!("pub const STATEMENT_ENTRY: u16 = {entry};\n"),
            )?;
        }
        if assembled.bytes.len() > 16384 {
            return Err("Original firmware page exceeds its mapped window".into());
        }
        let mut bytes = assembled.bytes;
        bytes.resize(16384, 0);
        if index == 0 {
            common_bytes.extend_from_slice(&bytes[..8192]);
            image.extend_from_slice(&common_bytes);
            shared_tail.extend_from_slice(&bytes[0x3a00..]);
        } else if common_bytes != bytes[..8192] {
            return Err("Common public firmware entries differ between pages".into());
        }
        if shared_tail != bytes[0x3a00..] {
            return Err("Public tables and character designs differ between firmware pages".into());
        }
        image.extend_from_slice(&bytes[8192..]);
        for (name, value) in assembled.symbols {
            writeln!(symbols, "{index}:{name}={value}")?;
        }
    }
    let output = PathBuf::from(std::env::var("OUT_DIR")?);
    fs::write(output.join("spectrum48.rom"), &image)?;
    fs::write(output.join("spectrum48.sym"), symbols)?;
    fs::write(
        output.join("spectrum48-manifest.json"),
        format!(
            "{{\"schemaVersion\":1,\"id\":\"eightbit-spectrum48-original-dev-1\",\"status\":\"development\",\"licence\":\"Apache-2.0\",\"bytes\":{},\"sha256\":\"{}\",\"sourceSha256\":\"{}\",\"builderSha256\":\"{}\",\"assemblerSha256\":\"{}\",\"commonBytes\":8192,\"pageBytes\":8192,\"pages\":{}}}\n",
            image.len(),
            digest(&image),
            digest(source_identity.as_bytes()),
            builder_hash,
            digest(assembler_identity.as_bytes()),
            pages.len()
        ),
    )?;
    Ok(())
}
