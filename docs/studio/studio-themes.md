# Themes and preferences

Studio ships 21 vendored theme JSONs (`themes/`) plus the built-in
light/dark pair. Its own UI is a Forge document, so every theme applies
to the workbench *and* the document you're previewing.

## Picking a theme

The title-bar **Select** lists every registered theme, grouped under
`--- Light Themes ---` / `--- Dark Themes ---` separators. Choosing one calls
`apply_theme_name` and repaints everything live — no restart.

The Preferences dialog (**File → Preferences…**) has a Theme page with the
same picker, which is also where theme choice gets persisted.

## The Theme tab

A live theme workbench:

- **Select** — pick a theme to edit.
- **JSON editor** — the theme's color/token config, editable. **Apply**
  pushes it through `Theme.apply_config` and the whole app repaints.
- **Reset** (the `rotate-ccw` button) — back to the default theme.
- **Showcase** — a preview of the theme across bases, split off the editor by
  a draggable `Resizable` divider. The base `Select` picks which widget set
  renders; the arrow button beside it cycles every base for a fast sanity
  pass.

## Theme fidelity per base

Not every base can honor a full theme — the showcase says so under each
foreign card instead of silently degrading:

- **native** — gpui-kit and the same-process libraries paint the theme
  directly.
- **approximate** — gpui02 and iced map the resolved palette onto their own
  semantic slots (guise `with_*` overrides, iced `Theme::custom`).
- **dark/light only** — bezel and adabraka accept no custom colors. Their
  cards carry a small `follow studio / dark / light` selector — a per-pane
  pin, sent as `{"cmd":"theme","dark":…}` to that pane only.
- **unmapped** — ratatui's card shows the raw terminal stream; ANSI colors
  come from the app, not the theme.

## Persisted preferences

`~/.gpui-forge/settings.toml` holds per-user settings — the theme `name`, the
`dark` flag, and `theme_preview` (the last previewed base). Studio loads
them at startup and applies the saved theme over the OS light/dark default.

## Where themes come from

1. `default-theme.json` — baked in, light + dark.
2. `themes/*.json` — the vendored catalog, loaded at startup and watched.
3. The OS appearance — the fallback before a preference exists.

Theme *documents* use `$theme.<token>` in `style:` values — see the
[style & layout model](style-layout.md).
