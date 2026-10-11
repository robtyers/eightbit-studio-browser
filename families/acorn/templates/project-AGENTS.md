Follow the exact BBC Model B, OS 1.2, BASIC II and DFS 1.20/NFS 3.00 contract in README.md.
Preserve `bbc.toml`, the target and its one configured source. Do not introduce
65C02 instructions, external command hooks, arbitrary linker configuration,
firmware, third-party programme bytes or unimplemented media/debugger workflows.
Use full uppercase BASIC keywords while preserving variable and string case.
For assembly, keep one `.org $1900`, documented NMOS instructions and an RTS
return. Explain memory and MOS-call assumptions. Build and Run require explicit
user actions; AI assistance has source authority only and cannot execute tools.
