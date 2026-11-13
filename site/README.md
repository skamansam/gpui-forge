# GPUI Forge docs site

SvelteKit static site with live gpui-kit previews (`crates/forge-web-canvas` compiled to WASM).

```bash
npm ci
./scripts/build-canvas.sh   # needs nightly Rust + wasm32 target, wasm-bindgen-cli 0.2.129, jq
npm run dev
```

`.github/workflows/site.yml` builds it on every PR and deploys it to GitHub Pages on every push to `main`.

`llms.txt` and `llms-full.txt` are generated from `../docs` by `scripts/generate-llms.mjs` (runs on `predev`/`prebuild`, outputs gitignored). Add or edit docs and the files update on the next build; curated section layout and non-doc links live in that script.
