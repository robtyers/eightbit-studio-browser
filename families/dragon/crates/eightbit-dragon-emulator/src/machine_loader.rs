//! Original guest bootstrap for binaries which replace BASIC's live stack.
//!
//! This code is assembled by the shared original CPU assembler and delivered as
//! the first physical cassette file. It calls only published firmware entries.
//! Once entered, it owns the initial machine-code stack below $0300 and never
//! returns through the BASIC frames which the selected payload may overwrite.

use eightbit_dragon_media::MachineCodeCassette;

pub(crate) const ADDRESS: u16 = 0x0300;
const END_EXCLUSIVE: u16 = 0x03d9;

pub(crate) struct MachineLoader {
    pub(crate) bytes: Vec<u8>,
    pub(crate) ready: u16,
    pub(crate) failed: u16,
    pub(crate) transport: Vec<u8>,
}
impl MachineLoader {
    #[cfg(test)]
    pub(crate) fn new(binary: &MachineCodeCassette, cassette: &[u8]) -> Result<Self, String> {
        Self::for_native(binary, cassette, false)
    }
    pub(crate) fn for_native(
        binary: &MachineCodeCassette,
        cassette: &[u8],
        native: bool,
    ) -> Result<Self, String> {
        let end = usize::from(binary.load_address) + binary.program.len();
        // $8021 is cassette sync, $B93E block input and $8018 motor off.
        // The public block workspace is $7C/$7D/$7E; no private ROM labels or
        // implementation offsets appear in the bootstrap. CURLINE=0 suppresses
        // the direct-mode screen marker which would alter payload byte $0400.
        let source = format!(
            "entry:\n\
             ORCC #$50\n\
             CLRA\n\
             TFR A,DP\n\
             LDS #$0300\n\
             CLRB\n\
             STD <$68\n\
             LDD #$01DA\n\
             STD <$7E\n\
             JSR $8021\n\
             BCS fail\n\
             JSR $B93E\n\
             BCS fail\n\
             LDA <$7C\n\
             BNE fail\n\
             LDA <$7D\n\
             CMPA #15\n\
             BNE fail\n\
             LDA $01E2\n\
             CMPA #2\n\
             BNE fail\n\
             LDD $01E5\n\
             CMPD #${entry:04X}\n\
             BNE fail\n\
             LDD $01E7\n\
             CMPD #${start:04X}\n\
             BNE fail\n\
             STD <$7E\n\
             next:\n\
             JSR $B93E\n\
             BCS fail\n\
             LDA <$7C\n\
             CMPA #1\n\
             BEQ data\n\
             CMPA #255\n\
             BNE fail\n\
             TST <$7D\n\
             BNE fail\n\
             LDD <$7E\n\
             CMPD #${end:04X}\n\
             BNE fail\n\
             LDD #${entry:04X}\n\
             STD <$9D\n\
             LDD #$FFFF\n\
             STD <$68\n\
             JSR $8018\n\
             ready:\n\
             JMP ${entry:04X}\n\
             data:\n\
             CLRA\n\
             LDB <$7D\n\
             ADDD <$7E\n\
             CMPD #${end:04X}\n\
             BHI fail\n\
             STD <$7E\n\
             BRA next\n\
             fail:\n\
             JSR $8018\n\
             failed:\n\
             BRA failed\n",
            entry = binary.execution_address,
            start = binary.load_address,
        );
        let source = if native {
            source
                .replace("$8021", "$C021")
                .replace("$B93E", "$F93E")
                .replace("$8018", "$C018")
        } else {
            source
        };
        let assembly = eightbit_m6809_core::assembler::assemble(&source, ADDRESS, "entry")
            .map_err(|error| format!("DME1502: internal guest loader assembly failed: {error}"))?;
        let limit = usize::from(END_EXCLUSIVE);
        if assembly.load_address != ADDRESS
            || assembly.entry_address != ADDRESS
            || usize::from(ADDRESS) + assembly.bytes.len() > limit
            || assembly.lines.iter().any(|line| {
                line.address < ADDRESS
                    || usize::from(line.address) + usize::from(line.bytes) > limit
            })
        {
            return Err("DME1502: internal guest loader exceeds its workspace".into());
        }
        let symbol = |name| {
            assembly
                .symbols
                .get(name)
                .copied()
                .filter(|address| {
                    *address >= ADDRESS
                        && usize::from(*address) < usize::from(ADDRESS) + assembly.bytes.len()
                })
                .ok_or_else(|| "DME1502: missing internal guest loader boundary".to_string())
        };
        let ready = symbol("READY")?;
        let failed = symbol("FAILED")?;
        let mut transport =
            eightbit_dragon_media::create_machine_code("STUDIO", &assembly.bytes, ADDRESS, ADDRESS)
                .map_err(|error| {
                    format!("DME1502: internal guest loader cassette failed: {error}")
                })?;
        transport.extend_from_slice(cassette);
        Ok(Self {
            bytes: assembly.bytes,
            ready,
            failed,
            transport,
        })
    }
}
