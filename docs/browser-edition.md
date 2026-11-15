# Eightbit Studio browser edition

## Original Spectrum 128K and grey +2 — ADR 0059

UK 128K and original grey UK +2 now use the same original Rust/WASM runtime and
shared browser workbench as 48K. BASIC and single-source Z80 assembly can build,
Play and Debug without supplied firmware. The existing selector retains explicit
MAME with supplied ROMs. Empty-machine boot also honours the selected model.
The same frame/session guards, local cassette handling and source packaging apply;
no browser BASIC, PLAY or RAM-disc evaluator is added. [Scope and
qualification](spectrum128-original-specification.md) do not constitute a public
deployment or physical-device acceptance.


## Expanded original BASIC profile

The approved Studio profile uses an original 32 KiB ROM while retaining 32 KiB
RAM, published BASIC interfaces and existing project/media formats. It needs no
third-party firmware. **Built-in Dragon 32** is the default for BASIC and RAM
machine code. Cartridges are unsupported on the website; use the desktop edition
with separately configured XRoar and third-party firmware. The workbench and emulator controls
are unchanged. Normal project Run/Debug admission is bound to the exact expanded
source, image, symbols and memory-map receipt. Use Check & build and Play or Debug.
Normal startup loads the current build through guest cassette I/O. The console
accepts up to 249 characters and supports local CAS/WAV mounting and recording.
The earlier 19 October BASIC native/WASM check passed 1,466 identical requests, including
source debugging, INPUT, graphics, music and a recorded data tape read by a fresh
guest. The actual browser Play journey also saved and reloaded that same
446-byte tape, printing `DATA OK 64 SURVEY 64`. See [testing](testing.md) for
exact evidence and release limits. The earlier checkpoints below describe the
retained 16 KiB development profile, not the expanded profile's delivery target.

## Original assembler and linker

Dragon 32 RAM projects use the original shared assembler/linker without a tool
installation. Import retains the exact manifest, ordered list of up to 64 assembly
files and any declared unused BASIC entry. The Explorer edits each declared
source; diagnostics and debugging identify the corresponding file. File-local
symbols, `EXPORT`/`EXTERN`, consecutive code sections and absolute constants cover
the unchanged DragonDoom and Battlezone projects. Both games build to the same
machine code as their established LWTOOLS builds and pass actual native/WASM
guest checks. This is a bounded source toolchain: macros, includes, arbitrary
linker scripts and object archives are unsupported. Desktop retains explicit
LWTOOLS compatibility; website cartridge development remains unsupported.

## Original Dragon monitor and retained development checkpoints

These earlier checkpoints describe the monitor/development actions. Statements
that project Play uses only XRoar are superseded by the expanded profile above.

For Dragon 32, Computer setup offers **Start original monitor**. This boots the
bundled original Eightbit firmware, using Rust/WebAssembly in a disposable
worker, without selecting a Dragon ROM. The existing Game panel presents its
screen and labelled keyboard-command field. Type `H` for help. Stop or page
suspension destroys the session; no RAM, input or firmware state is persisted.
Existing BASIC Play remains the explicit XRoar/selected-ROM workflow.

ADR 0057 adds **Start original BASIC (development)** beside the monitor action.
It uses the same bounded Worker/session core and labelled console, with Shift
chords, case, empty replies and a 251-character input limit. The ROM is visibly
unqualified; project Play still uses XRoar. This does not establish the remaining
graphics/music/file commands, full ABI compatibility or BASIC project integration.
The compacted development image additionally executes LIST/LLIST ranges, DEL
CIRCLE ellipses/arcs and MID$ assignment in guest code. A full ellipse now executes in the
shared native/WASM acceptance corpus; compact FN names and ELSE endings share
the Rust/guest contract. These commands do not change the shell,
the explicit XRoar path or the outstanding qualification and integration gates.

The development console's **Cassette** controls mount a checked `.cas` tape,
including tokenised/ASCII BASIC, machine code, opaque gapped data records and
multiple files. Limits are 1 MiB, 256 files and 8,192 blocks, including the
framing of a full 64 KiB memory capture. Rust streams physical cassette pulses
from the original bytes; it does not inject programme memory. Enter `CLOAD` then
`RUN` for BASIC, or `CLOADM` then `EXEC` for machine code. **Rewind cassette**
returns to the beginning; Reset retains the current position. Invalid replacement
files preserve the previous attachment, and Stop releases it. Cassette contents
stay volatile and are excluded from project recovery and GitHub saves.
Guest-pin recording also retains complete multi-file/data tapes; damage in any
file rejects the recording. Data-file transport does not add the staged
`OPEN`/`PRINT#`/`INPUT#` implementation to the current development ROM. Source
import and ordinary WAV recovery retain their single-programme contracts.

