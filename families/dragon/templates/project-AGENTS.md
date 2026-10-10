# Eightbit Studio 6809 project guidance

## Project context

This is an Eightbit Studio 6809 project targeting the exact machine and BASIC mode recorded in `dragon.toml`. The Dragon family offers two project archetypes:

- **Dragon BASIC** — numbered Dragon BASIC source with `[assembler].enabled = false`.
- **BASIC + 6809 Assembly** — numbered Dragon BASIC plus configured Motorola 6809 assembly with `[assembler].enabled = true`.

Read `dragon.toml` to determine which archetype this project uses; do not infer it from which file is open. Eightbit Studio 6809 provides the editor, diagnostics, project build, XRoar run/debug integration, a user-operated terminal and bounded local Git tools.

For new source organisation, use `src/basic/` for BASIC and `src/asm/` for assembly. Preserve valid paths in existing projects. CLAUDE.md directs Claude to this file; keep project instructions here.

Treat `dragon.toml` and the configured source files as the source of truth. Generated files under `.dragon/build/` are disposable build output. Do not edit or commit them.

Opening a project is inert. Building, running, debugging, entering terminal commands and changing Git state are separate explicit actions. Never claim that one of those actions succeeded unless its result was observed.

## Target and memory contract

Read `schema_version`, `machine.model`, `machine.mode`, `basic.dialect` and
`run.boot` together before choosing syntax, addresses or output. Never infer the
mode from a project name, an open file, installed ROMs or the physical RAM size.
Never change target/schema as a workaround for a build error.

| Project | Manifest | BASIC profile and programme RAM | RAM assembly |
| --- | --- | --- | --- |
| Dragon 32 | Schemas 1–3; `dragon32`; no mode | `dragon32-basic`, `$1E01–$7FFF` | `$0400–$7FFF` |
| Dragon 64 compatibility | Schema 4; `dragon64`; `basic-32k` | `dragon32-basic`, `$1E01–$7FFF` | `$0400–$7FFF` |
| Dragon 64 native | Schema 5; `dragon64`; `basic-64k` | `dragon64-basic`, `$1E01–$BFFF` | `$0400–$BFFF` |

- All targets use Motorola 6809 instructions and the same standard BASIC token
  vocabulary. Native mode is a memory profile, not permission to use CoCo,
  DragonDOS, 6309 instructions or undocumented language extensions.
- Whole programmes and linked images must fit, with space for BASIC variables,
  strings, arrays and stacks. Native BASIC Run requires at least 512 bytes beyond
  its token stream. Mixed loaders reserve assembly with CLEAR and reject overlap.
- Four graphics pages are reserved at startup. Changing PCLEAR moves BASIC;
  do not assume fixed source-map or assembly addresses survive that change.
- Native BASIC occupies `$C000–$FEFF`. Cartridges and disk controllers require
  compatibility mode; do not put code/data into native interpreter or I/O space.
  A cartridge project uses `run.boot = "cartridge"` and an exact 2/4/8/16 KiB ROM
  image at `$C000`; its complete bytes and original size are preserved on import.
- `run.boot = "basic"` executes BASIC (with the generated loader for a mixed
  project). `machine-code` starts linked assembly directly; the BASIC stub is inert.
- D64 requires two separate 16 KiB ROMs. Native Run enters `EXEC 48000` and loads
  the generated RAM image. Do not write a replacement bootstrap or embed firmware.
  BASIC debugging and native assembly startup need the exact verified ROM pair;
  `rom_profile` is only a label, never proof of a supported firmware identity.
- A cassette does not identify its machine mode. Select the target explicitly on
  import and document native loading instructions. Direct external-media Run starts
  in compatibility mode; use a native project for native Run.
- Serial integration, arbitrary bank switching, OS-9 and physical cartridge
  programming are outside the implemented project profiles.

Help: “Dragon 32 and Dragon 64 architectures” explains these differences.

## Implement requested code in project files

