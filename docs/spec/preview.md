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
- **Styles:** the canvas applies the keys of the [style & layout model](style-layout.md);
  `overflow: scroll` is drawn as `hidden`, and `border` draws a 1px border whatever its width.