After a fresh Dragon 32 build, **Mount built cassette in original BASIC
(development)** starts that explicit development session with a copy of the
current CAS. It types no command and does not replace Play's selected-ROM/XRoar
route. Editing or replacing the project invalidates that build and stops its
session. Native Dragon 32 BASIC-only projects also provide **Build and mount in
original BASIC (development)** in the existing emulator controls. This explicit
action saves current source, prepares a fresh checked CAS and opens the separate
window after the source revision is rechecked. **Cancel development launch**,
project closure, timeout and stale results discard preparation. A failed current
build cannot launch an older cassette. The native development window retains
manual CAS selection as well.
**Record cassette output** starts a bounded capture of guest cassette pins.
Enter `CSAVE` or `CSAVEM`, wait for the guest to return, then choose **Finish
recording**. Rust drains each execution slice, converts the physical levels to
9.6 kHz PCM and validates a recovered CAS through the existing media decoder.
**Save recorded cassette** downloads that checked file in the browser; the native
window opens a restricted Save dialogue and creates only a new `.cas` file.
Cancelling Save retains the recording. Only one native Save dialogue may be
pending; Reset, discard or session replacement invalidates its recording
revision before any destination is created. Incomplete signals, capture loss and the
15-minute signal limit fail visibly; **Discard recording** resets capture state.
Reset cancels an active recording, while Stop releases all volatile recordings.
No microphone or host BASIC evaluator is involved. Multi-file tapes, data-device
records and WAV mounting/export are not exposed by these development controls.
Normal original-ROM project
Run/Debug, complete language/ABI qualification and packaged browser/native
acceptance remain separate work.

The monitor and BASIC consoles remain experimental; complete BASIC and
historical-program compatibility remain open. See [ADR 0056](adr/0056-original-dragon32-monitor.md) and the
[specification](dragon32-monitor-specification.md). The original firmware source,
image, symbols and receipt are included in corresponding source. Source packaging
allows only the verified original monitor and development BASIC images with
their complete source receipts; all third-party firmware exclusions remain.

Local monitor build/check commands, using the pinned wasm-bindgen tool and
separately installed LWASM:

```sh
python3 scripts/build-dragon-monitor.py --check
bash scripts/browser/build-core.sh
node scripts/browser/monitor-parity.mjs
pnpm --dir apps/eightbit-studio-web/ui build
```

ADR 0052 adds an experimental GPL-3.0-or-later browser edition alongside the
desktop products. ADR 0053 adds C64 PAL, Spectrum 16K/48K/128K UK/+2 UK and ZX81 1K/16K alongside Dragon 32 BASIC. ADR 0055 adds BBC Micro Model B BASIC II. The existing workbench
shell is shared; the browser editor uses a native text area for touch/keyboard
input. ADR 0057 adds original Dragon 32 BASIC and machine debugging and assembly
compilation. Terminal and AI remain outside browser scope; these Dragon additions
do not extend the other computers' capabilities.
The optional GitHub service saves project snapshots through a separate Worker;
there is no browser terminal or general Git command interface.

The **Get Started** welcome uses the shared chip-and-eight mark and wordmark
from the desktop applications and website. Choose **Create new project**,
**Load existing project**, or a **Recent project**.
Creation asks for a name, a computer grouped under 6809/6502/Z80, and a blank
BASIC, original sample or Dragon 32 assembly template. BASIC projects contain one editable source; Dragon 32 assembly projects support 1–64 explicitly declared source files. They retain the
same family-owned manifest, source folder, AGENTS, CLAUDE, README, ignore file
and tests guidance as that computer's desktop Basic template. Shared pure Rust
scaffolds prevent the defaults drifting between editions.

| Computer | Desktop manifest | BASIC game file | System firmware |
| --- | --- | --- | --- |
| Dragon 32 | dragon.toml (schema 1–3 import, schema 3 creation) | BASIC/machine CAS and loading-audio WAV export; CAS/WAV recovery | Bundled original 32 KiB; optional XRoar uses exact third-party 16 KiB firmware |
| C64 PAL | c64.toml (schema 1) | PRG at $0801 | BASIC/KERNAL/character ROMs, 20 KiB, existing desktop CRC revisions |
| BBC Model B 32K | bbc.toml (schema 1) | 200 KiB DFS SSD; canonical BASIC subset recovery | OS1.2/BASIC II/DNFS120/speech/characters, 66,496 bytes, existing desktop CRC revisions |
| Spectrum 16K/48K UK | spectrum.toml (schema 1) | BASIC TAP | 16 KiB, exact model SHA-256 |
| Spectrum 128K UK/+2 UK | spectrum.toml (schema 1) | BASIC TAP | Two ordered 16 KiB ROMs, exact combined model SHA-256 |
| ZX81 1K/16K | zx81.toml (schema 1) | BASIC P | 8 KiB second revision, exact SHA-256 |

