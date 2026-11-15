# Original Vectrex runtime

ADR 0063 authorises a first-party development runtime, executive firmware and
cartridge assembler, preserving the existing shared workbench and VecX alternative.
This specification tracks implementation and qualification, not universal legacy
cartridge compatibility or physical-console acceptance.

## Contracts

- VXO-01: original 6809 CPU; bounded 32 KiB cartridge, 1 KiB mirrored physical RAM,
  VIA register mapping and immutable independently authored 8 KiB firmware.
- VXO-02: guest VIA timer/IRQ, DAC/multiplexer, vector integration/blanking,
  PSG tone/noise/envelope and two controller ports, with bounded display/audio.
- VXO-03: source-bound original executive; guest boot parses the conventional
  cartridge header and jumps to cartridge code. Published entry points execute
  actual instructions; unavailable services must not silently succeed.
- VXO-04: original assembly, structural cartridge validation and atomic build
  publication; existing project/export contracts and explicit LWTOOLS retained.
- VXO-05: one bounded cancellable identity-bound session shared by native/WASM;
  reset/release/stop, input, failure and recovery use the shared console/presenter.
- VXO-06: explicit original Run, standard source/export preservation, inert opening,
  stale-build rejection, optional VecX and source-complete browser composition.

## Provenance

Original implementation only. Technical facts: the manufacturer Vectrex
Programmer's Manual volume I (hardware description, register and executive RAM
maps), https://www.vectrex.de/wp-content/uploads/2013/12/vecprogman_v1.pdf;
the manufacturer's volume II executive contracts,
https://www.vectrex.de/wp-content/uploads/2013/12/vecprogman_v2.pdf;
public call names/addresses indexed by
https://www.playvectrex.com/designit/chrissalo/appendixa.htm. Do not consult or
translate ROM disassemblies or emulator implementation code.

## Qualification

Record implemented executive entries, device timing limits and exact commands in
this document and dated evidence in docs/testing.md. A demo or process launch does
not qualify full executive coverage, analogue hardware or historical games.

## Implemented development profile 0.5

The portable family crate is `eightbit-vectrex-emulator`; it depends on the shared
original CPU/assembler, serialisation and hashing only. Desktop project/filesystem
and process services remain in the existing family core/desktop adapters.

The source-assembled original executive fits the stock 8 KiB ROM. Its exact public
entries and exclusions are in `families/vectrex/firmware/original/README.md`, with
source/image hashes in `manifest.json`. The original starter builds to the same
204-byte cartridge as LWTOOLS 4.25. Existing valid source paths and schema-1 project
manifests are retained; the original assembler adds FCS and raw-image assembly
without reserving a synthetic user label. Build publishes the same four files
atomically. **Build with LWTOOLS** retains the explicit external build route.

The board implements 1 KiB RAM mirrored through $C800–$CFFF, read-only cartridge
and executive windows, mirrored VIA registers, T1 one-shot/free-running and T2
interval counting/IRQ, port directions and joystick comparator, manual CA2/CB2
reset/blanking, internally clocked output shift modes and idealised DAC sample/hold
integration. The PSG supplies three tone channels, noise and envelopes plus two
four-button/digital-axis controllers. PCM is 44.1 kHz; the display is an original
256 × 320 monochrome rasterisation of vector motion. Each advance is at most
120,000 cycles and retained PCM is capped at 8,192 samples. Per-frame pixel size,
firmware identity, schema and session are validated at the presentation boundary.

Pin-level handshake/external shift/PB6 pulse-count timing, analogue calibration,
phosphor persistence, precise intensity/loudness transfer functions, historical
condition flags, exact device postconditions and all unlisted BIOS services remain
unqualified or unimplemented. Missing instructions/services produce VXO1001;
there is no host service trap or silent substitute. No universal cartridge,
physical hardware, debugger, peripheral or public-release compatibility is claimed.

Desktop uses explicit **Run original (development)** for the current project or
opened cartridge, sharing Stop/Restart, rebuild-on-save and the existing native
embedded/separate-window presenter. Ordinary Run retains RetroArch/VecX. Built-in
Build/Run do not probe external executables or load a third-party ROM. Original
selection is session-local and does not change project or runtime settings schemas.

The browser adds Vectrex assembly projects to the existing shared workbench. Build
returns a cartridge-structure receipt rather than a BASIC round-trip claim; Play
reassembles and checks source/cartridge SHA-256 in the family session. ZIPs retain
standard vectrex.toml, the configured source and supporting documents. The same
bounded Rust session is exported through VectrexOriginalRuntime in WASM. Source is
at most 128 KiB; its JSON-escaped launch envelope is at most 1 MiB. The
corresponding-source archive includes the crate, original firmware/templates and
this contract. No deployment or GitHub remote write is part of local verification.

