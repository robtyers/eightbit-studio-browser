# C64 PAL verification

This is a manual checklist, not a test runner. Record date, source revision,
BASIC/assembly template, tool versions and actual results. Do not record ROM bytes,
private paths or credentials. Leave unperformed checks pending.

1. Save and Build. Expect a current successful PRG build, with no unresolved
   diagnostics. BASIC needs no external tools; assembly needs cc65 2.19.
2. Run in configured VICE 3.10 x64sc with the supported PAL firmware. BASIC:
   expect a cleared screen, greeting and numbers 1–5. Assembly: expect border 6,
   background 0 and a return to BASIC. Replace these expectations as code evolves.
3. Exercise each input, empty/invalid input and array/data boundaries added by
   the programme. Record expected and observed video/audio separately.
4. Stop, edit a visible result, Save and Restart/Run. Confirm the changed result
   and that the previous owned session ended.
5. With changes saved safely, introduce a temporary syntax error, Build and try
   Run. Expect failure and no launch of the previous artefact. Restore the source,
   rebuild and confirm recovery.
6. Export to a new `.prg` destination; load and run that exact file in VICE.
   Verify controls and result again. An existing destination must remain unchanged.

A process launch does not prove successful loading, input, graphics or sound.
Physical C64 checks are separate: document the transfer method and PAL hardware
before claiming hardware support. Add a repeatable regression check for each fix.
