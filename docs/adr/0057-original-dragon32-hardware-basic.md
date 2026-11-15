# ADR 0057: complete Dragon 32 hardware and original compatible BASIC

- **Status:** Implementation authorised by the user; compatibility requirement confirmed in this chat.
- **Extends:** ADR 0056, preserving ADRs 0032, 0045 and 0052.

## Stock Dragon cassette development requirement — 19 October 2026

The user confirms that authored games must remain suitable for a stock PAL
Dragon 32 with its original ROM, transferred through cassette. The expanded
Studio ROM is a development runtime, not a hardware requirement for exported
programmes. Target standard MC6809 instructions, the original 32 KiB RAM and
digital hardware, the Dragon BASIC dialect and published ROM/RAM interfaces;
do not introduce game dependencies on expanded-ROM implementation addresses.

Preserve the existing standalone machine-code CAS/WAV exports. Add explicit
complete-programme CAS/WAV formats for desktop BASIC/assembly projects, using
the existing generated BASIC loader and its memory/line-space checks. Machine
boot still exports its native cassette. Browser Dragon 32 BASIC and assembly
projects may explicitly download deterministic loading audio from the current
checked build through the shared Rust codec. Audio crosses the worker boundary
as bounded binary data, not a large JSON integer array. Cancellation, revision
changes and failures must never download an old result. No new firmware, tool,
network authority, project schema or workbench layout is required.

Acceptance distinguishes portable format/payload checks, guest loading with
the original stock ROM, and physical cassette loading/gameplay. Only the last
establishes acceptance of the real audio path, input, sound and timing. Physical
acceptance requires the user's machine and remains open until observed.

## Approved original assembler/linker extension — 19 October 2026

The user requires native development of the existing DragonDoom and Battlezone
projects without LWTOOLS. Extend the original shared 6809 assembler into a bounded
source assembler/linker for Dragon 32 RAM programmes. This is an independently
authored implementation; no LWTOOLS source, executable or object-format dependency
is introduced. Both native Rust and browser WebAssembly use the same implementation.

Support the declared assembly source list in its existing order, file-local
symbols, explicit `EXPORT`/`EXTERN` references, deterministic code placement,
absolute `EQU` constants including the existing `_constants` section, and the
instruction/data/expression forms required by those unchanged projects. Preserve
the existing single-source API. Project loading owns file access and root checks;
the processor library receives bounded source text and returns bytes, symbols,
source locations and diagnostics without filesystem or process authority.

The built-in toolchain becomes the default for eligible Dragon 32 RAM assembly
and mixed BASIC/assembly builds. Retain existing project schemas, source maps,
fresh-build Run/Debug, cassette outputs and atomic last-good publication. Browser
projects may carry multiple explicitly declared assembly sources, keeping their
manifest paths, order, load/entry settings and unused BASIC entry. Only declared
source and the existing allow-listed support files participate in interchange;
this does not authorise script execution, arbitrary assets or command hooks.

Retain an explicit LWTOOLS compatibility route. Existing cartridge development,
Dragon 64 and Vectrex tooling remain available through their accepted external
tools; the new original toolchain does not take over those workflows. Macro
systems, includes, arbitrary linker scripts, standalone object/archive formats
and other processors are outside this increment unless an unchanged acceptance
project establishes a specific requirement. Unsupported syntax fails visibly;
it must never silently invoke an external tool.

Acceptance requires both games to build from unchanged source with no installed
tool dependency, meaningful instruction/symbol/source-map comparison against
their established LWTOOLS builds, and actual original-emulator checks for boot,
input, rendering, audio activity and debugging. Native/browser project and lifetime
boundaries, malformed input, unresolved/duplicate symbols, failed-build recovery
and old explicit LWTOOLS routes require regressions. The implementation builds
both unchanged games without LWTOOLS and reproduces their established machine
code byte for byte. Actual native and WebAssembly guest execution verifies both
games, including controls, graphics, post-load audio and source debugging. See
`docs/testing.md` for the source-bound acceptance and remaining release gates.
Publication, deployment and physical-hardware acceptance remain separate.

## Approved default runtime and RAM machine-code integration — 19 October 2026

The user authorises the original emulator and qualified expanded ROM as the
default Dragon 32 runtime for BASIC and RAM machine-code programmes in the
desktop and browser IDEs. This extends the earlier BASIC-only normal launch
boundary below. Runtime selection must not require third-party firmware for
these programmes, and a configured XRoar installation must not displace this
default. Preserve an explicit XRoar compatibility alternative on desktop.

Desktop cartridge projects and cartridge selections use the existing XRoar
route automatically when its separately supplied emulator and firmware are
configured. Otherwise the existing setup flow explains what is required before
Run. Opening a project remains inert: automatic selection does not authorise
automatic execution. Browser cartridges are explicitly unsupported. Other
computers, Dragon 64 profiles and their existing tool/firmware boundaries remain
unchanged.

The user additionally confirmed browser authoring: write, build and run 6809
assembly source out of the box. An independently authored, bounded assembler
belongs to the processor layer, with no filesystem, subprocess or network
authority. The browser composes it with family-owned RAM/cassette validation.
Source builds must cover documented 6809 instructions and addressing modes;
unsupported assembler extensions produce explicit diagnostics. Existing desktop
LWTOOLS projects and their supervised external-tool contract remain compatible.
No third-party assembler implementation is copied or bundled by this decision.

RAM machine-code and mixed-project launches must use the current validated build
or explicitly selected, revalidated media. The guest ROM performs cassette
loading and execution; no host BASIC evaluator, memory injection or proprietary
ROM dependency is introduced. Machine debugging must use the same execution
owner, current-build source mappings and coherent stopped CPU state. Reset,
Stop, cancellation, changed source and failed builds retain their existing
revocation rules. Project schemas, legacy adapter fields, saved XRoar settings,
the workbench and separate native window are preserved.

Payloads may occupy `$0400–$7FFF`. Ordinary loading reserves BASIC's stack below
high loads. A payload spanning the live BASIC stack instead receives a small
original guest bootstrap as the first physical cassette file. It calls published
cassette entries, uses temporary workspace below `$0400`, verifies the selected
header/load extent and enters the checked payload with `S=$0300`. The payload must
establish its own stack; replacing BASIC workspace does not promise a return to
BASIC. The selected-media identity remains bound to the original bytes, and the
host verifies the exact loaded payload before execution. This does not reserve
additional RAM permanently, change firmware or grant a host memory-write API.

The extension is implemented in the CPU assembler, portable runtime, native
preparation/host and shared browser/desktop workbenches. Its focused evidence
includes real LWTOOLS mixed/native builds, full-RAM guest loading, 1,919 matching
native/WASM requests and actual browser assembly Run/Debug. The earlier
19 October BASIC qualification does not by itself prove this machine-code
integration. [Testing](../testing.md) records ordinary gates, real-tool checks,
packaged acceptance and corresponding-source verification separately from public
release. Publication and deployment are not authorised.

## Approved expanded Studio profile — 19 October 2026

The user has explicitly approved a 32 KiB original ROM for Studio's emulator,
with cartridge programmes using XRoar. This supersedes the 16 KiB delivery
constraint below for the new `eightbit-dragon32-basic-extended-1` profile only.
The goal is an independently distributable, compatible Studio runtime; the
expanded image is not a replacement ROM for a physical Dragon 32.

The new profile retains the existing 32 KiB RAM at `$0000–$7FFF`, published
ROM entry points, public RAM interfaces, token and cassette formats. Original
guest 6809 code occupies `$8000–$FEFF` in a deterministic 32 KiB firmware
container. `$FF00–$FFDF` remains the existing I/O/SAM window; `$FFE0–$FFFF`
retains the alias of the original `$BFE0–$BFFF` vector region. SAM all-RAM
behaviour remains unchanged. No banking, executable RAM overlay, additional
reserved RAM, host BASIC evaluation or firmware trap is introduced.

The expanded profile rejects cartridge insertion before changing session state.
Its default cartridge interrupt handler must return safely rather than jump
into the additional BASIC code at `$C000`; public RAM interrupt hooks remain.
Software relying on cartridge/DOS ROMs or unpublished ROM contents is outside
this profile's compatibility promise. The existing monitor and 16 KiB BASIC
development profiles, historical firmware validation and explicit XRoar option
retain their existing contracts.

