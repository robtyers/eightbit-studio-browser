//! MC6821 logical register, edge and handshake model from Motorola DS9435.
#[derive(Clone, Copy)]
pub(crate) struct Pia {
    pub data: [u8; 2],
    pub ddr: [u8; 2],
    pub control: [u8; 2],
    c1: [bool; 2],
    c2: [bool; 2],
    output2: [bool; 2],
    restore: [u8; 2],
    selected: bool,
}
impl Default for Pia {
    fn default() -> Self {
        Self {
            data: [0; 2],
            ddr: [0; 2],
            control: [0; 2],
            c1: [true; 2],
            c2: [true; 2],
            output2: [true; 2],
            restore: [0; 2],
            selected: false,
        }
    }
}
impl Pia {
    pub fn pins(&self, side: usize, input: u8) -> u8 {
        if side == 0 {
            // Port A reads actual pins, including externally grounded outputs.
            (self.data[side] | !self.ddr[side]) & input
        } else {
            (self.data[side] & self.ddr[side]) | (input & !self.ddr[side])
        }
    }
    pub fn control_pin(&self, side: usize) -> bool {
        if self.control[side] & 0x20 == 0 {
            self.c2[side]
        } else {
            self.output2[side]
        }
    }
    pub fn driven_control_pin(&self, side: usize) -> Option<bool> {
        (self.control[side] & 0x20 != 0).then_some(self.output2[side])
    }
    pub fn irq(&self) -> bool {
        self.control
            .iter()
            .any(|&c| c & 0x81 == 0x81 || c & 0x68 == 0x48)
    }
    pub fn edge(&mut self, side: usize, second: bool, level: bool) {
        let control = self.control[side];
        let previous = if second { self.c2[side] } else { self.c1[side] };
        if previous != level && level == (control & if second { 0x10 } else { 2 } != 0) {
            if second {
                if control & 0x20 == 0 {
                    self.control[side] |= 0x40;
                }
            } else {
                self.control[side] |= 0x80;
                if control & 0x38 == 0x20 && (side == 0 || self.restore[side] == 3) {
                    self.output2[side] = true;
                    self.restore[side] = 0;
                }
            }
        }
        if second {
            self.c2[side] = level;
        } else {
            self.c1[side] = level;
        }
    }
    fn strobe(&mut self, side: usize) {
        match self.control[side] & 0x38 {
            0x20 => {
                self.output2[side] = false;
                self.restore[side] = if side == 0 { 0 } else { 2 };
            }
            0x28 => {
                self.output2[side] = false;
                self.restore[side] = 1;
            }
            _ => {}
        }
    }
    pub fn read(&mut self, register: usize, input: u8) -> u8 {
        self.selected = true;
        let side = register / 2;
        if register & 1 != 0 {
            self.control[side]
        } else if self.control[side] & 4 == 0 {
            self.ddr[side]
        } else {
            self.control[side] &= 0x3f;
            if side == 0 {
                self.strobe(side);
            } else if self.restore[side] == 2 {
                self.restore[side] = 3;
            }
            self.pins(side, input)
        }
    }
    pub fn write(&mut self, register: usize, value: u8) {
        self.selected = true;
        let side = register / 2;
        if register & 1 != 0 {
            self.control[side] = (self.control[side] & 0xc0) | (value & 0x3f);
            if value & 0x20 != 0 {
                self.control[side] &= !0x40;
                self.output2[side] = value & 0x10 == 0 || value & 8 != 0;
                self.restore[side] = 0;
            }
        } else if self.control[side] & 4 == 0 {
            self.ddr[side] = value;
        } else {
            self.data[side] = value;
            if side == 1 {
                self.strobe(side);
            }
        }
    }
    pub fn tick(&mut self) {
        if !self.selected {
            for side in 0..2 {
                if self.restore[side] == 1 {
                    self.output2[side] = true;
                    self.restore[side] = 0;
                }
            }
        }
        self.selected = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pulse_restore_requires_a_deselected_e_cycle_on_either_port() {
        let mut p = Pia::default();
        p.write(1, 0x2c);
        p.read(0, 255);
        p.tick();
        assert!(!p.control_pin(0));
        p.read(1, 255);
        p.tick();
        assert!(!p.control_pin(0));
        p.tick();
        assert!(p.control_pin(0));
        p.write(3, 0x2c);
        p.write(2, 127);
        p.tick();
        assert!(!p.control_pin(1));
        p.read(0, 255);
        p.tick();
        assert!(!p.control_pin(1));
        assert!(!p.control_pin(0));
        p.tick();
        assert!(p.control_pin(1));
        assert!(p.control_pin(0));
    }
    #[test]
    fn ddr_latches_and_port_a_grounding() {
        let mut p = Pia::default();
        p.write(0, 0xf0);
        p.write(1, 4);
        p.write(0, 0xa0);
        assert_eq!(p.read(0, 0x5f), 0x0f);
        p.write(2, 0xf0);
        p.write(3, 4);
        p.write(2, 0xa0);
        assert_eq!(p.read(2, 0x5f), 0xaf);
        p.write(1, 0);
        assert_eq!(p.read(0, 0), 0xf0);
    }
    #[test]
    fn flags_are_independent_of_enables_and_clear_only_on_data_read() {
        let mut p = Pia::default();
        p.edge(0, false, false);
        assert_eq!(p.read(1, 255), 0x80);
        assert!(!p.irq());
        p.write(1, 1);
        assert!(p.irq());
        p.write(0, 255);
        assert!(p.irq());
        p.write(1, 5);
        p.write(0, 0);
        assert!(p.irq());
        p.read(0, 255);
        assert!(!p.irq());
        p.write(3, 0x1c);
        p.edge(1, true, false);
        p.edge(1, true, true);
        assert!(p.irq());
        assert_eq!(p.read(3, 255), 0x5c);
        p.write(3, 0x3c);
        assert!(!p.irq());
    }
    #[test]
    fn forced_pulse_and_handshake_outputs() {
        let mut p = Pia::default();
        p.write(1, 0x34);
        assert!(!p.control_pin(0));
        p.write(1, 0x3c);
        assert!(p.control_pin(0));
        p.write(1, 0x2c);
        p.read(0, 255);
        assert!(!p.control_pin(0));
        p.tick();
        assert!(!p.control_pin(0));
        p.tick();
        assert!(p.control_pin(0));
        p.write(1, 0x24);
        p.read(0, 255);
        p.tick();
        p.tick();
        assert!(!p.control_pin(0));
        p.edge(0, false, false);
        assert!(p.control_pin(0));
        p.write(3, 0x24);
        p.write(2, 1);
        p.edge(1, false, false);
        assert!(!p.control_pin(1));
        p.read(2, 255);
        p.edge(1, false, true);
        p.edge(1, false, false);
        assert!(p.control_pin(1));
    }
}

#[cfg(test)]
mod datasheet_control_tests {
    use super::Pia;

