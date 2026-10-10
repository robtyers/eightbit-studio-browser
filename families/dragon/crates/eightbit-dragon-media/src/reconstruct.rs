//! Original, conservative M6809 reconstruction from validated cassette bytes.
//! Opcode facts: Motorola M6809PM, Appendix F and indexed postbyte table.
use crate::{MachineCodeCassette, decode_wav_cassette, read_machine_code};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

/// Half-open address region chosen during review.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReconstructionRegion {
    /// First included Dragon address.
    pub start: u16,
    /// First excluded Dragon address.
    pub end: u16,
    /// Reviewer-selected classification.
    pub kind: RegionKind,
}

/// Classification is a review annotation, not a claim established by the bytes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RegionKind {
    /// Start control-flow traversal here.
    Code,
    /// Preserve as uninterpreted bytes.
    Data,
    /// Preserve bytes with an ASCII preview comment.
    Text,
    /// Preserve graphics bytes without inventing an image format.
    Graphics,
}

/// A portable symbol supplied by the reviewer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReconstructionLabel {
    /// Address receiving the label.
    pub address: u16,
    /// Unique portable assembler label.
    pub name: String,
}

/// Explicit container interpretation; raw bytes never guess their load address.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "format", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ReconstructionInput {
    /// One supported CAS or PCM WAV programme (the original default).
    #[default]
    Cassette,
    /// Headerless RAM bytes with reviewed addresses.
    Raw {
        /// RAM load address.
        #[serde(rename = "loadAddress")]
        load_address: u16,
        /// Entry inside the supplied bytes.
        #[serde(rename = "executionAddress")]
        execution_address: u16,
    },
    /// One DECB data segment and an execution postamble.
    Decb,
    /// Ordinary 2/4/8/16 KiB cartridge mapped at $C000.
    Cartridge,
}

/// Bounded input; never accepts assembler source from the `WebView`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReconstructionOptions {
    /// Explicit input interpretation, defaulting to the original cassette contract.
    #[serde(default)]
    pub input: ReconstructionInput,
    /// Non-overlapping reviewer-selected regions, at most 256.
    pub regions: Vec<ReconstructionRegion>,
    /// Reviewer-selected names, at most 256.
    pub labels: Vec<ReconstructionLabel>,
}

/// Complete proposal, identified by both original and generated source hashes.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconstructionPreview {
    /// Version of this preview contract.
    pub schema_version: u32,
    /// Whether the payload is an ordinary cartridge rather than RAM code.
    pub cartridge: bool,
    /// Identity of the original CAS or WAV bytes.
    pub input_sha256: String,
    /// Identity of the decoded machine-code payload.
    pub payload_sha256: String,
    /// Identity of the exact generated source shown for approval.
    pub source_sha256: String,
    /// Validated internal cassette filename.
    pub filename: String,
    /// Original load address.
    pub load_address: u16,
    /// Original execution address, inside the payload.
    pub execution_address: u16,
    /// Complete decoded byte count.
    pub payload_bytes: usize,
    /// Bytes provisionally interpreted as instructions.
    pub code_bytes: usize,
    /// Complete inert LWTOOLS assembly proposal.
    pub source: String,
    /// Bounded interpretation limitations.
    pub warnings: Vec<String>,
}

/// A stable, actionable reconstruction rejection.
#[derive(Debug, Error)]
#[error("DSM1300: {0}")]
pub struct ReconstructionError(pub String);

fn invalid(message: &str) -> ReconstructionError {
    ReconstructionError(message.to_owned())
}

/// Decode only one existing checksum-valid cassette programme, optionally in WAV.
pub fn reconstruction_machine(bytes: &[u8]) -> Result<MachineCodeCassette, ReconstructionError> {
    let cassette;
    let raw = if bytes.starts_with(b"RIFF") {
        cassette = decode_wav_cassette(bytes)
            .map_err(|e| ReconstructionError(e.to_string()))?
            .cassette;
        &cassette
    } else {
        bytes
    };
    let machine = read_machine_code(raw).map_err(|e| ReconstructionError(e.to_string()))?;
    let start = usize::from(machine.load_address);
    let end = start
        .checked_add(machine.program.len())
        .ok_or_else(|| invalid("payload address overflow"))?;
    if start < 0x400
        || end > 0xc000
        || machine.program.is_empty()
        || !(start..end).contains(&usize::from(machine.execution_address))
    {
        return Err(invalid(
            "require a non-empty $0400-$BFFF payload with its entry inside the payload",
        ));
    }
    Ok(machine)
}

