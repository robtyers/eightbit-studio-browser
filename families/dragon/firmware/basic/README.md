# Original BASIC development ROM

Apache-2.0 original 6809 assembly. This is unfinished firmware, not a qualified
Dragon BASIC replacement. The guest ROM boots the digital board, sets up the
published workspace, vectors/hooks and console I/O, tokenises the full existing
keyword contract and inserts/replaces/deletes linked programme lines. The
development interpreter now also implements computed ON branches, DEF FN/USR,
the named numeric/string/device functions, colour CLS and semigraphics, five
PMODEs, PCLS/PSET/PRESET/PPOINT, SCREEN/PCLEAR/COLOR, LINE with outline/filled
rectangles, CIRCLE ellipses/arcs, PAINT, PCOPY, INPUT/LINE INPUT, PRINT@/TAB, explicit screen/printer PRINT, RUN from a line or filename expression, inclusive LIST/LLIST ranges, DEL, SOUND and explicit MOTOR/AUDIO switches. MID$ assignment supports scalar/array destinations, programme-literal detachment,
forward self-overlap and the observed zero-length copy quirk. Low-level cassette entry
points sample and generate digital PIA signals, align bytes, frame blocks and
check checksums, timeouts and input RAM bounds. Named SKIPF, binary CLOADM and tokenised/ASCII
CLOAD share those checked input routines. Tokenised/ASCII CSAVE and binary CSAVEM
share the named-file frame and checked output framing. These are guest 6809 routines, including
an original arithmetic microprogram decoder used to keep the image within 16 KB.

The current image is `187d66f60d9f5c227a8a8eb74e3f11ca5d24a887430975496edca735c05963a9`:
exactly 16,384 bytes, with 21 unused bytes. Its paired joystick scan preserves
all host coordinates 0–63 with the corrected Board comparator. The previous
181-test receipt applies to the earlier image; the current Board/native/WASM
checks are recorded in the dated hardware evidence linked from the BASIC contract. DRAW, PLAY and GET/PUT execute
in that image. PRINT lists and variable-name whitespace now follow the retained
21-case synthetic reference fixture. Empty printer lines and busy-port recovery
are covered by guest and board tests. Shared bounded text output supports STR$
and future editor text; LIST hides structural ELSE/comment colons correctly.
INPUT retry restores its saved destination type, and keyboard mapping retains
shifted editing keys and alpha-lock. Public cassette count $79 is separate from
temporary formatting capacity. GET/PUT retains the native byte-copy path, packed transfers,
operation masks and bounded partial-transfer behaviour. The current checkpoint
and remaining work are maintained in the [BASIC contract](../../../../docs/dragon32-basic-specification.md);
the later sections here retain earlier implementation history.

Strings retain the legacy descriptors and descending heap, including overlapping
substring roots. FN records retain the observed name flag, body/parameter
pointers and restored parameter value. Shifted input and alpha-lock case use the
physical matrix; typed development BASIC input is bounded to 251 characters, with an empty submission sending Enter.
The shared Rust machine/session has an explicit `BasicDevelopment` profile; the
monitor remains its default.

This is unfinished BASIC. Remaining PRINT/device forms,
interactive editing/renumbering,
data files and file devices, exact errors and full ABI qualification
remain required work. Original RND code maintains the observed five-byte seed layout and 32-bit
recurrence and product-derived return guard. Selected fractional/large-bound and
halfway-rounding results now match the lawful profile. Wider numeric rounding/presentation,
transcendental extremes, complete memory-interface behaviour and physical/audio
acceptance remain unqualified. Web and desktop offer an explicit BASIC development console alongside the
monitor and XRoar. Explicit development Build and mount, CAS/WAV attachment and
checked guest cassette capture are implemented. Normal Build/Run/Debug adoption
still requires the complete fitting image; the development image is not a
qualified BASIC backend. The complete-vocabulary source and its separate
qualification are maintained in [basic-complete](../basic-complete/README.md).

