# Original C64 runtime: compatibility specification and acceptance plan

The original C64 PAL CPU, digital board and guest firmware are implemented under
ADR 0062. Firmware 0.5 executes BASIC V2 statements/functions, numeric and string
values, arrays and collection, control flow, DATA/INPUT/GET, DEF FN, STOP/CONT,
keyboard/screen editing and the published KERNAL calls in original guest 6510
code. No host BASIC evaluator or reference implementation is used. The standard
PRG format and all 64 KiB physical guest RAM are retained.

Desktop **Run original C64 (development)** explicitly selects the bundled original
runtime in the existing emulator panel; ordinary Run retains configured VICE.
The browser **Project emulator** selector offers Original C64 (development), with
chips/supplied ROM retained as the default. Both use the same family-owned bounded
session and shared screen, keyboard, audio, Stop and Reset controls. Native
assembly still requires separately installed ca65/ld65; browser C64 authoring
remains BASIC. There is no C64 debugger, attached drive, cartridge or NTSC target.

This is a development compatibility contract, not universal historical-software
or silicon parity. Conservative guest stack limits, independently drawn glyphs,
original random sequences, named transcendental tolerances and six-digit TI$
validation are explicit differences. The sections below and [testing](testing.md)
record exact qualification and platform boundaries. Public
release, default promotion, signing and physical-device acceptance are separate.

## Outcomes and compatibility contract

| ID | Required outcome | Acceptance and boundary |
| --- | --- | --- |
| C64REF-01 | Dependable C64 PAL BASIC and restricted assembly development with VICE 3.10 and cc65 2.19 | Actual packaged create/edit/save/build/export/Run/Stop/Restart, real-tool tests and independent exported PRG playback |
| C64REF-02 | Video, input and sound observations with original programmes | Expected text/arithmetic, screen/colour RAM, sprite display, Q cleanup and SID tone; digital PCM and physical listening recorded separately |
| C64REF-03 | Actionable failure and recovery without stale launches or overwritten exports | Missing/wrong tool and ROM, assembler rejection, stale source/build, cancellation, repeated sessions and correction tests |
| C64CPU-01 | Independently authored, reusable NMOS 6502 execution and 6510 port behaviour | Original instruction/flag/cycle/bus vectors, decimal arithmetic, branch/page boundaries, stack, reset, IRQ/NMI, wrapping and bounded failure behaviour |
| C64HW-01 | PAL C64 digital board with all 64 KiB RAM and public banking | CPU/VIC memory views, ROM and RAM-under-ROM, colour RAM nibbles, 6510 direction/data port and PLA mapping tests |
| C64HW-02 | VIC-II display, raster/interrupt and bus ownership | Text, bitmap, multicolour, sprites/priority/collision, scrolling, border, raster IRQ and bad-line/sprite DMA vectors plus observed frames |
| C64HW-03 | CIA timers, interrupts, time-of-day and input; SID synthesis and machine-side I/O pins | Keyboard/joystick matrices, timer/IRQ/NMI interactions, PAL timing, bounded audio, three voices/envelopes/filter and cassette/serial signal tests |
| C64ROM-01 | Independently authored guest BASIC/OS/editor/character firmware | Source/builder/image identity, original provenance, deterministic builds, public entry/workspace/banking contract and safe recovery |
| C64BAS-01 | Commodore BASIC V2 language and public memory behaviour | Statements/functions, numeric representation, strings/arrays/garbage collection, DATA, control flow, editor, syntax/runtime errors, STOP/CONT and recovery corpus |
| C64KERNAL-01 | Published OS entry/vector behaviour and screen/keyboard/clock services | Explicit register/flag/memory side-effect vectors for supported public calls; unsupported devices give documented errors |
| C64DEV-01 | Original desktop/browser runtime uses the existing Studio shell and ownership contracts | Native/WASM parity, inert opening, fresh-build explicit Run, keyboard/audio ownership, Stop/Reset, timeouts, suspension, cancellation and stale-result tests |
| C64MEDIA-01 | Existing schema-1 projects and stock-compatible PRGs remain portable | Byte/manifest compatibility, guarded exports, VICE plus stock-ROM playback; physical transfer/hardware is a separate gate |
| C64COMP-01 | Existing external runtimes, exact-ROM checks and other products remain compatible | Distinct firmware identities, explicit retained alternatives, regression/architecture checks, ordinary gate and affected packaged products |

These rows are coverage targets; the execution record below identifies the
implemented and observed subset. Undocumented CPU
opcodes, SID revisions/analogue effects and electrical edge cases need an explicit
coverage matrix. Unsupported behaviour must be diagnosed; it must not become a
silent NOP or a universal historical-software compatibility claim.

## Completion ledger — firmware and Studio delivery

The user approved completion of original guest firmware and native/browser Studio
integration, retaining the existing alternatives and defaults. The named firmware
and integration contract is implemented and locally qualified on Apple Silicon
macOS. The dated [delivery receipt](evidence/c64-original-firmware-studio-20261021.json)
records guest/native/WASM, reference-interface, real VICE, ordinary, packaged and
source-rebuild evidence separately. Earlier checkpoints below remain historical.

| Area | Current state | Qualification |
| --- | --- | --- |
| CPU/PAL hardware | Implemented, named stage-1/2 evidence | Retain regression matrix and explicit digital/analogue limits |
| BASIC V2 statements/functions | Implemented | Shared original/supplied-ROM language corpus; 14,594 exact numeric vectors; named transcendental tolerances |
| Loops, DATA and memory | Implemented | FOR/NEXT/ON/READ/RESTORE, SYS/USR/PEEK/POKE/WAIT, real guest stack and recovery |
| Strings/arrays | Implemented | Packed layouts, scalar insertion, temporary roots, multidimensional arrays and moving collection |
| Interactive language/editor | Implemented | INPUT/GET, STOP/CONT, LIST ranges, abbreviations, re-edit, quote/insert/delete/control modes |
| KERNAL and channels | Implemented for the no-peripheral target | 39-call fixture, six BASIC callbacks, CHRGET/CHRGOT, guest keyboard/screen and absent-device errors |
| Original combined runtime | Native/WASM corpus passed | Original-only BASIC, sprites, SID, physical-key input, SYS/RTS and lifecycle; identical named observations |
| Studio composition | Implemented and locally qualified | Explicit native/browser routes, retained defaults, source-bound PRG loads and ownership; ordinary gate, all three macOS packages and extracted-source rebuild |