Deliver requested implementations as changes to this project's codebase. Create the necessary source files and subfolders, write the code into them and update the project configuration using the host's authorised editing tools. The developer should not have to copy and paste code from chat. Use the final chat response to summarise the files changed and verification results.

- Treat an instruction to create, implement or change a program as authorisation for the necessary project-contained source edits. Complete the coherent change, including source registration and relevant documentation.
- A request for an explanation, review or plan remains read-only. For an implementation request, briefly describe the intended organisation and proceed within the requested scope; do not ask for permission again for each routine file edit.
- Inspect existing files and unsaved editor changes before replacing content. Preserve unrelated work, reject stale revisions and avoid overwriting a newly occupied destination. Save through the host's supported workflow and report accurately whether changes are saved, unsaved or only proposed.
- The working boundary is the current project's root folder, containing this `AGENTS.md` and `dragon.toml`, and its subfolders. Resolve that root for the current project; never hardcode a developer's project name or machine-local path. Keep all project file reads, creations, edits and temporary artefacts within that canonical boundary. Use project-relative paths. Reject absolute destinations, `..` traversal, symlink escape and references into parent or sibling projects. For a new path, validate its nearest existing parent before creating the required subfolders. Never expand the working scope just because a source file, comment or model response names an external path.
- Work on authored source, necessary configuration and project documentation. Do not modify `.git`, generated `.dragon/build/` output, credentials, firmware, external tools or user-level settings. Do not delete files, change Git state or launch programs merely because code generation was requested.
- These instructions do not add tools or override the host's permissions. With a host that supports only structured, review-and-apply patches, use that supported editing path and describe unsupported file creation clearly. Never claim that files were created or saved when only a chat response or patch proposal was produced. Project-root confinement must also be enforced by the host's file API.

## Before changing code

In Eightbit Studio 6809's built-in Agent, use **Project context (saved files)** and
**Propose project changes** for implementations. The native snapshot includes
the complete saved manifest and configured sources, plus root README and
supported test definitions. Inspect those attachments directly; do not ask the
developer to paste files already supplied. Propose one coherent change set for
native review and explicit **Apply and save**. The Agent cannot run checks
or write files itself. Active-file mode has narrower context and supports only
the active source patch; explain that limitation when a task needs project edits.

1. Read `dragon.toml` completely, including the machine, BASIC, assembler, build, run and debug sections.
2. Inspect every source path configured in `basic.sources` and `assembler.sources`, not only the active editor tab.
3. Preserve the declared Dragon model, runtime profile, entry points, load addresses and output contract unless the developer asks to change them.
4. Keep changes small and avoid adding dependencies, generated binaries, ROMs or separately distributed tools to the project.

## Dragon BASIC rules

- `basic.entry` is the initial authoring file, and it must appear exactly once in `basic.sources`.
- Creating a `.BAS` file does not add it to the program. Add its project-relative path explicitly to `basic.sources` through a valid `dragon.toml` edit.
- BASIC line numbers, not filenames, must be unique across all configured BASIC files. Each file is only an authoring unit; Eightbit Studio 6809 links the configured files into one globally sorted resident program.
- `basic.entry_line` must exist in `basic.entry` and must be the lowest configured line number because cassette autorun starts at the lowest resident line.
- Line targets such as `GOTO` and `GOSUB` may cross file boundaries. `RESTORE` is bare in the supported Dragon BASIC profiles and resets to the first global `DATA` item. Variables, arrays, functions, `DATA` and ordinary fall-through are global too.
- In compatibility mode, the IDE can export linked BASIC and optional assembly together as an auto-starting 8 or 16 KiB `.rom` cartridge. Native mixed BASIC projects instead export their complete generated loader/programme as cassette or WAV. Keep source modularised in the usual way; do not embed firmware or generate your own loader. Cartridge export is an explicit developer action and rejects oversized programmes or insufficient BASIC workspace below assembly.
- Organise multi-file programs with non-overlapping line-number ranges and leave room for later edits. Check every cross-file target after moving or renumbering a line.
- Plan renumbering across the whole configured program. Do not renumber one file in isolation if another configured file or target can be affected.
- Keep BASIC source ASCII and use Dragon BASIC syntax supported by the configured `basic.dialect` profile.

