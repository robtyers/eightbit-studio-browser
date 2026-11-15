//! Original PAL board composition. All clocks are guest clocks; no host I/O occurs.
use crate::{
    Device, Devices, Error, Firmware, Memory,
    cia::Cia,
    input::{Input, Joystick, Key},
    sid::{SAMPLE_RATE, Sid},
    vic::{FRAME_CYCLES, Vic, VideoMemory},
};
use eightbit_mos6502_core::{Bus, Cpu, Error as CpuError, InterruptLines, Registers};
use std::collections::VecDeque;

/// Maximum clocks accepted by one caller-driven board advance.
pub const MAX_CYCLES: u32 = 1_000_000;
/// One-second PCM retention; absent consumers cannot grow the queue.
pub const AUDIO_LIMIT: usize = SAMPLE_RATE as usize;

/// Machine-side external signals. High means released/not asserted.
#[derive(Clone, Copy, Debug)]
// Independent physical pins/flip-flops are intentionally represented as booleans.
#[allow(clippy::struct_excessive_bools)]
pub struct Pins {
    /// Cassette button sense (low when play is pressed).
    pub cassette_sense: bool,
    /// Cassette read level connected to CIA1 FLAG.
    pub cassette_read: bool,
    /// External IEC clock and data levels, wired-AND with CIA2 drivers.
    pub iec_clock: bool,
    /// External IEC data level.
    pub iec_data: bool,
    /// CIA2/user-port parallel inputs.
    pub user_parallel: u8,
    /// User-port CNT level.
    pub user_cnt: bool,
    /// User-port SP level.
    pub user_sp: bool,
    /// User-port FLAG level.
    pub user_flag: bool,
    /// CIA1 serial/timer CNT input (a separate user-port pin).
    pub cia1_cnt: bool,
    /// CIA1 serial SP input.
    pub cia1_sp: bool,
    /// RESTORE asserts NMI directly; it is outside the keyboard matrix.
    pub restore: bool,
}
impl Default for Pins {
    fn default() -> Self {
        Self {
            cassette_sense: true,
            cassette_read: true,
            iec_clock: true,
            iec_data: true,
            user_parallel: 0xff,
            user_cnt: true,
            user_sp: true,
            user_flag: true,
            cia1_cnt: true,
            cia1_sp: true,
            restore: false,
        }
    }
}
/// Physical output observations; they do not imply a connected tape/drive/printer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
// Independent physical pins/flip-flops are intentionally represented as booleans.
#[allow(clippy::struct_excessive_bools)]
pub struct Outputs {
    /// Cassette write output level.
    pub cassette_write: bool,
    /// Cassette motor enable, active low at the physical processor pin.
    pub cassette_motor: bool,
    /// Released/high IEC attention line.
    pub iec_attention: bool,
    /// Released/high IEC clock line.
    pub iec_clock: bool,
    /// Released/high IEC data line.
    pub iec_data: bool,
    /// CIA2 parallel user-port drivers (released pins appear high).
    pub user_parallel: u8,
    /// CIA2 serial CNT output.
    pub user_cnt: bool,
    /// CIA2 serial SP output.
    pub user_sp: bool,
    /// CIA1 serial CNT output.
    pub cia1_cnt: bool,
    /// CIA1 serial SP output.
    pub cia1_sp: bool,
    /// CIA2 active-low parallel handshake output.
    pub user_strobe: bool,
}
/// Family devices, clocked exclusively by the owned PAL bus.
pub struct Hardware {
    vic: Vic,
    cias: [Cia; 2],
    sid: Sid,
    input: Input,
    pins: Pins,
    audio: VecDeque<i16>,
    nmi_level: bool,
    nmi_edge: bool,
    cycles: u64,
    stalls: u64,
    mains_clock: u32,
}
impl Default for Hardware {
    fn default() -> Self {
        Self {
            vic: Vic::default(),
            cias: [Cia::default(), Cia::default()],
            sid: Sid::default(),
            input: Input::default(),
            pins: Pins::default(),
            audio: VecDeque::with_capacity(AUDIO_LIMIT),
            nmi_level: false,
            nmi_edge: false,
            cycles: 0,
            stalls: 0,
            mains_clock: 0,
        }
    }
}
impl Hardware {
    /// Read the completed frame and raster/IRQ observation interface.
    #[must_use]
    pub const fn video(&self) -> &Vic {
        &self.vic
    }
    /// Read-only SID digital observations, without clocking or guest register reads.
    #[must_use]
    pub const fn sound(&self) -> &Sid {
        &self.sid
    }
    /// Current external pins.
    #[must_use]
    pub const fn pins(&self) -> Pins {
        self.pins
    }
    fn capture_nmi(&mut self) {
        let level = self.cias[1].irq() || self.pins.restore;
        self.nmi_edge |= level && !self.nmi_level;
        self.nmi_level = level;
    }
}
impl Devices for Hardware {
    fn read(&mut self, device: Device, _address: u16, latch: u8) -> Result<u8, Error> {
        let result = match device {
            Device::Vic(register) => self.vic.read(register),
            Device::Sid(register) => self.sid.read(register),
            Device::Cia1(register) => {
                let levels = self
                    .input
                    .levels(self.cias[0].driven_port(0), self.cias[0].driven_port(1));
                self.cias[0].read(register, levels)
            }
            Device::Cia2(register) => {
                let drivers = self.cias[1].driven_port(0);
                let clock = self.pins.iec_clock && drivers & 0x10 == 0;
                let data = self.pins.iec_data && drivers & 0x20 == 0;
                let a = (0x3f | u8::from(clock) << 6 | u8::from(data) << 7) & drivers;
                self.cias[1].read(register, [a, self.pins.user_parallel])
            }
            // No cartridge/expansion is attached: the bus remains electrically open.
            Device::Expansion(_) => latch,
        };
        self.capture_nmi();
        Ok(result)
    }
    fn write(&mut self, device: Device, _address: u16, value: u8) -> Result<(), Error> {
        match device {
            Device::Vic(register) => self.vic.write(register, value),
            Device::Sid(register) => self.sid.write(register, value),
            Device::Cia1(register) => self.cias[0].write(register, value),
            Device::Cia2(register) => self.cias[1].write(register, value),
            Device::Expansion(_) => {}
        }
        self.capture_nmi();
        Ok(())
    }
}

