# My Dragon programme

This is an Eightbit Studio 6809 project for the target and mode recorded in `dragon.toml`. Replace this introduction with what your programme does, its controls and any special requirements. Keep the getting-started notes useful for the next developer who opens the project.

## Target requirements

Check the manifest before editing: Dragon 32 uses schemas 1–3, D64 compatibility
uses schema 4 / `basic-32k`, and D64 native uses schema 5 / `basic-64k` with
`dragon64-basic`. Native mode raises the BASIC/assembly RAM ceiling from `$7FFF`
to `$BFFF`; `$C000` and above hold its interpreter and hardware state. Physical
64 KiB RAM does not mean 64 KiB free for a programme. Both modes use the 6809.

D64 needs separate 16 KiB primary and alternate BASIC ROMs in user-level Settings.
BASIC debugging and native assembly startup require the verified ROM pair.
Cartridge workflows require compatibility mode. Import/Create Project asks for
the exact target; no existing project is silently converted. For native cassette
loading outside the project, enter `EXEC 48000`, `PCLEAR 4`, then `CLOAD`/`RUN`
or `CLOADM`/`EXEC` as appropriate. Direct external Run starts in compatibility mode.

Help: “Dragon 32 and Dragon 64 architectures”.

## Getting started

The usual workflow is: edit source, build, check tests, run in XRoar, debug, review and commit, then export and check the distributable programme. Opening a project does not build it, run tools or contact a remote service.

This guide is for developing Dragon programmes in an installed Eightbit Studio 6809 application. You do not need to build the IDE or install its Rust/Node development toolchain. BASIC-only editing, builds and build tests need neither an emulator nor a firmware ROM. Running and debugging need separately installed XRoar and lawfully obtained ROMs for the configured target (one for D32, both 16 KiB slots for D64). Assembly builds also need LWTOOLS 4.25.

### 1. Get the starter programme running

1. Open this project folder in Eightbit Studio 6809 and open `src/basic/main.bas` from Explorer.
2. Read [dragon.toml](dragon.toml): `[assembler].enabled = false` means Dragon BASIC; `true` means BASIC + 6809 Assembly. The mixed template also contains `src/asm/main.asm`, initially a routine that returns to BASIC at `$7000`.
3. For the mixed template, choose **Settings > Build Tools** and select the directory containing the separately installed `lwasm` and `lwlink` 4.25 executables.
4. Choose **Build** (Command–B). Check Problems and Output and fix any errors before continuing. A failed build must not be treated as a successful test of an older programme.
5. In **Settings > Emulators**, select your XRoar application/executable; configure lawful ROMs for the selected Dragon model under **Systems & Firmware**, then **Validate and Save**. The tested XRoar baseline is 1.12.1; Eightbit Studio 6809 validates the selected installation. Tools and firmware are not included in this project.
6. Choose **Run** (Command–R). Eightbit Studio 6809 builds the current configured project again and launches XRoar only after success. Expect `HELLO, DRAGON`; the mixed starter also prints `ASSEMBLY RETURNED`. Use **Focus** to show XRoar if needed, then **Stop** to end the session.
7. Change the greeting in `src/basic/main.bas`, choose **File > Save All**, then Run again and confirm the new text. Stop the session before further builds, tests or project-source changes.

For details, open **Help > Eightbit Studio 6809 User Guide** and search for “Build, run and debug in Eightbit Studio 6809”. All Help references below are exact bundled topic titles, available offline; they are not paths to files in this project.

### 2. Understand and extend the project

- [dragon.toml](dragon.toml) defines the target, entry point and explicit source membership. Build and Run use this configuration, not whichever editor tab is active.
- `src/basic/main.bas` is the starter BASIC entry. Additional `.BAS` files belong in `basic.sources`; create them in Explorer, then register them through **Project > Manage Sources…**.
- `src/asm/main.asm` exists only in the mixed template. Register additional assembly files in `assembler.sources`. Keep the exported entry symbol, linked load address and BASIC `EXEC` call consistent.
- [CLAUDE.md](CLAUDE.md) instructs Claude to read the AGENTS file in this folder.
- [AGENTS.md](AGENTS.md) contains editable project guidance for coding helpers. Add your conventions and programme-specific constraints there.
- [tests/README.md](tests/README.md) starts a repeatable verification checklist. Add build expectations in that folder using the example below.
- `.dragon/build/` contains generated output, including manifests, source maps and media. Rebuild it from source; do not edit or commit it. The generated `.gitignore` excludes it but does not automatically protect every secret or firmware file you might add elsewhere.

