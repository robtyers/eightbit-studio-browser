# Original Dragon 32 monitor specification

Version 1; 14 October 2026. Implementation scope: ADR 0056, phases 0–4.

ADR 0057 subsequently extends the CPU and digital board. The foundation limits
below describe the original monitor increment; current hardware/BASIC status and
additional acceptance requirements are in `dragon32-basic-specification.md`.
The monitor commands and schema-1 control requests retain their meaning. Session
frames now additionally carry bounded audio under ADR 0057. Its latest source
explicitly configures the installed 32 KB RAM before accessing the workspace.

## Acceptance contracts

| ID | Contract |
| --- | --- |
| D32MON-01 | Original monitor assembly builds reproducibly into a 16384-byte image with source, symbols, digest and provenance. |
| D32MON-02 | Reset fetches the machine's mapped reset vector and executes actual 6809 code, displaying the original monitor banner from guest screen RAM. |
| D32MON-03 | Keyboard input passes through the Dragon PIA matrix; commands inspect/edit RAM, checksum/fill bounded ranges, load original code, execute it and recover through reset. |
| D32MON-04 | Execution is bounded, cancellable between slices, deterministic for identical input events, and stops explicitly on unsupported opcodes. |
| D32MON-05 | Browser monitor boot is explicit, needs no original ROM, uses the existing emulator surface, and tears down on Stop/replacement/suspension/unmount. |
| D32MON-06 | Native monitor boot uses a separate owned window; closure and shutdown release the session without affecting unrelated services. |
| D32MON-07 | Existing XRoar firmware selection, Run/Debug and project formats retain their meaning; monitor execution never claims to execute BASIC. |
| D32MON-08 | Invalid input, stale session commands, repeated lifecycle actions and failed starts fail safely; no host file/network/process authority enters the core. |

## Machine and execution

Dragon 32, 32 KiB RAM, 16-bit address space, 16 KiB monitor ROM. ROM occupies
the normal BASIC address window `$8000–$BFFF`; vector reads `$FFE0–$FFFF`
select its final 32 bytes. Cartridge space remains unpopulated. Default open bus
is `$FF`, an explicit experimental policy rather than a measured electrical
claim. RAM starts zeroed for repeatable cold reset; warm monitor reset preserves
RAM while reinitialising CPU/peripheral state. No Dragon 64 is inferred.

The CPU implements the documented instructions required by the monitor and its
original test programs. The coverage table below records the tested subset; remaining
instructions produce `DME1001` with the stopped address/opcode, without reading
or logging user source. Address and arithmetic wrapping follow 16-bit/8-bit
processor behaviour. Bus reads/writes advance the machine clock; remaining
instruction cycles advance devices without extra memory operations. Fine-grained
internal bus placement and interrupt timing remain qualification work.

The initial SAM model includes control bit set/clear pairs and display base
selection. Standard ROM/RAM mapping and PIA decoding are implemented. All-RAM
mode, RAM alias quirks, fast clocks and undocumented modes must be explicitly
tested before compatibility claims. Initial video supports the monitor's 32 by
16 text screen, 256 by 192 output, original 5 by 7 glyphs and inverse characters.
PAL pacing is nominal; board-revision-specific sync padding, graphics,
semigraphics, analogue colour artefacts and audio are later phase-5 gates.

PIA0 uses `$FF00–$FF03`, PIA1 `$FF20–$FF23`; mirrors follow decoded chip-select
regions. Register bit 2 selects data versus data-direction access. Keyboard rows
PA0–PA6 are pulled high unless a pressed switch connects to a driven-low column
PB0–PB7. The Dragon matrix is:

| Row | PB0 | PB1 | PB2 | PB3 | PB4 | PB5 | PB6 | PB7 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| PA0 | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
| PA1 | 8 | 9 | : | ; | , | - | . | / |
| PA2 | @ | A | B | C | D | E | F | G |
| PA3 | H | I | J | K | L | M | N | O |
| PA4 | P | Q | R | S | T | U | V | W |
| PA5 | X | Y | Z | Up | Down | Left | Right | Space |
| PA6 | Enter | Clear | Break | — | — | — | — | Shift |

PIA interrupt flags, external edges and CPU interrupt delivery remain unimplemented;
the monitor polls keyboard input and keeps interrupts masked. Joystick comparator,
cassette and printer inputs have declared idle values; their active behaviour
and audio are not implemented by this increment.

## Monitor

The monitor reserves `$0000–$03FF` for its workspace/stack and `$0400–$05FF`
for text. User RAM is `$0600–$7FFF`. Input is uppercase ASCII with a bounded
line; addresses/counts/bytes are hexadecimal. A rejected command leaves user RAM
unchanged. Range validation precedes fill/write/checksum/load operations; address
wrap and writes to firmware, vectors, I/O or monitor workspace are rejected.
Inspection of device registers is deliberately absent to avoid read side effects.

Commands: help, RAM/ROM dump, RAM edit, bounded fill/checksum, bounded hexadecimal
program load, execute user RAM, and reset. Exact command syntax is printed by the
firmware's help screen. Guest execution has no implicit return guarantee: reset
remains available if a program loops, damages its stack or uses unsupported CPU
features. The monitor remains distinct from the future BASIC image.

