# Original C64 PAL development board

The original CPU, PAL digital board and bounded private-page firmware of
[ADR 0062](../../../../docs/adr/0062-c64-reference-qualification-and-original-runtime-plan.md)
are implemented. This portable Apache-2.0 crate depends on our CPU crate and the
pinned SHA-256 implementation. The family-owned original Studio session composes
this board on native/WASM; original execution is explicitly selected and retains
VICE/chips defaults. The compatibility specification owns timing/analogue,
firmware, peripheral and acceptance limits.

`Memory` preserves its stage-1 API: all 65,536 physical RAM bytes, colour nibbles,
6510 port drivers, borrowed firmware windows and injected devices. `NoDevices`
still faults explicitly. `Firmware::new` checks exact 8/8/4 KiB window lengths,
not identity. User-facing composition must retain strict firmware identities;
there is no runtime file API, automatic ROM provision or embedded vendor data.

`pal::Machine` composes our CPU and a timed `PalBus` with the original VIC, two
CIAs, SID and physical switch/pin modules. Construction is inert; explicit reset
starts guest code. Advance accepts 1–1,000,000 PHI2 clocks, checks cancellation
between bounded instructions and reports actual clocks/steps. A final
instruction may exceed the requested budget; individual bus holds are limited
to two raster lines (126 clocks). Healthy stopped machines require explicit
resume; faulted machines require reset. Stop/fault releases held inputs; reset
retains physical RAM and recreates device/audio ownership. No host BASIC, ROM
patch, clipboard buffer injection, process, filesystem or network API exists.

The 63×312 PAL schedule clocks peripherals during CPU DMA waits. VIC matrix,
glyph, idle/refresh and sprite-pointer/data fetches drive the shared bus. Two
384×272 palette-index buffers retain only the latest completed frame; reads do
not advance guest time. All normal text/bitmap/multicolour/extended modes,
scrolling, borders, sprites/priority/collisions/expansion, raster and light pen
have original vectors. Device snapshots remain Rust observations, not a new
product debugger. RGB exports use an original illustrative palette.

CIA register/timer/cascade/CNT/serial/TOD/alarm/strobe behaviour drives keyboard,
joysticks, IRQ/NMI and machine-side cassette/IEC/user pins. Input solves the
bidirectional 8×8 switch network, including ghosting; Key/Joystick constructors
validate physical coordinates/switches. A disconnected expansion slot is open
bus. Physical pins do not create an attached drive/tape/printer or host access.

SID uses three clock-domain oscillators, ADSR, waveform/sync/ring/noise controls,
filter routing and integer DSP. Paddles latch every 512 clocks. The fixed
44,100 Hz mono output retains at most one second (44,100 samples), discarding
the oldest if the caller is slow. Analogue transfer, combined-wave/noise quirks,
revision-specific envelopes/bus decay and colour calibration are unqualified;
see the specification rather than inferring full compatibility from a tone.

## Verification

```sh
cargo test --offline --locked -p eightbit-mos6502-core -p eightbit-commodore-machine
node scripts/c64-stage1-parity.mjs
node scripts/c64-stage2-parity.mjs
# Explicit exact-hash external firmware only:
EIGHTBIT_C64_ROMS=/absolute/external/folder node scripts/c64-stage2-parity.mjs --reference
EIGHTBIT_C64_ROMS=/absolute/external/folder cargo test --offline --locked --release -p eightbit-commodore-machine --test reference -- --ignored
```

WASM checks require Rust's installed wasm32 target and wasm-bindgen CLI 0.2.127
(`WASM_BINDGEN` selects an explicit executable). Scripts never download tools,
promote a runtime or modify browser assets; temporary generated glue is removed.
The pinned wasm-bindgen dependency remains test-only. Stage 3 uses the existing
sha2 0.10.9 at runtime for original container identity checks. ROM gates reject every incorrect/truncated identity before
boot and retain no reference bytes. The original stage-2 BASIC acceptance programme is typed
through physical keys. Guest DATA/POKE/SYS installs and executes original
assembly; the host never edits programme/workspace RAM in that qualification.
`EIGHTBIT_C64_CAPTURE_DIR` may select a new absolute folder for original
programme frames/PCM; existing destinations are rejected.

## Approved original storage/boot foundation

