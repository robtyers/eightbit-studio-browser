//! Original 6526 functional digital model; register facts from the manufacturer sheet.
//! Silicon-revision ICR race errata and analogue pin delays are not modelled.

#[derive(Clone, Copy, Debug)]
struct Timer {
    latch: u16,
    counter: u16,
    control: u8,
    toggle: bool,
    pulse: bool,
}
impl Default for Timer {
    fn default() -> Self {
        Self {
            latch: 0xffff,
            counter: 0xffff,
            control: 0,
            toggle: true,
            pulse: false,
        }
    }
}
impl Timer {
    fn tick(&mut self, clock: bool) -> bool {
        if self.control & 1 == 0 || !clock {
            return false;
        }
        if self.counter != 0 {
            self.counter -= 1;
            return false;
        }
        self.counter = self.latch;
        self.toggle = !self.toggle;
        self.pulse = true;
        if self.control & 8 != 0 {
            self.control &= !1;
        }
        true
    }
    fn write(&mut self, high: bool, value: u8) {
        let mut bytes = self.latch.to_le_bytes();
        bytes[usize::from(high)] = value;
        self.latch = u16::from_le_bytes(bytes);
        if high && self.control & 1 == 0 {
            self.counter = self.latch;
        }
    }
    fn control(&mut self, value: u8) {
        if value & 0x10 != 0 {
            self.counter = self.latch;
        }
        if value & 1 != 0 && self.control & 1 == 0 {
            self.toggle = true;
        }
        self.control = value & !0x10;
    }
    fn output(self) -> bool {
        if self.control & 4 != 0 {
            self.toggle
        } else {
            self.pulse
        }
    }
}

