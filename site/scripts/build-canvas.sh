#!/usr/bin/env bash
# Builds crates/forge-web-canvas to WASM per wasm-capable base and stages it,
# with gpui-kit's icons, in static/canvas/pkg/<base>/.
#
# Usage: build-canvas.sh [base]
#   no args  — build every base and stage icons (local dev)
#   <base>   — build only that base (CI matrix legs). Icons stage with gpui-kit
#
# gpui-kit/bezel need nightly Rust (wasm_thread's stdarch_wasm_atomic_wait).
# iced's graph is stable-clean. wasm-bindgen-cli must match the locked
# wasm-bindgen version.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="$ROOT/site/static/canvas"
cd "$ROOT"

declare -A TOOLCHAIN=( [gpui-kit]=+nightly [bezel]=+nightly [iced]="" [ratatui]="" )
declare -A FEATURE=( [gpui-kit]=base-gpui-kit [bezel]=base-bezel [iced]=base-iced [ratatui]=base-ratatui )

build_base() {
  local base="$1"
  local tc="${TOOLCHAIN[$base]}"
  local feat="${FEATURE[$base]}"
  echo "==> canvas-$base (toolchain '${tc:-stable}', feature $feat)"
  CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="-C link-arg=-zstack-size=8388608" \
    cargo ${tc:++nightly} build -p forge-web-canvas \
      --no-default-features --features "$feat" \
      --target wasm32-unknown-unknown --profile web --locked
  wasm-bindgen target/wasm32-unknown-unknown/web/forge_web_canvas.wasm \
    --out-dir "$OUT/pkg/$base" --target web --no-typescript
  echo "    staged $OUT/pkg/$base ($(du -h "$OUT/pkg/$base/forge_web_canvas_bg.wasm" | cut -f1) wasm)"
}

stage_icons() {
  # Icons only exist on the gpui-kit canvas — copy them next to its bundle.
  local assets
  assets="$(dirname "$(cargo metadata --format-version 1 --locked |
    jq -r '.packages[] | select(.name == "gpui-kit-assets") | .manifest_path')")"
  rm -rf "$OUT/assets/icons"
  mkdir -p "$OUT/assets"
  cp -r "$assets/assets/icons" "$OUT/assets/icons"
}

if [ $# -gt 0 ]; then
  base="$1"
  if [ -z "${FEATURE[$base]+x}" ]; then
    echo "unknown base '$base' (want one of: ${!FEATURE[*]})" >&2
    exit 2
  fi
  build_base "$base"
  if [ "$base" = gpui-kit ]; then stage_icons; fi
else
  for base in "${!FEATURE[@]}"; do
    build_base "$base"
  done
  stage_icons
  echo "canvas staged in $OUT"
fi
