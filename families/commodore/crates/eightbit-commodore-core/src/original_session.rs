//! Family-owned, bounded original C64 session shared by native and browser hosts.
//! Loading a standard PRG writes its public RAM image. All execution, values and
//! RUN handling then belong to the original guest firmware, never a host BASIC.
use eightbit_commodore_machine::{
    Firmware,
    input::Key,
    original::Image,
    pal::{Machine, State},
    sid::{CLOCK_HZ, SAMPLE_RATE},
    vic::{FRAME_CYCLES, HEIGHT, WIDTH},
};
use eightbit_source_core::{Result, digest, error};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};

pub const MAX_REQUEST_BYTES: usize = 64 * 1024;
pub const FIRMWARE_ID: &str = "eightbit-c64-original-development-0.5";
pub const BUNDLED_FIRMWARE: &[u8] = include_bytes!("../../../firmware/original/runtime.c64rom");
pub const BUNDLED_SHA256: [u8; 32] = [
    183, 100, 67, 159, 241, 182, 91, 200, 103, 123, 147, 171, 132, 134, 6, 134, 191, 238, 52, 161,
    218, 42, 117, 49, 243, 86, 32, 55, 173, 52, 123, 168,
];
const MAX_TEXT: usize = 4096;
const MAX_SLICE: u32 = FRAME_CYCLES * 4;
const KEY_TIME: u32 = FRAME_CYCLES * 2;
// Independent illustrative RGB values, not an extracted emulator palette.
const PALETTE: [[u8; 3]; 16] = [
    [0, 0, 0],
    [255, 255, 255],
    [165, 50, 50],
    [96, 200, 190],
    [160, 72, 180],
    [75, 162, 65],
    [60, 55, 160],
    [218, 211, 103],
    [170, 100, 40],
    [100, 60, 20],
    [230, 130, 130],
    [70, 70, 70],
    [120, 120, 120],
    [140, 220, 130],
    [150, 180, 255],
    [190, 190, 190],
];
#[derive(Deserialize)]
#[serde(
    tag = "operation",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Command {
    Advance { cycles: u32 },
    State {},
    Reset {},
    Stop {},
    Release {},
    Type { text: String },
    Key { code: String, pressed: bool },
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub schema_version: u32,
    pub session_id: u32,
    pub command: Command,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    pub schema_version: u32,
    pub session_id: u32,
    pub firmware_id: &'static str,
    pub width: usize,
    pub height: usize,
    pub frame_t_states: u32,
    pub state: &'static str,
    pub message: &'static str,
    pub text: String,
    pub pixels: Vec<u8>,
    pub audio_sample_rate: u32,
    pub audio: Vec<i16>,
}
pub struct Session<'a> {
    id: u32,
    machine: Machine<'a>,
    terminal: bool,
    failed: bool,
    pending: VecDeque<Vec<Key>>,
    remaining: u32,
    manual: BTreeSet<String>,
}
fn invalid(message: &str) -> eightbit_source_core::Error {
    error("C64RUN1000", message)
}
fn board<E: std::fmt::Display>(_: E) -> eightbit_source_core::Error {
    // Firmware/guest data and host paths never enter support diagnostics.
    error(
        "C64RUN1001",
        "Original C64 execution failed; reset to recover",
    )
}
impl<'a> Session<'a> {
    pub fn new(id: u32, bytes: &'a [u8], expected: &[u8; 32]) -> Result<Self> {
        if id == 0 {
            return Err(invalid("Invalid C64 session identity"));
        }
        let firmware =
            Firmware::original(Image::parse(bytes, expected).map_err(board)?).map_err(board)?;
        let mut machine = Machine::new(firmware);
        machine.reset().map_err(board)?;
        Ok(Self {
            id,
            machine,
            terminal: false,
            failed: false,
            pending: VecDeque::new(),
            remaining: 0,
            manual: BTreeSet::new(),
        })
    }
    /// Explicit fresh-build launch. A malformed hash/PRG is rejected before boot
    /// or RAM writes; cancellation never publishes a partly prepared session.
    pub fn from_prg(
        id: u32,
        firmware: &'a [u8],
        expected: &[u8; 32],
        prg: &[u8],
        sha256: &str,
        mut cancel: impl FnMut() -> bool,
    ) -> Result<Self> {
        let end = validate_prg(prg)?;
        if digest(prg) != sha256 {
            return Err(invalid("C64 build identity changed; build again"));
        }
        let mut session = Self::new(id, firmware, expected)?;
        for _ in 0..30 {
            if cancel() {
                return Err(invalid("C64 launch cancelled"));
            }
            session
                .machine
                .advance(CLOCK_HZ / 10, &mut cancel)
                .map_err(board)?;
        }
        // Install only at the completed public direct-mode boundary. No PC or
        // register override, opcode trap, private value storage or host parser.
        let mut ready = false;
        for _ in 0..100_000 {
            if session.machine.bus().original_page() == Some((0, 0))
                && session.text().contains("READY.")
            {
                ready = true;
                break;
            }
            if cancel() {
                return Err(invalid("C64 launch cancelled"));
            }
            session.machine.advance(1, &mut cancel).map_err(board)?;
        }
        if !ready {
            return Err(invalid("Original C64 did not become ready"));
        }
        session
            .machine
            .bus_mut()
            .initialise_ram(0x0801, &prg[2..])
            .map_err(board)?;
        for pointer in [0x2d, 0x2f, 0x31] {
            session
                .machine
                .bus_mut()
                .initialise_ram(pointer, &end.to_le_bytes())
                .map_err(board)?;
        }
        session.machine.bus_mut().take_audio();
        session.queue("RUN\r")?;
        if cancel() {
            return Err(invalid("C64 launch cancelled"));
        }
        Ok(session)
    }
    pub fn command(&mut self, request: Request) -> Result<Frame> {
        if request.schema_version != 1 || request.session_id != self.id {
            return Err(invalid("C64 session is stale"));
        }
        if self.terminal
            && !matches!(
                request.command,
                Command::State {} | Command::Stop {} | Command::Release {}
            )
        {
            return Err(invalid("C64 session is stopped; start a fresh run"));
        }
        match request.command {
            Command::State {} => {}
            Command::Stop {} => {
                self.release();
                self.machine.stop();
                self.terminal = true;
            }
            Command::Release {} => self.release(),
            Command::Reset {} => {
                self.release();
                self.machine.reset().map_err(board)?;
                self.failed = false;
            }
            Command::Type { text } => self.queue(&text)?,
            Command::Key { code, pressed } => {
                if !self.pending.is_empty() {
                    return Err(invalid("Finish the queued C64 input first"));
                }
                physical_code(&code)?;
                if pressed {
                    self.manual.insert(code);
                } else {
                    self.manual.remove(&code);
                }
                self.machine.bus_mut().release_inputs();
                for code in &self.manual {
                    for key in physical_code(code)? {
                        self.machine.bus_mut().key(key, true);
                    }
                }
            }
            Command::Advance { cycles } => {
                if !(1..=MAX_SLICE).contains(&cycles) {
                    return Err(invalid("C64 cycle budget is out of range"));
                }
                if self.failed {
                    return Err(invalid("Reset the failed C64 before advancing"));
                }
                if self.advance(cycles).is_err() {
                    self.release();
                    self.failed = true;
                }
            }
        }
        Ok(self.frame())
    }
    pub fn request(&mut self, encoded: &str) -> Result<String> {
        if encoded.len() > MAX_REQUEST_BYTES {
            return Err(invalid("C64 request exceeds its limit"));
        }
        let request: Request =
            serde_json::from_str(encoded).map_err(|_| invalid("Invalid C64 command"))?;
        serde_json::to_string(&self.command(request)?)
            .map_err(|_| invalid("Cannot encode C64 frame"))
    }
    fn advance(&mut self, mut cycles: u32) -> Result<()> {
        while cycles > 0 {
            if self.remaining == 0 {
                self.machine.bus_mut().release_inputs();
                if let Some(keys) = self.pending.pop_front() {
                    for key in keys {
                        self.machine.bus_mut().key(key, true);
                    }
                    self.remaining = KEY_TIME;
                } else {
                    for code in &self.manual {
                        for key in physical_code(code)? {
                            self.machine.bus_mut().key(key, true);
                        }
                    }
                }
            }
            let chunk = if self.remaining == 0 {
                cycles
            } else {
                cycles.min(self.remaining)
            };
            self.machine.advance(chunk, || false).map_err(board)?;
            self.remaining = self.remaining.saturating_sub(chunk);
            cycles -= chunk;
        }
        Ok(())
    }
    fn queue(&mut self, text: &str) -> Result<()> {
        if text.len() > MAX_TEXT
            || !self.pending.is_empty()
            || self.remaining != 0
            || !self.manual.is_empty()
        {
            return Err(invalid("C64 input is too large or still busy"));
        }
        let mut keys = VecDeque::new();
        for byte in text.bytes() {
            if byte == b'\n' {
                keys.push_back(physical_byte(b'\r')?);
            } else {
                keys.push_back(physical_byte(byte.to_ascii_uppercase())?);
            }
            keys.push_back(Vec::new());
        }
        self.pending = keys;
        Ok(())
    }
    fn release(&mut self) {
        self.pending.clear();
        self.remaining = 0;
        self.manual.clear();
        self.machine.bus_mut().release_inputs();
        self.machine.bus_mut().take_audio();
    }
    fn text(&self) -> String {
        let mut text = String::with_capacity(1025);
        for row in 0..25u16 {
            for col in 0..40u16 {
                let code = self.machine.bus().ram(0x0400 + row * 40 + col) & 127;
                text.push(char::from(match code {
                    1..=26 => code + 64,
                    32..=63 => code,
                    _ => b' ',
                }));
            }
            text.push('\n');
        }
        text
    }
    pub fn frame(&mut self) -> Frame {
        let state = if self.terminal {
            "stopped"
        } else if self.failed || self.machine.state() == State::Faulted {
            "failed"
        } else {
            "running"
        };
        let message = match state {
            "stopped" => "Original C64 stopped",
            "failed" => "Original C64 failed; reset to recover",
            _ => "Original C64 PAL development runtime",
        };
        let pixels = self
            .machine
            .bus()
            .hardware()
            .video()
            .frame()
            .iter()
            .flat_map(|index| {
                let [r, g, b] = PALETTE[usize::from(*index & 15)];
                [r, g, b, 255]
            })
            .collect();
        let audio = self.machine.bus_mut().take_audio();
        Frame {
            schema_version: 1,
            session_id: self.id,
            firmware_id: FIRMWARE_ID,
            width: WIDTH,
            height: HEIGHT,
            frame_t_states: FRAME_CYCLES,
            state,
            message,
            text: self.text(),
            pixels,
            audio_sample_rate: SAMPLE_RATE,
            audio,
        }
    }
}
fn validate_prg(prg: &[u8]) -> Result<u16> {
    if prg.len() < 4 || prg.len() > 38913 || prg[..2] != [1, 8] {
        return Err(invalid("Choose a C64 PRG loaded at $0801"));
    }
    let mut at = 2usize;
    let mut previous = None;
    loop {
        let link = prg
            .get(at..at + 2)
            .ok_or_else(|| invalid("Truncated C64 programme"))?;
        let next = u16::from_le_bytes([link[0], link[1]]);
        if next == 0 {
            break;
        }
        let header = prg
            .get(at + 2..at + 4)
            .ok_or_else(|| invalid("Truncated C64 line"))?;
        let line = u16::from_le_bytes([header[0], header[1]]);
        let end = usize::from(next)
            .checked_sub(0x0801)
            .and_then(|offset| offset.checked_add(2))
            .ok_or_else(|| invalid("Invalid C64 line link"))?;
        if line > 63999
            || previous.is_some_and(|p| p >= line)
            || end <= at + 4
            || end > prg.len()
            || end - at > 255
            || prg[end - 1] != 0
            || prg[at + 4..end - 1].contains(&0)
        {
            return Err(invalid("Invalid C64 line record"));
        }
        previous = Some(line);
        at = end;
    }
    // Machine-code tails are permitted by the existing ca65 SYS-loader contract.
    let end = 0x0801usize
        .checked_add(prg.len() - 2)
        .filter(|end| *end <= 0xa000)
        .ok_or_else(|| invalid("C64 PRG exceeds programme RAM"))?;
    let _ = end;
    u16::try_from(0x0801 + at).map_err(|_| invalid("Invalid C64 programme end"))
}
fn physical_byte(byte: u8) -> Result<Vec<Key>> {
    let rows = [
        b"\x14\r\x1d\x88\x85\x86\x87\x11",
        b"3WA4ZSE\x01",
        b"5RD6CFTX",
        b"7YG8BHUV",
        b"9IJ0MKON",
        b"+PL-.:@,",
        b"\x5c*;\x13\x02=^/",
        b"1\x01\x022 \x03Q\x04",
    ];
    let shifted = match byte {
        b'#' => Some((1, 0)),
        b'"' => Some((7, 3)),
        b'$' => Some((1, 3)),
        b'%' => Some((2, 0)),
        b'(' => Some((3, 3)),
        b')' => Some((4, 0)),
        b'<' => Some((5, 7)),
        b'>' => Some((5, 4)),
        b'?' => Some((6, 7)),
        _ => None,
    };
    if let Some((row, col)) = shifted {
        return Ok(vec![
            Key::new(1, 7).map_err(board)?,
            Key::new(row, col).map_err(board)?,
        ]);
    }
    for row in 0..8u8 {
        for col in 0..8u8 {
            if rows[usize::from(row)][usize::from(col)] == byte
                && (byte >= 32 || matches!(byte, 13 | 20 | 17 | 29))
            {
                return Ok(vec![Key::new(row, col).map_err(board)?]);
            }
        }
    }
    Err(invalid(
        "Text contains a character unavailable on the C64 keyboard",
    ))
}
fn physical_code(code: &str) -> Result<Vec<Key>> {
    let special = match code {
        "ShiftLeft" => Some((1, 7, false)),
        "ShiftRight" => Some((6, 4, false)),
        "ControlLeft" | "ControlRight" => Some((7, 2, false)),
        "AltLeft" | "AltRight" => Some((7, 5, false)),
        "Escape" => Some((7, 7, false)),
        "ArrowRight" => Some((0, 2, false)),
        "ArrowLeft" => Some((0, 2, true)),
        "ArrowDown" => Some((0, 7, false)),
        "ArrowUp" => Some((0, 7, true)),
        "Backspace" | "Delete" => Some((0, 0, false)),
        "Insert" => Some((0, 0, true)),
        "Home" => Some((6, 3, false)),
        "F1" => Some((0, 4, false)),
        "F2" => Some((0, 4, true)),
        "F3" => Some((0, 5, false)),
        "F4" => Some((0, 5, true)),
        "F5" => Some((0, 6, false)),
        "F6" => Some((0, 6, true)),
        "F7" => Some((0, 3, false)),
        "F8" => Some((0, 3, true)),
        _ => None,
    };
    if let Some((row, col, shift)) = special {
        let mut keys = vec![Key::new(row, col).map_err(board)?];
        if shift {
            keys.push(Key::new(1, 7).map_err(board)?);
        }
        return Ok(keys);
    }
    let byte = match code {
        "Enter" | "NumpadEnter" => 13,
        "Space" => 32,
        "Minus" => b'-',
        "Equal" => b'=',
        "Semicolon" => b';',
        "Quote" => b':',
        "Comma" => b',',
        "Period" => b'.',
        "Slash" => b'/',
        "Backslash" => b'\x5c',
        "BracketLeft" => b'@',
        "BracketRight" => b'*',
        "Backquote" => b'^',
        _ => {
            let bytes = code.as_bytes();
            if bytes.len() == 4 && &bytes[..3] == b"Key" && bytes[3].is_ascii_uppercase() {
                bytes[3]
            } else if bytes.len() == 6 && &bytes[..5] == b"Digit" && bytes[5].is_ascii_digit() {
                bytes[5]
            } else {
                return Err(invalid("Unsupported C64 physical key"));
            }
        }
    };
    physical_byte(byte)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    fn command(session: &mut Session<'_>, command: Command) -> Frame {
        session
            .command(Request {
                schema_version: 1,
                session_id: 7,
                command,
            })
            .unwrap()
    }
    fn boot() -> Session<'static> {
        Session::new(7, BUNDLED_FIRMWARE, &BUNDLED_SHA256).unwrap()
    }
    fn advance(session: &mut Session<'_>, frames: usize) -> Frame {
        let mut last = command(session, Command::State {});
        for _ in 0..frames {
            last = command(
                session,
                Command::Advance {
                    cycles: FRAME_CYCLES,
                },
            );
        }
        last
    }
    #[test]
    fn strict_protocol_identity_bounds_and_stop_are_failure_safe() {
        assert!(Session::new(0, BUNDLED_FIRMWARE, &BUNDLED_SHA256).is_err());
        assert!(Session::new(7, BUNDLED_FIRMWARE, &[0; 32]).is_err());
        let mut session = boot();
        let cycles = session.machine.bus().cycles();
        for json in [
            "{}",
            r#"{"schemaVersion":1,"sessionId":8,"command":{"operation":"reset"}}"#,
            r#"{"schemaVersion":1,"sessionId":7,"command":{"operation":"state","extra":true}}"#,
            r#"{"schemaVersion":1,"sessionId":7,"command":{"operation":"mount-cassette","bytes":[]}}"#,
        ] {
            assert!(session.request(json).is_err());
            assert_eq!(session.machine.bus().cycles(), cycles);
        }
        assert!(session.request(&" ".repeat(MAX_REQUEST_BYTES + 1)).is_err());
        for cycles in [0, MAX_SLICE + 1, u32::MAX] {
            assert!(
                session
                    .command(Request {
                        schema_version: 1,
                        session_id: 7,
                        command: Command::Advance { cycles }
                    })
                    .is_err()
            );
        }
        assert_eq!(command(&mut session, Command::Stop {}).state, "stopped");
        assert!(
            session
                .command(Request {
                    schema_version: 1,
                    session_id: 7,
                    command: Command::Reset {}
                })
                .is_err()
        );
        assert!(
            session
                .command(Request {
                    schema_version: 1,
                    session_id: 7,
                    command: Command::Advance { cycles: 1 }
                })
                .is_err()
        );
    }
    #[test]
    fn input_validation_is_atomic_and_release_revokes_all_held_input() {
        let mut session = boot();
        for text in ["ABCé".to_owned(), "A".repeat(MAX_TEXT + 1)] {
            assert!(session.queue(&text).is_err());
            assert!(session.pending.is_empty());
        }
        command(
            &mut session,
            Command::Key {
                code: "ArrowLeft".into(),
                pressed: true,
            },
        );
        command(
            &mut session,
            Command::Key {
                code: "ShiftLeft".into(),
                pressed: true,
            },
        );
        command(
            &mut session,
            Command::Key {
                code: "ShiftLeft".into(),
                pressed: false,
            },
        );
        assert!(session.manual.contains("ArrowLeft"));
        assert_eq!(physical_code("ArrowLeft").unwrap().len(), 2);
        assert!(session.queue("A").is_err());
        command(&mut session, Command::Release {});
        session.queue("A\n").unwrap();
        assert!(session.queue("B").is_err());
        assert!(
            session
                .command(Request {
                    schema_version: 1,
                    session_id: 7,
                    command: Command::Key {
                        code: "KeyB".into(),
                        pressed: true
                    }
                })
                .is_err()
        );
        command(&mut session, Command::Release {});
        assert!(session.pending.is_empty() && session.manual.is_empty() && session.remaining == 0);
        assert_eq!(physical_byte(b'Q').unwrap()[0].coordinates(), (7, 6));
        assert!(physical_code("arbitrary-host-shortcut").is_err());
    }
    #[test]
    fn prg_validation_rejects_corrupt_records_hashes_and_cancelled_launches() {
        let good = crate::basic::compile("10 PRINT\"GOOD\"\n").unwrap();
        assert_eq!(
            usize::from(validate_prg(&good).unwrap()),
            0x0801 + good.len() - 2
        );
        for bytes in [
            vec![],
            vec![1, 8],
            vec![0, 8, 0, 0],
            vec![1, 8, 1, 8, 10, 0, 0],
            vec![0; 38914],
        ] {
            assert!(validate_prg(&bytes).is_err());
        }
        let mut corrupt = good.clone();
        corrupt[4..6].copy_from_slice(&64000u16.to_le_bytes());
        assert!(validate_prg(&corrupt).is_err());
        corrupt = good.clone();
        corrupt[6] = 0;
        assert!(validate_prg(&corrupt).is_err());
        assert!(
            Session::from_prg(
                7,
                BUNDLED_FIRMWARE,
                &BUNDLED_SHA256,
                &good,
                &"0".repeat(64),
                || false
            )
            .is_err()
        );
        assert!(
            Session::from_prg(
                7,
                BUNDLED_FIRMWARE,
                &BUNDLED_SHA256,
                &good,
                &digest(&good),
                || true
            )
            .is_err()
        );
        let asm = crate::package_assembly(&crate::TARGETS[0], &[0x60]).unwrap();
        assert_eq!(validate_prg(&asm).unwrap(), 0x080d);
    }
    #[test]
    fn fresh_prg_guest_run_keyboard_reset_and_assembly_return() {
        let prg =
            crate::basic::compile("10 PRINT\"SESSION READY\"\n20 FOR I=1TO3:PRINT I:NEXT I\n")
                .unwrap();
        let mut session = Session::from_prg(
            7,
            BUNDLED_FIRMWARE,
            &BUNDLED_SHA256,
            &prg,
            &digest(&prg),
            || false,
        )
        .unwrap();
        let frame = advance(&mut session, 250);
        assert!(frame.text.contains("SESSION READY"), "{}", frame.text);
        assert!(frame.text.lines().any(|line| line.trim() == "3"));
        assert_eq!(frame.pixels.len(), WIDTH * HEIGHT * 4);
        assert_eq!(
            (frame.audio_sample_rate, frame.frame_t_states),
            (SAMPLE_RATE, FRAME_CYCLES)
        );
        assert!(frame.audio.len() <= 1000);
        command(&mut session, Command::Reset {});
        let reset = advance(&mut session, 160);
        assert!(
            reset.text.contains("EIGHTBIT C64 ORIGINAL") && !reset.text.contains("SESSION READY")
        );
        let asm =
            crate::package_assembly(&crate::TARGETS[0], &[0xa9, b'X', 0x20, 0xd2, 0xff, 0x60])
                .unwrap();
        let mut session = Session::from_prg(
            7,
            BUNDLED_FIRMWARE,
            &BUNDLED_SHA256,
            &asm,
            &digest(&asm),
            || false,
        )
        .unwrap();
        let frame = advance(&mut session, 160);
        assert!(
            frame.text.lines().any(|line| line.trim() == "X"),
            "{}",
            frame.text
        );
    }
}
