//! Strict identity-bound portable original session; native and WASM use this API.
use crate::{
    Result, error,
    machine::{FRAME_CYCLES, HEIGHT, Machine, SAMPLE_RATE, WIDTH},
};
use serde::{Deserialize, Serialize};
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
pub struct Session {
    id: u32,
    cartridge: Vec<u8>,
    machine: Machine,
    stopped: bool,
    failed: bool,
    keys: std::collections::BTreeSet<String>,
}
impl Session {
    pub fn new(
        id: u32,
        bytes: &[u8],
        sha256: &str,
        mut cancel: impl FnMut() -> bool,
    ) -> Result<Self> {
        if id == 0 || !(20..=32768).contains(&bytes.len()) || crate::digest(bytes) != sha256 {
            return Err(error("VXO1000", "Invalid cartridge/session identity"));
        }
        if cancel() {
            return Err(error("VXO1002", "Original Vectrex launch cancelled"));
        }
        let machine = Machine::new(bytes.to_vec())?;
        let mut s = Self {
            id,
            cartridge: bytes.to_vec(),
            machine,
            stopped: false,
            failed: false,
            keys: std::collections::BTreeSet::new(),
        };
        s.machine.advance(FRAME_CYCLES, &mut cancel)?;
        Ok(s)
    }
    #[must_use]
    pub fn running(&self) -> bool {
        !self.stopped && !self.failed
    }
    #[must_use]
    pub fn machine(&self) -> &Machine {
        &self.machine
    }
    pub fn request(&mut self, json: &str) -> Result<String> {
        if json.len() > 4096 {
            return Err(error("VXO1000", "Vectrex request exceeds its limit"));
        }
        let request: Request =
            serde_json::from_str(json).map_err(|_| error("VXO1000", "Invalid Vectrex request"))?;
        if request.schema_version != 1 || request.session_id != self.id {
            return Err(error("VXO1000", "Stale or unsupported Vectrex request"));
        }
        serde_json::to_string(&self.command(request.command)?)
            .map_err(|_| error("VXO1000", "Cannot encode Vectrex frame"))
    }
    pub fn command(&mut self, command: Command) -> Result<Frame> {
        match command {
            Command::State {} => {}
            Command::Stop {} => {
                self.release();
                self.stopped = true;
            }
            Command::Release {} => self.release(),
            Command::Reset {} => {
                let machine = Machine::new(self.cartridge.clone())?;
                self.machine = machine;
                self.failed = false;
                self.stopped = false;
                self.release();
            }
            Command::Advance { cycles } => {
                if self.stopped || self.failed {
                    return Err(error("VXO1001", "Vectrex is stopped; reset to recover"));
                }
                if let Err(e) = self.machine.advance(cycles, || false) {
                    if e.code == "VXO1001" {
                        self.failed = true;
                        self.release();
                    }
                    return Err(e);
                }
            }
            Command::Key { code, pressed } => {
                if self.stopped || self.failed {
                    return Err(error("VXO1001", "Vectrex is stopped; reset to recover"));
                }
                if !matches!(
                    code.as_str(),
                    "ArrowLeft"
                        | "ArrowRight"
                        | "ArrowUp"
                        | "ArrowDown"
                        | "KeyZ"
                        | "KeyX"
                        | "KeyC"
                        | "KeyV"
                        | "KeyA"
                        | "KeyD"
                        | "KeyW"
                        | "KeyS"
                        | "KeyF"
                        | "KeyG"
                        | "KeyH"
                        | "KeyJ"
                ) {
                    return Err(error("VXO1000", "Unsupported Vectrex controller key"));
                }
                if pressed {
                    self.keys.insert(code);
                } else {
                    self.keys.remove(&code);
                }
                self.input();
            }
        }
        let terminal = self.stopped || self.failed;
        Ok(Frame {
            schema_version: 1,
            session_id: self.id,
            firmware_id: crate::firmware::ID,
            width: WIDTH,
            height: HEIGHT,
            frame_t_states: FRAME_CYCLES,
            state: if self.failed {
                "failed"
            } else if self.stopped {
                "stopped"
            } else {
                "running"
            },
            message: if self.failed {
                "Original Vectrex failed; reset or use VecX"
            } else {
                "Original Vectrex development runtime"
            },
            text: String::new(),
            pixels: self
                .machine
                .board
                .pixels()
                .iter()
                .flat_map(|v| [*v, *v, *v, 255])
                .collect(),
            audio_sample_rate: SAMPLE_RATE,
            audio: if terminal {
                self.machine.board.take_audio();
                Vec::new()
            } else {
                self.machine.board.take_audio()
            },
        })
    }
    fn release(&mut self) {
        self.keys.clear();
        self.input();
    }
    fn input(&mut self) {
        for (port, names) in [
            [
                "ArrowLeft",
                "ArrowRight",
                "ArrowUp",
                "ArrowDown",
                "KeyZ",
                "KeyX",
                "KeyC",
                "KeyV",
            ],
            [
                "KeyA", "KeyD", "KeyW", "KeyS", "KeyF", "KeyG", "KeyH", "KeyJ",
            ],
        ]
        .iter()
        .enumerate()
        {
            let has = |i: usize| self.keys.contains(names[i]);
            let c = &mut self.machine.board.controllers[port];
            c.x = if has(0) == has(1) {
                0
            } else if has(0) {
                -127
            } else {
                127
            };
            c.y = if has(2) == has(3) {
                0
            } else if has(2) {
                127
            } else {
                -127
            };
            c.buttons = (0..4).fold(0, |buttons, i| {
                buttons | if has(i + 4) { 1 << i } else { 0 }
            });
        }
    }
}
