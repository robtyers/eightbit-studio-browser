# Original MC6809 CPU and assembler

This Apache-2.0 processor library contains the original bounded CPU interpreter
and a no-I/O assembler. Neither component reads files, starts a process, fetches
resources or depends on a machine family. CPU encodings are factual Motorola
6809 instruction definitions, also used by the repository's original decoder.
No external assembler or emulator implementation is included or translated.

`assembler::assemble(source, load_address, entry_symbol)` returns one contiguous
image, its load/entry addresses, resolved 16-bit symbols and instruction source
lines. The caller owns target RAM limits, media encoding, source identity and
execution. Failed assembly returns a line/column diagnostic and no image.

`assembler::assemble_image(source, load_address)` emits a raw image without
reserving a synthetic entry label. Its entry defaults to the load address; an
optional END operand must resolve, but does not select host execution.

`assembler::assemble_project(sources, load_address, entry_symbol)` links an
ordered slice of `AssemblySource { source_uri, source }` values. Each unit owns
its local names; cross-unit references require `EXTERN` in the consumer and
exactly one `EXPORT` provider. Repeated local names in different units are valid.
The entry selects an exported definition first, otherwise one unambiguous local
definition, and must lie within the result. The result includes source-qualified
symbols and emitted ranges for instructions, data, storage and padding. A range
larger than 65,535 bytes is split; declarations emit no ranges. Link failures
include the supplied source identity, line and column and return no partial image.

Units are placed consecutively in caller order; `code` sections retain their
source order. Forward references and automatic instruction widths are resolved
together within the bounded layout passes. There is no object-file format,
deferred relocation, library search, filesystem discovery or section-reordering
script. The caller supplies every source explicitly. A `_constants` section
accepts only non-emitting `EQU`/`=` definitions; declarations such as `EXPORT`
belong outside it. Only addressable 16-bit definitions appear in the result's
symbol map, although expressions may use wider checked constants internally.

The assembler supports documented 6809 instructions, short/long branch aliases,
immediate, direct, extended and all documented indexed/indirect modes. Labels
are case-insensitive. `PCR` computes a displacement to a target expression;
`PC` accepts a literal displacement. Indexed size prefixes `<<`, `<` and `>`
force 5-, 8- and 16-bit offsets. Address prefixes `<` and `>` select direct and
extended memory operands. Explicit direct addressing emits the address's low
byte; `SETDP` supplies the page assumption for automatic selection and does not
emit a CPU register write. A column-zero label may use an instruction/directive
name when the alternative is invalid operand syntax; use a colon to disambiguate
otherwise valid syntax.

Supported directives are `ORG`, `EQU`/`=`, `SETDP`, `FCB`/`DB`/`BYTE`,
`FDB`/`DW`/`WORD`, `FCC`, `FCS`, `RMB`/`DS`/`ZMB`, `FILL`, `ALIGN`, `SECTION code`,
`SECTION _constants`, `ENDSECTION`, `EXPORT`, `EXTERN` (project API only) and `END`.
`ORG` may move forwards from the configured
load address; gaps and reserved bytes are deterministic zero padding. `END`
may name the configured entry symbol. Includes, macros, external object files and
other section types are rejected explicitly; this is not an LWTOOLS linker or
a claim to support every LWASM language extension.

Expressions support decimal, `$`/`0x` hexadecimal, `%` binary, `@` octal,
character literals, labels, current address `*`, parentheses, checked arithmetic,
shifts and bit operations. `FCC` uses double-quoted ASCII strings. `FCS` sets the high bit of the last
character and rejects an empty string. `;` introduces
a comment outside strings, and a line beginning with `*` is a comment. Bounds
include 1 MiB source, 32,768 lines, 4,096 bytes per line, a 64 KiB image,
32 layout passes and bounded expression length/depth/work. The project API permits
1–64 units and applies the byte/line limits to the aggregate. Source identities
must be unique, non-empty, at most 1,024 bytes and contain no control characters;
they are opaque identities, not paths opened by this crate. Symbol/entry names
are at most 128 ASCII bytes. The caller separately enforces file access, target
RAM ranges and source-map policy.

Run `cargo test -p eightbit-m6809-core`. The original instruction corpus covers
636 combinations and each result executes one instruction on the CPU. Focused
tests separately cover exact encoding facts, forward symbols, real assembled
programme execution, directives, overflow, malformed input and hostile text.
The 19 October local comparison against separately installed LWASM 4.25 matched
all 636 generated examples. Only original fixture inputs are checked in;
LWTOOLS remains an optional external verification tool.

The 19 October 2026 local project comparison also matched independently authored,
unchanged DragonDoom (`game.asm`, then `data.asm`, load/entry `$2000`) and Battlezone
(`MAIN.asm`, load/entry `$3800`) against fresh LWASM/LWLINK 4.25 object/link outputs:

| Programme | Bytes | SHA-256 |
| --- | ---: | --- |
| DragonDoom | 13,415 | `03a7e6f6bbf9124f27dddb8f29e1436321ad67c0ce84346f21a923bc785f7267` |
| Battlezone | 5,116 | `6de323c475cdb26a1b23cc2e3895043c304b1e1dbe35faf36ba27d0af0e86681` |

These are byte-parity observations for those source revisions, not a claim of
full LWTOOLS language/object compatibility or game runtime acceptance. The games
and external-tool binaries are not embedded in this crate's ordinary fixtures.