No public release, signing, remote push, default promotion, attached-drive
implementation, NTSC or new project/export contract is implied. Physical and
platform acceptance remain separately identified evidence classes.

## Fixed boundaries

- Preserve `c64.toml` schema 1, C64 PAL identity, PRG load addresses, BASIC tokens,
  current source limits and guarded build/export semantics. No new project format
  or NTSC/other machine is implied.
- Preserve all physical guest RAM, CPU/VIC memory views, published workspace,
  KERNAL vectors and ordinary machine banking. Internal Commodore ROM routines
  are not automatically public compatibility promises: inventory required entry
  points from published interfaces and named acceptance programmes.
- BASIC executes in original guest code. The native host/React UI must not
  evaluate BASIC or secretly fall back to VICE or chips.
- The proposed core runs without application, process, network or filesystem
  access. Host adapters own bounded frames/audio/input, cancellation and lifecycle.
- Preserve the existing embedded/separate-window presentation contract for future
  first-party native sessions. External VICE keeps its current separate window.
- Retain ca65/ld65 initially. A first-party assembler, mixed/multi-file source,
  integrated debugger, attached 1541/DOS, cartridges, network/host serial access
  and new disk/tape export workflows need separate scope. Machine-side pins and
  explicit unsupported-device responses do not claim a connected peripheral.

## Firmware storage checkpoint

Before guest firmware implementation, document the public address/entry-point map,
workspace/numeric representation, language coverage and estimated storage needs.
Determine whether complete behaviour fits the stock 8 KiB BASIC, 8 KiB KERNAL and
4 KiB character windows. This is a feasibility measurement, not an imposed total
implementation-size constraint.

If a larger container is required, bring a bounded private-page design to the
checkpoint. It must specify visible fixed entries, read/write-under-ROM behaviour,
CPU/VIC access, private gate authority, nesting, IRQ/NMI, non-local error recovery,
Reset, firmware identity and native/WASM parity. Do not reserve guest RAM or
expose extension-dependent calls in stock PRG exports. Private pages require a
specific approved amendment; other families' decisions do not authorise them.

### Stage-3 storage/interface checkpoint — 20 October 2026

**State:** the user authorised continuation after committing stages 0–2 as
`1b6269975`. The interface inventory and reference probes below are implemented;
the user subsequently explicitly approved the expanded storage design under ADR
0062. Bounded containers/gates and a two-page original development foundation are
implemented and have native/WASM evidence. That historical storage/boot image had
no BASIC interpreter; the subsequent front-end checkpoint below is explicitly
partial and does not complete the language.
Complete stock-window fit for the complete language remains
unproved: neither the existence of the historical ROM nor a planning estimate
measures the eventual independently authored implementation.

#### Public address and workspace inventory

