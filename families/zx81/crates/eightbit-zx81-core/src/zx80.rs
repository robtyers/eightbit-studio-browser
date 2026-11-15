//! Original 4K integer dialect and O export, from manufacturer format facts.
use eightbit_source_core::{Result, error};
use eightbit_zx81_emulator::{Model, image::Image};
pub const ASSEMBLY_ORIGIN: u16 = 0x4050;
const TOKENS: &[(&str, u8)] = &[
    ("THEN", 213),
    ("TO", 214),
    (";", 215),
    (",", 216),
    (")", 217),
    ("(", 218),
    ("NOT", 219),
    ("-", 220),
    ("+", 221),
    ("*", 222),
    ("/", 223),
    ("AND", 224),
    ("OR", 225),
    ("**", 226),
    ("=", 227),
    ("<", 228),
    (">", 229),
    ("LIST", 230),
    ("RETURN", 231),
    ("CLS", 232),
    ("DIM", 233),
    ("SAVE", 234),
    ("FOR", 235),
    ("GOTO", 236),
    ("GO TO", 236),
    ("POKE", 237),
    ("INPUT", 238),
    ("RANDOMISE", 239),
    ("LET", 240),
    ("NEXT", 243),
    ("PRINT", 244),
    ("NEW", 246),
    ("RUN", 247),
    ("STOP", 248),
    ("CONTINUE", 249),
    ("IF", 250),
    ("GOSUB", 251),
    ("GO SUB", 251),
    ("LOAD", 252),
    ("CLEAR", 253),
    ("REM", 254),
];
fn encode(byte: u8) -> Option<u8> {
    match byte.to_ascii_uppercase() {
        b' ' => Some(0),
        b'"' => Some(212),
        b'$' => Some(13),
        b':' => Some(14),
        b'?' => Some(15),
        b'(' => Some(16),
        b')' => Some(17),
        b'-' => Some(18),
        b'+' => Some(19),
        b'*' => Some(20),
        b'/' => Some(21),
        b'=' => Some(22),
        b'>' => Some(23),
        b'<' => Some(24),
        b';' => Some(25),
        b',' => Some(26),
        b'.' => Some(27),
        b'0'..=b'9' => Some(byte - b'0' + 28),
        b'A'..=b'Z' => Some(byte.to_ascii_uppercase() - b'A' + 38),
        _ => None,
    }
}
fn dialect() -> eightbit_sinclair_basic::Dialect {
    eightbit_sinclair_basic::Dialect {
        tokens: TOKENS,
        extra_statements: &[],
        extra_tokens: &[],
        escaped_quote: None,
        encode,
        number_marker: 126,
        newline: 118,
        max_line: 9999,
        statements: false,
    }
}
pub fn compile(model: Model, source: &str) -> Result<Vec<u8>> {
    if !model.zx80() {
        return Err(error(
            "DSS1110",
            "Choose an original ZX80 integer BASIC target",
        ));
    }
    let program = eightbit_sinclair_basic::integer_program(source, &dialect())?;
    Image::from_program(model, &program)
        .map(|image| image.bytes().to_vec())
        .map_err(|_| error("DSS1111", "Program exceeds the selected ZX80 RAM budget"))
}
pub fn decompile(model: Model, bytes: &[u8]) -> Result<String> {
    if !model.zx80() {
        return Err(error("DSS1120", "Choose a ZX80 O image"));
    }
    let image = Image::parse(model, bytes).map_err(|_| error("DSS1120", "Invalid ZX80 O image"))?;
    eightbit_sinclair_basic::integer_listing(image.programme(), &dialect())
}
/// Keep all 256 machine-code bytes representable in a stock ZX80 O cassette.
pub fn package_assembly(model: Model, code: &[u8]) -> Result<Vec<u8>> {
    let limit = if model.ram_bytes() == 1024 { 256 } else { 7800 };
    if !model.zx80() || code.is_empty() || code.len() > limit {
        return Err(error(
            "DSS1111",
            "Machine code exceeds the ZX80 target budget",
        ));
    }
    let adjust = code.len() & 255 == 118;
    let count = code.len() + usize::from(adjust);
    let source = format!(
        "COUNT EQU {count}\nCOUNT_ADJUST_BYTE EQU {}\n{}",
        if adjust { 11 } else { 0 },
        include_str!("../../../firmware/original/zx80-binary-loader.asm")
    );
    let mut stub = eightbit_z80_core::assembler::assemble(&source, 0x402b, None).map_err(|_| {
        error(
            "DSS1111",
            "Original ZX80 cassette bootstrap could not be built",
        )
    })?;
    if stub.bytes.len() > usize::from(ASSEMBLY_ORIGIN - 0x402b) || stub.bytes.contains(&118) {
        return Err(error("DSS1111", "Invalid original ZX80 cassette bootstrap"));
    }
    stub.bytes.resize(usize::from(ASSEMBLY_ORIGIN - 0x402b), 0);
    let mut program = vec![0, 10, 254];
    program.extend(stub.bytes);
    for byte in code {
        program.extend([28 + (byte >> 4), 28 + (byte & 15)]);
    }
    program.push(118);
    program.extend(eightbit_sinclair_basic::integer_program(
        "20 PRINT USR(16427)\n",
        &dialect(),
    )?);
    Image::from_program(model, &program)
        .map(|image| image.bytes().to_vec())
        .map_err(|_| {
            error(
                "DSS1111",
                "Machine code and bootstrap exceed the ZX80 RAM budget",
            )
        })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn integer_images_use_original_records_pointers_and_no_hidden_numbers() {
        let source = "10 LET A=32767\n20 PRINT A+1\n30 IF A>0 THEN GOTO 20\n";
        for model in [Model::Zx80_1k, Model::Zx80_16k] {
            let bytes = compile(model, source).unwrap();
            assert_eq!(&bytes[40..44], &[0, 10, 240, 0]);
            assert!(!bytes[40..].contains(&126));
            assert_eq!(decompile(model, &bytes).unwrap(), source);
            assert_eq!(
                usize::from(u16::from_le_bytes([bytes[10], bytes[11]])) - 0x4000,
                bytes.len()
            );
        }
    }
    #[test]
    fn quotes_and_punctuation_keep_native_token_and_character_distinction() {
        let source = "10 PRINT \"A+1\";CHR$(38)\n20 REM PRINT A+1\n";
        let bytes = compile(Model::Zx80_1k, source).unwrap();
        assert!(
            bytes
                .windows(5)
                .any(|bytes| bytes == [212, 38, 19, 29, 212])
        );
        assert!(bytes.contains(&215));
        assert_eq!(decompile(Model::Zx80_1k, &bytes).unwrap(), source);
    }
    #[test]
    fn float_literals_other_dialects_and_invalid_images_are_rejected() {
        for source in [
            "10 PRINT 32768\n",
            "10 PRINT 1.5\n",
            "10 PRINT 1E3\n",
            "10 PRINT 1:PRINT 2\n",
            "10 SLOW\n",
        ] {
            assert!(compile(Model::Zx80_1k, source).is_err(), "{source}");
        }
        assert!(compile(Model::Zx81_1k, "10 PRINT 1\n").is_err());
        let bytes = compile(Model::Zx80_1k, "10 PRINT 1\n").unwrap();
        for n in 0..bytes.len() {
            assert!(decompile(Model::Zx80_1k, &bytes[..n]).is_err());
        }
        assert!(decompile(Model::Zx80_1k, &vec![0; 65536]).is_err());
    }
    #[test]
    fn binary_bootstrap_keeps_halt_and_every_other_byte_out_of_rem_scanning() {
        for size in [1, 118, 119, 256] {
            let code: Vec<u8> = (0..size).map(|n| u8::try_from(n % 256).unwrap()).collect();
            let bytes = package_assembly(Model::Zx80_1k, &code).unwrap();
            let image = Image::parse(Model::Zx80_1k, &bytes).unwrap();
            let body = &image.programme()[3..];
            assert_eq!(
                body.iter().position(|byte| *byte == 118),
                Some(37 + size * 2)
            );
        }
        assert!(package_assembly(Model::Zx80_1k, &[]).is_err());
        assert!(package_assembly(Model::Zx81_1k, &[201]).is_err());
    }
    #[test]
    fn original_cpu_decodes_exported_binary_for_all_count_low_bytes() {
        use eightbit_z80_core::{Bus, Cpu};
        struct Ram(Vec<u8>);
        impl Bus for Ram {
            fn read(&mut self, address: u16) -> u8 {
                self.0[usize::from(address)]
            }
            fn write(&mut self, address: u16, value: u8) {
                self.0[usize::from(address)] = value;
            }
            fn input(&mut self, _: u16) -> u8 {
                255
            }
            fn output(&mut self, _: u16, _: u8) {}
            fn tick(&mut self, _: u32) {}
        }
        for size in (1..=256).chain([374, 630, 7800]) {
            let code: Vec<u8> = (0..size).map(|n| u8::try_from(n % 256).unwrap()).collect();
            let image = package_assembly(Model::Zx80_16k, &code).unwrap();
            let mut ram = Ram(vec![0; 65536]);
            ram.0[0x4000..0x4000 + image.len()].copy_from_slice(&image);
            let mut cpu = Cpu::default();
            cpu.registers.pc = 0x402b;
            cpu.registers.sp = 0x7ffe;
            for _ in 0..size * 25 + 32 {
                if cpu.registers.pc == ASSEMBLY_ORIGIN {
                    break;
                }
                cpu.step(&mut ram).unwrap();
            }
            assert_eq!(cpu.registers.pc, ASSEMBLY_ORIGIN, "size {size}");
            assert_eq!(
                &ram.0[usize::from(ASSEMBLY_ORIGIN)..usize::from(ASSEMBLY_ORIGIN) + size],
                code
            );
        }
    }
}
