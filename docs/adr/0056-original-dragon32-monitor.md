# ADR 0056: original Dragon 32 emulator and monitor firmware

- **Status:** Implementation authorised by the user on 14 October 2026.
- **Extends:** ADRs 0032, 0045 and 0052; phases 0–4 only.

## Decision

Implement an original, bounded MC6809E execution library and Dragon 32 PAL
machine library, shared by native and WebAssembly adapters. Ship an original
16 KiB monitor ROM, its 6809 assembly source, reproducible build instructions,
licence and exact digest. This is a scoped exception to the prohibition on
bundling firmware: only this project's original monitor is authorised. Original
Dragon, DragonDOS and other third-party firmware remains excluded.

The first-party runtime is explicitly monitor-only. It does not execute BASIC,
advertise full CPU coverage, or establish historical game compatibility.
The existing XRoar backend, exact-ROM validation, settings, project schemas,
builds and debugging remain available without migration. Monitor boot is an
explicit user action independent of a BASIC build; opening a project is inert.

The CPU crate belongs to `cpu/m6809`; Dragon wiring and monitor firmware belong
to `families/dragon`. Neither core has filesystem, network, process, credentials
or frontend authority. Guest execution uses bounded slices and fixed guest
memory. Native and WASM hosts share the same machine implementation.

Preserve the existing shared workbench. Add monitor boot alongside existing
emulator setup/lifecycle actions. Native output uses a separate application-owned
window, preserving ADR 0045. Browser output uses the existing emulator surface.
No embedded desktop preview, editor splitting or UI redesign is authorised.

Each host owns one monitor session. Session identities reject stale commands and
results. Stop, reset, focus loss, window destruction and application shutdown
release input and execution state. Runtime data remains volatile. No monitor
command can perform host I/O. Host-assisted input represents explicit user
keyboard action, not a terminal or AI capability.

The product's custom-command router permits only `monitor_command` from the
monitor webview. Its Tauri plugin capability permits only closing that window
and applying the existing appearance preference. Auxiliary-window destruction
must not shut down main-window Git, terminal, AI or family runtime services.

## Provenance and licensing

Original emulator, monitor and glyph source is Apache-2.0. Existing browser GPL
obligations remain unchanged. No emulator implementation, ROM disassembly,
recovered BASIC implementation or third-party glyph table is used. Only factual
interfaces are taken from the references recorded in the specification. LWTOOLS
is an explicitly invoked, separately installed development tool; its code is
not linked or distributed. The monitor's source and binary are first-party
release material, not disposable caches.

## Qualification

The authoritative scope, reference register and acceptance IDs are in
`docs/dragon32-monitor-specification.md`. Ordinary tests use original firmware
and original programs. Native/WASM parity, packaged desktop, browser lifecycle,
physical iPad, independent-emulator and physical Dragon observations are distinct
gates. No publication, signing, notarisation or deployment is authorised here.

Full instruction coverage, detailed PAL/video/audio fidelity, BASIC, ROM ABI
compatibility, Dragon 64, disk controllers and historical software qualification
remain later phases. Unsupported CPU instructions stop with a stable diagnostic;
they are never guessed or silently treated as no-ops.
