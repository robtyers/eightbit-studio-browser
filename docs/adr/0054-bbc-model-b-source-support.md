# ADR 0054: BBC Model B in Eightbit Studio 6502

- **Status:** Accepted implementation scope under the user's request of 11 October 2026
- **Date:** 11 October 2026
- **Extends:** ADRs 0032, 0043, 0046 and 0045

Add the Acorn family to Eightbit Studio 6502 with one exact target: BBC Micro
Model B, 32 KiB RAM, UK OS 1.2, BASIC II, Acorn DFS 1.20/NFS 3.00 and the 8271 disc
controller. Keep the existing shared workbench, C64 projects and product identity.
`bbc.toml` is an independent schema-1 source-project manifest. Reject ambiguous
roots across every desktop product. No cross-family project conversion is added.

Implement an original bounded BASIC II tokeniser, deterministic 80-track,
single-sided DFS SSD images, and bounded single-source ca65/ld65 assembly using
the existing NMOS 6502 tool adapter. Assembly starts at $1900, remains below
$3000 and returns through RTS; the boot script selects MODE 7 before running it.
BASIC uses the same conservative $3000 programme ceiling and runs through CHAIN.
The BBC's own BASIC interpreter remains responsible for full runtime syntax,
including its native inline assembler. No new IDE source debugger is claimed.

Use separately installed MAME 0.289 with the `bbcb` driver. The b2 candidate's
documented default launch can forward a disc to an existing emulator process;
MAME fits the existing owned-process and private-directory supervision contract.
This is a process adapter, not a licence exception: no emulator implementation,
firmware or commercial programme is linked, copied, downloaded or bundled.

Validate exact OS, BASIC, combined DFS/NFS, speech-data and character-generator
revision fingerprints and sizes. MAME requires the speech-data ROM even with
the optional speech device disabled. Stage only
those files and the freshly built disc into a private directory, and compare
framed SHA-256 identities before launch. Disable optional speech, printer,
network and expansion devices; use fixed arguments and an app-owned boot script.
All writable emulator paths remain inside the disposable run directory.
Retain cancellation, timeouts, stale-build rejection and owned-process cleanup.

Source settings schema 3 adds a bounded family-keyed `emulators` map. Schema 2
and its existing `emulator` field remain readable; the legacy field belongs only
to the application's original first family. BBC never falls back to VICE.
Existing Z80 and C64 settings retain their values. Media limits become explicit
compiled profile values: existing targets retain 64 KiB; BBC SSD is 200 KiB.

Automated evidence must cover exact tokens and disc catalogue metadata,
malformed/boundary source, family routing, settings compatibility, cancellation,
failed-build preservation, guarded export and emulator staging. Real firmware,
emulator video/input/audio and packaged platform acceptance remain distinct gates.
BBC browser support, other models, tapes, arbitrary disc import, multi-file
builds, 65C02 extensions and integrated debugging are outside this increment.

Technical references are public format and process contracts, not implementation
sources: the [BASIC ROM User Guide](https://wiki.retrotechcollection.com/images/d/d8/BBC_Micro_BASIC_ROM_User_Guide.pdf)
sections 2.3–2.4 (keyword/line format facts), the
[Acorn DFS User Guide](https://www.retro-kit.co.uk/page.cfm/content/Disc-Filing-System-User-Guide/index.html)
(catalogue documentation), and MAME 0.289 `-listxml bbcb` metadata and
[command-line documentation](https://docs.mamedev.org/commandline/commandline-all.html).
All shipped code, examples and ordinary fixtures are original.
