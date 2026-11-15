# Original Dragon 32 hardware and BASIC contract

ADR 0057 extends the monitor specification. The user requires legacy ROM calls
and RAM layouts as well as BASIC syntax and files. The reference target is the
existing Dragon 32 PAL / 1982 BASIC profile, with 32 KB RAM and the unchanged
`dragon32-basic@1` token contract. This document defines required work; a listed
interface is not a claim that it has been implemented or qualified.

## Current checkpoint — 19 October 2026

Continue ADR 0057's complete original Dragon 32 replacement: guest-executed BASIC,
legacy ROM calls and RAM layouts, file compatibility and the shared browser/native
runtime. On 19 October the user approved a 32 KiB Studio ROM, with cartridge
programmes using XRoar. Preserve the existing 32 KiB RAM, published ROM calls,
workbench, separate native window and explicit execution. ADR 0057 records the
expanded memory map and retains the existing 16 KiB legacy profiles.

The subsequent default-runtime extension also provides native mixed and
machine-only launch, browser assembly authoring/build/Run/Debug and immutable
machine-media execution. The processor-owned assembler and family runtime share
bounded validation. Full-RAM cassette loading uses original guest code through
published firmware entries, never host injection. Current-build machine source
maps and a single execution owner provide coherent breakpoints, Step, Continue,
Pause and memory/register inspection. See ADR 0057 and [testing](testing.md) for
the separate integration receipts; ROM qualification alone does not prove these
IDE routes. Desktop cartridges retain XRoar; the website rejects cartridges.

The qualified expanded ROM is **32,768 bytes**, containing 19,794 bytes of
code/data and **12,718 usable free bytes**, including three padding bytes
between fixed entries. Its SHA-256 is
`59bd021ae55f5991c16ff631248655ab666ee2a3976cb5ab93753668c43bbf92`.
The finite inventory contains 59 concrete statement slots and 34 function slots;
all resolve to implemented handlers. The maintained 415-test guest suite passes
in 190.99 seconds: it uses the expanded production image and includes one
legacy-profile cartridge regression and two retained 16 KiB Board cassette checks. All 157 fixtures and exact
reassembly pass their provenance checks. Five Board tests cover the actual map,
cartridge rejection, returning FIRQ, physical keyboard and cassette save/load;
eight normal Run/Debug tests pass. These finite checks do not establish universal
historical compatibility.

| State | Location | Qualification |
| --- | --- | --- |
| Expanded Studio ROM | `families/dragon/firmware/basic-extended/` | Exactly 32,768 bytes; qualified exact-image admission for normal Run/Debug; 415 guest tests plus Board and runtime checks. |
| Current development ROM | `families/dragon/firmware/basic/` | Exactly 16,384 bytes; 181 guest tests; some commands and later compatibility corrections remain absent. |
| Canonical source and default measurement | `families/dragon/firmware/basic-complete/` | 19,813-byte default candidate retained for measurement and legacy cartridge regression; expanded production build uses the same source with an explicit define. |

The candidate includes authoritative string descriptors and public callback
semantics, signed-word floor conversion, observed PRINT/device/input/editor
ordering, complete graphics/music commands and cassette data/programme files.
SIN/COS/TAN, LOG/EXP and ATN use independently generated mathematical polynomials;
reference observations are used only for verification. Finite exact evidence
includes 104 trigonometric, 140 EXP, 75 LOG, 345 power and 39 ATN cases. Full-width
negative-base powers retain the observed parity cutoff at magnitude 2^31.
SQR now shares `EXP(LOG(x)*.5)`, with separate zero/domain handling. All 1,276
retained square-root observations match, including every nonzero exponent with
five mantissas. Its measured maximum is 53 private stack bytes and 60,025 E cycles.

Tokenised CLOAD writes through the public $7E cursor and admits a maximum block
with a 318-byte margin, including before EOF. Checksum/IO failure and memory
failure retain their separately observed programme/cursor state. Loading from a
programme reclaims obsolete control records only at the recognised native
statement boundary; nested public callers keep their frames. Per-byte IRQ/FIRQ
masking preserves incoming masks and waveform timing, while bounded NMI nesting
retains the payload/stack separation. These changes match 79 retained reference
facts and the existing input/output byte matrix. No permanent guest RAM is added.

Recent native sharing removes unused numeric constants, reuses graphics colour
parsing and validated scalar/array value traversal, and removes duplicate
arithmetic/formatter/programme-link state. The current image reproduces exactly
and its preceding checkpoint passed strict Clippy. All 157 fixtures are compiled into the maintained test
target; historical staging folders are not required. See
[the combined receipt](evidence/dragon32-complete-candidate-20261018.json) for
identities, commands and the separation of candidate and production evidence.

The complete ordinary gate passed for the preceding 19,811-byte image: 1,212 Rust tests passed
with 53 opt-in tests ignored, and 651 UI tests passed with one existing skip.
Formatting, type checks, production UI bundles, staged sidecar integration,
Clippy and the release workspace build also pass. Two test threads avoid the
previous external-emulator probe contention while retaining its five-second
timeout. That earlier failed gate and its unchanged focused retry remain in the
receipt. SOUND now has 513 pitch/duration cases and all 256 low-byte IRQ quota
replacements: the proposed pair-loop rewrite failed the interrupt case and was
rejected. Its source remains unchanged.

The public-cursor INPUT correction is now adopted with fourteen exact-profile
observations. It honours $35 after a subscript callback, preserves a resolved
destination during refill and accepts the observed colon-separated reply fields.
The finite vocabulary audit finds no missing command family; callback semantics
and other compatibility details still require evidence and may need corrections.

The expanded image is adopted through the distinct
`eightbit-dragon32-basic-extended-1` profile. It uses $C000–$FEFF for original code
and rejects cartridge insertion; XRoar retains that compatibility route.
Normal project Run/Debug and development Build and mount remain distinct flows.
Browser/native parity and packaged acceptance are recorded separately in
[testing](testing.md). The shared UI, legacy interfaces and explicit XRoar
alternative remain unchanged. Older compaction checkpoints below describe the
retained development profile, not the expanded profile's delivery target.

### Retained 16 KiB development checkpoint — historical

The implementation/fit gaps in this block describe the earlier development
image. They are superseded for the expanded profile by the current checkpoint
above; they are not outstanding work for that profile.

The current development image is
`187d66f60d9f5c227a8a8eb74e3f11ca5d24a887430975496edca735c05963a9`,
with **21 free bytes** and a largest gap of 18. Native DRAW is present in the
actual 16 KiB image, sharing bounded guest-stack macro strings and variable
arguments. It passes 85 original synthetic reference cases, including all five
modes, nesting, scale, angle, clipping, embedded NULs and argument failures.
Lower-mode lines now rasterise physical endpoints with observed direction-specific
tie handling; 110 reference planes cover every mode and both directions.
Recursive/maximal DRAW strings, stack collision and failure recovery are tested.

CIRCLE now follows 86 complete synthetic reference canvases: a 64-segment
polygon, physical-radius truncation before aspect scaling, clamped endpoints,
1/64-turn arc quantisation and clockwise wrapped arcs. This supersedes the
unqualified 2,048-angle/128-chord approximation described in the history below.
Colour zero is accepted; the arc bound is checked after quantisation, so 1.001
is accepted as 1. The shared full-width integer multiplier also supplies checked
array sizing. The measured ellipse uses 428,900 guest E cycles, down from about
1.09 million, while retaining the existing 1.5-million-cycle regression ceiling.
The complete vocabulary and remaining qualification/integration gates are still
unfinished; this is not completion of the ROM-space work.

The continuing native pass shares copying, literal allocation, stack-space checks,
bitwise operands and local call/failure islands. It retains allocating direct
string literals: deferring allocation changed observed heap addresses and the
legacy overlapping MID$ zero-count behaviour, so that experiment was discarded.
Guest keyword adjacency/case now matches the selected lexical observations;
the IDE compiler's existing source contract remains unchanged.

GET/PUT now executes in the same image. The original 399-byte native module
retains fast whole-byte transfers and packed bit transfers, all five modes,
clipping, operation masks and observed partial writes on array exhaustion.
Packed GET marks the final stored numeric-array extent as zero; lookup and the
collector validate that representation without accepting malformed payloads.
The retained fixture covers 181 synthetic reference cases. Full-plane tests also
cover all five modes in both representations, adjacent arrays, subsequent
collection and bounded execution cost.

All 181 guest tests pass against this image. Break is polled between statements
at its physical matrix position; unrelated keys leave execution running. A
stopped nested FOR/GOSUB programme retains its frames for CONT, including a
same-line continuation after STOP. END remains quiet. The poll restores the
keyboard/printer latch and records the held Break key so the next prompt does
not consume it again. The character-routing test bus models the physical printer
strobe, separating keyboard writes from output characters.

Native compaction shares typed INPUT storage/failure, clears fractional bytes in
FIX before masking the final partial byte, toggles packed-BCD nibbles, and shares
validated expression dispatch. Connected short-branch modules are packed together
between the unchanged public entry stubs. These reductions supply the new
Break/STOP code; they do not supply the space for the remaining commands.

The exact-profile opt-in `lawful_rom_runtime_break_observations` passes five
original physical-key programmes. Brief Break is ignored during SOUND tones,
PLAY notes and PLAY pauses; normal completion closes the DAC gate and permits
the next command. The original ROM has independent physical-board regressions
for these cases and nested continuation. The earlier 1,530-case decimal display
qualification remains applicable to the retained packed-BCD conversion.

The rebuilt browser core contains this exact image once. Its 32-command corpus
matches native state, pixels and 4,248,384 audio samples at 79,237,377 guest E
cycles. All nine actual WASM project-profile contracts pass. The 18 October full
ordinary lifecycle gate passes 777 Rust and 615 UI tests; 52 opt-in Rust checks
and one UI check remain ignored/skipped. The emulator contributes 86 passing
unit tests and 181 guest tests, with eight reference opt-ins excluded from the
ordinary gate. The local 6809 application was then rebuilt, and resource checks
passed for all three registered product bundles. This verifies packaging, not
actual window or physical-device behaviour; see the adopted gate and package
observations in the [lifecycle receipt](evidence/dragon32-debug-lifecycle-20261018.json).

The actual local browser also records `CSAVE "UITEST"`, downloads its checked
442-byte CAS, and loads it through the file picker into a fresh guest. A separate
RUN prints the synthetic `CASSETTE UI OK` marker. Sidebar collapse, panel scrolling
and the existing controls remain usable. This local development journey does not
establish packaged native or physical-device acceptance. Development firmware
remains separate from normal project execution. The compact receipt is
`docs/evidence/dragon32-break-cassette-development-20261018.json`.

The native source-build journey also passes explicit development Build and mount,
manual CLOAD/RUN, physical guest cassette recording, cancelled-save retry and
fresh-session reload. Existing-file replacement and malformed/oversized mounts
are rejected without changing the retained recording/tape. A failed project
build cannot launch the previous artefact; correcting it produces a fresh owned
session. The separate native window returns to the established XRoar controls
on Stop. This is local source-build evidence, not signed packaged acceptance;
see `docs/evidence/dragon32-native-development-launch-20261018.json`.

The latest isolated diagnostic/native-sharing candidate occupies **19,640 bytes**
with fixed public entries (3,256 bytes over the limit) and passes **248 tests**.
It adds observed NF/RG/OD/UL/CN/DD/BS/OS/OM/UF/LS errors and precedence, and ST
for a ninth pending temporary string without charging ordinary variable reads.
Temporary ownership flags use reserved bytes in existing transient stack roots;
no permanent user RAM is reserved. ASCII cassette language errors preserve the
valid loaded prefix and successful block status: direct statements report DS,
while an invalid line number reports SN. Malformed tape bytes retain bounded
failure/reset handling. Shared programme lookup and decimal text sinks recover
32 bytes within this candidate. The consolidated candidate also includes the
61-case PLAY/DRAW public-workspace correction, physical Break/STOP continuation,
50 bytes of native expression/EDIT/RENUM sharing and corrected branch relaxation.
Its 202-case direct diagnostic corpus covers typed string, bitwise, graphics
and exponentiation failures. Fifty-five further canvas/pin/workspace cases
qualify zero colours, SCREEN byte predicates with public $C1=0/8, and CIRCLE
angles which accept a byte of 1/64 turns and wrap whole turns before plotting.
These staged corrections supersede the narrower current-image arc bound above.
The public printer settings are qualified by 39 observations, including configurable line endings, wrapping, comma zones and BUSY/strobe handling. Twenty-one interactive INPUT observations establish programme-only ID, empty numeric fields, whole-list REDO, surplus-value notices and Break/CONT restarting the reply without allocating a destination. Cassette output shares the decimal renderer directly. OPEN evaluates its device/name before mode/already-open errors.
The combined candidate also passes published jump/pointer/interrupt routes, timer
carry/wrap, reset-flag/vector and cartridge-FIRQ regressions. Public hardware
initialisation now returns through Y with no RAM/stack access, including an
uninitialised stack and non-zero DP; software initialisation preserves all four
PIA controls at $34. Private BASIC boot/warm paths enable frame/CART sources. Its descending-segment
collector removes repeated overlap scans while preserving aliases and atomic
malformed-record rejection. Real-Board cold cartridge acceptance still requires
the fitting ROM: the test-only overflow and cartridge both occupy $C000.
All of these changes still await the complete 16 KiB fit. The
following staged byte counts describe earlier representations, not a new ROM.

