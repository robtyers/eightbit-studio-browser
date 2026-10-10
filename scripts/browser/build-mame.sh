#!/usr/bin/env bash
set -euo pipefail
browser_root="$(cd "$(dirname "$0")/../.." && pwd)"
browser_sdk="${MAME_EMSDK:-$browser_root/.cache/browser-tools/emsdk-6.0.2}"
browser_archive="$browser_root/.cache/mame-web/mame0289.tar.gz"
browser_source="$browser_root/.cache/mame-web/source"
browser_output="$browser_root/apps/eightbit-studio-web/ui/public/generated"
test "$(shasum -a 256 "$browser_archive" | cut -d ' ' -f 1)" = 0929cc749afabcef892900e10dd90bd8b05f94a7dde8f367ac6a5d2082589f84 || { echo 'MAME source digest mismatch' >&2; exit 1; }
mkdir -p "$browser_source" "$browser_output"
tar -xzf "$browser_archive" --strip-components=1 -C "$browser_source"
cd "$browser_source"
patch -p1 --forward < "$browser_root/scripts/browser/mame-browser.patch"
source "$browser_sdk/emsdk_env.sh" >/dev/null 2>&1
emcc --version | head -1 | rg -q '6\.0\.2' || { echo 'Emscripten 6.0.2 is required' >&2; exit 1; }
emmake make REGENIE=1 SUBTARGET=eightbit_z80 SOURCES=sinclair/zx.cpp,sinclair/spectrum.cpp,sinclair/spec128.cpp OSD=sdl NO_USE_MIDI=1 NO_USE_PORTAUDIO=1 NO_USE_PULSEAUDIO=1 NO_USE_PIPEWIRE=1 USE_TAPTUN=0 USE_PCAP=0 NO_X11=1 NO_OPENGL=1 SYMBOLS=0 OPTIMIZE=2 -j2
# The upstream emits the subtarget name into its loader; distribution uses one fixed name.
sed 's/eightbit_z80\.wasm/mame-z80.wasm.gz/g' eightbit_z80.js > "$browser_output/mame-z80.js"
gzip -n -9 -c eightbit_z80.wasm > "$browser_output/mame-z80.wasm.gz"
