# Dragon programme verification

Read the root README.md for Test Explorer definitions and exact offline Help
references. This checklist is for the model and mode in dragon.toml; record
source revision, tool versions and actual observations. Leave unperformed checks
pending and keep ROM contents, secrets and private paths out of evidence.

1. Save all configured sources and Build. The BASIC starter should build without
   tools or firmware. The mixed starter requires LWTOOLS 4.25. Inspect Problems
   and the current Build Results; a previous successful build may be stale.
2. Add a `build.dragon-test.toml` using the root README's example and choose
   Project → Run Tests. Definitions check the same current-project build; they
   do not run independent BASIC programmes or assert emulator state.
3. With authorised XRoar and lawful configured ROMs, Run on the selected Dragon
   model/mode. Expect HELLO, DRAGON and, for mixed source, ASSEMBLY RETURNED.
   D64 native loading needs the supported primary/alternate ROM pair. Replace
   these expectations as code changes and exercise controls and input boundaries.
4. Stop, edit a visible result, Save All and Run again. Check that the old owned
   session ends and the new programme appears. Separately check any changed
   BASIC breakpoint or machine-code stepping behaviour when Debug is configured.
5. Safely introduce a temporary source error. Build/Run should fail without
   launching an older artefact. Restore source and confirm a successful rebuild.
6. Export to a new destination and test the actual cassette, WAV or cartridge
   using the root README's mode-specific loading instructions. Native D64 BASIC
   cannot use cartridge export. Check that existing destinations are preserved.

Add repeatable regression steps for each fix. Build tests, observed XRoar behaviour,
physical Dragon operation and cassette-transfer checks are separate evidence.
