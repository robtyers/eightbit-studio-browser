# ADR 0052: static GPL browser edition

- **Status:** Browser implementation and GPL XRoar integration approved on
  10 October 2026. Publication remains separately authorised.
- **Extends:** ADRs 0032, 0043 and 0051 for the browser edition only.

## Decision and scope

Add an additional browser edition served as static HTTPS assets. The user
approved reduced feature breadth, prioritising building, testing, decompiling,
saving and running games, and explicitly approved a GPL-covered browser edition
using XRoar with corresponding source and build instructions available to users.
This is the scoped exception to the existing prohibition on bundling GPL tools;
the desktop products retain their separately installed emulator boundary.

Start with the previously agreed Dragon 32 target. Reuse the shared workbench
shell, toolbar, editor tabs, panels, colours and resizers. A browser-only plain
text editor initially provides native touch/keyboard text input within that
shell; reduced editor features do not change the desktop editor. Do not register
the browser as a supported desktop product or claim all-family parity.

The user approved a browser-only UX pass on 10 October 2026: welcome/sample or
blank-code entry, Edit → Check & build → Play → Save guidance, advanced raw imports,
plain-language feedback, explicit ROM setup and distinct save choices. Shared
toolbar labels and visible compact Save are optional presentation props; desktop
defaults and the common shell remain intact. Narrow browser screens initially
close the sidebar; users can reopen and resize it with the existing controls.
Save/open/ROM dialogues contain focus and close with Escape. ROM validation never
starts a game; Play remains a separate explicit action after successful setup.

A read-only version-1 `presentation` request returns bounded BASIC display ranges
from the existing Rust parser and dialect. It creates no build/media artefact and
has no execution authority. Colouring is debounced, cancellable and bound to the
exact source; non-ASCII, assembly and sources above 64 KiB retain plain native text.
Highlighting is conservative (numbered lines, statement heads, strings/comments),
not a replacement for explicit authoritative build validation.

Reuse authoritative Rust BASIC and media code through WebAssembly. BASIC builds
produce cassette bytes. Test validates the current source, builds it and checks
the cassette/token/listing round trip; it is not an automated gameplay assertion.
Decompile recovers BASIC listings or conservative 6809 assembly proposals from
supported media. Assembly proposals preserve uncertain bytes and cannot recover
original symbols, comments or source structure; assembling/rebuilding them and
the desktop LWTOOLS equivalence workflow remain a later browser increment.

The project/persistence follow-up was approved on 10 October 2026. Replace the
sample-first welcome with Create new project, Load existing project and recent
projects in the shared presentation. Named single-source projects recover
automatically on this device; keep the legacy explicit draft readable. Local
recovery is not a durable backup. Serialise local record replacement under a
bounded per-project Web Lock and reject stale versions; report unavailable
recovery while retaining source when locking or storage fails.
Provide a versioned browser project download
and a ZIP containing the existing desktop `dragon.toml` and source contract.
The user additionally required the same default files and structure as desktop:
use one family-owned scaffold for schema 3, `src/basic/main.bas`, AGENTS, CLAUDE,
README, ignore file and tests guidance. Preserve those imported files and bounded
project test definitions exactly; desktop files take precedence over stale browser
metadata when moving an edited project back. Multi-source/assembly builds remain
outside browser scope.
Recovered assembly remains inspection-only. Do not claim arbitrary desktop
project parity or silently simplify unsupported projects.

An optional, explicit GitHub save/open flow is also approved. A separate
Cloudflare Worker handles GitHub App authorisation and bounded REST operations;
authoring, checking and emulation remain local. App permissions are selected
repositories, Contents read/write and Metadata read, with private repositories
required initially. Use state and PKCE, secure HttpOnly session cookies, strict
same-origin request checks, and server-side token storage in session Durable
Objects. Refresh is serialised; sessions expire after 30 days and are removable.
The worker stores credentials, not project contents. Source/settings/guidance crosses this boundary
only when the user explicitly opens or saves a selected project. Fixed project
paths and conditional GitHub file SHAs prevent arbitrary repository writes and
silent overwrites; conflicts preserve the local work and require opening the
remote copy separately. ROMs, build artefacts and runtime state are excluded.

The browser edition has no terminal, AI, native child processes, server-side
build/emulator execution or automatic cloud synchronisation. GitHub App
registration, secrets, remote services and deployment remain a separate handover
requiring explicit authorisation; local implementation does not authorise them.

## Firmware, source and execution boundaries

Import user-selected firmware through the browser file picker. Validate the
family-owned size and exact Dragon 32 fingerprint before Run. Keep firmware in
memory only; never bundle, download, upload, log, export or persist it. An
imported ROM is separate from a user-selected game cartridge.

Compile pinned XRoar source locally without bundled ROMs or upstream software
catalogues. Run each explicit session in a disposable, same-origin iframe with
a restrictive content security policy. Its filesystem is ephemeral. Stop,
replacement, hidden-page suspension and unmount destroy the iframe and its
audio/execution context. Permit only the application's fixed runtime assets;
block automatic firmware/media downloads and arbitrary guest network access.
Bound files, execution setup time and retained output. Do not launch an earlier
successful build after an edit or failed current build.

Downloads and GitHub saves are explicit user actions. Browser-local project
recovery is automatic, not a durable backup; failures must remain visible. Import never
overwrites an original. Exported listings/media contain no firmware. Files and
iCloud picker/export behaviour require physical-iPad acceptance.

## Distribution and evidence

The browser application is GPL-3.0-or-later. Retain Apache-2.0 notices on reused
original libraries/components and all third-party notices. Build a corresponding
source archive containing the exact browser sources, dependency/source locks,
XRoar source, patches and repeatable build instructions. Public output must not
include desktop host source, credentials, private projects, ROMs or build caches.
Never deploy a browser release without its matching source archive.

The existing Cloudflare site uses assets-only Workers hosting. A browser build
can be staged under `/studio/`; changing production assets, routes, DNS or
services requires explicit deployment approval. The optional GitHub backend is
served at a fixed same-origin `/studio-api/` path; an unconfigured or unavailable
backend must leave local project work fully usable.

- **WEB-01:** Static build; BASIC Build/Test and source/media export reuse Rust.
- **WEB-02:** Bounded, inert BASIC/assembly decompilation preserves originals.
- **WEB-03:** Explicit ROM-gated XRoar Run/Stop with stale-build and lifecycle
  rejection; no ROM/media/source network transmission.
- **WEB-04:** Matching GPL notices, source archive and reproducible instructions.
- **WEB-05:** Real iPad keyboard/touch, sound, file selection/export, resizing,
  backgrounding and recovery acceptance before claiming iPad support.
- **WEB-06:** Named projects, recent projects, bounded automatic local recovery,
  visible storage failures, legacy draft recovery and desktop-compatible export.
- **WEB-07:** Optional user-initiated GitHub open/save, server-held credentials,
  selected private repositories, bounded network work and conflict preservation.

Host tests, WebAssembly compilation, desktop-browser observations, real-ROM
execution, physical-iPad acceptance and hosted publication are distinct gates.
