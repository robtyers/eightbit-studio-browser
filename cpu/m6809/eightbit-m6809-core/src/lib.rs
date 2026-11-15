//! Original bounded MC6809E interpreter, implemented from Motorola M6809PM facts.
//! Undocumented encodings stop explicitly; pin-level timing needs separate qualification.
#![allow(missing_docs)]

pub mod assembler;

pub const C: u8 = 1;
pub const V: u8 = 2;
pub const Z: u8 = 4;
pub const N: u8 = 8;
pub const I: u8 = 16;
pub const H: u8 = 32;
pub const F: u8 = 64;
pub const E: u8 = 128;

/// Memory operations advance one bus cycle; internal cycles have no memory effect.
pub trait Bus {
    fn read(&mut self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
    fn tick(&mut self, cycles: u32);
    fn interrupts(&self) -> InterruptLines {
        InterruptLines::default()
    }
    fn halted(&self) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InterruptLines {
    pub irq: bool,
    pub firq: bool,
    pub nmi: bool,
}
#[derive(Default, PartialEq, Eq)]
enum Wait {
    #[default]
    None,
    Sync,
    Cwai,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Registers {
    pub a: u8,
    pub b: u8,
    pub dp: u8,
    pub cc: u8,
    pub x: u16,
    pub y: u16,
    pub u: u16,
    pub s: u16,
    pub pc: u16,
}
impl Registers {
    #[must_use]
    pub fn d(self) -> u16 {
        u16::from_be_bytes([self.a, self.b])
    }
    pub fn set_d(&mut self, value: u16) {
        [self.a, self.b] = value.to_be_bytes();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unsupported {
    pub address: u16,
    pub opcode: u8,
}
impl std::fmt::Display for Unsupported {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DME1001: unsupported instruction ${:02X} at ${:04X}",
            self.opcode, self.address
        )
    }
}
impl std::error::Error for Unsupported {}

#[derive(Default)]
pub struct Cpu {
    pub registers: Registers,
    bus_cycles: u32,
    waiting: Wait,
    nmi_armed: bool,
    nmi_previous: bool,
    nmi_pending: bool,
}
impl Cpu {
    pub fn reset(&mut self, bus: &mut impl Bus) {
        self.registers = Registers {
            cc: I | F,
            ..Registers::default()
        };
        self.waiting = Wait::None;
        self.nmi_armed = false;
        self.nmi_previous = bus.interrupts().nmi;
        self.nmi_pending = false;
        self.bus_cycles = 0;
        self.idle(bus, 5);
        self.registers.pc = self.word(bus, 0xfffe);
    }
    fn read(&mut self, bus: &mut impl Bus, address: u16) -> u8 {
        let value = bus.read(address);
        self.idle(bus, 1);
        value
    }
    fn idle(&mut self, bus: &mut impl Bus, cycles: u32) {
        for _ in 0..cycles {
            bus.tick(1);
            let nmi = bus.interrupts().nmi;
            if nmi && !self.nmi_previous && self.nmi_armed {
                self.nmi_pending = true;
            }
            self.nmi_previous = nmi;
        }
        self.bus_cycles += cycles;
    }
    fn full_frame(&mut self, bus: &mut impl Bus) {
        self.registers.cc |= E;
        for value in [
            self.registers.pc,
            self.registers.u,
            self.registers.y,
            self.registers.x,
        ] {
            self.push16(bus, false, value);
        }
        for value in [
            self.registers.dp,
            self.registers.b,
            self.registers.a,
            self.registers.cc,
        ] {
            self.push8(bus, false, value);
        }
    }
    fn hardware_interrupt(&mut self, bus: &mut impl Bus) -> Option<u32> {
        let lines = bus.interrupts();
        if lines.nmi && !self.nmi_previous && self.nmi_armed {
            self.nmi_pending = true;
        }
        self.nmi_previous = lines.nmi;
        if self.waiting == Wait::Sync && (lines.irq || lines.firq || lines.nmi) {
            self.waiting = Wait::None;
        }
        let (vector, full, masks) = if self.nmi_pending {
            self.nmi_pending = false;
            (0xfffc, true, I | F)
        } else if lines.firq && self.registers.cc & F == 0 {
            (0xfff6, false, I | F)
        } else if lines.irq && self.registers.cc & I == 0 {
            (0xfff8, true, I)
        } else {
            return None;
        };
        let saved = self.waiting == Wait::Cwai;
        self.waiting = Wait::None;
        self.idle(bus, 2);
        if !saved {
            if full {
                self.full_frame(bus);
            } else {
                self.registers.cc &= !E;
                self.push16(bus, false, self.registers.pc);
                self.push8(bus, false, self.registers.cc);
            }
        }
        self.registers.cc |= masks;
        self.registers.pc = self.word(bus, vector);
        let cycles = if saved {
            7
        } else if full {
            19
        } else {
            10
        };
        self.idle(bus, cycles - self.bus_cycles);
        Some(cycles)
    }
    fn write(&mut self, bus: &mut impl Bus, address: u16, value: u8) {
        bus.write(address, value);
        self.idle(bus, 1);
    }
    fn word(&mut self, bus: &mut impl Bus, address: u16) -> u16 {
        let hi = self.read(bus, address);
        let lo = self.read(bus, address.wrapping_add(1));
        u16::from_be_bytes([hi, lo])
    }
    fn put_word(&mut self, bus: &mut impl Bus, address: u16, value: u16) {
        let [hi, lo] = value.to_be_bytes();
        self.write(bus, address, hi);
        self.write(bus, address.wrapping_add(1), lo);
    }
    fn fetch(&mut self, bus: &mut impl Bus) -> u8 {
        let address = self.registers.pc;
        self.registers.pc = address.wrapping_add(1);
        self.read(bus, address)
    }
    fn fetch_word(&mut self, bus: &mut impl Bus) -> u16 {
        let hi = self.fetch(bus);
        let lo = self.fetch(bus);
        u16::from_be_bytes([hi, lo])
    }
    fn flag(&mut self, mask: u8, value: bool) {
        if value {
            self.registers.cc |= mask;
        } else {
            self.registers.cc &= !mask;
        }
    }
    fn nz8(&mut self, value: u8) {
        self.flag(N, value & 0x80 != 0);
        self.flag(Z, value == 0);
    }
    fn nz16(&mut self, value: u16) {
        self.flag(N, value & 0x8000 != 0);
        self.flag(Z, value == 0);
    }
    fn push8(&mut self, bus: &mut impl Bus, user: bool, value: u8) {
        let sp = if user {
            &mut self.registers.u
        } else {
            &mut self.registers.s
        };
        *sp = sp.wrapping_sub(1);
        let address = *sp;
        self.write(bus, address, value);
    }
    fn push16(&mut self, bus: &mut impl Bus, user: bool, value: u16) {
        let [hi, lo] = value.to_be_bytes();
        self.push8(bus, user, lo);
        self.push8(bus, user, hi);
    }
    fn pop8(&mut self, bus: &mut impl Bus, user: bool) -> u8 {
        let sp = if user {
            &mut self.registers.u
        } else {
            &mut self.registers.s
        };
        let address = *sp;
        *sp = sp.wrapping_add(1);
        self.read(bus, address)
    }
    fn pop16(&mut self, bus: &mut impl Bus, user: bool) -> u16 {
        let hi = self.pop8(bus, user);
        let lo = self.pop8(bus, user);
        u16::from_be_bytes([hi, lo])
    }
    fn index(&mut self, bus: &mut impl Bus, address: u16) -> Result<(u16, u32), Unsupported> {
        let start = self.bus_cycles;
        let post = self.fetch(bus);
        let selected = (post >> 5) & 3;
        let base = match selected {
            0 => self.registers.x,
            1 => self.registers.y,
            2 => self.registers.u,
            _ => self.registers.s,
        };
        if post & 0x80 == 0 {
            let raw = post & 31;
            let offset = if raw & 16 == 0 {
                i16::from(raw)
            } else {
                i16::from(raw) - 32
            };
            self.idle(bus, 2);
            return Ok((base.wrapping_add_signed(offset), 1));
        }
        if post & 16 != 0 && matches!(post & 15, 0 | 2) || post & 15 == 15 && post != 0x9f {
            return Err(Unsupported {
                address,
                opcode: post,
            });
        }
        let mut updated = base;
        let (mut effective, mut extra) = match post & 15 {
            0 => {
                updated = base.wrapping_add(1);
                (base, 2)
            }
            1 => {
                updated = base.wrapping_add(2);
                (base, 3)
            }
            2 => {
                updated = base.wrapping_sub(1);
                (updated, 2)
            }
            3 => {
                updated = base.wrapping_sub(2);
                (updated, 3)
            }
            4 => (base, 0),
            5 => (
                base.wrapping_add_signed(i16::from(i8::from_ne_bytes([self.registers.b]))),
                1,
            ),
            6 => (
                base.wrapping_add_signed(i16::from(i8::from_ne_bytes([self.registers.a]))),
                1,
            ),
            8 => {
                let v = self.fetch(bus);
                (
                    base.wrapping_add_signed(i16::from(i8::from_ne_bytes([v]))),
                    1,
                )
            }
            9 => {
                let v = self.fetch_word(bus);
                (base.wrapping_add(v), 4)
            }
            11 => (base.wrapping_add(self.registers.d()), 4),
            12 => {
                let v = self.fetch(bus);
                (
                    self.registers
                        .pc
                        .wrapping_add_signed(i16::from(i8::from_ne_bytes([v]))),
                    1,
                )
            }
            13 => {
                let v = self.fetch_word(bus);
                (self.registers.pc.wrapping_add(v), 5)
            }
            15 if post & 16 != 0 => (self.fetch_word(bus), 2),
            _ => {
                return Err(Unsupported {
                    address,
                    opcode: post,
                });
            }
        };
        match selected {
            0 => self.registers.x = updated,
            1 => self.registers.y = updated,
            2 => self.registers.u = updated,
            _ => self.registers.s = updated,
        }
        self.idle(bus, 2 + extra - (self.bus_cycles - start));
        if post & 16 != 0 {
            effective = self.word(bus, effective);
            self.idle(bus, 1);
            extra += 3;
        }
        Ok((effective, extra))
    }
    fn operand(
        &mut self,
        bus: &mut impl Bus,
        mode: u8,
        address: u16,
    ) -> Result<(u16, u32), Unsupported> {
        Ok(match mode {
            0 => {
                let a = self.registers.pc;
                self.registers.pc = a.wrapping_add(1);
                (a, 0)
            }
            1 => {
                let lo = self.fetch(bus);
                self.idle(bus, 1);
                (u16::from_be_bytes([self.registers.dp, lo]), 2)
            }
            2 => {
                let (a, n) = self.index(bus, address)?;
                (a, n + 2)
            }
            _ => {
                let ea = self.fetch_word(bus);
                self.idle(bus, 1);
                (ea, 3)
            }
        })
    }
    fn branch(&self, condition: u8) -> bool {
        let cc = self.registers.cc;
        let c = cc & C != 0;
        let z = cc & Z != 0;
        let n = cc & N != 0;
        let v = cc & V != 0;
        match condition {
            0 => true,
            1 => false,
            2 => !c && !z,
            3 => c || z,
            4 => !c,
            5 => c,
            6 => !z,
            7 => z,
            8 => !v,
            9 => v,
            10 => !n,
            11 => n,
            12 => n == v,
            13 => n != v,
            14 => !z && n == v,
            _ => z || n != v,
        }
    }
    fn arithmetic8(&mut self, left: u8, right: u8, subtract: bool, carry: bool) -> u8 {
        let carry = u16::from(carry);
        let wide = if subtract {
            u16::from(left).wrapping_sub(u16::from(right) + carry)
        } else {
            u16::from(left) + u16::from(right) + carry
        };
        let value = wide.to_be_bytes()[1];
        self.flag(
            C,
            if subtract {
                u16::from(left) < u16::from(right) + carry
            } else {
                wide > 255
            },
        );
        self.flag(
            V,
            if subtract {
                (left ^ right) & (left ^ value) & 0x80 != 0
            } else {
                (!(left ^ right)) & (left ^ value) & 0x80 != 0
            },
        );
        if !subtract {
            self.flag(H, u16::from(left & 15) + u16::from(right & 15) + carry > 15);
        }
        self.nz8(value);
        value
    }
    fn unary(&mut self, op: u8, value: u8) -> Option<u8> {
        let carry = self.registers.cc & C != 0;
        let out = match op {
            0 => {
                let result = 0_u8.wrapping_sub(value);
                self.flag(C, value != 0);
                self.flag(V, value == 0x80);
                result
            }
            3 => {
                self.flag(C, true);
                self.flag(V, false);
                !value
            }
            4 => {
                self.flag(C, value & 1 != 0);
                self.flag(N, false);
                value >> 1
            }
            6 => {
                self.flag(C, value & 1 != 0);
                (value >> 1) | if carry { 0x80 } else { 0 }
            }
            7 => {
                self.flag(C, value & 1 != 0);
                (value >> 1) | (value & 0x80)
            }
            8 => {
                self.flag(C, value & 0x80 != 0);
                let result = value << 1;
                self.flag(V, (value ^ result) & 0x80 != 0);
                result
            }
            9 => {
                self.flag(C, value & 0x80 != 0);
                let result = (value << 1) | u8::from(carry);
                self.flag(V, (value ^ result) & 0x80 != 0);
                result
            }
            10 => {
                self.flag(V, value == 0x80);
                value.wrapping_sub(1)
            }
            12 => {
                self.flag(V, value == 0x7f);
                value.wrapping_add(1)
            }
            13 => {
                self.flag(V, false);
                value
            }
            15 => {
                self.flag(C, false);
                self.flag(V, false);
                0
            }
            _ => return None,
        };
        self.nz8(out);
        Some(out)
    }
    /// Execute one instruction, with a finite upper bound on work even for guest loops.
    pub fn step(&mut self, bus: &mut impl Bus) -> Result<u32, Unsupported> {
        self.bus_cycles = 0;
        if bus.halted() {
            self.idle(bus, 1);
            return Ok(1);
        }
        if let Some(cycles) = self.hardware_interrupt(bus) {
            return Ok(cycles);
        }
        if self.waiting != Wait::None {
            self.idle(bus, 1);
            return Ok(1);
        }
        let address = self.registers.pc;
        let first = self.fetch(bus);
        let (page, op) = if matches!(first, 0x10 | 0x11) {
            (first, self.fetch(bus))
        } else {
            (0, first)
        };
        let fail = Unsupported {
            address,
            opcode: first,
        };
        if !documented_opcode(page, op) {
            return Err(fail);
        }
        if page != 0 && op == 0x3f {
            self.full_frame(bus);
            self.registers.pc = self.word(bus, if page == 0x10 { 0xfff4 } else { 0xfff2 });
            self.idle(bus, 20 - self.bus_cycles);
            return Ok(20);
        }
        let cycles = if page == 0 && (0x20..=0x2f).contains(&op) {
            let relative = self.fetch(bus);
            if self.branch(op & 15) {
                self.registers.pc = self
                    .registers
                    .pc
                    .wrapping_add_signed(i16::from(i8::from_ne_bytes([relative])));
            }
            3
        } else if page == 0x10 && (0x21..=0x2f).contains(&op) {
            let relative = self.fetch_word(bus);
            let take = self.branch(op & 15);
            if take {
                self.registers.pc = self.registers.pc.wrapping_add(relative);
            }
            5 + u32::from(take)
        } else if op >= 0x80 {
            let mode = (op >> 4) & 3;
            let operation = op & 15;
            let b = op & 0x40 != 0;
            if page == 0 && op == 0x8d {
                let relative = self.fetch(bus);
                self.push16(bus, false, self.registers.pc);
                self.registers.pc = self
                    .registers
                    .pc
                    .wrapping_add_signed(i16::from(i8::from_ne_bytes([relative])));
                7
            } else {
                let word = matches!(operation, 3 | 12 | 14 | 15) || (b && operation == 13);
                if mode == 0 && (operation == 7 || operation == 15 || (b && operation == 13)) {
                    return Err(fail);
                }
                let (ea, extra) = self.operand(bus, mode, address)?;
                if mode == 0 && word {
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                }
                if !b && operation == 13 && page == 0 {
                    self.push16(bus, false, self.registers.pc);
                    self.registers.pc = ea;
                    5 + extra
                } else if word {
                    let store = operation == 15 || (b && operation == 13);
                    let left = match (page, b, operation) {
                        (0 | 0x10, false, 3) | (0, true, 3 | 12 | 13) => self.registers.d(),
                        (0, false, 12 | 14 | 15) => self.registers.x,
                        (0, true, 14 | 15) | (0x11, false, 3) => self.registers.u,
                        (0x10, false, 12 | 14 | 15) => self.registers.y,
                        (0x10, true, 14 | 15) | (0x11, false, 12) => self.registers.s,
                        _ => return Err(fail),
                    };
                    if store {
                        self.put_word(bus, ea, left);
                        self.nz16(left);
                        self.flag(V, false);
                        3 + extra + u32::from(page != 0)
                    } else {
                        let right = self.word(bus, ea);
                        let arithmetic = operation == 3 || (!b && operation == 12);
                        let out = if arithmetic {
                            let subtract = !(page == 0 && b && operation == 3);
                            let out = if subtract {
                                left.wrapping_sub(right)
                            } else {
                                left.wrapping_add(right)
                            };
                            self.flag(
                                C,
                                if subtract {
                                    left < right
                                } else {
                                    u32::from(left) + u32::from(right) > 65535
                                },
                            );
                            self.flag(
                                V,
                                if subtract {
                                    (left ^ right) & (left ^ out) & 0x8000 != 0
                                } else {
                                    (!(left ^ right)) & (left ^ out) & 0x8000 != 0
                                },
                            );
                            out
                        } else {
                            self.flag(V, false);
                            right
                        };
                        self.nz16(out);
                        if operation != 12 && !(page != 0 && operation == 3) {
                            match (page, b, operation) {
                                (0, _, 3) | (0, true, 12) => self.registers.set_d(out),
                                (0, false, 14) => self.registers.x = out,
                                (0, true, 14) => self.registers.u = out,
                                (0x10, false, 14) => self.registers.y = out,
                                (0x10, true, 14) => {
                                    self.registers.s = out;
                                    self.nmi_armed = true;
                                }
                                _ => {}
                            }
                        }
                        // LDD has operation $C but is a load rather than a compare.
                        if page == 0 && b && operation == 12 {
                            self.registers.set_d(out);
                        }
                        3 + extra + u32::from(arithmetic) + u32::from(page != 0)
                    }
                } else {
                    if page != 0 {
                        return Err(fail);
                    }
                    let left = if b {
                        self.registers.b
                    } else {
                        self.registers.a
                    };
                    if operation == 7 {
                        self.write(bus, ea, left);
                        self.nz8(left);
                        self.flag(V, false);
                        2 + extra
                    } else {
                        let right = self.read(bus, ea);
                        let carry = self.registers.cc & C != 0;
                        let out = match operation {
                            0 | 1 => self.arithmetic8(left, right, true, false),
                            2 => self.arithmetic8(left, right, true, carry),
                            4 | 5 => {
                                let out = left & right;
                                self.nz8(out);
                                self.flag(V, false);
                                out
                            }
                            6 => {
                                self.nz8(right);
                                self.flag(V, false);
                                right
                            }
                            8 => {
                                let out = left ^ right;
                                self.nz8(out);
                                self.flag(V, false);
                                out
                            }
                            9 => self.arithmetic8(left, right, false, carry),
                            10 => {
                                let out = left | right;
                                self.nz8(out);
                                self.flag(V, false);
                                out
                            }
                            11 => self.arithmetic8(left, right, false, false),
                            _ => return Err(fail),
                        };
                        if !matches!(operation, 1 | 5) {
                            if b {
                                self.registers.b = out;
                            } else {
                                self.registers.a = out;
                            }
                        }
                        2 + extra
                    }
                }
            }
        } else if page == 0 && (op <= 0x0f || (0x40..=0x7f).contains(&op)) {
            let accumulator = (0x40..=0x5f).contains(&op);
            let b = op & 16 != 0;
            let (ea, extra) = if accumulator {
                (0, 0)
            } else {
                match op >> 4 {
                    0 => {
                        let lo = self.fetch(bus);
                        self.idle(bus, 1);
                        (u16::from_be_bytes([self.registers.dp, lo]), 0)
                    }
                    6 => self.index(bus, address)?,
                    _ => {
                        let ea = self.fetch_word(bus);
                        self.idle(bus, 1);
                        (ea, 1)
                    }
                }
            };
            if op & 15 == 14 && !accumulator {
                self.registers.pc = ea;
                3 + extra
            } else {
                let value = if accumulator {
                    if b {
                        self.registers.b
                    } else {
                        self.registers.a
                    }
                } else {
                    self.read(bus, ea)
                };
                let out = self.unary(op & 15, value).ok_or(fail)?;
                if op & 15 != 13 {
                    if accumulator {
                        if b {
                            self.registers.b = out;
                        } else {
                            self.registers.a = out;
                        }
                    } else {
                        self.idle(bus, 1);
                        self.write(bus, ea, out);
                    }
                }
                if accumulator { 2 } else { 6 + extra }
            }
        } else if page == 0 {
            match op {
                0x12 => 2,
                0x13 => {
                    self.waiting = Wait::Sync;
                    4
                }
                0x16 | 0x17 => {
                    let relative = self.fetch_word(bus);
                    if op == 0x17 {
                        self.push16(bus, false, self.registers.pc);
                    }
                    self.registers.pc = self.registers.pc.wrapping_add(relative);
                    if op == 0x17 { 9 } else { 5 }
                }
                0x1a => {
                    let mask = self.fetch(bus);
                    self.registers.cc |= mask;
                    3
                }
                0x1c => {
                    let mask = self.fetch(bus);
                    self.registers.cc &= mask;
                    3
                }
                0x1d => {
                    self.registers.a = if self.registers.b & 0x80 != 0 { 255 } else { 0 };
                    self.nz16(self.registers.d());
                    2
                }
                0x19 => {
                    let a = self.registers.a;
                    let old_carry = self.registers.cc & C != 0;
                    let low = a & 15;
                    let correction = if low > 9 || self.registers.cc & H != 0 {
                        6_u16
                    } else {
                        0
                    } + if old_carry || a > 0x99 || a & 0xf0 > 0x80 && low > 9 {
                        0x60
                    } else {
                        0
                    };
                    let adjusted = u16::from(a) + correction;
                    self.registers.a = adjusted.to_be_bytes()[1];
                    self.flag(C, old_carry || adjusted > 255);
                    self.nz8(self.registers.a);
                    2
                }
                0x1e | 0x1f => {
                    let post = self.fetch(bus);
                    let src = post >> 4;
                    let dst = post & 15;
                    let read = |r: Registers, id: u8| match id {
                        0 => Some(r.d()),
                        1 => Some(r.x),
                        2 => Some(r.y),
                        3 => Some(r.u),
                        4 => Some(r.s),
                        5 => Some(r.pc),
                        8 => Some(u16::from(r.a)),
                        9 => Some(u16::from(r.b)),
                        10 => Some(u16::from(r.cc)),
                        11 => Some(u16::from(r.dp)),
                        _ => None,
                    };
                    if (src < 8) != (dst < 8) {
                        return Err(fail);
                    }
                    let value = read(self.registers, src).ok_or(fail)?;
                    let other = read(self.registers, dst).ok_or(fail)?;
                    self.set_register(dst, value);
                    if op == 0x1e {
                        self.set_register(src, other);
                        8
                    } else {
                        6
                    }
                }
                0x30..=0x33 => {
                    let (ea, extra) = self.index(bus, address)?;
                    match op {
                        0x30 => {
                            self.registers.x = ea;
                            self.flag(Z, ea == 0);
                        }
                        0x31 => {
                            self.registers.y = ea;
                            self.flag(Z, ea == 0);
                        }
                        0x32 => {
                            self.registers.s = ea;
                            self.nmi_armed = true;
                        }
                        _ => self.registers.u = ea,
                    }
                    4 + extra
                }
                0x34..=0x37 => {
                    let mask = self.fetch(bus);
                    let user = op & 2 != 0;
                    let pop = op & 1 != 0;
                    let mut bytes = 0;
                    if pop {
                        for bit in 0..8 {
                            if mask & (1 << bit) != 0 {
                                match bit {
                                    0 => self.registers.cc = self.pop8(bus, user),
                                    1 => self.registers.a = self.pop8(bus, user),
                                    2 => self.registers.b = self.pop8(bus, user),
                                    3 => self.registers.dp = self.pop8(bus, user),
                                    4 => self.registers.x = self.pop16(bus, user),
                                    5 => self.registers.y = self.pop16(bus, user),
                                    6 => {
                                        let value = self.pop16(bus, user);
                                        if user {
                                            self.registers.s = value;
                                        } else {
                                            self.registers.u = value;
                                        }
                                    }
                                    _ => self.registers.pc = self.pop16(bus, user),
                                }
                                bytes += if bit < 4 { 1 } else { 2 };
                            }
                        }
                    } else {
                        for bit in (0..8).rev() {
                            if mask & (1 << bit) != 0 {
                                if bit < 4 {
                                    let value = match bit {
                                        0 => self.registers.cc,
                                        1 => self.registers.a,
                                        2 => self.registers.b,
                                        _ => self.registers.dp,
                                    };
                                    self.push8(bus, user, value);
                                    bytes += 1;
                                } else {
                                    let value = match bit {
                                        4 => self.registers.x,
                                        5 => self.registers.y,
                                        6 => {
                                            if user {
                                                self.registers.s
                                            } else {
                                                self.registers.u
                                            }
                                        }
                                        _ => self.registers.pc,
                                    };
                                    self.push16(bus, user, value);
                                    bytes += 2;
                                }
                            }
                        }
                    }
                    5 + bytes
                }
                0x39 => {
                    self.registers.pc = self.pop16(bus, false);
                    5
                }
                0x3b => {
                    self.registers.cc = self.pop8(bus, false);
                    let full = self.registers.cc & E != 0;
                    if full {
                        self.registers.a = self.pop8(bus, false);
                        self.registers.b = self.pop8(bus, false);
                        self.registers.dp = self.pop8(bus, false);
                        self.registers.x = self.pop16(bus, false);
                        self.registers.y = self.pop16(bus, false);
                        self.registers.u = self.pop16(bus, false);
                    }
                    self.registers.pc = self.pop16(bus, false);
                    if full { 15 } else { 6 }
                }
                0x3c => {
                    let mask = self.fetch(bus);
                    self.registers.cc &= mask;
                    self.full_frame(bus);
                    self.waiting = Wait::Cwai;
                    20
                }
                0x3f => {
                    self.full_frame(bus);
                    self.registers.cc |= I | F;
                    self.registers.pc = self.word(bus, 0xfffa);
                    19
                }
                0x3a => {
                    self.registers.x = self.registers.x.wrapping_add(u16::from(self.registers.b));
                    3
                }
                0x3d => {
                    let value = u16::from(self.registers.a) * u16::from(self.registers.b);
                    self.registers.set_d(value);
                    self.flag(Z, value == 0);
                    self.flag(C, value & 0x80 != 0);
                    11
                }
                _ => return Err(fail),
            }
        } else {
            return Err(fail);
        };
        let cycles = cycles.max(self.bus_cycles);
        self.idle(bus, cycles - self.bus_cycles);
        Ok(cycles)
    }
    fn set_register(&mut self, id: u8, value: u16) {
        let byte = value.to_be_bytes()[1];
        match id {
            0 => self.registers.set_d(value),
            1 => self.registers.x = value,
            2 => self.registers.y = value,
            3 => self.registers.u = value,
            4 => self.registers.s = value,
            5 => self.registers.pc = value,
            8 => self.registers.a = byte,
            9 => self.registers.b = byte,
            10 => self.registers.cc = byte,
            11 => self.registers.dp = byte,
            _ => {}
        }
    }
}

fn documented_opcode(page: u8, op: u8) -> bool {
    match page {
        0 => match op {
            0x00..=0x0f | 0x40..=0x7f => {
                matches!(op & 15, 0 | 3 | 4 | 6..=10 | 12 | 13 | 15)
                    || op & 15 == 14 && !(0x40..=0x5f).contains(&op)
            }
            0x12
            | 0x13
            | 0x16
            | 0x17
            | 0x19
            | 0x1a
            | 0x1c..=0x1f
            | 0x20..=0x37
            | 0x39..=0x3d
            | 0x3f => true,
            0x80..=0xff => !matches!(op, 0x87 | 0x8f | 0xc7 | 0xcd | 0xcf),
            _ => false,
        },
        0x10 => matches!(
            op,
            0x21..=0x2f
                | 0x3f
                | 0x83
                | 0x93
                | 0xa3
                | 0xb3
                | 0x8c
                | 0x9c
                | 0xac
                | 0xbc
                | 0x8e
                | 0x9e
                | 0xae
                | 0xbe
                | 0x9f
                | 0xaf
                | 0xbf
                | 0xce
                | 0xde
                | 0xee
                | 0xfe
                | 0xdf
                | 0xef
                | 0xff
        ),
        0x11 => matches!(
            op,
            0x3f | 0x83 | 0x93 | 0xa3 | 0xb3 | 0x8c | 0x9c | 0xac | 0xbc
        ),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Memory {
        bytes: Vec<u8>,
        cycles: u64,
    }
    impl Bus for Memory {
        fn read(&mut self, a: u16) -> u8 {
            self.bytes[usize::from(a)]
        }
        fn write(&mut self, a: u16, v: u8) {
            self.bytes[usize::from(a)] = v;
        }
        fn tick(&mut self, n: u32) {
            self.cycles += u64::from(n);
        }
    }
    fn run(code: &[u8]) -> (Cpu, Memory) {
        let mut bus = Memory {
            bytes: vec![0; 65536],
            cycles: 0,
        };
        bus.bytes[..code.len()].copy_from_slice(code);
        let mut cpu = Cpu::default();
        while usize::from(cpu.registers.pc) < code.len() {
            assert!(cpu.step(&mut bus).is_ok());
        }
        (cpu, bus)
    }
    #[test]
    fn arithmetic_flags_and_big_endian_words() {
        let (c, b) = run(&[
            0x86, 0x7f, 0x8b, 1, 0xb7, 0x20, 0, 0xcc, 0x12, 0x34, 0xfd, 0x20, 1,
        ]);
        assert_eq!(b.bytes[0x2000], 0x80);
        assert_eq!(&b.bytes[0x2001..0x2003], &[0x12, 0x34]);
        assert_eq!(c.registers.d(), 0x1234);
        let (c, _) = run(&[0x86, 0xff, 0x8b, 1]);
        assert_eq!(c.registers.a, 0);
        assert_eq!(c.registers.cc & (C | Z | H), C | Z | H);
    }
    #[test]
    fn stack_and_indexed_postincrement() {
        let (c, b) = run(&[
            0x10, 0xce, 0x70, 0, 0x8e, 0x20, 0, 0x86, 0x41, 0xa7, 0x80, 0x34, 0x12, 0x4f, 0x8e, 0,
            0, 0x35, 0x12,
        ]);
        assert_eq!(c.registers.a, 0x41);
        assert_eq!(c.registers.x, 0x2001);
        assert_eq!(c.registers.s, 0x7000);
        assert_eq!(b.bytes[0x2000], 0x41);
    }
    #[test]
    fn branch_conditions_and_instruction_cycles() {
        let (c, b) = run(&[0x86, 0, 0x27, 2, 0x86, 0xff, 0x12]);
        assert_eq!(c.registers.a, 0);
        assert_eq!(b.cycles, 7);
    }
    #[test]
    fn unsupported_is_explicit() {
        let mut bus = Memory {
            bytes: vec![0; 65536],
            cycles: 0,
        };
        bus.bytes[0] = 0x02;
        let mut cpu = Cpu::default();
        assert_eq!(
            cpu.step(&mut bus),
            Err(Unsupported {
                address: 0,
                opcode: 2
            })
        );
    }
    #[test]
    fn register_sizes_and_address_wrap() {
        let (c, _) = run(&[0xcc, 0xab, 0xcd, 0x1f, 0x01]);
        assert_eq!(c.registers.x, 0xabcd);
        let mut cpu = Cpu::default();
        let mut bus = Memory {
            bytes: vec![0; 65536],
            cycles: 0,
        };
        bus.bytes[0xffff] = 0x86;
        bus.bytes[0] = 0x42;
        cpu.registers.pc = 0xffff;
        assert!(cpu.step(&mut bus).is_ok());
        assert_eq!(cpu.registers.pc, 1);
        assert_eq!(cpu.registers.a, 0x42);
    }
}