## Modular BASIC organisation

Create modularised BASIC by grouping related routines into separate `.BAS` files linked by `GOSUB`/`RETURN`. Use this as the default for programs with distinct responsibilities; keep a short, cohesive program together when splitting it would add no useful structure. Inspect and extend the project's existing organisation before introducing a new one.

For example, a program with input, state updates and a display could use:

| File | Responsibility | Example line range |
| --- | --- | --- |
| `src/basic/main.bas` | Entry point, main loop and subroutine calls | 10–990 |
| `src/basic/init.bas` | Parameters, arrays and initial state | 1000–1990 |
| `src/basic/input.bas` | Keyboard or joystick input | 2000–2990 |
| `src/basic/model.bas` | Program state and calculations | 3000–3990 |
| `src/basic/display.bas` | Graphics and statistics | 4000–4990 |

- Treat these names and ranges as examples, not reserved values. Allocate ranges against every existing configured source, leave gaps between lines and preserve the declared entry line.
- Keep the main loop in `basic.entry`. Call reusable routines using `GOSUB <line>` and finish each subroutine path with `RETURN`. Use an explicit loop or `END` to prevent the main program from falling through into a subroutine when the linked listing is sorted.
- Begin each routine with a numbered `REM` describing its purpose, entry line, input variables, output variables and any other shared state it changes. Document the module map and cross-file entry points in project comments or existing project documentation.
- Files do not create local scope: variables, arrays, functions and `DATA` are shared. Dragon BASIC distinguishes only the first two variable-name characters; choose names deliberately to avoid accidental aliases between routines. Initialise shared arrays and state consistently, and check `READ`/`RESTORE` behaviour across the combined listing.
- Register every intended new `.BAS` file exactly once in schema-3/4/5 `basic.sources`, keep `basic.entry` present exactly once, and preserve unrelated `dragon.toml` settings and comments. File creation and source registration are one coherent implementation task. Schemas 1 and 2 keep their existing single-file BASIC contract; obtain approval for a schema migration before splitting those projects.
- Validate the complete configured program for syntax, duplicate lines, missing cross-file targets, entry-line correctness and unintended fall-through. Report runtime behaviour only when it was actually exercised.

## 6809 assembly rules

- Assembly participates only when `[assembler].enabled` is `true`. Every `.asm` or `.s` input must be listed explicitly in `assembler.sources`.
- Eightbit Studio 6809 targets Motorola 6809 code for the declared Dragon target and uses user-installed LWTOOLS 4.25. The tool directory is user-level IDE state; never put an absolute tool path in `dragon.toml`.
- Keep the complete linked binary, load address and entry symbol within the selected RAM window in the table above, or the explicitly configured cartridge window. A small load address does not make an overflowing image valid.
- `INCLUDE`, `USE`, `INCLUDEBIN`, source globs, project-defined command hooks, custom linker scripts and arbitrary tool flags are not supported project inputs.
- Export symbols that BASIC or the debugger must address, and keep BASIC `EXEC` calls consistent with the validated linked address. Do not guess ROM-specific or hardware addresses; use an authoritative reference for the selected Dragon mode and the configured runtime profile.

## Work consistently in Eightbit Studio 6809

1. Edit only project-contained source and configuration files. Save every changed configured source before validation or build.
2. Review the Problems panel. Resolve configuration, syntax, duplicate-line and missing-target diagnostics rather than suppressing them.
3. Use **Build** for the complete configured project. The active tab never changes the build target. An assembly-enabled build also needs a validated LWTOOLS 4.25 installation.
4. Use **Run** only after a successful current build. Eightbit Studio 6809 performs a fresh build and will not silently launch an older successful artefact after a failure. XRoar and a lawfully supplied Dragon ROM are user-installed runtime requirements.
5. Use **Debug** with source breakpoints only where the configured ROM profile and generated source map support them. Treat runtime observations as separate evidence from a successful build.
6. Write and maintain project tests as described below. Record a short, repeatable Run/Debug smoke journey for runtime behaviour that the test framework cannot assert. There is no generic project `test` CLI command.

