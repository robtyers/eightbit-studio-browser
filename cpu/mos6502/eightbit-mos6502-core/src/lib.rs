//! Original NMOS 6502 instruction engine with a fallible, cycle-observable bus.
//!
//! Documented opcodes only. Optional bus interrupt sampling observes device edges
//! during instructions. The board owns bounded DMA/RDY waits inside bus accesses.
//! No host I/O, firmware, assembler, family or application dependency is present.

mod instructions;
use instructions::{Mode, Op, decode};

/// Carry flag.
pub const C: u8 = 0x01;
/// Zero flag.
pub const Z: u8 = 0x02;
/// Interrupt disable flag.
pub const I: u8 = 0x04;
/// Decimal mode flag.
pub const D: u8 = 0x08;
/// Stack-only software break marker; not a physical status latch.
pub const B: u8 = 0x10;
/// Unused status bit, always presented as one.
pub const U: u8 = 0x20;
/// Signed overflow flag.
pub const V: u8 = 0x40;
/// Negative flag.
pub const N: u8 = 0x80;
/// Maximum instructions/interrupt entries accepted by one bounded run.
pub const MAX_STEPS: u32 = 1_000_000;

/// Each call represents one CPU bus cycle, including dummy reads and old-value writes.
/// A failing access may have side effects; the CPU then requires an explicit reset.
pub trait Bus {
    /// Board-owned access failure.
    type Error;
    /// Read a bus address, permitting device side effects.
    fn read(&mut self, address: u16) -> Result<u8, Self::Error>;
    /// Write a bus address, permitting device side effects.
    fn write(&mut self, address: u16, value: u8) -> Result<(), Self::Error>;
    /// Optional device lines after each access. Default preserves caller-driven lines.
    /// Boards retain NMI pulses occurring during DMA as `nmi_edge` until sampled.
    fn interrupt_lines(&mut self) -> Option<InterruptLines> {
        None
    }
    /// Optional zero-cycle execution provenance for bounded board-owned ROM gates.
    /// This cannot replace an instruction or alter CPU registers. Legacy buses ignore it.
    fn execution(&mut self, _event: Execution) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Zero-cycle observations; every bus cycle still uses `read`/`write` normally.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Execution {
    /// Revoke board-owned execution context before the first reset cycle.
    Reset,
    /// A real opcode is about to be fetched at these registers.
    Instruction(Registers),
    /// A hardware interrupt is about to perform its normal bus sequence.
    Interrupt,
    /// A completed instruction/interrupt, with its resulting visible registers.
    Complete {
        /// Registers after the completed step, without host modification.
        registers: Registers,
        /// The opcode or hardware interrupt actually executed.
        kind: StepKind,
    },
    /// An instruction, bus or execution-observer fault requires reset.
    Fault,
}

/// Logical asserted device signals (physical IRQ/NMI pins are active low).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InterruptLines {
    /// Level-sensitive IRQ.
    pub irq: bool,
    /// Current NMI level.
    pub nmi: bool,
    /// A board-latched NMI assertion since the previous sample.
    pub nmi_edge: bool,
}

/// Architecturally visible registers. Status is normalised by `Cpu::from_registers`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Registers {
    /// Accumulator.
    pub a: u8,
    /// X index.
    pub x: u8,
    /// Y index.
    pub y: u8,
    /// Stack offset in page one.
    pub sp: u8,
    /// Status, with U set and B clear.
    pub p: u8,
    /// Next instruction address.
    pub pc: u16,
}

impl Default for Registers {
    fn default() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            sp: 0,
            p: I | U,
            pc: 0,
        }
    }
}

/// Stable execution failures; no unsupported opcode is silently treated as NOP.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error<E> {
    /// An attempted bus access failed; reset is required.
    Bus(E),
    /// Unsupported opcode at its original fetch address; reset is required.
    UnsupportedOpcode {
        /// Fetch address.
        address: u16,
        /// Fetched byte.
        opcode: u8,
    },
    /// Execution after an earlier fault is rejected without bus activity.
    NeedsReset,
    /// Run budget must lie in `1..=MAX_STEPS`.
    InvalidBudget,
    /// Internal decoder/executor disagreement. Stops rather than inventing a result.
    DecodeInvariant,
}