    // Motorola MC6821, pp8–10 and Figure18. These observations test the
    // documented logical control table, not analogue timing or E-clock phase.
    #[test]
    fn both_ports_latch_each_selected_edge_independently_of_irq_enable() {
        for side in [0, 1] {
            for (second, flag, enable, polarity) in [(false, 0x80, 1, 2), (true, 0x40, 8, 16)] {
                for (active, edge_control) in [(false, 0), (true, polarity)] {
                    let mut pia = Pia::default();
                    let control = 4 | edge_control;
                    pia.write(side * 2 + 1, control);
                    pia.edge(side, second, !active);
                    assert_eq!(pia.read(side * 2 + 1, 255), control);
                    pia.edge(side, second, active);
                    assert_eq!(pia.read(side * 2 + 1, 255), control | flag);
                    assert!(!pia.irq(), "flags latch even while IRQ is disabled");

                    pia.write(side * 2 + 1, control | enable);
                    assert!(pia.irq(), "enabling a latched source asserts IRQ");
                    pia.write(side * 2 + 1, edge_control | enable);
                    pia.read(side * 2, 255); // DDR access must not clear flags.
                    pia.write(side * 2, 0x55);
                    assert!(pia.irq());
                    pia.write(side * 2 + 1, control | enable);
                    pia.write(side * 2, 0xaa); // Data write is not an acknowledgement.
                    assert!(pia.irq());
                    pia.read(side * 2, 255);
                    assert!(!pia.irq());
                    assert_eq!(pia.read(side * 2 + 1, 255), control | enable);

                    pia.edge(side, second, active); // A level alone is not an edge.
                    assert!(!pia.irq());
                    pia.edge(side, second, !active);
                    assert!(!pia.irq());
                    pia.edge(side, second, active);
                    assert!(pia.irq());
                }
            }
        }
    }