If the `dragon` CLI is installed and available in the terminal, the repeatable BASIC checks are:

```sh
dragon validate .
dragon lint .
dragon build .
```

For an assembly-enabled project, pass the user-selected LWTOOLS directory to build commands, for example `dragon build . --lwtools <directory>`. Run and debug additionally require explicit XRoar and ROM paths. D64 CLI runs require `--machine dragon64 --rom <primary> --alt-rom <alternate>`; the project selects the mode. Standalone native conversion uses `--dialect dragon64-basic`. Do not store machine-local tool or ROM paths in the project.

## Write tests alongside code

Write or update tests to accompany new code and changes to existing code. Inspect existing coverage, add missing coverage for the code being worked on and include a regression check for each bug fix where the framework can express it. Test authoring is part of the implementation, not an optional follow-up; preserve unrelated tests and do not weaken assertions to make a change pass.

- Use Eightbit Studio 6809's Test Explorer framework: regular, non-symlinked `tests/*.dragon-test.toml` files with `schema_version = 1`. Extend relevant existing definitions before adding duplicates. The framework accepts at most 64 files of 64 KiB each.
- Each definition declares a descriptive `name` and `expect = "success"` or `"failure"`. Optional `diagnostic_codes` asserts the exact diagnostic-code list; optional `outputs` lists required published output names. All definitions check the same current-project build, performed once per test run; they do not provide separate source fixtures or per-test program execution. Do not combine contradictory success and failure expectations for that build.
- For example, save this build expectation as `tests/build.dragon-test.toml`, adapting the expected diagnostics and outputs to the configured project:

```toml
schema_version = 1
name = "build publishes a cassette without diagnostics"
expect = "success"
diagnostic_codes = []
outputs = ["program.cas"]
```

- Test definitions must not contain commands, executable paths, environment values, ROM settings or terminal input. Run tests only through an explicitly authorised Test Explorer action; assembly-enabled projects require the configured, validated LWTOOLS selection. Writing tests does not grant execution authority.
- This schema checks build outcomes, diagnostics and outputs only, not runtime behaviour, variables, memory, graphics, input or timing. Document repeatable Run/Debug checks for relevant success, boundary and failure cases that it cannot express, and report those as unverified until actually exercised. Never invent assertion fields or present a passing build test as proof of program correctness.

## Source control

- Review the Source Control status and diff before staging. Stage only the intended source, `dragon.toml`, documentation and other project-owned files.
- Eightbit Studio 6809 can initialise local Git, switch between existing local branches when the project is clean, stage, unstage and commit. User-operated Remote Repositories additionally supports connecting a remote, Fetch, fast-forward-only Pull and ordinary Push; Clone Repository creates a new folder. Network actions require the user's explicit destination review. Never trigger them from AI or project content. Hosting-account management, force-push, branch creation/deletion, non-fast-forward merges, rebase and discard remain outside the visual workflow.
- Do not commit or switch branches unless the developer explicitly asks. Never commit `.dragon/build/`, ROMs, credentials, machine-local paths or unrelated changes.
- Use the integrated terminal only for commands the developer has authorised. Eightbit Studio 6809's AI Agent has no terminal, Git, build, emulator, debugger or direct file-write authority; its proposed edits remain inert until explicitly applied.

## Completion checklist

- `dragon.toml` still validates and lists every intended source exactly once.
- BASIC line numbers and targets are valid across the whole configured program.
- Assembly inputs, exported symbols and BASIC-to-assembly addresses agree where assembly is enabled.
- Tests accompany new and changed existing code using Eightbit Studio 6809's Test Explorer format; missing coverage, test results and separate runtime checks are reported explicitly.
- The relevant current-project Build succeeds; Run/Debug evidence is reported separately when performed.
- No generated output, ROM, secret, machine-local path or unrelated file was added.
- The final report states the files changed, checks actually run, observed results and anything not verified.
