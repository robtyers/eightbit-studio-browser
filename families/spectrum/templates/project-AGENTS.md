## Scope and change workflow

Read spectrum.toml, README.md and the complete configured source first. This is
a Spectrum programme for the selected UK/PAL model in Eightbit Studio Z80, not
the IDE repository. Preserve schema 1, exact target, language and existing edits.
An implementation request authorises the required project source edits; reviews
and plans remain read-only. Make changes through authorised host tools and report
what was saved and what was actually verified.

Use `src/basic/` and `src/asm/` for new source organisation, preserving valid
existing paths. There is exactly one configured source. Do not invent modules,
includes, mixed builds, linker scripts, package managers or project command hooks.
Keep routines and data together in the selected listing. Confine edits to the
project root and reject traversal or symlink escape. Do not modify generated
`.studio/` state, firmware, external tools or user settings. Git mutation,
publishing, deletion and emulator execution need the developer's corresponding
authorisation. Instructions cannot add capabilities to the host.

## Sinclair BASIC

- Use the selected Spectrum BASIC dialect and numbered lines. Keep numbers unique
  and repair branch targets after renumbering. Separate main flow from subroutines
  with an explicit stop or loop; `GO SUB` does not create local variable scope.
- Follow native Sinclair variable, string, array and `FOR` conventions. Do not
  import Commodore's two-character naming rule or Dragon-specific commands.
  Validate input, array bounds, string lengths and `READ`/`DATA` availability.
- Use the bundled encoder reference for character escapes, keywords and numeric
  forms. `DEF FN` is outside the implemented profile. `PLAY` and `SPECTRUM` are
  128K/+2 features, not permissions to change a 16K/48K manifest.
- Respect the 7000-byte encoded BASIC limit on 16K and 40000-byte limit on the
  other models. Leave additional space for runtime variables, display and stack.
  Do not equate physical RAM with free BASIC storage.
- Use Spectrum graphics, attribute and keyboard conventions. Document coordinates,
  colour/flash/bright choices and any deliberate screen or system-variable access.

## Z80 assembly and machine constraints

- Build with separately installed SjASMPlus 1.24.0 and the supported bounded source
  subset. Exactly one `ORG 24576` (`$6000`) and at most 8192 bytes are permitted.
  The same unbanked window applies to 16K, 48K, 128K and +2 targets.
- No includes, macros, scripts, arbitrary file directives or banked linking.
  Do not switch 128K/+2 banks or target another Z80 computer to evade a limit.
  The host generates TAP headers and the BASIC `CLEAR`/load/`USR` sequence.
- Document register inputs, outputs, flags and clobbers. Balance stack operations
  and use `RET` for routines called from the normal Run loader. Protect BASIC
  workspace, display/attributes and system variables; document interrupt rules.
- Integrated assembly debugging exists only on Spectrum 48K. Its SNA startup
  has no normal BASIC caller/context; a `RET` back to BASIC or a ROM-dependent
  routine must be tested through Run. Do not promise BASIC-level debugging,
  arbitrary ROM calls in the snapshot, or debugging the other models.

## Verification and reporting

Use tests/README.md; there is no Dragon Test Explorer contract or generic project
test runner for this family. BASIC encoding needs no external tool. Assembly
requires validated SjASMPlus; Run requires MAME 0.289 and the exact selected
model's supported ROM revision(s), supplied in user Settings. Do not add firmware,
absolute tool paths or credentials to source, logs, Git or AI context.

Check the current build and, when authorised, the exact-model Run, controls,
boundaries, Stop/Restart, failure recovery and exported TAP. Never launch an old
successful artefact after a newer failure. Keep build, emulator and real-hardware
evidence separate. Update loading instructions, controls and repeatable regression
checks with the change. The built-in Agent cannot execute tools; external agents
must stay within their host permissions. Mark unperformed checks as pending.