Internal routines are packed between the unchanged fixed entry stubs. Shared
initialisation, expression/operator/function dispatch, checked conversions,
constant/unary/word microprograms, shared native failure islands and return paths
reduce duplication. The earlier pass recovered 263 bytes in pre-existing modules, then added the
239-byte MID$ module. The earlier native expression rewrite shares a packed
operand/collector descriptor and typed assignment across LET, READ and INPUT,
removes the duplicate unpacked expression save/restore routines and preserves U
through floating swaps. It recovers a further 59 bytes before the five-byte input workspace fix (54 net). A 31-operand power chain
uses 176 fewer stack bytes in the selected probe; both success and failure restore
the stack and root chain. Numeric operands do not enter the root chain.
Hardware initialisation retains its PIA write sequence in an original data table. Floating multiply
uses sixteen native 6809 MUL operations and retains the complete 64-bit product
for the observed nearest/halfway-away-from-zero guard/sticky rounding. Signal-timed sound/tape
bodies and the copied RAM scanner are excluded from branch shortening.

The earlier CIRCLE implementation used an original integer Q12 quarter-wave table returning Q14 values. Axes below 256 used 128
clipped Bresenham chords; larger axes retain 2,048 angle samples. Lawful selected
observations establish integer-truncated radii, aspect quantisation to 1/256 and
truncated vertical axes. The 60-by-35 ellipse from `CIRCLE(128,96),60,1,.6`
used 1,108,938 guest cycles in the earlier measurement, versus 76,858,994 before
that increment (about 69×).
The current implementation supersedes that approximation with 64 chords and
clamped physical endpoints; the BASIC contract records the 86 reference canvases
and 428,900-cycle ellipse. The guest performance gate permits at most 1.5 million cycles. Exhaustive angle,
clipping, public-workspace and shape tests supplement selected reference pixels;
complete OEM pixel/workspace equivalence remains unqualified. Compact `FNF(...)`
names and ELSE-terminated commands now have focused coverage.

The line editor keeps its overflow/cancellation flag at $03FA in the existing
private area. It preserves public $D8–$E9 across valid, overflowing, cancelled
and subsequent input; no extra RAM area is reserved.

The preceding receipt recorded 263 free bytes and a largest gap of 54 bytes:
129 bytes recovered from the 17 October starting image. This remains insufficient
for the outstanding command families. Zero-length MID$ assignment deliberately retains the bounded 256-byte forward
copy observed in the lawful reference, including writes beyond the string. Guest
ROM write rejection is unchanged. No extra bank, decompression area,
host evaluator or permanent user-memory reservation is used.

Public interfaces are recorded in
[the contract](../../../../docs/dragon32-basic-specification.md) and ADR 0057.
No OEM instructions, recovered BASIC source or third-party emulator implementation
are implementation inputs. User-approved black-box tests of an exact, lawfully
provisioned ROM supply interface and output-signal observations only. Retained source/image/symbols/manifest are development source artefacts,
not disposable build caches. The receipt identifies every source input and
labels the image as development.

```sh
python3 scripts/build-dragon-basic.py --lwasm /explicit/path/to/lwasm
python3 scripts/build-dragon-basic.py --lwasm /explicit/path/to/lwasm --check
cargo test -p eightbit-dragon-emulator basic_rom
python3 scripts/dragon32-original-reference.py --xroar /explicit/path/to/xroar --execution-probe
python3 scripts/dragon32-original-reference.py --xroar /explicit/path/to/xroar --abi-probe
python3 scripts/dragon32-original-reference.py --xroar /explicit/path/to/xroar --graphics-probe
```

LWASM is separately installed; it is not downloaded or bundled by these scripts.
The explicitly labelled development console is for local qualification. Guest
tests must qualify the remaining interfaces and failure paths before promoting
this image to the normal Build/Run/Debug runtime or claiming legacy compatibility.

The explicit XRoar probe uses our original ROM and synthetic guest code only.
It verifies boot, scalar layout and arithmetic/PRINT independently, retains a
compact observation and deletes its temporary RAM snapshot. It neither loads an
OEM ROM nor proves OEM compatibility. No reference implementation is copied.
The graphics probe checks selected ellipse pixels in separately installed XRoar.
It allows 15 emulated seconds and retains a 30-second host deadline. The
separate guest-cycle test measures performance; these gates do not establish
complete OEM timing compatibility.
Its synthetic keyboard fixture replaces only our keyboard body and reset jump;
it no longer requires unused padding in the distributed ROM.