Blank/sample changes only the initial programme. Guidance/settings files are
viewable in Explorer and preserved; edit them in the desktop edition. The shared Explorer, tabs, toolbar and resizable panels remain familiar;
**Getting started help** optionally reveals Edit → Check & build → Play → Save.
The sidebar initially closes below 701 px and Explorer reopens it. The native
editor retains conservative, cancellable Rust-supplied colouring. Assembly,
non-ASCII and sources above 64 KiB use plain text; source is limited to 1 MiB.

For Dragon 32, **Save work → Save cassette loading audio** renders the current
checked BASIC or assembly build as `game.wav`. Play the uncompressed file through
the stock Dragon cassette input: `CLOAD`, then `RUN` for BASIC; `CLOADM`, then
`EXEC` for machine code. Audio is deterministic 44.1 kHz, signed 16-bit mono PCM
using the shared Rust codec, with a 64 MiB output bound. A disposable worker
transfers binary samples; cancellation, changed source and late results cannot
download stale output. Input retains its 8 MiB limit, so larger WAV downloads
can use desktop import. Our expanded ROM is not a dependency on the physical
machine. Real cassette loading, audio levels, controls and timing still require
physical acceptance. Browser cartridges remain unsupported.

Projects recover automatically after a 350 ms pause in editing. **Saved on this
device** means the current snapshot was written to this browser's local storage,
not backed up or synchronised. Storage eviction, clearing website data, private
browsing and quota failures can remove recovery. Failures stay visible and keep
source open; download a backup or keep a separate local copy. Concurrent-tab
versions and a per-project Web Lock reject overwriting another tab's newer copy.
Browsers without Web Locks retain source and report recovery unavailable; downloads
still work. A lock wait expires after three seconds. Closing/switching while
recovery is pending or failed requires a discard confirmation. Recent cards list
up to eight valid copies; older stored projects are not deleted automatically.
Plain source files have no manifest, so choose **Computer for source files**
before opening a listing. Their preview retains this exact target identity.
A previous single-document draft remains accessible through **Restore browser
draft** and opens as an inert preview before creating a named project.

