# ADR 0055: BBC browser composition and native platform completion

- **Status:** Implementation scope approved by the user on 11 October 2026
- **Extends:** ADRs 0053 and 0054

Complete the exact BBC Model B target on macOS and the browser. Linux and Windows
acceptance are deferred at the user's subsequent request. Preserve the desktop project,
firmware, source and external-tool boundaries in ADR 0054 and the shared
workbench on every platform.

The browser follows its existing BASIC-only contract: create/edit, local
recovery, strict desktop-compatible `bbc.toml` ZIP import/export, deterministic
DFS SSD builds, recovery of BASIC from this encoder's exact disc subset, and
explicit local emulation. Reject assembly projects and arbitrary/non-canonical
disc layouts rather than silently discarding content. Desktop ca65 assembly
remains available on macOS and Linux. No browser assembler or new debugger is
introduced.

Make Acorn's BASIC, disc and firmware rules portable behind the existing
default-enabled `native` family feature. Native process/filesystem dependencies
must not enter WASM. Share the five-file firmware revision validation between
native and browser callers; firmware stays volatile and is excluded from project
recovery, exports, logs, source archives and distributions.

Extend ADR 0053's browser-only GPL exception with a restricted, pinned MAME
0.289 BBC build. Use the same exact Model B/32K/OS120/Acorn8271/DNFS120 options,
fixed app-owned boot command and disposable same-origin runtime as the native
profile. Permit only fixed static assets, bounded local ROM/disc buffers and
explicit keyboard/audio controls. Cancellation, edits, replacement, suspension
and unmount discard the runtime and pending firmware. Ship matching source,
local patches, notices and build instructions with the browser artefacts;
publication/deployment still require separate authorisation.

Record ordinary, actual WASM, real firmware/runtime and native packaged checks
separately. macOS acceptance must exercise the shared managed
Build/Run/Stop/Restart lifecycle and actual BASIC/assembly execution. Browser
acceptance must include the generated SSD, supported firmware, visible output,
input, cancellation and restart. A container or cross-build cannot establish
installed desktop acceptance. Physical iPad, physical BBC and Windows results
remain separate from the hosts actually observed.
