# ADR 0058: original Spectrum 48K development toolchain

- **Status:** Accepted; software paths implemented and locally verified on 19 October 2026. Original firmware retains its development identity; release and physical hardware acceptance are separate.
- **Extends:** ADRs 0041, 0042 and 0053; preserves ADRs 0032, 0043 and 0045.

## Approved outcome

Provide an independently distributable Z80 processor implementation, Spectrum
48K machine, original Sinclair-compatible BASIC firmware and original source
assembler for the desktop and browser IDEs. Write, build, Run and Debug must need
no user-installed emulator, assembler or third-party ROM. Preserve the shared UI,
existing project/media contracts and explicit MAME/SjASMPlus alternatives. Other
Spectrum models and ZX81 retain their current implementation and tool boundaries.

The user explicitly rejects a 16 KiB implementation-size constraint. Functional
compatibility is the objective, rather than reproducing the original firmware's
implementation, internal layout or physical construction. Preserve the 48 KiB
RAM map, native Z80 execution, public ROM/RAM interfaces, native BASIC tokens,
graphics/keyboard/beeper behaviour and ordinary cassette formats. Games exported
for a stock 48K Spectrum must not depend on Studio firmware extensions. Our
firmware is a Studio runtime, not a physical replacement-ROM deliverable.

## Firmware storage and provenance

An expanded firmware container may provide private ROM pages in the existing
low-memory ROM window. It must never consume guest RAM or turn a 48K project into
a 128K/banked-RAM target. Fixed public entry points remain visible in every page.
Private page selection is available only to original firmware gates; ordinary
RAM programmes and their I/O must not change the page latch accidentally. Source
and tests must identify the selected page and preserve it across interrupts,
public calls, errors, Reset and debugger stops. Firmware storage need not fit in
16 KiB. Container size, page count, input and execution limits remain bounded.

The implementation is independently authored from processor/manufacturer facts,
Sinclair's published user-facing contracts and bounded lawful black-box
observations. Do not copy or translate Sinclair ROM disassemblies or third-party
emulator/assembler/interpreter implementation code. No third-party firmware is
bundled. Verified original Spectrum firmware and its complete original source
are an additional bundling exception; do not broaden third-party ROM permissions.
Firmware manifests bind original source, builder, image and public memory map.

## Ownership and integration

The pure CPU and source assembler belong under `cpu/z80/`; Spectrum hardware,
firmware, tape policy and language/runtime interfaces belong to the Spectrum
family. Neither may depend on another family or an application. Native and WASM
hosts compose the same bounded Rust execution/session APIs. BASIC executes in
guest Z80 code; the host must not substitute a BASIC evaluator or secretly run
MAME/SjASMPlus. Unsupported source extensions fail explicitly.

Keep opening inert, explicit fresh-build Run/Debug, coherent stopped state,
source/byte-bound breakpoints, cancellation, stale-result rejection, owned native
windows and disposable browser workers. Preserve earlier schemas and diagnostics;
add versioned fields/contracts only where an accepted extension requires them.

The shared native source host accepts an application-composed runtime adapter;
it retains cancellation, operation exclusion and Git/project guards. With no
external emulator selected, a Spectrum 48K project uses the original development
adapter. Existing MAME settings retain their route. An explicit original Run
action can select the adapter without rewriting those settings; Restart and
explicit-save rebuild retain that choice. Only the main window can invoke
project services, and each auxiliary window can command only its own session.
Closing an older window must not stop a successor or shut down host services.

The browser catalogue declares the original default only for Spectrum 48K, and
the shared workbench routes its normal Play/Debug actions through the portable
session. The existing emulator selector retains explicit MAME/supplied-ROM use.
Older catalogues without that declaration keep their legacy route; other models
and other families retain their existing defaults. Source qualification and
actual browser/package acceptance remain separate from these implementation
choices and from any public release.

BASIC statement debugging observes the original firmware's build-bound statement
entry and published PPC, against the exact tokenised programme loaded in guest
RAM. It does not evaluate BASIC in the host. Assembly inspection remains bound
to the original source map and loaded instruction bytes. Both invalidate source
locations when their bytes change; Step and Continue require the latest stop.

The IDE's literal text queue must coexist with the guest's keyword/extended/
graphics keyboard modes. A private `$FDFB` input reports whether an
owned literal typing operation is active, only while the original page-16 editor
executes. Gated writes from that editor acknowledge when it is waiting for its
next key, so a queued character cannot overtake rendering or syntax checking.
Other callers retain ordinary floating-bus behaviour and cannot write the latch. The host still
supplies timed matrix chords; guest code interprets and tokenises their text.
The flag grants no extra RAM, executable service or host BASIC evaluation.
Stop, Reset and failure clear it; a focus change releases manual keys without
truncating already submitted text.

## Acceptance

Requirements and progress are tracked in `docs/spectrum48-original-specification.md`.
Qualification requires instruction/flag/interrupt tests, machine/video/input/audio
and tape tests, complete BASIC language/error/recovery tests, assembler/source-map
tests and native/WASM parity. Native/browser authoring, fresh-build Run/Debug,
keyboard, graphics, sound, cassette export/loading and failure recovery must be
observed across their actual boundaries. Stock-ROM reference-emulator loading and
physical Spectrum cassette loading/gameplay are separate evidence classes.
Neither monitor boot nor a small language subset qualifies the complete default.

Run the ordinary gate and all affected product/package checks before claiming a
completed increment. Source publication, deployment, signing, releases, commits
and pushes remain separate actions requiring their own approval. No universal
historical-software or physical-hardware compatibility claim follows from finite
automated coverage.
