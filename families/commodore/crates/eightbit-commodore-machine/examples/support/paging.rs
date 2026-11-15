//! Original/generated page-boundary fixture. No third-party firmware is involved.
use eightbit_commodore_machine::{
    Error, Firmware, Memory,
    original::{Gate, GateKind, HEADER, MAGIC},
};
use eightbit_mos6502_core::{Bus, Cpu, Registers};
use sha2::{Digest, Sha256};

pub fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
pub fn pack(
    pages: &[u8],
    kernal: &[u8],
    characters: &[u8],
    gates: &[Gate],
    public: &[(u16, u8)],
) -> Vec<u8> {
    let count = pages.len() / 8192;
    let mut metadata = vec![0; HEADER];
    metadata[..8].copy_from_slice(MAGIC);
    metadata[8..10].copy_from_slice(&1u16.to_le_bytes());
    metadata[10] = u8::try_from(count).unwrap_or(255);
    metadata[11] = u8::try_from(gates.len()).unwrap_or(255);
    metadata[12..14].copy_from_slice(&u16::try_from(public.len()).unwrap_or(65535).to_le_bytes());
    metadata[16..48].copy_from_slice(&hash(b"Original generated page fixture source v1"));
    metadata[48..80].copy_from_slice(&hash(b"Original generated fixture packer v1"));
    for page in pages.chunks(8192).chain([kernal, characters]) {
        metadata.extend(hash(page));
    }
    let map_start = metadata.len();
    for g in gates {
        metadata.extend(g.entry.to_le_bytes());
        metadata.extend(g.target.to_le_bytes());
        metadata.extend([
            g.page,
            match g.kind {
                GateKind::Call => 0,
                GateKind::Interrupt => 1,
                GateKind::Recovery => 2,
            },
            0,
            0,
        ]);
    }
    for (address, length) in public {
        metadata.extend(address.to_le_bytes());
        metadata.extend([*length, 0]);
    }
    let map_hash = hash(&metadata[map_start..]);
    metadata[80..112].copy_from_slice(&map_hash);
    let payload: Vec<u8> = pages
        .iter()
        .chain(kernal)
        .chain(characters)
        .copied()
        .collect();
    metadata[112..144].copy_from_slice(&hash(&payload));
    let length = u32::try_from(metadata.len()).unwrap_or(0).to_le_bytes();
    metadata[144..148].copy_from_slice(&length);
    metadata[148..152].copy_from_slice(&u32::try_from(payload.len()).unwrap_or(0).to_le_bytes());
    metadata.extend(payload);
    metadata
}
pub fn parts() -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<Gate>) {
    let mut pages = vec![0xea; 3 * 8192];
    for (index, page) in pages.chunks_exact_mut(8192).enumerate() {
        page[..4].copy_from_slice(&[0, 0xa4, 0, 0xa4]);
        page[0x10..0x13].copy_from_slice(&[0x4c, 0, 0xe1]);
        page[0x1fff] = u8::try_from(index).unwrap_or(0);
    }
    pages[8192 + 0x100..8192 + 0x109]
        .copy_from_slice(&[0xa9, 0x42, 0x8d, 0, 0xc0, 0x20, 0x80, 0xe1, 0x60]);
    pages[16384 + 0x200..16384 + 0x206].copy_from_slice(&[0xa9, 0x43, 0x8d, 1, 0xc0, 0x60]);
    pages[0x400..0x403].copy_from_slice(&[0x4c, 0, 0xa4]);
    let gates = vec![
        Gate {
            entry: 0xe100,
            target: 0xa100,
            page: 1,
            kind: GateKind::Call,
        },
        Gate {
            entry: 0xe180,
            target: 0xa200,
            page: 2,
            kind: GateKind::Call,
        },
        Gate {
            entry: 0xe300,
            target: 0xe500,
            page: 0,
            kind: GateKind::Interrupt,
        },
        Gate {
            entry: 0xe340,
            target: 0xe520,
            page: 0,
            kind: GateKind::Interrupt,
        },
        Gate {
            entry: 0xe400,
            target: 0xa400,
            page: 0,
            kind: GateKind::Recovery,
        },
    ];
    let mut kernal = vec![0xea; 8192];
    for g in &gates {
        let start = usize::from(g.entry - 0xe000);
        let code = g.code();
        kernal[start..start + code.len()].copy_from_slice(&code);
    }
    // Inert caller loop after a normal nested call; reset initialises real port pins.
    kernal[..18].copy_from_slice(&[
        0xa9, 7, 0x85, 1, 0x85, 0, 0xa2, 0xf8, 0x9a, 0x20, 0, 0xe1, 0x4c, 12, 0xe0, 0xea, 0xea,
        0xea,
    ]);
    kernal[0x500..0x504].copy_from_slice(&[0xee, 2, 0xc0, 0x60]);
    kernal[0x520..0x524].copy_from_slice(&[0xee, 3, 0xc0, 0x60]);
    kernal[0x1ffa..].copy_from_slice(&[0x40, 0xe3, 0, 0xe0, 0, 0xe3]);
    (pages, kernal, vec![0x5a; 4096], gates)
}
pub fn fixture() -> Vec<u8> {
    let (p, k, c, g) = parts();
    pack(&p, &k, &c, &g, &[(0xa000, 4), (0xa010, 3)])
}
pub fn memory(bytes: &[u8]) -> Result<Memory<'_>, Error> {
    let image = eightbit_commodore_machine::original::Image::parse(bytes, &hash(bytes))?;
    let mut m = Memory::new(Firmware::original(image)?);
    m.write(0, 7)?;
    m.write(1, 7)?;
    Ok(m)
}
pub fn trace() -> Result<String, Box<dyn std::error::Error>> {
    let bytes = fixture();
    let mut m = memory(&bytes)?;
    let mut cpu = Cpu::from_registers(Registers {
        pc: 0xe009,
        sp: 0xf8,
        y: 0x77,
        ..Registers::default()
    });
    let mut output = String::new();
    use std::fmt::Write;
    for index in 0..100 {
        if index == 12 {
            cpu.set_nmi(true);
        }
        if index == 13 {
            cpu.set_nmi(false);
        }
        cpu.step(&mut m)?;
        let r = cpu.registers();
        writeln!(
            output,
            "{:04x}:{:02x}:{:02x}:{:02x}:{:02x}:{:?}",
            r.pc,
            r.a,
            r.y,
            r.p,
            r.sp,
            m.original_page()
        )?;
    }
    if m.original_page() != Some((0, 0))
        || m.ram(0xc000) != 0x42
        || m.ram(0xc001) != 0x43
        || m.ram(0xc003) != 1
    {
        return Err("Generated nested/NMI page trace failed".into());
    }
    Ok(output)
}
