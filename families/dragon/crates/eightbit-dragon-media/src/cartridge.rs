//! Ordinary Dragon ROM images and an original RAM-copy startup loader.

use eightbit_dragon_basic_core::{Dialect, detokenise};
use thiserror::Error;

/// Cartridge mapping base on Dragon 32.
pub const CARTRIDGE_BASE: u16 = 0xc000;
/// Largest ordinary image; the last page is obscured by I/O and vectors.
pub const MAX_CARTRIDGE_BYTES: usize = 16 * 1024;
const PAYLOAD_OFFSET: usize = 128;
const VISIBLE_BYTES: usize = 0x3f00;
const BASIC_START: u16 = eightbit_dragon_targets::BASIC_START;

/// Stable cartridge validation/packaging error, containing no input bytes.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("DSM2300: {0}")]
pub struct CartridgeError(pub String);

/// Optional already-linked RAM code included alongside the BASIC programme.
pub struct CartridgeAssembly<'a> {
    /// First destination address of the linked binary.
    pub load_address: u16,
    /// Original linked bytes, executed only by authored BASIC.
    pub bytes: &'a [u8],
}

/// Validate container size only: raw images carry no reliable type signature.
pub fn validate_cartridge(bytes: &[u8]) -> Result<(), CartridgeError> {
    if !matches!(bytes.len(), 2048 | 4096 | 8192 | 16384) {
        return Err(CartridgeError(
            "ordinary cartridge images must be exactly 2, 4, 8 or 16 KiB; bank-switched and headered images are unsupported".into(),
        ));
    }
    Ok(())
}

/// Package BASIC and optional RAM-linked assembly in a standard auto-start ROM.
/// No firmware bytes or assembler installation are needed to generate the loader.
/// The documented ROM calls and BASIC workspace are Dragon 32 profile-specific.
pub fn create_program_cartridge(
    basic: &[u8],
    assembly: Option<CartridgeAssembly<'_>>,
) -> Result<Vec<u8>, CartridgeError> {
    let machine_bytes = assembly.as_ref().map_or(0, |code| code.bytes.len());
    let used = PAYLOAD_OFFSET
        .checked_add(basic.len())
        .and_then(|size| size.checked_add(machine_bytes))
        .filter(|size| *size <= VISIBLE_BYTES)
        .ok_or_else(|| {
            CartridgeError(
                "programme and startup loader exceed the 16 KiB cartridge address window".into(),
            )
        })?;
    let dialect = Dialect::dragon32()
        .map_err(|_| CartridgeError("Dragon BASIC dialect is unavailable".into()))?;
    let listing = detokenise(basic, &dialect).map_err(|_| {
        CartridgeError("cartridge requires a valid linked Dragon BASIC token stream".into())
    })?;
    if !listing.warnings.is_empty() || basic.len() < 7 {
        return Err(CartridgeError(
            "cartridge requires a non-empty canonical BASIC programme".into(),
        ));
    }
    let basic_end = BASIC_START
        .checked_add(word(basic.len())?)
        .ok_or_else(|| CartridgeError("BASIC programme exceeds RAM".into()))?;
    let ram_top = if let Some(code) = &assembly {
        let end = usize::from(code.load_address).checked_add(code.bytes.len());
        if code.bytes.is_empty()
            || end.is_none_or(|end| end > 0x7ffe)
            || usize::from(code.load_address) < usize::from(basic_end) + 512
        {
            return Err(CartridgeError("cartridge assembly must fit below $7FFE and leave at least 512 bytes above BASIC for its workspace".into()));
        }
        code.load_address - 1
    } else {
        0x7ffe
    };

    // Disable cartridge FIRQ, establish direct page and a stack below any copied
    // machine code. All bytes below encode our own short sequence, not ROM code.
    let mut loader = vec![0x1a, 0x50, 0x4f, 0x1f, 0x8b, 0x86, 0x34, 0xb7, 0xff, 0x23];
    loader.extend([0x10, 0xce]); // LDS #workspace stack
    loader.extend((ram_top - 200).to_be_bytes());
    copy_loop(
        &mut loader,
        CARTRIDGE_BASE + word(PAYLOAD_OFFSET)?,
        BASIC_START,
        word(basic.len())?,
    );
    if let Some(code) = &assembly {
        copy_loop(
            &mut loader,
            CARTRIDGE_BASE + word(PAYLOAD_OFFSET + basic.len())?,
            code.load_address,
            word(code.bytes.len())?,
        );
    }
    loader.extend([0x7f, 0x1e, 0x00]); // CLR sentinel immediately before BASIC
    store_word(&mut loader, BASIC_START, &[0x19]);
    store_word(&mut loader, basic_end, &[0x1b, 0x1d, 0x1f]);
    store_word(&mut loader, ram_top, &[0x23, 0x27]);
    store_word(&mut loader, ram_top - 200, &[0x21]);
    store_word(&mut loader, BASIC_START - 1, &[0xa6]);
    loader.extend([0x1c, 0xaf, 0x4f, 0x7e, 0x84, 0x9f]); // ANDCC #$AF; CLRA; JMP RUN BASIC
    if loader.len() > PAYLOAD_OFFSET {
        return Err(CartridgeError(
            "cartridge startup loader exceeds its reserved area".into(),
        ));
    }
    let mut rom = vec![
        0xff;
        if used <= 8192 {
            8192
        } else {
            MAX_CARTRIDGE_BYTES
        }
    ];
    rom[..loader.len()].copy_from_slice(&loader);
    rom[PAYLOAD_OFFSET..PAYLOAD_OFFSET + basic.len()].copy_from_slice(basic);
    if let Some(code) = assembly {
        rom[PAYLOAD_OFFSET + basic.len()..used].copy_from_slice(code.bytes);
    }
    Ok(rom)
}

