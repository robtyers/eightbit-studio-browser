# Eightbit Dragon monitor 0.1

Original Apache-2.0 firmware and source, created for ADR 0056. This is not the
Dragon/Microsoft BASIC ROM. It contains no third-party firmware or glyph data.
See `docs/dragon32-monitor-specification.md` for provenance and scope.

Rebuild from the repository root using separately installed LWTOOLS:

```sh
python3 scripts/build-dragon-monitor.py --lwasm /absolute/path/to/lwasm
python3 scripts/build-dragon-monitor.py --lwasm /absolute/path/to/lwasm --check
```

The retained image, symbols and receipt are release/source assets. Development
tools are not bundled. End users boot the packaged image without an assembler.

Type `H` for command help. Addresses/counts/bytes are hexadecimal. `D 0600`
dumps eight bytes; `M 0600 41` edits one byte; `F 0600 10 41` fills 16 bytes;
`C 0600 10` computes an 8-bit additive checksum. `L 0600 8641B7040039` loads
an original six-byte program; `G 0600` executes it, writing an A to the screen
and returning. `R` resets the monitor, preserving user RAM. Host Stop/Reset
remain available if guest code loops or stops on an unsupported instruction.
