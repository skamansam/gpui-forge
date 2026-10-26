#!/usr/bin/env bash
# Builds crates/forge-web-canvas to WASM and stages it, with gpui-kit's icons, in static/canvas.
# gpui-kit's web platform needs nightly Rust (wasm_thread) and wasm-bindgen-cli matching
# the locked wasm-bindgen version.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="$ROOT/site/static/canvas"
cd "$ROOT"

CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="-C link-arg=-zstack-size=8388608" \
  cargo +nightly build -p forge-web-canvas --target wasm32-unknown-unknown --profile web --locked

wasm-bindgen target/wasm32-unknown-unknown/web/forge_web_canvas.wasm \
  --out-dir "$OUT/pkg" --target web --no-typescript

ASSETS="$(dirname "$(cargo metadata --format-version 1 --locked |
  jq -r '.packages[] | select(.name == "gpui-kit-assets") | .manifest_path')")"
rm -rf "$OUT/assets/icons"
mkdir -p "$OUT/assets"
cp -r "$ASSETS/assets/icons" "$OUT/assets/icons"
echo "canvas staged in $OUT ($(du -h "$OUT/pkg/forge_web_canvas_bg.wasm" | cut -f1) wasm)"
