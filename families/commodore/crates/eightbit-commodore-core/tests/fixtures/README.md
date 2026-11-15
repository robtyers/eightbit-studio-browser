# Original C64 acceptance programmes

These independently authored source fixtures are Apache-2.0, like this crate.
They use the published BASIC V2, VIC-II, SID and KERNAL interfaces already
described in `families/commodore/help/GUIDE.html`. They contain no third-party
firmware, recovered implementation or commercial programme material.

`acceptance.bas` clears the screen, selects a blue border/black background,
prints `C64 BASELINE READY` and the sum 15, and shows a yellow striped sprite.
It uses the default VIC bank and cassette buffer at $0340–$037F for sprite data;
do not combine this exercise with cassette I/O. A moderate-volume triangle SID
tone is approximately 440 Hz on PAL. Press Q to hide the sprite, silence the
voice and print `INPUT OK`, then return to READY. Stop must also end the sound.
This is a bounded acceptance exercise, not a calibrated audio or timing test.

`acceptance.asm` uses the existing $080D source origin and Studio-generated
SYS2061 loader. It calls CHROUT, prints `ASM OK`, changes the border to yellow
and returns to BASIC. It does not include ROM code, a second loader or headers.

Ordinary tests check source acceptance. The opt-in
`real_c64_acceptance_build_export_and_recovery` test additionally uses explicitly
selected ca65/ld65, VICE and exact-hash firmware to verify both source builds,
guarded PRG exports, cancellation, failed-build preservation, stale-output
rejection, correction and managed execution. It does not inspect pixels or
establish keyboard or audible output. Observe those separately in the actual
packaged application and in an independent VICE launch; record each evidence
class in `docs/testing.md`.

For remote digital acceptance, `real_c64_exported_video_keyboard_and_pcm` loads
guardedly exported original PRGs in separate bounded VICE sessions. It adds only
test-owned capture/cycle-limit/keyboard-buffer arguments to the existing native
adapter. Set `EIGHTBIT_C64_CAPTURE_DIR` to a new absolute folder; existing folders
are rejected. Only original PRGs, PNG framebuffer exports and PCM WAV files are
retained there. ROM staging remains private and temporary. The fixture stays
active in one capture and receives Q through VICE's documented keyboard buffer
in another; a third capture runs assembly.

The test checks PRG autostart, bounded cycle completion, image dimensions, 44.1 kHz
16-bit mono PCM, a sustained approximately 440 Hz tone in the active capture's
final second and silence after Q. Inspect the frames separately for expected text,
sprite and return to READY. Keyboard-buffer input establishes guest GET/cleanup
behaviour, not physical keyboard/CIA-matrix or desktop focus delivery. WAV capture
establishes digital SID synthesis, not speaker/headphone output. VICE deliberately
returns exit code 1 when `-limitcycles` is reached; only that specific completion
is accepted. Its dummy playback driver is unsuitable for this capture: use the
WAV output driver rather than a recording layered on dummy playback.
