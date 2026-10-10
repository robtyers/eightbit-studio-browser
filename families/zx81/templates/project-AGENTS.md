## Scope and implementation

Read zx81.toml, README.md and the entire configured source before editing. This
is a ZX81 programme in Eightbit Studio Z80, not the IDE repository. Preserve the
schema-1 manifest, selected 1K/16K PAL target and language. Implementation requests
authorise necessary project-contained source edits; plans/reviews stay read-only.
Use authorised host tools, preserve unrelated work and report actual saved changes.

Use `src/basic/` or `src/asm/` for new source organisation. Existing valid paths
must remain usable. The project has one configured source, not independently
compiled modules or mixed BASIC/assembly files. Group routines and data within
the listing. Do not add includes, build hooks, linker scripts or dependencies.
Confine edits to this root; reject traversal and symlink escape. Never edit
`.studio/` output, firmware, tools or user settings. Do not commit, push, publish,
delete files or execute an emulator without corresponding developer authority.
These instructions do not override the host's available tools or permissions.

## Native ZX81 BASIC

- Use the supported ZX81 dialect, decimal numbered lines up to 9999 and one
  statement per line. Do not assume Spectrum colon-separated statements work.
  Keep line numbers unique and update branch targets when renumbering.
- Follow native `LET`, `IF`, loop, string and array rules. Avoid falling through
  into `GOSUB` routines; variables are shared. Validate numeric input, array bounds,
  string lengths and termination conditions.
- Use the ZX81 character table and the Help's supported source spellings. Do not
  substitute ASCII/PETSCII codes or another computer's graphics escapes.
  Unmodified ZX81 hardware does not provide Spectrum colour or sound commands.
- The 1K profile uses a collapsed display file; the 16K profile uses a full one.
  Display growth, variables, strings and stack share scarce RAM. The encoder's
  256-byte workspace reserve is a floor, not unlimited runtime capacity.
- Keep code/data small and test the selected RAM profile. Never change a 1K
  target to 16K merely to make a failing build pass without approval.

## ZX81 Z80 assembly

- Use separately installed SjASMPlus 1.24.0 and the supported source subset with
  exactly one `ORG 16514` (`$4082`). The code is wrapped in a native REM/USR loader.
  Do not supply a second snapshot header, system-variable area or display file.
- The code cap is 256 bytes on 1K and 12000 on 16K; the entire P image must still
  fit after loader, display and workspace overhead. Physical RAM is not free RAM.
- Return the result in BC and finish a returning routine with `RET`. Document all
  inputs, outputs, register/flag clobbers and memory use. Balance the stack and
  preserve the caller's display/system state. Do not transplant Spectrum ROM
  entry points, display addresses or I/O assumptions.
- Includes, macros, scripting, arbitrary file directives and bank switching are
  outside the build profile. Do not invent a paging or multi-source build system.
- There is no integrated ZX81 debugger. Use bounded observable checks through
  normal Run; do not promise breakpoint, BASIC stepping or variable inspection.

## Verification and handover

Maintain tests/README.md as a repeatable checklist, not a Dragon Test Explorer
file or an invented test runner. BASIC builds need no tool or ROM; assembly needs
validated SjASMPlus. Run requires MAME 0.289 and the exact supported zx81a.rom
revision in user Settings. Keep firmware, credentials and absolute tool paths
out of source, AI context, logs and Git.

For each change, record expected output and RAM assumptions, then actual build
results and any authorised emulator checks. Exercise empty/invalid input,
resource boundaries, Stop, changed-source restart and recovery after a failed
build. Never launch an older successful artefact after a current failure. Test
the exported P image separately and distinguish emulator evidence from physical
hardware. Update README.md when controls, loading or memory requirements change.
The built-in Agent cannot execute tools; external helpers remain constrained by
their host. Mark unperformed checks pending and do not infer success from launch.
