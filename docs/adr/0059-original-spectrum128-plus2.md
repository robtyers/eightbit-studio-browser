# ADR 0059: reuse the original Spectrum runtime for 128K and grey +2

- **Status:** Accepted; local software qualification completed on 20 October 2026. Physical acceptance and public release remain separate.
- **Extends:** ADR 0058. Preserves the shared workbench, external MAME/SjASMPlus routes and existing project contracts.

The user approved extending our original emulator, firmware and assembler to the
UK Spectrum 128K and original grey UK +2, with reuse as the priority. The +2A,
+2B and +3 are separate machines and are not included. There is no historical
firmware-size constraint and no permission to distribute third-party ROMs.

Use one Z80 core, assembler, digital board, BASIC implementation and portable
session. An explicit family-owned model selects 48K or 128K memory, timing and
peripherals; the grey +2 shares the 128K implementation. Keep the existing 48K
constructors and contracts. Model selection must flow from the validated project,
never from a source-controlled executable or host evaluator.

The expanded targets require eight physical 16 KiB RAM banks, public paging and
lock semantics, alternate display bank, AY sound and model timing. Private
firmware pages remain implementation storage and must not collide with public
paging or sound registers. Guest Z80 code implements PLAY, SPECTRUM, RAM-disc
commands and model-specific keyboard behaviour by reusing the original BASIC
engine. Neither RAM-disc files nor music may be interpreted by the host.

Normal desktop and browser BASIC/assembly Build, Run and Debug shall use the
original implementation without configuration when no external tool is selected.
Existing explicit external settings remain effective. The source assembler and
standard cassette loader are reused; this does not introduce a banked object
linker, new project schema, historical menu replica, physical replacement ROM,
MIDI/serial peripheral or universally qualified software compatibility.

Qualification must cover bank aliasing, lock/reset, screen selection, timing,
AY registers/tone/noise/envelopes, guest extended BASIC success/error/recovery,
cassette and assembly execution, source-bound debugging and native/WASM parity.
Run the ordinary gate and affected packaged products. Physical hardware,
audible playback, platform packaging and public release claims require their
own observed evidence. No commit, push or deployment is authorised by this ADR.

Technical references are manufacturer documentation only: Sinclair's Spectrum
128 introductory/service manuals, Amstrad's original +2 service manual and the
General Instrument AY-3-8910/8912 data manual. No reference implementation or ROM
disassembly is used. Original generated tests are retained in the family crates.
