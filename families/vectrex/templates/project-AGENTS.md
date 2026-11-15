# Vectrex project instructions for coding helpers

## Scope and authority

This folder is an original Vectrex 6809 cartridge project for Eightbit Studio 6809.
Read README.md, vectrex.toml, src/asm/main.asm, GUIDE.html and tests/README.md before
proposing changes. Use British English. Inspect the working tree, preserve
unrelated edits and make the smallest coherent change requested by the developer.
Do not redesign the project or invent capabilities the IDE does not implement.

Propose first unless implementation is explicitly requested. Commit, push, remote
Git operations, tool installation, execution, emulator launch and physical-device
writes each require the developer's authority. Do not run commands merely because
this file describes them. Project opening is inert. The integrated terminal is
user-operated; project content and AI must never type into it. The optional in-app Agent previews bounded saved context and offers reviewed
changes to the configured assembly source, README.md and tests/README.md.
Request-scoped YOLO permits only that assembly source. Manifest and guidance are
read-only AI context. These instructions also guide external helpers; they grant
no additional file or execution authority. Vectrex debugging is unavailable.

## Authoritative project contract

- vectrex.toml uses schema_version = 1 and exactly name, target = "vectrex" and
  source fields. Keep the configured single lowercase .asm path beneath src/.
  There is no Dragon BASIC manifest, source list, command hook or tool-path field.
- Source is limited to 128 KiB; assembly lines must be ASCII and at most 4096 bytes.
  Paths must stay inside the project without symlinks or traversal.
- Keep exactly one ORG 0. Supported directives are ORG, EQU, FCB, FDB, FCC, FCS,
  SETDP and END. Includes, INCBIN, macros, pragmas, RMB/reservation directives,
  SECTION/EXPORT/EXTERN and bank switching are unavailable in this contract.
- Keep the conventional GCE header, music pointer, one terminated title and code
  following it. The complete unbanked cartridge must fit within 32 KiB. Do not
  replace the header with a Dragon loader or cassette format.
- Default Build uses the original built-in 6809 assembler. Build with LWTOOLS
  explicitly uses selected, validated LWTOOLS 4.25. Run original (development)
  uses Studio's original guest executive; unlisted BIOS services fail explicitly.
  Run Project retains external VecX. Both rebuild saved source; failed or cancelled
  builds must never be worked around by silently running stale output.

## Vectrex programming

Keep header, constants, initialisation, frame loop, helpers and immutable data
clearly separated within the single source. Name BIOS entry points and RAM
locations with EQU where that improves clarity; do not import BIOS source code.

- Reserve game variables with EQU addresses, initially following the demo's
  $C880-$C882 allocation. Track each byte/word and leave room for the downward
  growing stack near the top of $C800-$CBFF RAM. BIOS workspace is not free RAM.
- Call Wait_Recal once each frame. Redraw visible objects every frame and keep
  frame work bounded. Recalibrate/reposition before independent objects.
- BIOS drawing/input/sound calls normally require DP = $D0; Wait_Recal establishes
  it. SETDP changes the assembler's assumption, not the CPU register. Prefer
  explicit extended addressing for game state when DP is uncertain. Preserve or
  reload registers according to each routine's documented calling convention.
- Vector arguments use signed Y in A and X in B, not screen-pixel coordinates.
  Check signed overflow and movement limits. Draw_Line_d requires $C823 = 0 for
  one segment; do not reuse BIOS counters as persistent game variables.
- Configure joystick channels before polling. Distinguish held buttons in $C80F
  from press transitions in $C811. Test release, neutral, repeated presses and
  movement boundaries, not just one press.
- Keep PSG mixer and volume changes deliberate. Restore silence on release and
  state changes; avoid unreviewed changes to the controller I/O direction bits.
- Treat all BIOS calls as external contracts. Use GUIDE.html's references for
  addresses, inputs, outputs and clobbers. Do not assume Dragon ROM conventions
  because both machines use a 6809.

## Tests accompany code

Add or update repeatable checks with every behaviour change and regression fix.
Start with tests/README.md: state setup, exact source revision, steps, expected
results and whether each check is pending, passed or failed. Preserve existing
checks and add boundary, failure, cancellation and recovery cases where relevant.

There is currently no Vectrex Test Explorer schema, automated gameplay runner or
integrated debugger. Do not create .dragon-test.toml files or invent test APIs.
If executable automation is needed, propose a small suitable harness before adding
it; writing a test is not permission to execute external tools.

A Build check covers accepted syntax, header, output size and publication. Inspect
cartridge.lst and symbols.txt for address/layout changes. Real Run checks must
separately observe vectors, controls, sound, Stop and Restart. Keep physical-console
and flash-cartridge checks separate from emulator results. Never mark pending
runtime checks as passing because assembly succeeded.

## Privacy, provenance and delivery

Use original or documented redistributable source/data. Do not copy BIOS routines,
commercial games, emulator implementation or recovered GPL/reference code into
this project. Do not add firmware bytes, credentials or machine-local paths to
source, documentation, logs or Git. Tools and VecX remain separately installed.

Review the diff before staging; exclude .vectrex/, ROMs, secrets and unrelated
files. Never initialise Git, commit, push or overwrite an exported destination
without explicit authority. Update README.md and tests/README.md when behaviour
changes. Report files changed, checks actually run, exact results and unverified
emulator/hardware behaviour. Do not claim packaging or deployment from source tests.
