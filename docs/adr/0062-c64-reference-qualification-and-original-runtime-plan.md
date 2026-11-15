# ADR 0062: qualify C64 tooling and plan an original runtime

- **Status:** Accepted stages 0–2 and stage-3 bounded original firmware storage/implementation, 20 October 2026. The user explicitly approved the reviewed private-page proposal. Stage 4 implementation was explicitly approved on 21 October 2026, retaining its firmware acceptance checkpoint.
- **Extends:** ADRs 0046 and 0053; preserves ADRs 0032, 0043, 0044 and 0045.

## Decision and current authority

Establish repeatable C64 PAL BASIC/assembly development acceptance with separately
installed VICE 3.10, cc65 2.19 and the exact supported user-selected firmware.
Prepare the compatibility specification, provenance rules, implementation stages
and acceptance gates for an independently authored emulator and ROM. The user
approved this scope after the two-outcome proposal on 20 October 2026.

The subsequent user instruction to deliver the next phase authorises stage 1:
the original documented NMOS CPU, 6510 port and C64 memory-bus foundation.
`eightbit-mos6502-core` and `eightbit-commodore-machine` implement that scope.
The subsequent instruction to continue authorises stage 2: the PAL board,
VIC-II, CIAs, SID, input and machine-side pins, with supplied-ROM hardware
qualification. That instruction did not authorise stages 3–4 or distribution. The board stays
separate from product composition. Extend the existing injected CPU bus with
optional device interrupt sampling; legacy buses retain their boundary-driven
behaviour. The family bus owns DMA wait clocks, peripheral clocks, bounded
frames/audio and explicit pin levels. No host BASIC evaluation, ROM patching,
programme injection into reference firmware is permitted. The subsequently approved
stage-3 original identity may use the bounded private pages described below.
It does not register a new runtime, change defaults, widen
source/project formats, introduce debugging or add an original assembler. No
push, signing, publication, deployment or third-party firmware bundling
is authorised. The subsequent explicit request to commit and continue authorised
the local stage-0–2 commit `1b6269975` and work on the stage-3 storage/interface
checkpoint. The user's subsequent explicit approval authorises its bounded
private-page design and original guest firmware implementation. The subsequent
instruction to continue implements the first guest language slice: tokenisation,
numbered editing/LIST/NEW/RUN and string-literal/control-flow execution. This is
explicitly partial. Continued stage-3 checkpoints add original guest numeric
evaluation/scalars, comparisons/logical operators, IF/THEN and bounded GOSUB/RETURN.
Arrays, functions, allocated strings, remaining statements and the full language/
editor/KERNAL corpus remain open. Product execution remains external VICE on desktop and the
pinned third-party chips runtime in the browser.

## Completion instruction — 21 October 2026

The user explicitly requested committing the control checkpoint and continuing
until the C64 implementation is complete, then confirmed **firmware and Studio
integration**: remaining original guest BASIC/KERNAL/editor work, native/browser
composition and local packaged acceptance. This authorises stage-4 implementation
after the stage-3 acceptance checkpoint is met. Preserve the shared workbench and
VICE/chips alternatives. Public release, remote push, signing/notarisation and
default promotion remain separate approvals. No partial-language checkpoint may
be relabelled complete or promoted to a default. Current authority supersedes the
earlier stage-4 approval-pending descriptions above; historical receipts remain.

## Reference development route

Keep the existing native C64-01–C64-06 contract. Use real ca65/ld65 gates, original
acceptance programmes, guarded PRG export and exact-hash ROM-gated VICE sessions.
Qualify actual packaged creation, editing, saving, building, export, keyboard,
video/sprites, sound and Run/Stop/Restart. Include missing/wrong tools and firmware,
failed builds, stale exports, correction, cancellation and repeated sessions.
Independent VICE loading of exported programmes is a separate boundary check.
Record digital audio capture separately from hearing a physical output device.

Evidence must identify the source revision, tool and firmware hashes, platform,
commands, observations and limitations. Mac qualification does not qualify Linux,
Windows, browsers or physical C64 hardware. Upstream signature failures are
documented explicitly; do not remove quarantine, re-sign vendor code or weaken
macOS protections to obtain acceptance. Tools remain outside the source tree and
application distribution.

## Original foundation and planned composition

The original CPU belongs under `cpu/mos6502/`, with no family, application,
host I/O or filesystem dependency. The memory foundation lives under
`families/commodore/`; future machine devices, firmware, media and sessions
remain family-owned. Native and WASM use the same bounded Rust CPU/memory
implementation. Future applications will compose machine/session code through
the existing shared host and workbench; no copied shell or family layout.

