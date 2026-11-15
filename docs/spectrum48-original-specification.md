# Original Spectrum 48K development toolchain

ADR 0058 authorises this implementation. Status: **software paths implemented and
locally verified; no public release or physical-hardware acceptance**. A native Spectrum 48K
project with no selected external emulator uses the original development runtime;
saved MAME choices retain their existing route. Browser Play uses the original
runtime for Spectrum 48K, with an explicit MAME/supplied-ROM choice. Spectrum 48K builds use the original assembler when no
external assembler is selected; explicit SjASMPlus and other models retain their
existing paths. No original Spectrum runtime is yet released.

| ID | Required outcome | Acceptance |
| --- | --- | --- |
| S48CPU-01 | Original shared Z80 execution, documented base/CB/ED/DD/FD instructions, flags, refresh, interrupts and HALT | Independent instruction/flag vectors, prefix bounds, interrupt and cycle tests |
| S48HW-01 | Stock 48K RAM map; display bitmap/attributes, border, keyboard, beeper, frame interrupt and cassette pins | Address/pixel/port/timing tests and actual native/WASM checks |
| S48ROM-01 | Original firmware without a 16 KiB size constraint, preserving public interfaces and all RAM | Source-bound manifest, page/interrupt/call isolation and rebuild tests |
| S48BAS-01 | Compatible 48K BASIC statements/functions, variables/arrays/strings, numeric representation, control flow and editor | Original programmes, boundary/error/recovery corpus and lawful reference observations |
| S48BAS-02 | Text/colour/pixel graphics, keyboard input, beeper and cassette LOAD/SAVE/VERIFY/MERGE/CODE/data behaviour | Guest execution, checked tape/audio round trips and observable I/O |
| S48ASM-01 | Original portable Z80 source assembly with symbols, diagnostics and instruction-bound source maps | Independent encoding vectors, unchanged acceptance sources, malformed/bounded input and deterministic builds |
| S48DEV-01 | Desktop and browser write/build/Run/Debug without tool or ROM setup | Actual workbench journeys, native/WASM parity, cancellation/stale/failed-build recovery |
| S48MEDIA-01 | Standard cassette export usable with stock Spectrum firmware | Independent payload checks, stock-ROM reference-emulator loading; physical acceptance recorded separately |
| S48COMP-01 | Preserve existing UI, project contracts, explicit external alternatives and other targets | Schema/lifecycle/UI regressions, full ordinary gate and affected packaged products |

