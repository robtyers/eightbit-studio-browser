# Original ZX80/ZX81 guest firmware

Implementation under ADR 0061. The current images are hardware/cassette
development builds, not qualified BASIC replacements or application defaults.

All execution and cassette routines are independently authored Z80 source,
assembled by `eightbit-z80-core`. `zx80-abi.asm` and `zx81-abi.asm` supply public
manufacturer addresses and character facts. The builder emits separate images
and manifests, recording image, source, builder and assembler identities.

The low ROM window retains the machine's normal decoding. Common code calls
the narrow gates at `01e0h` to select private 8 KiB implementation pages in the
ROM alias window. Physical RAM stays at 1 or 16 KiB. The guest uses its own RAM
stack; no host BASIC evaluator or hidden guest workspace is provided. Current
guest work includes M1/refresh display, keyboard edge detection, signal LOAD and
signal SAVE. SAVE currently uses the development filename `STUDIO` on ZX81.
The complete language, editor, public ROM entry compatibility and SLOW/FAST
behaviour remain unqualified and must precede default runtime admission.

`zx80-binary-loader.asm` is a separate original cassette bootstrap. It decodes
nibble-encoded code in place at `4050h`. This permits all machine-code bytes in
a stock ZX80 O cassette: the original REM scanner otherwise treats `76h` as
newline. The bootstrap executes without returning to the BASIC line scanner
mid-copy. A one-byte count adjustment prevents its own immediate operands from
containing `76h`. Its source and CPU execution tests require no external tools.

Character designs were drawn for the original Studio firmware and reordered
into the public native character codes. No historical ROM font was used.
Shared original arithmetic source resides in `cpu/z80/firmware/original`;
reference firmware is never a build input.

Manufacturer references are Sinclair's *ZX81 BASIC Programming* (Steven Vickers,
second edition, 1981), the original Sinclair ZX81 circuit diagram and Science of
Cambridge's *A Course in BASIC Programming* (Hugo Davenport, second edition,
1980, appendices II/III). The CPU contract is Zilog UM008011-0816. Reference ROM
tests observe execution, RAM and signals only; they do not inspect or copy ROM
implementation code. These inputs remain outside ordinary tests and packaging.