/// PAL-timed memory bus, with finite RDY/DMA waits and bounded frame/audio ownership.
pub struct PalBus<'a> {
    memory: Memory<'a, Hardware>,
}
impl<'a> PalBus<'a> {
    /// Inert board construction with borrowed, size-checked firmware.
    #[must_use]
    pub fn new(firmware: Firmware<'a>) -> Self {
        Self {
            memory: Memory::with_devices(firmware, Hardware::default()),
        }
    }
    /// Physical RAM observation without side effects.
    #[must_use]
    pub fn ram(&self, address: u16) -> u8 {
        self.memory.ram(address)
    }
    /// Current original page and continuation depth, absent for legacy firmware.
    #[must_use]
    pub fn original_page(&self) -> Option<(u8, usize)> {
        self.memory.original_page()
    }
    /// Inspect devices without executing or clocking the guest.
    #[must_use]
    pub const fn hardware(&self) -> &Hardware {
        &self.memory.devices
    }
    /// Total PHI2 clocks, including DMA holds.
    #[must_use]
    pub const fn cycles(&self) -> u64 {
        self.memory.devices.cycles
    }
    /// Clocks during which a CPU access was held for VIC ownership.
    #[must_use]
    pub const fn stalls(&self) -> u64 {
        self.memory.devices.stalls
    }
    /// Initialisation for original hardware vectors. Not a PRG import/run bypass.
    pub fn initialise_ram(&mut self, address: u16, bytes: &[u8]) -> Result<(), Error> {
        self.memory.initialise_ram(address, bytes)
    }
    /// Physical switch input; no source text is evaluated or written into guest buffers.
    pub fn key(&mut self, key: Key, pressed: bool) {
        self.memory.devices.input.key(key, pressed);
    }
    /// Present physical joystick switches.
    pub fn joystick(&mut self, port: u8, joystick: Joystick) -> Result<(), Error> {
        self.memory.devices.input.joystick(port, joystick)
    }
    /// Replace explicit external physical levels.
    pub fn pins(&mut self, pins: Pins) {
        self.memory.devices.pins = pins;
        self.memory
            .set_port_inputs(if pins.cassette_sense { 0x3f } else { 0x2f });
        self.memory.devices.capture_nmi();
    }
    /// Present bounded SID paddle/external-audio input samples.
    pub fn sid_inputs(&mut self, pots: [u8; 2], external: i16) {
        self.memory.devices.sid.inputs(pots, external);
    }
    /// Pulse the physical light-pen input.
    pub fn light_pen(&mut self) {
        self.memory.devices.vic.light_pen();
    }
    /// Release held keyboard, joystick and RESTORE inputs.
    pub fn release_inputs(&mut self) {
        self.memory.devices.input.release();
        self.memory.devices.pins.restore = false;
        self.memory.devices.capture_nmi();
    }
    /// Observe external drivers. IEC drivers are inverting open-collector outputs.
    #[must_use]
    pub fn outputs(&self) -> Outputs {
        let h = &self.memory.devices;
        let processor = self.memory.port.pins(self.memory.inputs);
        let a = h.cias[1].driven_port(0);
        let (cnt, sp) = h.cias[1].serial_outputs();
        let (cnt1, sp1) = h.cias[0].serial_outputs();
        Outputs {
            cassette_write: processor & 8 != 0,
            cassette_motor: processor & 32 == 0,
            iec_attention: a & 8 == 0,
            iec_clock: a & 16 == 0,
            iec_data: a & 32 == 0,
            user_parallel: h.cias[1].driven_port(1),
            user_cnt: cnt,
            user_sp: sp,
            cia1_cnt: cnt1,
            cia1_sp: sp1,
            user_strobe: h.cias[1].parallel_strobe(),
        }
    }
    /// Take at most one second of retained mono PCM. Reading does not clock the guest.
    pub fn take_audio(&mut self) -> Vec<i16> {
        self.memory.devices.audio.drain(..).collect()
    }
    fn discard_audio(&mut self) {
        self.memory.devices.audio.clear();
    }
    /// Explicit hardware reset. RAM is retained; input/audio/device ownership is cleared.
    pub fn reset(&mut self) {
        self.memory.devices = Hardware::default();
        self.memory.reset_port();
        self.memory.set_port_inputs(0x3f);
    }
    /// Advance disconnected hardware for bounded vectors; CPU execution uses `Machine`.
    pub fn advance_idle(&mut self, cycles: u32) -> Result<(), Error> {
        if !(1..=MAX_CYCLES).contains(&cycles) {
            return Err(Error::CycleBudget);
        }
        for _ in 0..cycles {
            self.clock()?;
        }
        Ok(())
    }
    fn clock(&mut self) -> Result<(), Error> {
        let h = &mut self.memory.devices;
        h.cycles = h.cycles.checked_add(1).ok_or(Error::ClockLimit)?;
        let bank = (!h.cias[1].driven_port(0)) & 3;
        let video = VideoMemory {
            ram: &self.memory.ram,
            colour: &self.memory.colour,
            characters: self.memory.firmware.characters,
            bank,
        };
        if let Some(value) = h.vic.clock(&video) {
            self.memory.bus_latch = value;
        }
        let levels = h.pins;
        h.cias[0].pins(levels.cia1_cnt, levels.cia1_sp, levels.cassette_read);
        h.cias[1].pins(levels.user_cnt, levels.user_sp, levels.user_flag);
        for cia in &mut h.cias {
            cia.tick();
        }
        // The physical 50 Hz mains source is independent of the 50.125 Hz raster.
        h.mains_clock += 50;
        if h.mains_clock >= crate::sid::CLOCK_HZ {
            h.mains_clock -= crate::sid::CLOCK_HZ;
            for cia in &mut h.cias {
                cia.tod_pulse();
            }
        }
        if let Some(sample) = h.sid.tick() {
            if h.audio.len() == AUDIO_LIMIT {
                h.audio.pop_front();
            }
            h.audio.push_back(sample);
        }
        h.capture_nmi();
        Ok(())
    }
    fn wait(&mut self, read: bool, address: u16) -> Result<(), Error> {
        for _ in 0..126 {
            let vic = &self.memory.devices.vic;
            if !(if read { vic.ba_low() } else { vic.aec_low() }) {
                return Ok(());
            }
            // RDY repeats a read while the CPU still owns PHI2 during BA lead time.
            if read && !vic.aec_low() {
                self.memory.read(address)?;
            }
            self.memory.devices.stalls = self
                .memory
                .devices
                .stalls
                .checked_add(1)
                .ok_or(Error::ClockLimit)?;
            self.clock()?;
        }
        Err(Error::BusWait)
    }
}
impl Bus for PalBus<'_> {
    type Error = Error;
    fn read(&mut self, address: u16) -> Result<u8, Error> {
        self.wait(true, address)?;
        self.clock()?;
        self.memory.read(address)
    }
    fn write(&mut self, address: u16, value: u8) -> Result<(), Error> {
        self.wait(false, address)?;
        self.clock()?;
        let video_data = self.memory.bus_latch;
        self.memory.write(address, value)?;
        if address <= 1 {
            // The on-chip port does not drive the external write data pins.
            self.memory.ram[usize::from(address)] = video_data;
            self.memory.bus_latch = video_data;
        }
        Ok(())
    }
    fn interrupt_lines(&mut self) -> Option<InterruptLines> {
        let h = &mut self.memory.devices;
        let edge = std::mem::take(&mut h.nmi_edge);
        Some(InterruptLines {
            irq: h.vic.irq() || h.cias[0].irq(),
            nmi: h.nmi_level,
            nmi_edge: edge,
        })
    }
    fn execution(&mut self, event: eightbit_mos6502_core::Execution) -> Result<(), Error> {
        self.memory.execution(event)
    }
}

