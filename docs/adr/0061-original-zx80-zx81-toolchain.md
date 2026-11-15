# ADR 0061: original ZX80 and ZX81 development toolchain

- **Status:** Accepted for implementation by the user on 20 October 2026; qualification pending.
- **Extends:** ADRs 0041, 0053 and 0058; preserves ADRs 0032, 0043 and 0045.

The user requests the same independent desktop/browser development workflow as
Dragon and Spectrum: original emulator, guest firmware and source assembler,
with no mandatory separately installed emulator, assembler or supplied ROM.
Existing explicit MAME/SjASMPlus choices remain effective. This instruction does
not authorise a commit, push, deployment or public release.

Reuse the existing Z80 processor and assembler. Move independently authored,
machine-neutral guest arithmetic into `cpu/z80/firmware/original`; both machine
families may compose it with their own ABI constants and guest services. No
family may depend on another family. Shared numeric extraction must preserve
the Spectrum image/behaviour, with fresh regression evidence.

The existing `zx81` family identity, `zx81.toml` schema, target identifiers and P
images remain supported. Extend that family to its direct predecessor, retaining
its internal compatibility name while identifying ZX80 and ZX81 explicitly in
target selection, dialects, firmware, media and machine windows. Do not interpret
ZX80 integer BASIC as ZX81 floating-point BASIC or import one model's image as
the other. Preserve the original ZX80 4K dialect and the supported ZX81 dialect;
an upgraded ZX80 with ZX81 firmware is a separate target, not an inferred mode.
The exact PAL RAM profiles are recorded in the specification after confirmation.

Original firmware storage is bounded but need not fit in 4/8 KiB. Private ROM
pages may occupy an original-ROM alias window only while authorised firmware
gates select them. Normal public ROM/RAM decoding and physical 1/16 KiB RAM must
remain model-correct; no host BASIC evaluator, hidden guest workspace or extra
public RAM is permitted. Reset, interrupts, public calls, errors and debugging
must preserve page ownership. Firmware manifests identify source, assembler,
builder, image and interfaces. Only verified original firmware may be bundled;
third-party ROMs remain lawful, exact-hash, opt-in test/runtime inputs.

Implement display-file fetching, character/refresh video, inverse characters,
keyboard matrix, interrupts, ZX81 NMI/SLOW/FAST and cassette signals from
manufacturer facts and original tests. A default no-op refresh observation may
extend the shared CPU bus without changing existing boards. BASIC and graphics
execute in guest Z80 code. Public memory structures, native token/character
formats, integer/floating arithmetic, strings, arrays, control flow, INPUT and
error recovery remain machine-specific contracts. Stock-machine exports must
not depend on Studio-only guest facilities. No analogue television/electrical
accuracy or arbitrary historical ROM-internal compatibility is inferred.

Reuse the shared workbench, bounded portable sessions, fresh-build/debug rules,
native auxiliary-window ownership and browser worker lifecycle. Do not redesign
the UI. Default BASIC/assembly Build, Run and Debug become available only with
implemented guest behaviour and boundary verification. Unsupported external
debugging does not become available merely by adding the original runtime.

Qualification requires focused CPU/hardware/language/media tests, malformed and
memory-boundary cases, cancellation/reset/stale-state recovery, source/byte-bound
debugging, actual native/WASM parity, browser/native journeys, matching source
archives, the ordinary gate and affected macOS packages. Supplied-ROM MAME checks,
physical cassette/gameplay, heard audio, Linux/Windows packaging and public
release are separate evidence classes. A monitor or partial BASIC checkpoint
does not complete this scope. Do not copy or translate third-party ROM listings
or emulator/interpreter implementation code.
