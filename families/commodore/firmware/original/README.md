# Original C64 guest BASIC, editor and KERNAL 0.5

Original Apache-2.0 guest 6510 assembly and independently drawn characters under
ADR 0062. BASIC statements/functions, values, strings/arrays/collection, control
flow, DATA/INPUT/GET, DEF FN, STOP/CONT, the screen editor and published KERNAL
calls execute in the guest. There is no host BASIC evaluator, copied firmware,
recovered glyph data or reference implementation. The development identity has
explicit compatibility limits; it is not universal C64 software/silicon parity.

Build with explicitly selected, separately installed ca65/ld65:

```sh
python3 scripts/build-c64-original.py --ca65 /absolute/ca65 --ld65 /absolute/ld65
python3 scripts/build-c64-original.py --ca65 /absolute/ca65 --ld65 /absolute/ld65 --check
```

The command prints a source/tool/image-bound JSON manifest. The single complete
container is published without replacement at
`.local/c64-original/<manifest.imageSha256>.c64rom`. `--output` selects an explicit
file; a different existing destination is rejected. Exact hashed input bytes are
staged in a private directory. External tools have bounded output/time and owned
process groups. No tool is downloaded or bundled. The builder requires POSIX;
actual acceptance is macOS Apple Silicon, with Linux unverified.

```sh
EIGHTBIT_C64_ORIGINAL=/absolute/built-image.c64rom \
  EIGHTBIT_C64_ORIGINAL_SHA256=<manifest.imageSha256> \
  node scripts/c64-stage2-parity.mjs --original
```

This explicitly compares native and compiled WebAssembly boot, physical input,
clock, KERNAL calls, user NMI/RTI, page restoration and language/error/lifecycle
and numerical observations. The language journey enters every programme through physical matrix
keys. Separate assembly and malformed/full-memory fixtures explicitly initialise
user RAM; they are labelled separately. No supplied ROM is required. Optional
native `EIGHTBIT_C64_ORIGINAL_CAPTURE_DIR` must be an absolute new directory and
writes an original illustrative PPM; it is not an application preference.

The image contains three 8 KiB BASIC pages, fixed 8 KiB KERNAL and 4 KiB original
characters. Nine immutable gates preserve call/interrupt/error paging. Every one
of the 65,536 physical guest RAM bytes retains its public role. Values and
expression/loop/subroutine temporaries use public workspace and the actual CPU
stack. There is no hidden host variable store or reserved user RAM.

`runtime.c64rom` and `runtime-manifest.json` are the original bundling exception.
The ordinary builder tests bind their image/source/builder hashes. Rebuilds use
separately installed ca65/ld65; no external assembler or emulator is bundled.
The native/browser Studio adapters explicitly select this image and the same
family-owned session. Standard PRGs load at $0801 and guest RUN is typed through
the matrix. Reset cold-boots; Restart builds fresh source. VICE and chips remain
the ordinary desktop and initial browser defaults respectively.

Numeric values use the public five-byte format and 32-bit significands; integers
use the public signed two-byte form. Exact integer algorithms handle arithmetic,
decimal/display conversion and SQR. Original bounded series/range reductions
handle transcendental functions with named tolerances. RND has an original
xorshift sequence. Decimal parsing is bounded to 80 digits; guest stack guards
are conservative and do not reproduce historical nesting capacity. TI$ requires
six decimal digits; undocumented nondigit conversions are unsupported.

The editor has an 80-character logical input limit, while existing linked BASIC
records may contain up to 250 body bytes. It implements public cursor, screen,
colour, quote, insert/delete, repeat, shift/control/Commodore and character-set
behaviour. Independent glyphs preserve symbol roles rather than ROM pixel shapes.
Keyboard/screen channels are implemented; absent devices return guest errors.
There is no attached drive, host filesystem, cartridge, NTSC target or debugger.

The specification owns detailed compatibility/provenance and acceptance:
[original C64 contract](../../../../docs/c64-original-specification.md).
The compiled native/WASM corpus and Studio session parity are separate gates.
A private-page image cannot be used as a stock physical replacement ROM or as a
complete VICE ROM set. Physical hardware, Linux/Windows acceptance, signing,
publication and default promotion remain separate from local Mac/browser checks.