/// Decode an explicitly chosen binary/container into bounded addresses and bytes.
pub fn reconstruction_input(
    bytes: &[u8],
    input: &ReconstructionInput,
) -> Result<MachineCodeCassette, ReconstructionError> {
    if bytes.len() > crate::MAX_WAV_BYTES {
        return Err(invalid("input exceeds 64 MiB"));
    }
    let (program, load, entry) = match input {
        ReconstructionInput::Cassette => return reconstruction_machine(bytes),
        ReconstructionInput::Raw {
            load_address,
            execution_address,
        } => (bytes, *load_address, *execution_address),
        ReconstructionInput::Decb => {
            if bytes.len() < 11 || bytes[0] != 0 {
                return Err(invalid(
                    "DECB requires one data record and an execution postamble",
                ));
            }
            let length = usize::from(u16::from_be_bytes([bytes[1], bytes[2]]));
            let end = 5 + length;
            if bytes.len() != end + 5 || bytes[end..end + 3] != [0xff, 0, 0] {
                return Err(invalid(
                    "DECB must contain exactly one data segment, a valid postamble and no trailing bytes",
                ));
            }
            (
                &bytes[5..end],
                u16::from_be_bytes([bytes[3], bytes[4]]),
                u16::from_be_bytes([bytes[end + 3], bytes[end + 4]]),
            )
        }
        ReconstructionInput::Cartridge => {
            crate::validate_cartridge(bytes).map_err(|error| invalid(&error.to_string()))?;
            (bytes, 0xc000, 0xc000)
        }
    };
    let end = usize::from(load)
        .checked_add(program.len())
        .ok_or_else(|| invalid("binary address overflow"))?;
    if program.is_empty()
        || !(usize::from(load)..end).contains(&usize::from(entry))
        || (input != &ReconstructionInput::Cartridge && (load < 0x400 || end > 0xc000))
    {
        return Err(invalid(
            "RAM input requires a non-empty $0400-$BFFF payload and an entry inside it",
        ));
    }
    let cassette = crate::create_machine_code("BINARY", program, load, entry)
        .map_err(|error| invalid(&error.to_string()))?;
    read_machine_code(&cassette).map_err(|error| invalid(&error.to_string()))
}