The CPU keeps its existing eight no-cartridge mappings. BASIC occupies
`$A000–$BFFF`, KERNAL `$E000–$FFFF`, and characters `$D000–$DFFF` when selected;
writes under BASIC/KERNAL still reach the original physical RAM. The VIC retains
its independent bank/character view. Sources: Commodore's
[PRG pp. 260–262](https://www.devili.iki.fi/Computers/Commodore/C64/Programmers_Reference/Chapter_5/page_260.html)
and [Service Manual p. 7](https://www.devili.iki.fi/Computers/Commodore/C64/Service_Manual/Page_07.html).

| Public area | Required contract |
| --- | --- |
| `$0000/$0001` | Existing 6510 direction, banking and cassette pins |
| `$002B–$0038` | Text, scalar, array and string boundaries; BASIC memory limit |
| `$0039–$0042` | Current/previous line, CONT and DATA positions |
| `$0061–$0070` | FAC1, FAC2, signs and rounding workspace |
| `$0073/$0079/$007A–$007B` | Original CHRGET/CHRGOT implementation and text pointer, retaining patchable RAM entry behaviour |
| `$0090/$0099/$009A/$00A0–$00A2` | Status, selected channels and jiffy clock |
| `$00C6/$00D1–$00D6` | Keyboard count and screen/editor position state |
| `$00FB–$00FE`, `$C000–$CFFF` | User RAM; never private firmware storage |
| `$0100–$01FF`, `$0200–$0258` | Existing processor stack and input workspace |
| `$0277–$0280`, `$0281–$0284` | Keyboard queue and OS memory bounds |
| `$0300–$030B`, `$030C–$0312` | Patchable BASIC vectors, SYS registers and USR vector |
| `$0314–$0333` | IRQ/BRK/NMI and vectored KERNAL services |
| `$033C–$03FB`, `$0400–$07FF` | Tape buffer, screen and sprite pointers remain guest-owned in their existing roles |
| `$0801–$9FFF` | Default BASIC programme/variable/string space; no hidden reservation |
| `$A000` | BASIC cold-start indirect entry |
| `$FF81–$FFF3`, `$FFFA–$FFFF` | Fixed KERNAL public jumps and CPU vectors |

This is an implementation checklist, not evidence that every entry already works
with original firmware. The remaining named public fields in
[Commodore's RAM map, p. 311](https://www.devili.iki.fi/Computers/Commodore/C64/Programmers_Reference/Chapter_5/page_311.html)
must retain their stated roles when their owning service is implemented. Do not
allocate private scratch space from fields labelled unused. The public
[power-up contract, p. 269](https://www.devili.iki.fi/Computers/Commodore/C64/Programmers_Reference/Chapter_5/page_269.html)
defines initialisation and BASIC entry. Cartridge autostart remains outside this
no-cartridge target. Additional fixed BASIC math entries need a specific public
reference and vector before being claimed; unspecified ROM internals are not a
compatibility promise.

All 39 published KERNAL calls must have separately tested parameter, flag,
register, workspace, vector-hook and error behaviour. The address inventory is
from [Commodore's table, p. 272](https://www.devili.iki.fi/Computers/Commodore/C64/Programmers_Reference/Chapter_5/page_272.html):

| Calls | Fixed addresses, in the same order |
| --- | --- |
| CINT, IOINIT, RAMTAS, RESTOR, VECTOR, SETMSG | `$FF81,$FF84,$FF87,$FF8A,$FF8D,$FF90` |
| SECOND, TKSA, MEMTOP, MEMBOT, SCNKEY, SETTMO | `$FF93,$FF96,$FF99,$FF9C,$FF9F,$FFA2` |
| ACPTR, CIOUT, UNTLK, UNLSN, LISTEN, TALK | `$FFA5,$FFA8,$FFAB,$FFAE,$FFB1,$FFB4` |
| READST, SETLFS, SETNAM, OPEN, CLOSE, CHKIN | `$FFB7,$FFBA,$FFBD,$FFC0,$FFC3,$FFC6` |
| CHKOUT, CLRCHN, CHRIN, CHROUT, LOAD, SAVE | `$FFC9,$FFCC,$FFCF,$FFD2,$FFD5,$FFD8` |
| SETTIM, RDTIM, STOP, GETIN, CLALL, UDTIM | `$FFDB,$FFDE,$FFE1,$FFE4,$FFE7,$FFEA` |
| SCREEN, PLOT, IOBASE | `$FFED,$FFF0,$FFF3` |

Disconnected-device calls retain their documented carry/accumulator or READST
error route; they must not fabricate a successful transfer. Keyboard and screen
channels remain real guest services. No drive, host filesystem, serial device,
new media loader or tape workflow is added by implementing the call inventory.

#### Numeric, string and array representation

Use guest 6510 arithmetic and the public two-byte integer/five-byte floating
formats, never host `f64` evaluation. Commodore's
[PRG p. 5](https://www.devili.iki.fi/Computers/Commodore/C64/Programmers_Reference/Chapter_1/page_005.html)
specifies integer range and storage; [pp. 6–7](https://www.devili.iki.fi/Computers/Commodore/C64/Programmers_Reference/Chapter_1/page_006.html)
describe display and variable-name behaviour.

The original keyboard-only `rom-interfaces` probe assigns `0,1,-1,.5,1.5,2`,
signed integer boundaries, `"ABC"`, and a two-element array. It then uses guest
DATA/POKE to install an original 6510 subroutine and passes `1.5` and `-1.5`
through USR. Only bounded public RAM data records are observed. There is no host
programme injection, firmware patch, disassembly or ROM data extraction.

Observed contracts, identical in native and WASM:

- Scalar records have two name/type bytes followed by five value bytes. Integer
  values `32767/-32768` use `7F FF/80 00`; the type markers occupy the name bytes.
- Packed floats `0,1,-1,.5,1.5,2` are respectively `00 00 00 00 00`,
  `81 00 00 00 00`, `81 80 00 00 00`, `80 00 00 00 00`,
  `81 40 00 00 00`, `82 00 00 00 00`. The planned representation is a zero
  exponent for zero, a biased binary exponent, sign in the packed mantissa's
  high bit and 31 stored fraction bits with the normalised leading bit implicit.
  These vectors do not qualify every exponent, tie, underflow or overflow case.
- The string descriptor contains length and a little-endian pointer to the three
  original text bytes. Its allocation address is an observation, not a promised
  constant. Strings must ultimately support 0–255 bytes and moving collection.
- `DIM X(1)` produces two five-byte elements; its header records a little-endian
  byte span, dimension count and big-endian element count. Multi-dimensional,
  integer/string arrays and allocator exhaustion remain future vectors.
- USR receives an unpacked leading mantissa bit at `$62`, with a separate sign
  at `$66`. The six observed bytes for `1.5/-1.5` are
  `81 C0 00 00 00 00/81 C0 00 00 00 FF`. The unchanged FAC returns to the correct
  packed scalar value. See the published
  [USR contract, p. 307](https://www.devili.iki.fi/Computers/Commodore/C64/Programmers_Reference/Chapter_5/page_307.html).

Before qualification, extend this corpus to decimal conversion/rounding, all
numeric limits, coercions and precedence, string temporaries/aliasing/collection,
multi-dimensional arrays, stack limits and patchable public vectors. Public
numeric layout and language behaviour must agree; matching the displayed number
alone is insufficient.

#### Language coverage and size assessment

The existing original tokeniser defines **76 tokens, `$80–$CB`**. All remain stage-3
targets: execution/control (`END` through `NEW`, including file statements,
`SYS`, `GET`, `DEF FN`, `STOP/CONT`, `ON`, DATA and recovery), syntax/operators
(`TAB(`, `TO`, `FN`, `SPC(`, `THEN`, `NOT`, `STEP`, arithmetic, logical and
relational operators), and every numeric/string function through `GO`.
Also cover literal punctuation, `?`, implicit LET, two-character names, `%/$`
types, `ST`, `TI/TI$`, the PI character, direct/programme modes, LIST/edit/delete,
abbreviations, quote/DATA/REM tokenisation and documented BASIC V2 errors. The
editor's stock logical-line limits are distinct from the existing Studio source
compiler limits; neither contract may silently change the other.

Exact proposed data allowances, calculated from our token list and public sizes:
NUL-terminated keyword spellings **331 bytes**, 76 two-byte dispatch entries
**152 bytes**, 39 KERNAL jumps **117 bytes**, CPU vectors **6 bytes**, and two
256-glyph eight-row original character sets **4096 bytes**. This **4702-byte**
subtotal is not executable firmware or a complete data budget. Error strings,
numeric coefficients, keyboard tables, public stubs and alignment are additional;
all must be authored/generated independently.

The following are engineering planning estimates in KiB, **not assembled sizes**:

| Guest implementation area | Initial range |
| --- | --- |
| Tokeniser, LIST, editor-to-parser and expression dispatch | 4–8 |
| Statements, control frames and DATA | 3–6 |
| Numeric arithmetic, conversion and formatting | 5–10 |
| Transcendentals and random-number behaviour | 3–7 |
| Scalars, strings, arrays and garbage collection | 4–8 |
| Channels, device failures and language error recovery | 3–5 |
| KERNAL, screen/editor and keyboard services | 4–8 |
| Page/public gates, tables and alignment | 1–3 |
| **Code/data estimate excluding characters** | **27–55** |

These conservative estimates allow straightforward independently authored
routines rather than an assumed match to the historical implementation's density.
They suggest allowing more storage, but **do not prove a stock-window overflow**.
The stock route has 16 KiB for code/data plus 4 KiB for characters. A genuine fit
decision requires the complete linked implementation and language/error corpus.
Do not claim stock-replacement feasibility or invent a measured code size now.

#### Approved storage contract — foundation implemented

1. **Container and identity.** Permit 1–16 exact 8192-byte BASIC pages, one fixed
   8192-byte KERNAL, one 4096-byte character image and at most 16 KiB metadata:
   a **143360-byte payload / 159744-byte total ceiling**. Metadata versions the
   format and binds source, deterministic builder/tool identity, every image,
   public map, replicated stubs and at most 128 fixed gate sites. Validate lengths,
   counts, hashes, page targets and non-overlap before constructing a machine.
   Use a distinct original development identity; it cannot pass the legacy
   supplied-ROM identity route. Actual linked maps report used/padded bytes per
   page and reject overflow. The ceiling is not a requirement to fill 16 pages.
2. **Mapping.** Only CPU reads of mapped `$A000–$BFFF` select the private page.
   KERNAL, character selection, RAM-under-ROM, all 64 KiB RAM, VIC fetches, bus
   clocks and normal `$0000/$0001` banking retain their existing rules. Page zero
   is the public BASIC view. All accepted fixed BASIC entries have byte-identical
   original stubs at their published addresses in every page. Private code and
   tables may differ; arbitrary reads of unspecified BASIC ROM internals are not
   promised to match historical ROM contents.
3. **Guest gate authority.** Generated 6510 code in fixed KERNAL performs calls and
   returns. A default-off CPU bus hook identifies the currently executing
   instruction without changing opcode semantics. Only a manifest-listed write
   instruction fetched from validated, currently mapped immutable KERNAL code
   may use private `$DE00–$DE02` controls. The manifest fixes each operation and
   target; guest register data cannot select an arbitrary page. Other origins,
   DMA, data reads, invalid gate bytes and legacy firmware retain existing
   expansion-I/O behaviour. Jumping into a gate's middle cannot acquire an
   unmatched continuation: enter/leave operations check gate identity, expected
   sequence and the pending return. No host BASIC or arbitrary host service is
   introduced. This changes only the explicitly original machine variant.
4. **Nesting and guest state.** At most 32 private page-continuation records store
   page/gate/return identity and recovery generation. They hold no BASIC values,
   variables, strings, evaluation stack or guest workspace. Guest registers and
   flags are preserved by the guest gate according to its declared calling
   convention; ordinary 6510 JSR/RTS still use `$0100–$01FF`. Check both the guest
   stack budget and private depth before changing page state. Invalid entry,
   underflow, overflow or stale return gives a stable machine fault with bounded
   cleanup, never a wrapped page index or silent continuation.
5. **Interrupts and callbacks.** IRQ/BRK/NMI vectors and their gate prologues remain
   fixed KERNAL code. Interrupt entry atomically saves the interrupted private
   page and selects page zero before calling any user RAM vector. RTI restores
   the matching page; nested NMI uses its own bounded record. SYS, USR, CHRGET
   patches and all user vector callbacks similarly expose page zero to user code,
   then restore the caller's page on a valid return. Test interruption before and
   after each control write, cross-page calls and RTI/RTS. A guest that banks ROM
   out keeps that real banking behaviour; the host must not force ROM visible.
6. **Recovery and lifecycle.** Non-local language errors and warm-start/STOP return
   unwind abandoned private continuations through fixed recovery code and select
   page zero; CONT uses published guest state, not hidden host evaluation state.
   Reset revokes all records/generations before CPU/device reset. Host Stop or
   cancellation freezes the matching CPU/page state and releases input/audio;
   resume preserves it. A machine fault revokes continuations, clears audio/input
   and requires Reset. Stale callbacks cannot restore a revoked page.
7. **Public outputs and test matrix.** Exports remain existing stock BASIC tokens
   and PRGs with no private gate/page calls. No project-schema, source-language,
   debugger or product-default change is included. A complete paged image cannot
   run in stock VICE: mark that matrix cell unavailable. Compare public vectors
   and exported original programmes with stock-ROM VICE instead; never label
   that whole-image parity. Run every original gate, interrupt, error, recovery
   and guest-language vector on both actual native and WASM builds.

#### Implemented container schema 1

The `original::Image` parser accepts a borrowed immutable container and a trusted
complete-container SHA-256 from its caller. Self-declared provenance is not an
identity grant. A future product composition must supply an independently
qualified build identity, not trust an arbitrary image's own claims. Parsing does
not open files, execute code, allocate an image copy or change a legacy identity.
Wrong size/map/version uses `DC64M09`, identity mismatch `DC64M10`, invalid gate
code/continuation `DC64M11`, and gate/guest stack exhaustion `DC64M12`.

All integers below are little-endian. Every reserved byte must be zero; trailing,
unknown or overlapping metadata is rejected.

| Header offset | Field |
| --- | --- |
| 0–7 | `EBC64R01` magic |
| 8–9 | Schema 1 |
| 10, 11 | BASIC page count 1–16; gate count 0–128 |
| 12–13, 14–15 | Public-range count 1–128; reserved |
| 16–47, 48–79 | Source and builder SHA-256 |
| 80–111 | Gate/public-range map SHA-256 |
| 112–143 | Complete image-payload SHA-256 |
| 144–147, 148–151 | Exact metadata and payload lengths |
| 152–159 | Reserved |

The header is followed by 32-byte hashes for each BASIC page, then KERNAL, then
characters. Gate records are eight bytes: entry `u16`, target `u16`, page `u8`,
kind `u8` (0 call, 1 interrupt, 2 recovery), two reserved bytes. Public records
are four bytes: BASIC-window address `u16`, length `u8` (1–16), reserved byte.
Records are strictly ordered by address and do not overlap. The public map must
cover `$A000–$A003`; each declared range must be identical across every page.
Gate instruction templates are checked in fixed KERNAL below `$FF81`; targets
must be ROM code, never the middle of another gate. Interrupt/recovery gates
select page zero. Every per-window, payload, map and trusted image hash is checked.
Payload order is BASIC pages, KERNAL, characters. At maximum counts schema 1
uses 2272 metadata bytes and 145632 total bytes, within the approved ceilings;
it does not accept arbitrary padding up to those ceilings.

`Bus::execution` adds zero-cycle generic observations to the CPU, with default
no-op behaviour. It cannot replace instructions, mutate registers or introduce
host evaluation. The board admits continuations only after real guest JSR,
declared public-stub jumps or hardware/BRK vector completion. Prefix/epilogue
order, stack position and return instructions are checked; nested interrupts
preserve already-admitted prefixes. Direct/stale entry faults and revokes the
original page context. Gate control requires BASIC, KERNAL and I/O mapped; a
hidden-I/O gate faults before it could silently write RAM-under-I/O. Other CPU
accesses retain the physical banking rules. Lookup uses the sorted bounded map.

There are at most 32 page-continuation records, including pending gate prefixes.
No BASIC scalar, string, expression value or workspace is stored there. Record
revocation rather than reusable external continuation tokens prevents stale
restoration after error/fault/Reset. Recovery selects zero and discards abandoned
records; the guest recovery target resets its own processor stack. Host Stop and
cancel retain CPU/page state while releasing input/audio; faults require Reset.
The guest IRQ/NMI handler constructs a synthetic status/return frame so a user
RAM handler's ordinary register pulls and RTI return through the fixed epilogue;
the final guest RTI consumes the real hardware frame. No host register injection
or forced ROM banking is used.

Returning gates bind the actual caller return PC and stack position, including
the hardware interrupt frame; a wrapped caller stack or altered return faults
before it can become a valid private continuation. User interrupt callbacks see
the real interrupted status byte in their synthetic frame. Arbitrary interrupt
return-PC edits or non-local stack surgery are not qualified by the normal
register-pull/RTI vector; stale/private bypasses fault. Recovery has its separate
explicit non-local gate rather than pretending a mismatched return succeeded.

The alternative is to restrict firmware to the stock windows and measure/optimise
until a complete implementation fits. That preserves potential whole-ROM VICE
execution but has unmeasured engineering cost and no demonstrated fit. The
recommended bounded-page allowance avoids making historical code density a
requirement while preserving the public machine and export contracts. The user
explicitly approved this bounded allowance and original guest implementation.
Product composition, default promotion, further commits and publication remain
separate authorities.

The approved delivery order is storage/gate hostile-input and interrupt/recovery
vectors first, then original reset/KERNAL/editor/glyphs, then the complete language and
public-interface corpus. A boot screen or partial interpreter is a named
development checkpoint only. Stage 3 remains open until its full exit criteria
are met. Source and current observations are recorded in
`docs/evidence/c64-stage3-interface-checkpoint-20261020.json`; historical stage-1/2
receipts retain their original source hashes.

## Independent authorship and references

Use original Commodore publications as factual interface references, including
the *Commodore 64 Programmer's Reference Guide* (1982), the original C64 user
manual and MOS 6510/6569/6526/6581 data sheets. Record edition, page/section and
the fact used when each contract/vector is implemented. These publications are
references, not reusable implementation, prose, numeric tables or glyph data.
The VICE public manual describes the separately installed reference tool:
<https://vice-emu.sourceforge.io/>. The pinned assembler release and its reported
V2.18 identity are documented at <https://github.com/cc65/cc65/releases/tag/V2.19>.

Keep implementation work separate from reference disassemblies/emulator code.
Record provenance per source/fixture; use original programmes and independently
generated expected observations. Exact-hash supplied ROM execution remains opt-in,
external and observation-only. No vendor ROM bytes, commercial programmes or
third-party-generated binaries become ordinary fixtures. Review provenance and
distribution permissions before any future original-runtime release; tests alone
do not establish permission or compatibility.

## Stages and exit criteria

| Stage | Deliverable | Exit criteria |
| --- | --- | --- |
| 0 — recorded | Third-party baseline plus this ADR/specification/acceptance plan | C64REF evidence recorded, missing checks explicit, implementation checkpoint reviewable |
| 1 — approved and implemented, 20 October 2026 | Pure original CPU/6510 foundation and machine bus | Independent CPU/bus/decimal/interrupt vectors; deterministic bounded execution; native/WASM build parity |
| 2 — approved and implemented, 20 October 2026 | PAL machine/video/input/audio with supplied-ROM qualification | Named public hardware contracts, repeatable frames/register/input/audio observations and lawful stock-ROM runs; no bundled reference ROM |
| 3 — approved and in progress; storage/boot foundation implemented | Original guest firmware after storage checkpoint | Full named language/OS/editor corpus, errors/recovery, source-bound manifests, no supplied-ROM requirement and public-interface comparisons |
| 4 — approved and implemented; named acceptance gates | Native/browser Studio composition and qualification | Actual BASIC/assembly PRG journeys, retained alternatives, performance and lifecycle gates, native/WASM parity, ordinary gate and affected packages |

A monitor or partial language can be an explicitly labelled development checkpoint;
it cannot satisfy stages 3/4 or become a qualified default. Each implementation
stage must identify its precise supported subset and unresolved failures.
This plan does not authorise publication or default promotion.

## Reference matrix and evidence

| Emulator | Firmware | Evidence sought |
| --- | --- | --- |
| VICE | Exact supported Commodore set | Baseline source/media/runtime behaviour |
| Original | Exact supported Commodore set | Hardware/CPU isolation; supplied firmware stays external |
| VICE | Original firmware | Firmware isolation only if the approved image uses stock memory mapping; otherwise explicitly unavailable |
| Original | Original firmware | Combined native/WASM and actual Studio acceptance |

If a storage extension prevents a complete original image from running in VICE,
compare named public-interface vectors and stock-compatible exported programmes
against the baseline. That is behavioural evidence, not whole-image VICE parity.
Keep per-programme inputs, expected observations, target/ROM/source/build hashes,
commands, platform and result. Normal CI uses generated/original fixtures; real
tools and supplied ROMs are opt-in. Capture digital SID samples separately from
speaker/headphone listening. Record browser, packaged macOS, Linux, Windows and
physical C64 acceptance separately; none inherits another's result.

## Current execution record

See `docs/testing.md` and `docs/evidence/c64-third-party-macos-20261020.json` for
stage-0 observations.

Stage 1 adds `eightbit-mos6502-core` (151 documented NMOS instructions, fallible
cycle-observable bus, reset/stack/interrupt and bounded/cancellable stepping) and
`eightbit-commodore-machine` (all 64 KiB RAM, six-pin 6510 port, eight no-cartridge
banking combinations, RAM-under-ROM, colour storage and distinct VIC memory view).
Original tests and the compiled native/WASM parity command are documented in
those crates and `docs/testing.md`. C64CPU-01 and the memory portion of C64HW-01
have foundation evidence; the remaining compatibility rows are planned.

Stage 2 extends the same family crate with the original `pal`, `vic`, `cia`,
`sid` and physical `input` modules. It owns 63×312 PAL clocks, matrix/glyph/sprite
fetches, bounded frame/audio buffers, DMA waits and input release. Optional bus
signals extend the CPU without changing caller-driven buses; normal instruction
IRQ sampling, NMI pulse capture and pre-vector BRK/IRQ redirection have original
vectors. The board implements all normal text/bitmap/multicolour/extended modes,
scrolling/borders, sprite priorities/collisions/expansion, raster/light-pen IRQs,
CIA timers/cascade/CNT/serial/TOD/alarm/strobes, keyboard ghosting/joysticks, SID
voices/envelopes/filter and machine-side cassette/IEC/user-port drivers.

The exact supported reference ROMs cold-boot through guest code; the original
acceptance programme is entered through physical matrix keys. No host BASIC
evaluator, ROM patch, source/PRG injection or guest-buffer write is used in that
qualification. Guest DATA/POKE installs original assembly for SYS/CHROUT/RTS;
the absent IEC drive produces a guest error. Reset, cancellation, stopped/faulted
states and bounded audio/frame retention are separate vectors. Native/WASM
observations compare byte for byte, including complete frame/PCM hashes.
Commands, observations and source digests live in `docs/testing.md` and
`docs/evidence/c64-stage2-pal-20261020.json`.

### Stage-2 coverage limits

- The PAL development schedule follows nominal 6569R3 PHI2 slots. Half-dot colour
  changes, revision-specific VIC glitches, sprite edge/wrap tricks, illegal-mode
  analogue output and adversarial mid-line DEN/scroll/DMA races are unqualified.
  Sprite pointers are fetched even when disabled; graphics/C/refresh accesses and
  all eight sprite DMA slots have explicit clock-domain vectors. A pathological
  late DMA write conflict uses a bounded hold, not an electrical collision model.
- IRQ polling covers normal penultimate-cycle cases and the retained CLI/SEI/PLP
  mask behaviour. Every branch-edge interrupt race, SO, undocumented opcodes and
  invalid-BCD hardware results remain unqualified; unsupported opcodes fault.
- CIA functional register/clock contracts are implemented, not every 6526 revision's
  timer start/reload/ICR race or TOD erratum. Digital serial/parallel/cassette/IEC
  pins do not imply an attached drive, tape, printer or user-port peripheral.
- SID oscillator timing, normal ADSR, three voices and filter routing are tested.
  Combined/noise edge cases, filter transfer/resonance, DAC/DC/volume quirks,
  bus decay and revision-specific analogue sound remain approximations. The RGB
  observation palette is original/illustrative, not calibrated PAL colour.
- Physical power-up RAM patterns, floating/unbonded 6510 pins and capacitor decay
  remain unqualified. RAM-under-port writes follow the last VIC-driven bus byte;
  that digital rule does not claim electrical/chip-revision qualification.
- Stage 2 alone did not implement original BASIC/KERNAL/editor/glyph firmware, media import/autoload,
  original assembler/debugger, product runtime registration or UI acceptance.
  Linux/Windows runtime, graphical-browser UI, physical hardware and distribution
  acceptance remain separate. Existing VICE/chips defaults and exact ROM checks
  are retained. Stage 3 extends this with the explicitly approved storage and boot
  foundation below; its complete language/editor exit criteria are not yet met.

### Stage-3 storage and original firmware foundation

The user explicitly approved the checkpoint's bounded storage design. The core
now validates and executes original containers with guarded guest gates, all RAM
and VIC mappings intact. Generated vectors cover maximum counts, malformed and
internally rehashed images, nested calls, NMI at every gate instruction, nested
NMI, IRQ/BRK, gate order/stack/hidden-I/O rejection, recursion bounds, non-local
recovery, stale returns and owned Stop/cancel/fault/Reset. The CPU has independent
observer/cycle/fault vectors. Native/WASM page traces match exactly.

`families/commodore/firmware/original` contains independently authored 6510 reset,
KERNAL/editor foundation and development lettering. The deterministic Python
builder stages its exact source snapshot, uses explicit separately installed
ca65/ld65, bounds tool output/time and publishes a single complete immutable image
without replacement. Its source/tool/builder/image/map manifest and exact image
identity are recorded in `docs/evidence/c64-original-storage-boot-20261020.json`.
That storage/boot checkpoint has two BASIC-window pages, a fixed KERNAL and original character
image, with a banner deliberately saying **FOUNDATION - NO BASIC**.

Actual native/WASM runs boot this image without supplied ROMs, type through the
physical matrix and echo text, observe the approximately 60 Hz clock, check page
restoration and repeat Stop/cancel/Reset. Separate original assembly fixtures
explicitly initialise user RAM and call SCREEN, IOBASE, MEMTOP/MEMBOT, SETLFS,
SETNAM, LOAD/READST, PLOT, CHROUT, GETIN, SETTIM/RDTIM, VECTOR and RESTOR. A user
NMI callback returns with RTI during a nonzero-page call. These are named vectors,
not a complete KERNAL equivalence claim. The boot/keyboard run injects no host
programme/workspace bytes. The retained original illustrative frame is
`docs/evidence/c64-original-foundation-20261020.png`.

At that checkpoint the full BASIC interpreter, numeric/string/array behaviour,
screen line editor, repeat/quote/insert/CTRL/Commodore modes, complete PETSCII
graphics and full KERNAL parameter/error corpus remain open. CHRIN is currently
one blocking character. File calls have no attached medium and only the named
absent-device observations. This foundation cannot complete C64BAS-01 or stage 3,
become a qualified default, or imply Stage-4 UI/media acceptance. Stock-window
feasibility of the complete language is still unmeasured. Product defaults,
projects, PRG exports, supplied-ROM checks and explicit VICE/chips routes remain.

### Stage-3 partial guest BASIC front end — historical 0.2 checkpoint

The user's instruction to continue extends the approved firmware work. Original
`basic.s` now reads at most 80 physical input characters into the public input
buffer, compresses all 76 BASIC V2 keyword assignments in place and preserves
quote/DATA/REM literals. `?` is PRINT. The portable Commodore core fixture
`basic-v2-keywords.txt` contains only the published token spellings; the deterministic builder emits numeric byte operands, without
including external implementation or macro libraries. Input/utility/temporary
workspace follows the [PRG RAM map](https://www.devili.iki.fi/Computers/Commodore/C64/Programmers_Reference/Chapter_5/page_311.html);
no user RAM or host BASIC state is reserved.

Numbered lines 0–63999 are inserted, replaced, deleted, compacted and relinked in
ascending order. Empty numbered input deletes that line. Preflight checks the
complete increasing linked list, line order, lengths/terminators and public
TXTTAB/VARTAB/MEMSIZ boundaries. A failed capacity check moves no programme
bytes or VARTAB. Successful editing clears the future variable/array/string and
CONT boundaries; it does not implement those language services. LIST expands
tokens only outside quoted/DATA/REM text; NEW clears text. Stored output uses
the existing stock PRG contract, including the final two zero bytes. Explicit
generated fixtures cover 255-byte existing records and exact `$A000` capacity;
these are distinct from the physical-keyboard-only language journey.

The executed slice is direct and numbered string-literal PRINT (including
semicolon/comma zones), colon-separated statements, RUN with optional line,
GOTO, END, STOP, CLR, REM and DATA skipping. Physical STOP breaks a loop;
Stop/cancel/resume and Reset remain owned board lifecycle operations. Guest
errors recover through the approved fixed gate. Unsupported PRINT expressions and
recognised but unimplemented tokenised statements report **?UNIMPLEMENTED ERROR**,
a development diagnostic rather than
stock BASIC error equivalence. Malformed stored text reports **?BAD PROGRAM
ERROR**. No approximate integer evaluator stands in for the future numeric
engine. CHRGET/CHRGOT patches, BASIC vectors, CONT, abbreviations, screen re-edit,
full line-editor modes, numeric/string-expression/variable/array/function/control
flow and the remaining KERNAL corpus are not qualified by this slice.

Expanded physical typing exposed stale SHIFT sampling; modifiers are now read
when a key is accepted. A 32-phase physical-key corpus exercises the correction.
Scrolling now copies screen and colour blocks in increasing address order so
earlier destinations cannot overwrite unread source; original pattern vectors
cross all page boundaries and preserve sprite pointers. Wrapped physical-line
delete has a separate vector. These named fixes do not imply complete editor
or physical-keyboard compatibility.

The banner says **GUEST BASIC FRONT END - PARTIAL**. Historical build, tests, native/
WASM observations, exact source hashes and outstanding work are recorded in
`docs/evidence/c64-original-front-end-20261020.json` and `docs/testing.md`.
Historical storage/boot receipts and frames retain their original identities.
This work advances C64ROM-01/C64BAS-01/C64KERNAL-01 but does not complete stage 3
or authorise stage-4 product composition, defaults, commits or distribution.

### Stage-3 guest numbers and scalars — historical 0.3 checkpoint, 21 October 2026

Development identity `eightbit-c64-original-numeric-0.3` extends the approved
partial guest interpreter. Original `numbers.s` executes add/subtract/multiply/
divide, decimal conversion, nine-significant-digit display and signed-integer
conversion as ordinary 6510 instructions. `expressions.inc` implements precedence
for +, -, * and /, unary signs, parentheses, implicit/explicit LET, two-significant-
character names and float/integer scalar records. Undefined numeric variables
initialise to zero; failed assignment preserves an existing value. Numeric PRINT
uses the existing guest output path after its private numeric gate returns.
This is a named development subset, not complete BASIC V2 compatibility.

The third 8 KiB BASIC page and fixed `$F880` private call gate remain within
ADR 0062's already approved bounds. There is no new public BASIC entry promise,
project schema, product registration, default or distribution change. Numeric
values remain in published guest FAC/ARG or packed seven-byte scalar records;
expression operands use the hardware stack. Decimal parsing/display use two
bounded big integers in 80 transient stack bytes, guarded before allocation.
Display produces a bounded temporary string at `$0100–$011F` and returns to
page zero before public CHROUT, including user hooks. No permanent user-RAM
reservation or host expression/value substitution is introduced.

Floating-point operations retain 32 significand bits with eight guard bits and
sticky tails; exact halves round up in magnitude. Range checks follow rounding,
including the smallest representable value. Integer assignment floors negative
fractions and rejects values outside `[-32768,32768)`. Exact rational decimal
conversion rounds once; literals are bounded to 80 mantissa digits. Recursion
checks the real hardware stack and recovers with an explicit development error.
Scalar access validates text/variable/array/string/memory ordering, seven-byte
record spans and capacity before allocation. Nonempty arrays are explicitly
unsupported in this increment. These checks do not qualify strings or GC.

Only manufacturer interface facts were used, including five-byte storage and
nine-digit display ([PRG BASIC programming rules, pp. 5–6](https://www.commodore.ca/manuals/c64_programmers_reference/c64-programmers_reference_guide-01-basic_programming_rules.pdf)).
Rounding ties and integer conversion were additionally observed through exact-
hash supplied-ROM keyboard commands and public scalar records; no implementation
or numeric tables were inspected. `generate-c64-numeric-fixtures.py` supplies
original exact Fraction/integer test expectations, never a runtime evaluator.
The shared keyboard corpus runs unchanged against both firmware identities.

Functions, powers/logical/relational operators, arrays, allocated strings/GC,
FOR/NEXT, IF, READ/INPUT, CONT, patchable public BASIC vectors/CHRGET, complete
error presentation, editor/KERNAL/PETSCII compatibility and stage-4 Studio
composition remain open. Decimal conversion has a deliberately bounded corpus;
these observations do not establish universal stock numeric equivalence.
Current commands/results and source-bound evidence are in `docs/testing.md` and
`docs/evidence/c64-original-numeric-20261021.json`. Earlier receipts are unchanged.

### Stage-3 comparisons, conditions and subroutines — 21 October 2026

`eightbit-c64-original-control-0.4` extends the approved partial interpreter with
numeric <, =, >, <=, >= and <> (including measured reversed two-sign pairs),
integer NOT/AND/OR, IF/THEN with a statement or line target, conditional GOTO and
GOSUB/RETURN. Comparisons produce numeric -1/0 without subtracting potentially
overflowing operands. Logical operands are converted through the existing signed
integer path and evaluated eagerly. Precedence follows arithmetic, comparison,
NOT, AND, OR; chained comparisons are left associative. Manufacturer facts are
in the PRG [relations](https://www.devili.iki.fi/Computers/Commodore/C64/Programmers_Reference/Chapter_1/page_012.html),
[logical operations](https://www.devili.iki.fi/Computers/Commodore/C64/Programmers_Reference/Chapter_1/page_013.html)
and [precedence table](https://www.devili.iki.fi/Computers/Commodore/C64/Programmers_Reference/Chapter_1/page_016.html).
Named combinations, conversion boundaries and control flow are additionally
observed using physical keys and the exact-hash supplied ROM.

False IF conditions skip the remainder of the line, including colon tails.
Subroutines retain caller text/record/line positions in seven-byte records on the
real guest CPU stack, without private host BASIC state. This development subset
admits at most six nested records; expressions share the hardware-stack guard.
RETURN validates frame depth/tag, caller programme record and line identity,
text bounds and the saved delimiter before dispatch. Direct-mode returns are
bounded to the existing input buffer. CLR/RUN invalidate subroutine frames,
while guest errors and Reset recover through the existing gates. This is not a
claim of stock stack-frame ABI or complete stock nesting capacity.

The exact-rational generator adds 2,507 comparison/logical assembly vectors to
the existing 5,527 numerical vectors. The unchanged shared keyboard corpus
checks 27 expression cases, true/false/nested IF, skipped colon tails, direct/
nested GOSUB returns, eager errors and CLR/RUN invalidation. Original-only
fixtures damage four real return frames and verify that programme text is
preserved. Stop/cancel/resume and Reset are checked while decimal workspace and
a caller frame coexist. Output assertions require the current result/READY tail,
so an earlier matching row cannot satisfy a later case.

Provenance: the expression/control structure was independently planned before
manual lookup and uses original guest code and fixtures. An initial broad manual search returned
an unrequested short third-party implementation/disassembly snippet. That page
was not opened and the snippet was excluded as an implementation source; only
manufacturer descriptions and lawful behavioural observations informed the
implementation. No implementation, tables, firmware or glyphs were copied.

The same three-page image and existing private gate suffice; the container,
physical guest RAM, project/PRG contracts, tool boundary and product defaults are
unchanged. Remaining work includes powers/functions, strings/arrays/GC, FOR/NEXT,
ON, READ/INPUT, SYS/USR, CONT, public BASIC vectors/CHRGET, complete error/editor/
KERNAL/PETSCII compatibility and stage-4 composition. Current verification is in
`docs/testing.md` and `docs/evidence/c64-original-control-20261021.json`; previous
receipts retain their original identities and narrower scope.

## Firmware and Studio completion continuation — 21 October 2026

Firmware identity `eightbit-c64-original-development-0.5` is built from the original
assembly/character sources and `scripts/build-c64-original.py`. The checked-in
`runtime.c64rom` and `runtime-manifest.json` are the verified-original bundling
exception under ADR 0062. Ordinary tests bind their image, source and builder
hashes; rebuilding uses explicitly supplied, separately installed ca65/ld65.
Three 8 KiB BASIC pages, fixed 8 KiB KERNAL, 4 KiB characters and nine immutable
gates fit the approved container. No guest RAM is privately reserved.

Public BASIC vectors `$0300–$030B`, RAM CHRGET/CHRGOT, the 32-byte KERNAL vector
range `$0314–$0333`, SYS/USR/FAC, variable/array/string records, logical file tables,
clock and editor workspace are tested through independently authored programmes.
String collection keeps scalar, array and temporary roots. Guest errors restore
the public page; Stop/cancellation/Reset and corrupted-frame recovery are bounded.
Disconnected services return errors rather than host-file or invented transfers.

The original implementation retains conservative real-stack bounds (six GOSUB
records at the standalone bound; expressions and FOR records share the stack),
80-digit decimal parsing and original transcendental approximations. RND uses an
original xorshift sequence. TI$ accepts six decimal digits and normalises time;
undocumented nondigit conversions are rejected. Character symbol roles follow
the manufacturer chart but all lettering/geometry is independently drawn. None
of this promises internal Commodore ROM addresses, identical random sequences,
stock nesting capacity or pixel-identical fonts.

Native and browser sessions validate the PRG/source identity before explicit
execution, load the standard `$0801` RAM image, set public programme boundaries
and type RUN through the physical matrix. Guest firmware executes the code.
Reset cold-boots without reloading; Restart builds fresh source. Terminal Stop
cannot be reversed by a stale Reset. Frame/audio/input queues and requests are
bounded, and release revokes queued and held input. Native presentation uses the
existing session/epoch service; browser execution stays in one owned worker.

Provenance additions: Commodore's 1982 User Guide, Appendix E printed pp. 132–134,
was visually checked for character-code/symbol roles; no glyph pixels were copied.
Keyboard modifier observations read only the public queue. An incidental broad
reference search also returned a C128 mapping/disassembly snippet; it was not
opened or used. This is recorded alongside the earlier incidental snippet
exception, rather than claiming zero exposure. No reference ROM, programme,
disassembly or emulator implementation is shipped.

The existing VICE/chips, exact supplied-ROM validation, source schemas and PRG
export remain separate retained routes. Native C64 assembly uses ca65/ld65;
browser C64 supports the existing BASIC authoring scope. Qualification receipts
must separate firmware/native/WASM, ordinary gates, Studio/package observations,
physical acceptance and public release. The historical sections above describe
their original narrower checkpoints.

Local completion evidence is recorded in
`docs/evidence/c64-original-firmware-studio-20261021.json`. That receipt owns exact
commands, source/tool/image identities, native/WASM and Studio observations,
ordinary/package/source-archive results and unperformed platform/release gates.
