# Original Dragon 64 Studio firmware

[ADR 0060](adr/0060-original-dragon64-basic.md) authorises both PAL Dragon 64
modes, using the independently authored Dragon BASIC engine. Both images remain
**development** firmware. Their finite software tests do not establish universal
historical-software, physical-machine or serial-device compatibility.

## Contracts

| Requirement | Implemented boundary |
| --- | --- |
| D64BAS-01 | Compatibility uses schema 4 and the existing Dragon 32 token vocabulary; BASIC/RAM code remains below `$8000`. |
| D64BAS-02 | Native uses schema 5 and `dragon64-basic`; BASIC starts at `$1E01` with the existing graphics reservation and RAM code remains below `$C000`. Both models have 64 KiB physical RAM. |
| D64BAS-03 | Original guest 6809 code executes BASIC. There is no host evaluator, historical ROM dependency or implicit external emulator. |
| D64BAS-04 | Compatibility retains published low ROM entries; native supplies relocated public entries and `$C49F` RUN. Both retain public token, FAC, workspace, vector, cassette and source-map contracts. Serial entries are `$802A/$802D/$8030`, relocated to `$C02A/$C02D/$C030` in native mode. |
| D64BAS-05 | `EXEC` initially, or `EXEC 48000`, enters native mode without deleting an existing programme. It clears variables and installs the native public hooks. Reset restores the selected startup mode and revokes automatic loading. |
| D64BAS-06 | Native CLEAR and MEM support the larger address space; native cassette block input may reach `$BFFF`. Guest loading is compared against every byte of the current build before Run/Debug. |
| D64BAS-07 | Native key repeat uses the public `$011F` delay (default 5 frames), an initial delay eight times that value, and guest frame interrupts. Compatibility retains single presses; the documented `$BF20` IRQ entry permits explicit opt-in. Cassette motor activity suppresses repeat work. |
| D64BAS-08 | `$FF04–$FF07` address the disconnected ACIA, rather than D32 PIA aliases. Bounded digital transmit output supports guest PEEK/POKE and serial printer selection at `$03FF`; `$03FD/$03FE` retain the line-delay setting. SERIN/SEROUT/SERSET use the published baud selectors. No host serial device or network connection is opened. |
| D64BAS-09 | Both modes share revision-bound BASIC debugging. Native RAM machine-code loading/debugging includes addresses above `$7FFF`, using the existing guest cassette loader and public relocated calls. |
| D64BAS-10 | Desktop and browser BASIC use the same portable execution owner. Desktop retains explicit XRoar and separately supplied firmware, existing project schemas and the shared workbench. Browser D64 authoring is single-source BASIC with desktop-compatible ZIPs. |
| D64BAS-11 | Firmware, symbols, sources, controls and builders have exact provenance; D32 qualification is neither changed nor inherited. Failed, cancelled and stale launches cannot execute an earlier build. |

## Firmware map

Compatibility has an expanded 32 KiB container and the established low public
interfaces. It does not occupy additional programme RAM. Native storage is not
limited to 16 KiB: its common 16 KiB container has an I/O aperture and a private
4 KiB page window at `$D000–$DFFF`. Common executable islands are
`$C000–$CFFF` and `$E000–$FEFF`; vectors remain at `$FFF0–$FFFF`.
The current container size and page count are recorded in
`families/dragon/firmware/basic64/manifest.json`.

The builder retains the D32 branch relaxation for timing-sensitive signal
routines. Tables and stack-sensitive interpreter primitives remain common;
array/expression routines share one page because they transfer an existing
stack frame. Data pointers never become callable page gates. Cross-page guest
calls preserve registers and condition flags. Only compiled common STA sites
can operate private page controls. Page continuations follow the guest stack,
including non-local error unwinds, and have a fixed bound. Reset clears paging.
An invalid continuation fails with `DME1601`.

D64 printer settings own `$03FD–$03FF`. The original engine's private diagnostic
byte therefore moves to otherwise unused interpreter scratch `$03D8`, after the
bounded input buffer. No programme RAM is permanently reserved for extra code,
page storage, an evaluator or a bootstrap. The native transition uses temporary
stack space, preserves programme bytes and reinstalls mode-specific public
hooks. The external XRoar path and its two historical ROM slots are unchanged.

The disconnected profile reports IO for DLOAD/DLOADM after validating its D64
baud selector; a connected host download protocol is not implemented.
Cartridges, DragonDOS, OS-9, arbitrary banking and connections to real serial
hardware retain their separate external-runtime boundaries. The disconnected
ACIA model is not a serial-device or pin-timing qualification. Stock cassette
exports contain programme/loader bytes and public calls, never private firmware
page dependencies. Native cassette loading on a stock D64 requires selecting
its native mode; cassette files contain no mode marker.

## Reproduction and provenance

```sh
python3 scripts/build-dragon64-basic.py --verify
python3 scripts/build-dragon64-basic.py --check --lwasm /opt/homebrew/bin/lwasm
python3 scripts/test_dragon64_basic_build.py
cargo test -p eightbit-dragon-emulator --lib dragon64
```

The builder consumes the original `basic-complete` assembly and the original
D64 peripheral module. It does not read historical firmware or emulator source.
A separately installed LWASM is a build instrument, not a distributed runtime
dependency. `--output-dir` creates a new, verified development snapshot and
refuses an existing destination. `--check` reproduces maintained images and
symbols without promoting their status. Ordinary checks verify provenance
without requiring an assembler.

Factual references are Dragon Data's [Dragon 64 supplement](https://dragondata.co.uk/Publications/BASIC-MAN/DD_D64_Supplement.pdf)
(memory modes, serial registers/printer settings and keyboard repeat), its
[archived scan](https://www.8bit-homecomputermuseum.at/computer/pictures/dragon%2064/dragon_64_supplement.pdf),
and the public hardware/interface descriptions in
[Inside the Dragon](https://www.dragondata.co.uk/Publications/InsideTheDragon.pdf).
The shared engine retains its [D32 provenance](dragon32-basic-specification.md).
No recovered ROM, GPL emulator or third-party interpreter implementation was
copied or translated.

Source/local checks, native/WASM parity, actual browser and packaged interaction,
lawful exact-ROM comparisons, and physical cassette acceptance are separate
evidence classes. Dated commands and results belong in [testing](testing.md).
