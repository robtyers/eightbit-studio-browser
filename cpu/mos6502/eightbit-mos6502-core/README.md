# Original NMOS 6502 foundation

Stages 1–2 and the approved stage-3 observation boundary of the [C64 plan](../../../docs/c64-original-specification.md), approved
20 October 2026. This Apache-2.0 implementation has no runtime dependencies,
firmware, assembler, family imports, host I/O or automatic execution.

`Cpu::step` executes one of the 151 documented NMOS instructions or a pending
IRQ/NMI entry. Every read/write, including indexed/branch/stack dummy reads and
both read-modify-write writes, passes through the injected fallible `Bus`.
`Step::cycles` counts those accesses. Address, zero-page and stack arithmetic
wraps intentionally; indirect JMP uses the NMOS pointer-page wrap. Status keeps
U set and B clear, adding B only to software-pushed status. NMOS decimal mode is
retained through reset and interrupt entry. Cold state is deterministic, not a
claim about physical power-up registers.

Interrupts are logical asserted levels. Caller-driven buses retain the original
instruction-boundary API. An optional `Bus::interrupt_lines` callback observes
signals after each access and accepts a board-latched NMI pulse during DMA;
normal IRQ decisions use the penultimate cycle. NMI is edge-latched and takes
priority, including redirection before an IRQ/BRK vector fetch. CLI/SEI/PLP
retain the previous IRQ mask for the next boundary; RTI restores it immediately.
Original timed-device vectors cover these cases. Branch-edge polling races and
SO remain unqualified. The board owns finite RDY/BA/AEC waits inside accesses:
`Step::cycles` still counts CPU accesses, while the PAL receipt counts actual
elapsed clocks including waits. This is a cycle-domain model, not transistor or
half-dot timing qualification.

All 105 other opcode values return `D650202` after the opcode fetch. Bus faults
return `D650201`; both require reset (`D650203` prevents further accesses).
Failed operations can leave partial architectural/bus side effects; no rollback
is implied. A successful seven-read reset clears the fault and pending NMI,
retains external levels/RAM/A/X/Y/D and decrements the stack three times without
writes. The board separately owns port/peripheral reset.

`Cpu::run` accepts 1–1,000,000 steps, checks caller cancellation before each and
returns exact completed-step/cycle counts. Interrupt entries consume steps.
There is no unbounded run API or retained trace. Cancellation does not interrupt
one instruction; the synchronous bus adapter must itself return promptly.

Stage 3 additionally provides default-no-op `Bus::execution` observations for
Reset, actual opcode/interrupt entry, completion and fault. They consume no bus
cycles and cannot replace guest instructions or edit registers. An observer
failure becomes a normal sticky bus fault; cleanup preserves the original error.
The CPU knows no firmware, page map or family policy. The original C64 board uses
these observations solely to admit/check immutable guest ROM gates. Independent
vectors verify exact read/cycle counts, interrupt provenance and fault recovery;
legacy buses retain their existing behaviour.

## Verification and provenance

```sh
cargo test --offline --locked -p eightbit-mos6502-core
cargo clippy --offline --locked -p eightbit-mos6502-core --all-targets -- -D warnings
```

Source, decoder and tests were independently authored for this change. No
emulator implementation, ROM disassembly, external test binary or third-party
fixture was used. Instruction facts derive from the MOS Technology *MCS6500
Microcomputer Family Programming Manual*, January 1976, second edition:

- [Appendix B](https://lbaeza.neocities.org/mcs6500/6500_appb): documented opcodes,
  addressing forms and instruction-cycle totals.
- [Chapters 2](https://lbaeza.neocities.org/mcs6500/6500_ch02),
  [6](https://lbaeza.neocities.org/mcs6500/6500_ch06),
  [8](https://lbaeza.neocities.org/mcs6500/6500_ch08) and
  [9](https://lbaeza.neocities.org/mcs6500/6500_ch09): arithmetic, wrapping,
  stack, reset and interrupt interfaces. The online copy is an archival
  transcription of the manufacturer publication, not reference implementation.

The original tests check binary ADC/SBC across every accumulator/operand/carry
combination (262,144 instruction executions), valid BCD results/carry across
every decimal pair/carry (40,000), explicit NMOS flag cases, every documented
opcode/addressing form, bus ordering, error/reset and bounded cancellation.
Invalid BCD digit combinations have deterministic implementation behaviour but
are not independently hardware-qualified. Boundary interrupt and dummy-bus
vectors are original expected observations; they do not replace future
cycle-sensitive supplied-ROM/hardware qualification. No whole C64 boot or
historical-software compatibility is established by these tests.
