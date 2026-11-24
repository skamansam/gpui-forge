# Editing TUIs in Studio

Studio is a visual builder for terminal UIs. A document with
`base: ratatui` gets a live cell canvas you can click, drag onto, and
inspect. The same editing loop the GPUI bases get now applies to a
terminal grid. TUI frameworks ship widgets and a render loop. None of
them ships a graphical editor. Studio lets you assemble a TUI app by
dragging widgets onto a live terminal.

## The canvas is the real renderer

The `ratatui-embed` feature compiles `render::ratatui` into Studio.
Open a ratatui document and the canvas card renders it in-process into
a ratatui `TestBackend` buffer. `forge-term`'s `GridView` paints the
cells in the theme's mono font. What you see is the exact grid the
terminal backend produces — box-drawing borders, widget glyphs, ANSI
palette. No emulator and no host process sit between the document and
the card.

The canvas mirrors the document state every frame. Palette inserts,
source edits, inspector changes and undo all repaint on the next frame.

## Drag, drop, select

The full editing contract works on the cell grid.

- **Palette drag** — drag `Slider`, `Tabs`, `Gauge` or any catalog
  element onto the canvas. The drop position hit-tests against the
  rendered zones, so a widget lands inside the block you aimed at.
- **Click to select** — clicking a cell selects the node that owns it.
  A cyan frame paints over the element's zone and the outline and
  inspector follow. The crosshair pick mode works the same way.
- **Move nodes** — drag a selected node to a new position. The move
  commits through the same path the outline uses.

## The inspector knows the widgets

The inspector resolves prop definitions from the document's own base
catalog. A ratatui `Spinner` shows `kind`, `step` and `symbol` — not
the gpui-kit prop set. Typed editors still apply: enums become
dropdowns and catalog validation flags unknown props.

## Live widgets

Input on the canvas goes straight to the in-process view. Bound props
drive the widget state, so the canvas is a working TUI, not a static
frame.

- `Input` fields take typing once focused.
- `Select`, `Tabs` and `Tree` respond to arrows and clicks.
- `Slider` drags, `Scrollbar` tracks wheel events, `Dialog` commits
  choices.

`Tab` cycles focusables in document order. The widget contract is in
[ratatui-widgets.md](../format/ratatui-widgets.md).

## Native-only widgets

`Slider`, `PieChart`, `BigText`, `Globe`, `CodeEditor` and `Markdown`
pull native-only dependencies. They render in Studio's native build.
On the web canvas (`forge-web-canvas` for wasm32) the same elements
paint a placeholder.

## The remote host still exists

`forge-preview-ratatui` remains for cases that need a real terminal —
it draws through `CrosstermBackend` and streams ANSI frames back over
the preview protocol. A document opts in by declaring a
`forge.Preview` pane with `mode: stream`. Studio's canvas no longer
needs it.