**Download project ZIP** keeps source, browser snapshot schema 2 (including its
exact computer identity), native manifest and supported guidance files. Schema-1
browser snapshots remain readable as Dragon 32. Extract a BASIC project and open
its folder in the matching desktop processor product, or zip its contents and
reopen here. Put exactly one manifest at the archive root. Desktop source and
manifest files override stale browser metadata. Imported AGENTS, CLAUDE, README,
ignore files, tests/README and bounded tests/*.dragon-test.toml files are preserved
exactly; the browser does not execute project-test definitions.

Imports support one BASIC source and the profiles above, or a Dragon 32 RAM
machine-code project with 1–64 declared assembly sources and its unchanged, unused
BASIC entry. Legacy assembly snapshots remain readable. Reject mixed machine manifests,
unknown assets, browser BASIC-loader mixed programmes, symlinks, unsafe paths and corrupt/duplicate ZIP entries rather
than losing files. Project names use 1–64 ASCII letters/numbers/spaces/underscores/
hyphens and become portable desktop names. Archive size is capped at 8 MiB and 128
entries, total decoded text at 9 MiB, support files at 32 (64 KiB each, 512 KiB total).
Dragon source/manifest limits are 1 MiB/64 KiB; the C64/Sinclair native contracts
retain 128 KiB source/8 KiB manifest limits and their own programme RAM budgets.
Stored/deflated ZIP imports are bounded; exports use deterministic stored entries.

**Check & build** creates the selected computer’s game file using authoritative
family-owned Rust compiled to WebAssembly, then verifies media and restored BASIC
listing round trips. Results describes the next action; diagnostic
messages link to their source position, while codes, sizes and hashes are under
**Technical details**. These checks validate code/file integrity; playing is the
gameplay check. **Recover source…** opens a BASIC listing or conservative 6809
assembly preview from Dragon CAS/WAV, or BASIC from C64 PRG, Spectrum TAP and ZX81 P. Sinclair recovery accepts representable canonical BASIC without saved variables; C64 rejects machine-code tails. Unsupported graphics/control codes are rejected with an actionable result. Raw binary import is under **Advanced import**
with an explicit decimal load/start address; addresses are never guessed. Original
symbols/comments cannot be recovered. Imports stay inert until explicitly used;
original files are never overwritten. Original machine-code media can run without
reassembly; its bytes and declared load/start addresses are checked again at each
launch. Recovered assembly becomes editable source when creating a project, and
Check & build then assembles that current source rather than reusing imported bytes.

Dragon 32 assembly projects use the bundled original 6809 assembler, with no
tool installation or firmware selection. It supports documented instructions,
addressing modes, file-local labels, explicit `EXPORT`/`EXTERN` linking, checked
expressions and common data/layout directives. Declared files link in manifest
order into one contiguous RAM image. `SECTION code` and the absolute, EQU-only
`_constants` section are supported. Includes, macros, arbitrary link sections and
standalone object/archive formats receive explicit diagnostics; desktop LWTOOLS
remains an explicit compatibility choice. No unsupported input silently invokes
an external tool. All assembly files together are limited to 1 MiB.
See the [assembler contract](../cpu/m6809/eightbit-m6809-core/README.md). A new
project defaults to load address `$7000` and entry symbol `START`; its schema-3
`dragon.toml` retains these settings and `run.boot = "machine-code"` in ZIP/GitHub
exports. New projects use `10 END` for the required desktop BASIC entry; imports
preserve the existing entry contents without running them for machine boot.
Explorer edits each declared assembly file, and source Debug uses file-specific,
assembler-derived current-build positions;
imported media uses an address breakpoint. Registers, disassembly and memory are
valid only while stopped. Editing, failed builds, Stop and stale replies cannot
silently launch an older executable.

Machine images and their entry must lie in `$0400–$7FFF`. Programmes owning most
of RAM use an original guest cassette bootstrap and enter with stack `$0300`;
initialise a safe stack before making calls. A programme replacing BASIC's
workspace cannot assume it can return to BASIC. Small routines which preserve
that workspace retain the ordinary BASIC `EXEC`/`RTS` route.

**Save work** offers project ZIP, explicit **Save to GitHub**, editable listing,
current playable game file and a separate local recovery copy. Ctrl/Cmd+S opens
these choices; Ctrl/Cmd+B checks/builds and Ctrl/Cmd+R chooses Play. Dialogues
contain keyboard focus, Escape cancels pending core work and restores focus.
Downloads request a save; check Files/Downloads for the final destination.
Files/iCloud behaviour still needs physical iPad acceptance.

**Continue with GitHub** is optional and works only after the backend is deployed
and the GitHub App registered. Opening this dialogue checks the existing session;
opening a local project does not make a GitHub request. Connect/install the App
on a selected private repository created with an initial README. Choose the
repository, then **Save to GitHub** or open a project as a new local copy. Saves
send only the selected source/settings/guidance snapshot. Later edits stay local until
another explicit save. Remote copies live under
`eightbit-studio/projects/<name>--<uuid>.json` on the repository's default branch.
This single file is saved conditionally using its previous GitHub blob SHA.
A conflict preserves local work; open the remote copy separately or choose
**Save a separate copy**. Cancelling stops waiting; a write already accepted by
GitHub may complete, so inspect the remote copy before retrying an uncertain save.
Private repositories with write access are required. Lists are paginated (30
repositories per page, up to 100 installations/100 pages per installation); a
repository collection is limited to 100 files. Requests have time/size limits,
redirects are rejected, and provider errors never echo tokens or private payloads.

Sessions use secure HttpOnly same-site cookies containing a random ID only.
GitHub tokens stay in a private session Durable Object, refresh serially, and
expire after 30 days (or earlier revocation). Disconnect deletes this device's
session; revoke the App separately in GitHub to remove all installations. OAuth
uses state, PKCE and a ten-minute encrypted authorisation cookie. Source, ROMs,
media and build results are never stored in the session service. The Worker
relays explicit project requests; it does not build or execute code. Local work
and downloads remain available when GitHub is disconnected or unconfigured.

For Spectrum 48K, **Play** and **Debug** use the original emulator and guest BASIC
firmware without user-supplied tools or ROMs. BASIC and single-source Z80 assembly
use the original Rust build engine; project ZIPs preserve the desktop manifest.
Computer setup retains an explicit MAME/supplied-ROM choice for BASIC. Other
Spectrum/ZX81 models retain their existing setup. The original 48K implementation
is still undergoing compatibility qualification under ADR 0058. Its guest editor
supports native keyword keys and literal command input, graphics, beeper and
standard cassette operations. The Cassette controls mount bounded TAP/PCM WAV and
record guest SAVE output to a checked TAP before requesting a download.

C64 PAL additionally offers **Original C64 (development)** in Project emulator
under ADR 0062. Selection is explicit; chips remains the initial choice. A current
checked BASIC build is recompiled and source/PRG hashes are checked before one
owned Rust/WASM guest session starts. The bundled original firmware supplies
BASIC, editor, KERNAL and character graphics. Reset cold-boots, Stop disposes the
worker and hidden-page handling releases input/audio. There is no host BASIC
execution, C64 debugger, attached drive or browser C64 assembler. Compatibility
limits are listed in the C64 specification. Corresponding source includes the
original firmware, build manifest, builder and this session adapter.

The retained external **Play** route requires a current checked BASIC build and user-selected firmware matching
the selected family’s existing revision fingerprints. The ROM dialogue lists the
required filenames and total size. Select individual files together or one combined
file in the displayed order. C64 uses the compact zlib-licensed chips core;
Sinclair and BBC use separate restricted MAME 0.289 WASM builds, and Dragon retains XRoar.
BBC uses OS 1.2, BASIC II, 32K and Acorn 8271/DNFS120. Select `os12.rom`,
`basic2.rom`, `dnfs120.rom`, `cm62024.bin` and extensionless `saa5050`, or their
combined bytes in that order. Speech data is required even with speech disabled.
The MAME WASM download is gzip-compressed and requires browser streaming
`DecompressionStream("gzip")` when the host has not already applied gzip content encoding; an unavailable runtime produces a visible failure.
BBC plays only generated single-sided DFS BASIC SSDs; recovery rejects arbitrary
disc contents and assembly. These adapters add no general disc, cartridge,
snapshot, debugger or assembly support. Decompressed WASM is limited to 64 MiB
for BBC and 48 MiB for Sinclair; each MAME runtime has a 256 MiB memory ceiling.

ROM bytes remain in volatile page/emulator memory; they never enter storage,
source downloads, logs or server requests. Remove ROM clears the retained image.
Switching computers invalidates the firmware receipt and requires new setup.
If the ROM is missing, Play opens its setup dialogue and explains the file needed.
Successful validation returns to the Game panel; choose Play again to start.
Closing pending setup cancels the check; no game starts automatically.
Run is explicit; Stop, source edits, closing the Game panel, page suspension
and unmount end its iframe session. The runtime has touch arrow/Space/Break
controls, ASCII keyboard entry and a separate Enable sound action. Hardware
keyboard and audio behaviour on iPad remain acceptance gates.

## Local build

Use Rust 1.92.0, Node 24, pnpm 11.5.0, wasm-bindgen CLI 0.2.127 and activated
Emscripten SDK 4.0.10 for XRoar/C64, plus a separate SDK 6.0.2 for MAME. Existing dependencies remain pinned in both lockfiles.
Emscripten and wasm-bindgen are developer tools, never downloaded by the app.

Provision the official source archive at
`.cache/xroar-web/xroar-1.12.1.tar.gz`:

```sh
mkdir -p .cache/xroar-web
curl --fail --location --proto '=https' --tlsv1.2 --max-time 120 \
  https://www.6809.org.uk/xroar/dl/xroar-1.12.1.tar.gz \
  -o .cache/xroar-web/xroar-1.12.1.tar.gz
shasum -a 256 .cache/xroar-web/xroar-1.12.1.tar.gz
```

Expected SHA-256:
`ba525225fbd732c4dbf2cbb571dc9d2810fd9dcedf0482133cf637e2eef61f88`.
The source bundle already includes this archive. Install the SDK using its
[official instructions](https://emscripten.org/docs/getting_started/downloads.html),
selecting `4.0.10`; do not modify shell startup files. Point `EMSDK` at its directory.
Install wasm-bindgen CLI version `0.2.127` through Cargo, or use its official
matching release; point `WASM_BINDGEN` at the executable. The development defaults
look in `.cache/browser-tools/` on Apple Silicon macOS.

Provision the additional official pinned sources before building:

```sh
mkdir -p .cache/mame-web
curl --fail --location --max-time 300 https://codeload.github.com/mamedev/mame/tar.gz/refs/tags/mame0289 -o .cache/mame-web/mame0289.tar.gz
curl --fail --location --max-time 120 https://codeload.github.com/floooh/chips/tar.gz/9e88298ce56319953ac7a43213a1120359f7a3a6 -o .cache/mame-web/chips-9e88298.tar.gz
```

MAME SHA-256: `0929cc749afabcef892900e10dd90bd8b05f94a7dde8f367ac6a5d2082589f84`.
Chips SHA-256: `fcae39995f61bfe461175047b11f16d147be0e7353c8a247ac503b5fd3a09b8d`.
Set `MAME_EMSDK` to the separately activated 6.0.2 SDK. It defaults to
.cache/browser-tools/emsdk-6.0.2. The separate MAME builds use only Sinclair or BBC
driver sources, SDL2,
software video, disabled HTTP/network adapters, and bounded 256 MiB memory.
The original patch exports readiness/natural-key input and explicit sound cleanup.
Resizable WASM buffer views are disabled for browser TextDecoder compatibility;
the adapter starts native main explicitly, cancels downloads, and rejects late
initialisation after Stop. Typed input has a 256-character queue, with at most one
additional queued natural-key character inside MAME.
The C64 adapter uses 16 MiB fixed memory and a bounded audio/key queue.

```sh
rustup target add --toolchain 1.92.0 wasm32-unknown-unknown
pnpm install --frozen-lockfile
bash scripts/browser/build-core.sh
bash scripts/browser/build-xroar.sh
bash scripts/browser/build-c64.sh
bash scripts/browser/build-mame.sh z80
bash scripts/browser/build-mame.sh bbc
node scripts/browser/check-core.mjs
node --test scripts/browser/verify-assets.test.mjs scripts/browser/runtime.test.mjs scripts/browser/classic-runtime.test.mjs
node scripts/browser/package-source.mjs
pnpm --dir apps/eightbit-studio-web/ui build
pnpm --dir apps/eightbit-studio-web/ui preview
```

Open `http://127.0.0.1:1425/studio/`. The build scripts use pinned tools and source digests. XRoar is built with the checked-in patch disabling missing-file network
fetches. No third-party ROM or upstream programme catalogue is copied. On a source-only
rebuild, run build-core, contract check, package-source and UI build again. Any
XRoar source/patch change also requires build-xroar. MAME source/patch changes
require rebuilding both restricted cores. The SDL canvas uses its required
`canvas` identifier; BBC autoboot passes a Lua-escaped carriage return to MAME,
while the native adapter uses `post_coded`. Do not publish an unmatched
old source archive with new binaries.

The browser core uses `target/browser-core` by default, separately from desktop
artefacts. `CARGO_TARGET_DIR` can override it. Complete browser builds/packaging
before starting the ordinary workspace gate, which stages and tests native tools.

The corresponding-source packer uses an explicit allow-list: browser application,
portable family libraries and scaffolds, shared workbench components, dialect tables,
locks, exact emulator/SDL port sources, Cargo dependency sources, JS package sources,
notices and build instructions. It excludes desktop hosts, credentials, ROMs,
user projects, recovery data, Git metadata and unrelated caches. A generated
source manifest and build receipt identify the snapshot and runtime hashes.
Rebuild the reduced workspace from an extracted archive before distribution.

**Computer setup** separates **System ROM** (built-in computer software needed to
play; editing and saving work without it) from **Open-source software**. The ROM
picker keeps the exact computer name in its accessible label; multi-ROM machines
offer **Choose ROM files…**. Validated ROMs are kept only until the page closes,
and **Remove ROM** clears them without starting the emulator.

The single **Source code on GitHub** link opens the exact release of the public
[`eightbit-studio-browser`](https://github.com/robtyers/eightbit-studio-browser)
repository matching this browser build. The repository contains the reduced
browser workspace with its own public snapshot history, README, build guide and
licences. The old generated/sources.html URL remains a small compatibility page
linking to the same release; it hosts no source downloads.

Download **browser-source.tar.gz**, **mame0289.tar.gz** and **SHA256SUMS** from that
release. Verify their SHA-256 values, extract the browser archive, and place the
MAME archive in its browser-source/.cache/mame-web/ folder. Follow
docs/browser-edition.md inside the extracted workspace. The main archive includes
the chips and XRoar archives, linked Emscripten port sources/notices, vendored Rust
dependencies, JavaScript package sources and all original adapters/build patches.
The MAME WASM runtime remains gzip-compressed below Cloudflare's 25 MiB asset limit;
complete source archives are hosted only as GitHub release assets.

Set a fresh versioned tag in packages/browser-workbench/src/source-release.json
for each source/runtime deployment. The source packer writes the six release
assets to .cache/xroar-web/github-release/&lt;tag&gt;/ and the schema-2 build receipt
to the runtime's generated folder. Schema-1 receipts remain verifiable for earlier
archives; new staging requires schema 2 and refuses source archive parts.
Publish the reduced workspace at the configured tag and upload all six assets:
browser-source.tar.gz, mame0289.tar.gz, COPYING.txt, NOTICE.txt, build-receipt.json
and SHA256SUMS. Then run:

```sh
node scripts/browser/verify-source-release.mjs
```

This performs unauthenticated public downloads, checks the repository/release
identity, streams every asset against its local size and SHA-256, and writes a
local publication proof. Requests have time/size/redirect bounds and accept only
GitHub-owned HTTPS hosts. Staging requires that proof to match the exact build
receipt and archive hashes; changed source or a missing/unverified release fails
before replacing local website output. Publish/verify source before removing the
older Cloudflare archive parts or promoting the new runtime.
The reduced source Cargo manifests explicitly omit native optional adapters and
the native-acceptance test feature. Run `cargo test -p eightbit-browser --lib`
there; the full checkout’s native-acceptance feature verifies desktop interchange.
The core remains buildable from the extracted source without desktop host code.

## Static hosting and rollback

The existing site uses an assets-only Cloudflare Worker. The browser build is
staged under `/studio/` in that asset output. Optional GitHub authorisation and
project transfer use the separate Worker described below. Opening the normal
welcome fetches only static assets; builds and emulation execute in disposable
local workers/iframes. There is no server execution API or analytics.
Static asset requests are visible to the hosting provider. Source/settings/guidance
are transferred only through explicit GitHub operations; ROMs and media are never
sent. No service worker or offline-install guarantee is claimed.

Stage only after local verification and source-bundle checks. Publishing, DNS
changes and production promotion require explicit approval. Keep the complete
previous asset deployment for provider rollback; runtime files and matching
source-release link must be deployed/rolled back together. Retain the public
GitHub releases and verify the matching source before rollback. Do not replace the desktop download
pages or claim a public browser release before hosted verification.

After the browser build, build the marketing website with its existing command,
then run `node scripts/browser/stage-website.mjs`. This stages only local assets
and adds scoped caching/content-type safety headers. It rejects symlinks,
unexpected files (including source archive parts and the disposable runtime smoke
harness), changed source inputs, mismatched source/runtime hashes, an unverified
GitHub source release and files exceeding Cloudflare's
[25 MiB individual asset limit](https://developers.cloudflare.com/workers/platform/limits/).
Rebuild the browser UI after any smoke harness before staging. Re-run staging
after a website rebuild, which replaces its generated asset directory.
Then run `pnpm --dir website check:static` to verify the combined static site.
The public runtime omits generated TypeScript declarations; source is provided
through the dedicated corresponding-source archive.

## Verification

```sh
cargo test -p eightbit-browser --features native-acceptance --lib
cargo clippy -p eightbit-browser --all-targets -- -D warnings
pnpm --dir packages/browser-workbench check
pnpm --dir packages/browser-workbench test
pnpm --dir apps/eightbit-studio-web/ui check
pnpm --dir apps/eightbit-studio-web/ui test
node scripts/browser/check-core.mjs
node scripts/verify-architecture.mjs
./scripts/check.sh
```

WEB-01 covers build/test/save and stale-build prevention; WEB-02 covers inert
BASIC/assembly recovery; WEB-03 covers explicit Run/Stop, firmware and lifecycle;
WEB-04 covers source/notices matching; WEB-05 requires Safari on a physical iPad,
Files/iCloud import/export, touch/hardware keyboards, audio, repeated Run/Stop,
background suspension and practical game execution. Mock tests, host WASM checks
and a desktop browser do not establish WEB-05 or firmware-gated runtime acceptance.

## Optional GitHub service — deployed 10 October 2026

The source is `apps/eightbit-studio-web/github-worker/index.ts`; its checked-in
Wrangler config contains the production route and public App identifiers, with
secret values held only in Cloudflare. It uses the already
pinned Wrangler 4.92.0 tooling; no GitHub SDK or new frontend dependency is added.
Type checking and mock boundary tests run with the browser app commands.

The user approved registration, secret storage, deployment and a private live-test
repository. [Eightbit Studio Online](https://github.com/apps/eightbit-studio-online)
is owned by `robtyers` (App ID `5265214`, public client ID
`Iv23liwtB9Ftah2hTxsY`) and can be installed by other accounts. It uses expiring
user tokens, web OAuth and callback `https://eightbitstudio.com/studio-api/callback`.
It requests only Contents read/write and Metadata read on selected repositories,
with no webhook. The initial installation selects only the private
`robtyers/eightbit-studio-browser-test` acceptance repository.

The `eightbit-studio-github` Worker stores `GITHUB_CLIENT_SECRET` and a randomly
generated 32-byte base64 `SESSION_KEY` as secrets, with the SQLite-backed
`GitHubSession` Durable Object bound as `SESSIONS`. Its route is only
`eightbitstudio.com/studio-api/*`, retaining the static site's
existing route and runtime CSP. The JSON API emits no CORS permissions and POST
requires the exact configured Origin, JSON content type and studio request header.
Never put secrets in `VITE_` settings, browser code, Git or corresponding source.
User credentials stay in opaque server sessions for at most 30 days; disconnect
deletes the current device's session. GitHub saves remain explicit and include
only the bounded source/settings/guidance snapshot. GitHub requests use manual
redirect handling and reject every redirect before reading its body; Cloudflare's
workerd runtime does not support `redirect: "error"`.

After approval for a future deployment, use the checked-in config and read back
the deployed version and secret names. Secret values must be supplied privately,
without command arguments, logs or checked-in files.

```sh
WRANGLER_SEND_METRICS=false pnpm --dir website exec wrangler deploy --config ../apps/eightbit-studio-web/github-worker/wrangler.jsonc
WRANGLER_SEND_METRICS=false pnpm --dir website exec wrangler deployments list --config ../apps/eightbit-studio-web/github-worker/wrangler.jsonc
WRANGLER_SEND_METRICS=false pnpm --dir website exec wrangler secret list --config ../apps/eightbit-studio-web/github-worker/wrangler.jsonc
```

For local development, run Wrangler **locally**, with private `.dev.vars` values
and `APP_ORIGIN=http://127.0.0.1:1425`, `ALLOW_LOCAL_HTTP=true`; production rejects
HTTP. Proxy `/studio-api/` from Vite to `127.0.0.1:1426` by setting
`STUDIO_GITHUB_PROXY=http://127.0.0.1:1426`. This is a dev-only server setting and
is not emitted into the browser bundle. A real GitHub callback still requires a
separately registered test App and an approved user-operated account journey.

```sh
WRANGLER_SEND_METRICS=false pnpm --dir website exec wrangler dev --local --port 1426 --config ../apps/eightbit-studio-web/github-worker/wrangler.jsonc
STUDIO_GITHUB_PROXY=http://127.0.0.1:1426 pnpm --dir apps/eightbit-studio-web/ui dev
```

The dated [verification record](testing.md#github-service-handover--10-october-2026)
distinguishes live account checks from mock refresh/expiry/revocation coverage and
physical-device acceptance. Use a selected private test repository for future
login, save, reopen, conflict and disconnect checks; never use user ROMs or existing
projects as fixtures. Check cookie flags, credential/source logging and provider
failure recovery. Session storage and API traffic use the existing Cloudflare plan;
no plan upgrade was approved or performed.

Rollback requires approval to remove this Worker's API route, retaining the static
site and local authoring. To deploy that rollback repeatably, remove the `routes`
entry from the GitHub Worker config and run its deployment command above. Do not
delete the static `eightbit-studio` Worker or its assets. Restoring the approved
route/config and deploying re-enables the service; rotating `SESSION_KEY` also
invalidates pending OAuth requests and needs its own planned credential handling.

## Original Vectrex development — ADR 0063

New Project offers Vectrex assembly and the original Vector, sound and controls
starter. Check & build uses the original 6809 assembler; Play uses the same
family-owned guest executive/board as native, without a firmware upload. Download
stock `.bin` cartridges and ZIPs containing schema-1 vectrex.toml. The build receipt
validates cartridge structure; it does not claim a BASIC listing round trip.
See docs/vectrex-original-specification.md for the exact executive subset, input
keys and qualification limits. Corresponding source includes that crate, original
executive and project templates. Local changes do not publish the browser edition.

Original Vectrex profile 0.3 adds bounded packet-list guest services. Native/WASM
qualification covers the optional `packet-demo.asm` alongside the unchanged
starter and text/counted-list example; public distribution still requires a
matching new corresponding-source archive and explicit publication authority.

Original Vectrex profile 0.4 adds two bounded text-list formats. Its optional
`text-list-demo.asm` joins the native/WASM parity fixtures; it does not replace
the starter or imply the neighbouring $F38A entry or historical font/timing support.
The same corresponding-source and explicit publication boundaries apply.


Original Vectrex profile 0.5 adds three bounded patterned-vector services. The
optional `pattern-demo.asm` joins native/WASM parity, including malformed-pattern
failure. The same guest executive and source-bound identity are used; Play stays
explicit and local. Public release/source-archive publication and physical,
Linux graphical and Windows acceptance remain separate gates.
