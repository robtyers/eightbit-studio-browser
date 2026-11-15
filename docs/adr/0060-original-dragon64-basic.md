# ADR 0060: original Dragon 64 BASIC in both memory modes

- **Status:** Implementation authorised by the user, 20 October 2026; qualification pending.
- **Extends:** ADRs 0036, 0039 and 0057.

Implement independently authored Studio firmware for PAL Dragon 64 in both
32K compatibility and native BASIC modes. Reuse the original Dragon BASIC
sources and digital machine, preserving existing project schemas, token formats,
four graphics pages, public workspace, cassette formats and mode-specific bounds.
Compatibility programmes retain `$1E01–$7FFF`; native programmes retain
`$1E01–$BFFF`. Physical RAM remains 64 KiB. Firmware storage need not fit in
16 KiB; this is a Studio runtime, not a physical replacement-ROM deliverable.

Native firmware uses common code and public gates in `$C000–$CFFF` and
`$E000–$FEFF`, with bounded private 4 KiB pages at `$D000–$DFFF`.
Generated guest 6809 gates preserve registers and flags, save/restore page
selection across nested calls and interrupts, and keep stack-sensitive BASIC
control flow and tables common. Only the immutable common gates may use the
private page controls; RAM programmes cannot select pages accidentally. Errors,
non-local interpreter recovery and Reset revoke abandoned page continuations.
No host BASIC evaluator, third-party ROM, executable RAM reservation or new
process/network authority is introduced. Native public entries use their
relocated addresses; compatibility entries retain their existing addresses.

Share source without altering the qualified D32 artefacts. Each D64 mode has
its own image/source/builder/symbol/map identity and qualification. A D32
qualification cannot admit D64. Run and Debug must use the current target,
current build and exact firmware identity; the existing XRoar alternative,
third-party firmware checks and separate native window remain available.
Cartridges, arbitrary banking, serial host tooling, DragonDOS and OS-9 retain
their existing external runtime boundaries. Stock cassette exports must not
depend on private Studio firmware addresses or page controls.

Verification covers actual guest boot, both RAM limits, public entries, page
isolation and restoration, language/control/error recovery, high-RAM programmes,
cassette loading/saving, debugging, cancellation and stale-build rejection.
Ordinary checks, lawful exact-ROM comparisons, native/WASM parity, browser and
packaged application acceptance, and physical cassette acceptance are distinct
gates. No finite test suite establishes universal historical compatibility.
No commit, push, publication, signing or deployment is authorised.

Implementation requirements and provenance are tracked in
[the D64 specification](../dragon64-basic-specification.md). Native mode entry
preserves the existing programme while resetting variables and reinstalling
public hooks. Its guest keyboard repeat and serial-printer settings follow the
Dragon 64 supplement. The disconnected ACIA opens no host serial connection.