New projects use `src/basic/` for BASIC and `src/asm/` for assembly; the latter is created only for the mixed template. Existing projects retain their configured paths and schemas. Do not move their sources merely to match a newer template.

Multiple BASIC files become one programme sorted by line number, not separately loaded modules. Keep line numbers unique across all files, keep `basic.entry_line` as the lowest line in the configured entry source, and organise routines using `GOSUB`/`RETURN`. Use distinct ranges, such as main flow below 1000 and display routines from 1000, with an explicit `END` or loop before subroutines to prevent fall-through. Variables and `DATA` are shared; variable names are distinguished by their first two significant characters.

Help: “Programming in Dragon BASIC”, “Dragon BASIC graphics, sound and input”, “Assemble and link with LWTOOLS” and “Programming the Motorola 6809”.

### 3. Keep work in source control

1. Open **Source Control** and initialise Git if this folder has no repository. Creation of a Dragon project does not initialise Git or make a first commit automatically.
2. Review each change and diff. Stage intended sources, `dragon.toml`, tests, `README.md`, `AGENTS.md`, `CLAUDE.md` and `.gitignore`, then write a descriptive commit message and commit.
3. Work in small changes: edit, build/test, exercise behaviour, inspect the diff and commit. Use **History** to inspect commits and **File > History > Git File History…** for one file. Recovery History is local edit recovery, not a replacement for Git or a backup.
4. Optionally create a hosted repository separately, then use **Source Control > Remote Repositories…** to connect it and explicitly Fetch, Pull or Push. Check the displayed destination and branch. A local commit is not a remote backup until Push succeeds.

Visual branch switching uses existing local branches and requires a clean project. Pull is fast-forward-only and requires saved editors, a clean tree and no active Run/Debug session. Branch creation and conflict resolution are outside the visual workflow; use your own Git client or deliberately entered terminal commands when needed. Never put credentials in a remote URL. Keep firmware, API keys and machine-local tool paths outside the repository; review staged files before every commit.

Help: “Build, run and debug in Eightbit Studio 6809” — the local source-control, remote-repository and Git-history sections.

### 4. Use agentic helpers with review

Start with a small, specific request and an observable result. For example:

> Add a score display using the existing programme structure. Read AGENTS.md and dragon.toml, preserve the configured target and entry point, register any new sources, and add relevant build tests plus a repeatable runtime check. Summarise the edits and distinguish checks actually run from checks still needed. Do not commit, push or export.

For the built-in Agent, configure your chosen provider under **Settings > AI Providers**. Save `AGENTS.md`, select the relevant BASIC/assembly source, refresh and review the context preview, then explicitly Send. This transmits the previewed context to the selected provider and may incur provider charges. Exclude secrets and firmware. Keys stay in macOS Keychain, not project files.

Treat generated code as a proposal: review the diff, explicitly apply supported changes, then build, test and exercise the result yourself. The built-in Agent has no terminal, Git, build, emulator or debugger authority. `AGENTS.md` gives guidance, not new permissions. An external coding agent may have different editing and execution tools: keep it confined to this project and authorise actions deliberately. Never assume files were saved or tests passed just because a chat reply says so.

For a game or other multi-file implementation, save all open edits and use **Propose project changes** with **Project context (saved files)**. Check that the preview lists `dragon.toml` and all configured BASIC/assembly sources. **Review project changes** validates and displays each proposed file; **Apply and save** writes the reviewed set, including source registration, only after your click. **Undo last applied project change** is available while the saved project is unchanged. Use active-file change mode for a smaller unsaved editor patch. Larger project requests may incur higher provider charges.

Help: “Build, run and debug in Eightbit Studio 6809” — “Use agentic helpers with review” and “Project guidance for the AI Agent”.

### 5. Build and test each change

Save changes, choose **Build**, and inspect Problems, Output and Build Results and its current/stale state. A successful build checks the configured sources and creates a coherent output snapshot; it does not prove that gameplay, input, graphics or calculations are correct.

In the existing `tests` folder, create a TOML file named `build.dragon-test.toml`:

```toml
schema_version = 1
name = "current project builds without diagnostics"
expect = "success"
diagnostic_codes = []
```

Choose **Project > Run Tests** and inspect the Test Explorer report. This example works for either starter template; mixed projects still require configured LWTOOLS. To check an output too, add `outputs = ["program.cas"]` for BASIC or `outputs = ["run/program.cas", "assembly/machine.bin"]` for a mixed project. Names are relative to the published build snapshot.

Each test definition checks the same current-project build, performed once per test run. Optional `diagnostic_codes` checks the exact diagnostic list; `expect = "failure"` is for a deliberately failing current build, not an independent source fixture. Do not combine contradictory expectations. Tests cannot contain commands or assert emulator variables, memory, input or timing.