/// SHA-256 identity for portable reconstruction evidence.
#[must_use]
pub fn reconstruction_hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// Build an inert, deterministic proposal. Work is bounded by the supported Dragon 32/64 RAM/ROM windows.
pub fn reconstruct_machine_code(
    bytes: &[u8],
    options: &ReconstructionOptions,
) -> Result<ReconstructionPreview, ReconstructionError> {
    let mut machine = reconstruction_input(bytes, &options.input)?;
    let full_program = machine.program.clone();
    // The last page is hardware I/O, not executable cartridge memory. Preserve
    // every image byte while keeping control-flow analysis below $FF00.
    if options.input == ReconstructionInput::Cartridge {
        machine.program.truncate(0x3f00);
    }
    let base = machine.load_address;
    let end =
        base + u16::try_from(machine.program.len()).map_err(|_| invalid("payload too large"))?;
    if options.regions.len() > 256 || options.labels.len() > 256 {
        return Err(invalid("at most 256 regions and 256 labels are supported"));
    }
    let mut regions = options.regions.clone();
    regions.sort_by_key(|r| r.start);
    let mut previous = base;
    for r in &regions {
        if r.start < previous || r.start < base || r.end > end || r.end <= r.start {
            return Err(invalid(
                "regions must be non-overlapping, non-empty and inside the payload",
            ));
        }
        previous = r.end;
    }
    let mut names = BTreeSet::new();
    let mut labels = BTreeMap::new();
    for label in &options.labels {
        let name = &label.name;
        let upper = name.to_ascii_uppercase();
        if !(base..end).contains(&label.address)
            || name.is_empty()
            || name.len() > 32
            || !name.as_bytes()[0].is_ascii_alphabetic()
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            || upper == "START"
            || upper.starts_with("L_")
            || !names.insert(upper)
            || labels.insert(label.address, name.clone()).is_some()
        {
            return Err(invalid(
                "labels require unique addresses and ASCII names (1-32 characters); START and L_ names are reserved",
            ));
        }
    }
    let kind = |address| {
        regions
            .iter()
            .find(|r| r.start <= address && address < r.end)
            .map(|r| r.kind)
    };
    let mut queue = BTreeSet::from([machine.execution_address]);
    queue.extend(
        regions
            .iter()
            .filter(|r| r.kind == RegionKind::Code)
            .map(|r| r.start),
    );
    let mut visited = BTreeSet::new();
    let mut decoded = BTreeMap::<u16, Instruction>::new();
    let mut owners = vec![None; machine.program.len()];
    let mut conflicts = BTreeSet::new();
    let mut uncertain = 0usize;
    let mut targets = BTreeSet::from([machine.execution_address]);
    targets.extend(labels.keys().copied());
    while let Some(address) = queue.pop_first() {
        if !(base..end).contains(&address) || !visited.insert(address) {
            continue;
        }
        if kind(address).is_some_and(|k| k != RegionKind::Code) {
            continue;
        }
        let offset = usize::from(address - base);
        let Some(instruction) = decode(address, &machine.program[offset..]) else {
            uncertain += 1;
            continue;
        };
        let next = address
            + u16::try_from(instruction.length).map_err(|_| invalid("instruction too large"))?;
        if next > end
            || (address..next).any(|a| kind(a).is_some_and(|k| k != RegionKind::Code))
            || labels.range((address + 1)..next).next().is_some()
        {
            uncertain += 1;
            continue;
        }
        for owner in owners[offset..offset + instruction.length].iter().flatten() {
            conflicts.insert(*owner);
            conflicts.insert(address);
        }
        if conflicts.contains(&address) {
            uncertain += 1;
            continue;
        }
        for owner in &mut owners[offset..offset + instruction.length] {
            *owner = Some(address);
        }
        if let Some(target) = instruction.target
            && (base..end).contains(&target)
        {
            queue.insert(target);
            targets.insert(target);
        }
        if instruction.next {
            queue.insert(next);
        }
        if instruction.unresolved {
            uncertain += 1;
        }
        decoded.insert(address, instruction);
    }
    for address in conflicts {
        decoded.remove(&address);
    }
    // A discovered branch into an instruction invalidates that interpretation.
    decoded.retain(|address, instruction| {
        targets
            .range((address + 1)..(address + u16::try_from(instruction.length).unwrap_or(0)))
            .next()
            .is_none()
    });
    for &address in &targets {
        labels
            .entry(address)
            .or_insert_with(|| format!("L_{address:04X}"));
    }
    labels
        .entry(machine.execution_address)
        .or_insert_with(|| "entry".to_owned());
    let mut source = format!(
        "; Reconstructed M6809 source: original comments and names are unavailable.\n; Original payload SHA-256: {}\n; Labels and code/data classifications require review.\n        section code\n        export START\n",
        reconstruction_hash(&full_program)
    );
    let mut address = base;
    let mut code_bytes = 0;
    while address < end {
        if address == machine.execution_address {
            source.push_str("START\n");
        }
        if let Some(label) = labels.get(&address) {
            source.push_str(label);
            source.push('\n');
        }
        if let Some(instruction) = decoded.get(&address) {
            let operand = instruction
                .target
                .filter(|t| (base..end).contains(t))
                .and_then(|t| labels.get(&t))
                .map_or_else(
                    || instruction.operand.clone(),
                    |label| {
                        if instruction.extended_target {
                            format!(">{label}")
                        } else {
                            label.clone()
                        }
                    },
                );
            source.push_str(&format!(
                "        {} {} ; ${address:04X}\n",
                instruction.mnemonic, operand
            ));
            code_bytes += instruction.length;
            address += u16::try_from(instruction.length).unwrap_or(1);
        } else {
            let start = address;
            let annotation = kind(address);
            address += 1;
            while address < end
                && address - start < 8
                && !decoded.contains_key(&address)
                && !labels.contains_key(&address)
                && kind(address) == annotation
            {
                address += 1;
            }
            let data = &machine.program[usize::from(start - base)..usize::from(address - base)];
            let values = data
                .iter()
                .map(|b| format!("${b:02X}"))
                .collect::<Vec<_>>()
                .join(",");
            let description = match annotation {
                Some(RegionKind::Text) => format!(
                    "text: {}",
                    data.iter()
                        .map(|b| if (32..=126).contains(b) {
                            char::from(*b)
                        } else {
                            '.'
                        })
                        .collect::<String>()
                ),
                Some(RegionKind::Graphics) => "graphics (bytes preserved)".to_owned(),
                Some(RegionKind::Data) => "data".to_owned(),
                _ => "unclassified bytes".to_owned(),
            };
            source.push_str(&format!(
                "        fcb {values} ; ${start:04X} {description}\n"
            ));
        }
    }
    if full_program.len() > machine.program.len() {
        for chunk in full_program[machine.program.len()..].chunks(8) {
            let values = chunk
                .iter()
                .map(|byte| format!("${byte:02X}"))
                .collect::<Vec<_>>()
                .join(",");
            source.push_str(&format!(
                "        fcb {values} ; cartridge I/O-page bytes preserved\n"
            ));
        }
    }
    source.push_str("        endsection\n");
    let mut warnings = vec!["Reconstructed source: original names, comments and file structure are unavailable. Byte equivalence does not prove code/data interpretation or runtime safety.".to_owned()];
    if uncertain > 0 {
        warnings.push(format!("{uncertain} unresolved or unsupported control-flow sites; review unclassified bytes and add code regions where justified."));
    }
    Ok(ReconstructionPreview {
        schema_version: 1,
        cartridge: options.input == ReconstructionInput::Cartridge,
        input_sha256: reconstruction_hash(bytes),
        payload_sha256: reconstruction_hash(&full_program),
        source_sha256: reconstruction_hash(source.as_bytes()),
        filename: machine.filename,
        load_address: base,
        execution_address: machine.execution_address,
        payload_bytes: full_program.len(),
        code_bytes,
        source,
        warnings,
    })
}

