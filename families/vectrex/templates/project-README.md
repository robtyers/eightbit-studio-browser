# Vectrex cartridge project

This is the **Vector, sound and controls** starter for the original Vectrex.
It draws a movable diamond and an animated line, with a tone while button 1 is
held. This source is an original Apache-2.0 demonstration; this project contains
no firmware or commercial game. Studio supplies an independently authored
development runtime and executive.

## Start here

1. Open this folder in Eightbit Studio 6809. [vectrex.toml](vectrex.toml) selects Vectrex
   automatically; opening the project does not build or run anything.
2. Open [src/asm/main.asm](src/asm/main.asm) in the source editor. Save your changes before
   Build or Run Project. Source editing works without external tools.
3. Build uses the original built-in 6809 assembler; no external tools are needed.
   **Build with LWTOOLS** in Emulator is the explicit compatibility alternative,
   requiring a separately installed LWTOOLS 4.25 selection in Build Tools.
4. Select **Build**. A successful build publishes `cartridge.bin`, `cartridge.lst`,
   `symbols.txt` and `manifest.json` in `.vectrex/build/`. A build proves assembly
   and cartridge structure, not graphics, sound or gameplay.
5. Select **Run original (development)** in Emulator to play this starter with
   our original executive. Focus the shared screen: arrows move; Z/X/C/V are buttons
   1–4. The executive implements a documented subset; unsupported BIOS services
   fail explicitly. Browser **Play** uses the same original runtime.
   For external compatibility, select your installed **RetroArch executable** and **VecX core**
   in Settings → Emulators. Select **Run Project** to build saved source and start
   an external emulator window. Focus that window for controls. Use its input
   settings to check the controller mapping; button 1 is a Vectrex button number.
6. **Stop** ends the owned session. **Restart** in the Emulator panel stops,
   rebuilds and starts again. **Export cartridge…** in Build Results writes the
   current successful build to a new filename; it never overwrites an existing file.

Application Settings (Command-comma) is shared with the Dragon workbench.
General selects the startup screen or last project; Appearance selects System,
Light or Dark. Each app remembers its own preferences. New project locations
default to your home folder.

## Project files

| File | Purpose |
| --- | --- |
| `vectrex.toml` | Authoritative schema-1 name, target and single source path |
| `src/asm/main.asm` | Cartridge header, code, data and RAM address labels |
| [CLAUDE.md](CLAUDE.md) | Directs Claude to AGENTS.md in this folder |
| [AGENTS.md](AGENTS.md) | Vectrex-specific instructions for external coding helpers |
| [.gitignore](.gitignore) | Generated build state, local environment files and ROM-folder exclusions |
| [GUIDE.html](GUIDE.html) | Complete offline operating and programming guide; open in a browser |
| [tests/README.md](tests/README.md) | Repeatable build, input, sound, failure and recovery checks |

The Vectrex Explorer edits the configured assembly source. Selecting a supporting
document reveals it in Finder; use an external editor to read or customise it.
Reopen the project after changing its manifest externally. The same guide
is available inside the app through **Emulator → Vectrex guide** or the guide
command in the Command Palette (Command-Shift-P). Search by topic, routine or
error code. GUIDE.html is self-contained and needs no internet connection.

## Project and coding rules

New projects keep assembly in `src/asm/`; no BASIC folder is created because
Vectrex has no BASIC template. Existing valid source paths remain supported.
Keep one `.asm` source beneath `src/`, up to 128 KiB, with ASCII lines no longer
than 4096 bytes and exactly one `ORG 0`. Additional source files, includes, macros,
`RMB`, object linking, bank switching, BASIC, integrated Vectrex debugging
and Dragon Test Explorer definitions are not supported. Use `EQU` for RAM
addresses; keep mutable state away from BIOS workspace and the stack. See the guide
for the memory map, direct-page rules, BIOS calls and original exercises.

## Source control and helpers

Git initialisation is a separate, explicit Source Control action. Review changes
before staging or committing; opening or creating this project does not initialise
Git. Keep source, documentation and tests in version control. Do not commit build
outputs, firmware, secrets or computer-specific executable paths. Ignore patterns
are a convenience, not proof that a file is safe to publish.

Ask an external coding helper to read AGENTS.md and the guide, make bounded source
changes and update tests/README.md with repeatable checks.
The optional in-app Agent previews context, streams responses and offers reviewed
Apply and save with guarded Undo. YOLO is off by default and saves only the
configured assembly source. AI never runs Build, Run, Git or terminal commands.
Record checks as pending until actually performed;
never describe a successful build as emulator or physical-console acceptance.

## Next steps

Follow the guide's demo walkthrough, then change one thing at a time: shape size,
movement bounds, button behaviour or pitch. Run the corresponding checks in
[tests/README.md](tests/README.md). Keep a known-good source revision. Existing
projects are never rewritten just because the IDE's starter template changes.