### Earlier fitting and qualification checkpoints

The following dated measurements retain their original scope. The two states
above and later dated receipts take precedence over historical missing-feature
lists or smaller test counts.

The current PRINT list accepts leading/repeated separators and adjacent
expressions. Variable scanning ignores internal spaces, as observed for scalar,
string and array names. The 21-case `print-list.json` fixture records original
synthetic printer output and success observations. Empty printed strings emit a
blank before carriage return; an explicit device followed by an empty comma
argument fails. Busy-printer checks retain the column until output can proceed.

The shared character router also supports a bounded temporary RAM text sink.
Its pointer advances and its length counts remaining capacity; no extra persistent
RAM byte is reserved. STR$ uses a 32-byte capacity. Tests cover zero, one, 32,
251 and 255 bytes, register preservation, adjacent canaries and output recovery.
LIST now suppresses only the tokeniser's structural colon before ELSE and
apostrophe comments, while retaining user colons, quoted text and DATA context.
Two original reference programmes cover those detokenised listings.

PRINT USING is still an isolated development candidate, not part of this ROM.
Its ordinary and field-formatted numbers share one original formatter, and it is
checked against 973 synthetic field cases. The shared formatter candidate is
1,358 bytes including its temporary test bridge; the three focused candidate
checks pass after its field flags moved into a register. RENUM is a separate
530-byte native candidate, including undefined-target warnings. EDIT is a
468-byte candidate with retained original editing observations and insertion,
abort and recovery checks. These candidates are not included in this image;
all 181 formatter, 183 RENUM and 183 EDIT staged tests pass on the current base.
RENUM preflights both the live stack and BASIC's configured memory ceiling before
writing; a 500-line test retains every link/reference at 206,964,332 guest E cycles.
EDIT preserves the original programme on storage/expanded-text failure, ignores
extra insertion at the 251-character limit and matches physical Break observations.
Their complete integrated runtime behaviour still needs qualification.
The candidate's larger test address space is not a 16 KiB completion result.

The isolated combined native harness now runs all candidates together: 202 tests
pass, including EDIT/RENUM followed by formatted output and a physical-signal
cassette record round trip into a fresh guest. A formatter failure preserves a
pending output record and subsequent recovery. The assembled complete native
candidate measures 19,163 bytes before fixed-entry padding, 2,779 bytes over the
16 KiB limit. This byte-count artefact is not runnable or distributable; fixed
public entry locations must still be preserved when the code fits.

The user subsequently authorised a guest command-interpreter experiment in
ADR 0057. The complete native source can now also be assembled on an isolated
qualification bus with every fixed public entry retained: it occupies 19,272
bytes, including three bytes of padding, and passes all 202 combined tests.
Its explicit test overflow above $BFFF is not a board ROM, bank or distributable
image. The actual 16 KiB development image above remains unchanged.

The first original command decoder converts DLOAD, POKE and TIMER, uses only
guest stack frames and the existing private word $03FE/$03FF, and passes 205
combined tests. These include every condition-code combination, nested native
calls and a caught nested statement failure. The assembled complete candidate
is 19,476 bytes: 204 bytes larger than the native candidate. It is not adopted.
An automated whole-function variant shares native instruction sequences and
converts ten modules, but occupies 21,144 bytes and exposes cassette timing and
execution-cost regressions. An internal native continuation initially allocated
an extra function frame; a separate resume bridge corrects that CLOAD failure.
These measurements reject the tested encodings as space solutions. They do not
establish that a different semantic command interpreter cannot fit, and do not
complete the ROM-space, compatibility or integration requirements.

The current native code shares source-line ingestion, unsigned byte comparison,
lexical context and typed CSV parsing. CSV input has its own cursor and does not
borrow the READ/DATA cursor. Native failure frames restore temporary output mode.
The INPUT retry path now restores the saved type at the current frame offset;
a rejected quoted string followed by a valid string is a deterministic regression.
Physical-matrix opt-in observations establish shifted navigation, Shift-Backspace,
alpha-lock behaviour and ignored Break in both EDIT modes. The original-ROM keyboard tests preserve those results.
Cassette ASCII output now keeps its buffered count in public $79, separate from
the temporary RAM formatter's capacity. Full data-file workspace remains staged.

An isolated cassette data-file candidate reuses the existing physical header,
block and ASCII-output routines, plus the interactive INPUT frame and variable
loop. Its reference fixtures cover OPEN/CLOSE, PRINT#/INPUT#/LINE INPUT#, EOF,
public buffer count/cursor, error cases and records across 255-byte blocks.
Observed input records retain at most 249 printable characters and discard the
rest through CR; quotes, leading spaces and trailing separators differ between
INPUT# and LINE INPUT#. The combined candidate occupies 1,858 bytes including its replacements for existing
file/input/error routines; all 185 staged tests pass. END flushes/closes the data
stream; STOP, natural programme completion, CLEAR and NEW retain it. CSAVE/SKIPF
reject an already-open file without disturbing buffered records; CLOAD abandons
it without flushing. OPEN accepts any file-type byte when both ASCII/gap flags
are non-zero. CLOADM ignores the ASCII flag, and a format mismatch retains the
successful block status while reporting FM. Corruption, missing input, malformed
EOF and incomplete records stop the motor and permit a subsequent command.
This candidate is not part of the deliverable ROM. The missing commands together still require substantially more space than the
20 free bytes, before broader ABI and integration qualification.

A staged 42-byte DLOAD handler follows 32 original base-Dragon-32 command/error
observations and six open-file lifecycle observations. DLOAD/DLOADM closes an
open cassette file before parsing arguments, reports IO without a transfer,
and reports FC for a supplied numeric selector. Type/syntax/arithmetic errors
retain their ordinary precedence. This is the base profile's unavailable-transfer
behaviour; it implements neither the Dragon 64 serial facility nor networking.

Required work remains further native compaction; PRINT USING and EDIT/RENUM;
data-device files; complete numeric/error/public ROM/RAM and CPU/board
qualification; then normal original-ROM project Build/Run/Debug/media integration.
PLAY executes in the actual ROM: 92 parameter/reference cases, nested strings,
note/volume commands, frame-based durations and observed RUN defaults. Seventeen
note checks cover DAC levels, dotted durations and pitches within four E cycles
of sampled reference medians. The original eight-cycle delay grid does not
establish exact reference timing. The board test covers actual frame IRQs,
generated audio and reset during a long note. Complete music/public-workspace
and keyboard-Break qualification remain open. Development launch is available,
but normal project execution has not been promoted to this firmware. Passing a
selected corpus does not complete these gates.

The detailed implementation sections below retain development history; older
free-space counts and missing-feature lists describe their earlier checkpoints.
Use this checkpoint, the actual firmware receipt and the latest dated entries in
`testing.md` for the current state.

### Live string readers and native ownership — 19 October 2026

The complete candidate now distinguishes each observed string-reader contract.
PRINT USING retains a descriptor reference and a remaining-character count, so
later expressions can relocate an array, collect the heap or deliberately alter
the descriptor and still affect subsequent format fields. DRAW and PLAY instead
latch the initial payload address and extent: payload edits are visible, while
descriptor retargeting does not redirect an active macro. Their public
semicolon continuation remains writable by an authored machine-code callback.
VAL reads its bounded source in place and uses constant stack space for strings
of lengths 1, 64 and 255. None of these readers copies the string onto the stack.

The existing private word at `$03EA–EB` links transient endpoint contexts on the
ordinary guest stack. It is not an additional collector root or permanent RAM
allocation. A context bounds reads at its endpoint without hiding independently
addressed nested function text. Failure unwinding removes only contexts in the
discarded stack interval, retaining an outer reader and permitting the next
command to recover. The PRINT USING frame retains its collision preflight before
metadata writes, including all protected bytes at each rejected stack gap.

Collection leaves static descriptors whose payload starts above MEMTOP untouched,
including ROM and wrapping 16-bit addresses. It neither reads nor relocates those
payloads. Managed payloads must still have a non-wrapping extent within MEMTOP;
all descriptors are preflighted before moving any bytes. Exact-profile observations
retain pointer, length and heap metadata only for the static-address cases.

The enclosing expression or statement owns private register and failure state.
Scalar/array lookup, slice/radix evaluation and allocation therefore share their
existing native boundary; DEF FN retains the inner boundary needed to restore
its parameter. Allocation failures now propagate OS consistently, including
HEX$/STR$ inside DEF FN, rather than being translated to FC by a function return.
Private test probes enter through an authored owner adapter; their original
register, carry, stack and memory assertions remain intact. No public entry or
callback convention changes.

FOR/NEXT termination compares the signs of the public step byte and numeric
comparison. The independent regression covers all 256 edited step bytes and both
incoming flag extremes. Line rasterisation shares one major-axis kernel with
directional tie handling; an independent nearest-pixel model covers 11,560 cases
across all five modes, odd/even origins and every octant, including public cursor
and caller-register state. The existing complete reference canvases and execution
ceilings remain required.

### Cartridge values and native re-entry — 19 October 2026

An authored extension may return a descriptor inside read-only cartridge memory.
Six exact-profile observations cover both operand orders and LEFT$ for RAM and
cartridge descriptors. The private pending-string record now stores its array
relocation flag separately from the full address, using an already unused byte.
PRINT USING retains that flag in its existing frame. LET and MID$ keep their
compact RAM-only lvalue representation. Regressions additionally cover collection,
array movement, INSTR, both numeric and string fields, errors and recovery. No
public descriptor, permanent RAM allocation or cartridge mapping changes.

Public software initialisation clears the first-party private region before
initialising the interpreter. Cold boot shares the same loop from address zero.
This prevents stale control frames, text contexts and formatting destinations
from surviving re-entry, while preserving the adjacent input and screen bytes.
This assertion concerns the original interpreter's own state; it is not an
observation about the reference ROM's private workspace.

The native INPUT paths now retain only the cursors each reply source needs.
File destinations are resolved before reading; keyboard destinations after the
reply. Break/CONT, whole-list REDO, record refill and the public INPUT pointer
retain their existing tests. DATA derives its initial traversal state from
TXTTAB-1, and control records are published only after their capacity checks.
The combined 341-test candidate occupies 19,931 bytes; these changes do not
complete the 16 KiB fit or normal runtime qualification.

### Keyboard input and final arithmetic rounding — 19 October 2026

Seventeen exact-profile observations at the public crunch hook establish the
keyboard editor's 249-character limit. Excess characters are ignored, Backspace
edits the accepted prefix and Clear permits fresh input. The guest regression
failed on the previous 251-character implementation and now covers those facts,
empty input, Break, adjacent private state and public graphics/music workspace.
This changes the shared keyboard reader used by direct input, INPUT and LINE
INPUT; EDIT retains its separately qualified limit. ASCII cassette input now shares
the 249-character record limit documented below.

Product and quotient rounding retain all 32 significand bits and the next
rounding bit. Exact halfway magnitudes round upwards. The final multiply/divide
adapters omit lower discarded bits after normalisation; addition still retains
its guard/sticky information for cancellation. The full 64-bit product used by
RND is unchanged. An independent integer oracle checks 8,192 division cases
across every exponent, signs, zero division, underflow and overflow. This saves
28 bytes without a larger guest stack or new permanent workspace.


### Public output, cassette continuation and shared native ownership — 19 October 2026

The public character router classifies all 256 device bytes after the output
hook: `$00–$7E` goes to screen, `$FF` to cassette, `$FD` discards, and the other
values go to printer. Raw screen/printer entries keep their independent contracts.
PRINT column selection retains its separately observed rules. Cassette output
fills byte 255 without flushing; the next character triggers the pending-block
write. A failed write retains that block, cursor and count for a guest retry.
PRINT#-1 shares ordinary USING/TAB grammar and follows live hook redirection.

ASCII CSAVE lists through that same public device boundary. A hook redirecting
it away from cassette retains its observed open state; retained cassette output
closes normally. Successful ASCII saves stop the current direct/programme
continuation, while binary saves continue. Failures restore their scoped output
mode and permit retry. BASIC/data headers retain unused address words; CSAVEM
writes its explicit addresses. Twelve new fixtures retain 409 exact-profile
observations, including failure/column/continuation boundaries. Guest regressions
also decode the physical cassette waveform and recover after injected failures.

PRINT USING shares string/numeric field width, and STR$ consumes the formatter's
completed character span directly. The previous stack-collision thresholds stay
unchanged. Magnitude-ordered addition shares its comparator and removes redundant
negative-difference repair; an exact integer oracle covers 65,280 calls across
all exponents, signs, cancellation and rounding. The measured aggregate cost is
3.15% higher, with unchanged stack use and existing performance ceilings.
Private wrappers used only by the harness are removed; authored test adapters
preserve their exact call/stack shape outside the candidate and guest RAM. No
published entry or runtime code is supplied by those adapters.

The combined 362-test image is 19,815 bytes, still 3,431 above the limit. Its
compiler-to-guest journey passes the shared IDE compiler's cassette through guest CLOAD,
BASIC arrays/functions, graphics, music, formatted output, STOP/CONT and data-file
record/reload. It uses the explicit qualification bus and does not establish
normal browser/desktop admission or the final 16 KiB fit.

### Shared cassette record input — 19 October 2026