/// One independently clocked CIA. No key interpretation or host I/O occurs here.
#[derive(Clone, Debug)]
// Independent physical pins/flip-flops are intentionally represented as booleans.
#[allow(clippy::struct_excessive_bools)]
pub struct Cia {
    ports: [u8; 2],
    ddr: [u8; 2],
    timers: [Timer; 2],
    pending: u8,
    mask: u8,
    tod: [u8; 4],
    alarm: [u8; 4],
    held: Option<[u8; 4]>,
    tod_divider: u8,
    tod_running: bool,
    serial: u8,
    shift: u8,
    bits: u8,
    buffered: Option<u8>,
    cnt: bool,
    sp: bool,
    flag: bool,
    cnt_out: bool,
    sp_out: bool,
    pc: bool,
}
impl Default for Cia {
    fn default() -> Self {
        Self {
            ports: [0xff; 2],
            ddr: [0; 2],
            timers: [Timer::default(); 2],
            pending: 0,
            mask: 0,
            tod: [0, 0, 0, 1],
            alarm: [0; 4],
            held: None,
            tod_divider: 0,
            tod_running: false,
            serial: 0,
            shift: 0,
            bits: 0,
            buffered: None,
            cnt: true,
            sp: true,
            flag: true,
            cnt_out: false,
            sp_out: true,
            pc: true,
        }
    }
}
impl Cia {
    /// CIA interrupt output; reading ICR acknowledges every pending source.
    #[must_use]
    pub const fn irq(&self) -> bool {
        self.pending & self.mask & 0x1f != 0
    }
    /// Released/output-high port pins before external wiring pulls them low.
    #[must_use]
    pub fn driven_port(&self, port: usize) -> u8 {
        let port = port & 1;
        let mut value = self.ports[port] | !self.ddr[port];
        if port == 1 {
            for (index, bit) in [(0, 6), (1, 7)] {
                if self.timers[index].control & 2 != 0 {
                    value = (value & !(1 << bit)) | (u8::from(self.timers[index].output()) << bit);
                }
            }
        }
        value
    }
    /// Serial CNT/SP output levels. The board owns the physical connection.
    #[must_use]
    pub const fn serial_outputs(&self) -> (bool, bool) {
        (self.cnt_out, self.sp_out)
    }
    /// Active-low parallel handshake strobe, pulsed by a port-B access.
    #[must_use]
    pub const fn parallel_strobe(&self) -> bool {
        self.pc
    }
    /// Present CNT/SP and FLAG inputs; rising CNT clocks serial input and timers.
    pub fn pins(&mut self, cnt: bool, sp: bool, flag: bool) {
        if self.flag && !flag {
            self.pending |= 0x10;
        }
        let rising = !self.cnt && cnt;
        self.cnt = cnt;
        if rising {
            if self.timers[0].control & 0x40 == 0 {
                self.shift = (self.shift << 1) | u8::from(sp);
                self.bits += 1;
                if self.bits >= 8 {
                    self.serial = self.shift;
                    self.bits = 0;
                    self.pending |= 8;
                }
            }
            self.tick_timers(true, false);
        }
        self.cnt = cnt;
        self.sp = sp;
        self.flag = flag;
    }
    /// Advance one PHI2 cycle (also during VIC bus ownership).
    pub fn tick(&mut self) {
        self.pc = true;
        self.tick_timers(false, true);
    }
    fn tick_timers(&mut self, cnt_edge: bool, phi2: bool) {
        if phi2 {
            for timer in &mut self.timers {
                timer.pulse = false;
            }
        }
        let source_a = if self.timers[0].control & 0x20 != 0 {
            cnt_edge
        } else {
            phi2
        };
        let a = self.timers[0].tick(source_a);
        let source_b = match (self.timers[1].control >> 5) & 3 {
            0 => phi2,
            1 => cnt_edge,
            2 => a,
            _ => a && self.cnt,
        };
        let b = self.timers[1].tick(source_b);
        self.pending |= u8::from(a) | (u8::from(b) << 1);
        if a && self.timers[0].control & 0x40 != 0 {
            if self.bits == 0
                && let Some(value) = self.buffered.take()
            {
                self.shift = value;
                self.bits = 8;
                self.sp_out = self.shift & 0x80 != 0;
            }
            if self.bits != 0 {
                self.cnt_out = !self.cnt_out;
                if !self.cnt_out {
                    self.shift <<= 1;
                    self.bits -= 1;
                    self.sp_out = self.shift & 0x80 != 0;
                    if self.bits == 0 {
                        self.pending |= 8;
                    }
                }
            }
        }
    }
    /// One mains-frequency pulse; CRA selects five or six pulses per tenth.
    pub fn tod_pulse(&mut self) {
        if !self.tod_running {
            return;
        }
        self.tod_divider += 1;
        let divisor = if self.timers[0].control & 0x80 != 0 {
            5
        } else {
            6
        };
        if self.tod_divider < divisor {
            return;
        }
        self.tod_divider = 0;
        if self.tod[0] >= 9 {
            self.tod[0] = 0;
            if increment_bcd(&mut self.tod[1], 60) && increment_bcd(&mut self.tod[2], 60) {
                let pm = self.tod[3] & 0x80;
                let hour = unbcd(self.tod[3] & 0x1f);
                self.tod[3] = match hour {
                    11 => 0x12 | (pm ^ 0x80),
                    12 => 1 | pm,
                    _ => bcd(hour + 1) | pm,
                };
            }
        } else {
            self.tod[0] += 1;
        }
        if self.tod == self.alarm {
            self.pending |= 4;
        }
    }
    /// Read a mirrored register; ports combine driven pins with external low pulls.
    pub fn read(&mut self, register: u8, inputs: [u8; 2]) -> u8 {
        let register = register & 15;
        if register == 1 {
            self.pc = false;
        }
        match register {
            0 | 1 => {
                self.driven_port(usize::from(register & 1)) & inputs[usize::from(register & 1)]
            }
            2 | 3 => self.ddr[usize::from(register & 1)],
            4..=7 => self.timers[usize::from((register - 4) / 2)]
                .counter
                .to_le_bytes()[usize::from(register & 1)],
            8..=11 => {
                if register == 11 && self.held.is_none() {
                    self.held = Some(self.tod);
                }
                let value = self.held.unwrap_or(self.tod)[usize::from(register - 8)];
                if register == 8 {
                    self.held = None;
                }
                value
            }
            12 => self.serial,
            13 => {
                let value = self.pending | if self.irq() { 0x80 } else { 0 };
                self.pending = 0;
                value
            }
            14 | 15 => self.timers[usize::from(register & 1)].control,
            _ => 0,
        }
    }
    /// Write a mirrored register. Mask set/clear uses ICR bit seven.
    pub fn write(&mut self, register: u8, value: u8) {
        let register = register & 15;
        if register == 1 {
            self.pc = false;
        }
        match register {
            0 | 1 => self.ports[usize::from(register & 1)] = value,
            2 | 3 => self.ddr[usize::from(register & 1)] = value,
            4..=7 => self.timers[usize::from((register - 4) / 2)].write(register & 1 != 0, value),
            8..=11 => {
                let index = usize::from(register - 8);
                let value = value & [0x0f, 0x7f, 0x7f, 0x9f][index];
                if self.timers[1].control & 0x80 != 0 {
                    self.alarm[index] = value;
                } else {
                    self.tod[index] = value;
                    if index == 3 {
                        self.tod_running = false;
                    }
                    if index == 0 {
                        self.tod_running = true;
                        self.tod_divider = 0;
                    }
                }
            }
            12 => {
                self.serial = value;
                if self.timers[0].control & 0x40 != 0 {
                    self.buffered = Some(value);
                }
            }
            13 => {
                if value & 0x80 != 0 {
                    self.mask |= value & 31;
                } else {
                    self.mask &= !(value & 31);
                }
            }
            14 | 15 => {
                if register == 14 && (self.timers[0].control ^ value) & 0x40 != 0 {
                    self.bits = 0;
                    self.buffered = None;
                    self.cnt_out = false;
                    self.sp_out = true;
                }
                self.timers[usize::from(register & 1)].control(value);
            }
            _ => {}
        }
    }
}
fn bcd(value: u8) -> u8 {
    (value / 10) * 16 + value % 10
}
fn unbcd(value: u8) -> u8 {
    (value >> 4) * 10 + (value & 15)
}
fn increment_bcd(value: &mut u8, limit: u8) -> bool {
    let next = unbcd(*value) + 1;
    if next >= limit {
        *value = 0;
        true
    } else {
        *value = bcd(next);
        false
    }
}
