# Previewing in Studio

The Preview tab hosts `forge.Preview#canvas` — it can paint the document
itself or act as a window into a separate preview process.

## Embedded canvas

By default the canvas is the in-process render twin — the same `ForgeView`
that powers the [web canvas](preview.md). Every edit repaints instantly.
`bind` props write through to the document's `view.state`, and `on <event>`
hooks log to the hook sink rather than running Rust.

The **Vars** tab (`forge.Vars`) lists every `{$path}` variable bound in the
document. Edit a scalar and watch the app react — it is the same state the
`state:` declaration seeds and `bind $path` writes. The **View state** group
in the inspector shows the raw JSON.

## Preview in Host — `forge-preview-<base>`

**File → Preview in Host…** spawns the out-of-process preview binary for the
document's base. Studio talks to it over a JSON channel on stdin/stdout:

- Studio sends `source` (the buffer — so *unsaved* documents preview too) or
  `load` (a path, for saved docs), plus commands like `raise` and the
  pick/place toggles.
- The host runs the document through its *native* event loop and reports
  `ready`, diagnostics, fired hooks, and view-state writes back to Studio.

This is how bases without an in-process twin preview from Studio. `ratatui`
runs on a real `TestBackend` and streams its cell grid back as `preview.frame`
(the TUI text you see under the canvas). `iced`/`gpui02`/`adabraka`/`bezel`
open real native windows, and `gpui-kit` itself can run in-host when you want
the genuine platform event loop.

The status bar's `host` label shows host attachment state. Clicking the
preview raises the host window.

## Declared preview panes — `remote:` + `id:`

The canvas above is the *implicit* preview — one host, one document. A
document can also declare its own preview panes: any `forge.Preview` with a
non-empty `remote:` prop and an `id:` gets its own `forge-preview-{base}`
host, spawned automatically:

```yaml
- forge.Preview:
  id: pane-iced            # required — addresses the pane's host + slot
  remote: iced             # non-empty → out-of-process host
  base: iced               # which forge-preview-{base} binary
  doc: ./login.forge.yaml  # doc-relative path, named ref, or parent doc
  mode: embedded           # host window floats over this pane's slot
```

- `base:` empty → the document's own `base:`. Each unique `(doc, base)`
  pane spawns one host; change either prop and the pane respawns.
- `doc:` a doc-relative `.forge.yaml` path (sent as `load`; the file
  re-loads on change), a named ref's head-less body (`name: &name` —
  streamed as `source`), or nothing — the pane previews the edited
  document itself.
- `mode: embedded` floats the host window over the pane's tracked slot.
  Without it the pane renders the framed "click to focus" card.

Show, hide, and selection are ordinary control flow — gate the pane with
`if:` or pick it from a `Select` + `match:`/`case:`. The host spawns when
the pane enters the document and dies when the pane leaves it. Panes with
`remote:` but no `id:` log a status note and spawn nothing. See
`examples/previews-demo.forge.yaml` for a working selector.

Studio's own Themes tab uses the same mechanism — `studio.forge.yaml`'s
`previewSelector` binds a `Select` to `$studio.theme_preview` and each
`case` is a `mode: embedded` pane over the matching showcase document.
The Rust side only maintains state and plumbing; the document decides
which bases exist and which preview is live. Theme changes reach pane
hosts too, so a selected preview repaints in place.

## `base_dir`

Studio forwards the open document's directory to the host so `src:` props and
other relative asset paths resolve. One more reason Open Project (a
`forge.toml`) is the most faithful way to preview.