struct Instruction {
    length: usize,
    mnemonic: String,
    operand: String,
    target: Option<u16>,
    next: bool,
    extended_target: bool,
    unresolved: bool,
}
impl Instruction {
    fn new(length: usize, mnemonic: &str, operand: String) -> Self {
        Self {
            length,
            mnemonic: mnemonic.to_owned(),
            operand,
            target: None,
            next: true,
            extended_target: false,
            unresolved: false,
        }
    }
}
fn word(bytes: &[u8], index: usize) -> Option<u16> {
    Some(u16::from_be_bytes([
        *bytes.get(index)?,
        *bytes.get(index + 1)?,
    ]))
}
fn relative(address: u16, length: usize, displacement: i16) -> u16 {
    address
        .wrapping_add(u16::try_from(length).unwrap_or(0))
        .wrapping_add_signed(displacement)
}

fn decode(address: u16, bytes: &[u8]) -> Option<Instruction> {
    let mut opcode = *bytes.first()?;
    let mut prefix = 0;
    let mut n = 1;
    if opcode == 0x10 || opcode == 0x11 {
        prefix = opcode;
        opcode = *bytes.get(1)?;
        n = 2;
    }
    const BRANCH: [&str; 16] = [
        "BRA", "BRN", "BHI", "BLS", "BCC", "BCS", "BNE", "BEQ", "BVC", "BVS", "BPL", "BMI", "BGE",
        "BLT", "BGT", "BLE",
    ];
    if (prefix == 0 && ((0x20..=0x2f).contains(&opcode) || matches!(opcode, 0x16 | 0x17 | 0x8d)))
        || (prefix == 0x10 && (0x21..=0x2f).contains(&opcode))
    {
        let long = prefix == 0x10 || matches!(opcode, 0x16 | 0x17);
        let displacement = if long {
            i16::from_be_bytes(word(bytes, n)?.to_be_bytes())
        } else {
            i16::from(i8::from_ne_bytes([*bytes.get(n)?]))
        };
        let length = n + if long { 2 } else { 1 };
        let mnemonic = match opcode {
            0x16 => "LBRA".to_owned(),
            0x17 => "LBSR".to_owned(),
            0x8d => "BSR".to_owned(),
            _ => format!(
                "{}{}",
                if long { "L" } else { "" },
                BRANCH[usize::from(opcode & 15)]
            ),
        };
        let mut i = Instruction::new(
            length,
            &mnemonic,
            format!(
                "*{:+}",
                i32::from(displacement) + i32::try_from(length).ok()?
            ),
        );
        if opcode != 0x21 {
            i.target = Some(relative(address, length, displacement));
        }
        i.next = !matches!(opcode, 0x20 | 0x16);
        return Some(i);
    }
    if prefix != 0 && opcode == 0x3f {
        let mut i = Instruction::new(
            2,
            if prefix == 0x10 { "SWI2" } else { "SWI3" },
            String::new(),
        );
        i.next = false;
        return Some(i);
    }
    if prefix == 0 {
        let inherent = match opcode {
            0x12 => "NOP",
            0x13 => "SYNC",
            0x19 => "DAA",
            0x1d => "SEX",
            0x39 => "RTS",
            0x3a => "ABX",
            0x3b => "RTI",
            0x3d => "MUL",
            0x3f => "SWI",
            _ => "",
        };
        if !inherent.is_empty() {
            let mut i = Instruction::new(1, inherent, String::new());
            i.next = !matches!(opcode, 0x39 | 0x3b | 0x3f);
            return Some(i);
        }
        if matches!(opcode, 0x1a | 0x1c | 0x3c) {
            return Some(Instruction::new(
                2,
                match opcode {
                    0x1a => "ORCC",
                    0x1c => "ANDCC",
                    _ => "CWAI",
                },
                format!("#${:02X}", bytes.get(1)?),
            ));
        }
        if matches!(opcode, 0x1e | 0x1f) {
            let b = *bytes.get(1)?;
            let reg = |r| match r {
                0 => Some("D"),
                1 => Some("X"),
                2 => Some("Y"),
                3 => Some("U"),
                4 => Some("S"),
                5 => Some("PC"),
                8 => Some("A"),
                9 => Some("B"),
                10 => Some("CC"),
                11 => Some("DP"),
                _ => None,
            };
            if (b >> 4 < 8) != (b & 15 < 8) {
                return None;
            }
            let mut i = Instruction::new(
                2,
                if opcode == 0x1e { "EXG" } else { "TFR" },
                format!("{},{}", reg(b >> 4)?, reg(b & 15)?),
            );
            if b & 15 == 5 || (opcode == 0x1e && b >> 4 == 5) {
                i.next = false;
                i.unresolved = true;
            }
            return Some(i);
        }
        if (0x34..=0x37).contains(&opcode) {
            let b = *bytes.get(1)?;
            if b == 0 {
                return None;
            }
            let regs = [
                "CC",
                "A",
                "B",
                "DP",
                "X",
                "Y",
                if opcode < 0x36 { "U" } else { "S" },
                "PC",
            ];
            let mut i = Instruction::new(
                2,
                ["PSHS", "PULS", "PSHU", "PULU"][usize::from(opcode - 0x34)],
                regs.iter()
                    .enumerate()
                    .filter(|(j, _)| b & (1 << j) != 0)
                    .map(|(_, r)| *r)
                    .collect::<Vec<_>>()
                    .join(","),
            );
            i.next = !(opcode & 1 != 0 && b & 128 != 0);
            return Some(i);
        }
        if (0x30..=0x33).contains(&opcode) {
            let (length, operand) = indexed(bytes, 1, address)?;
            return Some(Instruction::new(
                length,
                ["LEAX", "LEAY", "LEAS", "LEAU"][usize::from(opcode - 0x30)],
                operand,
            ));
        }
        if opcode < 0x10 || (0x40..0x80).contains(&opcode) {
            let op = match opcode & 15 {
                0 => "NEG",
                3 => "COM",
                4 => "LSR",
                6 => "ROR",
                7 => "ASR",
                8 => "ASL",
                9 => "ROL",
                10 => "DEC",
                12 => "INC",
                13 => "TST",
                14 => "JMP",
                15 => "CLR",
                _ => return None,
            };
            let group = opcode >> 4;
            if group == 4 || group == 5 {
                if op == "JMP" {
                    return None;
                }
                return Some(Instruction::new(
                    1,
                    &format!("{op}{}", if group == 4 { "A" } else { "B" }),
                    String::new(),
                ));
            }
            let (length, operand) = match group {
                0 => (2, format!("<${:02X}", bytes.get(1)?)),
                6 => indexed(bytes, 1, address)?,
                7 => (3, format!(">${:04X}", word(bytes, 1)?)),
                _ => return None,
            };
            let mut i = Instruction::new(length, op, operand);
            if op == "JMP" {
                i.next = false;
                i.unresolved = group != 7;
                if group == 7 {
                    i.target = Some(word(bytes, 1)?);
                    i.extended_target = true;
                }
            }
            return Some(i);
        }
    }
    if opcode < 0x80 {
        return None;
    }
    let low = opcode & 15;
    let group = opcode >> 4;
    let mode = group & 3;
    let (op, wide) = if prefix == 0 {
        if group < 12 {
            match low {
                0 => ("SUBA", false),
                1 => ("CMPA", false),
                2 => ("SBCA", false),
                3 => ("SUBD", true),
                4 => ("ANDA", false),
                5 => ("BITA", false),
                6 => ("LDA", false),
                7 => ("STA", false),
                8 => ("EORA", false),
                9 => ("ADCA", false),
                10 => ("ORA", false),
                11 => ("ADDA", false),
                12 => ("CMPX", true),
                13 => ("JSR", false),
                14 => ("LDX", true),
                15 => ("STX", true),
                _ => return None,
            }
        } else {
            match low {
                0 => ("SUBB", false),
                1 => ("CMPB", false),
                2 => ("SBCB", false),
                3 => ("ADDD", true),
                4 => ("ANDB", false),
                5 => ("BITB", false),
                6 => ("LDB", false),
                7 => ("STB", false),
                8 => ("EORB", false),
                9 => ("ADCB", false),
                10 => ("ORB", false),
                11 => ("ADDB", false),
                12 => ("LDD", true),
                13 => ("STD", true),
                14 => ("LDU", true),
                15 => ("STU", true),
                _ => return None,
            }
        }
    } else if prefix == 0x10 {
        match (group < 12, low) {
            (true, 3) => ("CMPD", true),
            (true, 12) => ("CMPY", true),
            (true, 14) => ("LDY", true),
            (true, 15) => ("STY", true),
            (false, 14) => ("LDS", true),
            (false, 15) => ("STS", true),
            _ => return None,
        }
    } else {
        match (group < 12, low) {
            (true, 3) => ("CMPU", true),
            (true, 12) => ("CMPS", true),
            _ => return None,
        }
    };
    if mode == 0 && (op.starts_with("ST") || op == "JSR") {
        return None;
    }
    let (length, operand) = match mode {
        0 => {
            if wide {
                (n + 2, format!("#${:04X}", word(bytes, n)?))
            } else {
                (n + 1, format!("#${:02X}", bytes.get(n)?))
            }
        }
        1 => (n + 1, format!("<${:02X}", bytes.get(n)?)),
        2 => indexed(bytes, n, address)?,
        _ => (n + 2, format!(">${:04X}", word(bytes, n)?)),
    };
    let mut i = Instruction::new(length, op, operand);
    if op == "JSR" {
        if mode == 3 {
            i.target = Some(word(bytes, n)?);
            i.extended_target = true;
        } else {
            i.unresolved = true;
        }
    }
    Some(i)
}