Processor facts: [Zilog Z80 CPU User Manual UM008011-0816](https://www.zilog.com/docs/z80/um0080.pdf).
Language and user-facing machine contracts: Sinclair Research's original
[ZX Spectrum BASIC Programming manual](https://worldofspectrum.org/ZXBasicManual/)
(archived primary publication). These references supply facts and contracts;
their prose, example implementations and recovered ROM code are not shipped.

New acceptance vectors and fixtures must be original or independently generated.
Third-party firmware stays outside ordinary tests, distribution and logs.
Record exact implemented/verified subsets here as work proceeds; this table is
the approved outcome, not a declaration that its rows are already implemented.

## Development implementation, 19 October 2026

The new `eightbit-z80-core` crate owns an independently authored CPU and pure
single-source assembler. `eightbit-spectrum-emulator` owns the digital board,
streamed TAP pulses and the original guest firmware build. Neither crate launches
an external tool or evaluates BASIC in the host. Their native tests and WASM
compilation are development evidence; the runtime remains labelled as development.

The guest firmware currently has tested arithmetic, comparison/logical operations,
elementary functions, numeric/string/array records, temporary string allocation,
concatenation, combined/separate array slicing and overlapping substring assignment,
VAL/VAL$, user-graphic addresses, DEF FN/FN argument bindings and decimal output.
The guest editor supports insertion, deletion, cursor movement and programme-line
recall with CAPS SHIFT/1. CAPS SHIFT/6 and /7 move the published programme cursor,
including gaps left by deleted lines. Recall retains original tokens and omits
hidden numeric cells outside strings/comments before the line is edited and
tokenised again. It tokenises full words and stores, replaces and deletes sorted native programme records. Edits
include native hidden numeric and function-parameter cells before an old line is
removed. Existing hidden decimal/BIN payloads remain authoritative. Statement tests combine
LET, PRINT/AT, POKE/OUT, IF, RUN/GO TO, nested GO SUB/RETURN and positive, negative,
nested and zero-iteration FOR/NEXT. Additional tests exercise LIST/LLIST routing,
NEW's preserved system values, reports with line/statement positions, CONTINUE
following STOP, retry after an expression error and removed-line detection.
INPUT tests use real keyboard matrix events and frame IRQs for numeric expression
editing, quoted/raw strings, array targets, correction and interruption. Screen,
lower input and printer streams retain separate public cursors/buffers; channel
reassignment, invalid streams, TAB, commas and apostrophes have regressions.
Video tests exercise PLOT, straight/curved DRAW, CIRCLE, POINT, ATTR, SCREEN$ and permanent/temporary colour state, including transparency,
contrast, INVERSE and OVER. The public cassette loader consumes real EAR pulses
for header/data loading and verification, including flag/checksum failures.
DATA/READ/RESTORE tests cover mixed scalar/array/slice values, expression evaluation, line-specific
RESTORE and repeated RUN. CLEAR tests rebase the live call stack below reserved
code, including overlapping moves and invalid RAMTOP recovery. BEEP and the public
beeper call have pin-level timing tests, including contended stacks. Native
cassette saving is independently decoded for all 256 byte values. High-level
LOAD/SAVE/VERIFY, CODE, SCREEN$, numeric/character arrays and MERGE are connected; cassette tests run an autostart
BASIC loader through CLEAR, LOAD CODE and USR into code produced by our assembler.
Cassette tests also decode guest SAVE waveform headers and edited BASIC/array
payloads independently, verify mismatches, reject malformed array shapes without
replacing old values, and check MERGE's line/name replacement and retained records.
The sync decoder has a regression for stack contention after CLEAR. Error-return
tests also exercise recovery into a fresh command. PAUSE has real frame IRQ and
fresh-key tests; INPUT covers print items, tokenised functions, STOP and raw LINE
interruption. INKEY$ accepts one ordinary key with optional shifts; stream input
and invalid/output-only streams have tests. PLOT applies the documented absolute
coordinate rule.

The original RST $28 interpreter executes published calculator bytecode in Z80:
numeric/string operations, checked memory cells, compressed literals, branches,
counted loops and Chebyshev series. Original independent fixtures were compared
with the exact-hash UK 48K reference ROM supplied locally by the user. Arithmetic,
memory, loops, modulo/division, sine and independently chosen polynomial vectors
agree. The unusable historical E-to-FP bytecode remains explicitly rejected;
internal ROM addresses and exact register/rounding/raster side effects are not
generally qualified. Interface facts come from Toni Baker's original ZX Computing
August/September/November 1986 articles; their implementations, examples and ROM
polynomial tables are not reused.

The portable original assembler builds 1–40,960 bytes at $6000, preserving all
48K RAM and using CLEAR $5FFF in the cassette loader. It emits the existing schema-1
source map; instructions, data and padding remain distinct. Native builds retain
atomic publication, cancellation and revision checks. Browser Rust project APIs
accept single-source Spectrum assembly with the unchanged desktop manifest and
preserve user guidance through ZIP interchange. Both BASIC and assembly source
launches rebuild and verify source/cassette identities before starting the shared
development session. The ROM loads the cassette through real EAR pulses; text
input becomes timed keyboard chords. Tests cover breakpoint/Step/Continue/Pause,
stale session/stop rejection, input bounds, held-key release and reset after a CPU
failure. The WASM bridge exposes the same session used by normal Spectrum 48K
Play/Debug, retaining its development identity during qualification.
MAME 0.289 with the exact stock ROM independently loaded and executed the generated
small and full 40 KiB cassette files. It also passed two original-assembler debugger
lifecycles. Those observations do not qualify the replacement emulator or physical
hardware.

These are **finite compatibility results, not a guarantee for arbitrary historical
software or undocumented ROM internals**. Curved DRAW uses original guest trigonometry with bounded circular
chord subdivisions; it is not yet qualified against the historical rasteriser.
Native cassette load/merge follows guest memory semantics, rather than a host
transaction: a failed late allocation or tape read can leave an incomplete load.
The browser worker/workbench and native separate-window adapters now execute
BASIC and assembly source through this same session core. Browser journeys have
observed graphics, arithmetic, string output, INPUT and assembly public ROM calls
without supplied firmware. Native Run/Restart and BASIC/assembly debugger adapters
retain the shared operation, cancellation and project guards. BASIC breakpoints
and statement Step are bound to the loaded programme, original statement entry
and published PPC; machine Step remains instruction based. Native BASIC Run/input/graphics and Stop have also been observed in the local
`.app`. PCM WAV input and guest MIC recording now share the portable cassette
path: bounded PCM input loads through EAR, and standard-speed recordings must
pass pulse, byte, checksum and header/data-length validation before TAP export.
The native save dialogue rechecks the recording and refuses overwrite. Browser debugger controls now share the source-bound BASIC and assembly core;
the local Run, source Debug and cassette journeys are recorded below. Finite numeric tests do not establish exact agreement with
every historic ROM result. Electrical CPU corner cases and ULA timing also need
additional qualification. The development identity describes those compatibility
limits; it does not require user tools or firmware for the implemented 48K paths.

Focused development commands:

```sh
cargo test -p eightbit-z80-core -p eightbit-spectrum-emulator
cargo clippy -p eightbit-z80-core -p eightbit-spectrum-emulator --all-targets -- -D warnings
cargo check -p eightbit-z80-core -p eightbit-spectrum-emulator -p eightbit-sinclair-basic -p eightbit-spectrum-core --no-default-features --target wasm32-unknown-unknown
node scripts/verify-architecture.mjs
```

The full ordinary gate and application/runtime/package acceptance remain separate
requirements. Reference tests require an explicit local path and exact SHA-256
`d55daa439b673b0e3f5897f99ac37ecb45f974d1862b4dadb85dec34af99cb42`;
no reference ROM is included in these sources or tests. The gated tests and their
commands are recorded in `docs/testing.md`.

Additional interface facts were checked in Andrew Hewson's original
[Sinclair User 29 sound article](https://sinclairuser.com/029/helplne.htm) and
Claus Jahn's [TAP format specification](https://worldofspectrum.net/zx-modules/fileformats/tapformat.html).
Waveform code is independently authored and timed from Z80 instruction facts;
these references do not substitute for reference-ROM or physical acceptance.

Guest recovery implementation now retains nested GO SUB records above the
stopped editor's ERR_SP and restores them for CONTINUE/RETURN. RUN/CLEAR discard
those records, and RETURN re-finds programme positions by native line/statement
rather than trusting an edited programme's stale pointer. A regression loads an
original cassette, stops inside two subroutines, resumes and returns through
both; another corrects a division error before resuming. These interface facts
come from Sinclair's original [subroutine chapter](https://worldofspectrum.org/ZXBasicManual/zxmanchap5.html)
and [statement reference](https://worldofspectrum.org/ZXBasicManual/zxmanappc.html),
without reusing its examples or any ROM implementation. Direct-command reports
now update the CONTINUE position, and resume resolves the current command line.
One-past-end resumption finishes normally; a missing later statement reports
Statement lost. Regressions cover both direct and edited programme lines.

The lower input window now grows while retaining earlier characters, and upper
scrolling respects that window. A fresh answer to the guest scroll prompt can
continue printing or report BREAK - CONT repeats. Tests exercise 90-character
physical INPUT, listing/printing recovery and the full 4095-character editor
limit. These behaviours follow the original manual's editor and display
contracts; exact historical screen layout and wider recovery still need
qualification.

Further editor development adds guest keyword, extended, graphics and capitals
mode handling. The published MODE and capitals flag are retained, colour control
pairs are inserted together, and the IDE literal queue continues to enter whole
words through timed matrix chords. The private typing/readiness handshake is restricted
to the original editor page, with reset and outside-caller tests. Key legends were
checked against Sinclair Research's original Spectrum+ User Guide, pp. 18–21,
and the 48K manual's keyboard/colour chapters; no ROM disassembly or keyboard
implementation was reused. Programme entry and cursor navigation now request a
bounded automatic listing around the current native line. Wider editor behaviour
is being qualified across its actual host boundaries.

The syntax pass checks tokenised statement grammar and static expression types
in guest code without evaluating values, creating variables or performing I/O.
It validates a temporary copy before accepting an edit; rejection retains the
spelling and displays a flashing question-mark caret. K/L/C/E/G carets save and
restore the underlying bitmap/attribute cell. Tests correct a rejected line,
retain an existing programme record on a bad replacement, and prevent earlier
POKE/LET statements in an invalid command from executing. Native keyboard tokens
remain single bytes, including inside INPUT strings, with separate display
widths for token movement and deletion. Literal typing retains its full-word
behaviour. A guest acknowledgement paces that queue while rendering/checking is
busy; a 900-character case retains every byte and executes its following command.
Acceptance and reports collapse the expanded input area. The long-line viewport
tracks display columns separately from byte offsets and keeps off-screen bytes
available for movement, insertion and deletion.

## Local software acceptance — 19 October 2026

The [acceptance receipt](evidence/spectrum48-development-acceptance-20261019.json)
binds the firmware/source identity and observed checks. The complete ordinary
gate passed, including 1,416 Rust tests, 732 UI tests, architecture/product
validators, Clippy and the release workspace build. Fifty-seven explicit optional
checks remained ignored; the separately run Spectrum reference checks are listed
in `docs/testing.md`. All three unsigned macOS product packages built. Native
Spectrum BASIC and assembly Run/Debug, browser normal Play for both languages,
source-bound Step/Continue/Pause/Stop, graphics and INPUT were observed. Guest
SAVE produced checked TAP files through the native save dialogue and browser
download; both files' headers, payloads and checksums were inspected independently.
Cancellation retained the native recording.

`node scripts/browser/spectrum-runtime-parity.mjs` compared 438 requests across
the real native/WASM hosts, including graphics/audio state, input, debugging and
physical MIC capture. The extracted corresponding-source workspace rebuilt its
Rust/WASM runtime and UI, passed its 119 UI tests and reproduced the same trace.
No third-party ROM or executable is needed by these original runtime paths.

Persistent heap allocation reserves 512 bytes beneath the live stack for recovery;
the reclaimed syntax copy uses the smaller live-stack margin. Near-full allocation
failure retains existing variables and accepts CLEAR. This can reject an allocation
earlier than historical firmware; all 48 KiB remains accessible as raw RAM. The
editor is bounded to 4095 bytes and available workspace. Its syntax-error caret
marks rejection at the edit's end rather than reproducing the historic exact byte
position. A long INPUT caption is not reconstructed after it scrolls out of the
viewport; the complete entered value remains available. These presentation and
workspace differences, exact historic curve pixels/numeric rounding, electrical
corner cases and undocumented internal ROM calls are not universal compatibility
claims. Physical cassette loading, heard sound/controls and gameplay still need
observations on an original Spectrum; no source test substitutes for that gate.
