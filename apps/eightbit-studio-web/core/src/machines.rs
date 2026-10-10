//! Application composition of portable family-owned BASIC profiles.
use eightbit_source::{Engine, Family, Profile};
use serde_json::{Value, json};
static COMMODORE: &[&Family] = &[&eightbit_commodore_core::FAMILY];
static SPECTRUM: &[&Family] = &[&eightbit_spectrum_core::FAMILY];
static ZX81: &[&Family] = &[&eightbit_zx81_core::FAMILY];
pub fn engine(target: &str) -> Result<Engine, String> {
    let (family, product): (&'static Family, _) = if eightbit_commodore_core::TARGETS
        .iter()
        .any(|t| t.id == target)
    {
        (&eightbit_commodore_core::FAMILY, "Eightbit Studio 6502")
    } else if eightbit_spectrum_core::TARGETS
        .iter()
        .any(|t| t.id == target)
    {
        (&eightbit_spectrum_core::FAMILY, "Eightbit Studio Z80")
    } else if eightbit_zx81_core::TARGETS.iter().any(|t| t.id == target) {
        (&eightbit_zx81_core::FAMILY, "Eightbit Studio Z80")
    } else {
        return Err("DSW1020: unsupported browser computer".into());
    };
    let families: &'static [&'static Family] = match family.definition.id {
        "commodore" => COMMODORE,
        "spectrum" => SPECTRUM,
        _ => ZX81,
    };
    Ok(Engine {
        families,
        foreign_manifests: &[],
        product,
    })
}
pub fn profile(target: &str) -> Result<&'static Profile, String> {
    let engine = engine(target)?;
    engine
        .profile(engine.families[0].definition.id, target)
        .map_err(|e| e.to_string())
}
pub fn catalog() -> Value {
    let mut machines = vec![
        json!({"id":"dragon32", "label":"Dragon 32", "cpu":"6809", "extension":"cas", "manifest":"dragon.toml", "runtime":"xroar", "romSize":16384, "romNames":["d32.rom"], "sample":include_str!("../../../../packages/browser-workbench/src/guess-the-number.bas")}),
    ];
    for family in [
        &eightbit_commodore_core::FAMILY,
        &eightbit_spectrum_core::FAMILY,
        &eightbit_zx81_core::FAMILY,
    ] {
        for p in family.profiles {
            machines.push(json!({"id":p.target.id, "label":p.target.display_name, "cpu":if p.target.cpu == "mos6502" {"6502"} else {"Z80"}, "extension":p.extension, "manifest":family.manifest, "runtime":if family.definition.id == "commodore" {"chips-c64"} else {"mame-z80"}, "romSize":p.rom_size, "romNames":if family.definition.id == "commodore" {vec!["basic-901226-01.bin", "kernal-901227-03.bin", "chargen-901225-01.bin"]} else {p.rom_names.to_vec()}, "sample":p.basic_template}));
        }
    }
    json!({"machines":machines})
}
pub fn compile(target: &str, source: &str) -> Result<Vec<u8>, String> {
    let p = profile(target)?;
    (p.basic)(p.target, source).map_err(|e| e.to_string())
}
pub fn recover(target: &str, bytes: &[u8]) -> Result<String, String> {
    let p = profile(target)?;
    let value = match p.target.family {
        "commodore" => eightbit_commodore_core::basic::decompile(bytes),
        "spectrum" => eightbit_spectrum_core::decompile_basic(p.target, bytes),
        "zx81" => eightbit_zx81_core::decompile_basic(p.target, bytes),
        _ => return Err("DSW1020: unsupported browser computer".into()),
    };
    value.map_err(|e| e.to_string())
}
pub fn firmware(target: &str, bytes: &[u8]) -> Result<Value, String> {
    use sha2::{Digest, Sha256};
    let p = profile(target)?;
    if bytes.len() != p.rom_size {
        return Err(format!(
            "DSW1021: choose the supported {} firmware ({} bytes)",
            p.target.display_name, p.rom_size
        ));
    }
    let digest = hex::encode(Sha256::digest(bytes));
    let matches = if p.target.family == "commodore" {
        eightbit_commodore_core::firmware::validate_packed(bytes).is_ok()
    } else {
        digest == p.rom_sha256
    };
    Ok(json!({"matchesVerifiedFirmware":matches}))
}
pub fn build(target: &str, source: &str) -> Result<Value, String> {
    use sha2::{Digest, Sha256};
    let bytes = compile(target, source)?;
    let recovered = recover(target, &bytes)?;
    if compile(target, &recovered)? != bytes {
        return Err("DSW1022: programme round-trip check failed".into());
    }
    Ok(
        json!({"cassette":bytes, "summary":{"tokenBytes":bytes.len(),"cassetteSha256":hex::encode(Sha256::digest(&bytes))},"roundTripVerified":true}),
    )
}
