# Forge Studio

`cargo xtask studio` opens the native workbench: the same document model as the
web editor, plus real windows, host previews, themes, files on disk and the CLI
toolchain.

## Starting points

The Welcome tab offers starters: the Welcome screen, the gpui-kit component
gallery, a blank document, and a widget-set Select that opens the showcase for
any registered set (`ely`, `gc`, `gflow`, `base`, `video`, `hig`, `pdf`, `wb`,
`math`). The other bases' demos (`ratatui`, `iced`, `bezel`, `gpui02`,
`adabraka`) ship in `examples/` — open them with Open…. File → New gives an
empty document; File → New Project… opens a scaffolded document *without*
asking for a folder — a project is one YAML file, so a path is only needed
when you save or build.

## Editing

- **Palette** — click or drag catalog elements onto the preview; they land
  inside the selected tree node.
- **Outline** — the node tree: select, reorder, delete, undo/redo.
- **Inspector** — typed prop/style editors; the catalog drives validation.
- **Source pane** — the YAML itself, kept in sync with palette edits.
  Squiggles and the Problems panel show catalog diagnostics (line, column,
  hint) on every keystroke (#128).
- **Dialogs** — `Dialog#name:` declarations open as real windows
  (`open_dialog("name")` from a menu/button), with their own bound state in
  `dialog.state`.

## Preview

- **Embedded canvas** — the in-process twin paints every edit; bindings write
  `view.state`, hooks log to the hook sink.
- **Preview in Host** (File → Preview in Host…) — spawns the out-of-process
  `forge-preview-<base>` binary, which runs the doc through its *native* event
  loop and reports events/state over a JSON channel. This is how remote bases
  (ratatui on a real terminal, iced, gpui02) preview from Studio: unsaved
  buffers stream over `source`, saved docs over `load`.

## Themes and preferences

The title-bar theme Select merges the shipped `themes/*.json` set plus
light/dark mode and applies live (`~/.gpui-forge/settings.toml` persists the
pick). Theme JSON is editable in the Themes editor; the Themes dir is watched
so edits reload. Preferences… dialog holds app options.

## Files

Open… / Open Project… pick a `.forge.yaml` or a `forge.toml` (the manifest's
`forge.document` resolves the doc and `dir` for relative asset paths). Recent
files live in the File menu. Save writes in place; unsaved docs go through
Save As — format keeps anchors and comments where `forge fmt` would.

## CLI companions

| Command | What it does |
|---|---|
| `forge check <file>` | Validate a doc against its base catalog (line/col diagnostics) |
| `forge fmt <file>` | Canonical formatting, preserving comments |
| `forge new <name> [--base <id>]` | Scaffold `forge.toml` + `app.forge.yaml` |
| `forge build <file>` | Emit a standalone Cargo app from a document |
| `forge preview <file>` | Launch the native preview host for a doc |
