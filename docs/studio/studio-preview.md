# Previewing in Studio

The Preview tab hosts `forge.Preview#canvas` — it can paint the document
itself or act as a window into a separate preview process.

## Embedded canvas

By default the canvas is the in-process render twin — the same `ForgeView`
that powers the [web canvas](preview.md). Every edit repaints instantly;
`bind` props write through to the document's `view.state`, and `on <event>`
hooks log to the hook sink rather than running Rust.

The **Vars** tab (`forge.Vars`) lists every `{$path}` variable bound in the
document — edit a scalar and watch the app react; it's the same state the
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

This is how bases without an in-process twin preview from Studio: `ratatui`
runs on a real `TestBackend` and streams its cell grid back as `preview.frame`
(the TUI text you see under the canvas), `iced`/`gpui02`/`adabraka`/`bezel`
open real native windows, and `gpui-kit` itself can run in-host when you want
the genuine platform event loop.

The status bar's `host` label shows whether a host is attached; clicking the
preview raises the host window.

## `base_dir`

Studio forwards the open document's directory to the host so `src:` props and
other relative asset paths resolve — one more reason Open Project (a
`forge.toml`) is the most faithful way to preview.
