## Scope and workflow

Read `c64.toml`, README.md and the complete configured source before editing.
This is a C64 PAL programme in Eightbit Studio 6502, not the Studio application
repository. Preserve schema 1, `c64-pal`, the source language and the developer's
existing work. An implementation request authorises the necessary project source
edits; a plan or review remains read-only. Complete changes in the actual files
using authorised host tools and summarise saved changes and observed checks.

Use `src/basic/` for BASIC and `src/asm/` for assembly in new work, but preserve
valid existing paths. The manifest owns exactly one source: do not invent a
multi-file linker, mixed template, build hook, include path or dependency system.
Organise the single listing with labelled/numbered routines and comments. Keep
all edits within the project root; reject traversal and symlink escapes. Do not
modify generated `.studio/` output, firmware, user settings or external tools.
Do not commit, push, publish, delete files or launch an emulator without the
corresponding developer authorisation. These instructions grant no host capabilities.

## Commodore BASIC V2

- Use the native line-numbered V2 contract in offline Help. Keep line numbers
  unique, update all branch targets when renumbering, and terminate main flow
  before subroutines. `GOSUB`/`RETURN` does not introduce local variable scope.
- Only the first two variable-name characters distinguish names. Plan globals,
  array dimensions, string storage and `DATA` ordering before expanding a listing.
  Keep integer `%` and string `$` types distinct and avoid keyword collisions.
- Do not emit Dragon, Spectrum or BASIC 3.5/7.0 extensions. V2 graphics, sprites
  and sound use hardware registers; there is no native `SPRITE` or `SOUND` command.
- Preserve the C64's PETSCII conventions; do not assume ASCII screen codes are
  interchangeable with PETSCII. Explain any `CHR$` controls and `POKE` addresses.
- Check user input, array bounds, division by zero and `READ` exhaustion. Report
  unsupported syntax rather than silently changing the dialect or target.

## NMOS 6502 / 6510 assembly

- Use the supported ca65 subset and separately installed cc65 2.19 (ca65/ld65
  report V2.18). Exactly one `.org $080D` is required. The build supplies the
  PRG load address and BASIC `SYS 2061` stub; do not duplicate either.
- Use documented NMOS instructions, not 65C02/65816 extensions or undocumented
  opcodes. Respect the 32768-byte code cap and the actual occupied memory range.
  Includes, macros, equates, custom linker scripts, scripts and bank switching
  are outside this build profile.
- Record routine inputs, outputs, clobbered A/X/Y registers and flags. Pair stack
  operations, preserve return addresses and use `RTS` when returning to BASIC.
  Use `CLC` before addition and `SEC` before subtraction where required; do not
  assume carry or decimal mode from an unrelated call.
- Treat zero page, `$0100–$01FF` stack, processor port `$0000/$0001`, ROM workspace
  and mapped I/O as owned resources. Account for screen/colour RAM and VIC-II
  memory selection before placing code, sprite data or buffers. Do not introduce
  interrupt handlers or ROM calls without documenting their entry and restore rules.
- Use verified C64 references or the bundled Help for KERNAL calls, VIC-II and SID
  registers. PAL timing matters; do not claim NTSC or another 6502 machine works.

## Verification and delivery

Use the existing checklist in tests/README.md. BASIC builds need no external tools;
assembly needs validated ca65 and ld65. Run uses separately installed VICE 3.10
`x64sc` and the three exact supported C64 firmware images configured in user
Settings. There is no integrated C64 debugger or Dragon Test Explorer contract.
Do not invent a project test runner or claim a build exercised runtime behaviour.

For a change, record expected output, controls, boundary cases and recovery steps.
Check the current build, explicit Run/Stop/Restart and exported PRG when authorised.
Separate ordinary build results, emulator observations and physical hardware
results. Keep unperformed checks marked pending; never launch an older successful
build after a newer failure. Update README.md when loading, controls or requirements
change. Keep firmware, secrets, tool paths and generated output out of Git and AI
context. The built-in Agent cannot execute tools; external agents remain subject
to their host's permissions and the developer's scope.
