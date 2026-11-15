//! Assemble only the independently authored, bounded guest sources.
use eightbit_z80_core::assembler::assemble;
use sha2::{Digest, Sha256};
use std::{fmt::Write, fs, io::Read, path::Path};

fn source(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
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
    println!("cargo:rerun-if-changed={}", path.display());
    Ok(text)
}
fn digest(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(text, "{byte:02x}");
    }
    text
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let firmware = root.join("../../firmware/original");
    let common = source(&firmware.join("common.asm"))?;
    let font = source(&firmware.join("font.asm"))?;
    let page = source(&firmware.join("page-00.asm"))?;
    let mut identity = String::new();
    for (name, text) in [
        ("common.asm", &common),
        ("font.asm", &font),
        ("page-00.asm", &page),
    ] {
        writeln!(identity, "{name}:{}", digest(text.as_bytes()))?;
    }
    let output = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    let builder = source(&root.join("build.rs"))?;
    let mut assembler = String::new();
    let cpu = root.join("../../../../cpu/z80/eightbit-z80-core/src");
    for name in [
        "lib.rs",
        "assembler.rs",
        "assembler/encode.rs",
        "assembler/expression.rs",
    ] {
        writeln!(
            assembler,
            "{name}:{}",
            digest(source(&cpu.join(name))?.as_bytes())
        )?;
    }
    for (machine, length) in [("zx80", 40), ("zx81", 125)] {
        let abi = source(&firmware.join(format!("{machine}-abi.asm")))?;
        let mut initial = vec![0_u8; length];
        initial[0] = 255;
        initial[1] = 0x40;
        if machine == "zx81" {
            for (address, value) in [
                (0x22, 2),
                (0x27, 15),
                (0x28, 55),
                (0x35, 128),
                (0x38, 188),
                (0x39, 33),
                (0x3a, 24),
                (0x3b, 128),
                (0x5c, 118),
            ] {
                initial[address] = value;
            }
        } else {
            initial[0x12] = 2;
            initial[0x24] = 33;
            initial[0x25] = 23;
        }
        let data = initial
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let text = format!(
            "{abi}\n{common}\nORG $c00\nINITIAL_SYSTEM:\nDB {data}\n{font}\nORG $2000\n{page}"
        );
        let assembled = assemble(&text, 0, None)
            .map_err(|error| format!("Original {machine} firmware: {error}"))?;
        if assembled.bytes.len() > 16384 {
            return Err("Firmware page exceeds its window".into());
        }
        let mut image = assembled.bytes;
        image.resize(16384, 0);
        let mut symbols = String::new();
        for (name, value) in assembled.symbols {
            writeln!(symbols, "0:{name}={value}")?;
        }
        fs::write(output.join(format!("{machine}.rom")), &image)?;
        fs::write(output.join(format!("{machine}.sym")), symbols)?;
        let source_identity = format!("{identity}abi:{}", digest(abi.as_bytes()));
        fs::write(
            output.join(format!("{machine}-manifest.json")),
            format!(
                "{{\"schemaVersion\":1,\"id\":\"eightbit-{machine}-original-dev-1\",\"status\":\"hardware-development\",\"licence\":\"Apache-2.0\",\"bytes\":{},\"sha256\":\"{}\",\"sourceSha256\":\"{}\",\"builderSha256\":\"{}\",\"assemblerSha256\":\"{}\",\"commonBytes\":8192,\"pageBytes\":8192,\"pages\":1}}\n",
                image.len(),
                digest(&image),
                digest(source_identity.as_bytes()),
                digest(builder.as_bytes()),
                digest(assembler.as_bytes())
            ),
        )?;
    }
    Ok(())
}
