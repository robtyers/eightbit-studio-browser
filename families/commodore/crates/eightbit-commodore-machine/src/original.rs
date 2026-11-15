//! Original, strictly bounded firmware container and guest-controlled page state.
//! No filesystem, executable service or BASIC value is represented here.
use crate::Error;
use eightbit_mos6502_core::{Execution, Registers, StepKind};
use sha2::{Digest, Sha256};

/// Original container schema-1 magic, distinct from legacy raw firmware windows.
pub const MAGIC: &[u8; 8] = b"EBC64R01";
/// Maximum BASIC page count.
pub const MAX_PAGES: usize = 16;
/// Maximum fixed gate sites.
pub const MAX_GATES: usize = 128;
/// Maximum private page-continuation records; no guest values are stored here.
pub const MAX_DEPTH: usize = 32;
/// Maximum complete container length, including 16 KiB metadata.
pub const MAX_IMAGE: usize = 159_744;
/// Fixed schema-1 header length.
pub const HEADER: usize = 160;

/// Validated original image, borrowed without retaining a second copy.
/// A caller must supply its trusted, build-bound complete-container SHA-256.
#[derive(Clone, Copy)]
pub struct Image<'a> {
    pub(crate) pages: &'a [u8],
    pub(crate) kernal: &'a [u8; 8192],
    pub(crate) characters: &'a [u8; 4096],
    gates: &'a [u8],
    public: &'a [u8],
}
/// An original guest gate's transfer convention.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GateKind {
    /// JSR/RTS page call preserving A, X, Y and flags around page writes.
    Call,
    /// Hardware/BRK entry, fixed handler and RTI exit.
    Interrupt,
    /// Non-local error/warm recovery, revoking every abandoned continuation.
    Recovery,
}
/// Fixed gate metadata. Its complete instruction template must match immutable ROM.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Gate {
    /// KERNAL entry, below the fixed public jump table.
    pub entry: u16,
    /// Guest subroutine or recovery target.
    pub target: u16,
    /// BASIC page selected by this immutable gate.
    pub page: u8,
    /// Transfer convention.
    pub kind: GateKind,
}
impl Gate {
    /// Independently authored, ordinary 6510 instructions; no host opcode extension.
    #[must_use]
    pub fn code(self) -> Vec<u8> {
        let [lo, hi] = self.target.to_le_bytes();
        if self.kind == GateKind::Recovery {
            vec![0x08, 0x48, 0xa9, 0, 0x8d, 2, 0xde, 0x68, 0x28, 0x4c, lo, hi]
        } else {
            vec![
                0x08,
                0x48,
                0xa9,
                self.page,
                0x8d,
                0,
                0xde,
                0x68,
                0x28,
                0x20,
                lo,
                hi,
                0x08,
                0x48,
                0xa9,
                0,
                0x8d,
                1,
                0xde,
                0x68,
                0x28,
                if self.kind == GateKind::Call {
                    0x60
                } else {
                    0x40
                },
            ]
        }
    }
    const fn len(self) -> u16 {
        if matches!(self.kind, GateKind::Recovery) {
            12
        } else {
            22
        }
    }
}
fn word(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}
fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn gate(bytes: &[u8]) -> Gate {
    Gate {
        entry: word(bytes, 0),
        target: word(bytes, 2),
        page: bytes[4],
        kind: match bytes[5] {
            0 => GateKind::Call,
            1 => GateKind::Interrupt,
            _ => GateKind::Recovery,
        },
    }
}
impl<'a> Image<'a> {
    /// Validate exact schema, all bounds/images/maps/templates and trusted identity.
    pub fn parse(bytes: &'a [u8], expected: &[u8; 32]) -> Result<Self, Error> {
        if !(HEADER..=MAX_IMAGE).contains(&bytes.len()) {
            return Err(Error::OriginalFormat);
        }
        if &bytes[..8] != MAGIC
            || word(bytes, 8) != 1
            || bytes[14..16] != [0; 2]
            || bytes[152..160] != [0; 8]
        {
            return Err(Error::OriginalFormat);
        }
        let pages = usize::from(bytes[10]);
        let count = usize::from(bytes[11]);
        let public_count = usize::from(word(bytes, 12));
        if !(1..=MAX_PAGES).contains(&pages) || count > MAX_GATES || public_count > 128 {
            return Err(Error::OriginalFormat);
        }
        let metadata = HEADER + (pages + 2) * 32 + count * 8 + public_count * 4;
        let payload = pages * 8192 + 8192 + 4096;
        let declared_metadata = u32::from_le_bytes(
            bytes[144..148]
                .try_into()
                .map_err(|_| Error::OriginalFormat)?,
        );
        let declared_payload = u32::from_le_bytes(
            bytes[148..152]
                .try_into()
                .map_err(|_| Error::OriginalFormat)?,
        );
        if metadata > 16_384
            || usize::try_from(declared_metadata).ok() != Some(metadata)
            || usize::try_from(declared_payload).ok() != Some(payload)
            || metadata + payload != bytes.len()
        {
            return Err(Error::OriginalFormat);
        }
        if &hash(bytes) != expected
            || bytes[16..48] == [0; 32]
            || bytes[48..80] == [0; 32]
            || hash(&bytes[metadata..]).as_slice() != &bytes[112..144]
        {
            return Err(Error::OriginalIdentity);
        }
        let gates_start = HEADER + (pages + 2) * 32;
        let public_start = gates_start + count * 8;
        if hash(&bytes[gates_start..metadata]).as_slice() != &bytes[80..112] {
            return Err(Error::OriginalIdentity);
        }
        let mut offset = metadata;
        for index in 0..pages + 2 {
            let length = if index == pages + 1 { 4096 } else { 8192 };
            if hash(&bytes[offset..offset + length]).as_slice()
                != &bytes[HEADER + index * 32..HEADER + (index + 1) * 32]
            {
                return Err(Error::OriginalIdentity);
            }
            offset += length;
        }
        let image = Self {
            pages: &bytes[metadata..metadata + pages * 8192],
            kernal: bytes[metadata + pages * 8192..metadata + pages * 8192 + 8192]
                .try_into()
                .map_err(|_| Error::OriginalFormat)?,
            characters: bytes[metadata + pages * 8192 + 8192..]
                .try_into()
                .map_err(|_| Error::OriginalFormat)?,
            gates: &bytes[gates_start..public_start],
            public: &bytes[public_start..metadata],
        };
        for (index, raw) in image.gates.chunks_exact(8).enumerate() {
            let g = gate(raw);
            if raw[5] > 2
                || raw[6..8] != [0; 2]
                || usize::from(g.page) >= pages
                || !(0xe000..=0xff81 - g.len()).contains(&g.entry)
                || !((0xa000..=0xbfff).contains(&g.target) || (0xe000..0xff81).contains(&g.target))
                || (g.kind != GateKind::Call && g.page != 0)
                || (g.kind == GateKind::Interrupt && g.target < 0xe000)
            {
                return Err(Error::OriginalFormat);
            }
            let start = usize::from(g.entry - 0xe000);
            if image.kernal[start..start + usize::from(g.len())] != g.code() {
                return Err(Error::OriginalGate);
            }
            for earlier in image.gates[..index * 8].chunks_exact(8).map(gate) {
                if g.entry < earlier.entry + earlier.len() {
                    return Err(Error::OriginalFormat);
                }
            }
            if image
                .gates()
                .any(|other| g.target > other.entry && g.target < other.entry + other.len())
            {
                return Err(Error::OriginalFormat);
            }
        }
        for (index, raw) in image.public.chunks_exact(4).enumerate() {
            let address = word(raw, 0);
            let length = u16::from(raw[2]);
            if !(1..=16).contains(&length)
                || raw[3] != 0
                || !(0xa000..=0xc000 - length).contains(&address)
            {
                return Err(Error::OriginalFormat);
            }
            for earlier in image.public[..index * 4].chunks_exact(4) {
                if address < word(earlier, 0) + u16::from(earlier[2]) {
                    return Err(Error::OriginalFormat);
                }
            }
            let start = usize::from(address - 0xa000);
            let end = start + usize::from(length);
            for page in image.pages.chunks_exact(8192) {
                if page[start..end] != image.pages[start..end] {
                    return Err(Error::OriginalGate);
                }
            }
        }
        if !image
            .public
            .chunks_exact(4)
            .any(|p| word(p, 0) == 0xa000 && p[2] >= 4)
        {
            return Err(Error::OriginalFormat);
        }
        Ok(image)
    }
    /// Number of validated 8 KiB BASIC pages.
    #[must_use]
    pub fn page_count(self) -> usize {
        self.pages.len() / 8192
    }
    fn gates(self) -> impl Iterator<Item = Gate> + 'a {
        self.gates.chunks_exact(8).map(gate)
    }
    fn covering(self, pc: u16) -> Option<Gate> {
        let (mut low, mut high) = (0, self.gates.len() / 8);
        while low < high {
            let mid = usize::midpoint(low, high);
            if word(self.gates, mid * 8) <= pc {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        if low == 0 {
            return None;
        }
        let g = gate(&self.gates[(low - 1) * 8..low * 8]);
        (pc < g.entry + g.len()).then_some(g)
    }
    fn public_jump(self, pc: u16, target: u16, page: u8) -> bool {
        self.public.chunks_exact(4).any(|entry| {
            if word(entry, 0) != pc || entry[2] != 3 {
                return false;
            }
            let start = usize::from(page) * 8192 + usize::from(pc - 0xa000);
            self.pages[start] == 0x4c && word(self.pages, start + 1) == target
        })
    }
}

#[derive(Clone, Copy)]
struct Frame {
    gate: Gate,
    sp: u8,
    previous: u8,
    phase: u16,
    ready: bool,
    return_pc: u16,
    return_sp: u8,
}
pub(crate) struct Paging<'a> {
    image: Image<'a>,
    page: u8,
    frames: Vec<Frame>,
    origin: Option<Registers>,
    active: Option<Gate>,
}
impl<'a> Paging<'a> {
    pub fn new(image: Image<'a>) -> Self {
        Self {
            image,
            page: 0,
            frames: Vec::with_capacity(MAX_DEPTH),
            origin: None,
            active: None,
        }
    }
    pub fn revoke(&mut self) {
        self.frames.clear();
        self.page = 0;
        self.origin = None;
        self.active = None;
    }
    pub fn page(&self) -> u8 {
        self.page
    }
    pub fn depth(&self) -> usize {
        self.frames.len()
    }
    pub fn read(&self, address: u16) -> u8 {
        self.image.pages[usize::from(self.page) * 8192 + usize::from(address - 0xa000)]
    }
    pub fn event(&mut self, event: Execution, pins: u8, stack: [u8; 3]) -> Result<(), Error> {
        match event {
            Execution::Reset | Execution::Fault => self.revoke(),
            Execution::Interrupt => {
                self.origin = None;
                self.active = None;
            }
            Execution::Instruction(r) => {
                self.origin = Some(r);
                self.active = None;
                if pins & 2 == 0 {
                    return Ok(());
                }
                if let Some(g) = self.image.covering(r.pc) {
                    let f = self.frames.last().ok_or(Error::OriginalGate)?;
                    if f.gate != g
                        || r.pc != g.entry + f.phase
                        || (f.phase == 12 && !f.ready)
                        || (matches!(f.phase, 4 | 16) && pins & 7 != 7)
                    {
                        return Err(Error::OriginalGate);
                    }
                    self.active = Some(g);
                }
            }
            Execution::Complete { registers, kind } => {
                self.complete(registers, kind, pins, stack)?;
            }
        }
        Ok(())
    }
    fn complete(
        &mut self,
        r: Registers,
        kind: StepKind,
        pins: u8,
        stack: [u8; 3],
    ) -> Result<(), Error> {
        let before = self.origin.take();
        if let Some(g) = self.active.take() {
            let f = self.frames.last_mut().ok_or(Error::OriginalGate)?;
            match f.phase {
                0 | 1 | 7 | 8 | 12 | 13 | 19 | 20 => f.phase += 1,
                2 | 14 => f.phase += 2,
                4 | 16 => f.phase += 3,
                9 if g.kind != GateKind::Recovery => {
                    f.phase = 12;
                    f.ready = false;
                }
                9 => {
                    self.frames.pop();
                }
                21 => {
                    if r.pc != f.return_pc || r.sp != f.return_sp {
                        return Err(Error::OriginalGate);
                    }
                    self.frames.pop();
                }
                _ => return Err(Error::OriginalGate),
            }
        }
        if let Some(f) = self.frames.last_mut()
            && f.phase == 12
            && !f.ready
            && r.pc == f.gate.entry + 12
        {
            if r.sp != f.sp
                || !(kind == StepKind::Instruction(0x60)
                    || (f.gate.kind == GateKind::Interrupt && kind == StepKind::Instruction(0x40)))
            {
                return Err(Error::OriginalGate);
            }
            f.ready = true;
        }
        if pins & 2 == 0 {
            return Ok(());
        }
        if let Some(g) = self.image.covering(r.pc).filter(|g| g.entry == r.pc) {
            // RTI can resume the first instruction of an already admitted gate.
            // Its continuation survived the interrupt; do not admit it twice.
            if kind == StepKind::Instruction(0x40)
                && self
                    .frames
                    .last()
                    .is_some_and(|f| f.gate == g && f.phase == 0 && f.sp == r.sp)
            {
                return Ok(());
            }
            let allowed = match g.kind {
                GateKind::Call => {
                    kind == StepKind::Instruction(0x20)
                        || (kind == StepKind::Instruction(0x4c)
                            && before.is_some_and(|b| {
                                pins & 3 == 3 && self.image.public_jump(b.pc, g.entry, self.page)
                            }))
                }
                GateKind::Interrupt => matches!(
                    kind,
                    StepKind::Irq | StepKind::Nmi | StepKind::Instruction(0)
                ),
                GateKind::Recovery => matches!(kind, StepKind::Instruction(0x20 | 0x4c | 0x6c)),
            };
            if !allowed {
                return Err(Error::OriginalGate);
            }
            if self.frames.len() == MAX_DEPTH || r.sp < 8 {
                return Err(Error::OriginalDepth);
            }
            let (return_pc, return_sp) = match g.kind {
                GateKind::Call => (
                    u16::from_le_bytes([stack[0], stack[1]]).wrapping_add(1),
                    r.sp.checked_add(2).ok_or(Error::OriginalDepth)?,
                ),
                GateKind::Interrupt => (
                    u16::from_le_bytes([stack[1], stack[2]]),
                    r.sp.checked_add(3).ok_or(Error::OriginalDepth)?,
                ),
                GateKind::Recovery => (
                    0,
                    if kind == StepKind::Instruction(0x20) {
                        r.sp.checked_add(2).ok_or(Error::OriginalDepth)?
                    } else {
                        r.sp
                    },
                ),
            };
            self.frames.push(Frame {
                gate: g,
                sp: r.sp,
                previous: self.page,
                phase: 0,
                ready: false,
                return_pc,
                return_sp,
            });
        }
        Ok(())
    }
    pub fn control(&mut self, address: u16, value: u8, pins: u8) -> Result<bool, Error> {
        let Some(g) = self.active else {
            return Ok(false);
        };
        let r = self.origin.ok_or(Error::OriginalGate)?;
        let f = *self.frames.last().ok_or(Error::OriginalGate)?;
        if pins & 7 != 7 || r.sp != f.sp - 2 {
            return Err(Error::OriginalGate);
        }
        match (address, f.phase, g.kind) {
            (0xde00, 4, GateKind::Call | GateKind::Interrupt) if value == g.page => {
                self.page = g.page;
            }
            (0xde01, 16, GateKind::Call | GateKind::Interrupt) if value == 0 => {
                self.page = f.previous;
            }
            (0xde02, 4, GateKind::Recovery) if value == 0 => {
                self.revoke();
                self.frames.push(Frame { previous: 0, ..f });
                self.origin = Some(r);
                self.active = Some(g);
            }
            _ => return Err(Error::OriginalGate),
        }
        Ok(true)
    }
}
