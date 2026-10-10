## Start here

This programme targets the exact **ZX81 1K or 16K PAL** configuration above and in
[zx81.toml](zx81.toml). Use Eightbit Studio Z80 on macOS. You do not need to build
the IDE or install Rust/Node. Replace this introduction with your programme's
purpose, controls and expected results, keeping its memory requirement explicit.

1. Open the folder and edit the manifest's configured source. Opening is inert.
   Save before Build or Run; the active tab does not select a different target.
2. BASIC builds use the built-in native ZX81 encoder. Assembly requires separately
   installed **SjASMPlus 1.24.0**, selected in **Settings → Build Tools**.
3. Select **Build** and inspect Problems, Output and Build Results. The BASIC
   starter clears the display, prints a greeting and counts. The assembly starter
   returns 42 in BC to the generated BASIC `USR` caller.
4. Install **MAME 0.289** separately, select it in **Settings → Emulators**, and
   configure your lawful supported 8192-byte `zx81a.rom` revision under
   **Systems & Firmware**. Validation checks identity, not just the filename.
5. Select **Run**, then focus MAME's external window for keyboard input. Allow
   the native programme to load. **Stop** ends the owned session; **Restart**
   rebuilds saved source and starts again. Confirm a visible edit takes effect.
6. **Export** saves a `.p` snapshot to a new destination. Test that exact file
   separately on the configured machine; it is not a Spectrum TAP or audio tape.

Firmware and external tools are not included. There is no integrated ZX81
debugger. Consult offline Help under Studio essentials, the Z80 reference and
ZX81 for BASIC, keyboard/character conventions, native programme layout and
troubleshooting. Use the installed application's workflows rather than inventing
IDE-source build commands for this programme repository.

## Project files

| Path | Purpose |
| --- | --- |
| `zx81.toml` | Schema-1 exact target and one source path |
| `src/basic/main.bas` | BASIC template's numbered ZX81 listing |
| `src/asm/main.asm` | Assembly template's single Z80 source |
| [AGENTS.md](AGENTS.md) | ZX81-specific coding and assistant guidance |
| [CLAUDE.md](CLAUDE.md) | Tells Claude to read AGENTS.md beside it |
| [tests/README.md](tests/README.md) | Build, runtime and recovery checks |
| `.studio/build/` | Generated output, excluded from Git |

Only the selected language's source directory is created. The alternatives are
not a mixed project: the manifest names exactly one source. Extra files are not
compiled automatically. Organise routines and data in that listing. Valid older
`src/` paths continue to work; do not move them just to match this template.
Supporting files open in read-only tabs; customise them in an external editor
and reopen after manifest changes.

## Working within ZX81 limits

ZX81 BASIC is not Spectrum BASIC. Use native line-numbered source, one statement
per line and line numbers no greater than 9999. Use `LET`, `PRINT`, `IF`, loops and
`GOSUB`/`RETURN` according to the supported native dialect in Help. Keep numbers
unique and prevent main flow from falling into subroutines. Follow the ZX81's
own character table and input conventions: ASCII, PETSCII and Spectrum character
codes are not interchangeable. Colour and sound commands from other computers
do not apply to an unmodified ZX81. Check user input, numeric conversions and
array bounds explicitly.

On the 1K target every byte matters. The generated image uses the collapsed
display-file layout; the 16K target uses a full display file. Programme data,
screen growth and runtime variables compete for RAM. The encoder reserves 256
bytes for stack/workspace, but this is not a guarantee that future arrays or a
larger display will fit. Keep changes small and verify the actual selected
memory profile instead of changing the target to hide an overflow.

Assembly has exactly one `ORG 16514` (`$4082`). The build embeds it in a generated
REM/USR programme; do not create a second system-variable block, display file or
snapshot header. The raw code cap is 256 bytes for 1K and 12000 bytes for 16K;
the complete native image must also fit. Return a numeric result in BC and use
`RET` when called by the generated loader. Preserve the stack, system variables,
display state and registers required by the caller. Includes, macros, scripts,
arbitrary file directives and banked linking are unsupported. Never assume a
Spectrum ROM address, screen map or I/O port applies here.

## Verify and share

Use [tests/README.md](tests/README.md), updating expected results alongside code.
A successful build proves encoding/packaging, not keyboard behaviour or runtime
memory sufficiency. Fix failed current builds before Run. Check the selected
RAM model, exact firmware and external tool versions when diagnosing loading.
Test Stop, visible source changes, restart and recovery as well as the happy path.

Git creation, commits and remote actions require deliberate developer actions.
Track sources, manifest and documentation; exclude `.studio/`, firmware, secrets
and local tool paths. External helpers should read AGENTS.md first. The built-in
Agent uses previewed context and reviewed changes and cannot run tools or Git.
Keep unperformed checks marked pending. Share only authorised source/exports,
with 1K/16K requirements, loading method and controls. Verify physical hardware
separately before claiming it works there.
