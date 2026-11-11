# Web editor

The editor at [/editor](https://gpui-forge.rudeboy.dev/editor) builds `.forge.yaml`
documents visually: click elements in, tune props, watch the real renderer paint
them. Everything is client-side — the renderers are the same Rust twins Studio
uses, compiled to WASM.

## Layout

- **Components** (left) — the palette, grouped accordion-style (Layout, Windows,
  Display, Input, Navigation…). Click an item to insert it inside the node
  selected in the Tree. A `use:` import adds that library's elements under their
  `alias.` prefix.
- **Tree** — the document outline. Select a node to edit it, reorder with
  ↑/↓, delete with the Delete button, undo/redo with ↩/↪ (or Ctrl+Z / Ctrl+Y).
- **Inspector** — props and `style:` entries for the selected node, typed from
  the catalog (enums are dropdowns, colors checked, unknown props flagged).
- **Canvas** — the live render. The WASM canvas behind it depends on `base:`:

  | Base | Canvas |
  |---|---|
  | `gpui-kit` | Real gpui-kit widgets in an iframe (needs WebGL2) |
  | `ratatui` | The terminal frame as a monospace cell grid |
  | other bases | No web renderer — the source editor still works |

- **Live state** — the view's current state JSON. Two-way `bind` props write
  here, hooks fire above the canvas title.
- **doc.forge.yaml** — the generated YAML, editable both ways: edits re-check
  and re-render, palette edits rewrite the text. Diagnostics land under the
  editor with line/column and a hint.
- **View state (JSON)** — the `$path` bindings' initial state. Apply reseeds it.

## Sharing and files

- **Share** copies a link with the whole document + state in the URL hash —
  the editor stores nothing anywhere.
- **Open** imports a `.forge.yaml` file or a `.json` document model from disk.
- **Save** (Ctrl+S) downloads the `.forge.yaml`. **JSON** downloads the parsed
  document model — `forge check`/`forge build` can consume either.
- The **Examples…** picker loads any shipped `examples/*.forge.yaml` — curated
  examples plus every widget-set demo.

## Limits

The canvas previews layout and props. Host-only concerns (file IO, native
menus, GStreamer video) don't run in the browser — those elements render "not
provided by this adapter yet". For full fidelity use Studio.
