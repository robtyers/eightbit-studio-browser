//! Shared bounded cartridge/source contracts; no filesystem or tools.
use crate::{Result, error};
const OPS: &str = "abx adca adcb adda addb addd anda andb andcc asl asla aslb asr asra asrb bcc bcs beq bge bgt bhi bhs bita bitb ble blo bls blt bmi bne bpl bra brn bsr bvc bvs clr clra clrb cmpa cmpb cmpd cmps cmpu cmpx cmpy com coma comb cwai daa dec deca decb eora eorb exg inc inca incb jmp jsr lbcc lbcs lbeq lbge lbgt lbhi lbhs lble lblo lbls lblt lbmi lbne lbpl lbra lbrn lbsr lbvc lbvs lda ldb ldd lds ldu ldx ldy leas leau leax leay lsl lsla lslb lsr lsra lsrb mul neg nega negb nop ora orb orcc pshs pshu puls pulu rol rola rolb ror rora rorb rti rts sbca sbcb sex sta stb std sts stu stx sty suba subb subd swi swi2 swi3 sync tfr tst tsta tstb equ fcb fdb fcc fcs setdp end org";
/// An explicit first-increment language subset prevents assembler file access/macros.
pub fn validate_source(source: &str) -> Result<()> {
    let mut origin = false;
    for (index, line) in source.lines().enumerate() {
        if line.len() > 4096 || !line.is_ascii() {
            return Err(error(
                "DSV1100",
                format!(
                    "Line {}: use ASCII assembly lines of at most 4096 bytes",
                    index + 1
                ),
            ));
        }
        let code = line.split(';').next().unwrap_or("").trim();
        if code.is_empty() || code.starts_with('*') {
            continue;
        }
        let mut words = code.split_ascii_whitespace();
        let first = words.next().unwrap_or("").to_ascii_lowercase();
        let operation =
            if first.ends_with(':') || !OPS.split_ascii_whitespace().any(|op| op == first) {
                words.next().unwrap_or("").to_ascii_lowercase()
            } else {
                first
            };
        if operation.is_empty() && code.ends_with(':') {
            continue;
        }
        if !OPS.split_ascii_whitespace().any(|op| op == operation) {
            return Err(error(
                "DSV1100",
                format!(
                    "Line {}: unsupported instruction/directive; includes, macros and file-reading directives are unavailable",
                    index + 1
                ),
            ));
        }
        if operation == "org" {
            let value = words.next().unwrap_or("");
            if origin || !matches!(value, "0" | "$0000" | "$0") {
                return Err(error("DSV1100", "Cartridges require exactly one ORG 0"));
            }
            origin = true;
        }
    }
    if !origin {
        return Err(error("DSV1100", "Cartridges require ORG 0"));
    }
    Ok(())
}
/// Validate the conventional startup header and bounded unbanked address space.
pub fn validate_cartridge(bytes: &[u8]) -> Result<()> {
    if !(20..=32768).contains(&bytes.len()) || !bytes.starts_with(b"g GCE ") {
        return Err(error(
            "DSV1101",
            "Expected a 20-byte to 32-KiB Vectrex cartridge with a GCE startup header",
        ));
    }
    let copyright_end = bytes
        .iter()
        .take(40)
        .position(|b| *b == 0x80)
        .ok_or_else(|| error("DSV1101", "Missing cartridge header terminator"))?;
    let text_start = copyright_end + 7;
    if text_start >= bytes.len() {
        return Err(error("DSV1101", "Truncated cartridge title header"));
    }
    let end = bytes
        .iter()
        .enumerate()
        .skip(text_start)
        .take(80)
        .find(|(_, b)| **b == 0x80)
        .map(|(i, _)| i)
        .ok_or_else(|| error("DSV1101", "Missing cartridge title terminator"))?;
    if bytes.get(end + 1) != Some(&0) || end + 2 >= bytes.len() {
        return Err(error(
            "DSV1101",
            "Expected one terminated cartridge title followed by executable code",
        ));
    }
    Ok(())
}

/// Assemble a stock cartridge with the original no-I/O processor assembler.
pub fn assemble(source: &str) -> Result<eightbit_m6809_core::assembler::Assembly> {
    if source.len() > 128 * 1024 || source.contains('\0') {
        return Err(error(
            "DSV1100",
            "Source exceeds 128 KiB or contains NUL bytes",
        ));
    }
    validate_source(source)?;
    let result = eightbit_m6809_core::assembler::assemble_image(source, 0).map_err(|e| {
        error(
            "DSV1103",
            format!("Assembly failed at source line {}", e.line),
        )
    })?;
    validate_cartridge(&result.bytes)?;
    Ok(result)
}

/// The unchanged portable schema-1 project shape. Filesystem access belongs to hosts.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub name: String,
    pub target: String,
    pub source: String,
}
pub fn parse_manifest(text: &str) -> Result<Manifest> {
    if text.len() > 8192 {
        return Err(error("DSV1000", "Manifest exceeds 8 KiB"));
    }
    let m: Manifest = toml::from_str(text).map_err(|_| {
        error(
            "DSV1000",
            "Invalid vectrex.toml; use the documented schema-1 fields",
        )
    })?;
    if m.schema_version != 1
        || m.target != "vectrex"
        || m.name.trim().is_empty()
        || m.name.len() > 80
        || m.name.chars().any(char::is_control)
        || !m.source.starts_with("src/")
        || std::path::Path::new(&m.source)
            .extension()
            .and_then(|s| s.to_str())
            != Some("asm")
        || m.source.len() > 240
        || m.source.contains('\0')
        || m.source.contains('\\')
        || std::path::Path::new(&m.source)
            .components()
            .any(|p| !matches!(p, std::path::Component::Normal(_)))
    {
        return Err(error(
            "DSV1000",
            "Unsupported Vectrex project configuration",
        ));
    }
    Ok(m)
}
pub const TEMPLATE: &str = include_str!("../../../templates/vector-demo.asm");
pub const SUPPORT: &[(&str, &str)] = &[
    (
        "README.md",
        include_str!("../../../templates/project-README.md"),
    ),
    (
        "AGENTS.md",
        include_str!("../../../templates/project-AGENTS.md"),
    ),
    (
        "CLAUDE.md",
        include_str!("../../../../../templates/project-CLAUDE.md"),
    ),
    (
        ".gitignore",
        include_str!("../../../templates/project.gitignore"),
    ),
    (
        "tests/README.md",
        include_str!("../../../templates/project-tests-README.md"),
    ),
    ("GUIDE.html", include_str!("../../../templates/GUIDE.html")),
];