fn indexed(bytes: &[u8], n: usize, address: u16) -> Option<(usize, String)> {
    let post = *bytes.get(n)?;
    let reg = ["X", "Y", "U", "S"][usize::from((post >> 5) & 3)];
    if post & 128 == 0 {
        let offset = i16::from(post & 31) - if post & 16 != 0 { 32 } else { 0 };
        return Some((n + 1, format!("<<{offset},{reg}")));
    }
    let indirect = post & 16 != 0;
    let mode = post & 15;
    let (length, operand) = match mode {
        0 if !indirect => (n + 1, format!(",{reg}+")),
        1 => (n + 1, format!(",{reg}++")),
        2 if !indirect => (n + 1, format!(",-{reg}")),
        3 => (n + 1, format!(",--{reg}")),
        4 => (n + 1, format!(",{reg}")),
        5 => (n + 1, format!("B,{reg}")),
        6 => (n + 1, format!("A,{reg}")),
        11 => (n + 1, format!("D,{reg}")),
        8 => (
            n + 2,
            format!("<{},{}", i8::from_ne_bytes([*bytes.get(n + 1)?]), reg),
        ),
        9 => (
            n + 3,
            format!(
                ">{},{}",
                i16::from_be_bytes(word(bytes, n + 1)?.to_be_bytes()),
                reg
            ),
        ),
        12 | 13 if post & 0x60 == 0 => {
            let wide = mode == 13;
            let length = n + if wide { 3 } else { 2 };
            let offset = if wide {
                i16::from_be_bytes(word(bytes, n + 1)?.to_be_bytes())
            } else {
                i16::from(i8::from_ne_bytes([*bytes.get(n + 1)?]))
            };
            let _target = relative(address, length, offset);
            (
                length,
                format!(
                    "{}*{:+},PCR",
                    if wide { ">" } else { "<" },
                    i32::from(offset) + i32::try_from(length).ok()?
                ),
            )
        }
        15 if indirect && post & 0x60 == 0 => (n + 3, format!("${:04X}", word(bytes, n + 1)?)),
        _ => return None,
    };
    Some((
        length,
        if indirect {
            format!("[{operand}]")
        } else {
            operand
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{create_machine_code, encode_cassette_wav};
    fn media(data: &[u8]) -> Vec<u8> {
        create_machine_code("ORIGINAL", data, 0x4000, 0x4000).unwrap()
    }
    #[test]
    fn traversal_keeps_unreachable_data_and_follows_calls() {
        let p = reconstruct_machine_code(
            &media(&[0xbd, 0x40, 0x06, 0x39, 0x41, 0x42, 0x86, 0x2a, 0x39]),
            &ReconstructionOptions::default(),
        )
        .unwrap();
        assert_eq!(p.code_bytes, 7);
        assert!(p.source.contains("JSR >L_4006"));
        assert!(p.source.contains("fcb $41,$42"));
        assert!(p.source.contains("LDA #$2A"));
    }
    #[test]
    fn wav_and_cas_produce_same_source() {
        let cas = media(&[0x12, 0x39]);
        let wav = encode_cassette_wav(&cas).unwrap();
        let a = reconstruct_machine_code(&cas, &ReconstructionOptions::default()).unwrap();
        let b = reconstruct_machine_code(&wav, &ReconstructionOptions::default()).unwrap();
        assert_eq!(a.source, b.source);
        assert_ne!(a.input_sha256, b.input_sha256);
    }
    #[test]
    fn explicit_data_and_labels_are_preserved() {
        let mut o = ReconstructionOptions::default();
        o.regions.push(ReconstructionRegion {
            start: 0x4000,
            end: 0x4002,
            kind: RegionKind::Graphics,
        });
        o.labels.push(ReconstructionLabel {
            address: 0x4000,
            name: "Picture".into(),
        });
        let p = reconstruct_machine_code(&media(&[0x12, 0x39]), &o).unwrap();
        assert_eq!(p.code_bytes, 0);
        assert!(p.source.contains("Picture\n"));
        assert!(p.source.contains("graphics"));
        o.labels[0].name = "x\n include secret".into();
        assert!(reconstruct_machine_code(&media(&[0x12, 0x39]), &o).is_err());
    }
    #[test]
    fn bad_regions_entry_and_opcode_are_rejected_or_retained() {
        let cas = media(&[0x02, 0x12, 0x39]);
        let p = reconstruct_machine_code(&cas, &ReconstructionOptions::default()).unwrap();
        assert_eq!(p.code_bytes, 0);
        let o = ReconstructionOptions {
            input: ReconstructionInput::Cassette,
            regions: vec![ReconstructionRegion {
                start: 0x3fff,
                end: 0x4002,
                kind: RegionKind::Code,
            }],
            labels: vec![],
        };
        assert!(reconstruct_machine_code(&cas, &o).is_err());
        assert!(
            reconstruction_machine(&create_machine_code("BAD", &[0x39], 0xc000, 0xc000).unwrap())
                .is_err()
        );
    }
    #[test]
    fn branch_into_operand_reverts_overlapping_interpretation() {
        let p = reconstruct_machine_code(
            &media(&[0x86, 0x12, 0x20, 0xfd]),
            &ReconstructionOptions::default(),
        )
        .unwrap();
        assert!(p.source.contains("fcb $86"));
        assert!(!p.source.contains("LDA"));
    }
    #[test]
    fn indexed_widths_and_prefixed_instructions() {
        for (bytes, expected) in [
            (&[0xa6, 0x00][..], "<<0,X"),
            (&[0xa6, 0x88, 0x00], "<0,X"),
            (&[0xa6, 0x89, 0x00, 0x00], ">0,X"),
        ] {
            assert_eq!(decode(0x4000, bytes).unwrap().operand, expected);
        }
        assert_eq!(
            decode(0x4000, &[0x10, 0xce, 0x7f, 0xff]).unwrap().mnemonic,
            "LDS"
        );
        assert!(decode(0x4000, &[0x10, 0xce, 0x7f]).is_none());
    }
    proptest::proptest! { #[test] fn arbitrary_payloads_are_total(data in proptest::collection::vec(proptest::num::u8::ANY,1..1024)) {let p=reconstruct_machine_code(&media(&data),&ReconstructionOptions::default()).unwrap();proptest::prop_assert!(p.code_bytes<=data.len());proptest::prop_assert!(p.source.len()<1024*128);} }
}

#[cfg(test)]
mod binary_input_tests {
    use super::*;
    #[test]
    fn raw_and_decb_require_exact_addresses_and_one_segment() {
        let raw = ReconstructionInput::Raw {
            load_address: 0x4000,
            execution_address: 0x4000,
        };
        let machine = reconstruction_input(&[0x86, 0x2a, 0x39], &raw).unwrap();
        assert_eq!(machine.load_address, 0x4000);
        let decb = [0, 0, 3, 0x40, 0, 0x86, 0x2a, 0x39, 0xff, 0, 0, 0x40, 0];
        assert_eq!(
            reconstruction_input(&decb, &ReconstructionInput::Decb)
                .unwrap()
                .program,
            machine.program
        );
        for bytes in [
            &decb[..12],
            &decb[..5],
            &[0, 0, 0, 0x40, 0, 0xff, 0, 0, 0x40, 0][..],
        ] {
            assert!(reconstruction_input(bytes, &ReconstructionInput::Decb).is_err());
        }
        let mut trailing = decb.to_vec();
        trailing.push(0);
        assert!(reconstruction_input(&trailing, &ReconstructionInput::Decb).is_err());
        for input in [
            ReconstructionInput::Raw {
                load_address: 0xbfff,
                execution_address: 0xbfff,
            },
            ReconstructionInput::Raw {
                load_address: 0x4000,
                execution_address: 0x3000,
            },
            ReconstructionInput::Raw {
                load_address: 0xc000,
                execution_address: 0xc000,
            },
        ] {
            assert!(reconstruction_input(&[0x12, 0x39], &input).is_err());
        }
    }
    #[test]
    fn cartridge_reconstruction_preserves_every_byte_and_never_decodes_the_io_page() {
        for size in [2048, 4096, 8192, 16384] {
            let mut bytes = vec![0xff; size];
            bytes[..2].copy_from_slice(&[0x12, 0x39]);
            let options = ReconstructionOptions {
                input: ReconstructionInput::Cartridge,
                ..Default::default()
            };
            let preview = reconstruct_machine_code(&bytes, &options).unwrap();
            assert!(preview.cartridge);
            assert_eq!(preview.payload_bytes, size);
            assert_eq!(preview.load_address, 0xc000);
            assert_eq!(preview.payload_sha256, reconstruction_hash(&bytes));
            if size == 16384 {
                assert_eq!(
                    preview
                        .source
                        .matches("cartridge I/O-page bytes preserved")
                        .count(),
                    32
                );
            }
        }
        assert!(
            reconstruct_machine_code(
                &[0; 2049],
                &ReconstructionOptions {
                    input: ReconstructionInput::Cartridge,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
}
