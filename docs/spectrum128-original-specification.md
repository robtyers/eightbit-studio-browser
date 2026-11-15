# Original UK Spectrum 128K and grey +2 development toolchain

Status: local software acceptance under ADR 0059, 20 October 2026.
The user confirms UK 128K and the original grey UK +2 only. This extends the
[48K specification](spectrum48-original-specification.md); its compatibility
limits still apply. Publication and physical acceptance are separate actions.

## Shared implementation

| Area | Reused implementation | Extension |
| --- | --- | --- |
| Processor and assembler | `eightbit-z80-core` | No second CPU or assembler |
| Board | `eightbit-spectrum-emulator` | Closed model, banks, timing and AY |
| Guest BASIC | Common ROM and existing 24 pages | Two pages for PLAY and RAM-disc transfers |
| Files | Original cassette header/array/MERGE engine | RAM-disc byte transport |
| Runtime/debugger | `original_session::Session` | Validated target and model frame identity/timing |
| Desktop/browser | Existing adapters, workers and workbench | Model-aware selection; no separate shell |

There is one firmware image. Its expanded private pages remain implementation
storage, independent of the public RAM banks and ROM-mode selection. The old
48K constructors, schemas, diagnostics, UI and explicit external routes remain.
No host BASIC, music interpreter, RAM-disc filesystem or guest-memory injection
is introduced. No new third-party dependency, ROM or implementation is bundled.

## Requirements

- **S128-01:** UK 128K and original UK +2 share eight physical 16 KiB banks,
  fixed banks 5/2, public `$7FFD` partial decoding, ROM/memory selection and
  paging lock until Reset. Each bank remains independently writable. The two
  public ROM modes reuse our original BASIC; arbitrary historical internal ROM
  addresses are not a replacement-firmware contract.
- **S128-02:** Display and floating-bus reads follow screen bank 5/7 independently
  of the CPU's upper bank. Model timing uses nominal 3,546,900 CPU clocks/second,
  70,908 clocks/frame and 228 clocks/line; odd RAM banks are contended. 48K keeps
  its previous timing. The portable frame provides its bounded advance duration.
- **S128-03:** An original AY-3-8912 models public selection/data ports, register
  masks, three tone generators, noise and all envelope shapes. Sound mixes into
  the existing bounded 44.1 kHz PCM stream. Logarithmic levels approximate the
  analogue chip; electrical output and heard playback need separate evidence.
- **S128-04:** Guest PLAY accepts three string expressions with notes, octave,
  accidentals, durations/triplets/ties, rests, volume/envelope, tempo, mixer,
  comments, bounded nested repeats and H/BREAK termination. Invalid input reports
  a BASIC error and silences owned sound. MIDI controls parse without a connected
  external device; extra MIDI voices and peripheral signalling are not supplied.
  Note scheduling uses the shared frame interrupt.
- **S128-05:** Guest SPECTRUM preserves the current BASIC programme and enters
  locked 48 BASIC mode; Reset returns to the selected 128K machine. Physical keys
  enter ordinary letters in 128 BASIC. The shared editor, syntax pass, graphics,
  numeric representation and baseline statements remain in guest Z80 code.
- **S128-06:** SAVE!/LOAD! support BASIC, CODE, SCREEN$ and DATA; MERGE! reuses
  the checked BASIC merge implementation. CAT! lists names alphabetically and
  ERASE! reclaims storage. There are 32 entries, ten-character names and 80,896
  payload bytes in the five extra banks after a 1 KiB directory. Replacement
  stages a complete new file before retiring the old one and requires spare
  capacity. NEW preserves files; Reset clears them. Invalid/missing/full cases
  return through the existing BASIC error/recovery path. A full disc leaves the prior file intact. VERIFY! is unsupported.
- **S128-07:** Normal desktop/browser BASIC and assembly Build/Run/Debug need no
  external configuration when no external tool is selected. Source, cassette,
  revision, model/session and stop identities are checked. Stop, Reset, stale
  messages and failed builds retain the existing lifecycle rules. Existing
  explicit MAME/SjASMPlus routes remain effective; external MAME assembly Debug
  remains 48K-only.
- **S128-08:** The shared original assembler emits a contiguous 1–40,960-byte
  image at 24576–65535 and the standard CLEAR/LOAD CODE/USR TAP loader. Code can
  operate the public banking and AY ports. A banked object linker, new project
  format, exact historical menus/editor presentation, external serial/printer
  peripherals, +2A/+2B/+3 and physical replacement-ROM delivery are not included.

## Qualification

The family board tests cover independent banks/aliases, lock/reset, shadow screen,
floating bus, line contention, private-gate isolation, AY generators and guest
PLAY/RAM-disc/error recovery. The portable session tests exercise actual cassette
loading, extended BASIC, alphabetical catalogue and source-bound Debug for both
targets. Desktop and browser tests use the same family APIs; the native/WASM
parity script compares exact frames, audio, requests and debugger results.

Additional gates are the ordinary repository check, three affected macOS product
packages, actual browser/native journeys, matching browser corresponding source
and opt-in stock-ROM MAME cassette loading/execution. Reference tests require
exact UK ROM hashes outside source/artefacts; no ROM bytes or snapshots are logged.
Physical original-hardware cassette loading/gameplay, new Linux/Windows packages,
heard audio and universal historical-software compatibility remain unclaimed.

The [acceptance receipt](evidence/spectrum128-development-acceptance-20261020.json)
records the firmware, source identities, command results, matching native/WASM
traces, stock-ROM MAME checks, rebuilt source archive and local UI/package evidence.
[Testing](testing.md) provides the repeatable commands. The firmware retains its
development identity; this acceptance does not constitute a public release.

Manufacturer references and independently authored firmware provenance are listed
in [the firmware README](../families/spectrum/firmware/original/README.md).
