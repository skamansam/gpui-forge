# Forge Studio

Forge Studio is the native workbench. It uses the same document model as the
[web editor](editor.md), plus real windows, host previews, themes, files on
disk, and the [CLI toolchain](forge-cli.md). Studio itself is a Forge
document (`crates/forge-studio/studio.forge.yaml`) rendered by its own
runtime, so every panel you see is an example of the format.

## Launching

```sh
cargo xtask studio            # from the workspace — builds and runs it
forge studio                  # via the CLI (alias: forge run)
cargo run -p forge-studio -- <starter|doc.forge.yaml>
```

The optional argument names a starter (`welcome`, `gallery`, `blank`, or a
widget set like `ely`) or a `.forge.yaml` path to open.

## The window

```
┌─ menu bar ─ File: New · New Project… · Open… · Open Project… · Preview in Host… · Format · Undo · Redo · Save · Preferences…
│
│ palette        Tabs: Start | Document | Canvas | State | Theme          inspector
│ (230px)      ┌─────────────────────────────────────────────────────┐   (280px)
│ click/drag   │  the document — source text, live canvas, state,    │   Structure outline
│ to insert    │  theme editor                                        │   typed props
│              │                                                      │   Diagnostics
│              └─────────────────────────────────────────────────────┘   View state
└─ status bar ── message · project dir · host · file · spec/base tags
```

- **[Editing](studio-editing.md)** — palette, outline, inspector, source pane,
  diagnostics, undo/redo, pick mode.
- **[Preview](studio-preview.md)** — the embedded canvas vs *Preview in Host*,
  remote bases, the State tab.
- **[Themes & preferences](studio-themes.md)** — theme picker, the theme JSON
  editor, persisted settings.
- **[Projects & files](studio-projects.md)** — New vs New Project, `forge.toml`,
  save/save-as, recents, file watching, dialogs.

## Entry points

The Start tab offers starters: the Start screen, the gpui-kit component
gallery, and a blank document. A widget-set Select opens the showcase for
any registered set (`ely`, `gc`, `gflow`, `base`, `video`, `hig`, `pdf`, `wb`,
`math`). The other bases' demos (`ratatui`, `iced`, `bezel`, `gpui02`,
`adabraka`) ship in `examples/showcases/` — open them with Open….
