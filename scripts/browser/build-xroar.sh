#!/usr/bin/env bash
set -euo pipefail
browser_root="$(cd "$(dirname "$0")/../.." && pwd)"
browser_sdk="${EMSDK:-$browser_root/.cache/browser-tools/emsdk-4.0.10}"
browser_archive="$browser_root/.cache/xroar-web/xroar-1.12.1.tar.gz"
browser_build="$browser_root/.cache/xroar-web/build-1.12.1"
browser_output="$browser_root/apps/eightbit-studio-web/ui/public/generated"
test -f "$browser_sdk/emsdk_env.sh" || { echo 'Set EMSDK to an activated Emscripten 4.0.10 SDK.' >&2; exit 1; }
test -f "$browser_archive" || { echo 'Provision the pinned XRoar source archive described in docs/browser-edition.md.' >&2; exit 1; }
browser_actual="$(shasum -a 256 "$browser_archive" | cut -d ' ' -f 1)"
test "$browser_actual" = ba525225fbd732c4dbf2cbb571dc9d2810fd9dcedf0482133cf637e2eef61f88 || { echo 'XRoar source digest mismatch.' >&2; exit 1; }
# A fresh, known source tree prevents old configure output or patches being reused.
mkdir -p "$browser_build" "$browser_output"
tar -xzf "$browser_archive" --strip-components=1 -C "$browser_build"
cd "$browser_build"
patch -p1 --forward < "$browser_root/scripts/browser/xroar-local-files.patch"
source "$browser_sdk/emsdk_env.sh" >/dev/null 2>&1
emcc --version | head -1 | rg -q '4\.0\.10' || { echo 'Emscripten 4.0.10 is required.' >&2; exit 1; }
emconfigure ./configure --enable-wasm --disable-fast-install CFLAGS='-O3 -flto' LDFLAGS='-O3 -flto -g2'
emmake make -j2
cp src/xroar.js src/xroar.wasm "$browser_output/"
