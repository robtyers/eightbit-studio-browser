# Original ZX80 and ZX81 development toolchain

Status: implementation under ADR 0061, 20 October 2026. Not qualified for a
completed default workflow until the following requirements pass.

## Models and compatibility

The existing UK/PAL ZX81 1K and 16K projects retain their exact schema and P
format. ZX80 uses its original integer BASIC, character set and O image; its
stock 1K configuration is included. The proposed additional 16K ZX80 profile
awaits the user's model confirmation. No NTSC, Timex, ZX80 ROM upgrade or printer
peripheral is implied. Firmware size is not constrained to historical ROM chips;
physical replacement-ROM delivery is not included.

## Requirements

- **ZXM-01:** Reuse the original Z80 CPU/assembler, shared guest arithmetic and
  established build/source-map infrastructure. Preserve Spectrum/Dragon and all
  explicit external-tool routes. No new external runtime or compiler dependency.
- **ZXM-02:** Model RAM/ROM decoding and aliases, keyboard, M1/refresh video,
  inverse characters, interrupts, ZX81 NMI and SLOW/FAST. Guest code owns BASIC
  display structures and output. Do not replace hardware rendering with a host
  BASIC or display-file interpreter.
- **ZXM-03:** Implement the machine's complete published BASIC statements and
  functions, native numbers/variables/arrays/strings, line editing, INPUT,
  control flow, syntax/error recovery and memory exhaustion within stock RAM.
  ZX80 retains signed integer semantics, zero-based arrays and its own USR
  result register; ZX81 retains five-byte numbers, slicing and its own USR ABI.
- **ZXM-04:** Load/record standard cassette signals and bounded WAV input;
  validate/export original O/P images without host injection of a built programme.
  Stock-ROM reference loading and physical loading remain distinct checks.
- **ZXM-05:** Native/browser BASIC and assembly Build, Run and Debug default to
  original implementations when no external tool is selected. Preserve the
  existing UI, project contracts, inert opening and explicit operation authority.
- **ZXM-06:** Bind builds and debugging to exact source, media, model, session and
  stopped bytes. Preserve cancellation, invalidation, reset/stop, stale-response
  rejection, safe memory limits and failed-build recovery.
- **ZXM-07:** Keep original firmware, specification, provenance, Help, source
  packaging and local acceptance evidence current. Run focused tests, native/WASM
  parity, actual UI journeys, the ordinary gate and affected product packages.

Manufacturer references: Sinclair's *ZX81 BASIC Programming* (Steven Vickers,
second edition, 1981), Science of Cambridge's *A Course in BASIC Programming*
(Hugo Davenport, second edition, 1980), original circuit/service documentation
and Zilog UM008011-0816. Implementation code must remain independently authored;
ROM listings and reference-emulator sources are excluded.

## Implementation evidence and remaining scope

The current code includes the shared Z80 refresh observation, separate original
ZX80/ZX81 guest images and a digital board with RAM/ROM aliases, keyboard matrix,
M1/refresh glyph generation, inverse/RAM fonts, refresh interrupt and ZX81 NMI
lines. Original guest code boots and performs keyboard-triggered signal LOAD
and SAVE in both physical RAM sizes. Streaming O/P playback, bounded PCM WAV,
recorded O/P recovery and PCM export have ordinary tests. The current guest
keyboard is a development command surface, not the completed BASIC editor.

ZX80 integer source encoding uses two-byte line headers without hidden floating
numbers; O validation checks pointers, line records and native variable extents.
The assembly exporter uses an original in-place nibble decoder to retain every
machine-code byte despite the original REM newline restriction. Its fixed code
origin is `4050h`; the cassette bootstrap starts at `402bh`. Current binary
budgets are 256 bytes in 1K and 7,800 bytes in 16K. These new ZX80 source APIs are
not yet registered as application targets or defaults.

On 20 October 2026, opt-in exact-hash tests using the locally supplied ROMs
verified boot/video and EAR loading of generated O/P programmes on ZX80 and
ZX81 with both RAM sizes. A further ZX80 check loaded exported code, ran its
bootstrap under the reference ROM and compared all decoded bytes, including
`76h` and the count-adjustment case. Reference SAVE signals were recovered into
validated images and PCM. These checks use our CPU/board and are distinct from
MAME, physical hardware, browser/native workbench or package acceptance.

The complete guest BASIC interpreter, editor/INPUT, native variable and array
behaviour, graphics commands, full error recovery, public ROM calls, SLOW/FAST
execution, source-bound debugger and desktop/browser default integration remain
required. No completed no-configuration workflow is claimed. Existing MAME and
SjASMPlus routes remain effective. See the firmware README for development
limitations; no new public runtime profile has been admitted.

Focused verification (the optional ROM paths are never committed):

```sh
CARGO_INCREMENTAL=0 cargo test -p eightbit-z80-core -p eightbit-sinclair-basic -p eightbit-zx81-core -p eightbit-zx81-emulator -p eightbit-spectrum-core -p eightbit-spectrum-emulator
CARGO_INCREMENTAL=0 cargo clippy -p eightbit-zx81-emulator -p eightbit-zx81-core -p eightbit-z80-core -p eightbit-sinclair-basic --all-targets -- -D warnings
EIGHTBIT_ZX80_ROM=/path/to/zx80.rom EIGHTBIT_ZX81_ROM=/path/to/zx81.rom CARGO_INCREMENTAL=0 cargo test -p eightbit-zx81-emulator --test reference -- --ignored --nocapture
EIGHTBIT_ZX80_ROM=/path/to/zx80.rom CARGO_INCREMENTAL=0 cargo test -p eightbit-zx81-core --test cassette_reference -- --ignored --nocapture
```

The dated partial implementation receipt is
[`zx80-zx81-development-20261020.json`](evidence/zx80-zx81-development-20261020.json).
It records the required scope still outstanding and the observed full-gate
failures. It is not a runtime admission or release receipt.
