# Original Spectrum firmware

This is **original development source**, under ADRs 0058/0059, with the local software
acceptance recorded in `docs/spectrum48-original-specification.md`. Native Spectrum 48K/128K/original +2
Run/Debug uses it when no external emulator is selected; saved MAME settings are
preserved. Browser Play uses this runtime for Spectrum 48K/128K/original +2, retaining an explicit
MAME/supplied-ROM choice. Exact historical compatibility retains the limits
recorded in that specification; the image keeps its development identity.
The guest editor, arithmetic, variable, control-flow, display and cassette routines
are assembled by the original Rust assembler in the emulator crate's
build script. Build failure cannot publish a new runtime.
The image, source digest, symbols and development manifest are generated in Cargo's
private output folder; no third-party assembler or firmware is involved.

The CPU sees 16 KiB ROM and a 48 KiB RAM window. The 48K model has flat RAM;
128K and original +2 select eight physical 16 KiB banks through the public
`$7FFD` paging latch, including the alternate screen and irreversible-until-reset
paging lock. The same firmware implements both public ROM modes. Implementation storage has
8 KiB common code and up to 128 private 8 KiB pages. Common gates at `$01F0` and
`$01F8` access port `$FDFD`; the board ignores page selection outside those gates.
All public entry stubs and the original character designs must remain consistent
across pages. This container is not subject to a 16 KiB total-size requirement.

The gated `$FDFB` sideband distinguishes literal IDE typing from physical keyword
keys and acknowledges when the guest editor is waiting for another character.
It changes no RAM layout or ordinary programme I/O. Tokenisation, syntax/type
checking, editing and BASIC execution still run entirely as Z80 instructions.

`common.asm` owns common public stubs and hardware-facing routines. Page sources
own interpreter implementation; `tables.asm` and `font.asm` are original shared
data. S48ROM-01/S48BAS-01/02 and actual native/browser acceptance are recorded
separately from public release and physical-hardware testing.

See `docs/spectrum48-original-specification.md` for tested subsets and explicit
remaining work. Arithmetic functions run entirely as guest Z80 instructions;
host arithmetic appears only in independent test oracles. Tape loading samples
EAR pulses and validates flags/parity in guest code. Saving emits physical MIC
pulses, and the BASIC-loader/CODE journey executes those same guest interfaces.
High-level BASIC/array/CODE/SCREEN$ tape operations and native-editor canonical
SAVE have focused guest tests. PCM WAV input and checked standard-speed MIC-to-TAP
recording use the same portable session. Broader cassette, editor and actual
host acceptance remain separate requirements.

The editor supports native keyword/extended/graphics modes, literal IDE typing,
line recall, automatic listing, syntax checking without evaluation and a bounded
long-line viewport. Persistent workspace allocation leaves 512 bytes beneath the
live stack so a failed near-full allocation can still accept a recovery command;
the reclaimed syntax copy uses the smaller live-stack margin. This can reject
allocations earlier than historical firmware without reducing the raw 48 KiB RAM.

The 128K extensions reuse the existing statement, expression, syntax and cassette
file code. Page 24 executes three-voice PLAY; page 25 transfers RAM-disc bytes
through five extra physical banks, with interrupts disabled only while needed.
Every transfer restores the caller bank before touching its stack. The directory
has 32 entries and 1 KiB of overhead; deletion and replacement compact live data.
A new file is staged before its directory commit. NEW preserves files; Reset
clears them. No host BASIC, music, RAM-disc filesystem or memory injection is used.
`$FDF9`/`$FDF7` provide model/paging facts only through the common read gate.
AY accesses outside private firmware gates retain their public port decoding.

Manufacturer references (facts and interfaces, never implementation code):

- [Sinclair 128 introductory manual](https://worldofspectrum.org/ZXSpectrum128Manual/), especially pages 6–14.
- [Spectrum 128 service manual](https://spectrumforeveryone.com/wp-content/uploads/2017/11/ZX-Spectrum-128-Service-Manual.pdf).
- [Amstrad original +2 service manual](https://speccy4ever.speccy.org/doc/plus2sm.pdf).
- [GI AY-3-8910/8912 data manual](https://map.grauw.nl/resources/sound/generalinstrument_ay-3-8910.pdf).

This is a functional Studio environment; exact historical menus, editor presentation,
peripheral serial/MIDI signalling, analogue output and undocumented ROM internals
are not replicated. MIDI music controls are parsed but have no connected peripheral.
PLAY uses the shared frame interrupt for note scheduling. Physical cassette/audio
and historical software qualification require their own observed evidence.
