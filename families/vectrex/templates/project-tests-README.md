# Vectrex project checks

These are repeatable manual checks, not an executable Test Explorer suite.
Vectrex has no built-in automated gameplay runner or .dragon-test.toml support.
All checks below are **pending** until you perform and record them. Run tools and
emulators only with the developer's explicit authority. Use a disposable copy for
intentional invalid-source checks; never destroy the only good source revision.

Record date, source revision, selected original/external runtime and tool versions, input device,
expected and actual results for each check. Record export hash/size when available.
Do not include firmware, secrets or machine-local paths in shared evidence.

| Check | Steps | Expected result |
| --- | --- | --- |
| Open is inert | Open the folder with tools unconfigured | Source is editable; no build or emulator starts |
| First build | Leave external tools unconfigured, save and select Build | Fresh cartridge.bin, cartridge.lst, symbols.txt and manifest.json appear in .vectrex/build; cartridge is at most 32768 bytes |
| Repeatability | Build the same saved source twice with the same tools; compare manifest hashes | Same cartridge SHA-256 and size |
| Dirty source | Edit without saving | Build and Run Project remain unavailable until Save succeeds |
| First frame | Select Run original (development), or browser Play | Shared screen shows a diamond and independently moving line; confirm visually, not from process status |
| Movement | Test left, right, up, down, diagonal and neutral input | Correct direction; neutral stops movement; animation continues |
| Movement boundary | Hold each direction long enough to cross the signed-byte range | Record the starter's wrapping behaviour; a changed clamp/wrap policy needs explicit expected bounds and regression checks |
| Held sound | Press, hold and release Vectrex button 1 repeatedly | Tone continues while held and stops on release; neutral is silent |
| Other buttons | Press buttons 2-4 without button 1 | No button-1 tone; record intended changes if assigning new actions |
| Restart | Move the diamond, then select Restart in Emulator | Old session stops, saved source rebuilds and initial position returns |
| Stop/recovery | Stop during playback; then Run Project again | Owned process ends; a new session starts successfully |
| Cancel build | Stop while a build is in progress, where timing permits | No new successful result or automatic emulator launch; subsequent Build works |
| Invalid source | In a disposable copy add an unsupported INCLUDE, then Save and Build/Run | DSV1100; no new successful output or fallback to an older cartridge |
| Assembly error | In that copy use an undefined label | DSV1103; fix it, save and rebuild successfully |
| Original executive limit | In a disposable copy call an unimplemented BIOS service | VXO1001; no host substitute; fix source and restart |
| Compatibility route | Configure LWTOOLS and RetroArch/VecX; Build with LWTOOLS then Run Project | External build/runtime works independently; record its acceptance separately |
| Missing tool | Select an unavailable tool path in setup | Actionable setup failure; correct selection permits retry |
| Fresh export | After successful Build choose Export cartridge and a new filename | File matches the current cartridge; an existing destination is rejected intact |
| Stale save | Change source externally after opening, then attempt to save the old editor revision | Save is rejected; preserve unsaved text and reopen before retrying |

A successful build cannot prove refresh stability, correct analogue input, audible
sound or hardware compatibility. Test on physical equipment only under a separate,
explicitly approved procedure with lawfully provisioned cartridges and the device
manufacturer's instructions. Do not describe a pending hardware check as passed.

When adding a feature, append its normal, edge and regression cases here. If a
repeatable automated harness becomes necessary, agree its scope before adding it.