    #[test]
    fn flag_bits_are_read_only_and_data_reads_acknowledge_only_the_selected_port() {
        for side in [0, 1] {
            for requested in 0..=255 {
                let mut pia = Pia::default();
                pia.write(side * 2 + 1, requested);
                assert_eq!(pia.read(side * 2 + 1, 255), requested & 63);
            }
        }
        let mut pia = Pia::default();
        for side in [0, 1] {
            pia.write(side * 2 + 1, 0x0d);
            pia.edge(side, false, false);
            pia.edge(side, true, false);
            assert_eq!(pia.read(side * 2 + 1, 255), 0xcd);
        }
        pia.read(0, 255);
        assert_eq!(pia.read(1, 255), 0x0d);
        assert_eq!(pia.read(3, 255), 0xcd);
        assert!(pia.irq());
        pia.read(2, 255);
        assert!(!pia.irq());
    }

    #[test]
    fn c2_output_modes_clear_only_c2_flags_and_keep_forced_levels() {
        for side in [0, 1] {
            for (control, high) in [(0x35, false), (0x3d, true)] {
                let mut pia = Pia::default();
                pia.write(side * 2 + 1, 0x0d);
                pia.edge(side, false, false);
                pia.edge(side, true, false);
                pia.write(side * 2 + 1, control);
                assert_eq!(pia.read(side * 2 + 1, 255), control | 0x80);
                assert!(pia.irq(), "changing C2 mode must preserve C1's flag");
                assert_eq!(pia.driven_control_pin(side), Some(high));
                for level in [true, false, true] {
                    pia.edge(side, true, level);
                    assert_eq!(pia.read(side * 2 + 1, 255) & 0x40, 0);
                    assert_eq!(pia.control_pin(side), high);
                }
                pia.read(side * 2, 255);
                pia.write(side * 2, 0xa5);
                pia.tick();
                pia.tick();
                assert!(!pia.irq());
                assert_eq!(pia.control_pin(side), high);
                pia.write(side * 2 + 1, 4);
                assert_eq!(pia.driven_control_pin(side), None);
                assert!(pia.control_pin(side), "input exposes the current C2 level");
            }
        }
    }

    #[test]
    fn pulse_strobes_use_a_data_read_and_b_data_write_without_ddr_side_effects() {
        let mut pia = Pia::default();
        pia.write(1, 0x28);
        pia.write(3, 0x28);
        for register in [0, 2] {
            pia.write(register, 0xff);
            pia.read(register, 0);
        }
        assert!(pia.control_pin(0) && pia.control_pin(1));
        pia.write(1, 0x2c);
        pia.write(3, 0x2c);
        pia.write(0, 0x55);
        pia.read(2, 0);
        assert!(pia.control_pin(0) && pia.control_pin(1));
        pia.read(0, 0);
        assert!(!pia.control_pin(0) && pia.control_pin(1));
        pia.write(2, 0x55);
        assert!(!pia.control_pin(0) && !pia.control_pin(1));
    }
}