ASCII CLOAD and data INPUT now use one guest character/refill and bounded record
reader. Forty additional exact-profile observations cover control-byte filtering,
249-character truncation, first-block prefetch, public input/crunch hooks, eager
refill, motor-off parsing and preservation of the keyboard length byte `$D7`.
An unfinished EOF tail is discarded. Syntax and I/O failures retain completed
programme lines and the observed open input state; a fresh CLOAD recovers.
Reading a corrupt following block prevents the pending CR from storing its line.

The source-import validator keeps its printable-text contract. Physical cassette
transport keeps record bytes opaque, allowing the guest to apply these rules.
The existing ASCII save/reload test now uses the motor-aware tape clock; exact
payload and reloaded-programme comparisons remain unchanged. The corrected
source cases were observed as accepted by the reference, replacing earlier
unqualified rejection assumptions.

Sharing the record reader saves 34 bytes. Decimal digit parsing also shares the
existing integer conversion and arithmetic operand ownership, saving eight bytes
while keeping its exact digit/ARG cases and 450-cycle ceiling (414 measured
maximum). The full ordinary gate passes on the 19,815-byte image: 1,152 Rust tests,
636 UI tests, formatting, type checks, production bundles, Clippy, staged sidecar
checks and the release workspace build. This remains an oversized qualification
image; it is not admitted to the normal original-ROM Run/Debug path.

## Acceptance

| ID | Required observation |
| --- | --- |
| D32CPU-01 | Every documented opcode/addressing mode, condition flags, instruction counts, invalid encoding rejection and address wrapping. |
| D32CPU-02 | Reset, IRQ/FIRQ/NMI priority, masks, frames, arming, RTI, software interrupts, SYNC and CWAI, including short NMI pulses. |
| D32HW-01 | PIA data/DDR/control, edge flags/enables, read clearing, forced/pulse/handshake outputs and board interrupt routing. |
| D32HW-02 | SAM controls, clock rates, installed RAM addressing, upper-map writes, display counter and mismatched VDG/SAM modes. |
| D32HW-03 | Raster text, inverse, semigraphics 4/6/8/12/24, all eight graphics modes and PAL sync; original glyphs. |
| D32HW-04 | Deterministic bounded audio/cassette input/output, motor pause, joystick axes/buttons/comparator and printer signals. |
| D32BAS-01 | Direct prompt and line editing; original guest-executed tokeniser, linked programme storage, LIST/NEW/RUN/CONT/CLEAR and errors/recovery. |
| D32BAS-02 | The complete checked-in Dragon BASIC statement/function vocabulary, numeric precedence/range/rounding, strings, arrays, control flow, DATA and user functions. |
| D32BAS-03 | Graphics, sound/PLAY/DRAW, device selection, cassette BASIC/machine-code/data files and deterministic source/token/media round trips. |
| D32BAS-04 | Legacy ROM calls, CPU/RAM vectors, RAM hooks, interpreter workspace, scalar/array/string records, FAC/USR conventions and control stacks. |
| D32BAS-05 | Studio build/run/debug integration uses the original ROM's own validated profile; the OEM-ROM profile/signature is not weakened. |
| D32RUN-01 | Shared Rust native/WASM state/video/audio parity, actual browser/native journeys, bounded lifecycle, cancellation and reset recovery. |

No host BASIC evaluator, firmware trap or source-only demonstration can satisfy
these gates. Unsupported commands and unqualified calls must remain visible.
XRoar stays an explicit alternative, with its existing firmware checks. Preserve
the workbench and ADR 0045's separate native window.

## Digital scheduling implementation

The shared native/WASM board advances video, audio and cassette state between
observable events. Video visits pixel clocks and every horizontal counter reset;
a bus interval ends before any fetch at its exclusive boundary. RAM, SAM and VDG
pins supplied by the next interval therefore govern that fetch. Audio integrates
the supplied constant signal to the next fractional 48 kHz sample boundary, using
the same integer mean and signed truncation. Cassette input consumes exact pulse
durations and changes level only when the next pulse's first clock is consumed.
Motor pause, output transitions, sixty-second output splits and bounded capture
queues keep their existing semantics. CPU bus cycles, PIA edges and guest clocks
are not skipped or rescaled.

Scalar master-clock oracles, changing-pin/RAM tests and native/WASM corpus parity
qualify this scheduling change. They establish equivalence with the prior digital
model, not new physical timing, analogue fidelity or full firmware compatibility.
The 16 KiB original ROM, public ROM/RAM interfaces and development-only launch
status are unchanged. Dated measurements and commands are in `testing.md`.

## Legacy interfaces

Dragon Data's March 1983 *Information for Machine Code Users*, pages 3–7, is the
primary interface source. Its scanned page 5 says `$800C` for screen output;
the OCR incorrectly reads `$8000`. Use the scan. Required fixed calls include hardware/software initialisation `$8000/$8003`,
keyboard `$8006`, cursor `$8009`, screen `$800C`, printer `$800F`, joystick
`$8012`, motor `$8015/$8018`, tape preparation/output `$801B/$801E` and tape
input `$8021/$8024/$8027`. Preserve the pointer table `$A000–$A00D`, documented
direct routine addresses, and the published numeric conversions `$8B2D/$8C37`.

Workspace compatibility includes programme/scalar/array/end pointers
`$19/$1B/$1D/$1F`, stack/string/RAM limits `$21/$23/$27`, execution/DATA/input
pointers `$2D/$2F/$31/$33/$35`, line `$68`, device `$6F`, restart `$71–$73`,
cassette `$78–$81`, cursor `$88/$89`, text pointer `$A6/$A7`, USR table `$B0`,
graphics `$B2–$C0`, timer `$112–$114`, token dispatch `$120–$133`, USR entries
`$134–$147`, rollover/joysticks `$150–$15D` and patchable hooks `$15E–$1A8`.
The eight five-byte public temporary-string descriptors occupy `$1A9–$1D0`;
`$0B` is their exclusive free pointer and `$0D` identifies the last descriptor.
The verified input/output hooks are `$16A`/`$167`, and line input uses `$182`.
CPU vectors must route through the corresponding three-byte RAM slots
`$100/$103/$106/$109/$10C/$10F`. Layouts must be maintained by guest code.

Scalar records have two significant name bytes and five value bytes. Strings
use their legacy descriptor. Packed numbers retain the five-byte signed MBF
representation, including its zero convention and 32-bit significant mantissa.
The guest FAC retains exponent, normalised mantissa and separate sign; USR
receives the legacy pointer and returns through the documented conversions.
Default programme storage starts at `$1E01`, as required by Studio's current
Dragon 32 build contract. Graphics reservation and CLEAR must move the actual
legacy pointers coherently, without corrupting adjacent programme or strings.
The keyboard input area starts at `$02DD`; the published `$02DD–$03D8` bounds
include the terminator. Original-interpreter private state uses `$03D9–$03FF`, leaving the public
graphics/PLAY/DRAW fields available to their guest routines. The keyboard editor accepts 249 printable characters within that area and
ignores excess characters; EDIT and cassette source loading retain their own
independently qualified limits. The line/input hooks preserve the four preamble
bytes `$02D9–$02DC` while collecting input. Before crunching numbered input or
an accepted EDIT, guest code publishes the line number in `$02DA/$02DB` and X;
`$A6/$A7` identifies the source body. Direct crunching retains X=`$02DC`.
EDIT calls the public character-input hook and retains its acceptance key in A
at the crunch hook. Callbacks own the stored line word independently of the
numeric lookup workspace `$2B/$2C`; the latter survives storage. An authored
callback which makes the reference create a duplicate line is rejected before
programme mutation under the accepted bounded-storage contract. Consistent
header/lookup changes retain their observed replacement or insertion behaviour.
Number presentation uses guest stack scratch, preserving active direct tokens
and adjacent workspace.

User-approved exact-profile black-box observations on 15 October 2026 confirm
default MEMTOP/FRETOP `$7FFE` and stack limit `$7F36`. `CLEAR n,m` uses `m-1`
as MEMTOP and reserves `n` string bytes; argument-free CLEAR retains those limits.
Arrays use `[name2,total-record-length2,dimension-count1,reversed-extents2*N,values]`,
five-byte elements and first-subscript-fastest order. Implicit arrays have
extent eleven in each dimension. Strings use `[length,0,pointer-hi,pointer-lo,0]`;
programme literals can remain in programme memory, while assigned direct strings
occupy the descending heap. FOR frames have eighteen bytes: tag `$80`, variable
pointer, unpacked step magnitude (five bytes), direction `$FF/0/1`, packed limit
(five bytes), line and text pointer. GOSUB frames have seven bytes: tag `$BD`,
line, text pointer and an implementation-specific return slot. The stack ends
in a two-byte zero sentinel. Do not substitute an OEM internal return target.
STOP retains frames at the prompt and CONT restores the execution cursor.
DATA's public cursor points at the delimiter after the last item; RESTORE resets
that pointer to TXTTAB-1 and retains the last DATA-line number.

Arbitrary OEM ROM bytes, internal instruction addresses and timing are not
assumed interchangeable merely because documented calls work. Extend the probe
inventory whenever software depends on another entry or workspace location;
do not advertise general legacy compatibility without evidence for it.

## Implementation and qualification history

The CPU now decodes all documented encodings. Original tests exercise the 268
opcode forms/counts, all indexed postbytes, exhaustive eight-bit arithmetic,
valid BCD additions and interrupt/wait behaviour. This is functional evidence;
internal-cycle placement, every flag/register combination and independent
hardware timing still need qualification.

The digital board models PIA edges/outputs, SAM rate/normal RAM and upper-map
write behaviour, raster modes, bounded 48 kHz audio, pulse cassette transport,
joystick comparison and printer signals. Normal memory uses the SAM's 64K DRAM
routing with the Dragon
32's single installed 32K bank; high address aliases and absent bank selects are
modelled explicitly. Original-ROM boot in separately installed XRoar caught and
corrected an earlier incorrect two-bank assumption. The selected PAL model uses 910 crystal
clocks per line, 312 lines, continuous HS and 25 added lines on either side of
the VDG field padding. PAL board revision padding, unusual SAM RAM modes,
pin-level bus phase, analogue audio/colour and hardware comparison remain
unqualified. This assumption must not be presented as measured Dragon hardware.

Product launchers retain the monitor and explicit XRoar alternatives. Web and
desktop additionally offer an explicitly selected BASIC development console in
the existing emulator controls. Native execution retains the owned separate
window; browser execution uses an owned Worker. The shared console validates
firmware identity and uses a bounded 0–249-character matrix input path supporting
Shift chords and case. Build/Run/Debug and high-level media integration remain
open; this launch path does not imply complete compatibility.

The development ROM now implements the named function vocabulary, DEF FN/USR,
computed ON branches, numeric/string/array expressions and existing control/DATA
statements. Compact `FNF(...)` names share guest/authoring tokenisation. ELSE is recognised
as a statement end by shared command parsing. FN records use the observed `[body2,parameter2,0]` value and a high
first-name bit; calls restore the parameter on success and failure. The collector
moves overlapping string-root intervals together. An original guest arithmetic
microprogram decoder reduces ROM size; no host evaluator is involved.

Graphics include colour CLS, 64×32 semigraphics SET/RESET/POINT, all five BASIC
PMODEs, PCLS/PSET/PRESET/PPOINT, SCREEN, COLOR, PCLEAR relocation and integer LINE
with B/BF rectangles and validated PCOPY page transfers. Public graphics fields and selected pixel colours have
lawful-reference observations. PCLEAR rejects zero pages and a reduction below
the active mode's required graphics extent. SOUND uses original DAC code with
an observed pitch law; MOTOR/AUDIO operate the PIA pins. Low-level cassette
routines provide bounded edge/bit/byte input, byte output, block framing,
checksum status and input RAM bounds. A checksum failure can leave already received
guest RAM bytes, as distinct from a host file operation; no host file is written.

CIRCLE accepts radius, optional colour, height/width ratio and start/end
fractions. Original guest code uses a Q14 integer quarter-wave table with exact
cardinals and at most six integer units of interpolation error at each of the
2,048 angle positions. Integer axes below 256 use 128 clipped Bresenham chords;
larger axes retain dense sampling. Radius, aspect and vertical-axis truncation
match selected lawful observations: radius 10.9 becomes 10, aspect .6 quantises
to 153/256, and radius 60 with that aspect has vertical radius 35. Equal or
reversed arc endpoints select a full turn. Guest tests exhaust angles, cover
ellipse cardinals in every mode, check the curve's shape, reject invalid
parameters and guard programme/public DRAW/PLAY workspace during clipping.
The measured `CIRCLE(128,96),60,1,.6` cost falls from 76,858,994 to 1,108,938
guest E cycles (about 69×); its regression ceiling is 1.5 million cycles.
Complete OEM pixels, volatile workspace and timing equivalence remain unqualified.

INPUT and LINE INPUT use bounded guest-stack reply storage, preserving the
active direct tokens and DATA cursor. Tests cover quoted CSV, case, retries,
continuation replies, empty string replies, Break and stack recovery. PRINT@ accepts positions 0–511;
TAB pads from the current column and can cross rows, matching selected lawful
reference observations. Screen output retains semigraphics bytes. RUN can start at an existing numbered line after clearing variables. LLIST
restores the selected output device on success and failure. Explicit `PRINT#0`
and `PRINT#-2` share that restoration boundary. Printer TAB and comma zones use
$9C; carriage return clears it. The digital BUSY model uses high while busy and low when ready. Approved
black-box reads establish the reference ready level as zero; independent
original-ROM XRoar printing verifies the guest handshake. Waiting remains guest execution in bounded host slices; release
resumes the byte, and warm reset selects screen output before the prompt.
Hardware/software
initialisation entries return to callers. Exact input prompts,
extra-value notices, line limits and all device variants remain unqualified.