The intended machine is the existing PAL C64, with 64 KiB physical RAM, the 6510
processor port and memory banking, VIC-II, both CIAs, SID, colour RAM, keyboard,
joystick inputs and machine-side cassette/serial signals. External drives,
cartridges, NTSC and other Commodore machines need separate scoped decisions.
Unsupported device behaviour must be explicit, not fabricated success.

The original ROM would execute BASIC and operating-system services as guest
6510 code. Preserve BASIC V2 tokens, public RAM/workspace and published KERNAL
interfaces, and stock-compatible PRG outputs. Do not substitute a host BASIC
evaluator. Independently author screen/editor and character graphics as well as
interpreter/OS code. Reusing chips, VICE or an existing replacement ROM does not
satisfy independent authorship.

Firmware storage is a design checkpoint, not an assumed original-size limit.
Measure the feasibility of standard BASIC/KERNAL/character windows first. Any
larger bounded container/private-page design must preserve all guest RAM and
public memory/banking behaviour, isolate private controls, define interrupt/error/
Reset semantics and be approved before implementation. Neither ADR 0058 nor
ADR 0060 grants C64 private pages. A physical replacement-ROM product is not
currently proposed. Programmes exported for stock machines must never depend on
Studio-only storage or implementation entry points.

Keep explicit VICE and browser chips/supplied-ROM alternatives. Add a distinct
original firmware identity and validation route; never weaken legacy exact-ROM
checks. Do not promote a monitor or partial BASIC runtime to the qualified default.
Assembly initially retains separately installed ca65/ld65. An original assembler
and integrated debugger remain separate possible extensions.

## Provenance and qualification

Use manufacturer/interface facts and documented lawful behavioural observations.
Do not copy, translate or derive shipped code from Commodore ROM disassemblies,
VICE/chips or other interpreter/emulator implementations. Keep reference ROMs
outside Git, ordinary tests, logs, support bundles and distribution. Original or
generated fixtures and source/builder/image manifests must establish provenance.

Qualify hardware using supplied reference firmware, then firmware against stock
hardware behaviour, then the combined original runtime. VICE can execute our ROM
only if its format/memory mapping fits the stock machine. If private storage is
eventually approved, mark that matrix cell unavailable and use independently
specified public-interface/behaviour vectors; do not claim full-ROM VICE parity.

The implementation stages, compatibility contract and exit criteria are owned by
[the C64 original-runtime specification](../c64-original-specification.md).
[Testing](../testing.md) owns dated observations. That specification distinguishes implemented stages 1–2 and their named
development observations from planned firmware/product stages; it is not a supported-formats declaration or a complete
emulator/firmware acceptance claim.

## Stage-3 storage decision — explicitly approved

The [storage/interface checkpoint](../c64-original-specification.md#stage-3-storageinterface-checkpoint--20-october-2026)
inventories the published interfaces, measures original public-RAM vectors and
separates exact data sizes from code-size estimates. Complete stock-window fit
has not been proved or disproved. The recommended implementation allowance is
at most sixteen 8 KiB original BASIC pages, one fixed 8 KiB original KERNAL and
one 4 KiB original character image, with bounded source-bound metadata. This is
an approved 140 KiB firmware payload ceiling, not additional C64 RAM.

Only validated original firmware enables manifest-bound guest 6510
gates in fixed KERNAL code. They may change the BASIC page and a bounded private
page-continuation stack, with explicit interrupt/callback/error/Reset rules.
No host language execution or hidden BASIC variable store is permitted. All
public RAM, 6510 banking and VIC accesses remain unchanged; public BASIC entries
are common stubs in each page. Stock PRGs cannot call private controls. Legacy
firmware validation and ordinary VICE/chips product routes remain unchanged.

The user's explicit approval accepts this page extension and stage-3 guest
implementation. Qualification remains incremental: implemented storage, a boot
screen or a partial language does not complete stage 3. Product composition,
default promotion, commits, pushes and publication are not authorised by this
approval. Historical checkpoint receipts retain their recorded pending state.

## Explicit Studio composition — 21 October 2026

The confirmed firmware/Studio continuation uses the existing explicit original
Run action, not a settings-schema change. The shared native runtime interface has
an additive explicit-selection hook whose default preserves existing families.
C64 accepts only the explicit original route; ordinary VICE and browser chips
remain the defaults. The portable Commodore session validates a fresh standard
PRG, boots the source-bound original firmware, loads public programme RAM and
queues RUN through the physical key matrix. Guest firmware performs all execution.
Native assembly retains ca65/ld65; browser authoring retains its BASIC scope.

Only the original image bound by `runtime-manifest.json`, the original assembly/
character sources and builder is admitted to native/browser bundles. The browser
corresponding-source graph now includes the original MOS6502/machine/session and
firmware/builder inputs. No reference ROM, GPL native emulator or recovered
implementation enters the original runtime. Local verification and packages do
not authorise default promotion, publication, signing or hardware claims.
