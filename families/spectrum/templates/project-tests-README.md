# Spectrum verification

This checklist is not an automated test definition. Record the exact target
(16K, 48K, 128K or +2), source revision, tool versions and observed results.
Leave unperformed steps pending; do not record firmware contents or private paths.

1. Save and Build. Expect a current successful TAP snapshot. BASIC uses the
   built-in encoder; assembly requires SjASMPlus 1.24.0.
2. Run with MAME 0.289 and the target's validated ROM slot(s). BASIC should print
   the starter greeting and play eight tones; assembly should set border colour 2 and return
   to BASIC. Replace these expectations as the programme develops.
3. Exercise controls and boundary/invalid input. Check text, screen attributes,
   drawing and sound that the programme actually uses. Verify 16K memory fit on
   the 16K target; another machine's successful run is not evidence for this one.
4. Stop, change a visible result, Save, and Restart/Run. Confirm the updated
   programme and termination of the old owned session.
5. Temporarily introduce a syntax error after saving a recoverable version.
   Build/Run must fail without launching an old artefact. Restore and rebuild.
6. Export to a new `.tap` filename and load that exact image in the selected
   machine. Check the loader and controls again; existing files must be preserved.
7. For a Spectrum 48K assembly debugging change, separately check breakpoints,
   stepping and Stop. The debug snapshot lacks a BASIC caller; use Run to verify
   `RET` into BASIC or routines requiring normal ROM/BASIC state. Other Spectrum
   models and BASIC programmes have no integrated debugger.

Add a repeatable regression check for each fix. A successful build or process
launch does not prove input, display or sound. Physical hardware acceptance and
real tape transfer are separate checks with their own recorded setup.
