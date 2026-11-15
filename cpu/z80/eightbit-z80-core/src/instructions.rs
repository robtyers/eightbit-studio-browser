//! Opcode bit fields follow the manufacturer's encodings, not a reference core.
#![allow(clippy::many_single_char_names)] // Conventional x/y/z/p/q opcode fields.
use super::{Bus, C, Cpu, H, Index, N, PV, S, X, Y, Z, parity, sz53, szp53};

impl Cpu {
    pub(super) fn base(&mut self, bus: &mut impl Bus, opcode: u8, index: Index) {
        let x = opcode >> 6;
        let y = (opcode >> 3) & 7;
        let z = opcode & 7;
        let p = y >> 1;
        let q = y & 1;
        match x {
            0 => match z {
                0 => match y {
                    0 => {}
                    1 => {
                        std::mem::swap(&mut self.registers.a, &mut self.registers.alternate[0]);
                        std::mem::swap(&mut self.registers.f, &mut self.registers.alternate[1]);
                    }
                    2 => {
                        self.idle(bus, 1);
                        let offset = self.next(bus);
                        self.registers.b = self.registers.b.wrapping_sub(1);
                        if self.registers.b != 0 {
                            self.branch(bus, offset);
                        }
                    }
                    _ => {
                        let offset = self.next(bus);
                        if y == 3 || self.condition(y - 4) {
                            self.branch(bus, offset);
                        }
                    }
                },
                1 if q == 0 => {
                    let value = self.next_word(bus);
                    self.set_pair(p, index, value);
                }
                1 => {
                    self.idle(bus, 7);
                    self.add_pair(p, index);
                }
                2 => {
                    let address = if p <= 1 {
                        self.pair(p, Index::Hl)
                    } else {
                        self.next_word(bus)
                    };
                    if p == 2 {
                        if q == 0 {
                            self.put_word(bus, address, self.pair(2, index));
                        } else {
                            let value = self.word(bus, address);
                            self.set_pair(2, index, value);
                        }
                        self.registers.wz = address.wrapping_add(1);
                    } else if q == 0 {
                        self.write(bus, address, self.registers.a);
                        self.registers.wz =
                            u16::from_be_bytes([self.registers.a, address.wrapping_add(1) as u8]);
                    } else {
                        self.registers.a = self.read(bus, address);
                        self.registers.wz = address.wrapping_add(1);
                    }
                }
                3 => {
                    let old = self.pair(p, index);
                    self.set_pair(
                        p,
                        index,
                        if q == 0 {
                            old.wrapping_add(1)
                        } else {
                            old.wrapping_sub(1)
                        },
                    );
                    self.idle(bus, 2);
                }
                4 | 5 => {
                    if y == 6 {
                        let address = self.address(bus, index, 5);
                        let value = self.read(bus, address);
                        let result = if z == 4 {
                            self.inc(value)
                        } else {
                            self.dec(value)
                        };
                        self.idle(bus, 1);
                        self.write(bus, address, result);
                    } else {
                        let old = self.reg(y, index);
                        let value = if z == 4 { self.inc(old) } else { self.dec(old) };
                        self.set_reg(y, index, value);
                    }
                }
                6 => {
                    if y == 6 {
                        let address = self.address(bus, index, 0);
                        let value = self.next(bus);
                        if index != Index::Hl {
                            self.idle(bus, 2);
                        }
                        self.write(bus, address, value);
                    } else {
                        let value = self.next(bus);
                        self.set_reg(y, index, value);
                    }
                }
                _ => self.accumulator(y),
            },
            1 if opcode == 0x76 => self.halted = true,
            1 if y == 6 || z == 6 => {
                let address = self.address(bus, index, 5);
                // H/L alongside an indexed memory operand are the real H/L,
                // whereas register-only DD/FD forms use the index halves.
                if z == 6 {
                    let value = self.read(bus, address);
                    self.set_reg(y, Index::Hl, value);
                } else {
                    self.write(bus, address, self.reg(z, Index::Hl));
                }
            }
            1 => self.set_reg(y, index, self.reg(z, index)),
            2 => {
                let value = if z == 6 {
                    let address = self.address(bus, index, 5);
                    self.read(bus, address)
                } else {
                    self.reg(z, index)
                };
                self.alu(y, value);
            }
            _ => match z {
                0 => {
                    self.idle(bus, 1);
                    if self.condition(y) {
                        self.registers.pc = self.pop(bus);
                        self.registers.wz = self.registers.pc;
                    }
                }
                1 if q == 0 => {
                    let value = self.pop(bus);
                    if p == 3 {
                        self.registers.set_af(value);
                    } else {
                        self.set_pair(p, index, value);
                    }
                }
                1 => match p {
                    0 => {
                        self.registers.pc = self.pop(bus);
                        self.registers.wz = self.registers.pc;
                    }
                    1 => {
                        std::mem::swap(&mut self.registers.b, &mut self.registers.alternate[2]);
                        std::mem::swap(&mut self.registers.c, &mut self.registers.alternate[3]);
                        std::mem::swap(&mut self.registers.d, &mut self.registers.alternate[4]);
                        std::mem::swap(&mut self.registers.e, &mut self.registers.alternate[5]);
                        std::mem::swap(&mut self.registers.h, &mut self.registers.alternate[6]);
                        std::mem::swap(&mut self.registers.l, &mut self.registers.alternate[7]);
                    }
                    2 => self.registers.pc = self.pair(2, index),
                    _ => {
                        self.registers.sp = self.pair(2, index);
                        self.idle(bus, 2);
                    }
                },
                2 => {
                    let address = self.next_word(bus);
                    self.registers.wz = address;
                    if self.condition(y) {
                        self.registers.pc = address;
                    }
                }
                3 => match y {
                    0 => {
                        self.registers.pc = self.next_word(bus);
                        self.registers.wz = self.registers.pc;
                    }
                    2 | 3 => {
                        let low = self.next(bus);
                        let port = u16::from_be_bytes([self.registers.a, low]);
                        if y == 2 {
                            self.output(bus, port, self.registers.a);
                            self.registers.wz =
                                u16::from_be_bytes([self.registers.a, low.wrapping_add(1)]);
                        } else {
                            self.registers.a = self.input(bus, port);
                            self.registers.wz = port.wrapping_add(1);
                        }
                    }
                    4 => {
                        let old = self.pair(2, index);
                        let value = self.word(bus, self.registers.sp);
                        self.idle(bus, 1);
                        // The exchange writes high first, unlike LD (nn),rr.
                        self.write(bus, self.registers.sp.wrapping_add(1), (old >> 8) as u8);
                        self.write(bus, self.registers.sp, old as u8);
                        self.idle(bus, 2);
                        self.set_pair(2, index, value);
                        self.registers.wz = value;
                    }
                    5 => {
                        let de = self.registers.de();
                        self.registers.set_de(self.registers.hl());
                        self.registers.set_hl(de);
                    }
                    6 => {
                        self.registers.iff1 = false;
                        self.registers.iff2 = false;
                        self.ei_delay = 0;
                    }
                    7 => {
                        self.registers.iff1 = true;
                        self.registers.iff2 = true;
                        self.ei_delay = 2;
                    }
                    _ => {} // Prefixes are consumed by step, not by this decoder.
                },
                4 => {
                    let address = self.next_word(bus);
                    self.registers.wz = address;
                    if self.condition(y) {
                        self.idle(bus, 1);
                        self.push(bus, self.registers.pc);
                        self.registers.pc = address;
                    }
                }
                5 if q == 0 => {
                    self.idle(bus, 1);
                    let value = if p == 3 {
                        self.registers.af()
                    } else {
                        self.pair(p, index)
                    };
                    self.push(bus, value);
                }
                5 if p == 0 => {
                    let address = self.next_word(bus);
                    self.idle(bus, 1);
                    self.push(bus, self.registers.pc);
                    self.registers.pc = address;
                    self.registers.wz = address;
                }
                6 => {
                    let value = self.next(bus);
                    self.alu(y, value);
                }
                7 => {
                    self.idle(bus, 1);
                    self.push(bus, self.registers.pc);
                    self.registers.pc = u16::from(y) * 8;
                    self.registers.wz = self.registers.pc;
                }
                _ => {}
            },
        }
    }
    fn accumulator(&mut self, operation: u8) {
        let a = self.registers.a;
        let old_flags = self.registers.f;
        match operation {
            0..=3 => {
                let carry = if operation & 1 == 0 { a >> 7 } else { a & 1 };
                self.registers.a = match operation {
                    0 => a.rotate_left(1),
                    1 => a.rotate_right(1),
                    2 => (a << 1) | (old_flags & C),
                    _ => (a >> 1) | ((old_flags & C) << 7),
                };
                self.registers.f =
                    (old_flags & (S | Z | PV)) | (self.registers.a & (X | Y)) | carry;
            }
            4 => self.daa(),
            5 => {
                self.registers.a = !a;
                self.registers.f =
                    (old_flags & (S | Z | PV | C)) | H | N | (self.registers.a & (X | Y));
            }
            6 => self.registers.f = (old_flags & (S | Z | PV)) | (a & (X | Y)) | C,
            _ => {
                self.registers.f = (old_flags & (S | Z | PV))
                    | (a & (X | Y))
                    | if old_flags & C == 0 { C } else { H }
            }
        }
    }
    pub(super) fn cb(&mut self, bus: &mut impl Bus, index: Index) {
        let indexed = index != Index::Hl;
        let address = if indexed {
            self.address(bus, index, 0)
        } else {
            self.registers.hl()
        };
        let opcode = if indexed {
            self.next(bus)
        } else {
            self.m1(bus)
        };
        let register = opcode & 7;
        let operation = (opcode >> 3) & 7;
        let group = opcode >> 6;
        let memory = indexed || register == 6;
        if indexed {
            self.idle(bus, 2);
        }
        let value = if memory {
            self.read(bus, address)
        } else {
            self.reg(register, Index::Hl)
        };
        if memory {
            self.idle(bus, 1);
        }
        let result = match group {
            0 => {
                let carry = if matches!(operation, 0 | 2 | 4 | 6) {
                    value >> 7
                } else {
                    value & 1
                };
                let result = match operation {
                    0 => value.rotate_left(1),
                    1 => value.rotate_right(1),
                    2 => (value << 1) | (self.registers.f & C),
                    3 => (value >> 1) | ((self.registers.f & C) << 7),
                    4 => value << 1,
                    5 => (value >> 1) | (value & 0x80),
                    6 => (value << 1) | 1,
                    _ => value >> 1,
                };
                self.registers.f = szp53(result) | carry;
                result
            }
            1 => {
                let mask = 1 << operation;
                let clear = value & mask == 0;
                let xy = if memory {
                    (self.registers.wz >> 8) as u8
                } else {
                    value
                };
                self.registers.f = (self.registers.f & C)
                    | H
                    | (xy & (X | Y))
                    | if clear {
                        Z | PV
                    } else if operation == 7 {
                        S
                    } else {
                        0
                    };
                return;
            }
            2 => value & !(1 << operation),
            _ => value | (1 << operation),
        };
        if memory {
            self.write(bus, address, result);
        }
        if register != 6 {
            self.set_reg(register, Index::Hl, result);
        }
    }
    pub(super) fn ed(&mut self, bus: &mut impl Bus, opcode: u8) {
        let y = (opcode >> 3) & 7;
        let z = opcode & 7;
        let p = y >> 1;
        let q = y & 1;
        if opcode & 0xc0 == 0x40 {
            match z {
                0 => {
                    let port = self.registers.bc();
                    let value = self.input(bus, port);
                    if y != 6 {
                        self.set_reg(y, Index::Hl, value);
                    }
                    self.registers.f = (self.registers.f & C) | szp53(value);
                    self.registers.wz = port.wrapping_add(1);
                }
                1 => {
                    let port = self.registers.bc();
                    let value = if y == 6 { 0 } else { self.reg(y, Index::Hl) };
                    self.output(bus, port, value);
                    self.registers.wz = port.wrapping_add(1);
                }
                2 => {
                    self.idle(bus, 7);
                    self.adc_pair(p, q == 0);
                }
                3 => {
                    let address = self.next_word(bus);
                    if q == 0 {
                        self.put_word(bus, address, self.pair(p, Index::Hl));
                    } else {
                        let value = self.word(bus, address);
                        self.set_pair(p, Index::Hl, value);
                    }
                    self.registers.wz = address.wrapping_add(1);
                }
                4 => {
                    let value = self.registers.a;
                    self.registers.a = 0;
                    self.registers.a = self.arithmetic(value, true, 0);
                }
                5 => {
                    self.registers.pc = self.pop(bus);
                    self.registers.wz = self.registers.pc;
                    self.registers.iff1 = self.registers.iff2;
                    if opcode == 0x4d {
                        bus.reti();
                    }
                }
                6 => self.registers.interrupt_mode = [0, 0, 1, 2, 0, 0, 1, 2][usize::from(y)],
                _ => match y {
                    0 | 1 => {
                        if y == 0 {
                            self.registers.i = self.registers.a;
                        } else {
                            self.registers.r = self.registers.a;
                        }
                        self.idle(bus, 1);
                    }
                    2 | 3 => {
                        self.registers.a = if y == 2 {
                            self.registers.i
                        } else {
                            self.registers.r
                        };
                        self.registers.f = (self.registers.f & C)
                            | sz53(self.registers.a)
                            | if self.registers.iff2 { PV } else { 0 };
                        self.idle(bus, 1);
                    }
                    4 | 5 => {
                        let address = self.registers.hl();
                        let value = self.read(bus, address);
                        let a = self.registers.a;
                        let (new_a, new_mem) = if y == 4 {
                            ((a & 0xf0) | (value & 15), (a << 4) | (value >> 4))
                        } else {
                            ((a & 0xf0) | (value >> 4), (value << 4) | (a & 15))
                        };
                        self.idle(bus, 4);
                        self.write(bus, address, new_mem);
                        self.registers.a = new_a;
                        self.registers.f = (self.registers.f & C) | szp53(new_a);
                        self.registers.wz = address.wrapping_add(1);
                    }
                    _ => {}
                },
            }
        } else if opcode & 0xe4 == 0xa0 {
            self.block(bus, z, opcode & 8 != 0, opcode & 0x10 != 0);
        }
        // Unassigned ED encodings consume their two M1 cycles as a hardware NOP.
    }
    fn block(&mut self, bus: &mut impl Bus, operation: u8, decrement: bool, repeat: bool) {
        let direction: i16 = if decrement { -1 } else { 1 };
        let hl = self.registers.hl();
        let bc = self.registers.bc();
        let again = match operation {
            0 => {
                let value = self.read(bus, hl);
                self.write(bus, self.registers.de(), value);
                self.idle(bus, 2);
                self.registers.set_hl(hl.wrapping_add_signed(direction));
                self.registers
                    .set_de(self.registers.de().wrapping_add_signed(direction));
                self.registers.set_bc(bc.wrapping_sub(1));
                let sum = self.registers.a.wrapping_add(value);
                self.registers.f = (self.registers.f & (S | Z | C))
                    | (sum & X)
                    | ((sum & 2) << 4)
                    | if self.registers.bc() != 0 { PV } else { 0 };
                self.registers.bc() != 0
            }
            1 => {
                let value = self.read(bus, hl);
                self.idle(bus, 5);
                let a = self.registers.a;
                let difference = a.wrapping_sub(value);
                let half = (a ^ value ^ difference) & H;
                let adjusted = difference.wrapping_sub(u8::from(half != 0));
                self.registers.set_hl(hl.wrapping_add_signed(direction));
                self.registers.set_bc(bc.wrapping_sub(1));
                self.registers.f = (self.registers.f & C)
                    | N
                    | half
                    | (sz53(difference) & (S | Z))
                    | (adjusted & X)
                    | ((adjusted & 2) << 4)
                    | if self.registers.bc() != 0 { PV } else { 0 };
                self.registers.wz = self.registers.wz.wrapping_add_signed(direction);
                self.registers.bc() != 0 && difference != 0
            }
            2 | 3 => {
                self.idle(bus, 1);
                let value = if operation == 2 {
                    let byte = self.input(bus, bc);
                    self.write(bus, hl, byte);
                    byte
                } else {
                    let byte = self.read(bus, hl);
                    self.registers.b = self.registers.b.wrapping_sub(1);
                    self.output(bus, self.registers.bc(), byte);
                    byte
                };
                if operation == 2 {
                    self.registers.b = self.registers.b.wrapping_sub(1);
                }
                self.registers.set_hl(hl.wrapping_add_signed(direction));
                let addend = if operation == 2 {
                    self.registers
                        .c
                        .wrapping_add_signed(if decrement { -1 } else { 1 })
                } else {
                    self.registers.l
                };
                let sum = u16::from(value) + u16::from(addend);
                self.registers.f = sz53(self.registers.b)
                    | if value & 0x80 != 0 { N } else { 0 }
                    | if sum > 255 { H | C } else { 0 }
                    | if parity((sum as u8 & 7) ^ self.registers.b) {
                        PV
                    } else {
                        0
                    };
                self.registers.wz = if operation == 2 {
                    bc
                } else {
                    self.registers.bc()
                }
                .wrapping_add_signed(direction);
                self.registers.b != 0
            }
            _ => false,
        };
        if repeat && again {
            self.idle(bus, 5);
            self.registers.pc = self.registers.pc.wrapping_sub(2);
            self.registers.wz = self.registers.pc.wrapping_add(1);
        }
    }
}