/// Explicit lifecycle for a pure caller-owned machine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum State {
    /// Inert or explicitly stopped.
    Stopped,
    /// CPU may advance only through an explicit caller request.
    Running,
    /// A CPU/bus fault requires reset.
    Faulted,
}
/// Receipt for one bounded run; a final instruction may pass the requested clock limit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Advance {
    /// Actual elapsed board clocks including DMA.
    pub cycles: u64,
    /// Completed instructions/interrupt entries.
    pub steps: u32,
    /// Cancellation observed before a step.
    pub cancelled: bool,
}
/// Original PAL CPU/board composition. Construction is inert; reset is explicit.
pub struct Machine<'a> {
    cpu: Cpu,
    bus: PalBus<'a>,
    state: State,
}
impl<'a> Machine<'a> {
    /// Create an inert machine with explicit borrowed firmware.
    #[must_use]
    pub fn new(firmware: Firmware<'a>) -> Self {
        Self {
            cpu: Cpu::new(),
            bus: PalBus::new(firmware),
            state: State::Stopped,
        }
    }
    /// Current lifecycle.
    #[must_use]
    pub const fn state(&self) -> State {
        self.state
    }
    /// Observe CPU registers without advancing guest time.
    #[must_use]
    pub const fn registers(&self) -> Registers {
        self.cpu.registers()
    }
    /// Observe the owned board.
    #[must_use]
    pub const fn bus(&self) -> &PalBus<'a> {
        &self.bus
    }
    /// Supply physical inputs/read bounded observations through the owned board.
    pub fn bus_mut(&mut self) -> &mut PalBus<'a> {
        &mut self.bus
    }
    /// Reset peripherals and CPU; no RAM/ROM patches or host firmware calls occur.
    pub fn reset(&mut self) -> Result<(), CpuError<Error>> {
        self.state = State::Faulted;
        self.bus.reset();
        self.cpu.reset(&mut self.bus)?;
        self.state = State::Running;
        Ok(())
    }
    /// Stop clocking and release held input/audio. RAM and the latest frame remain.
    pub fn stop(&mut self) {
        if self.state != State::Faulted {
            self.state = State::Stopped;
        }
        self.bus.release_inputs();
        self.bus.discard_audio();
    }
    /// Resume an explicitly stopped healthy machine; faulted machines require reset.
    pub fn resume(&mut self) -> Result<(), Error> {
        if self.state == State::Faulted {
            return Err(Error::MachineState);
        }
        self.state = State::Running;
        Ok(())
    }
    /// Advance 1–1,000,000 PAL clocks, checking cancellation between bounded CPU steps.
    pub fn advance(
        &mut self,
        budget: u32,
        mut cancel: impl FnMut() -> bool,
    ) -> Result<Advance, CpuError<Error>> {
        if !(1..=MAX_CYCLES).contains(&budget) {
            return Err(CpuError::Bus(Error::CycleBudget));
        }
        if self.state != State::Running {
            return Err(CpuError::Bus(Error::MachineState));
        }
        let start = self.bus.cycles();
        let mut steps = 0;
        while self.bus.cycles() - start < u64::from(budget) {
            if cancel() {
                self.bus.release_inputs();
                self.bus.discard_audio();
                return Ok(Advance {
                    cycles: self.bus.cycles() - start,
                    steps,
                    cancelled: true,
                });
            }
            if let Err(error) = self.cpu.step(&mut self.bus) {
                self.state = State::Faulted;
                self.bus.release_inputs();
                self.bus.discard_audio();
                return Err(error);
            }
            steps += 1;
        }
        Ok(Advance {
            cycles: self.bus.cycles() - start,
            steps,
            cancelled: false,
        })
    }
}

/// A useful exact one-frame budget for tests and future schedulers.
pub const FRAME_BUDGET: u32 = FRAME_CYCLES;