impl<E: core::fmt::Display> core::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Bus(error) => write!(f, "D650201: bus access failed: {error}"),
            Self::UnsupportedOpcode { address, opcode } => write!(
                f,
                "D650202: unsupported NMOS opcode ${opcode:02X} at ${address:04X}"
            ),
            Self::NeedsReset => f.write_str("D650203: CPU faulted; reset required"),
            Self::InvalidBudget => f.write_str("D650204: instruction budget must be 1–1000000"),
            Self::DecodeInvariant => f.write_str("D650205: internal instruction decode mismatch"),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for Error<E> {}

/// What a completed step executed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StepKind {
    /// One documented opcode (BRK is included here).
    Instruction(u8),
    /// Maskable hardware interrupt entry.
    Irq,
    /// Latched non-maskable interrupt entry.
    Nmi,
}
/// One completed instruction or interrupt entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Step {
    /// Entry type.
    pub kind: StepKind,
    /// Bus cycles, including dummy accesses.
    pub cycles: u8,
}
/// Why a bounded run returned normally.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StopReason {
    /// Caller-supplied cancellation was observed between steps.
    Cancelled,
    /// The exact instruction/interrupt budget was exhausted.
    Budget,
}
/// Bounded execution receipt. Interrupt entries count as steps.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Run {
    /// Completed steps.
    pub steps: u32,
    /// Bus cycles in completed steps.
    pub cycles: u64,
    /// Normal stop reason.
    pub reason: StopReason,
}

#[derive(Clone, Copy, Debug, Default)]
enum ExecutionState {
    #[default]
    Ready,
    Faulted,
}

/// Instruction-boundary NMOS CPU. Construction does not read the bus or boot firmware.
#[derive(Clone, Debug, Default)]
pub struct Cpu {
    r: Registers,
    irq: bool,
    nmi: bool,
    nmi_pending: bool,
    delayed_i: Option<bool>,
    polled_irq: Option<bool>,
    state: ExecutionState,
    ticks: u8,
}

impl Cpu {
    /// Construct inert state; use `reset` for the reset bus sequence.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct explicit test/debug state, without an implicit reset or bus access.
    #[must_use]
    pub fn from_registers(mut registers: Registers) -> Self {
        registers.p = (registers.p | U) & !B;
        Self {
            r: registers,
            ..Self::default()
        }
    }

    /// Read visible state without touching the bus.
    #[must_use]
    pub const fn registers(&self) -> Registers {
        self.r
    }

    /// Whether execution requires reset after an access or opcode fault.
    #[must_use]
    pub const fn is_faulted(&self) -> bool {
        matches!(self.state, ExecutionState::Faulted)
    }

    /// Set logical asserted IRQ level (not the physical active-low voltage).
    pub fn set_irq(&mut self, asserted: bool) {
        self.irq = asserted;
    }

    /// Set logical asserted NMI level. Only a new inactive-to-active transition latches.
    pub fn set_nmi(&mut self, asserted: bool) {
        self.nmi_pending |= asserted && !self.nmi;
        self.nmi = asserted;
    }

    /// Seven-cycle reset; stack decrements three times without writes. NMOS D is retained.
    /// A/X/Y and external line levels are retained; a successful reset clears fault/NMI.
    pub fn reset<T: Bus>(&mut self, bus: &mut T) -> Result<u8, Error<T::Error>> {
        self.ticks = 0;
        self.polled_irq = None;
        self.state = ExecutionState::Faulted;
        bus.execution(Execution::Reset).map_err(Error::Bus)?;
        self.read(bus, self.r.pc)?;
        self.read(bus, self.r.pc)?;
        for _ in 0..3 {
            self.read(bus, 0x100 | u16::from(self.r.sp))?;
            self.r.sp = self.r.sp.wrapping_sub(1);
        }
        self.r.pc = self.vector(bus, 0xfffc)?;
        self.r.p = (self.r.p | I | U) & !B;
        self.delayed_i = None;
        self.nmi_pending = false;
        self.state = ExecutionState::Ready;
        Ok(self.ticks)
    }