| Syntax | Behaviour and limits |
| --- | --- |
| `H` | Display command help. |
| `D address` | Dump eight bytes; start address `$0000–$BFF0`. |
| `M address byte` | Write one byte in `$0600–$7FFF`. |
| `F address count byte` | Fill `1–$0400` bytes wholly inside user RAM. |
| `C address count` | Print the modulo-256 additive checksum of the same bounded RAM range. |
| `L address hexbytes` | Validate then load contiguous hexadecimal pairs; at most 44 bytes with a four-digit address. |
| `G address` | Call user RAM with JSR; an RTS returns to the monitor. |
| `R` | Reset CPU/PIAs/SAM and monitor workspace/screen, preserving user RAM. |

The guest accepts at most 95 characters per line. Any additional character marks
the entire line invalid, including a syntactically valid prefix. The host accepts
at most 96 ASCII characters per submission so this failure boundary is testable.
Non-ASCII and unsupported physical keys are rejected before enqueueing. All
commands reject unexpected trailing input; loader records require complete pairs.

## Host boundaries

Versioned, typed commands carry a current session identity. The native adapter
owns a bounded execution task; browser execution owns a disposable worker.
The core accepts bounded cycles and key events, not paths, URLs, scripts or host
handles. Rendering presents Rust-produced pixels; JavaScript implements no
processor, keyboard-matrix, monitor or media semantics. Late events are ignored.
Blur releases physical keys without truncating an explicitly submitted command.
Stop/reset cancel all input. User text submission is a bounded keyboard sequence generated
by Rust and clocked into the matrix; no host trap implements firmware functions.
ADR 0057 adds `audioSampleRate: 48000` and at most 4096 signed 16-bit `audio`
samples to Advance responses. Other responses have empty audio. Older frames
without audio remain accepted by presentation. Sound is generated by the Rust
board and played through a bounded owned queue after user interaction; it is
discarded on reset, stop, failure, suspension and teardown.

No runtime persistence, automatic execution, BASIC loading, debugger claims,
firmware picker relaxation or settings migration is included. XRoar remains the
explicit existing BASIC/game runtime. The original monitor image is the only
bundled firmware permitted by ADR 0056.

## Reference register

Public accessibility does not establish an open redistribution licence. These
references supply factual interfaces only; no prose, sample implementation,
disassembly or glyph data is copied into shipped code.

| Reference | Use | Licence/provenance treatment |
| --- | --- | --- |
| Motorola M6809PM/AD (1981; May 1983 reprint), https://www.maddes.net/m6809pm/index.htm | Instructions, flags, addressing and documented cycle counts | Copyrighted manufacturer reference; independently written implementation/tests. |
| Motorola MC6821 datasheet, https://bitsavers.trailing-edge.com/components/motorola/_dataSheets/6821.pdf | Port, DDR and control register interfaces | Copyrighted manufacturer reference; factual register contract. |
| Motorola MC6883 datasheet, https://colorcomputerarchive.com/repo/Documents/Datasheets/MC6883%20Synchronous%20Address%20Multiplexer%20%28Motorola%29.pdf | SAM address selection and control bits | Copyrighted manufacturer reference; factual hardware contract. |
| Motorola MC6847 datasheet, https://colorcomputerarchive.com/repo/Documents/Datasheets/MC6847%20MOS%20Video%20Display%20Generator%20%28Motorola%29.pdf | Text geometry and signal interfaces | Copyrighted manufacturer reference; original glyph drawings. |
| Ciaran Anscomb, Dragon hardware details, https://www.6809.org.uk/dragon/hardware.shtml | Dragon wiring and recorded hardware uncertainties | Original technical observations; implementation code is not consulted. |
| Dragon Archive keyboard matrix, https://worldofdragon.org/index.php?title=Keyboard | Factual physical key positions | Independent table transcription; no emulator code. |

No XRoar/DragonPy/MAME/SAMx4 implementation or Microsoft BASIC implementation
is used. A later BASIC implementation decision requires its own provenance and
compatibility review. Research/legal review is not a claim of legal clearance.

## CPU qualification boundary

| Exercised by current tests and original monitor | Remaining qualification |
| --- | --- |
| Reset vector; immediate/direct/extended loads and stores; A/B/D/X/Y/U/S registers | Exhaustive opcode, addressing and flag combinations |
| Arithmetic/logic and condition flags; short/long branches; address wrapping | Fine cycle placement and independent reference/hardware comparison |
| Indexed offsets and auto increment; JSR/RTS; stack push/pull; transfers | Complete indexed-postbyte conformance, interrupt stack frames |
| Explicit unsupported-instruction failure and reset recovery | DAA, SWI/SWI2/SWI3, RTI, CWAI, IRQ/FIRQ/NMI and SYNC wake-up |

Decoding additional instructions does not establish their conformance. This
increment qualifies the original monitor journeys, not arbitrary 6809 software.

## Verification and remaining qualification

Tests cover instruction effects, bus selection, ROM protection, PIA DDR/data
selection, keyboard press/release, boot, command boundaries, original program
execution, unsupported instructions, reset/recovery and bounded repeated slices.
Headless tests use this original ROM and original programs. Native/WASM parity
compares the same cycle/input schedule. Firmware rebuilding uses separately
installed LWTOOLS and checks bytes against the retained original image.

Record commands and actual observations in `docs/testing.md`. Packaged native,
real browser, physical iPad, independent-emulator and physical Dragon acceptance
remain distinct. Do not infer complete CPU, BASIC, sound or historical-program
compatibility from monitor boot. Publishing remains separately authorised.