Normal Run and Debug must select the expanded image explicitly, bind firmware
identity and breakpoint validation to that image, and preserve fresh-build,
cancellation, recording ownership and native-window boundaries. The existing
workbench, project target and backend value are unchanged. Development identity
must not inherit qualification from the expanded image. The new image, symbols,
source and memory-map contract require exact reviewed provenance; image size
alone is not admission. Ordinary tests, real Board execution, native/WASM parity
and browser/desktop acceptance remain required and separately reported. No
commit, publication, signing or deployment is authorised by this amendment.

## Decision

Complete the documented MC6809E instruction/addressing/interrupt model and the
Dragon 32 PAL digital board model, then implement an original guest-executed
BASIC ROM for the shared native/WebAssembly machine. Preserve the workbench,
explicit launch, separate native window, bounded volatile sessions and the
existing explicit XRoar alternative. No commit or public delivery is authorised.

The user requires old ROM entry points and memory layouts as well as language
and file compatibility. A language-only interpreter, host BASIC evaluator or
undocumented host trap does not fulfil that requirement. The BASIC compatibility
contract must cover published ROM calls/vectors, Studio's existing runtime ABI,
linked tokenised programmes, five-byte numerical representation, variable/string/
array workspace and control stacks. Each supported interface needs original
regression programmes and precise observations. Do not label a subset complete.

Only original Apache-2.0 emulator/firmware/glyph implementation is permitted.
Manufacturer manuals and published Dragon interface tables supply facts; no
third-party emulator or recovered BASIC implementation is copied, translated or
used to derive shipped code. Original BASIC source/image/symbols/digests extend
the original-firmware exception. Third-party ROMs remain excluded from all source,
ordinary CI and packages. Black-box reference comparisons require opt-in lawful
firmware with its exact existing fingerprint, and retain only observations.

## Completion gates

CPU: all documented opcodes, flags, addressing postbytes, instruction counts,
stack frames, reset, interrupt priorities/masks, NMI arming, SYNC and CWAI.
Invalid/undocumented encodings remain explicit diagnostics. Board: PIA register/
edge/handshake behaviour, SAM decoding/clocks/display addressing and Dragon 32
alias/contention policy, VDG text/semigraphics/graphics and PAL sync, DAC audio,
cassette digital transport, joysticks/comparator and printer port signals.

BASIC: ordinary commands/functions and error/recovery paths, source/token/media
round trips and legacy ABI probes, all executed by guest 6809 code. Native/WASM
state/video/audio parity and actual browser/desktop journeys are separate gates.
Analogue colour/electrical effects, measured board revision quirks and physical
acceptance must be identified accurately; digital implementation alone cannot
establish them. Public distribution still needs explicit approval and matching
browser corresponding source under the existing GPL edition.

## Interpreter-private development state

Persistent original-interpreter state occupies `$03D9–$03FF`, above the bounded
legacy line-input area and below the text screen. It must not occupy the public
graphics, PLAY or DRAW workspace at `$D8–$E9`. This replaces the development-only
scratch allocation; no shipped project or OEM profile changes. These bytes are
implementation-private and are not an additional user-memory reservation. The
line editor’s private overflow/cancellation flag is $03FA; it must not borrow
public $D8 when accepting the next command.

## ROM space

The fixed 16 KB image may use original stack-based arithmetic microprograms
executed by an original 6809 decoder inside the ROM. This is an internal code
representation: the guest still performs every operation and retains the legacy
numeric/workspace interfaces. No host evaluator, firmware trap, extra bank or
reserved user RAM is introduced. Fixed entry stubs and firmware receipts remain
mandatory, with guest arithmetic tests after compaction.

The 15 October compaction also shares function dispatch, parser helpers, local
failure branches and a single original power-series recurrence. Internal routines
may be repacked between fixed public stubs; relocated RAM templates retain
absolute external calls and their exact operands. Build receipts bind symbols
and report verified NOP gaps. This does not authorise additional ROM banks,
host execution or reserved user RAM. The expanded development image still needs
further compaction and is not a completed replacement.

The deeper compaction shares return/initialisation/conversion code, unifies
ordinary and special function dispatch and adds an original constant dictionary
to the arithmetic microprogram decoder. Original floating multiplication uses
the MC6809's MUL instruction while retaining the full product and existing
rounding. CIRCLE's stack-only integer quadrant/chord kernel preserves fixed
ROM entries and public RAM allocation; selected radius/aspect quantisation facts
come from approved black-box observations. No host evaluator, code overlay,
decompression allocation or additional reserved RAM is introduced. A fast curve
and selected ABI probes do not complete the outstanding language/media scope.

The arithmetic microprogram representation also permits original unary calls,
comparisons and signed frame-word conversions. Decoder state remains on the
caller’s guest stack and nested calls preserve its continuation/frame. Failure
scanning must skip word operands, including operand zero, before resuming native
code with C set. Native parser helpers and shared failure islands remain ordinary
6809 code; the separate parser-decoder experiment was discarded because its
encoded savings did not cover its decoder cost. Original hardware-initialisation
register/value data retain the physical write sequence.

The native expression rewrite uses one eight-byte pending-operand frame: a
previous-root link, a five-byte packed value/descriptor and its type. Only string
operands join the collector root chain. Numeric values retain all 32 significant
bits through the existing packed ABI, and both operands are restored before
application. Success, type errors, depth failures and collection must unwind the
same frame. Shared typed assignment preserves string allocation errors and the
numeric cursor; F_SWAP preserves the caller's U frame. This introduces no new
bytecode engine or persistent state. The attempted general word-code and parser
extensions were not retained because their measured decoder cost exceeded the
converted code savings. This rewrite alone does not supply the space or missing
behaviour required to complete BASIC.

## Explicit development launch

The existing web/native emulator controls may launch the original BASIC image
as **development**, using a closed first-party firmware selector and the same
bounded session protocol. The default monitor and explicit XRoar route remain.
This permits local interactive qualification without promoting the unfinished
ROM to the normal project Build/Run/Debug profile. Keep the workbench layout and
native owned window, and show the actual firmware identity and remaining gaps.
Interactive INPUT stores its bounded reply on the guest stack after a collision
check, preserving direct tokens, DATA state and the published RAM allocation.

The subsequent native interval collector replaces temporary descriptor links
with bounded repeated root scans and moves overlapping unions together. All
roots must validate before the first move, and array enumeration may not borrow
live collector scratch. A Boolean changed flag must not wrap with large root
sets. PAINT uses only checked temporary three-byte guest-stack spans plus a
local frame; it introduces no persistent allocation or host execution. Existing
modules shrink by 201 bytes and the new 342-byte PAINT module leaves 16 bytes
free. Source/image/symbol receipts and fixed stubs remain mandatory. Known
historical PAINT differences stay explicit; neither PAINT nor the development
console completes the outstanding full-ROM and project/media acceptance gates.

The next native pass consolidates complete array-record validation and keeps
carry/borrow loops, swaps and VAL allocation in native 6809 code. It recovers
98 bytes before explicit PRINT devices and reset recovery; the resulting image
has 48 free bytes. PAINT's 310-byte implementation uses two-byte stack seeds:
horizontal border-free spans propagate only when they contain a colour change.
This fixes the observed directional border-gap mismatch and removes completed-
span history. Twenty-eight complete synthetic graphics planes match the lawful
reference; full PAINT ABI/timing/error qualification remains open.

`PRINT#0` and `PRINT#-2` share a restoring device boundary with LLIST. TAB/comma
spacing reads the active column, printer carriage return clears $9C, and warm
reset selects screen output even during printer BUSY. Corrected guest/board BUSY
polarity and bounded wait/resume/reset have independent and board-level evidence.
Printer capture remains volatile and bounded; no host print service or automatic
file output is added. File devices, remaining commands/music/cassette, complete
ABI and normal project integration remain required work.


The 16 October native pass moves first-letter validation and U preservation to
the variable parser, shares quote/DATA/comment transitions and uses the existing
integer normaliser for unsigned values. The arithmetic decoder centralises its
native continuation saves without adding parser operations. Borrow-normalised
subtraction and native packed-colour/decimal loops preserve the numeric precision
and guest-execution boundary. After TIMER assignment and guest-console line zero,
the fixed image has 305 free bytes (257 net recovered). No new permanent state,
bank, parser VM or RAM overlay is introduced. The IDE source/project line-number
contract is unchanged; full language/media/ABI and project integration remain open.

