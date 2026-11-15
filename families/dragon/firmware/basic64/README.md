# Original Dragon 64 development firmware

Apache-2.0. Independently authored guest 6809 firmware shared with the original
Dragon BASIC engine. No historical firmware or third-party interpreter is
included. The images are Studio runtimes, not physical replacement ROMs.

`compatibility.rom` supplies the 32K compatibility profile. `native.rom` contains
common code plus bounded private ROM pages for native BASIC. `native.sym`,
`native-controls.rs` and `manifest.json` bind their original sources and builder.
The D64-specific source is `source/peripherals.inc`; the remaining original
assembly is shared from `../basic-complete/source` through the pinned builder.

Reproduce with `python3 scripts/build-dragon64-basic.py --check` from the
repository root, using separately installed LWASM. Neither image inherits D32
qualification. See `docs/dragon64-basic-specification.md` and ADR 0060 for memory,
public interfaces, provenance and acceptance boundaries.
