# Forge Studio

Forge Studio is the native workbench. It uses the same document model as the
[web editor](editor.md), plus real windows, host previews, themes, files on
disk, and the [CLI toolchain](forge-cli.md).

## Built on itself

Studio runs on its own document end-to-end: its entire UI is a Forge
document, `crates/forge-studio/studio.forge.yaml`. `include_str!` bundles
the ~850 lines into the binary, and the same `ForgeView` that renders
your documents renders them at startup. The shell
declares `use: forge: forge` and passes the spec suite's validation tests,
so every panel in the window doubles as a living example of the format:

- the canvas is `forge.Preview` — a live document render nested inside
  another document.
- the palette, outline, inspector, project sidebar and state tab are
  `forge.Palette`, `forge.Outline`, `forge.Inspector`, `forge.Files` and
  `forge.Vars` — the same catalog elements any document can use.
- the showcase tabs nest more `forge.Preview` elements, so other bases'
  demos render inside the shell doc.
- menu items are `on click: name()` calls dispatched to Rust hooks
  registered in `main.rs`.
- Preferences, New Project and Rename are `Dialog#` blocks declared in the
  shell's `dialogs:` — `open_dialog("rename")`/`close_dialog()` are hooks,
  not windowing code.

You can open `studio.forge.yaml` in Studio itself and watch the editor edit
its own shell. When the format can't express what Studio needs, we extend
the format rather than work around it.

## Starting

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
│ project      Tabs: Start | Document | Canvas | State | Theme          inspector
│ (resizable)  ┌─────────────────────────────────────────────────────┐   (280px)
│ .forge.yaml  │  the document — source text, live canvas, state,    │   Structure outline
│ files, click │  theme editor                                        │   typed props
│ to open      │  (the palette docks beside the Canvas tab now)       │   Diagnostics
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
`math`). The other bases' demos (`ratatui`, `iced`, `bezel`, `guise`,
`adabraka`) ship in `examples/showcases/` — open them with Open….