    /// Execute one instruction/interrupt. Faults stop at the failing access, without rollback.
    pub fn step<T: Bus>(&mut self, bus: &mut T) -> Result<Step, Error<T::Error>> {
        if self.is_faulted() {
            return Err(Error::NeedsReset);
        }
        self.ticks = 0;
        self.sample_lines(bus, false);
        let result = self.execute(bus).and_then(|kind| {
            bus.execution(Execution::Complete {
                registers: self.r,
                kind,
            })
            .map_err(Error::Bus)?;
            Ok(kind)
        });
        if result.is_err() {
            self.state = ExecutionState::Faulted;
            // Cleanup must not replace the original access/observer diagnostic.
            let _ = bus.execution(Execution::Fault);
        }
        result.map(|kind| Step {
            kind,
            cycles: self.ticks,
        })
    }

    /// Run a bounded count; cancellation is checked before each step and cannot split one.
    /// On a CPU/bus error the caller can inspect registers and reset explicitly.
    pub fn run<T: Bus>(
        &mut self,
        bus: &mut T,
        budget: u32,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<Run, Error<T::Error>> {
        if !(1..=MAX_STEPS).contains(&budget) {
            return Err(Error::InvalidBudget);
        }
        let mut run = Run {
            steps: 0,
            cycles: 0,
            reason: StopReason::Budget,
        };
        while run.steps < budget {
            if cancelled() {
                run.reason = StopReason::Cancelled;
                break;
            }
            run.cycles += u64::from(self.step(bus)?.cycles);
            run.steps += 1;
        }
        Ok(run)
    }

    fn read<T: Bus>(&mut self, bus: &mut T, address: u16) -> Result<u8, Error<T::Error>> {
        self.ticks += 1;
        let value = bus.read(address).map_err(Error::Bus)?;
        self.sample_lines(bus, true);
        Ok(value)
    }
    fn write<T: Bus>(
        &mut self,
        bus: &mut T,
        address: u16,
        value: u8,
    ) -> Result<(), Error<T::Error>> {
        self.ticks += 1;
        bus.write(address, value).map_err(Error::Bus)?;
        self.sample_lines(bus, true);
        Ok(())
    }
    fn sample_lines<T: Bus>(&mut self, bus: &mut T, after_access: bool) {
        if let Some(lines) = bus.interrupt_lines() {
            // The previous cycle supplies the normal penultimate-cycle IRQ poll.
            if after_access {
                self.polled_irq = Some(self.irq);
            }
            self.irq = lines.irq;
            self.set_nmi(lines.nmi);
            self.nmi_pending |= lines.nmi_edge;
        } else {
            self.polled_irq = None;
        }
    }
    fn fetch<T: Bus>(&mut self, bus: &mut T) -> Result<u8, Error<T::Error>> {
        let value = self.read(bus, self.r.pc)?;
        self.r.pc = self.r.pc.wrapping_add(1);
        Ok(value)
    }
    fn word<T: Bus>(&mut self, bus: &mut T) -> Result<u16, Error<T::Error>> {
        let low = self.fetch(bus)?;
        Ok(u16::from_le_bytes([low, self.fetch(bus)?]))
    }
    fn vector<T: Bus>(&mut self, bus: &mut T, address: u16) -> Result<u16, Error<T::Error>> {
        let low = self.read(bus, address)?;
        Ok(u16::from_le_bytes([
            low,
            self.read(bus, address.wrapping_add(1))?,
        ]))
    }
    fn push<T: Bus>(&mut self, bus: &mut T, value: u8) -> Result<(), Error<T::Error>> {
        self.write(bus, 0x100 | u16::from(self.r.sp), value)?;
        self.r.sp = self.r.sp.wrapping_sub(1);
        Ok(())
    }
    fn pull<T: Bus>(&mut self, bus: &mut T) -> Result<u8, Error<T::Error>> {
        self.r.sp = self.r.sp.wrapping_add(1);
        self.read(bus, 0x100 | u16::from(self.r.sp))
    }
    fn flag(&mut self, mask: u8, set: bool) {
        if set {
            self.r.p |= mask;
        } else {
            self.r.p &= !mask;
        }
    }
    fn nz(&mut self, value: u8) {
        self.flag(N, value & N != 0);
        self.flag(Z, value == 0);
    }
    fn compare(&mut self, left: u8, right: u8) {
        self.flag(C, left >= right);
        self.nz(left.wrapping_sub(right));
    }

    fn address<T: Bus>(
        &mut self,
        bus: &mut T,
        mode: Mode,
        always_dummy: bool,
    ) -> Result<u16, Error<T::Error>> {
        match mode {
            Mode::Imm => {
                let address = self.r.pc;
                self.r.pc = self.r.pc.wrapping_add(1);
                Ok(address)
            }
            Mode::Zp => Ok(u16::from(self.fetch(bus)?)),
            Mode::Zpx | Mode::Zpy => {
                let base = self.fetch(bus)?;
                self.read(bus, u16::from(base))?;
                let index = if mode == Mode::Zpx {
                    self.r.x
                } else {
                    self.r.y
                };
                Ok(u16::from(base.wrapping_add(index)))
            }
            Mode::Abs => self.word(bus),
            Mode::Abx | Mode::Aby | Mode::Izy => {
                let base = if mode == Mode::Izy {
                    let pointer = self.fetch(bus)?;
                    let low = self.read(bus, u16::from(pointer))?;
                    u16::from_le_bytes([low, self.read(bus, u16::from(pointer.wrapping_add(1)))?])
                } else {
                    self.word(bus)?
                };
                let index = if mode == Mode::Abx {
                    self.r.x
                } else {
                    self.r.y
                };
                let address = base.wrapping_add(u16::from(index));
                if always_dummy || base & 0xff00 != address & 0xff00 {
                    self.read(bus, (base & 0xff00) | (address & 0xff))?;
                }
                Ok(address)
            }
            Mode::Izx => {
                let base = self.fetch(bus)?;
                self.read(bus, u16::from(base))?;
                let pointer = base.wrapping_add(self.r.x);
                let low = self.read(bus, u16::from(pointer))?;
                Ok(u16::from_le_bytes([
                    low,
                    self.read(bus, u16::from(pointer.wrapping_add(1)))?,
                ]))
            }
        }
    }

    fn interrupt<T: Bus>(
        &mut self,
        bus: &mut T,
        vector: u16,
        software: bool,
    ) -> Result<(), Error<T::Error>> {
        let [low, high] = self.r.pc.to_le_bytes();
        self.push(bus, high)?;
        self.push(bus, low)?;
        self.push(bus, self.r.p | if software { B } else { 0 })?;
        self.r.p |= I;
        // An NMI sampled before vector fetch can redirect IRQ/BRK without changing
        // the return address or the software break marker already pushed.
        let vector = if vector == 0xfffe && self.nmi_pending {
            self.nmi_pending = false;
            0xfffa
        } else {
            vector
        };
        self.r.pc = self.vector(bus, vector)?;
        Ok(())
    }

    fn execute<T: Bus>(&mut self, bus: &mut T) -> Result<StepKind, Error<T::Error>> {
        let masked = self.delayed_i.take().unwrap_or(self.r.p & I != 0);
        if self.nmi_pending || (self.polled_irq.unwrap_or(self.irq) && !masked) {
            bus.execution(Execution::Interrupt).map_err(Error::Bus)?;
            let nmi = self.nmi_pending;
            self.nmi_pending = false;
            self.read(bus, self.r.pc)?;
            self.read(bus, self.r.pc)?;
            self.interrupt(bus, if nmi { 0xfffa } else { 0xfffe }, false)?;
            return Ok(if nmi { StepKind::Nmi } else { StepKind::Irq });
        }
        let address = self.r.pc;
        bus.execution(Execution::Instruction(self.r))
            .map_err(Error::Bus)?;
        let opcode = self.fetch(bus)?;
        let Some((op, mode)) = decode(opcode) else {
            return Err(Error::UnsupportedOpcode { address, opcode });
        };
        if let Some(mode) = mode {
            let modify = matches!(
                op,
                Op::Asl | Op::Lsr | Op::Rol | Op::Ror | Op::Inc | Op::Dec
            );
            let store = matches!(op, Op::Sta | Op::Stx | Op::Sty);
            let address = self.address(bus, mode, modify || store)?;
            if store {
                let value = match op {
                    Op::Sta => self.r.a,
                    Op::Stx => self.r.x,
                    _ => self.r.y,
                };
                self.write(bus, address, value)?;
            } else {
                let value = self.read(bus, address)?;
                if modify {
                    self.write(bus, address, value)?;
                    let result = self.modify(op, value).ok_or(Error::DecodeInvariant)?;
                    self.write(bus, address, result)?;
                } else if !self.operand(op, value) {
                    return Err(Error::DecodeInvariant);
                }
            }
        } else {
            self.special(bus, op)?;
        }
        Ok(StepKind::Instruction(opcode))
    }

    fn operand(&mut self, op: Op, value: u8) -> bool {
        match op {
            Op::Lda => {
                self.r.a = value;
                self.nz(value);
            }
            Op::Ldx => {
                self.r.x = value;
                self.nz(value);
            }
            Op::Ldy => {
                self.r.y = value;
                self.nz(value);
            }
            Op::Ora | Op::And | Op::Eor => {
                self.r.a = match op {
                    Op::Ora => self.r.a | value,
                    Op::And => self.r.a & value,
                    _ => self.r.a ^ value,
                };
                self.nz(self.r.a);
            }
            Op::Adc => self.adc(value),
            Op::Sbc => self.sbc(value),
            Op::Cmp => self.compare(self.r.a, value),
            Op::Cpx => self.compare(self.r.x, value),
            Op::Cpy => self.compare(self.r.y, value),
            Op::Bit => {
                self.flag(N, value & N != 0);
                self.flag(V, value & V != 0);
                self.flag(Z, self.r.a & value == 0);
            }
            _ => return false,
        }
        true
    }

    fn modify(&mut self, op: Op, value: u8) -> Option<u8> {
        let carry = u8::from(self.r.p & C != 0);
        let result = match op {
            Op::Asl | Op::Rol => {
                self.flag(C, value & 0x80 != 0);
                (value << 1) | if op == Op::Rol { carry } else { 0 }
            }
            Op::Lsr | Op::Ror => {
                self.flag(C, value & 1 != 0);
                (value >> 1) | if op == Op::Ror { carry << 7 } else { 0 }
            }
            Op::Inc => value.wrapping_add(1),
            Op::Dec => value.wrapping_sub(1),
            _ => return None,
        };
        self.nz(result);
        Some(result)
    }

    fn adc(&mut self, value: u8) {
        let a = self.r.a;
        let carry = u16::from(self.r.p & C != 0);
        let binary = u16::from(a) + u16::from(value) + carry;
        let mut sum = binary;
        if self.r.p & D != 0 {
            let mut low = u16::from(a & 15) + u16::from(value & 15) + carry;
            if low > 9 {
                low = ((low + 6) & 15) | 0x10;
            }
            sum = u16::from(a & 0xf0) + u16::from(value & 0xf0) + low;
        }
        let intermediate = sum.to_le_bytes()[0];
        self.flag(Z, binary.to_le_bytes()[0] == 0);
        self.flag(N, intermediate & N != 0);
        self.flag(V, (!(a ^ value) & (a ^ intermediate) & N) != 0);
        if self.r.p & D != 0 && sum >= 0xa0 {
            sum += 0x60;
        }
        self.flag(C, sum > 0xff);
        self.r.a = sum.to_le_bytes()[0];
    }

    fn sbc(&mut self, value: u8) {
        let a = self.r.a;
        let borrow = i16::from(self.r.p & C == 0);
        let binary = i16::from(a) - i16::from(value) - borrow;
        let result = binary.to_le_bytes()[0];
        self.nz(result);
        self.flag(C, binary >= 0);
        self.flag(V, (a ^ value) & (a ^ result) & N != 0);
        self.r.a = if self.r.p & D != 0 {
            let mut low = i16::from(a & 15) - i16::from(value & 15) - borrow;
            let mut high = i16::from(a >> 4) - i16::from(value >> 4);
            if low < 0 {
                low -= 6;
                high -= 1;
            }
            if high < 0 {
                high -= 6;
            }
            ((high << 4) & 0xf0 | (low & 15)).to_le_bytes()[0]
        } else {
            result
        };
    }

    fn special<T: Bus>(&mut self, bus: &mut T, op: Op) -> Result<(), Error<T::Error>> {
        match op {
            Op::Brk => {
                self.fetch(bus)?;
                self.interrupt(bus, 0xfffe, true)?;
            }
            Op::Jmp => self.r.pc = self.word(bus)?,
            Op::Jmi => {
                let pointer = self.word(bus)?;
                let low = self.read(bus, pointer)?;
                // NMOS indirect jump wraps the high-byte fetch within the pointer page.
                let high = self.read(
                    bus,
                    (pointer & 0xff00) | u16::from(pointer.to_le_bytes()[0].wrapping_add(1)),
                )?;
                self.r.pc = u16::from_le_bytes([low, high]);
            }
            Op::Jsr => {
                let low = self.fetch(bus)?;
                self.read(bus, 0x100 | u16::from(self.r.sp))?;
                let [ret_low, ret_high] = self.r.pc.to_le_bytes();
                self.push(bus, ret_high)?;
                self.push(bus, ret_low)?;
                self.r.pc = u16::from_le_bytes([low, self.fetch(bus)?]);
            }
            Op::Rts | Op::Rti => {
                self.read(bus, self.r.pc)?;
                self.read(bus, 0x100 | u16::from(self.r.sp))?;
                if op == Op::Rti {
                    self.r.p = (self.pull(bus)? | U) & !B;
                }
                let low = self.pull(bus)?;
                self.r.pc = u16::from_le_bytes([low, self.pull(bus)?]);
                if op == Op::Rts {
                    self.read(bus, self.r.pc)?;
                    self.r.pc = self.r.pc.wrapping_add(1);
                }
            }
            Op::Bpl | Op::Bmi | Op::Bvc | Op::Bvs | Op::Bcc | Op::Bcs | Op::Bne | Op::Beq => {
                let offset = i8::from_ne_bytes([self.fetch(bus)?]);
                let (mask, set) = match op {
                    Op::Bpl => (N, false),
                    Op::Bmi => (N, true),
                    Op::Bvc => (V, false),
                    Op::Bvs => (V, true),
                    Op::Bcc => (C, false),
                    Op::Bcs => (C, true),
                    Op::Bne => (Z, false),
                    _ => (Z, true),
                };
                if (self.r.p & mask != 0) == set {
                    let target = self.r.pc.wrapping_add_signed(i16::from(offset));
                    self.read(bus, self.r.pc)?;
                    if target & 0xff00 != self.r.pc & 0xff00 {
                        self.read(bus, (self.r.pc & 0xff00) | (target & 0xff))?;
                    }
                    self.r.pc = target;
                }
            }
            _ => {
                self.read(bus, self.r.pc)?;
                match op {
                    Op::Pha | Op::Php => self.push(
                        bus,
                        if op == Op::Pha {
                            self.r.a
                        } else {
                            self.r.p | B
                        },
                    )?,
                    Op::Pla | Op::Plp => {
                        self.read(bus, 0x100 | u16::from(self.r.sp))?;
                        let value = self.pull(bus)?;
                        if op == Op::Pla {
                            self.r.a = value;
                            self.nz(value);
                        } else {
                            self.delayed_i = Some(self.r.p & I != 0);
                            self.r.p = (value | U) & !B;
                        }
                    }
                    Op::Asl | Op::Lsr | Op::Rol | Op::Ror => {
                        self.r.a = self.modify(op, self.r.a).ok_or(Error::DecodeInvariant)?;
                    }
                    Op::Clc => self.r.p &= !C,
                    Op::Sec => self.r.p |= C,
                    Op::Cli | Op::Sei => {
                        self.delayed_i = Some(self.r.p & I != 0);
                        self.flag(I, op == Op::Sei);
                    }
                    Op::Cld => self.r.p &= !D,
                    Op::Sed => self.r.p |= D,
                    Op::Clv => self.r.p &= !V,
                    Op::Inx => {
                        self.r.x = self.r.x.wrapping_add(1);
                        self.nz(self.r.x);
                    }
                    Op::Iny => {
                        self.r.y = self.r.y.wrapping_add(1);
                        self.nz(self.r.y);
                    }
                    Op::Dex => {
                        self.r.x = self.r.x.wrapping_sub(1);
                        self.nz(self.r.x);
                    }
                    Op::Dey => {
                        self.r.y = self.r.y.wrapping_sub(1);
                        self.nz(self.r.y);
                    }
                    Op::Tax => {
                        self.r.x = self.r.a;
                        self.nz(self.r.x);
                    }
                    Op::Tay => {
                        self.r.y = self.r.a;
                        self.nz(self.r.y);
                    }
                    Op::Txa => {
                        self.r.a = self.r.x;
                        self.nz(self.r.a);
                    }
                    Op::Tya => {
                        self.r.a = self.r.y;
                        self.nz(self.r.a);
                    }
                    Op::Tsx => {
                        self.r.x = self.r.sp;
                        self.nz(self.r.x);
                    }
                    Op::Txs => self.r.sp = self.r.x,
                    Op::Nop => {}
                    _ => return Err(Error::DecodeInvariant),
                }
            }
        }
        Ok(())
    }
}