New lawful synthetic GET observations establish a packed-graphics array with a
zero extent and creation of a same-name scalar. Its treatment in array lookup/
collection needs explicit qualification when GET/PUT are implemented; the ordinary
numeric-array validator alone does not establish that interface. Retain only the
reviewed synthetic array metadata/payload, never instructions or raw snapshots.


Direct-command tokens are now produced in place in the published $02DD–$03D8
line-input area. The earlier private $0200 token buffer overlapped the legacy
$01DA–$02D8 cassette buffer. Every vocabulary replacement is no longer than its
source spelling, allowing forward in-place tokenisation without a second buffer.
Output, including NUL, is limited to 252 bytes; successful input, overflow and
cancellation must preserve cassette data, the input preamble and private state.
This corrects a development-only address, preserves the legacy RAM allocation
and creates no new reservation. It prepares the cassette boundary; it does not
implement high-level file commands or qualify their workspace.

## Native statement failure boundary

The authorised native rewrite may consolidate fatal statement parsing failures
through one guest-stack boundary. A statement saves its previous boundary, live
string-root link and caller registers before dispatch. Checked native argument
helpers return directly on success and unwind that boundary on failure; they
must not leak temporary frames, collector roots or expression depth. Nested
published statement calls restore the enclosing boundary. Control-flow commands
continue to install the observed FOR/GOSUB records at CTL_SP, discarding only
temporary parsing frames. Published numerical/peripheral routines and standalone
expression evaluation retain their carry-return contracts. This is ordinary
6809 control flow, with no parser opcode stream, host evaluator, new bank, RAM
overlay or additional user-memory reservation. Two bytes at $03FB–$03FC hold the
private boundary pointer inside the existing $03D9–$03FF allocation.

The failure frame is shared by native statement, expression and variable
boundaries. FN body evaluation catches failure before restoring its lexical
parameter. EXEC removes its temporary twelve-byte frame before the machine-code
handoff. Cold setup initialises the existing $9D entry pointer to the ordinary
error path rather than reserving zero as an invalid loaded address. Numerical
and physical I/O entry contracts keep their independent carry returns.

Original SKIPF and binary CLOADM share bounded native header search and block
input through the legacy cassette buffer. CLOADM relocates unsigned sixteen-bit
load/entry addresses, validates each block's RAM extent and latches the entry
from the checked header. Payload bytes, including a checksum-failing block,
remain in guest RAM on I/O failure, matching the retained lawful observation. No
host file authority or normal project runtime promotion is added. Exact-profile
lawful observations and independent original-ROM XRoar tests use original
synthetic cassette files with fast tape traps disabled. The strict observer
retains only the reviewed original marker/status/RAM entry values.

Expression parsing now saves caller Y/U once per expression. A precedence call
keeps only its threshold, and pending operators alone allocate packed values and
collector links. Nested functions, collection, failures and external statement
calls must preserve those ownership boundaries; no parser decoder is introduced.
Tokenised CLOAD stages each block and checks the programme/stack extent before
copying. It rebuilds links by bounded sequential parsing rather than following
untrusted addresses. A failed selected load leaves valid NEW state; successful
loading during programme execution stops the overwritten programme. Exact OEM
partial-load/error behaviour remains a qualification item. The command loop must
check failure carry before resetting control state, whose CLR changes that flag.

The failure/cassette-loading image had 31 free bytes, with a largest gap of six. Relative to
the 305-byte baseline, pre-existing modules recover 191 bytes while the three new
cassette modules occupy 465 bytes. This does not satisfy the required substantial
space recovery or remaining replacement scope. It authorises no distribution,
extra bank, general parser VM, executable overlay or additional reserved RAM.


## Shared native grammar and control helpers

Required delimiters may share an ordinary native subroutine taking the expected
character in A. The helper preserves Y/U and leaves the token cursor at the first non-space character on a
failed match, consumes exactly one matched character after whitespace, and
unwinds the existing failure boundary on error. Parenthesised expressions own
their opening/closing checks so callers cannot duplicate or omit those guards.
These are native calls, not an encoded parser programme. Carry clears may be
removed only where preceding native operations prove the same success result;
public numerical/peripheral carry contracts and signal-timed bodies stay intact.
This consolidation prepares space for the remaining language/media routines;
all fixed stubs, RAM interfaces and failure/collector tests remain mandatory.

Tokenised CSAVE shares the existing cassette name/cursor frame and legacy buffer.
It validates the linked programme before enabling the motor, emits a 15-byte
original header, up to 255 programme bytes per block and an empty EOF block,
then disables the motor. The native writer streams directly from TXTTAB through
VARTAB without borrowing graphics/music state or allocating a second programme.
Tests must decode the actual DAC periods, validate the resulting file through
the independent media parser, and reload it through guest CLOAD. ASCII and
machine-code save, device files and physical cassette acceptance remain separate.

CSAVEM shares the named-file writer with an inclusive RAM end address and
unsigned execution address. Parsing/range checks finish before motor-on. An
independent XRoar output capture exposed a pre-existing asymmetric zero-bit
waveform: correct rising-to-rising periods alone did not establish valid tape
output. The byte writer now balances both half-periods while retaining its bit
periods, LSB order and caller registers. Duty-cycle tests and independent decoded
capture are required in addition to the guest writer/reader round trip.

That grammar/output image had 32 free bytes (largest gap five), recovering
194 bytes in previous modules against the 31-byte-free baseline, then adding a
five-byte opening helper and 188-byte writer (189 bytes net before the writer). This is still short of substantial recovery for the
outstanding scope. The original opening-delimiter helper occupies an existing
fixed-entry-point gap; all public entry stubs are unchanged. Final-bit low delay
allows for the following native framing/call overhead; inter-byte period tests
are mandatory. The XRoar full save-capture gate remains failing on trailing
non-block data after its correct output blocks, and is not qualification evidence
for canonical file export. No normal project runtime promotion is authorised by
these partial results.

## Native programme walker and cassette capture qualification

Strict validation, programme reindexing and tokenised cassette loading may use
one native bounded line walker. Strict mode must never mutate programme bytes;
repair mode may replace links only after checking each complete line body. Both
modes enforce the same line-number ordering, terminal position and RAM bounds.
Failed cassette loading continues to restore NEW state. The walker preserves
the caller's cassette frame and introduces no interpreter decoder or workspace.

