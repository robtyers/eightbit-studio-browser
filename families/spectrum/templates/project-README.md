## Start here

This is a Sinclair ZX Spectrum programme for the exact UK/PAL model identified
above and in [spectrum.toml](spectrum.toml). Develop it in Eightbit Studio Z80 on
macOS; you do not need the IDE's Rust or Node toolchain. Replace this introduction
with the programme's purpose, controls and expected output.

1. Open this folder in Eightbit Studio Z80 and edit the configured source. Opening
   does not execute anything. Save your changes before Build or Run.
2. BASIC builds use the built-in native Sinclair BASIC encoder. For assembly,
   install **SjASMPlus 1.24.0** separately and select its executable in
   **Settings → Build Tools**. The starter uses `ORG 24576` and changes the border.
3. Select **Build** and inspect Problems, Output and Build Results. A successful
   build produces tape media; it does not prove the programme runs correctly.
4. Install **MAME 0.289**, select its executable in **Settings → Emulators**, and
   choose the exact supported ROM file(s) for the selected model in
   **Systems & Firmware**. The 128K and +2 profiles require their two ROM slots;
   a filename or RAM size alone does not establish firmware compatibility.
5. Select **Run**, focus the external MAME window for input and wait for loading.
   The BASIC starter prints a greeting and plays eight tones; the assembly starter sets
   border colour 2 then returns. **Stop** ends the owned session. **Restart**
   rebuilds saved source and starts again. Confirm a visible edit appears.
6. **Export** writes a `.tap` to a new destination. Test that exact tape image,
   including its loader, separately. Keep machine requirements and controls here.

Tools, emulator and firmware are separately supplied. See offline Help under
Studio essentials, Z80 processor reference and ZX Spectrum for native BASIC,
assembly, loading, input and troubleshooting. Only Spectrum 48K currently offers
integrated machine-code debugging; BASIC debugging and debugging the other models
are unavailable. A 48K debug snapshot starts machine code without a normal BASIC
caller: use Run to test returning to BASIC or routines that depend on ROM state.

## Files and source organisation

| Path | Purpose |
| --- | --- |
| `spectrum.toml` | Schema-1 target and one configured source |
| `src/basic/main.bas` | BASIC template's numbered listing |
| `src/asm/main.asm` | Assembly template's single Z80 source |
| [AGENTS.md](AGENTS.md) | Spectrum-specific coding instructions |
| [CLAUDE.md](CLAUDE.md) | Instructs Claude to read the neighbouring AGENTS file |
| [tests/README.md](tests/README.md) | Repeatable build and runtime checks |
| `.studio/build/` | Disposable generated build output, excluded from Git |

Only the chosen language's folder is created. BASIC and assembly are alternative
templates; this is not a mixed multi-file project. The manifest names the source;
placing more files under `src/` does not add them to a build. Keep routines and
data in that source. Older projects may use other valid paths under `src/`;
there is no need to move them. Supporting files open in read-only tabs: customise
them externally and reopen after changing the manifest.

## Native Spectrum programming

Use line-numbered Sinclair BASIC for this exact target. Keep line numbers unique,
use `GO SUB`/`RETURN` for routines and avoid falling through into them. Strings use
`$`; string variables and loop/control conventions differ from Commodore and
Dragon BASIC. Follow Help for supported token spellings, numeric encoding and
character escapes rather than pasting another machine's listing unchanged.
`INK`, `PAPER`, `BORDER`, `PLOT`, `DRAW`, `BEEP` and keyboard handling belong to the
Spectrum environment; check coordinates, colour ranges and shared display attributes.
The implemented BASIC encoder does not support `DEF FN`; 128K/+2-only keywords
such as `PLAY` and `SPECTRUM` must not enter a 16K/48K listing.

The 16K BASIC payload limit is 7000 encoded bytes; the other profiles permit 40000.
These are build limits, not free runtime memory: leave room for variables, arrays,
stack and display. The 128K/+2 profile does not enable arbitrary paged assembly.

Assembly is bounded Z80 with exactly one `ORG 24576` (`$6000`), at most 8192
code bytes. Run's generated BASIC loader reserves space with `CLEAR`, loads the
code and invokes `USR`; a normal returning routine ends with `RET`. Do not supply
your own tape headers or loader. Includes, macros, scripting, arbitrary file
output and banked linking are unsupported. Protect the screen and attributes,
system variables, stack and any memory still owned by BASIC. Document register
clobbers and interrupt assumptions, and preserve the caller's stack.

## Verification, helpers and sharing

Use [tests/README.md](tests/README.md) for current-build, input, display, failure
and recovery checks. Fix failed builds before Run; the last successful artefact
is stale after failure. Verify the selected model, ROM slots and tool versions
when diagnosing a loader problem. A successful emulator launch is not a test of
the programme's calculations, graphics or controls.

Git creation, commits and remote actions are separate deliberate actions. Track
source, manifest and guidance, not `.studio/`, firmware, secrets or absolute tool
paths. Ask external coding helpers to read AGENTS.md before making bounded edits.
The optional built-in Agent previews context and offers reviewed source changes;
it cannot run Build, Run, Git or terminal commands. Report actual checks and keep
unperformed ones pending. Share authorised exports with exact model, loading and
control instructions; test physical hardware separately before claiming support.