LIST and LLIST accept inclusive single/open/closed line ranges. DEL uses the
same validated range, compacts/relinks survivors and clears variable/control
state; it rejects a missing range and programme execution. Reversed/malformed
ranges fail before programme bytes or variables change. Selected rejection
behaviour has lawful-reference observations. The shared guest recurrence and
repacked original code preserve the 16 KiB image and fixed entry stubs; the
receipt reports only 16 unused bytes, so further compaction remains necessary.

The native expression engine now uses an eight-byte pending operand record
with one packed five-byte value, a root link and a type byte. Only live string
operands register with the collector. This avoids separate left-value and root
copies; pending frames retain the same 32-bit numeric significand and string
alias behaviour. Shared typed assignment serves LET, READ and INPUT, and floating
swap preserves U. A selected 31-operand expression saves 176 stack bytes and
succeeds with resident data ending at $6D00 where the previous image rejects the
same expression. The operand rewrite recovers 59 bytes; moving the line editor’s private error
flag from public $D8 to existing private $03FA uses five bytes (54 net). Tests
guard $D8–$E9 through successful, overflowing, cancelled and subsequent input.
This is insufficient for the
remaining command families. Fixed ROM entries and public RAM allocation remain
unchanged.

MID$ assignment accepts scalar/array string variables, one-based start and an
optional byte length. It resolves array destinations again after scalar creation
in either index or RHS expressions. Programme literals are detached before RHS
evaluation; existing heap values copy forwards in place, preserving observed
self-overlap. Empty RHS leaves contents unchanged. Tests cover allocation failure,
repeated collection, malformed arguments and stack/root recovery. Selected lawful
probes establish the historical length-zero quirk: a 256-byte forward guest copy,
which can write beyond the string; canaries at offsets 255/256/257 establish its
extent. The guest bus still rejects ROM writes. These observations do not qualify
every volatile string workspace byte or every allocation address.

Shared native grammar/failure helpers, PIA initialisation data and expanded
guest arithmetic microprograms recover 263 bytes in the existing modules,
including the new statement dispatch. The added MID$ module occupies 239 bytes.
Fixed stubs, the copied scanner, signal-timed kernels and RAM allocation remain
unchanged. This compaction is insufficient for the outstanding command families;
no extra bank, host evaluator, code overlay or permanent RAM reservation is used.

Remaining language work includes the additional PRINT/device/RUN-file forms,
interactive editing/renumbering and GET/PUT/DRAW/PLAY/file commands.
Cassette bit/block routines do not establish CLOAD/CSAVE or file-device support.
Exact errors, uncommon numerical/array/control cases, all legacy ABI calls and
real media journeys remain qualification work. Original RND code maintains the observed seed layout and 32-bit recurrence;
returned fractions still have selected one-ULP differences from the reference. No commercial-software, physical
Dragon or audible-output compatibility is inferred from these tests.

Schema-1 monitor session frames gain additive bounded signed 16-bit audio at
48 kHz, drained once per Advance. Shared presentation schedules only a short
owned queue after user interaction, and clears it on reset, stop, failure and
teardown. The workbench and controls are unchanged. Browser/native audible
acceptance and cassette/media journeys remain separate gates.

## Provenance and verification

No recovered BASIC source/disassembly, ROM instructions or third-party emulator
implementation is consulted to derive shipped code. Keep source and build
receipts for original firmware, and use original synthetic guest programmes in
ordinary tests. A lawful exact-profile comparison remains an opt-in additional
gate and records observations only. Public delivery needs separate approval.

The lawful-ROM harness uses original synthetic programmes, fixed argument vectors,
an exact fingerprint and temporary 32K RAM-only snapshots. Retained observations
exclude instructions and internal return targets. Its input uses XRoar's
documented ROM-call intercept. A separate explicitly ignored native gate executes
the lawful ROM on our CPU/board and verifies a synthetic POKE through emulated
matrix presses spanning several PAL fields. The monitor's faster typing queue
does not establish OEM debounce compatibility. Our original ROM also passes a
separate installed-XRoar cold-boot probe for arrays, strings, thirty FOR iterations,
heap compaction and GOSUB. These gates establish their specific observations,
not complete hardware or BASIC compatibility.

Technical references:

