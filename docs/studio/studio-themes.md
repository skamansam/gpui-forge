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
- **Showcase** — a `forge.Showcase` widget strip rendering beside the editor,
  so you see every component in the theme as you tweak it.

Studio watches the `themes/` directory — save a theme JSON on disk and the
registry reloads it without a restart.

## Persisted preferences

`~/.gpui-forge/settings.toml` holds per-user settings — currently the theme
`name` and the `dark` flag. Studio loads them at startup and applies the saved
theme over the OS light/dark default.

## Where themes come from

1. `default-theme.json` — baked in, light + dark.
2. `themes/*.json` — the vendored catalog, loaded at startup and watched.
3. The OS appearance — the fallback before a preference exists.

Theme *documents* use `$theme.<token>` in `style:` values — see the
[style & layout model](style-layout.md).