The earlier XRoar capture failure was a test-reader defect: XRoar appends CUE
timing metadata after the cassette blocks. The original test reader now validates
the documented footer offset, bounded variable-length chunks, nonzero timings
and complete ordered data coverage before checking every raw cassette block.
Unknown chunks and malformed/truncated/range-invalid suffixes remain rejected.
This qualification helper does not extend the production media import contract
or discard timing information during user-file conversion. Its format facts come
from the authors' [CUE specification discussion](https://archive.worldofdragon.org/phpBB3/viewtopic.php?f=8&t=5621)
and the [XRoar manual](https://www.6809.org.uk/xroar/doc/xroar.shtml), without reading
reference implementation code. It supersedes the trailing-data diagnosis above.

Native statement bodies may rely on the existing dispatch frame to restore
caller Y/U rather than saving those registers a second time. Their local stack
allocations and nested helper ownership remain explicit; numerical and peripheral
entry points retain their independent register contracts. Native RND uses bounded
base-256 multiplication while preserving the observed seed and product scratch.

Cassette output reads the guest's complete sixteen-bit bus, including our ROM
and the final address. The output block count bounds every read; the RAM-only
write limit remains specific to cassette input. CSAVEM validates an ordered
inclusive range before motor-on and streams it with a remaining-byte counter;
zero represents 65,536 bytes only at the initial full-address-space boundary.
This adds no host file access or OEM bytes to fixtures. Tests use our original
ROM and generated RAM only. Volatile workspace, active stack and I/O reads retain
their ordinary guest-bus effects; this is not an atomic machine snapshot.


The shared native digit scanner and radix parser retain the newly isolated lawful
numeric observations recorded in the BASIC specification. In particular radix
values are unsigned, while logical operations retain their signed-word range;
negative HEX$ is rejected. Do not infer a successful statement from an allocated
but zero-valued destination: continuation markers distinguish errors in probes.
Complete cassette output blocks, rather than individual bytes, mask IRQ/FIRQ to
avoid byte-boundary jitter; the caller's masks are restored before status return.
Independent WAV checks compare exact payloads even when a checksum is valid.


## Native jump islands and ASCII cassette streaming

Ordinary native conditional checks may share nearby JMP islands. BSR through an
island tail-jumps to its native callee and retains the original caller's return
address, including arithmetic microprogramme continuations. No new opcode stream
or decoder is introduced. Success flags/registers and the existing statement
failure boundary remain authoritative. Signal-timed cassette bodies are excluded
from this layout optimisation. Fixed public stubs and measured complete-ROM
accounting remain mandatory.

ASCII CSAVE streams the existing original LIST implementation through the legacy
$01DA–$02D8 cassette buffer. A temporary negative value of the existing private
FORMAT_BUFFER flag selects this sink; no additional persistent state is needed.
Each full or final block gets an original finite approximately half-second gap.
The output header uses the observed BASIC/ASCII/gapped flags and the text starts
with CR, then the ordinary numbered listing, without an extra EOF newline.

ASCII CLOAD stages blocks separately from its bounded 251-byte line in the
published $02DD input buffer. A nested native failure frame lets malformed source
restore NEW state and stop the motor. CR terminates a line; LF is ignored and an
unterminated EOF tail is discarded, matching isolated lawful observations.
Tokenisation/storage remain guest native code. Filename-expression RUN loads
first, then starts the selected programme; failure cannot run the old programme.
This does not promote the unfinished ROM or expand the production media-import/
WAV format contract. Remaining language, graphics/music, data files, ABI, hardware
qualification and normal web/native project integration are still required.

The CUE qualification reader now permits silence chunks to replace only ordered
raw $55 leader padding. Every non-padding byte must still be covered exactly
once. Missing silence, non-leader holes, reordering, overlap, invalid chunks and
truncation remain errors. This addresses real gapped ASCII captures using the
same primary format facts as the earlier CUE reader; no GPL source is consulted.

Warm reset clears the temporary ASCII output sink before printing its prompt,
as well as restoring screen device selection and motor-off hardware state. A
board-level regression interrupts an active full-block save, verifies reset
clears $03F6 and the motor, then prints successfully through the physical
keyboard path. Temporary output routing must never survive cancellation/reset.

## Shared ASCII media integration

The authorised file-compatibility work adds explicit ASCII/gapped BASIC CAS/WAV
variants to the existing shared media inspection/provenance contracts. Existing
tokenised identifiers are unchanged. Recovery uses bounded checked block parsing
and the existing BASIC source/tokeniser; it is not a host execution engine.
Complete CR-terminated lines apply replacement/deletion in order; unfinished EOF
text is discarded with a warning. Existing project line-number/dialect rules
still apply. Browser recovery and native import/preview use those same APIs.
CLI conversion can explicitly encode ASCII CAS/WAV. Desktop external ASCII CAS
uses the established separately installed XRoar process boundary and identity
revalidation. No new filesystem, process, network or automatic-run authority is
introduced. Generic CUE/custom-loader support and normal original-ROM project
runtime integration remain unimplemented. The shared workbench is preserved.

The arithmetic decoder's failure path now scans directly to its native error
exit, skipping word operands even when zero. Copy/compare operations retain their
separate continuation, while arithmetic/conversion errors preserve C. Removing
the previous failure byte and redundant jumps recovers 12 bytes, leaving 74;
this does not provide the substantial compaction required for the full ROM.


## Native values, register roots and integer interpolation

The next native rewrite shares typed five-byte value loading/storing between
variables and pending expression operands. The pending eight-byte value/root
frame and string-only collector chain are unchanged. Active collection now
visits live FAC/ARG pointer words directly through the ordinary interval visitor;
one temporary stack byte selects active registers. Ordinary collection excludes
dead register values, and complete validation still precedes every heap move.
No persistent state, public RAM allocation, parser VM or executable overlay is
introduced. Native conditional guards and common failure unwinding retain the
same failure-frame ownership and carry semantics.

The original CIRCLE quarter table now contains upward-rounded Q12 knots and
returns signed Q14 through native MUL interpolation. Every adjacent difference
fits one unsigned byte; modulo-256 low-byte subtraction therefore gives the
complete non-negative slope. Exact cardinals, bounded interpolation error and
clipped plotting remain mandatory. Positive fractional RND bounds are truncated
before scaling, and public PLAY defaults include the separately observed DAC
levels. These original routines use only mathematical/interface facts and
synthetic black-box observations. A net 123-byte reduction leaves 197 free bytes;
substantial further compaction and the unfinished feature/qualification scope
remain required. The workbench, monitor default and explicit XRoar route remain.


## Numeric rounding compatibility correction

Bounded synthetic black-box tests now distinguish the original development
arithmetic's nearest/even policy from the required legacy interface. Exact
halfway magnitudes in positive and negative addition/multiplication round away
from zero in the approved Dragon 32 profile. This supersedes nearest/even as an
implementation choice; it is a compatibility correction within the authorised
legacy numeric ABI. Retain 32 significant bits and guard/sticky arithmetic.
RND additionally retains product bits 40–47 as its return guard while its stored
seed remains the low-32-bit recurrence. A shared original native 64-bit product
serves floating multiplication and RND, with temporary guest-stack storage only.
No reference instructions, extra bank, host execution or persistent allocation
are introduced. Wider numeric/ABI qualification and missing commands remain open.


The same numeric qualification observes that invalid string RND arguments still
advance once before the type error. The numeric dispatch guard performs a
zero-argument discarded advance only for this observed case, then uses the
existing native failure boundary. It must not derive a seed from a descriptor or
continue the failed command tail. This retains the existing guest error ownership.


## Ordered development-console teardown and shared native addition

Stop disables input/advancement and clears buffered audio immediately, then waits
for queued commands and its owned Stop acknowledgement before closing the native
window or disposing the browser owner. A failed transport still permits owner
teardown. Window-close failures retain an actionable Stop retry. In-flight Stop
is deduplicated; generation identity prevents an old completion closing a new
session. The destruction callback captures its window’s session identity: a late event
neither releases a newer session nor takes focus from it. Otherwise destruction
releases the owned session and returns focus to the main workbench. Existing child-window permissions and command allow-lists
are unchanged. OS window closure and browser disposal remain independent cleanup
paths. Ordinary lifecycle tests and actual packaged acceptance remain separate.

Native floating addition/subtraction may share one carry loop: a zero/FF mask
selects the operand or its complement and seeds the guard carry accordingly.
The complement and descending loads/stores preserve C between bytes. The
subtraction path interprets the final carry as absence of borrow, then restores
the magnitude/sign before the existing normaliser/rounder. This introduces no
new arithmetic microprogramme, decoder, bank, public scratch or persistent state.
The public numeric ABI and all fixed ROM entry points remain mandatory.


## Native diagnostic boundaries

The native decimal parser may share the checked line-number scanner while
saving/restoring its caller's pending line word, Y and U. Statement/function
keywords remain in their public high-bit-terminated lists; one native ordinal
scan traverses the contiguous lists and maps function tokens after a match.
Neither change introduces parser operations or encoded grammar.

Selected language boundaries classify type mismatch, illegal numeric arguments,
arithmetic overflow and division by zero; other failures retain syntax error as
the development fallback. This is not complete legacy error qualification.
The existing private byte $03FD stores the selected diagnostic and resets at
each prompt/statement. Pure numeric conversions return carry without entering a
parser frame or writing that byte. The prompt reports an active programme line,
then clears it before accepting the next direct command. Keep error source/data
in the strict firmware receipt and preserve native/WASM error rejection checks.


## Native decimal and frame compaction

The 17 October pass retains native control flow, the existing arithmetic
microprogramme vocabulary and all public ROM/RAM/token contracts. Internal
line, circle and cassette frames may bias U for shorter indexed instructions;
array creation and validation include the five-byte element width in their
checked extent product. A validated array's descriptor walk need not repeat
that same extent proof for every element. Descriptor updates cannot alter its
header. Native optional-USR, ASCII-sign and PIA-mux helpers share existing rules.

Decimal display uses an original 6809 packed-BCD accumulator with forty integer
and fifty fractional positions. Native DAA doubling and decimal halving retain
the complete value needed to round once to nine significant digits; truncation
below 10^-50 cannot affect the tenth significant digit of the normalised MBF
range. The 59-byte temporary frame is collision-checked before output and uses
only the guest stack. No host conversion, executable overlay, bank or permanent
RAM allocation is introduced. Original Python Decimal fixtures provide an
independent numerical oracle; lawful reference observations remain a separate
selected-compatibility gate. This does not authorise normal project promotion
or change the outstanding full-replacement acceptance requirements.

## Guest lexical compatibility

The original guest console and ASCII cassette loader use the observed Dragon
lexical rules: a keyword can precede an adjacent operand (`PRINTA`), whereas an
already-started uppercase identifier protects subsequent letters/digits
(`BORDER`). Raw lowercase is preserved; `?` abbreviates PRINT; ELSE and apostrophe
comments receive their legacy colon prefix. These are guest input rules. The IDE
compiler's existing case-insensitive source and identifier contract is retained,
as are its versioned project formats and already-tokenised programme payloads.
An in-place apostrophe expansion must read each following source byte before its
output write; it may not overwrite an unread byte or escape the 252-byte output
area. Exact token facts come only from bounded, original synthetic source lines
on the explicitly provisioned reference ROM, without inspecting ROM instructions.


## Native DRAW and physical line rasterisation

The 17 October continuation implements DRAW in the fixed 16 KiB ROM. Macro text
is copied into bounded transient guest-stack storage so nested string-variable
lookup and collection cannot invalidate it. The native handler retains B/N
movement flags across nested macros and uses the published angle, scale and
cursor fields. It shares string/number parsing with the forthcoming PLAY work;
PLAY is not yet implemented. No executable RAM, extra bank, host parser or new
permanent workspace is introduced. Every published stub and token-table entry
remains in place.

Lines are rasterised after mapping endpoints into physical pixels. Directional
equality rules are original implementations of the retained synthetic plane
observations. Logical caller endpoints, including their low bits, are restored
after rasterisation. Complete-circle raster equivalence and all volatile drawing
workspace/timing remain qualification work.

Native compaction shares a byte-copy loop, allocating literal-copy path and a
stack-space guard. The guard accounts for its own return/saved word and retains
the caller's 128-byte margin. A proposed borrowed direct-literal representation
was discarded because it changed observed heap placement and overlapping MID$
behaviour. Existing allocating literal semantics remain authoritative. This work
retains native control flow and the existing arithmetic microprogramme vocabulary.


## Observed circle polygon and shared integer products

The 17 October CIRCLE rewrite is a compatibility correction within the existing
native architecture. Original synthetic canvases establish a 64-segment polygon,
physical-radius truncation before aspect scaling, endpoint clamping, quantised
arc bounds and clockwise wrapping. Mathematical quadrant coefficients and a
bounded native 16-by-16 product replace the earlier floating-point approximation.
No reference instructions are inspected or used. `circle-raster.json` retains
86 complete canvas digests and success markers; the originating commands live
in `scripts/dragon32-lawful-abi.py`.

The native multiplier retains all 32 product bits. Array sizing checks its high
word for overflow and preserves the existing complete record validation; unit
extents take a direct path so maximal-dimensional arrays retain bounded cost.
Obsolete approximation-helper tests are replaced by exact reference canvases
and complete-product tests. The original ellipse speed ceiling remains intact.
Published entry points, RAM allocation, the 16 KiB limit, UI and XRoar boundary
are unchanged. Further ROM recovery and the remaining language/media/integration
work are still required.


## Native PLAY and shared value paths

PLAY uses the existing bounded guest-stack macro engine, the legacy DE–E9
parameters, DAC pins and frame timer. It introduces no host parser, additional
bank or permanent RAM allocation. RUN resets music and DRAW defaults while
retaining graphics state; CLEAR and NEW retain those defaults, as established by
original synthetic observations. PLAY closes the DAC gate after each completed
note or pause; SOUND shares the native selection/disable helpers while retaining
its timed waveform loops. Selected pitch tests allow four E cycles around the
observed medians; exact historical PLAY timing remains unqualified.

Native division retains the carry beyond its four-byte remainder directly,
reducing its quotient/remainder frame to nine bytes without losing guard or
sticky precision. LIST/trace/error integers share decimal presentation, seeding
unsigned integers directly into the existing BCD conversion. Negative formatter
routing still selects ASCII cassette output. Expressions are bounded by actual
stack/live-storage collision checks rather than an artificial nesting count.

The private failure frame saves D/Y/U in a single caller record. Internal inline
entries take their native body from an ordinary JSR return address; no opcode
stream is decoded. Native continuations and public ROM entry stubs remain
unchanged. Single-character strings share the existing copying allocator, and
validated programme relocation shares the forward tail-copy loop. These changes
preserve the legacy heap, array records and public FAC conversion contracts.

The 16 KiB limit remains binding. PRINT USING, EDIT/RENUM, GET/PUT, cassette data
files, complete compatibility qualification and normal project integration are
still required; this remains development firmware and does not change XRoar or
the approved workbench.


The continuing 17 October native pass fits original GET/PUT into the unchanged
16 KiB image. Shared rectangle parsing, numeric copying/normalisation and checked
native helpers recover space without a new decoder or persistent RAM. The
399-byte transfer module uses a checked temporary stack frame and retains its
fast byte-copy path; a smaller bit-only experiment was rejected because full-plane
transfers became substantially slower. Numeric packed-array markers remain
compatible with lookup and collection. The development boot warning remains
explicit, with its second line shortened to `NOT QUALIFIED`. The remaining six
bytes do not resolve the space required for the outstanding language/file scope.
Audio reset also clears partial sample integration, preserving peripheral state.


### PRINT list and identifier qualification, 17 October

The native PRINT loop shares its separator handling before each item, allowing
leading/repeated separators and adjacent expressions. Explicit device selection
enters that non-empty item path; an empty device/comma argument still fails.
Scalar and array names use the existing whitespace scanner throughout their
name, retaining their two significant bytes. The printer inserts one blank
before an empty line's carriage return, and waits for the physical busy input
before changing its public column. The added fixture retains only original
synthetic text and success observations. These changes fit the same 16 KiB image
with six free bytes and preserve the hardware entry, workbench and XRoar route.
PRINT USING remains a candidate outside the deliverable ROM until it fits and
passes the remaining qualification; its test staging is not a larger-ROM option.


The 17 October bounded text-output change shares the character router between
STR$ and future detokenised editor buffers. Positive FORMAT_BUFFER selects RAM
output, FORMAT_TARGET advances after each write, and FORMAT_LENGTH is remaining
capacity. Negative cassette output retains its separate buffered-count meaning.
No new permanent byte is allocated. LIST suppresses the tokeniser's structural
ELSE/apostrophe colons only in code context. Original guest/reference tests cover
capacity boundaries, register/guard preservation and quoted/DATA/user-colon cases.
PRINT USING and RENUM remain isolated development candidates; their larger test
address space does not relax the fixed 16 KiB firmware requirement.


### Shared text, input and cassette workspace, 17 October

Native tokenisation, listing, conditional skipping and DATA search share one
bounded lexical-context transition routine. It distinguishes every function-code
byte from a statement token and retains quoted/DATA/comment boundaries. Direct
and cassette source lines share checked ingestion; unsigned byte comparison
shares one native loop with defined empty-span and mismatch flags. No decoder
or permanent workspace is added.

Interactive INPUT owns its CSV cursor in its temporary frame and leaves the
published DATA cursor unchanged. Retry restores the saved destination type at
its current frame offset. Native failure boundaries save/restore temporary output
mode, so failed expressions cannot retain a formatter sink. ASCII cassette output
uses public $79 for its persistent buffer count; FORMAT_LENGTH remains the RAM
sink's capacity. The pending data-file candidate shares the existing header,
block and ASCII-output code and the ordinary INPUT frame/variable loop. It uses
public $78/$79/$7A for mode, count and cursor, with bounded temporary record data.
These candidate routines still require fitting and qualification in the fixed
16 KiB image; isolated staging does not authorise a larger ROM or runtime bank.

Exact-SHA opt-in physical-matrix observations establish shifted control results
and Shift-0 alpha-lock consumption. Only original synthetic keys and observable
public values are retained, with no firmware instructions inspected. These are
digital board/interface checks, not physical-machine acceptance.


### Native candidate failure boundaries, 17 October

The staged RENUM preflight checks both the caller's live stack and the configured
BASIC storage ceiling before any programme/variable changes. Staged EDIT keeps
its bounded line buffer separate from stored programme bytes until successful
storage; physical-matrix reference observations confirm that Break remains
inside both EDIT modes. The staged cassette lifecycle keeps END distinct from
STOP and natural completion, rejects conflicting save/skip operations, and
abandons an open data stream on CLOAD. Original record/header observations own
its mode/flag semantics; malformed streams stop the motor and unwind through
the existing native failure frame. These candidates remain outside the 16 KiB
image. The command-interpreter alternative has been raised for a user decision;
this ADR's existing native-only parser boundary remains in force.


The combined native candidate now has original cross-command tests for EDIT,
RENUM, formatting and cassette round trips. Base Dragon 32 DLOAD follows the
observed close-before-argument-validation and unavailable-transfer error paths;
no serial/network facility is introduced. The padding-free assembled candidate
measures 19,163 bytes, exceeding 16 KiB by 2,779 bytes. The measurement image does
not preserve fixed public positions and is never a runtime or distribution input.
All 202 combined staged tests and the actual source tree's ordinary gate pass;
neither result changes the native-only boundary or completes the replacement.

### Approved guest command-interpreter experiment, 17 October

The user subsequently approved a compact command-bytecode interpreter experiment
to address the measured 2,779-byte deficit. This supersedes the native-only parser
restriction for this experiment. Its original decoder and every command must run
entirely on the emulated 6809, alongside the existing arithmetic microprograms.
Account for the decoder, dispatch data, entry bridges and stack frames when
measuring savings. Preserve the 16 KiB image, fixed public entry addresses, legacy
RAM layout, numerical precision, error recovery, collector roots and nested
published calls. No host execution, extra bank, executable overlay, decompression
allocation or permanent user-memory reservation is permitted. Signal-timed I/O
must retain its qualified behaviour. The workbench and XRoar alternative remain.

The experiment is not approval to promote incomplete firmware. Adoption requires
an actual fitting image and guest regression evidence, including nested calls,
failure unwinding and boundary conditions, followed by the ordinary gate and the
remaining native/browser qualification. Record unsuccessful measurements without
presenting estimates or a larger development address space as a fitting ROM.

The measured call-token prototype preserves register/flag and nested-failure
boundaries, but adds 204 bytes to the complete fixed-entry native candidate.
The broader native-sequence translation is also larger and violates qualified
cassette timing and execution budgets. Neither representation is adopted.
Native continuations into the middle of an interpreted routine require a resume
bridge, distinct from a new function frame. A shared decoder also adds latency
between otherwise unchanged physical I/O routines; retaining only their inner
bit loops does not preserve a complete signal contract. Subsequent experiments
must include those transition costs as well as all decoder/table/bridge bytes.


## Explicit development cassette attachment and export — 18 October 2026

The authorised media integration extends the existing development session with
strict, volatile CAS mount/rewind/eject and explicit guest-pin recording. Playback
streams bounded immutable CAS bytes through board I/O; recording drains physical
output after each execution slice and uses the existing media codec to validate
a single recoverable CAS. The input cap is 1 MiB including framing, and recording
is bounded to 15 minutes of 9.6 kHz PCM. No host evaluator, memory injection,
firmware trap, automatic guest command or normal project profile promotion is
permitted. Bad replacement input preserves the current attachment; invalid or
incomplete recorded output cannot be presented as a valid cassette.

The browser may mount a fresh project CAS through an explicitly labelled
original-BASIC development action and download a validated recording on request.
Native users can select a CAS in the existing separate development window and
save recorded output with a narrow Rust-owned dialogue. That command accepts only
its owned session identity, selects the recording in Rust, revalidates it after
the dialogue and creates a new local `.cas` destination without overwriting.
It receives no caller-supplied file bytes/path and adds no general filesystem,
dialogue, process or credential permission to the child WebView. Closing or
replacing the session invalidates any pending write. Reset and discard also
invalidate the pending recording revision, including a subsequent recording with
identical bytes. One pending Save dialogue is admitted independently of the
bounded execution queue, so Stop remains available. Ordinary firmware/default
selection, the workbench and explicit XRoar route remain unchanged. Multi-file
tapes, data-device records, WAV input/export and packaged/physical acceptance
remain separate gates.

## Guest Break and continuation — 18 October 2026

Break is sampled through the physical keyboard matrix between statements. Its
poll preserves the keyboard/printer output latch and updates the existing key
rollover bit before returning to the prompt. The existing continuation path owns
the saved FOR/GOSUB frames; STOP and physical Break share the public line-number
message suffix, while END is quiet. Brief Break during SOUND/PLAY is ignored as
observed with the exact lawful reference profile. No host interruption, new
persistent state, keyboard trap or alternate continuation format is introduced.
Native INPUT, numeric and expression compaction plus connected-module packing
retain the 16 KiB image and fixed entries with 20 bytes free. The staged command
set still does not fit; this development image is not the complete replacement.


## Typed development project preparation — 18 October 2026

The explicitly gated project journey uses a separate schema-1 original launch
contract, accepting only the original-development backend and Dragon 32. Optional
CAS bytes must match their digest and a supplied build identity must be a bounded
hash. This contract selects only the built-in development BASIC image and never
accepts a ROM path, executable, host load trap or automatic guest command.

For native BASIC-only Dragon 32 projects, Rust prepares a fresh build, rechecks
its input hashes and bounded regular CAS against the same in-memory manifest,
and retains one prepared payload under an owned request identity. UI source
revisions are checked before explicit activation in ADR 0045's separate window.
Cancellation, timeout, host shutdown, superseded preparations and failed window
publication release the owner; stale work cannot replace a later request.
Native preparatory work is bounded and checks cancellation around synchronous
build/file stages. Browser initialisation binds the fresh CAS digest through the
same contract. Existing monitor/default constructors remain supported.

This independent development action preserves project schemas 1–5, runtime
settings schemas 1–2, `run.adapter = "xroar"`, normal Play/Run/Debug and exact OEM
profile/signature checks. A fresh build receipt is not a declaration of complete
original BASIC compatibility. Normal backend promotion and packaged browser/
native user acceptance remain separate completion gates.

An abandoned native preparation expires after 30 seconds and is discarded before
a later launch is admitted. This recovers a UI reload without retaining an
unusable launch reservation.


## Multi-file and data-record transport — 18 October 2026

The explicit development transport may retain complete multi-file tapes and opaque
gapped data records. The family-owned validator bounds bytes, file count and
block count; source imports and ordinary WAV APIs keep their strict single-programme
contracts. The additive transport decoder rejects damaged or incomplete streams
without exporting a valid prefix. It never evaluates records or injects guest RAM.
Existing version-1 receipt fields and format labels remain valid, with additive
`ascii-data` and `cassette-stream` labels; UI layout, session/write ownership and
normal XRoar/default selection are unchanged. See the [transport contract and
limits](../dragon32-basic-specification.md#multi-file-and-data-record-transport--18-october-2026)
and [verification](../testing.md#original-development-multi-file-and-data-transport--18-october-2026).
The staged OPEN/PRINT#/INPUT# firmware must still fit and pass real guest acceptance
before data-file language compatibility is claimed.

The existing development mount action also accepts explicitly selected PCM WAV
through an additive `mount-wav` session command. Both input formats retain the
1 MiB session envelope. Rust owns RIFF/PCM validation, channel ambiguity and
complete transport recovery; only the checked recovered CAS is retained and
sent through PIA pins. Its receipt describes those recovered bytes. A failed
decode cannot replace the attachment or execute the guest. Existing CAS launch
contracts, ordinary media import limits and normal Run/Debug remain unchanged.


### Staged descriptor and public RAM corrections — 18 October 2026

The complete, still oversized original candidate qualifies the published token
descriptor chains and returning callbacks. Raw input starts at `$02DD`; crunched
direct tokens start at `$02DC`, while EDIT and RENUM keep separate raw/token sink
origins. Statement/function descriptors retain their independent terminators,
word-list/dispatch rebinding and the fixed extension dispatch ownership. The
standard statement cutoff remains separate from the 78-word lexical count.

Programme INPUT now stores actual replies in the public input buffer and exposes
its CSV cursor through `$35`; this supersedes the earlier stack-reply workaround
because qualified direct INPUT is rejected before parsing arguments. The stack
frame shrinks by 256 bytes. LINE INPUT preserves `$35`. The public EOF flag and
wrapping motor-delay count now follow authored exact-profile observations, with
cold `$95–96=$DA5C`. These changes use no additional permanent RAM. They are
qualified in the 19,703-byte/232-test candidate only: the ROM-fit and promotion
gates remain outstanding, and the shipped development image is unchanged.


## Portable BASIC breakpoint ownership — 18 October 2026

The original development emulator may compose its existing media/input session
with a Rust-owned line-breakpoint and Continue controller. It uses the original
fixed statement entry, existing token/source mapping and shared scalar reader;
it executes real guest instructions and preserves public hooks. A statement
stop is published only after the loaded programme matches the bound build map.
The controller owns session/build/revision checks, bounded execution, cancellation
and reset/stop recovery. Browser/native adapters must serialise requests, yield
between slices and reject results whose revision no longer owns the view.

This is a portable library contract, with no new normal runtime profile, host
command, settings migration or UI action. The existing BASIC breakpoint/Continue
scope is retained; step-in/over/out remain deferred. Polling does not implicitly
load media or type RUN, and idle state inspection executes nothing. The existing
MonitorSession remains the sole cassette, audio and input owner. A fitting
qualified image and explicit normal launch/debug composition remain separate
gates; exact OEM signatures and the XRoar paths are unchanged.


## Native window visibility — 18 October 2026

The shared console keeps its existing browser hidden-tab Stop behaviour. Its
native host instead pauses scheduled CPU slices and clears queued audio while
WebKit reports the separate window hidden, retaining the owned guest and media
session. WebKit can report an occluded window or file dialogue as hidden; that
notification must not close the emulator. Returning to visibility resumes one
bounded polling chain. In-flight slices may complete, but their audio is dropped
across the visibility transition. Blur releases physical keys. Explicit Stop,
window closure and component disposal retain their existing session teardown.
No workbench layout, control, runtime profile or XRoar behaviour changes.


## Portable development debug wire boundary — 18 October 2026

Browser/native composition may use a shared strict version-1 adapter around the
existing original-development controller. Its constructor receives a trusted
controller/build map in-process; external requests cannot submit source maps,
firmware, runtime profiles or a process ID. Rust retains all execution, source,
media and recovery rules. The shared TypeScript service validates replies and
serialises bounded calls, with lossless decimal-text u64 revisions, cancellation
and stale/disconnect handling. Existing monitor validators are reused outside
React without changing the console's rendering or actions.

A revision-matched rejected action returns its current owned frame plus a bounded
typed failure, preserving cassette/inspection recovery. Stale or malformed
requests receive no current inspection; uncertain transport failures release the
owner. This adapter enables no host route or UI action. Connecting the existing
controls requires explicit session activity instead of XRoar PID inference and
one controller-aware frame scheduler. A fitting qualified original image and
normal runtime promotion remain separate gates; OEM signatures are unchanged.

## Explicit development debug host ownership — 18 October 2026

The portable controller/adapter may be composed by the existing browser worker
and separate native emulator window for an explicit original-development debug
preparation. Native preparation retains the current compiler's token object and
checks the same fresh manifest/input/cassette identities used by development
launch. Browser preparation rebuilds the supplied bounded source in Rust and
requires its source and cassette digests to match the current build. The trusted
statement map is made in-process, compared with the exact cassette programme,
and never accepted from an external serialised map. No external firmware or
normal runtime-profile selection is added.

One Rust `DevelopmentSession` owns either the existing monitor session or the
existing debug adapter. The existing frame scheduler advances boot/CLOAD during
preparation; after explicit Continue it polls the controller, and at a stopped,
cancelled or failed search its ticks are inert. Reset restores preparation. Debug
IPC permits only state, Continue and cancellation; execution slices, guest input,
media and capture continue through the existing session channel. Native commands
share command admission and exact window/session ownership. Recording destinations,
existing-file protection and pending-save revision checks retain their original
native authority.

A shared bounded connection serialises frame operations with revision refresh
and explicit control. Its explicit start queues the real guest RUN command and
Continue atomically relative to frame ticks; callers first complete guest CLOAD.
It does no background execution. A five-second operation limit, sixteen queued
actions, cancellation, stale-revision rejection and disposal contain transport
failure. Stop returns the final frame before releasing its owner.

These executable development host routes do not promote normal Run/Debug or
introduce UI controls on their own. Existing XRoar settings, OEM verification,
layout and separate window remain unchanged. The presentation binding below
supplies explicit session activity in place of PID inference and reuses the
shared bottom-panel system. Real browser/native-window journeys, rebuilt WASM
and the ordinary gate remain separate adoption checks. Evidence is in
[the dated host receipt](../evidence/dragon32-debug-host-20261018.json).

## Development debugger presentation and native bridge — 18 October 2026

The explicit original-development route may bind the existing BASIC breakpoint,
Debug, Continue and Stop controls to its trusted fresh-build controller. This does
not register a normal runtime profile or relax exact OEM firmware admission. The
shared BASIC presenter accepts explicit activity and capabilities; it never uses
a fabricated process ID. Development stop/revision counters remain decimal text.

Native controls remain in the main workbench; the separate emulator window owns
the sole frame scheduler and `DevelopmentHostConnection`. Main-only, session/build-
bound typed actions are relayed to that window. Its publication command reads
authoritative Rust inspection and emits only the bounded inspection, without
framebuffer/audio/media bytes. Raw debug commands remain restricted to the owned
window. The window receives listen/unlisten permission for the bridge, with no
event emission or new filesystem/process/dialog authority. Closure, identity
changes, missing readiness and five-second command timeouts release ownership;
late replies cannot replace the active project or stop.

Native Build and mount selects this debug owner only after the user has explicitly
chosen a BASIC breakpoint; ordinary mounting retains its existing behaviour. In
the browser, the existing explicit development mount exposes the existing shell's
conditional Debug bottom tab. Selecting a breakpoint and Debug replaces the owned
development monitor with a fresh source-bound debug session. Both surfaces retain
the physical guest CLOAD journey: after the prompt returns, Debug queues guest RUN and
Continue atomically, and only the existing frame scheduler advances execution.
XRoar Play/Run and OEM debugging remain unchanged. No debugger controls are added
to the native emulator window.

The adopted browser binding rejects every original-firmware start and mount route
while a development draft or debug owner is active, including Computer setup and
the Play ROM dialogue. A stopped source location selects the programme tab before
focusing its range. The advertised F5 action uses the existing availability and
modal guards, ignores key repeat, and is kept out of the guest keyboard channel.
It adds no runtime profile, layout, debugger action or execution scheduler.


## Development debug prompt and physical-input lifecycle — 18 October 2026

The controller observes the published line-input hook `$0182` with published
current-line `$0068–69=$FFFF` only after a programme statement hook has been observed. It
returns the existing version-1 `ready` status with the neutral message “Programme
returned to BASIC prompt”; END, fall-through and BASIC errors can all reach this
boundary. This is not a success classification or a private firmware-address
heuristic. The guest hook remains unmodified and executes when the existing
frame scheduler resumes prompt input. Boot/CLOAD and programme INPUT do not
satisfy this completion condition. A later explicit Debug action queues RUN again.

Accepted physical Key, Type and Release requests retain the current search or
pause, accumulated search budget and hook-resume state. They advance the owned
revision so older control requests cannot act. Thus canvas blur after F5 cannot
cancel the search, and guest INPUT receives physical keyboard replies while the
same scheduler searches. Reset, media replacement and source/build identity
changes retain their revocation boundaries. Cancel and failed searches remain
paused; input alone cannot restart them or clear failure. No new action, wire
state, layout, scheduler or normal runtime profile is introduced.


## Gated normal original-runtime dispatch — 18 October 2026

Normal original BASIC launch has an additive explicit backend contract, separate
from existing XRoar settings and project adapters. Its admission reads only the
compiled original 16 KiB image and compiled manifest. The reviewed promotion
identity is `eightbit-dragon32-basic-1`, with manifest status `qualified`, schema 1,
Apache-2.0 provenance and an exact image SHA-256. The current `0.1-dev` identity
and `development` status fail closed. No request, project, external source map,
firmware path or environment variable can provide or override this admission.
Changing the reviewed manifest/build identity is a later promotion step after
fit and qualification, not a user-facing switch or an automatic inference from
image size. Existing source/symbol/image provenance checks remain mandatory.

An explicit admitted Run/Debug action builds the current source, retains its
immutable token map/cassette and uses the existing original session owner and
separate native window. The portable owner boots the guest, supplies physical
CLOAD input, waits for a bounded public prompt boundary, compares every loaded
programme byte with the fresh build and only then supplies physical RUN input.
Debug continues through the existing breakpoint controller. This sequence may
not inject RAM or interpret BASIC. Failure, timeout, cancellation and replacement
release or pause the same owner; a stale build is never launched. Project opening
is inert. Initial normal dispatch supports the same Dragon 32 BASIC-only build
scope already implemented by development preparation; mixed/machine-code and
other targets remain on their existing explicit routes.

The build manifest gains an optional versioned `originalRuntime` evidence object
containing backend, target, profile, bundled firmware digest and core version.
`runtimeProfile` names that original profile and `tools.xroar` stays absent/null.
Existing manifests omit this field; XRoar evidence and all supported project
schemas remain unchanged. Evidence comes from the in-process admitted profile,
not untrusted serialised claims. No fabricated XRoar version, PID or OEM signature
exception is permitted. Browser/native callers use the same admission, fresh
build rules and execution owner. No new workbench layout, default backend or
visible capability is enabled while the admission gate is closed.


## Qualification-driven original runtime selection — 19 October 2026

The existing emulator selection and Run/Debug controls may expose Original BASIC
only when the portable Rust admission returns its compiled qualified profile.
The read-only capability carries that exact profile or null; absent, malformed
or unavailable capability leaves XRoar as the default. A browser or WebView
cannot supply qualification. Launch always repeats authoritative Rust admission.
The choice is application-session state, with no project/settings migration.
Other targets, external programmes and mixed/assembly projects retain their
existing routes. Development starts remain explicit and labelled.

Normal Run owns a real session/build identity without a debugger or fabricated
process ID. Its separate native window remains the only execution scheduler;
the existing main-window bridge admits only identity-bound state/focus/close
for a Run owner and carries null debug inspection. Debug retains the existing
inspection and Continue/Cancel protocol. Restart or a later Debug action creates
a fresh build/owner; Reset inside the guest window never reruns an old build.
Browser owners use the same current-source digests and existing console scheduler.
Source/project replacement, Stop, unmount and failed setup release the owner.

The repository may carry an optional `families/dragon/firmware/basic/qualification.json`
only after the documented functional, fit and provenance review. It contains exactly
`schemaVersion: 1`, `id: "eightbit-dragon32-basic-1"`, `status: "qualified"`,
`licence: "Apache-2.0"`, `bytes: 16384`, `romSha256`, `symbolsSha256` and the complete
`sourceSha256` map from the exact rebuilt image. The builder emits the qualified
manifest identity only when this bounded regular-file record matches every bound
field; without the record it emits the existing development manifest unchanged.
The source packer independently verifies the same record and every actual input.
An edited source, image, symbol dump or record fails closed. This record documents
a reviewed image; it is not evidence generated automatically from size or test
counts. No qualification record is created by this integration change. Test-only
records cannot override compiled admission or select `basic-complete/` as firmware.

Normal Restart waits for the owned native window's bounded close acknowledgement;
Rust releases that exact owner before sending the event. Browser normal startup
finishes its initial worker request before mounting the existing frame scheduler.
Stop, late responses and failed startup cannot publish over a newer owner. No
new worker scheduler, fake debugger inspection or host process identifier is used.

XRoar-specific window capture and rebuild-on-save controls are omitted while
Original BASIC is selected. Source changes revoke its build/session; the next
explicit Run/Debug builds again. The native updater also treats this owned
session as active work. Existing XRoar controls and defaults remain unchanged.

The curated public browser source excludes the complete-candidate integration
target `tests/basic_complete.rs`, its `tests/fixtures/complete-basic/` tree and
`firmware/basic-complete/`. These remain maintained repository qualification
inputs, outside the shipped runtime and firmware-bundling exception. The archive
retains the production BASIC tests and their normal 16 KiB inputs.

## Browser normal-runtime identity handshake — 19 October 2026

The authoring capability and a later emulator worker may fetch fixed asset URLs
at different times. Normal original Run/Debug therefore binds the selected
compiled profile to the worker before providing source or constructing a guest.
The internal worker handshake carries a request ID, session ID and the existing
strict schema-1 `OriginalRuntimeProfile`. The worker compares every profile field
with its own compiled Rust capability, acknowledges that exact identity and fixes
it to the session. Capability inspection and subsequent construction share one
initialised module. The host independently validates the acknowledgement; missing,
malformed or mismatched identity requires reloading the studio and releases the
worker. Cancellation, timeout and stale replies cannot proceed to construction.

Older workers reject the handshake before receiving programme source. New workers
reject a normal source launch without the handshake. Existing monitor and explicit
development payloads are unchanged. No project/source schema, UI layout, backend
default, qualification authority or Rust admission rule changes: the handshake
checks that the selected and executing compiled identities agree, and the Rust
factory still performs authoritative admission and fresh-build validation.

## Numeric rounding and cassette load ownership — 19 October 2026

The complete test candidate now composes SQR from the guest LOG and EXP kernels,
with an intervening multiplication by one half. This replaces its earlier Newton
iteration: the reference's finite rounding results, rather than a more accurate
real-number approximation, define compatibility here. All 1,276 retained scalar
observations match, including five mantissa boundaries at every nonzero binary
exponent. The private path uses at most 53 stack bytes and 60,025 E cycles in that
corpus. Zero, negative-domain failure, first arithmetic failure, caller ownership
and subsequent command recovery remain tested. No host evaluator is introduced.
Unused two/pi/quarter-pi constants are removed from the private arithmetic
dictionary; named private ordinals bind the remaining streams. Published tokens,
function tables and numeric representation remain unchanged.

Tokenised CLOAD now streams into the public `$7E` destination, superseding the
staged-block description earlier in this decision. Its ordinary pre-read check is
`destination + 318 <= STACKTOP`, including an empty EOF. A successful data block
advances the cursor; a checksum failure leaves it at that block's start. I/O
failure selects IO and resets the programme's first link. Memory exhaustion
selects OM, retains already received bytes and does not manufacture a tape
failure status. The 79 retained reference observations include these distinctions
and loads reached through RUN, FOR and GOSUB.

NEW has already invalidated control records before BASIC data input. The loader
reclaims those records only when its 23-byte active native continuation exactly
abuts the executor's recorded statement and control boundaries. It copies that
continuation backwards to the established control base and updates S/U and the
existing private boundary words. A nested public caller retains its own frame.
An additional live-stack guard protects such callers without narrowing ordinary
BASIC admission. No persistent RAM or executable overlay is added.

After synchronisation, block input releases saved Y/U before writing payload.
Byte input saves/restores the caller's condition register and defers IRQ/FIRQ for
one byte; NMI remains available. One nested IRQ/FIRQ/NMI sequence consumes at most
37 bytes below the compact block reader, leaving two bytes above the largest
admitted payload. Ordinary IRQ input succeeds; a deliberately delayed three-
handler sequence may fail checksum but must preserve the payload bound and unwind.
This establishes the tested returning-handler boundary, not arbitrary recursive
or non-returning user handlers. All 256 byte inputs retain their decoded values;
0-, 1- and 255-byte output blocks retain identical edge timing and call duration.

These changes remain in `basic-complete/` on its explicit test bus. They do not
qualify or distribute an image larger than 16 KiB. Fresh exact-image normal
Run/Debug and packaged browser/native acceptance remain prerequisites after fit.