- [Motorola M6809PM](https://www.maddes.net/m6809pm/index.htm), documented instruction semantics and counts.
- [Motorola MC6821 datasheet](https://colorcomputerarchive.com/repo/Documents/Datasheets/MC6821%20Peripheral%20Interface%20Adapter%20%28Motorola%29.pdf), pages 7–10, PIA interfaces and strobes.
- [Motorola MC6883 datasheet](https://colorcomputerarchive.com/repo/Documents/Datasheets/MC6883%20Synchronous%20Address%20Multiplexer%20%28Motorola%29.pdf), display/address routing and control tables.
- [Motorola MC6847 datasheet](https://colorcomputerarchive.com/repo/Documents/Datasheets/MC6847%20MOS%20Video%20Display%20Generator%20%28Motorola%29.pdf), geometry, modes and sync timing.
- [Dragon Data BASIC manual](https://colorcomputerarchive.com/repo/Documents/Books/Dragon/An%20Introduction%20to%20BASIC%20programming%20using%20the%20Dragon%20micro%20computer%20%28Dragon%20Data%20Ltd%29.pdf), language and device syntax; programme listings are not copied.
- [Dragon Data machine-code interfaces](https://colorcomputerarchive.com/repo/Documents/Books/Dragon/Dragon%2032%20Information%20for%20Machine%20Code%20Users%20%28Dragon%20Data%20Ltd%29.pdf), published ROM/RAM ABI.
- [Dragon 32 Programmer's Reference Guide](https://archive.worldofdragon.org/browse/downloads/Books/Dragon-32-Programmers-Reference-Guide.pdf), language semantics and additional published interfaces; examples/implementation listings are not copied.
- [Inside the Dragon](https://colorcomputerarchive.com/repo/Documents/Books/Dragon/Inside%20The%20Dragon%20%28Duncan%20Smeed%20and%20Ian%20Sommerville%29.pdf), pages 229–234 and 247–248, published scalar/array/string and workspace structures; sample implementation code is not copied.
- [Dragon board observations](https://www.6809.org.uk/dragon/hardware.shtml), wiring and specifically identified board uncertainties.

Run targeted Rust tests and Clippy first, then firmware reproducibility, browser
WASM/native parity and changed boundary tests. The ordinary gate, actual web/
native journeys and all registered product packages are required before this
increment can be described as complete. Record exact results in `docs/testing.md`
and compact evidence separately from the earlier monitor receipt.

### Native interval collection and PAINT increment (15 October 2026)

The native collector no longer writes ordered links into descriptor housekeeping
bytes. It repeatedly selects/closes the highest connected interval, validates the
whole root set before moving data, copies each union once and relocates its roots.
A saturating changed flag avoids wrap at 256 roots. Array traversal keeps its
local bounds on the stack instead of borrowing the collector's destination word.
Numeric exponent/format loops, packed transfers and control-flow return paths
share existing native operations. Existing modules shrink by 201 bytes; a new
342-byte PAINT module originally left 16 bytes free. A subsequent native pass
recovers 98 bytes before adding explicit PRINT devices and reset recovery. That
image had 48 free bytes (largest gap 28), and PAINT occupies 310 bytes.
Array lookup and collection share a stack-local validator for every extent and
the complete five-byte element product. Floating carry/borrow loops, a pointer-
bounded swap, VAL's bounded stack allocation and static branch shortening reduce
native code. Signal-timed sound/tape bodies and the copied RAM scanner retain
their instructions. No decoder, bank, overlay or permanent RAM reservation is added.

PAINT accepts `(x,y)[,paint[,border]]` and uses physical-pixel steps in all five
PMODEs. Omitted colours use the foreground; explicit colours accept 0–8, with
zero mapping to physical colour three in colour modes and zero in monochrome.
PCLS zero has the same mapping. Other graphics commands retain their existing
validation. Original stack-only two-byte seeds replace completed-span history.
Each maximal horizontal border-free row must contain a colour change before it
propagates vertically. Existing paint is traversable horizontally, while a wholly
painted row does not bridge a vertical gap. Invalid arguments fail before plotting.
Exhausting the checked stack margin returns an error and discards all temporary
records; pixels already painted remain changed. Tests guard public $D8–$E9 and
programme memory, cover nested coordinate expressions and repeated recovery,
and compare all pixels with an independent boundary-fill model.

The reported paint-coloured vertical-border-gap mismatch is fixed. All 28 retained
complete synthetic graphics planes match the lawful reference, including before-
fill geometry, five modes, colour zero, painted seeds, isolated gaps, vertical
stripes, horizontal stripes and wholly painted interior rows. The latter cases
establish the directional rule above; a general four-neighbour flood is insufficient.
The earlier failing receipt remains historical evidence and is superseded by
`dragon32-basic-paint-gap-lawful-20261015.json`. Do not infer complete OEM pixel,
workspace, timing or error equivalence from these selected comparisons. High-level cassette/file commands, GET/PUT/DRAW/PLAY,
remaining language forms and complete ABI/project integration remain mandatory.

Printer provenance: the factual wiring reference identifies PB0 with a `!BUSY`
label. The retained independent observations establish XRoar's ready level as
zero; the original guest and digital board now agree with that observed contract.
Electrical polarity, ACK timing and a physical printer remain separate acceptance
work. No OEM instructions or printer-driver implementation were consulted.


### Native parser/conversion pass (16 October 2026)

The image remains exactly 16,384 bytes, with 305 free bytes (largest gap 105),
257 more than the preceding image after the additional language work. First-letter
validation and U preservation belong to the native variable parser, with no
creation on invalid names. Lexing/listing share four literal contexts and a normal-code context;
quotes, DATA colons and comments retain their distinct token rules. Colour and
unsigned-decimal routines use native loops. Arithmetic dispatch saves its native
continuation once per operation; its opcode vocabulary is unchanged. Temporary
return-D storage adds two guest-stack bytes per active arithmetic decoder call;
there is no new persistent allocation. Native subtraction normalises a complete
mantissa/guard difference using borrow; all 32 significant bits remain represented.

Unsigned values enter the existing integer normaliser directly, preserving X/Y/U.
This fixes TIMER reads above 32767 losing X and then rejecting the rest of the
statement. `TIMER=expression` validates 0–65535 and the statement ending before
resetting $112 and writing $113/$114; errors leave the counter intact. Console
line zero is supported in storage, RUN, branches and inclusive LIST/DEL ranges.
The IDE's source/project line-range contract remains unchanged. Missing numeric
arguments and lines above 63999 still fail.

The lawful packed-GET observations use only original cleared/drawn canvases and
one bounded `DIM A(0,9)` array. `GET ...,A,G` creates scalar A and changes the last
stored extent from one to zero. Its packed bits occupy the existing numeric array
storage. This is a required graphics-array ABI case, currently unimplemented;
the ordinary complete-product validator rejects that layout and must be adapted
with explicit lookup/collection/indexing qualification when GET/PUT are added.
The fixture observer accepts only this reviewed record/canvas and retains no
OEM instructions, raw snapshot or arbitrary memory range. The manufacturer’s
[GET/PUT language manual](https://www.dragondata.co.uk/Publications/BASIC-MAN/DRAGON_32_BASIC_MANUAL_rel-v2.pdf)
supplies syntax facts; it is not an implementation source.

These changes do not complete PRINT/editor/file forms, GET/PUT/DRAW/PLAY,
high-level cassette, full ROM/RAM/numeric ABI or normal project Build/Run/Debug/
media integration. The shared workbench and explicit XRoar alternative remain.


Direct-command tokens now share the input buffer at $02DD–$03D8. Each keyword
replacement has at most its source length, so the lexer compacts forwards in
place. The 252-byte output limit includes NUL, matching 251 input characters.
The former private token buffer at $0200 overlapped the legacy cassette area;
this correction leaves $01DA–$02D8 and the $02D9–$02DC input preamble available.
Tests cover quoted DATA/comments, maximum uncompressed input, compacted input
and overflowing output, guarding the entire cassette/preamble and $03D9–$03FF.
It does not establish high-level cassette commands. The published map on page
248 of [Inside the Dragon](https://www.dragondata.co.uk/Publications/InsideTheDragon.pdf)
supplies these buffer ranges; no sample implementation is used.

## Native failure scopes and binary cassette input — 16 October 2026

The original ROM now shares a native guest-stack failure frame across statement,
expression and variable entry points. It restores the enclosing frame, string
roots, expression depth and caller registers; device selection is restored on
failure. FN retains its parameter-restoration boundary. EXEC discards the twelve
private parsing bytes before handing control to machine code, preserving the
observed FOR/GOSUB records. The private frame pointer occupies $03FB–$03FC within
the existing allocation. There is no parser VM, overlay, bank or new user-RAM
reservation. Published numeric/peripheral calls retain carry-return behaviour.

`SKIPF [name]` and binary `CLOADM [name][,offset]` use the legacy cassette buffer
$01DA–$02D8, native framed input and the physical PIA signal. Binary data streams into its
RAM destination after header search, avoiding a cassette-buffer alias copy. Empty/omitted names
select the next file. Names are at most eight bytes. A search is bounded to
65,534 blocks; checksum, malformed-block, missing-signal and RAM-boundary errors
stop the motor. CLOADM accepts unsigned offsets 0–65535, wraps load/entry addresses
at sixteen bits and checks each destination block against 32 KiB RAM before
copying it. Earlier good blocks remain after a later error; the public default
execution pointer is latched from a valid binary header. Bytes already read into
RAM remain on I/O failure, including a block that fails its checksum. Negative literal offsets are
rejected, matching the retained lawful observations. A valid execution entry at
zero remains usable; cold setup initialises the public entry pointer to our
ordinary error path. Filename progress display and complete cassette ABI/timing
qualification remain open.

The IDE analyser and both tokenisers now accept CLOADM/CSAVEM as the existing
CLOAD/CSAVE token followed by literal M, without splitting longer identifiers or
altering quoted/DATA/comment text. CSAVEM execution was added by the subsequent native grammar/cassette-output increment below.
The manufacturer [language manual](https://www.uroboros.es/dragon/manuales/DRAGON_BASIC_v1.pdf),
printed pages 37–38 and 135, supplies syntax facts only. Approved exact-ROM
black-box tests establish the selected offset/entry observations. The new strict
observer admits only reviewed marker bytes, RAM entry addresses and status;
firmware addresses, instructions and raw snapshots are not retained.

The subsequent expression rewrite saves Y/U once per expression, retains only
one precedence byte per recursive call, and creates the eight-byte packed
operand/root record only while an operator is pending. Thirty-one operands in
a right-associative power expression now pass with STREND=$6D80 and S=$7000,
with the lowest S remaining at or above $6E00. String operands remain registered
through RHS allocation and collection; numeric values do not become roots.
Success/failure preserve caller registers, roots, depth and the enclosing boundary.

The initial tokenised `CLOAD [name]` shared named/wildcard search. That pass
accepted only binary/tokenised file type zero; ASCII followed in the later
streaming pass below. Data-device files remain unsupported. Data blocks stage in
$01DA–$02D8 before a checked copy, leaving at least the existing 128-byte stack
collision allowance. Loading replaces the programme and variables after a valid
header. EOF requires a complete bounded linked-line body, strictly increasing
line numbers through 63999 and exactly one final two-byte zero marker. Incoming
non-zero link addresses are indicators only; guest code rebuilds them at TXTTAB.
An invalid/partial selected load leaves NEW state with a valid empty programme.
Successful CLOAD discards the rest of its command line, including a trailing
RUN. RUN on the next command line executes the loaded programme. These selected
semantics and rebuilt line bytes match the approved lawful-ROM observations.
Loading from a running programme stops rather than resuming overwritten text. Exact OEM error/display,
partial-load recovery, gap flags and all cassette workspace effects are still
unqualified. The command loop observes failure carry before clearing control
state, so failure recovery cannot turn an error into success.

Exact assemblies reproduced the failure/cassette-loading baseline and image. That image
had 31 free bytes, largest gap six; its SHA-256 is
`7c1450bba437d76b890274246e2db6f1f818b9bf67d9828ef7acf8947104c26b`.
Relative to the initial 305-byte baseline, pre-existing modules recover 191 bytes;
the three new cassette modules occupy 465 bytes. This is too little for the
outstanding feature set. Further substantial rewrite, BASIC editor/PRINT/file
forms, GET/PUT/DRAW/PLAY, cassette output/ASCII/data files, complete ABI/CPU/
hardware qualification and normal project Build/Run/Debug/media integration
remain required work. The ROM remains development; UI and explicit XRoar routes
are preserved. Focused, ordinary, independent-reference, parity and package
results are recorded separately in [testing](testing.md).


### Native grammar and cassette output — 16 October 2026

Required delimiters share ordinary 6809 calls. Parenthesised expressions own
both delimiters. An optional-comma helper consumes only a matched comma and
preserves comparison flags, including Z despite LEAX changing it. Native address
conversions and errors share the existing unwind boundary; DEF USR retains the
parsed address rather than converting the FAC again. No parser decoder, bank,
executable overlay or additional permanent RAM reservation is introduced.

Development `CSAVE [filename]` writes the complete checked linked programme,
including its terminal zero link. It accepts an omitted/empty filename or an
up-to-eight-byte string, padded to eight spaces. It emits a type-0, tokenised,
ungapped 15-byte header with zero addresses, up to 255 bytes per data block and
an empty EOF block. Each block has a 128-byte leader and the existing additive
checksum. ASCII `,A` and other extra arguments are rejected before motor-on.

`CSAVEM [filename],start,end,execution` uses unsigned sixteen-bit expressions.
The initial implementation restricted its inclusive range to $0000–$7FFF;
the shared-walker follow-on below extends reads to the whole guest bus. The
execution address may span $0000–$FFFF. It emits a type-2 binary header and the
same checked block framing. All three addresses are currently mandatory. It
streams live RAM; legacy cassette workspace and the active guest stack remain
volatile, so this is not an atomic RAM snapshot. Saving leaves the loaded $9D
execution pointer unchanged. Both save commands restore the command cursor,
continue a following colon statement and turn the motor off. Filename/type/range/
syntax checks and BASIC programme validation finish before signal output.

The byte writer now provides balanced high/low halves and accounts for native
framing/call overhead at the final bit of each byte. Guest tests measure both
halves, inter-byte periods, flags/registers and emitted payload/checksum bytes.
An independent media parser validates empty, single- and multi-block saves;
guest CLOAD/CLOADM reload the resulting files. These tests establish the stated
development behaviour, not all OEM errors, workspace effects or physical tape
acceptance. Both emulated-WAV save/decode reference gates pass with exact BASIC
and binary metadata/payloads. The direct XRoar CAS-capture gate initially rejected 38 trailing bytes after
six correct blocks. The shared-walker follow-on below resolves that test-reader
defect; complete cassette exchange still needs the remaining file/ABI gates.

The image is exactly 16,384 bytes, SHA-256
`01e96b48e4f399d917df5bf4e738e6dc660783aa46b0e3c41f87985f167435c6`,
with 32 free bytes, largest gap five. Exact assemblies account for 194 bytes
recovered from previous modules relative to the 31-byte-free image, a new
five-byte opening helper and 188 bytes of new writer code (189 bytes net
compaction before the writer). The strict bundling allow-list includes
both new original modules. Full BASIC/editor/PRINT forms, GET/PUT/DRAW/PLAY,
ASCII/data-device files, ABI qualification and project runtime promotion remain
open. The development-only controls, monitor default, UI and explicit XRoar
alternative are preserved. See the dated testing receipt for actual gates.


### Native shared walkers, numeric compatibility and cassette capture — 16 October 2026

The image is exactly 16,384 bytes, SHA-256
`9bfef73f870133cbade97f2e7b77e717d6216d3907cfe0f347171425deaf2a88`,
with 179 free bytes and a largest gap of 118. Independent module assemblies
reproduce the 32-byte-free baseline and this image: net recovery is 147 bytes,
including the added behaviour. This remains insufficient for the full outstanding
vocabulary. There is no new decoder, bank, overlay or permanent RAM reservation.

Strict programme validation and link repair share a native sequential walker.
Strict mode writes no programme bytes; repair mode follows bounded line bodies,
never supplied links. Both enforce ascending 0–63999 line numbers and an exact
terminal-word position, preserving the caller's cassette frame. Existing NEW
recovery on failed CLOAD is unchanged. Statement bodies use the dispatch frame's
register restoration, and control transfers share native return paths. Native
RND uses ten byte products while retaining the observed seed and product scratch;
generated seed boundaries and 512 further seeds exercise carry propagation.

The shared decimal scanner accepts insignificant internal spaces, including
`1 2`, `1 . 5`, `1 E - 2`, VAL input and console branch line numbers. This changes
only guest parsing, not the IDE/project source schema. Hexadecimal and octal
literals, including bare `&`, now use unsigned 0–65535 results. VAL recognises an
initial radix prefix after spaces, accepts empty prefixes as zero and ignores
non-numeric suffixes according to the retained lawful cases. A sign before a
radix prefix in VAL yields zero. Overflow and illegal later octal digits fail and
unwind temporary roots/stack. The observed bare-& initial digit is retained;
explicit O and later digits enforce octal range. Negative HEX$ fails, whereas
`HEX$(65535.9)` truncates to `FFFF`. Earlier assumptions about negative HEX$ were
incorrect; isolated lawful probes with a continuation marker establish rejection.
Logical operators retain their signed-word range, separately observed.

CSAVEM supersedes the initial RAM-only restriction: start/end form an ordered
inclusive range anywhere on the sixteen-bit guest bus. The remaining-byte counter
handles $FFFF and the initial 65,536-byte full range. Short real DAC captures cover
ROM/final-address reads; a separate instrumented-byte-writer test covers full-range
counting without claiming its waveform. Only our own ROM/generated RAM is used.
Cassette workspace, active stack and I/O reads remain volatile guest operations.
Output block routines mask IRQ/FIRQ throughout all bytes and restore the caller's
masks before returning status. Masking individual bytes was insufficient: a
pending frame IRQ stretched a final one into a zero at 9.6 kHz. The checksum can
remain valid when two high bits change, so exact independent payload comparison
and an injected pending-IRQ regression remain required.

The direct XRoar capture reader validates the documented CUE footer/offset,
variable-length chunk bounds, nonzero timing and complete ordered data coverage.
It rejects unknown types, unexplained/non-leader gaps, overlaps and truncated/invalid suffixes. This fixes
the prior capture gate without changing the production media format contract.
Both CAS and emulated-WAV gates check exact headers/payloads/EOF with tape traps
disabled. Format facts come from the authors' published CUE specification linked
in ADR 0057, not GPL source. The 30-command native/WASM corpus adds unsigned radix,
spaced decimals and a one-byte ROM-address CSAVEM journey. Full BASIC/editor/PRINT,
GET/PUT/DRAW/PLAY, ASCII/data files, complete ABI/CPU/hardware qualification and
normal project integration remain required. Monitor default, UI and explicit
XRoar selection are unchanged.


## Native ASCII cassette and filename RUN follow-on

The preceding ASCII-cassette development image is
`15804300a10fa07bc5a929ecd81c265fdb05f7478719d95a287e9040ba02f771`.
It is exactly 16,384 bytes, with 62 verified unused bytes (largest gap 52).
Relative to the preceding 179-byte-free image, pre-existing modules are a net
90 bytes smaller after their added hooks. New ASCII input/output occupy 106/101
bytes. These exact counts include all code/data and the unchanged fixed stubs;
they are further compaction, not sufficient space recovery for the full scope.

`CSAVE [name-expression],A` writes the original BASIC/ASCII/gapped header
(0/$FF/$FF), leading CR, each ordinary LIST line and its CR, and EOF. It uses the
existing cassette buffer with up to 255 text bytes per block. Native unsigned
buffer indexing must cover offsets 128–254 correctly. Approximately 500 ms low
gaps precede data/EOF leaders. No second programme buffer, reserved user RAM,
host file operation or change to the loaded execution entry is introduced.

`CLOAD [name-expression]` recognises exact ASCII flag $FF in BASIC headers as
well as tokenised files. It accepts CR and CR/LF, spans lines across blocks,
inserts/replaces lines through the existing native tokeniser/store, and discards
an unterminated final line. Whole source lines are bounded to 251 bytes plus
NUL in the legacy input area. Control/non-ASCII bytes, invalid line numbers,
overlong lines, bad checksums and storage failures leave valid NEW state and
motor-off. Loading discards the overwritten command-line tail. A nested native
frame preserves the file cursor/budget and restores roots/depth on source errors.
Exact OEM error wording and partial-load workspace are still unqualified.

`RUN` accepts a filename string expression alongside its existing no-argument/
line-number forms. It loads before clearing execution state and starting the
selected programme. A rejected or failing load cannot execute stale programme
bytes. Ordinary project source schemas and line-number rules are unchanged.

Native compaction shares numeric sign/absolute helpers and bounded grammar/
condition checks, and forwards nearby calls through ordinary JMP islands. Those
islands preserve arithmetic continuation addresses and introduce no interpreter
VM. Original cassette signal bodies and public entry stubs are retained.

Independent XRoar probes use original source and real cassette transport with
fast traps disabled. Its ASCII capture's silence chunks omit raw leader padding;
the qualification reader accepts such holes only when preceded by silence and
containing exclusively $55. At that checkpoint production media still accepted only its previously
implemented tokenised/binary variants. The follow-on below adds ASCII recovery
and WAV conversion; explicit desktop project ASCII export remains outside its
existing tokenised/binary choices. PRINT/editor forms,
GET/PUT/DRAW/PLAY, data files, full ABI/CPU/hardware and normal original project
Build/Run/Debug remain mandatory. UI, monitor default and explicit XRoar remain.

Warm reset also clears the temporary private ASCII output-routing flag before
printing DEV>. Reset while a full block is being saved must stop the motor,
restore screen output, and accept a subsequent PRINT. The new board regression
failed against the prior image (the flag remained $FF) and passes with this fix.

## Shared ASCII media and native arithmetic failure scan

The subsequent shared-media pass uses the exact original development image
`6e7eb00806a6f9b523f5d1e6593aeb8c0f41f326631e4a24372770531e6d3374`: 16,384 bytes, **74 free bytes**, largest gap 52.
A separate native arithmetic failure scan removes a per-operation stack flag
and redundant jumps, recovering 12 bytes without repacking include groups. Internal addresses move
within their existing group; public entry stubs remain fixed.
This is a small verified reduction; substantial interpreter compaction remains
necessary for the missing language, graphics and music routines.

The shared Rust media layer now reads ASCII/gapped BASIC CAS and PCM WAV with
explicit `dragon-cassette-ascii-basic` / `dragon-wav-cassette-ascii-basic` format
identifiers. Existing tokenised format identifiers retain their meaning. ASCII
recovery resolves complete CR-terminated replacement/deletion lines, ignores LF
and reports discarded unfinished EOF text. Project recovery retains the existing
positive line-number and dialect/source validation contract; it does not expand
that contract to guest-console line zero or every historical input spelling.
Transport text is bounded to 251 printable bytes per line; source inspection is
bounded to 1 MiB. Checksums, block ordering, EOF and trailing bytes remain strict.
Arbitrary CUE extensions/custom loaders remain unsupported production inputs.

Browser programme recovery and desktop preview/import/rebuild share these Rust
rules. Provenance hashes the unmodified original and distinguishes ASCII from
tokenised input. Desktop external ASCII CAS uses the existing explicit XRoar
Run boundary, with identity rechecked before launch/reset. CLI `media convert`
accepts the two explicit ASCII output identifiers, using filename PROGRAM;
existing CAS/WAV conversion transports checked ASCII files too. WAV output adds
500 ms gaps before following leaders and retains bounded deterministic PCM.
ASCII WAV encoding requires a leader/padding byte between complete blocks.
Desktop project export retains its existing tokenised/binary output choices.

The workbench, legacy RAM allocation, default monitor and explicit XRoar choice
are preserved. PRINT USING, EDIT/RENUM, GET/PUT/DRAW/PLAY, data-device files, full
ABI/CPU/hardware qualification and normal original-ROM project Build/Run/Debug
remain mandatory unfinished work. This is still a development backend.


## Native value/collector rewrite and numeric qualification

The current development image is
`4142de254daa1f95d2a33323e499f7a83e8c2c2e8a7dfa14c00ea2cb6df083f0`:
16,384 bytes, **197 free bytes**, largest gap 65. Relative to the preceding
74-byte checkpoint, exact source-module assemblies recover a net 123 bytes.
Include groups and published ROM stubs are preserved; internal addresses within
those groups move. No additional RAM, host execution or parser VM is introduced.

Native expression operands use the same typed five-byte load/store boundary as
variables. Their existing eight-byte frame, collector link, type and precedence
are unchanged. Active collection visits FAC/ARG's pointer words directly and
uses one stack mode byte rather than temporary copied descriptors; ordinary
collection excludes dead register values. Array, stack-root and active-register
validation still completes before movement. Overlapping roots retain one copied
union and every live pointer is updated. Failure preserves the heap, pointers,
root chain and stack ownership.

The CIRCLE table now stores original upward-rounded Q12 knots. Its adjacent
slopes are at most 201, permitting native byte MUL instead of repeated additions.
The returned values remain signed Q14: all 4,097 tested angle inputs across two
turns stay within five units, including exact cardinals. Total kernel/conversion
cost falls from 2,714,902 to 2,236,958 guest cycles; the maximum falls from 1,006 to
977. The representative 60-by-35 ellipse takes 1,092,543 cycles. These bounds and
selected reference pixels do not establish full historical pixel equivalence.

New exact-profile black-box observations distinguish RND's truncation order:
with seed $FFFFFFFF, RND(.5), RND(1.5) and RND(1.9) return one, while RND(2.5)
returns two. The guest truncates the positive bound before scaling its random
fraction, then truncates the product and adds one. Zero/negative handling and the
seed recurrence are retained. Public PLAY defaults are O2/L4/T2 and volume-15 DAC
bytes $BA/$42 at $DF/$E0. Separate probes observe V0=$7E/$7E and V31=$FA/$02;
those volume commands are qualification facts, not an implemented PLAY claim.
DRAW's cold scale byte is zero in the observed reference; its documented drawing
default must not be confused with initial RAM contents.

PRINT USING, EDIT/RENUM, GET/PUT/DRAW/PLAY, data-device cassette files, complete
numeric/error/public ABI and CPU/hardware qualification, and normal original-ROM
project Build/Run/Debug remain unfinished. Existing web/native development
selection, UI, monitor default and explicit XRoar alternative are preserved.


Before the following correction, the wider numeric comparison was a failing
qualification gate. Independent XRoar execution matched 12 of 13 new RND/default cases;
RND(1E20) differs by one least-significant mantissa bit even though its 1E20
argument and public seed match. A further ten-seed RND(0) comparison matches six
results and finds four one-bit normalisation differences. The positive-bound
truncation fix is verified, but exact RND return rounding is unresolved. Retain
these counterexamples and do not infer complete numeric ABI compatibility from
the ordinary regression gate or native/WASM equality.


## RND return precision and shared rounding correction

The current image is `aa29d0ea03a350045fcc401be8f20f7793cdc306a747f33927ae8254fe089e98`:
16,384 bytes, **228 free bytes**, largest gap 68. Native multiplication and RND
share a complete original 64-bit product routine; the net reduction from the
197-byte checkpoint is 31 bytes (math seven, RND 36, less 12 for the error-side-effect path). This is incremental
compaction and does not provide the space required for the missing command families.

For unsigned input seed s, let p=s*$40E64DAB. The stored next seed remains
(p+$05B0658B) modulo 2^32 and $AB–$AE remain p modulo 2^32. The returned fraction
normalises that next seed followed by guard byte (p>>40)&255, then rounds to
32 significant bits. Bounded black-box facts include very small returns and
exact halfway values; the reference rounds halfway magnitudes upwards. Additional
positive/negative addition and multiplication observations establish that the
same rule applies to ordinary arithmetic: nearest with exact halfway magnitudes
away from zero. This supersedes the development nearest/even policy. It also
corrects the previously differing 1E38 literal and large positive RND bounds.
The guard/sticky representation, numeric packing and published interfaces remain.

The native product writes an eight-byte caller-owned guest-stack destination,
uses sixteen MUL instructions and preserves both four-byte input operands. Its
row construction writes each new high byte before that byte can be read, so only
the low four bytes require initial clearing. No public music/drawing scratch,
extra bank, host evaluator, parser VM or permanent memory reservation is added.
Regression tests cover exact observed values, both signs, full 64-bit products,
dirty destinations/canaries, seed/product boundaries and native error recovery.
The shared web/native corpus now includes a packed-result RND assertion.

D32BAS-01–04 and D32RUN-01 remain incomplete: substantial further native compaction,
PRINT USING, EDIT/RENUM, GET/PUT/DRAW/PLAY, data-device files, complete numeric/error/
ROM/RAM/CPU/hardware qualification and normal original-ROM Build/Run/Debug/media
integration are still required. The workbench, default monitor and explicit
XRoar route are preserved. Selected numerical agreement does not establish full
historical programme compatibility or packaged/physical acceptance.


A string argument to RND advances the recurrence once before the type error.
The generic numeric-function guard now performs that discarded zero-argument
advance specifically for RND, then unwinds through the ordinary typed-error
boundary. It never seeds from string descriptor bytes. Nine selected empty,
single-character and HELLO cases, with positive/negative prior values and an
explicit prior RND seed, establish the side effect and abort of the command tail.
Numeric RND arguments retain the ordinary path; other function guards are unchanged.


### Ordered Stop and native addition pass (16 October 2026)

Preceding image: `83ec4d1020ec588bd6c47751ec067df314737c529f98dc9139cfbb3a0a71d111`,
16,384 bytes, **238 free bytes**, largest gap 78. Sharing the native add/subtract
byte loop recovers ten bytes. Addition uses a zero operand mask/carry; subtraction
uses an FF mask and carry-in one, including the guard. Negative differences use
the existing magnitude/sign normalisation and halfway-away rounding. No new
persistent allocation, parser decoder, overlay or host execution is introduced.
The independent exact-integer oracle covers 8,192 operations and guards both
private state and the public music/drawing workspace; 64 selected lawful packed
addition/subtraction comparisons also match. These are bounded qualification
facts, not a full numeric or historical-programme compatibility claim.

The development console now awaits the queued Stop acknowledgement before owner
teardown. It disables controls/advancement and clears sound immediately, deduplicates
Stop, permits close after a failed transport, reports close failure with retry,
and rejects late callbacks from previous lifetimes. Native window destruction
returns focus to the workbench and releases the session. The existing layout,
child permissions, monitor default and explicit XRoar alternative are preserved.
This corrects the source teardown race; actual packaged Stop acceptance is recorded
separately in testing evidence.

D32BAS-01–04 / D32RUN-01 remain incomplete. Substantial native compaction,
PRINT USING, EDIT/RENUM, GET/PUT/DRAW/PLAY, data-device files, full numeric/error/
public ROM/RAM/CPU/hardware qualification and normal original-ROM project
Build/Run/Debug/media integration remain required.


### Decimal/token compaction and selected diagnostics (16 October 2026)

Current image: `5f75263ad4270012091c4aaa370a3b4ab9d3ea4a5b6ce4040ce37a69ab599495`,
16,384 bytes, **134 free bytes**, largest gap 46. The decimal/exponent rewrite
uses a five-byte local frame and shares bounded native word scanning while
restoring the caller's line word/Y/U on success and failure. The tokeniser
traverses the contiguous public word lists with one ordinal and keeps the
matched source cursor directly. These recover 23 and 10 bytes respectively.

Selected native expression/argument boundaries now report `?TM ERROR`,
`?FC ERROR`, `?OV ERROR` and `?/0 ERROR`; other development failures use
`?SN ERROR`. Programme failures append ` IN <line>`, and the next prompt clears
the line. One private byte at $03FD owns this state. Address conversion now
returns carry on rejection even without a parser frame; PEEK/HEX$/JOYSTK and
argument boundaries translate rejection into a guest error before using a value.
Existing public carry-return arithmetic remains independent of that state.
The diagnostic module/data and boundary changes use 137 bytes after the 33-byte
reduction, so the net image uses 104 more bytes than the preceding image.

Nine selected lawful error observations agree, including a line-1000 failure.
Original tests cover mixed types, domain/range/overflow/zero-division, balanced
roots/stack, rejected address conversion, corrupt diagnostic index fallback and
subsequent prompt recovery. These do not establish all error classifications,
transcendental extremes, wording/cursor details or full compatibility. Selected
DRAW cursor/scale/rotation observations are retained as qualification facts;
DRAW remains unimplemented. Outstanding scope is unchanged: substantial further
compaction, PRINT USING, EDIT/RENUM, GET/PUT/DRAW/PLAY, data-device files, complete
numeric/error/public ABI/hardware qualification and normal original-ROM project
Build/Run/Debug/media integration. Preserve the shared UI and explicit XRoar.


## Explicit development cassette integration — 18 October 2026

D32BAS-03/05 and D32RUN-01 gain a bounded shared session path for one checked
BASIC tokenised, ASCII/gapped BASIC or machine-code CAS. Existing family media
readers validate the original bytes before replacing the attachment. The 1 MiB
cap includes full 65,536-byte CSAVEM captures plus framing. Playback retains
immutable bytes and validated gap offsets, producing one physical signal segment
at a time; it does not allocate an expanded pulse array, inject programme RAM,
intercept ROM calls or execute a host BASIC interpreter. ASCII files retain their
500 ms inter-block gaps. SHA-256 receipts identify exact mounted bytes.

The version-1 protocol adds `mount-cassette`, `rewind-cassette` and
`eject-cassette`, with an optional/null cassette receipt in each frame. Existing
commands and firmware identifiers remain supported. The JSON envelope is bounded
to four times the 1 MiB byte limit plus 4 KiB; native deserialisation also limits
the cassette array before allocation can grow past the cap. Invalid attachments
leave the old tape and guest state intact. Rewind does not type a command;
Reset retains tape position; Stop ejects and releases all attachment state.

Browser/native development consoles expose the same typed controls. An explicit
browser action can mount the fresh current Dragon 32 project build; normal Play
still selects XRoar and its existing OEM-ROM validation. Native users may select
a freshly exported project CAS in the separate development window. Guest input
remains explicit: `CLOAD` followed by `RUN`, or `CLOADM` followed by `EXEC`.
Neither mounting nor opening a project starts programme execution.

Native input has one admitted in-flight command and a five-second UI timeout.
Old browser failure callbacks, keyboard completions and file reads cannot affect
a replacement session. Reset invalidates pending file reads, and terminal
stopped frames cease polling. Existing window ownership, keyboard navigation,
Stop acknowledgement and source/build invalidation are retained.

This is development integration only. The actual firmware's remaining vocabulary,
complete legacy ABI/hardware qualification, normal original-ROM Run/Debug,
multi-file and data-device tape transport, cassette recording/export and WAV
attachment remain separate requirements. Source tests do not establish rebuilt
WASM, packaged desktop/browser, audible output or physical-device acceptance.

The same session now offers explicit start/finish/discard/export for guest-pin
cassette recording. Bounded Io pulse batches are drained after each execution
slice into 9.6 kHz PCM, capped at 15 minutes of cassette signal. The existing
media WAV decoder recovers and validates a single CAS before any Save action.
Recording never reads programme RAM or bypasses the guest cassette routines.
Loss, unsupported/incomplete output and duration exhaustion fail closed; reset
cancels active capture, and Stop releases capture and recovered bytes. A ready
recording remains available after Save or a cancelled dialogue until discarded
or the session ends. Ordinary guest CSAVE, ASCII CSAVE and CSAVEM recordings each
reload and execute in a fresh original guest through physical cassette input.

Browser Save downloads only the recovered bounded CAS. Native Save accepts only
the owned session identity, selects its recovered bytes in Rust, opens an owned
Save dialogue, rechecks the session and recording, then creates a new `.cas`
destination without overwriting. No caller-selected bytes/path, generic file
permission or microphone authority is added. The native dialogue may wait for
the user while ordinary bounded execution and Stop remain independently owned;
a stale/closed session cannot authorise a later write. Packaged dialogue and
actual download acceptance still require observation.


## Multi-file and data-record transport — 18 October 2026

The development cassette boundary now checks a complete tape before replacement:
up to 1 MiB, 256 files and 8,192 blocks. The additive family-media transport API
preserves opaque data records, empty files/blocks and the observed OPEN rule that
both ASCII/gap flags are non-zero regardless of the file-type byte. Existing
single-programme BASIC/machine-code readers and ordinary WAV import/export remain
strict. Transport does not parse records into host variables or change wire bytes.
Printable receipt labels replace non-printable filename characters only for display.

The original version-1 receipt fields are retained. Existing format labels remain
valid; `ascii-data` identifies a data file and `cassette-stream` identifies a tape
with multiple files. Physical playback retains checked offsets and inserts the
existing half-second gaps between gapped blocks and files. Explicit capture uses
an additive bounded WAV transport decoder and rejects damaged or incomplete files
anywhere in the stream, rather than exporting a valid prefix. The 15-minute
recording limit, cancellation, reset/stop, stale-session checks and create-new Save
ownership remain unchanged. The shared controls and normal XRoar route are unchanged.

The same mount control accepts `.wav` through the additive version-1 `mount-wav`
command. CAS and WAV input each remain limited to 1 MiB; this session limit is
smaller than the ordinary media importer's WAV limit. Rust validates PCM framing,
channel agreement and the whole recovered tape before replacing the attachment.
The receipt hashes the recovered CAS bytes delivered through the PIA. Mounting
does not run a command or inject guest memory. Invalid audio preserves the old
tape; reset/session replacement cancels late file reads. WAV recording export
and longer audio attachments are not supplied by this session action.

Ordinary tests execute two guest CSAVE calls into one capture, mount the checked
result in a fresh guest, search for the second filename with CLOAD and RUN its
original programme. Synthetic pin-level tests cover data records independently of
the unfinished ROM. The current fixed 16 KiB image still lacks the staged complete
data-file implementation; real guest OPEN/PRINT#/INPUT# round-trip acceptance
therefore remains outstanding. This supersedes the earlier single-file transport
limitation, without claiming complete BASIC, hardware or normal Run qualification.


## PIA control-table qualification — 18 October 2026

D32HW-01 gains original tests derived from Motorola MC6821 pages 8–10, Figure 18:
both C1/C2 polarities on both ports, edge latching independent of interrupt enable,
late enable of a pending interrupt, DDR/data acknowledgement ownership and all
256 control-register write values per port. Forced C2 outputs clear/suppress only
C2 interrupt flags while retaining C1, and A-read/B-write strobes exclude DDR
accesses. These checks pass the existing logical implementation; no runtime
behaviour changed. Electrical timing, E-clock edge placement and the unqualified
handshake transition cases remain separate. The dated receipt is
[evidence/dragon32-pia-control-qualification-20261018.json](evidence/dragon32-pia-control-qualification-20261018.json).

The 18 October descriptor checkpoint additionally qualifies the published
`$0120`/`$012A` token descriptors against 22 original synthetic observations.
Word lists and primary dispatch tables can be rebound; each lexical chain stops
at its own first zero count; later extension tokens still enter STUB1. LIST uses
the current word lists, primary statement dispatch retains the `$BA` cutoff, and
returning functions can update `$A6`/`$A7`. Raw input remains at `$02DD`; tokenised
direct commands begin at `$02DC`. The isolated candidate separates the EDIT raw
text sink from the RENUM token sink and preserves the cassette buffer. It also
stores published error selectors directly and retains the qualified keyboard
punctuation mapping. These corrections are included in the 19,674-byte/227-test
checkpoint above; they have not been promoted to the 16 KiB board image.


## Published INPUT, EOF and motor-delay qualification — 18 October 2026

The staged complete candidate now uses `$02DD` for actual programme INPUT reply
bytes and publishes the CSV cursor at `$35`. LINE INPUT preserves that pointer.
Removing the former reply copy reduces its transient frame from 265 to nine
bytes without changing the public allocation or collector rules. Empty fields,
REDO, reply-string collection, nested control frames and Break/CONT remain tested.
The public EOF flag at `$70` changes on actual reads, while OPEN and EOF queries
preserve its previous value. Keyboard input clears it for a new reply.

MOTORON honours the public `$95–96` delay, including zero wrapping through 65,536
iterations; cold setup installs `$DA5C`. The authored entry-call observation is
39 + 8 × count E cycles, while MOTOROFF takes 32; TAPE_PREP adds its independently
qualified leader waveform. These counts describe the selected guest call/profile,
not physical relay acceptance. Existing cassette high-half-wave assertions remain
strict across inter-block motor pauses; separate byte tests still assert periods.

All 232 combined candidate tests pass. The exact source is retained in
`.local/dragon-rom12-20261017/diagnostic-completion/qualified-19703-source`;
reference observations and script/test receipts are in
`.local/dragon-abi-omissions-20261018/receipt.json`. Its 19,703-byte image remains
3,319 bytes over the fixed ROM limit and is not a board firmware or distribution
input. The actual 16 KiB development ROM and its integration remain unchanged.


The subsequent published cursor/joystick check adds two combined regressions.
Blink reloads fifty when the countdown expires and changes only the cursor cell.
Both joystick entries return the qualified raw-threshold values, scan all four
axes, clamp overrange input and restore mux selection zero. The 108 authored
comparisons and unchanged register-preservation checks pass; this does not claim
analogue voltage accuracy or define the separate host coordinate convention.
The integrated candidate is now 19,709 bytes with 234 passing tests. The board
coordinate mapping is being qualified separately for coordinated adoption.

## Finite published-boundary qualification — 18 October 2026

Six further isolated original regressions cover all 31 retained fixed JMP targets,
all seven pointer words and indirect call forms, six CPU-to-RAM interrupt routes,
24-bit timer carry/wrap, reset selection and the default cartridge FIRQ. Twelve
exact-profile reset cases confirm that only `$55` plus a NOP at the user vector
selects that continuation; the `$12` shortcut was removed. The cartridge handler
filters CB1, clears `$71`, disables its interrupt, permits an independently timed
settling interval and enters authored code at `$C000`. It leaves other FIRQ
sources to their owners. The isolated candidate passes all 240 tests; its
19,738-byte image remains oversized and is not a board firmware input.

The coordinated Board changes remain staged: host joystick coordinates use
64 digital bins, and an explicitly mounted autorun cartridge supplies Q edges
to CART. Mounting and state inspection remain inert; only explicit guest
advancement executes code. Fourteen paired exact-profile observations and the
bounded Board suite pass. Cold-mounted cartridge/reset succeeds on the reference;
the oversized original's temporary `$C000` overflow prevents that acceptance
journey until it fits. No extra runtime bank or overlay is proposed.

One further concrete interface correction was identified from [*Inside the
Dragon*, section 9.1, p. 224](https://www.dragondata.co.uk/Publications/InsideTheDragon.pdf):
`$8000/$BB40` receives its return address in Y and must work without a RAM stack.
Eight authored observations confirm Y return, preserved S, no RAM accesses and
all four PIA controls at `$34`. Subsequent `$8003` calls retain those controls;
BASIC's frame/CART interrupt enables belong to private boot/warm setup. The
parent corrected this contract in a separate 19,737-byte image; independent source
review and all eight actual-Board comparisons pass against the unchanged exact
reference. This additional regression is separate from the all-240 checkpoint.
Fitting-image acceptance remains an explicit gate. The compact [published-boundary receipt](evidence/dragon32-published-boundaries-20261018.json)
and retained local finite matrix distinguish source, staged Board, exact-profile
and remaining acceptance evidence.


## Shared stopped-guest inspection — 18 October 2026

The portable BASIC core now owns the bounded scalar-workspace decoder used by
native XRoar debugging. Its reader receives explicit stopped-target RAM/table
bounds; numeric MBF values, string descriptors, source aliases and the existing
`BasicVariable` JSON shape are shared without an OEM transport dependency. It
accepts at most 128 seven-byte records, checks exact response lengths and string
bounds, propagates reader cancellation/errors and never returns partial results.
The existing exact OEM firmware hashes, hook signatures and profile selection
remain unchanged; malformed guest state retains native diagnostic `DSR2007`.

The original Machine exposes real CPU registers, side-effect-free mapped memory,
one CPU step and cancellable bounded address-breakpoint slices. Memory reads are
limited to 1–4096 bytes without wrapping and reject the `$FF00–$FFDF` device/SAM
aperture. At most 64 address breakpoints are accepted atomically; they never patch
guest bytes. An initial matching PC stops before any CPU step. Continuing from
that address requires an explicit step. A step can be an instruction, interrupt
entry or idle wait cycle; it is not a BASIC/source step. Each slice uses the
existing 32,768-cycle limit and preserves the machine's failure/reset behaviour.
`DME1300` and `DME1301` are library-only invalid-breakpoint and memory-read errors.

An authored guest programme creates `COUNT=42` and `MESSAGE$="HELLO"`, enters its
authored EXEC sentinel and is inspected without changing its registers or clocks.
This is reusable debugger groundwork under D32BAS-05, not a normal runtime profile
or a new host command. Normal Run/Debug, runtime settings, project schemas,
XRoar alternatives, browser controls and the separate native window are unchanged.
A fitting qualified image, its own runtime identity, normal launch composition,
source/BASIC breakpoint correlation and browser/native acceptance remain required.


## Original BASIC statement source correlation — 18 October 2026

The portable BASIC core can bind the existing tokeniser's linked records and
statement ranges into an immutable `StatementMap`. Construction validates bounded
record links, line order, addresses and source/token ranges. At an independently
established statement stop, it reads the published programme base/current line/
text pointer, matches an exact statement entry and verifies every loaded programme
byte in reads of at most 1024 bytes before returning the existing BASIC line,
statement ordinal and source range. Direct mode and operand cursors never claim a
source location. Relocation, changed/partial loads, malformed replies and reader
cancellation cannot reuse an old location. No execution bytes are patched.

An actual development guest loads an authored generated CAS through the physical
tape input, stops at the original source's fixed `$84D6` entry, distinguishes two
statements on one line and continues through an authored EXEC callback. The test
checks public cursor values, the expected scalar changes and unchanged hook bytes,
then rejects a changed trailing programme byte and recovers after restoration.
The caller still owns firmware validation, the stopped machine and the build/
source revision. This is correlation infrastructure, not a new normal runtime
profile, BASIC source stepping, host command or change to OEM signature checks.


## Portable original BASIC breakpoint controller — 18 October 2026

`eightbit-dragon-emulator::basic_debug::BasicDebugController` now composes the
existing development MonitorSession, immutable StatementMap and shared scalar
inspection. Its sole breakpoint line must exist in that map. A caller supplies
the fresh build's identity and optional source-name aliases; the controller
checks the 64-character lowercase digest shape, and every published stop verifies
all loaded token bytes. It does not independently recreate the build manifest.

Continue clears previous values and starts one revision-owned search. Polls use
1–32,768 CPU cycles, check cancellation before every CPU step and inspection read,
and cumulatively stop after 16,777,216 cycles or 10,000 statement-hook encounters.
An instruction may overshoot a cycle limit by its final instruction cost. All
stops use actual registers and public RAM; cancellation never publishes partial
inspection. Source aliases are bounded to the source limit in aggregate and
8,192 bytes each, preventing a repeated corrupt scalar name from amplifying the
retained output beyond that source limit. The shared 128-record/string bounds
continue to apply.

Continue leaves the original hook without patching it. If a real IRQ enters
before its JMP executes, the controller retains the originating stack through
IRQ/RTI, slice boundaries and cancellation, then waits for the actual JMP before
admitting a new statement stop. Same-line statements and returning GOSUB/EXEC
callbacks retain their actual guest behaviour; no host BASIC evaluation occurs.

Session ID, build identity and monotonically increasing control revision reject
stale requests before mutation. Stop/cancel/completion revoke the search token.
Ready, prompt-return, Running, Cancelled, Failed and Terminated states retain no
scalar/register inspection. Accepted Key/Type/Release requests preserve the
search/pause and its budget while advancing revision ownership. Other accepted
mutating session commands clear inspection and revoke an active search;
State/Export are inert. Reset clears the failure and hook-resume state, while a
source/build change requires a new controller. Rejected commands
preserve debug ownership; the existing session owns media command semantics.
Each execution poll drains the existing bounded audio/recording queues once.
The new DME1302/1303/1304 diagnostics are library-only configuration/state,
stale-ownership and search/inspection errors; host allow-lists are unchanged.

This completes reusable breakpoint/Continue ownership at the Rust library layer.
Normal original Run/Debug composition and browser/native acceptance remain
unimplemented gates pending qualified fitting firmware. No additional source
stepping actions, runtime profile, UI controls or OEM-signature exceptions are
introduced. See the [controller receipt](evidence/dragon32-basic-controller-20261018.json).


## Portable development debug wire contract — 18 October 2026

`basic_debug_protocol::BasicDebugAdapter` composes the original controller through
a strict version-1 request/reply boundary. Requests contain session/build/revision
ownership and only State, Continue, Poll, Cancel or an existing MonitorCommand.
The trusted host passes the immutable build/controller in-process; JSON cannot
supply source mappings, firmware, executable paths or profiles. Every reply names
the original-development backend and contains the existing session frame plus
actual stopped source/register/scalar state when valid. No XRoar process ID or
OEM-profile claim is synthesised. Control revisions and stop IDs use canonical
u64 decimal text so browser and native identities remain exact above 2^53.

The request envelope is bounded to 4 MiB + 4096 bytes; replies to 12 MiB. The latter
covers the maximum combined escaped scalar aliases, full CAS export and fixed
frame/scalar metadata. Strict validation includes complete field shapes, revision
ownership, register/string/source bounds and the existing monitor/media checks.
Domain rules remain in Rust. State reads are inert; the service does not poll,
load, type RUN or continue automatically. It admits one call at a time and has a
five-second transport timeout with abort/release and late-result rejection.

A validated, revision-matched command rejection carries a bounded typed failure
with the unchanged control revision and current session frame. The shared owner
rejects that action while retaining the usable session; a failed media selection
therefore retains the existing cassette and real stop. Unowned/malformed envelopes
return no current inspection. Uncertain transport failure disconnects the owner;
explicit Cancel between bounded polls retains the controller's resumable state.

At the adapter-only checkpoint, native activity depended on processId/OEM runtime
configuration, browser Debug was disabled and MonitorConsole issued raw Advance.
The subsequent development host and workbench binding below now supply explicit
session activity and one controller-aware scheduler while preserving existing
actions, layout and the separate native window. The wire contract itself adds no
route. Normal original runtime/profile promotion remains gated on fitting
qualified firmware. See the [adapter receipt](evidence/dragon32-debug-adapter-20261018.json).

The earlier qualified joint native layout used 19,640 bytes, including 37 padding
bytes. It changes symbolic branch widths and module placement while preserving
fixed stubs and excluding timing/RAM-template modules from instruction changes.
All 248 guest tests pass. `qualify-layout.py` reproduces the selected image and
binds its test harness; `build.py` alone retains the 19,653-byte source-layout
baseline. Neither output is a deployable 16 KiB image.

### Development debug host composition — 18 October 2026

The original-development session now has portable browser/native host composition
for a fresh BASIC build and line breakpoint. Compiler token metadata is retained
in-process and checked against the exact cassette payload; external source maps
are rejected. Both hosts run guest CLOAD and genuine 6809 execution. The sole
frame scheduler switches to bounded controller polls after explicit Continue,
freezes at stopped/cancelled/failed states, and permits Reset recovery. The shared
connection queues explicit RUN plus Continue without an intervening frame tick.
No statement execution or variable evaluation occurs on the host.

Library and exported browser-API tests observe line 20 with COUNT=41, cancellation
and resumed ordinal 1 with COUNT=42, frozen stopped ticks, stale requests and Reset.
The native manager also captures actual guest CSAVEM, saves a new cassette, and
rejects an existing destination and a pending save invalidated by Reset. These
are source-level Rust/IPC ownership tests; they are not packaged-window or actual
browser-worker acceptance. Existing UI controls are not activated by this host
patch. Normal runtime promotion still requires the fitting qualified original
ROM and subsequent UI/adoption checks. See
[the host receipt](evidence/dragon32-debug-host-20261018.json).

### Development debugger workbench binding — 18 October 2026

The adopted presentation binding composes the shared BASIC inspector with the
original development host. Native controls stay in the main workbench while the
separate window retains execution and cassette-save ownership. Browser inspection
uses the existing bottom-panel Debug tab, visible only after explicit original
development selection. Selecting Debug prepares a fresh immutable build-bound
session; guest CLOAD precedes Debug's RUN/Continue action. Source/project changes
and teardown release that owner. No normal original-ROM profile is promoted.

Focused host tests continue to execute the unchanged fitted development ROM. UI
tests use typed fake transport/inspection and do not establish actual browser
WASM, native window, packaged, accessibility or physical-device acceptance.

The browser's settings and ROM dialogue use the same launch-ownership guard, so a
development draft or live debug owner cannot start another original session.
At a valid stop, source navigation opens the programme tab and selects/focuses the
reported range after the editor mounts, including when dragon.toml was visible.
F5 invokes the existing explicit Debug action only within that development route;
repeated keydown does not launch again, modal and availability guards still apply,
and the shared console does not send F5 to the guest keyboard. This development
image returns DEV>, so the loading instructions refer to the returning prompt.
The [presentation receipt](evidence/dragon32-debug-ui-20261018.json) separates
initial staged verification, adopted corrections and outstanding runtime gates.


### Development debug prompt and INPUT recovery — 18 October 2026

The controller now detects return to direct input at published hook `$0182` with
`CURLINE=$FFFF`, after observing programme execution at the public statement hook.
The existing version-1 `ready` status reports “Programme returned to BASIC prompt”
and clears stopped inspection. The message deliberately covers END, fall-through
and errors without asserting success. The unchanged guest line hook runs on the
next ordinary frame; an authored callback regression verifies it is neither
patched nor skipped. The same scheduler services prompt input and an explicit
Debug action can RUN the loaded programme again. Boot input and programme INPUT
remain distinct from a return to the direct prompt.

Physical key, submitted text and blur release preserve an active search, its
cumulative budget, a stopped/cancelled pause and any required hook continuation.
Each accepted input advances the controller revision. This corrects the focused
canvas F5/blur race and permits physical replies to guest INPUT during debugging.
Input cannot clear a failed search or execute a cancelled/stopped guest; Reset
recovers failure. Existing media/source/build revocation remains in force.

The [lifecycle receipt](evidence/dragon32-debug-lifecycle-20261018.json) records
actual fitted-ROM library regressions and shared wire validation. These source
checks do not qualify rebuilt browser/native runtime delivery, packaged products,
physical devices, the oversized complete candidate or normal original-ROM Run.

### Observed browser and packaged development journeys — 18 October 2026

The rebuilt browser worker and locally packaged 6809 application now pass the
authored INPUT/Debug journey: physical CLOAD, RUN, reply 41, verified line-30
COUNT=42, Continue and prompt-ready recovery. The packaged cassette journey also
records guest CSAVE, validates and saves a new CAS, then reloads it after NEW and
again prints 42. The recovered source and all cassette blocks pass the ordinary
media inspector. Visible LINE, CIRCLE and DRAW output is recorded; SOUND/PLAY
return to the prompt, without claiming an independently measured speaker output.

These observations use the incomplete fitted development image. Its colour-zero
syntax failure is reproduced in a fresh core session and remains a required
physical-input retest after adoption of the complete candidate's correction.
The [browser receipt](evidence/dragon32-debug-browser-20261018.json) and
[native receipt](evidence/dragon32-native-development-acceptance-20261018.json)
record exact runtime hashes, screenshots and limits. Z80/6502 packaged checks
cover the existing shared shell and authored BASIC builds, not machine emulation.

The subsequent native source-reveal correction uses the Rust-provided statement
range within its original source document. Single-file and multi-file/CRLF
regressions select the actual statement rather than its line-number digit; all
102 App tests and the Dragon UI type check pass. The package receipt predates
this presentation correction and the separately staged normal-original route.


### Gated normal original-runtime composition

The normal original route is distinct from the implemented development route.
It admits only the compiled, hash-matched `eightbit-dragon32-basic-1` manifest with
status `qualified`; the current 16 KiB development image is rejected before build
or launch. Existing XRoar settings/defaults and project schemas are preserved.
An admitted explicit Run/Debug owns fresh source/token/cassette identity, bounded
guest boot/CLOAD and exact loaded-token verification before physical RUN. The
existing single scheduler, breakpoint controller and separate native window remain
authoritative. A versioned optional `originalRuntime` manifest receipt records the
actual original profile, firmware digest and core version without claiming XRoar.
This composition is not permission to promote the incomplete image or the oversized
candidate. Qualification, rebuilt native/WASM and actual runtime acceptance remain
required before that gate can open.


Promotion acceptance must include the actual physical-input path for zero colours,
not only a token-level graphics test. Fresh fitted development-ROM sessions at
SHA-256 `42659208cbd8d81ca658f9d8f2073e85033b5185f86c3c21acae3fdbff8968fe`
returned SN for `COLOR1,0`, `COLOR 1,0`, `PMODE4,1:COLOR1,0` and
`PMODE4,1:COLOR 1,0`; `COLOR1,1` returned to DEV> without SN. These are library
CPU/Board observations through physical keyboard input, consistent with the
separately observed packaged limitation. The complete candidate's correction
must be retested after fitting, rebuilding WASM/native, and repeating the graphics
journey. The normal-runtime composition does not mask this firmware gap.


### Qualification-driven normal UI composition — 19 October 2026

The shared/native/browser presentation now reads the compiled Rust original-runtime
capability. A null, missing or malformed capability offers no normal original
selection. A qualified Dragon 32 BASIC capability adds Original BASIC to the
existing project emulator selector, with XRoar still selected initially and still
explicitly available. No project schema or persisted XRoar setting changes.
Run and Debug use the existing fresh-build owner; the native emulator stays in
its separate window and the browser uses the existing bottom panel. A plain Run
has no fabricated debugger state or PID. Debug presents the existing breakpoint,
Continue and Stop controls. The development build/mount action remains explicit.

The qualification record described in ADR 0057 binds final manifest activation to
exact source, symbol and image hashes under `firmware/basic/`. There is no such
record for the current development image. Source bundling now retains the actual
shared monitor/debug protocol, controller transport, capability parser and Debug
presentation dependencies. Neither the test-only complete candidate nor these UI
mocks qualify a normal execution profile.

Final promotion still requires adopting the fitted complete source into the
maintained `basic/` build, updating its exact module list and keyword dispatch
alongside the fitted layout and truthful guest banner, then rebuilding and reviewing
all source/symbol/image hashes. Only that reviewed qualifying image can acquire
the record and normal identity. Rebuild native/WASM and repeat the ordinary,
source-package and actual browser/native Run/Debug/cassette/graphics journeys on
that exact image before recording normal-runtime acceptance.


### Compiled-profile keyboard capacity — 19 October 2026

The physical typed-input queue follows the compiled firmware identity: the current
BASIC development image retains its 251-character limit, while an admitted
`eightbit-dragon32-basic-1` image uses the complete keyboard/ASCII-record contract
of 249 characters. The shared command field presents the same respective limits.
Rust still validates the complete request and every keyboard chord before enqueueing;
invalid, oversized or overlapping input cannot partially replace pending keys.
Empty BASIC replies continue to submit Return. This preparation creates no
qualification record and does not activate a normal profile. The complete
candidate's guest tests for 249-character INPUT/LINE INPUT, backspace and ASCII
records remain distinct from these host boundary tests.

The submitted-text route also supplies the guest's existing shifted navigation
characters: `[` (Shift-Down), `]` (Shift-Right), backslash (Shift-Clear) and `_`
(Shift-Up). These use physical matrix chords with the established Shift-settle
interval; no character is inserted into guest RAM. An actual guest regression
stores the four characters and mixed-case text in string variables in both
capitalisation modes, retaining the case flag and releasing every key. Braces
remain unavailable and are rejected before queue mutation. This corrects the
host mapper; both maintained ROM keyboard tables already supplied these values.

### Digital joystick and clocked cartridge source verification — 19 October 2026

The Board now maps the existing four 0–63 joystick coordinates to digital ADC
bins, with bin 63 above the maximum DAC setting. A bin is an API convention, not
an analogue-voltage or physical calibration claim. The paired development-ROM
`JOYIN` scan returns all 64 coordinates and leaves the public mux at zero;
the Board change must be adopted with that routine. The old-ROM counterfactual
fails immediately with `[1,18,32,48]` instead of `[0,17,31,47]`.

Explicitly mounted autorun cartridges provide both Q edges to PIA CB1 per E
cycle. Mounting and side-effect-free inspection execute no guest instructions.
Only explicit advancement can service the guest-enabled interrupt; a new
authored cartridge test exercises the actual CPU/FIRQ/RTI path. Reset retains
mounted bytes, axes and buttons while resetting the PIA enables. Ejection stops
the source; an already latched flag remains guest-owned until acknowledged.
Both current monitor and development firmware retain their existing prompts
without automatically executing the mounted cartridge.

The paired development image is exactly 16,384 bytes, SHA-256
`187d66f60d9f5c227a8a8eb74e3f11ca5d24a887430975496edca735c05963a9`, with
21 free bytes (largest gap 18). Its status remains `development`; no
qualification record, normal original-ROM admission, project schema, command,
UI or default/XRoar route changes. No oversized candidate is used by these
Board tests. See the [dated evidence](evidence/dragon32-board-joystick-cartridge-20261019.json)
for exact source/native/WASM checks. Packaged and physical-device acceptance
remain separate gates.