fn word(value: usize) -> Result<u16, CartridgeError> {
    u16::try_from(value).map_err(|_| CartridgeError("cartridge address overflow".into()))
}
fn store_word(loader: &mut Vec<u8>, value: u16, addresses: &[u8]) {
    loader.push(0x8e); // LDX #value
    loader.extend(value.to_be_bytes());
    for address in addresses {
        loader.extend([0x9f, *address]);
    } // STX direct
}
fn copy_loop(loader: &mut Vec<u8>, source: u16, destination: u16, count: u16) {
    loader.push(0x8e); // LDX #source
    loader.extend(source.to_be_bytes());
    loader.push(0xce); // LDU #destination
    loader.extend(destination.to_be_bytes());
    loader.extend([0x10, 0x8e]); // LDY #count
    loader.extend(count.to_be_bytes());
    loader.extend([0xa6, 0x80, 0xa7, 0xc0, 0x31, 0x3f, 0x26, 0xf8]);
    // LDA ,X+; STA ,U+; LEAY -1,Y; BNE loop
}

#[cfg(test)]
mod tests {
    use super::*;
    use eightbit_dragon_basic_core::{analyse, tokenise};
    fn tokens(source: &str) -> Vec<u8> {
        let dialect = Dialect::dragon32().unwrap();
        tokenise(&analyse(source, &dialect).program, &dialect)
            .unwrap()
            .bytes
    }

    #[test]
    fn packages_original_payloads_deterministically_with_padding() {
        let basic = tokens("10 EXEC &H7000\n20 END\n");
        let code = [0x86, 42, 0xb7, 6, 0, 0x39];
        let make = || {
            create_program_cartridge(
                &basic,
                Some(CartridgeAssembly {
                    load_address: 0x7000,
                    bytes: &code,
                }),
            )
            .unwrap()
        };
        let rom = make();
        assert_eq!(rom, make());
        assert_eq!(rom.len(), 8192);
        assert_eq!(&rom[128..128 + basic.len()], basic);
        assert_eq!(
            &rom[128 + basic.len()..128 + basic.len() + code.len()],
            code
        );
        assert!(
            rom[128 + basic.len() + code.len()..]
                .iter()
                .all(|byte| *byte == 0xff)
        );
        validate_cartridge(&rom).unwrap();
    }

    #[test]
    fn enforces_rom_capacity_and_ram_workspace_boundaries() {
        let basic = tokens("10 END\n");
        let large = vec![0x12; 8200];
        let rom = create_program_cartridge(
            &basic,
            Some(CartridgeAssembly {
                load_address: 0x4000,
                bytes: &large,
            }),
        )
        .unwrap();
        assert_eq!(rom.len(), 16384);
        assert!(rom[VISIBLE_BYTES..].iter().all(|b| *b == 0xff));
        for (address, size) in [
            (0x1e01, 10),
            (0x1f00, 10),
            (0x7000, 5000),
            (0x7000, 0),
            (0x4000, 16384),
        ] {
            assert!(
                create_program_cartridge(
                    &basic,
                    Some(CartridgeAssembly {
                        load_address: address,
                        bytes: &vec![0x12; size]
                    })
                )
                .is_err()
            );
        }
        assert!(create_program_cartridge(&vec![0; 16384], None).is_err());
        assert!(create_program_cartridge(&[0, 0], None).is_err());
        assert!(create_program_cartridge(&[0xff; 20], None).is_err());
        for size in [0, 1, 2047, 2049, 8193, 16385, 32768] {
            assert!(validate_cartridge(&vec![0; size]).is_err());
        }
        for size in [2048, 4096, 8192, 16384] {
            validate_cartridge(&vec![0; size]).unwrap();
        }
    }
}
