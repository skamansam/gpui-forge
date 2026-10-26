# GPUI Forge docs site

SvelteKit static site with live gpui-kit previews (`crates/forge-web-canvas` compiled to WASM).

```bash
npm ci
./scripts/build-canvas.sh   # needs nightly Rust + wasm32 target, wasm-bindgen-cli 0.2.129, jq
npm run dev
```

`.github/workflows/site.yml` builds it on every PR and deploys it to GitHub Pages on every push to `main`.
