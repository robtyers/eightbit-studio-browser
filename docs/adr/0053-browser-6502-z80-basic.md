# ADR 0053: browser BASIC support for 6502 and Z80

- **Status:** Implementation scope approved by the user on 10 October 2026.
- **Extends:** ADRs 0032, 0046 and 0052, for the browser edition only.

The browser application composes Dragon 32, C64 PAL, Spectrum 16K/48K/128K UK/+2
UK and ZX81 1K/16K. Reuse one shared browser workbench. Family-owned Rust remains
responsible for BASIC tokens, media, memory limits, firmware and project semantics.
The pure `eightbit-source-core` crate shares source-project types/parser/scaffold
with the native `eightbit-source` wrapper. Default family `native` features retain
existing desktop services; portable builds exclude process, filesystem-host,
debugger and AI dependencies. Pure manifest
validation and scaffold generation are shared with the existing desktop loader
and creator. No desktop project schema changes are authorised.

The first increment supports BASIC creation/editing, bounded project import/export,
local recovery, explicit optional GitHub copies, builds, media round-trip checks,
listing recovery from supported PRG/TAP/P subsets, and explicit local emulation.
Assembly compilation and additional debugging remain subsequent work. Existing
Dragon machine-code recovery remains inspection-only. Tests/checks do not establish
BASIC ROM syntax acceptance or gameplay correctness.

Browser snapshot schema 2 adds the exact target identity. Schema 1 remains readable
as Dragon 32. Desktop manifests and source/support files are authoritative over
stale ZIP metadata. Reject mixed manifests, unsupported projects, extra files,
malformed media and machine-code tails rather than silently simplifying them.
Generated defaults must match their desktop product byte for byte, including
AGENTS, CLAUDE, README, ignore and test guidance.

Use the restricted MAME 0.289 WebAssembly build for Sinclair, using pinned source and
an original bounded natural-keyboard adapter. The C64 MAME driver additionally
requires PLA firmware beyond the existing desktop contract. Use the zlib-licensed
`floooh/chips` C64 core at commit 9e88298ce56319953ac7a43213a1120359f7a3a6 for
the existing three-ROM PAL contract. This browser runtime choice does not change desktop VICE.
Ship matching emulator source, local adapters, build instructions and notices.
Keep the approved GPL browser distribution and Apache notices on reused code.
Native tools remain separately installed and supervised; this exception permits
only the browser runtime builds. New upstream code never enters native authoring.

Firmware is supplied explicitly through Files, validated against the family-owned
revision fingerprints, and kept in volatile memory only. C64 accepts the desktop
BASIC/KERNAL/character ROM set. Sinclair uses the existing exact model hashes and
ROM splitting contracts. No firmware download, catalogue, upload, storage or export.
Emulators run in disposable same-origin documents, load only fixed static assets,
have no project-defined command/script or network authority, and stop on edits,
replacement, hidden-page suspension, cancellation and unmount. Late results and
old builds cannot launch. Audio and keyboard/touch input remain user controlled.

Source changes, host/WASM/mock evidence, ROM-gated gameplay and physical iPad
acceptance are separate gates. Publication, provider configuration and deployment
remain separately authorised. Measure runtime and source asset sizes against the
Cloudflare file limit; split corresponding source archives when required.