The separate opt-in `scripts/dragon32-lawful-abi.py --configured` gate reads the
saved macOS ROM selection, validates the existing exact Dragon 32 1982 SHA-256,
runs original synthetic programmes and reports structured data records. Temporary
RAM snapshots and logs are deleted. It never reports ROM instructions, firmware
bytes or implementation-specific return targets. `-type` intercepts ROM input
calls, so those comparisons do not qualify keyboard hardware. Ignored native
Rust gates separately verify lawful-ROM boot/matrix input and measure only DAC
and cassette signal timings on our original CPU/board. Neither gate runs in ordinary CI or bundles that firmware.

The native interval collector retains aliases/overlap offsets without temporary
descriptor links. The latest pass recovers 98 bytes before explicit PRINT devices
and reset recovery. Lookup and collection share complete array-record validation;
native arithmetic loops, a pointer-bounded swap, VAL allocation and static branch
shortening introduce no decoder or permanent RAM allocation.

PAINT now occupies 310 bytes. Its two-byte stack seeds propagate a horizontal
border-free row only when some pixel changes colour. Existing paint remains
horizontally traversable; wholly painted rows do not bridge vertical gaps. This
fixes the reported border-gap mismatch. All 28 retained synthetic graphics planes
match the lawful reference; the earlier mismatch receipt remains historical.
Full PAINT workspace/timing/errors and historical software remain unqualified.

`PRINT#0` and `PRINT#-2` select screen/printer output and restore the previous
device after success or failure. TAB/comma spacing uses the active column;
printer carriage return clears $9C. Corrected BUSY polarity is exercised through
PIA signals and independently in XRoar. Warm reset selects screen output even
while the printer is busy. No host printer or file output service is added.

`python3 scripts/dragon32-original-reference.py --xroar /explicit/path/to/xroar --paint-probe`
checks selected fill/border/exterior pixels using only our own ROM in XRoar.

The additional `--paint-gap-probe` and `--printer-probe` options independently
exercise the corrected fill and original parallel-port output in XRoar. Each is
a separate bounded invocation; the printer probe attaches only a temporary
original-output file and deletes it with its RAM/log files.


The 16 October native pass shares quote/DATA/comment transitions between the
lexer and listing, and puts first-letter validation and U preservation inside
the variable parser. Colour packing uses native MUL in both mode families.
The arithmetic decoder saves its continuation once per operation; its original
opcode vocabulary is unchanged. Native subtraction uses the final borrow to
normalise the difference, and unsigned conversion enters the existing integer
normaliser directly. This also fixes TIMER/unsigned cursor loss above 32767.
The decimal unsigned printer suppresses leading zeroes before its digit loop.

`TIMER=expression` validates an unsigned 16-bit value and statement ending before
clearing the upper byte at $112 and storing the lower word at $113. Invalid input
preserves the live counter. Guest console programme line zero is accepted,
including RUN/branch/LIST/DEL; missing line numbers still fail. The IDE source
and project schema retain their existing line limits. The net ROM recovery after
these additions is 257 bytes; the remaining 305 bytes do not fit the outstanding
feature set. No new parser decoder or permanent RAM area is introduced.

`--native-compaction-probe` in the original-ROM reference script independently
checks line zero, its branch, unsigned TIMER and invalid-assignment recovery.
The lawful harness separately observes packed GET arrays, which are not yet
implemented/qualified: G changes a stored extent to zero and the named-array
operation creates a same-name scalar. These facts must inform subsequent array,
collector and GET/PUT work rather than relaxing ordinary malformed-input guards.


Direct-command tokenisation now compacts in place in the legacy $02DD–$03D8
input area (251 characters plus NUL). This removes the old $0200 overlap with
the cassette buffer and guards its input preamble/private neighbours. It adds
no reserved RAM; high-level cassette/file and complete ABI qualification remain open.

The subsequent native failure-scope rewrite restores nested parser/root/device
state and preserves the legacy machine-code control stack. Its net recovery is
111 bytes before two new cassette modules (291 bytes). Development SKIPF and
binary CLOADM now consume checked physical tape blocks, support named searches
and unsigned relocation, and stop the motor on I/O failure. CLOADM latches its
entry from the checked header and streams directly into RAM. Bytes already read,
including a checksum-failing block, remain after an I/O error. CLOADM/CSAVEM
suffix tokenisation also matches the IDE. Negative offsets reject; 65535 wraps.
Default EXEC uses the public pointer, including a legitimate loaded address zero.

