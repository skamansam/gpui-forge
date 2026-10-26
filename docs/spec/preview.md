# Preview support

The docs site renders Forge documents with the real gpui-kit components, compiled to WebAssembly
(`crates/forge-web-canvas`). It is an early slice of the gpui-kit adapter
([#75](https://github.com/skamansam/gpui-forge/issues/75)), so a few things are limited:

- **Elements:** every element in the gpui-kit catalog renders. Elements from imported libraries
  (`ely.Badge`) show a placeholder.
- **Props not applied yet:** `Input.readonly`, `Select.searchable` and `Select.cleanable`.
- **Bindings:** `$path` reads from the example's sample state; `bind $path` writes back to it, and
  the playground shows the state as it changes. Hooks (`on click: sign_in()`) are logged instead of
  running Rust.
- **Styles:** the style and layout model is still being designed
  ([#5](https://github.com/skamansam/gpui-forge/issues/5)). Until then the preview understands this
  provisional set:

| Key | Values |
|---|---|
| `gap`, `padding`, `width`, `height`, `max-width` | lengths: `12px`, `1.5rem`, `50%` |
| `radius` | `px` or `rem` length |
| `align` | `start`, `center`, `end`, `stretch` (cross axis) |
| `justify` | `start`, `center`, `end`, `between` (main axis) |
| `grow`, `wrap` | `true` |
| `background`, `color`, `border` | `"#rrggbb"` or `$theme.primary`, `$theme.muted_foreground`, … |
| `size` | `xs`, `sm`, `md`, `lg`, `xl`, `2xl`, `3xl`, or a number of pixels |
| `weight` | `normal`, `medium`, `semibold`, `bold` |
