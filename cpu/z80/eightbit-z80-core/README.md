# Original Z80 execution and source assembly

Independent Apache-2.0 Rust implementation from Zilog UM008011-0816 instruction
facts. The library has no firmware, filesystem, process, network or family
dependency. It is portable to native and WebAssembly callers. ADR 0058 owns its
Spectrum integration; this crate alone does not establish a complete Spectrum.

`Cpu::step` executes one instruction/interrupt against an injected timed bus.
It implements base, CB, ED and DD/FD decoding, indexed memory and register halves,
block instructions, flags, refresh, EI delay and interrupt modes. Repeated index
prefixes are limited to 64; this deliberate bounded-input safeguard returns a
typed fault rather than monopolising the host. Unassigned ED encodings act as
two-fetch NOPs. Undocumented electrical/flag quirks need separate qualification;
passing the present vectors does not establish universal historical compatibility.

`assembler::assemble` consumes bounded ASCII source and returns bytes, symbols and
instruction-bound source lines. It accepts ordinary documented instructions,
index halves/indexed CB copy forms, colon labels, forward `EQU`, integer expressions,
`ORG`, `DB`/`DEFB`, `DW`/`DEFW`, `DS`/`DEFS` and `ALIGN`. It rejects includes, macros,
scripts, malformed or out-of-range operands, cycles and unconverged layouts.
This is an original assembler rather than an implementation of every SjASMPlus
extension. The local `assemble` example writes binary to stdout for controlled
first-party build scripts; user-facing diagnostics should remain typed.

```sh
cargo test -p eightbit-z80-core
cargo clippy -p eightbit-z80-core --all-targets -- -D warnings
```