`original::Image::parse` requires a trusted complete-container SHA-256 and checks
strict schema-1 lengths/counts, every image/map binding, canonical non-overlapping
metadata, replicated public BASIC ranges and immutable 6510 gate templates.
`Firmware::original` enables that separate identity; `Firmware::new` never does.
`original_page` reports page/depth without mutation. There is no image file API.
The existing SHA-256 dependency is pinned and permits native/WASM integrity checks.

At most 16 BASIC pages and 32 private page continuations are supported. Only
admitted, immutable fixed-KERNAL instructions may use the private controls.
Real JSR/RTS, IRQ/NMI/BRK, guest stack/flags and peripheral clocks remain CPU/bus
operations. Gates preserve interrupted prefixes, reject forged/stale returns,
and require all relevant ROM/I/O mappings before control writes. User callbacks
see the public page and return through guest 6510 epilogues. No BASIC values or
guest RAM are replaced with private host storage. Fault/Reset revokes context;
Stop/cancel retains the CPU/page context and releases input/audio.

The [original firmware source](../../firmware/original/README.md) builds guest
BASIC V2, KERNAL, editor and original characters without vendor ROMs. Version 0.5
adds the remaining language functions, strings/arrays/collection, FOR/NEXT, ON,
DATA/READ, interactive input, DEF FN, clock/status, STOP/CONT, channels, public
callbacks and editor modes. Exact numeric vectors, shared supplied-ROM language
observations and original-only sprite/SID/input/SYS journeys qualify named
behaviour. Conservative stack limits, original random/transcendental algorithms
and glyphs retain explicit development compatibility differences.

`--original` compares a source-bound image on native/WASM. `--reference` executes
the same public language corpus with the exact externally supplied ROM set;
`--interfaces` retains the public RAM-format observations. These scripts do not
change product defaults or copy reference firmware into source or distribution.
The separate Studio session parity checks the actual source/PRG loading API.

## Source and fixture provenance

All CPU/board/firmware/fixture/programme code was independently authored. The
numeric fixture generator uses exact Python Fraction/integer arithmetic solely
as an independent test oracle; no host floating-point or BASIC evaluator runs
in the machine. Original decimal conversion uses bounded guest-stack integers;
no numeric tables or implementation code were obtained from external firmware. No emulator
source, ROM disassembly, recovered implementation, commercial programme,
reference palette, binary test suite, firmware numeric table or glyph data was
copied. Ordinary tests use generated ROM-shaped arrays and original programmes.
Manufacturer tables are used only as factual register/interface specifications.

- Commodore *64 Programmer's Reference Guide* (1982), chapter 3 pages
  [123](https://www.devili.iki.fi/Computers/Commodore/C64/Programmers_Reference/Chapter_3/page_123.html)
  and [136](https://www.devili.iki.fi/Computers/Commodore/C64/Programmers_Reference/Chapter_3/page_136.html):
  bitmap colour and sprite bit-pair/expansion facts. Chapter 5 pages
  [260](https://www.devili.iki.fi/Computers/Commodore/C64/Programmers_Reference/Chapter_5/page_260.html)–262:
  port/banking and I/O ranges. C64 Service Manual (1985), keyboard schematic:
  physical matrix wiring, independently encoded as coordinates.
- CSG/MOS *6526 Complex Interface Adapter*, November 1981, functional register,
  timer, serial, ICR, TOD/alarm and parallel-handshake sections
  ([archival scan](https://www.emuverse.ru/downloads/datasheets/peripherals/PIA/6526/mos_6526_cia.pdf)).
- MOS *6581 Sound Interface Device*, control-register, oscillator, envelope and
  filter descriptions ([manufacturer-sheet transcription](https://www.waitingforfriday.com/?p=661)).
  Integer filter and illustration palette are original approximations; no
  reference implementation's coefficients, tables or measured analogue model.
- Marko Mäkelä, *The memory accesses of the MOS 6569 VIC-II and MOS 8566 VIC-IIe*,
  3 June 1994 ([original hardware measurements](https://www.zimmers.net/anonftp/pub/cbm/documents/chipdata/pal.timing)):
  6569R3 PHI1/PHI2 slot facts, graphics pipeline, sprite Y offset, disabled sprite
  pointer reads and BA lead. Only observed facts were used; no attached test
  programme or implementation was copied. This does not qualify every revision.

Dated commands/results, source hashes and limitations are in
[testing](../../../../docs/testing.md). Stage-1 evidence describes its historical
source checkpoint; stage-2 evidence binds the extended sources.
