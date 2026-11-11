# The web canvas

The site renders Forge documents with the real renderers — `gpui-forge-runtime`
compiled to WebAssembly (`crates/forge-web-canvas`), one bundle per
WASM-capable base. This is the same render twin Studio embeds. What you see
in the browser is what the document looks like natively — within the limits
below.

## Canvas per base

The editor swaps the canvas based on the document's `base:`:

| Base | Canvas |
|---|---|
| `gpui-kit` | Real gpui-kit widgets in an iframe — needs WebGL2/WebGPU |
| `ratatui` | `TuiCanvas` — the terminal frame drawn as a monospace cell grid, straight from `TestBackend` |
| `iced`, `bezel`, `gpui02`, `adabraka` | No web renderer that paints yet — the source editor still works |

## What renders (gpui-kit)

- **Elements:** every element in the gpui-kit catalog, plus the wasm-viable
  `use:` libraries — `ely`, `gc` (gpui-component), `hig`, `base`, `zorite`'s
  math.
- **Graceful placeholders:** libraries that can't run in a browser. They render a
  truthful "not provided by this adapter yet" element. Instead of a black
  box — `video.*` (GStreamer can't compile to wasm), `wb.Board`,
  `gflow.Canvas`, and `pdf.Viewer` (which shows its own surface).
- **Props not applied yet:** `Input.readonly`, `Select.searchable` and
  `Select.cleanable`.

## Bindings and hooks

- `$path` reads from the view state (seeded from the doc's `state:` decls,
  editable as JSON in the view-state editor). `bind $path` writes back, and
  the editor shows the live state as it changes.
- `on <event>: name()` calls are *logged* above the canvas rather than run —
  hooks are Rust-side, and the browser has no Rust glue. [Studio](studio-preview.md)
  or [`forge build`](forge-cli.md) gives them a home.

## Styles

The canvas applies the keys of the [style & layout model](style-layout.md).
`overflow: scroll` draws as `hidden`, and `border` draws a 1px border whatever
its width.

## Ratatui documents

`base: ratatui` docs don't need graphics — the WASM twin renders them into a
`TestBackend` and `TuiCanvas` paints the resulting character grid. Layouts,
blocks, lists, gauges, tabs, inputs and tables all show up as real terminal
frames on [the examples page](web-examples.md).
