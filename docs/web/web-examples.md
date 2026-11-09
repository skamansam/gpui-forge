# The examples page

[/examples](https://gpui-forge.rudeboy.dev/examples) is the showcase: every
shipped `examples/*.forge.yaml` rendered beside its source. It's the quickest
way to see what a document looks like — and what each widget set offers.

## Two kinds of examples

- **Curated documents** — the login form, settings panel, and tabs demos.
  These also show the same view hand-written in gpui-kit Rust, so you can
  compare the YAML with the code it replaces.
- **Widget-set showcases** — one demo per registered library and base:
  `ely`, `gc` (gpui-component), `gflow`, `base`, `video`, `hig`, `pdf`,
  `whiteboard`, `math`, plus the `gpui-kit` base itself.

## How each one renders

The page picks a canvas per example, matching what the
[web canvas](preview.md) supports:

- **gpui-kit docs** render live — including the wasm-viable `use:` libraries.
  Ones a browser can't run degrade to an honest placeholder element.
- **`base: ratatui`** docs draw a terminal frame via `TuiCanvas` — three demos
  ship: the showcase, a dashboard (tabs + gauges + a table), and a forms demo
  (inputs + lists).
- **Native-only bases** (`iced`, `bezel`, `gpui02`, `adabraka`) show their
  source with a "native only" note — no black canvas pretending to render.
  Open them in [Studio](studio.md) to run them for real.

## Loading one into the editor

The editor's **Examples…** picker loads any of these documents — curated
examples and showcases share one list — so you can remix them, then
[share the result](editor.md#sharing-and-files) as a link.