Keyboard: controller 1 uses arrows and Z/X/C/V for buttons 1–4. Controller 2 uses
A/D/W/S and F/G/H/J. Opposing directions produce neutral; blur, Stop, Reset and
surface changes release held controls. Audio activation follows the shared console's
explicit user gesture. This keyboard mapping does not qualify analogue controllers.

## Executive conformance continuation

Profile `eightbit-vectrex-original-development-0.2` corrects the documented frame,
button, intensity, sound and line-pointer effects and adds single-string text and
counted draw/move vector lists. The complete entry and RAM contract, corrections
from 0.1, original font geometry and unsupported services are published in the
[executive guide](../families/vectrex/firmware/original/README.md). The existing
starter cartridge stays unchanged. The optional `executive-demo.asm` exercises
text and both list forms. Register/RAM conformance tests use independently authored
cartridges; no historical firmware or game is an ordinary fixture.

All text and list processing executes guest instructions against VIA hardware;
there are no host drawing/text traps and no extra RAM reservation. Strings are
limited to 256 supported characters and a terminator, counts to 256 vector pairs.
The text font, spacing and timing are original. Rendering may straddle fixed
display frames; a single capture can contain a partial scene without CRT persistence.
Historical condition flags/timing, exact VIA postconditions, interrupt callbacks,
grid lists, unlisted patterned/text entries, music/score/math and historical ROM tables
remain unqualified or absent. The presenter accepts the explicit 0.1–0.5
identities; new sessions emit 0.5. This does not promote default Run, change project
schemas or qualify packaged/Linux/Windows/physical playback from source tests.

## Packet-list continuation

Profile `eightbit-vectrex-original-development-0.3` retains the preceding executive
and adds PACK2X/PACK1X/LPACK/TPACK/PACKET at $F404/$F408/$F40C/$F40E/$F410.
The five calls use guest VIA motions, signed Y/X triples and a strict mode
allow-list, with no host call traps, extra RAM or third-party implementation.
The executive guide owns the exact scale, return-pointer, 256-vector and
address-wrap bounds. `packet-demo.asm` exercises every entry through the native
and WASM session. It is optional; both earlier example sources are unchanged.
The manufacturer describes X as the end of the list; this development contract
leaves it on the terminator. Exact historical pointer/flags/timing and
physical/packaged playback require independent qualification. New firmware is
bound to new source/image hashes; older evidence is retained as historical.

## Text-list continuation

Profile `eightbit-vectrex-original-development-0.4` adds the manufacturer-documented
TXTSIZ ($F385) and TXTPOS ($F38C). It scans each record in guest instructions and
uses the existing original-font renderer/VIA motions. No host text traps, private
pages, reserved RAM or new dependency is introduced. The exact leading-zero,
return-pointer, empty-list, 64-message and 256-character contracts are in the
executive guide. Working state uses S; only public text size/pointer bytes change.
The adjacent $F38A remains unsupported because these manufacturer references do
not specify its entry contract. No historical alias is assumed.

`text-list-demo.asm` is an optional original fixture for both formats, independent
of the unchanged starter, executive and packet examples. Tests cover equivalent
single-string pixels, RAM/register effects, limits, malformed records, wrapping,
cancellation in validation/rendering, fault/release/Reset/Stop and actual
native/WASM session parity. Font, flags, timing and broad historical compatibility
remain independently unqualified. New source/image identities do not inherit
packaged, Linux/Windows or physical acceptance from older profiles.

## Pattern-list continuation

Profile `eightbit-vectrex-original-development-0.5` adds DSHDF ($F434), DASHDF
($F437) and DASHPK ($F46E), using original guest instructions and the existing
VIA shift-register model. DASH/$C829 is reloaded per patterned vector; no host
traps, reserved RAM, private pages, dependencies or project schema changes occur.
The executive guide owns the exact counted/patterned packet formats, bounds,
return pointers, original bit phase and preserved registers/configuration. T2
is not reconfigured. Non-zero ZSKIP consistently zeros after a whole list.

`pattern-demo.asm` is an optional 95-byte original fixture; earlier examples and
the default remain unchanged. Tests cover pattern/blank/solid output, signed
geometry, counts, mode/wrap failures, public state, cancellation during motion,
fault recovery and actual native/WASM parity. $F433/$F439 remain unsupported.
The manual's historical zeroing inconsistency is not reproduced; exact dash phase,
analogue/CRT, broad game compatibility and physical acceptance remain unqualified.
Source, browser/native playback and packaged evidence remain separate gates.
