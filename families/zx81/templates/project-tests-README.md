# ZX81 verification

Record the exact 1K/16K PAL target, source revision, tool versions and observed
results. This is a manual checklist, not a test runner. Keep unperformed checks
pending; never include ROM contents, credentials or private paths in evidence.

1. Save and Build. Expect a current P snapshot without unresolved diagnostics.
   BASIC needs no external tools; assembly needs SjASMPlus 1.24.0.
2. Run using MAME 0.289 with the selected RAM profile and validated zx81a.rom.
   BASIC should clear the screen, print its greeting and count; assembly should
   return 42 through the generated USR caller. Update these checks with code changes.
3. Exercise each control, empty/invalid input and numeric/array boundary. On 1K,
   check display growth and variable use: a small source alone does not prove
   runtime memory sufficiency. Do not substitute a 16K run for a 1K check.
4. Stop, change an observable result, Save and Restart/Run. Confirm the old owned
   session ended and that the changed programme is running.
5. Safely introduce a temporary syntax error. Build/Run must fail without starting
   the old artefact. Restore source, rebuild and verify recovery.
6. Export to a new `.p` filename and load that exact snapshot. Verify the result
   and controls again. An existing destination must not be overwritten.

Add a repeatable regression check for each fix. There is no integrated ZX81
debugger. Process launch does not prove loading, display or keyboard behaviour.
Physical ZX81/tape-transfer acceptance requires a separately recorded setup.