Those earlier additions did not complete the requested replacement: ASCII/data-file I/O,
ASCII cassette output, PRINT/editor forms, GET/PUT/DRAW/PLAY,
complete compatibility and normal project integration remain open. The exact
current contract and measured space limits are in the BASIC specification.
`--cassette-probe` in the original reference script checks named skip, binary
load and default EXEC in XRoar with fast tape traps disabled, using only original
synthetic files. Ordinary board tests additionally exercise the physical keyboard,
PIA, bounded transport, loaded code, missing-signal recovery and reset together.

The later expression pass saves caller Y/U once and reserves operand/root records
only for pending operators. The 31-operand power case fits above STREND=$6D80
without writing below $6E00; failure and collector cases remain covered.
Tokenised CLOAD stages checked blocks, enforces programme/stack bounds and
rebuilds links from bounded sequential bodies. Failure leaves valid NEW state;
loading during a running programme stops overwritten execution. Direct
`CLOAD"name"` followed by RUN on the next command line works. CLOAD discards
its command-line tail, matching the approved lawful-ROM observations. Exact OEM partial-load/error/workspace behaviour remains
unqualified. The main execution loop checks failure carry before resetting
control state. Relative to the 305-byte baseline, pre-existing modules recover
191 bytes and the three new cassette modules occupy 465 bytes, leaving 31 free.


The native grammar/output pass recovers 194 bytes in previous modules, adds a
five-byte shared opening helper and 188 bytes of writer code, leaving 32 bytes free in the exact 16 KiB image. Native
required/optional delimiter helpers retain cursor/flag semantics; the optional
helper preserves Z across LEAX. CSAVEM writes a checked inclusive RAM range and
unsigned execution address; CSAVE validates and streams the linked programme.
The DAC writer balances both halves and compensates for inter-byte call overhead.
Pulse-capture tests, the independent media parser and guest reloads cover empty,
single- and multi-block files and rejection/recovery before motor-on. The separate
XRoar full output-capture probe initially rejected the documented CUE trailer;
the following pass validates that metadata and all six exact output blocks.
Both emulated-WAV save/decode gates pass with exact original file contents.
See the BASIC specification for exact supported arguments, volatile workspace
and remaining ABI/project-integration limitations.


The shared-walker/numeric follow-on has 179 free ROM bytes (largest gap 118),
net 147 recovered from the 32-byte-free image. Validation and relinking share a
bounded sequential walker; statement bodies rely on their existing register
boundary. Native RND preserves the observed product scratch as well as its seed.
Radix literals/VAL return unsigned values and decimal scanning accepts internal
spaces. Fresh lawful observations confirm that negative HEX$ is invalid; its
rejection is retained. CSAVEM now reads the complete guest bus, with a remaining
count that also handles the full 65,536-byte range. Cassette output masks IRQ/FIRQ
for the complete block, then restores the caller's masks; a byte-boundary IRQ
previously caused checksum-valid but incorrect 9.6 kHz WAV recovery. Complete
language/graphics/music/file/ABI and normal project integration remain open.


The preceding native compaction and ASCII cassette pass keeps the exact 16 KiB image
and all fixed ROM/RAM interfaces. Existing modules are 90 bytes smaller in total,
including their new cassette/RUN hooks; two new original ASCII modules occupy
207 bytes, leaving **62 free bytes**. Native checked branches, ordinary local
jump islands and shared numeric helpers introduce no parser VM, host evaluator,
extra bank, executable overlay or permanent RAM allocation.

The development ROM now supports `CSAVE"name",A`, ASCII `CLOAD` and `RUN` with a
filename string expression. Output streams LIST through the legacy cassette
buffer with gapped physical blocks. Input assembles at most 251 bytes in the
legacy input buffer, handles CR/LF and replacement lines, and discards an
unterminated EOF tail as observed in the lawful ROM. Malformed input restores
NEW state and stops the motor; a failed RUN load never starts the old programme.
Original-RAM layout, the workbench, monitor default and explicit XRoar route are
unchanged. That guest-only pass did not add ASCII cassette import/WAV conversion
to the project-media API; the follow-on below does. PRINT/editor forms, GET/PUT/DRAW/PLAY,
data-device files, full ABI/CPU/hardware qualification and normal original-ROM
project integration remain required work. See the BASIC specification and the
new dated verification record; the full replacement is still incomplete.


