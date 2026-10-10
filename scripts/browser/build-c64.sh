#!/usr/bin/env bash
set -euo pipefail
browser_root="$(cd "$(dirname "$0")/../.." && pwd)"
browser_sdk="${EMSDK:-$browser_root/.cache/browser-tools/emsdk-4.0.10}"
browser_archive="$browser_root/.cache/mame-web/chips-9e88298.tar.gz"
browser_source="$browser_root/.cache/mame-web/chips"
browser_output="$browser_root/apps/eightbit-studio-web/ui/public/generated"
test "$(shasum -a 256 "$browser_archive" | cut -d ' ' -f 1)" = fcae39995f61bfe461175047b11f16d147be0e7353c8a247ac503b5fd3a09b8d || { echo 'C64 core source digest mismatch' >&2; exit 1; }
mkdir -p "$browser_source" "$browser_output"
tar -xzf "$browser_archive" --strip-components=1 -C "$browser_source"
source "$browser_sdk/emsdk_env.sh" >/dev/null 2>&1
emcc --version | head -1 | rg -q '4\.0\.10' || { echo 'Emscripten 4.0.10 is required' >&2; exit 1; }
emcc "$browser_root/scripts/browser/runtime/c64.c" -I "$browser_source" -O2 -s MODULARIZE=1 -s EXPORT_ES6=1 -s ENVIRONMENT=web -s ALLOW_MEMORY_GROWTH=0 -s INITIAL_MEMORY=16777216 -s FILESYSTEM=0 -s EXPORTED_RUNTIME_METHODS='["HEAPU8","HEAPF32"]' -s EXPORTED_FUNCTIONS='["_malloc","_free","_studio_init","_studio_load","_studio_step","_studio_key","_studio_pixels","_studio_width","_studio_height","_studio_audio","_studio_audio_count","_studio_audio_clear","_studio_stop"]' -o "$browser_output/c64.js"
