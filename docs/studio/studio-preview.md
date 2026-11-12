# Previewing in Studio

The Canvas tab hosts `forge.Preview#canvas` — it can paint the document
itself or act as a window into a separate preview process.

## Embedded canvas

By default the canvas is the in-process render twin — the same `ForgeView`
that powers the [web canvas](preview.md). Every edit repaints instantly.
`bind` props write through to the document's `view.state`, and `on <event>`
hooks log to the hook sink rather than running Rust.

The **State** tab (`forge.Vars`) lists every `{$path}` variable bound in the
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
draws through a real `CrosstermBackend` into a byte sink and streams the ANSI
output — the same bytes a terminal would get — back as `preview.frame`
chunks; Studio folds them through `forge-term`'s `alacritty_terminal`
emulator and paints styled cells in the card. `iced`/`gpui02`/`adabraka`/
`bezel` open real native windows, and `gpui-kit` itself can run in-host when
you want the genuine platform event loop.

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
`examples/demos/previews-demo.forge.yaml` for a working selector.

Studio's own Theme tab uses the same mechanism — `studio.forge.yaml`'s
`previewSelector` binds a `Select` to `$studio.theme_preview` and each
`case` picks one of three presentations:

- **inline** — same-generation `gpui-kit` documents (gpui-kit itself and
  the ely/hig/gc/gflow/math/pdf/gpui-base demos) render in a nested
  `forge.Preview` inside the preview area, no host process at all.
- **message card** — foreign-base windowed hosts (gpui02, bezel,
  adabraka, iced) get a plain `remote:` pane whose card says the preview
  lives in a separate window; a different GPUI generation can't
  composite into Studio's scene graph.
- **streamed terminal** — `mode: stream` routes the ratatui host's
  `frame` events (`{seq, cols, rows, data}` — raw ANSI bytes from the
  real `CrosstermBackend`) into `preview.panes.<id>.frame`. The card
  feeds new chunks into a `forge-term` `Emulator` (alacritty's `vte`
  parser → styled cell grid) and paints it with `TermView`: per-row
  background quads plus shaped text runs, in the theme's mono font.
  Colors, bold, dim, inverse — whatever the TUI emits is what shows.
  The grid is sized from the slot's `frame_rect`.

The Rust side only maintains state and plumbing; the document decides
which bases exist and which preview is live. Panes are also gated behind
the Theme tab (`if: $studio.tab / is: 4`), so no host spawns before its
preview is on screen.

## Theme propagation

Every `theme` command carries three things: the theme `name`, the `dark`
flag, and `colors` — Studio's resolved `ThemeColor` palette serialized
to `{field: "#rrggbbaa"}` hex. Hosts that can't resolve a Forge theme
name can still paint with its actual values. Each base maps what its
theme API accepts:

- **gpui-kit** resolves the name in its own `ThemeRegistry` (keeping
  token gradients); when the name misses it deserializes `colors` back
  into the same `ThemeColor` type — byte-identical values.
- **gpui02** maps semantic overrides onto guise: `accent→primary`,
  `background→body`, `popover→surface`, `muted→surface_hover`,
  `foreground→text`, `muted_foreground→dimmed`, `border`, and
  `success`/`warning`/`danger`/`info` to their named slots.
- **iced** builds `Theme::custom` from `background`/`text`/`primary`/
  `success`/`danger` over a dark- or light-base palette.
- **bezel** and **adabraka** toggle dark/light only — their theme APIs
  accept no custom colors.
- **ratatui** ignores the command — its base renderer hardcodes ANSI
  colors per widget, which is honest: a real terminal shows the TUI's
  own palette, and the emulated stream displays exactly that.

## Pane lifecycle

Hosts spawn when a rendered `remote:` pane enters the document and are
killed when it leaves — including every pane host on Studio's own exit,
so nothing orphans across runs. A host that exits is respawned, but two
consecutive deaths mark it unusable and the status bar reports the give-
up; switching to a different pane and back resets the count. A host that
stays alive but never maps a window is caught the same way: three
seconds after spawn, Studio probes X11 for a top-level window carrying
the child's `_NET_WM_PID` and kills the zombie with a "opened no window"
status when none exists (old-gpui's event loop keeps running even when
window creation fails, so an alive process alone is not proof a window
opened).

## `base_dir`

Studio forwards the open document's directory to the host so `src:` props and
other relative asset paths resolve. One more reason Open Project (a
`forge.toml`) is the most faithful way to preview.
