//! Original active-low keyboard/joystick wiring. No host event API or text evaluator.
use crate::Error;

/// Physical matrix position (CIA1 PA row, PB column), independently of key legends.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Key {
    row: u8,
    column: u8,
}
impl Key {
    /// Validate the physical 8×8 matrix coordinates.
    pub fn new(row: u8, column: u8) -> Result<Self, Error> {
        if row >= 8 || column >= 8 {
            return Err(Error::InputRange);
        }
        Ok(Self { row, column })
    }
    /// Matrix row/column, both in 0..8.
    #[must_use]
    pub const fn coordinates(self) -> (u8, u8) {
        (self.row, self.column)
    }
}
/// Active-high pressed switches for one joystick; bits: up/down/left/right/fire.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Joystick(u8);
impl Joystick {
    /// Validate the five physical switches.
    pub fn new(pressed: u8) -> Result<Self, Error> {
        if pressed & !31 != 0 {
            Err(Error::InputRange)
        } else {
            Ok(Self(pressed))
        }
    }
}
/// Physical keyboard/joystick switches. Matrix ghosting propagates bidirectionally.
#[derive(Clone, Debug, Default)]
pub struct Input {
    rows: [u8; 8],
    joysticks: [Joystick; 2],
}
impl Input {
    /// Press/release one validated physical key. Shift Lock uses the Left Shift switch.
    pub fn key(&mut self, key: Key, pressed: bool) {
        let mask = 1 << key.column;
        if pressed {
            self.rows[usize::from(key.row)] |= mask;
        } else {
            self.rows[usize::from(key.row)] &= !mask;
        }
    }
    /// Select physical control port 1 or 2. Nothing is changed on an invalid port.
    pub fn joystick(&mut self, port: u8, joystick: Joystick) -> Result<(), Error> {
        if !(1..=2).contains(&port) {
            return Err(Error::InputRange);
        }
        self.joysticks[usize::from(port - 1)] = joystick;
        Ok(())
    }
    /// Release every input, for focus loss/stop/reset ownership boundaries.
    pub fn release(&mut self) {
        *self = Self::default();
    }
    /// Solve the eight-by-eight switch network; output-low drivers win over pull-ups.
    #[must_use]
    pub fn levels(&self, driven_a: u8, driven_b: u8) -> [u8; 2] {
        let mut a = driven_a & !self.joysticks[1].0;
        let mut b = driven_b & !self.joysticks[0].0;
        // There are only sixteen wires: monotone low propagation settles in <=16.
        for _ in 0..16 {
            let before = (a, b);
            for row in 0..8 {
                let switches = self.rows[row];
                if a & (1 << row) == 0 {
                    b &= !switches;
                }
                if switches & !b != 0 {
                    a &= !(1 << row);
                }
            }
            if before == (a, b) {
                break;
            }
        }
        [a, b]
    }
}