Keep a short runtime checklist alongside your tests: input or starting state, action, expected result, and what you actually observed. Include normal behaviour, boundaries, invalid input and a regression check for each fix. Re-run it in XRoar after changes, and on a physical Dragon before claiming hardware compatibility. A passing build test is not proof of runtime correctness.

If you separately have the `dragon` CLI installed, `dragon validate .`, `dragon lint .` and `dragon build .` provide repeatable project checks from this folder. Mixed builds require `dragon build . --lwtools <directory>` with your selected tool directory. There is no generic `dragon test` command; use Test Explorer. Only you enter commands in the integrated terminal.

Help: “Build, run and debug in Eightbit Studio 6809” — “Test your programme”; “Troubleshooting and supported limits”.

### 6. Debug unexpected behaviour

For BASIC, open a configured source, set a numbered-line breakpoint in the gutter and choose **Debug** (F5). At a verified stop, inspect scalar variables and registers; use Continue to resume. BASIC debugging requires an exact supported ROM identity even when another valid ROM can Run. It is line-level debugging, not statement stepping; arrays, watches and variable mutation are not supported.

For assembly, set a breakpoint on an instruction in a configured assembly source and choose Debug. Inspect the mapped source, registers, bounded memory and disassembly. Step advances one machine instruction; Continue resumes and Pause interrupts. The current machine debugger supports one installed breakpoint. Use Stop/Terminate when finished and keep BASIC `EXEC` calls aligned with linked symbols.

Help: “Build, run and debug in Eightbit Studio 6809”, “Mix BASIC and assembly and debug machine code” and “Dragon 32 error messages”.

### 7. Export and deploy your programme

Here, deployment means producing a distributable Dragon programme and loading it into an emulator or suitable physical hardware, not publishing the IDE. First build, run the tests and complete your runtime checklist. Then choose **File > Export** and a new destination; export builds afresh and never overwrites an existing file.

| Export | What it contains | Intended use |
| --- | --- | --- |
| Cassette image (`.cas`) | BASIC, standalone machine code, or the complete native BASIC/assembly loader according to the selected boot mode | A compatible emulator or cassette-transfer tool. It is not an audio recording. |
| WAV cassette audio (`.wav`) | The same mode-specific cassette content encoded as audio | A suitable playback/recording chain and the Dragon cassette input; follow your hardware instructions. |
| Cartridge image (`.rom`) | Linked BASIC and optional assembly together, with auto-start code | Preview and explicitly start in XRoar, or use compatible cartridge hardware and its separate programming tools. |

In Dragon 32 and D64 compatibility projects, mixed cassette/WAV export contains standalone assembly; use cartridge export to package BASIC and assembly together. In native D64 BASIC projects, cassette/WAV export includes the complete generated BASIC/assembly loader because cartridges conflict with the interpreter. Machine-code boot exports standalone assembly in either mode. Exported cartridges are 8 or 16 KiB; oversized programmes or insufficient BASIC workspace are rejected. They contain no Dragon firmware. Eightbit Studio 6809 does not program physical cartridges.

Inspect and test the actual exported file, not just the source project's Run result. BASIC cassette loading uses `CLOAD` then `RUN`; standalone machine code uses `CLOADM` and its documented entry/`EXEC` convention. See the command reference before loading on real hardware. Check your real transfer setup and record the Dragon model, runtime/firmware requirements, loading steps, controls and observed results in this README. Emulator success alone does not establish physical-hardware compatibility.

Share only your authorised source/assets and chosen programme artefacts, with appropriate licence and loading instructions. Keep source in Git and distribute exports separately from disposable `.dragon/build/` state. Do not redistribute Dragon firmware, credentials or third-party software without permission.

Help: “Build, run and debug in Eightbit Studio 6809” — the export sections; “Using your Dragon 32”, “Dragon 32 command reference” and “Troubleshooting and supported limits”.

### Import and reconstruct existing programmes

Use File → Import for CAS/WAV programmes, recorded tapes or reviewed BAS/ASM/S
source copies. Tools → Disassemble Programme… also accepts raw binaries (supply
load and entry addresses), one-segment DECB and ordinary ROM cartridges. Review
inferred code/data and create a new assembly project only after LWTOOLS reproduces
all bytes. Originals and path-free verification evidence are preserved.
A reconstructed cartridge project builds/runs/exports its original 2/4/8/16 KiB
ROM profile at $C000; cassette/WAV export is unavailable. D64 compatibility projects support cartridge assembly debugging. Native projects cannot build cartridges.
