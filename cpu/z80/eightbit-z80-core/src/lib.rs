//! Independently authored Z80 execution from Zilog UM008011-0816 facts.
//! No filesystem, firmware, host evaluator or external emulator dependency.
#![allow(missing_docs)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

pub mod assembler;

pub const C: u8 = 0x01;
pub const N: u8 = 0x02;
pub const PV: u8 = 0x04;
pub const X: u8 = 0x08;
pub const H: u8 = 0x10;
pub const Y: u8 = 0x20;
pub const Z: u8 = 0x40;
pub const S: u8 = 0x80;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    Fetch,
    Read,
    Write,
    Input,
    Output,
}

/// The CPU owns T states; a board can add contention before each bus transfer.
pub trait Bus {
    fn read(&mut self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
    fn input(&mut self, port: u16) -> u8;
    fn output(&mut self, port: u16, value: u8);
    fn tick(&mut self, t_states: u32);
    fn wait_states(&self, _address: u16, _access: Access) -> u8 {
        0
    }
    fn fetch(&mut self, address: u16) -> u8 {
        self.read(address)
    }
    /// Observe the refresh address after each M1/interrupt refresh increment.
    fn refresh(&mut self, _address: u16) {}
    fn irq(&self) -> bool {
        false
    }
    fn nmi(&self) -> bool {
        false
    }
    fn interrupt_acknowledge(&mut self) -> u8 {
        0xff
    }
    fn reti(&mut self) {}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Registers {
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub alternate: [u8; 8],
    pub ix: u16,
    pub iy: u16,
    pub sp: u16,
    pub pc: u16,
    pub i: u8,
    pub r: u8,
    pub iff1: bool,
    pub iff2: bool,
    pub interrupt_mode: u8,
    pub wz: u16,
}
impl Registers {
    #[must_use]
    pub fn af(self) -> u16 {
        u16::from_be_bytes([self.a, self.f])
    }
    #[must_use]
    pub fn bc(self) -> u16 {
        u16::from_be_bytes([self.b, self.c])
    }
    #[must_use]
    pub fn de(self) -> u16 {
        u16::from_be_bytes([self.d, self.e])
    }
    #[must_use]
    pub fn hl(self) -> u16 {
        u16::from_be_bytes([self.h, self.l])
    }
    pub fn set_af(&mut self, value: u16) {
        [self.a, self.f] = value.to_be_bytes();
    }
    pub fn set_bc(&mut self, value: u16) {
        [self.b, self.c] = value.to_be_bytes();
    }
    pub fn set_de(&mut self, value: u16) {
        [self.d, self.e] = value.to_be_bytes();
    }
    pub fn set_hl(&mut self, value: u16) {
        [self.h, self.l] = value.to_be_bytes();
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Index {
    #[default]
    Hl,
    Ix,
    Iy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrefixLimit {
    pub address: u16,
}
impl std::fmt::Display for PrefixLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "S48CPU1001: excessive instruction prefixes at ${:04X}",
            self.address
        )
    }
}
impl std::error::Error for PrefixLimit {}

#[derive(Default)]
pub struct Cpu {
    pub registers: Registers,
    pub halted: bool,
    t_states: u32,
    ei_delay: u8,
    nmi_previous: bool,
    nmi_pending: bool,
}

#[must_use]
pub fn parity(value: u8) -> bool {
    value.count_ones().is_multiple_of(2)
}
fn sz53(value: u8) -> u8 {
    (value & (S | X | Y)) | if value == 0 { Z } else { 0 }
}
fn szp53(value: u8) -> u8 {
    sz53(value) | if parity(value) { PV } else { 0 }
}

impl Cpu {
    pub fn reset(&mut self) {
        // Only reset-defined state is an acceptance contract; all other registers
        // start deterministically, rather than modelling electrical power noise.
        *self = Self::default();
        self.registers.sp = 0xffff;
        self.registers.set_af(0xffff);
    }
    fn idle(&mut self, bus: &mut impl Bus, count: u32) {
        for _ in 0..count {
            bus.tick(1);
            let nmi = bus.nmi();
            self.nmi_pending |= nmi && !self.nmi_previous;
            self.nmi_previous = nmi;
        }
        self.t_states += count;
    }
    fn wait(&mut self, bus: &mut impl Bus, address: u16, access: Access) {
        let count = u32::from(bus.wait_states(address, access));
        self.idle(bus, count);
    }
    fn refresh(&mut self, bus: &mut impl Bus) {
        self.registers.r = (self.registers.r & 0x80) | (self.registers.r.wrapping_add(1) & 0x7f);
        bus.refresh(u16::from_be_bytes([self.registers.i, self.registers.r]));
    }
    fn m1(&mut self, bus: &mut impl Bus) -> u8 {
        let pc = self.registers.pc;
        self.wait(bus, pc, Access::Fetch);
        self.idle(bus, 3);
        let byte = bus.fetch(pc);
        self.registers.pc = pc.wrapping_add(1);
        self.refresh(bus);
        self.idle(bus, 1);
        byte
    }
    fn read(&mut self, bus: &mut impl Bus, address: u16) -> u8 {
        self.wait(bus, address, Access::Read);
        self.idle(bus, 2);
        let byte = bus.read(address);
        self.idle(bus, 1);
        byte
    }
    fn write(&mut self, bus: &mut impl Bus, address: u16, byte: u8) {
        self.wait(bus, address, Access::Write);
        self.idle(bus, 2);
        bus.write(address, byte);
        self.idle(bus, 1);
    }
    fn next(&mut self, bus: &mut impl Bus) -> u8 {
        let pc = self.registers.pc;
        self.registers.pc = pc.wrapping_add(1);
        self.read(bus, pc)
    }
    fn next_word(&mut self, bus: &mut impl Bus) -> u16 {
        let lo = self.next(bus);
        let hi = self.next(bus);
        u16::from_le_bytes([lo, hi])
    }
    fn word(&mut self, bus: &mut impl Bus, address: u16) -> u16 {
        let lo = self.read(bus, address);
        let hi = self.read(bus, address.wrapping_add(1));
        u16::from_le_bytes([lo, hi])
    }
    fn put_word(&mut self, bus: &mut impl Bus, address: u16, word: u16) {
        let [lo, hi] = word.to_le_bytes();
        self.write(bus, address, lo);
        self.write(bus, address.wrapping_add(1), hi);
    }
    fn push(&mut self, bus: &mut impl Bus, word: u16) {
        let [hi, lo] = word.to_be_bytes();
        self.registers.sp = self.registers.sp.wrapping_sub(1);
        self.write(bus, self.registers.sp, hi);
        self.registers.sp = self.registers.sp.wrapping_sub(1);
        self.write(bus, self.registers.sp, lo);
    }
    fn pop(&mut self, bus: &mut impl Bus) -> u16 {
        let word = self.word(bus, self.registers.sp);
        self.registers.sp = self.registers.sp.wrapping_add(2);
        word
    }
    fn input(&mut self, bus: &mut impl Bus, port: u16) -> u8 {
        self.wait(bus, port, Access::Input);
        self.idle(bus, 3);
        let value = bus.input(port);
        self.idle(bus, 1);
        value
    }
    fn output(&mut self, bus: &mut impl Bus, port: u16, value: u8) {
        self.wait(bus, port, Access::Output);
        self.idle(bus, 3);
        bus.output(port, value);
        self.idle(bus, 1);
    }
    fn pair(&self, register: u8, index: Index) -> u16 {
        match register {
            0 => self.registers.bc(),
            1 => self.registers.de(),
            2 => match index {
                Index::Hl => self.registers.hl(),
                Index::Ix => self.registers.ix,
                Index::Iy => self.registers.iy,
            },
            _ => self.registers.sp,
        }
    }
    fn set_pair(&mut self, register: u8, index: Index, value: u16) {
        match register {
            0 => self.registers.set_bc(value),
            1 => self.registers.set_de(value),
            2 => match index {
                Index::Hl => self.registers.set_hl(value),
                Index::Ix => self.registers.ix = value,
                Index::Iy => self.registers.iy = value,
            },
            _ => self.registers.sp = value,
        }
    }
    fn reg(&self, register: u8, index: Index) -> u8 {
        match register {
            0 => self.registers.b,
            1 => self.registers.c,
            2 => self.registers.d,
            3 => self.registers.e,
            4 => (self.pair(2, index) >> 8) as u8,
            5 => self.pair(2, index) as u8,
            _ => self.registers.a,
        }
    }
    fn set_reg(&mut self, register: u8, index: Index, value: u8) {
        match register {
            0 => self.registers.b = value,
            1 => self.registers.c = value,
            2 => self.registers.d = value,
            3 => self.registers.e = value,
            4 => self.set_pair(
                2,
                index,
                (self.pair(2, index) & 255) | (u16::from(value) << 8),
            ),
            5 => self.set_pair(2, index, (self.pair(2, index) & 0xff00) | u16::from(value)),
            _ => self.registers.a = value,
        }
    }
    fn condition(&self, condition: u8) -> bool {
        let mask = [Z, C, PV, S][usize::from(condition / 2)];
        (self.registers.f & mask != 0) == (condition & 1 != 0)
    }
    fn address(&mut self, bus: &mut impl Bus, index: Index, idle: u32) -> u16 {
        if index == Index::Hl {
            self.registers.hl()
        } else {
            let displacement = self.next(bus).cast_signed();
            self.idle(bus, idle);
            let address = self
                .pair(2, index)
                .wrapping_add_signed(i16::from(displacement));
            self.registers.wz = address;
            address
        }
    }
    fn inc(&mut self, value: u8) -> u8 {
        let result = value.wrapping_add(1);
        self.registers.f = (self.registers.f & C)
            | sz53(result)
            | if value & 15 == 15 { H } else { 0 }
            | if value == 0x7f { PV } else { 0 };
        result
    }
    fn dec(&mut self, value: u8) -> u8 {
        let result = value.wrapping_sub(1);
        self.registers.f = (self.registers.f & C)
            | N
            | sz53(result)
            | if value.is_multiple_of(16) { H } else { 0 }
            | if value == 0x80 { PV } else { 0 };
        result
    }
    fn arithmetic(&mut self, value: u8, subtract: bool, carry: u8) -> u8 {
        let a = self.registers.a;
        let full = if subtract {
            u16::from(a).wrapping_sub(u16::from(value) + u16::from(carry))
        } else {
            u16::from(a) + u16::from(value) + u16::from(carry)
        };
        let result = full as u8;
        let overflow = ((a ^ result) & if subtract { a ^ value } else { !(a ^ value) }) & 0x80 != 0;
        self.registers.f = sz53(result)
            | if subtract { N } else { 0 }
            | ((a ^ value ^ result) & H)
            | if overflow { PV } else { 0 }
            | if full & 0x100 != 0 { C } else { 0 };
        result
    }
    fn alu(&mut self, operation: u8, value: u8) {
        let carry = self.registers.f & C;
        let result = match operation {
            0 => self.arithmetic(value, false, 0),
            1 => self.arithmetic(value, false, carry),
            2 | 7 => self.arithmetic(value, true, 0),
            3 => self.arithmetic(value, true, carry),
            4 => self.registers.a & value,
            5 => self.registers.a ^ value,
            _ => self.registers.a | value,
        };
        if (4..=6).contains(&operation) {
            self.registers.f = szp53(result) | if operation == 4 { H } else { 0 };
        }
        if operation == 7 {
            self.registers.f = (self.registers.f & !(X | Y)) | (value & (X | Y));
        } else {
            self.registers.a = result;
        }
    }
    fn add_pair(&mut self, register: u8, index: Index) {
        let a = self.pair(2, index);
        let b = self.pair(register, index);
        let result = a.wrapping_add(b);
        self.registers.f = (self.registers.f & (S | Z | PV))
            | ((result >> 8) as u8 & (X | Y))
            | if (a ^ b ^ result) & 0x1000 != 0 { H } else { 0 }
            | if u32::from(a) + u32::from(b) > 65535 {
                C
            } else {
                0
            };
        self.registers.wz = a.wrapping_add(1);
        self.set_pair(2, index, result);
    }
    fn adc_pair(&mut self, register: u8, subtract: bool) {
        let a = self.registers.hl();
        let b = self.pair(register, Index::Hl);
        let carry = u32::from(self.registers.f & C);
        let full = if subtract {
            u32::from(a).wrapping_sub(u32::from(b) + carry)
        } else {
            u32::from(a) + u32::from(b) + carry
        };
        let result = full as u16;
        self.registers.f = ((result >> 8) as u8 & (S | X | Y))
            | if result == 0 { Z } else { 0 }
            | if subtract { N } else { 0 }
            | if (a ^ b ^ result) & 0x1000 != 0 { H } else { 0 }
            | if ((a ^ result) & if subtract { a ^ b } else { !(a ^ b) }) & 0x8000 != 0 {
                PV
            } else {
                0
            }
            | if full & 0x10000 != 0 { C } else { 0 };
        self.registers.wz = a.wrapping_add(1);
        self.registers.set_hl(result);
    }
    fn daa(&mut self) {
        let old = self.registers.a;
        let subtract = self.registers.f & N != 0;
        let carry = self.registers.f & C != 0 || (!subtract && old > 0x99);
        let correction = if self.registers.f & H != 0 || (!subtract && old & 15 > 9) {
            6
        } else {
            0
        } | if carry { 0x60 } else { 0 };
        let result = if subtract {
            old.wrapping_sub(correction)
        } else {
            old.wrapping_add(correction)
        };
        self.registers.a = result;
        self.registers.f = (self.registers.f & N)
            | szp53(result)
            | ((old ^ result) & H)
            | if carry { C } else { 0 };
    }
    fn branch(&mut self, bus: &mut impl Bus, displacement: u8) {
        self.idle(bus, 5);
        self.registers.pc = self
            .registers
            .pc
            .wrapping_add_signed(i16::from(displacement.cast_signed()));
        self.registers.wz = self.registers.pc;
    }
    fn interrupt(&mut self, bus: &mut impl Bus) -> bool {
        let nmi = bus.nmi();
        self.nmi_pending |= nmi && !self.nmi_previous;
        self.nmi_previous = nmi;
        if self.nmi_pending {
            self.nmi_pending = false;
            self.halted = false;
            // NMI leaves IFF2 unchanged, including a nested NMI (Zilog Table 1).
            self.registers.iff1 = false;
            self.refresh(bus);
            self.idle(bus, 5);
            self.push(bus, self.registers.pc);
            self.registers.pc = 0x66;
        } else if bus.irq() && self.registers.iff1 && self.ei_delay == 0 {
            self.halted = false;
            self.registers.iff1 = false;
            self.registers.iff2 = false;
            let vector = bus.interrupt_acknowledge();
            self.refresh(bus);
            self.idle(bus, 7);
            match self.registers.interrupt_mode {
                0 if vector & 0xc7 != 0xc7 => self.base(bus, vector, Index::Hl),
                2 => {
                    self.push(bus, self.registers.pc);
                    let table = u16::from_be_bytes([self.registers.i, vector]);
                    self.registers.pc = self.word(bus, table);
                }
                _ => {
                    self.push(bus, self.registers.pc);
                    self.registers.pc = if self.registers.interrupt_mode == 0 {
                        u16::from(vector & 0x38)
                    } else {
                        0x38
                    };
                }
            }
        } else {
            return false;
        }
        self.registers.wz = self.registers.pc;
        true
    }
    /// Executes one bounded instruction or interrupt; returns actual elapsed T states.
    pub fn step(&mut self, bus: &mut impl Bus) -> Result<u32, PrefixLimit> {
        self.t_states = 0;
        if self.interrupt(bus) {
            return Ok(self.t_states);
        }
        if self.halted {
            self.wait(bus, self.registers.pc, Access::Fetch);
            bus.fetch(self.registers.pc);
            self.refresh(bus);
            self.idle(bus, 4);
            return Ok(self.t_states);
        }
        let start = self.registers.pc;
        let mut index = Index::Hl;
        for prefix_count in 0..64 {
            match self.m1(bus) {
                0xdd => index = Index::Ix,
                0xfd => index = Index::Iy,
                0xcb => {
                    self.cb(bus, index);
                    break;
                }
                0xed => {
                    let opcode = self.m1(bus);
                    self.ed(bus, opcode);
                    break;
                }
                opcode => {
                    self.base(bus, opcode, index);
                    break;
                }
            }
            if prefix_count == 63 {
                return Err(PrefixLimit { address: start });
            }
        }
        self.ei_delay = self.ei_delay.saturating_sub(1);
        Ok(self.t_states)
    }
}

mod instructions;