The preceding native arithmetic failure scan recovered 12 bytes and left 74 free.
It removes its former per-operation stack flag, skips word operands on failure
and keeps copy/comparison and checked arithmetic continuations distinct. The
exact source/image/symbol identity is in manifest.json. This small reduction is
insufficient for the outstanding full-ROM scope. Shared browser/desktop ASCII
CAS/WAV recovery and CLI encoding are now implemented outside the ROM; see the
[BASIC contract](../../../../docs/dragon32-basic-specification.md) for strict
bounds, project limitations, remaining scope and verification.


The next native pass recovers a net 123 bytes from that 74-byte checkpoint.
Expression operands use shared typed load/store routines without changing their
packed five-byte value or eight-byte root frame. Active collection visits FAC/ARG
pointer words directly with a single stack mode byte; ordinary collection still
excludes dead registers. No root descriptors or additional private RAM are
reserved. Checked native branches, proven carry preservation and common failure
unwinding remove further duplicated code.

CIRCLE reflects an original `ceil(4096*sin(n*pi/64))` table, multiplies its bounded
byte slopes with native MUL and returns Q14 values. All 4,097 tested angles remain
within five units, with exact cardinals; total kernel/conversion cost falls from
2,714,902 to 2,236,958 guest cycles. Selected graphics regressions remain separate
from complete OEM pixel qualification. RND truncates a positive bound before
scaling; values between zero and one return integer one. The public PLAY defaults
now include the observed V15 DAC bytes $BA/$42. PLAY itself remains unimplemented.
The 16 KiB image and fixed public stubs remain development artefacts. Full language,
graphics/music/files/ABI qualification and normal project execution are unfinished.


The preceding RND-qualified image `aa29d0ea03a350045fcc401be8f20f7793cdc306a747f33927ae8254fe089e98`
has 228 free bytes, largest gap 68: 31 bytes recovered from the preceding image.
Floating multiplication and RND share one original native 64-bit product routine.
It initialises every byte before use, preserves both operands and writes only its
caller-owned eight-byte destination. RND uses product bits 40–47 as its guard;
its public seed and low-product scratch remain unchanged. The shared arithmetic
rounds exact halfway magnitudes away from zero, including negative results.
Temporary guest-stack storage introduces no new persistent RAM allocation.
This remains a development ROM with the outstanding scope listed above.


The native add/subtract pass now uses one original two’s-complement byte loop,
including its guard and complete carry chain. The image remains 16,384 bytes:
`83ec4d1020ec588bd6c47751ec067df314737c529f98dc9139cfbb3a0a71d111`,
238 free bytes, largest gap 78 (ten bytes recovered). An independent integer
oracle covers 8,192 operations; 64 selected lawful addition/subtraction facts
also agree. No persistent RAM, parser VM, host evaluator or public-interface
change is introduced. This is incremental compaction and does not implement
the outstanding command families or qualify the complete replacement.


The current decimal/token/diagnostic image is
`5f75263ad4270012091c4aaa370a3b4ab9d3ea4a5b6ce4040ce37a69ab599495`:
16,384 bytes, 134 free, largest gap 46. Two native parser reductions recover
33 bytes; selected guest diagnostics and their boundaries consume 137 bytes.
Five error prefixes (SN/TM/FC/OV//0), programme-line reporting and prompt recovery
replace the former generic message for the reviewed paths. $03FD is private
diagnostic state; pure numeric calls still use carry without a parser frame.
The BASIC specification records exact limits. Substantial compaction and the
outstanding feature/qualification/integration work remain unfinished.


The 17 October native pass shares optional USR digit and ASCII sign parsing,
constant addresses, array size products and PIA mux selection. Biased native
line/circle/cassette frames keep fields within the 6809's shorter indexed range.
Keyboard ASCII ranges and semigraphics shifts replace redundant table data.
The public token tables and all fixed entry points are retained.

Numeric output now converts the complete binary value through native packed BCD,
then rounds once to nine decimal significant digits. Forty integer and fifty
fractional decimal positions cover the normalised five-byte range. The conversion
uses a checked 59-byte guest stack frame, with no bank, overlay, new decoder or
permanent RAM reservation. The exact packed value 999999999.75 now prints as
`1E+09`, matching the lawful observation. The independent decimal oracle covers
all 255 non-zero exponents, both signs and three mantissas (1,530 cases):
`python3 scripts/generate-dragon-decimal-fixtures.py --check`.
Remaining numeric-input, error, ABI and command qualification still applies.
