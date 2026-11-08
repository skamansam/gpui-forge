#!/usr/bin/env bash
# Builds crates/forge-web-canvas to WASM per wasm-capable base and stages it,
# with gpui-kit's icons, in static/canvas/pkg/<base>/.
#
# gpui-kit/bezel need nightly Rust (wasm_thread's stdarch_wasm_atomic_wait);
# iced's graph is stable-clean. wasm-bindgen-cli must match the locked
# wasm-bindgen version.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="$ROOT/site/static/canvas"
cd "$ROOT"

declare -A TOOLCHAIN=( [gpui-kit]=+nightly [bezel]=+nightly [iced]="" [ratatui]="" )
declare -A FEATURE=( [gpui-kit]=base-gpui-kit [bezel]=base-bezel [iced]=base-iced [ratatui]=base-ratatui )

for base in "${!FEATURE[@]}"; do
  tc="${TOOLCHAIN[$base]}"
  feat="${FEATURE[$base]}"
  echo "==> canvas-$base (toolchain '${tc:-stable}', feature $feat)"
  CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="-C link-arg=-zstack-size=8388608" \
    cargo ${tc:++nightly} build -p forge-web-canvas \
      --no-default-features --features "$feat" \
      --target wasm32-unknown-unknown --profile web --locked
  wasm-bindgen target/wasm32-unknown-unknown/web/forge_web_canvas.wasm \
    --out-dir "$OUT/pkg/$base" --target web --no-typescript
  echo "    staged $OUT/pkg/$base ($(du -h "$OUT/pkg/$base/forge_web_canvas_bg.wasm" | cut -f1) wasm)"
done

# Icons only exist on the gpui-kit canvas — copy them next to its bundle.
ASSETS="$(dirname "$(cargo metadata --format-version 1 --locked |
  jq -r '.packages[] | select(.name == "gpui-kit-assets") | .manifest_path')")"
rm -rf "$OUT/assets/icons"
mkdir -p "$OUT/assets"
cp -r "$ASSETS/assets/icons" "$OUT/assets/icons"
echo "canvas staged in $OUT"
