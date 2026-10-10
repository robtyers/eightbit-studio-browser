## Start here

This is a Commodore 64 **PAL** programme for Eightbit Studio 6502 on macOS.
Replace this introduction with your programme's purpose, controls and expected
results. This project does not require the IDE's Rust or Node development tools.

1. Open this folder in Eightbit Studio 6502. Opening is inert. Read [c64.toml](c64.toml)
   and edit its configured source; Save before Build or Run.
2. BASIC builds use the built-in Commodore BASIC V2 encoder. For assembly, install
   **cc65 2.19** separately and select `ca65` in **Settings → Build Tools**;
   sibling `ld65` is required. This distribution's tools report **V2.18**.
3. Select **Build** and inspect Problems, Output and Build Results. The assembly
   starter changes the border/background and returns to BASIC; the BASIC starter
   clears the screen, prints a greeting and counts from 1 to 5.
4. To run, install **VICE 3.10**, select `x64sc` in **Settings → Emulators**, and
   configure lawful firmware in **Systems & Firmware**: `basic-901226-01.bin`
   (8192 bytes), `kernal-901227-03.bin` (8192 bytes) and `chargen-901225-01.bin`
   (4096 bytes). Validation checks the supported revisions, not just filenames.
5. Select **Run**, then focus the external VICE window for keyboard input.
   Run builds saved source afresh. **Stop** ends the owned session; **Restart**
   rebuilds and starts it again. Change the source and confirm the change appears.
6. Use **Export** to save the current programme as a PRG to a new destination.
   Check the exported file separately in VICE. A PRG is load-address-prefixed
   programme data, not a disk image. Never infer hardware compatibility from a build.

Tools and firmware are user-installed and are not included. No C64 debugger is
implemented. Use the offline **Help** sections for Studio essentials, the MOS 6502
and the Commodore 64: BASIC walkthroughs, language reference, assembly examples,
memory, KERNAL, graphics, sprites, sound and troubleshooting.

## Project organisation

| Path | Purpose |
| --- | --- |
| `c64.toml` | Schema-1 target and exactly one configured source |
| `src/basic/main.bas` | BASIC template's numbered Commodore BASIC V2 listing |
| `src/asm/main.asm` | Assembly template's single ca65 source |
| [AGENTS.md](AGENTS.md) | C64 programming and coding-helper instructions |
| [CLAUDE.md](CLAUDE.md) | Directs Claude to the AGENTS file beside it |
| [tests/README.md](tests/README.md) | Repeatable verification checklist |
| `.studio/build/` | Generated build snapshot; excluded from Git |

Only the selected language's source folder is created. The two paths above are
alternatives, not a mixed BASIC/assembly project. Extra files are not automatically
compiled: `c64.toml` selects a single source. Keep related routines and data in
that listing. Existing projects may use different valid `src/` paths; do not
rename them merely to match a newer starter. Supporting documents open in read-only
editor tabs; use an external editor to customise them and reopen after manifest edits.

## Programming the C64

BASIC source uses decimal line numbers and Commodore BASIC V2, not Dragon BASIC,
BBC BASIC or later Commodore dialects. Use `GOSUB`/`RETURN` for routines, keep line
numbers unique and prevent fall-through into subroutines with `END` or a loop.
Variables are global and only their first two characters are significant; `%`
marks integers and `$` strings. Reserve space for variables, strings and arrays.
`PRINT`, `GET`, `READ`/`DATA` and `PEEK`/`POKE` are useful starting points. V2 has
no built-in `PLOT`, `SPRITE` or `SOUND` commands: graphics and audio need deliberate
VIC-II/SID register and memory access. Follow the Help examples for the encoder's
supported character spelling and PETSCII rules.

Assembly uses NMOS 6502 instructions on the C64's 6510. Keep exactly one
`.org $080D`; the build creates the BASIC `SYS 2061` loader. Do not supply a second
loader, PRG address header or linker configuration. A returning routine ends in
`RTS`. The supported source subset allows labels, instructions and bounded data
directives, but not includes, macros, equates, arbitrary file access or bank switching.
The maximum 32768 code bytes is a packaging limit, not permission to overwrite
live memory. Protect the zero page, stack, BASIC/KERNAL workspace and I/O; never
change the processor port at `$0000/$0001` casually. Use explicit carry handling
for arithmetic and document registers and flags clobbered by subroutines.

## Review, test and share

Follow [tests/README.md](tests/README.md) after each meaningful change. Keep a
known-good revision and distinguish successful encoding/assembly from observed
video, sound and input. If a build fails, fix the current source before Run; an
older successful artefact is stale. Diagnose syntax first, then tool validation,
then firmware/runtime configuration.

Git initialisation, staging, commits and remote actions are deliberate developer
actions. Commit source, the manifest and project guidance; exclude `.studio/`,
firmware, credentials and absolute tool paths. The optional in-app Agent uses
previewed context and reviewed changes; it has no Build, Run, Git or terminal
authority. An external helper should read AGENTS.md and report files actually
changed and checks actually run. Publish only authorised programme assets and
your export, with PAL requirements, loading instructions and controls documented.
