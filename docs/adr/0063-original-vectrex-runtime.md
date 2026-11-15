# ADR 0063: original Vectrex machine, executive and cartridge toolchain

- **Status:** User-authorised implementation, 23 October 2026
- **Extends:** ADRs 0034, 0043, 0045 and 0053

Implement an independently authored Vectrex runtime using the shared original
6809 CPU and assembler. Vectrex owns its board, VIA timers/interrupts, vector
integrators/display, PSG, controller ports, executive firmware and portable
bounded session. Firmware executes as guest 6809 instructions through machine
registers; no host BIOS-call traps, host game execution or borrowed firmware.
Manufacturer/interface facts are references, never implementation source.

Use the existing stock 32 KiB cartridge contract, 1 KiB public physical RAM and
8 KiB executive ROM window. No private firmware pages or reserved cartridge/RAM
space are authorised. Independently authored source assembled by our own assembler
is the only Vectrex firmware-bundling exception. Do not include Mine Storm,
commercial cartridges, external firmware or third-party emulator implementation.
Unsupported executive entries/CPU encodings fail explicitly; publish the exact
implemented API and timing limits, rather than implying universal compatibility.

Keep schema-1 vectrex.toml, cartridge exports and external RetroArch/VecX/LWTOOLS
routes. Built-in assembly is available without installed tools. Original Run is
an explicit development action in the shared Emulator panel and uses its existing
presenter, console, audio/input and embedded/separate-window contract. It does not
promote the original runtime to the ordinary compatibility default. Preserve
family-neutral shared services and family-owned input/build/firmware validation.

Native and browser hosts compose the same portable session and original source
builds. Inputs are schema/version/session/revision bound and work/output is bounded;
Stop, Reset, release, cancellation, stale launch, unsupported code and recovery are
explicit paths. Opening a project remains inert. Browser execution remains local
and explicit; preserve the GPL browser edition's matching corresponding source.

Qualify CPU/device facts, guest executive APIs, original demo vectors/input/audio,
assembler byte parity, native/WASM parity, native presenter and packaged products
separately. Existing VecX results do not qualify original execution. Broad legacy
cartridge, analogue/CRT, physical-console, peripherals, debugger and public release
claims require their own evidence. No commit, push, signing or publication is
included in this approval. See the Vectrex specification and docs/testing.md.

## Executive conformance continuation — 23 October 2026

The user approved the next executive increment: check the existing public ABI,
correct documented register/RAM effects and add text and counted vector-list
services. Development profile 0.2 identifies the changed firmware separately from
0.1. Preserve schema-1 cartridges and explicit development selection; the presenter
may accept either known development identity. Text uses an independently drawn
font and guest VIA operations, never a historical character table or host text
trap. This increment does not promote the default runtime or qualify historical
condition flags, exact instruction timing, historical games or physical hardware.

## Packet-list continuation — 23 October 2026

The user authorised committing the verified checkpoint and continuing the original
executive. Profile 0.3 adds the five packet-list entry points, implemented as guest
instructions using the existing VIA motion primitives. Accept only documented
blank/solid/terminator modes, at most 256 vectors and no address-space wrap.
Temporary state stays on the guest stack. Publish the precise original return
pointer and bounds; these are development contracts, not historical timing or
condition-flag qualification. Preserve known 0.1/0.2 presenter identities. Text
lists, patterns, grids, music and mathematics remain separate continuations.

## Text-list continuation — 23 October 2026

The user authorised committing profile 0.3 and continuing with text lists.
Profile 0.4 adds the manufacturer's documented TXTSIZ ($F385) and TXTPOS
($F38C), with original guest validation and the existing original-font renderer.
Use a leading zero byte as the list terminator, at most 64 messages of at most
256 characters each, and reject 16-bit pointer wrapping. Working state remains
on S; only published text size/pointer RAM may change. Publish empty-list effects
and exact return pointers. Preserve all known presenter identities, ordinary Run,
project schemas and external routes. The neighbouring $F38A entry has no contract
in these manufacturer references and remains unsupported; do not invent an alias.
Historical font/timing/flags, patterns, grids, music, mathematics and physical
acceptance remain separate.

## Pattern-list continuation — 24 October 2026

The user authorised committing profile 0.4 and continuing with patterned vectors.
Profile 0.5 adds DSHDF ($F434), DASHDF ($F437) and DASHPK ($F46E) as original
6809 guest instructions. Counted lists accept 1–256 signed pairs; packets accept
0–256 vectors with distinct blank/solid/dashed modes and a terminator. Reject
pointer wrapping and malformed packets. DASH/$C829 is read without modification;
only LIST and the caller's S stack hold working state. Non-zero ZSKIP consistently
zeros on completion, rather than reproducing the manual's unspecified inconsistency.

Pattern output uses the existing VIA shift-register model, reloaded for each
vector, with the current T2 low latch controlling the original bit period. Preserve
T1 scale, T2 configuration, Y/U/DP and the incoming ACR; return blanked. Exact
historical dash phase, analogue output and VIA timing remain unqualified. Keep
DSHDF1 ($F433) and the undocumented neighbouring $F439 unsupported in this
increment; no alias or zero-count semantics are inferred. Preserve known presenter
identities, schema-1 cartridges, the shared workbench and ordinary external Run.
