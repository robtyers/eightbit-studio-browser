#!/usr/bin/env bash
set -euo pipefail
browser_root="$(cd "$(dirname "$0")/../.." && pwd)"
browser_bindgen="${WASM_BINDGEN:-$browser_root/.cache/browser-tools/wasm-bindgen-0.2.127-aarch64-apple-darwin/wasm-bindgen}"
"$browser_bindgen" --version | rg -q '^wasm-bindgen 0\.2\.127$' || { echo 'wasm-bindgen CLI 0.2.127 is required.' >&2; exit 1; }
cd "$browser_root"
browser_target="${CARGO_TARGET_DIR:-$browser_root/target/browser-core}"
CARGO_TARGET_DIR="$browser_target" cargo build -p eightbit-browser --release --locked --target wasm32-unknown-unknown
mkdir -p apps/eightbit-studio-web/ui/public/generated
"$browser_bindgen" "$browser_target/wasm32-unknown-unknown/release/eightbit_browser.wasm" --target web --no-typescript --out-name dragon_core --out-dir apps/eightbit-studio-web/ui/public/generated
# Discard declarations from earlier builds; the UI narrows the dynamic WASM API.
rm -f apps/eightbit-studio-web/ui/public/generated/dragon_core.d.ts apps/eightbit-studio-web/ui/public/generated/dragon_core_bg.wasm.d.ts
