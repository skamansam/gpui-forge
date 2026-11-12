# Writing a Forge adapter

An **adapter** (`forge-lib-*` crate) lets a Forge document use a third-party
GPUI component library by namespace. A document `use:`-imports the library
and renders `alias.Element` elements that the adapter maps onto real widgets.

```
use:
  ely: ely-core        # alias → library id
root:
  ely.Button: Click me   # rendered by forge-lib-ely
```

## The contract — `ForgeLibrary`

Defined in `gpui-forge-runtime` (`crates/gpui-forge-runtime/src/render.rs`):

```rust
pub trait ForgeLibrary {
    /// The import alias this adapter serves ("ely").
    fn alias(&self) -> &'static str;
    /// Optional embedded catalog — merged into validation + palette metadata.
    fn catalog(&self) -> Option<&Catalog> { None }
    /// Map a `alias.Element` node to a rendered element.
    /// Return `None` to fall through to the built-in catalog.
    fn render(&self, node: &Node, key: &str, view: &ForgeView,
              window: &mut Window, cx: &mut Context<ForgeView>)
              -> Option<AnyElement> { None }
}
```

Register once at app start:

```rust
let mut view = ForgeView::new(check::catalogs());
view.register_library(forge_lib_ely::Ely);
```

`register_library` merges the adapter's catalog (so `use: alias: lib` validates,
props validate, and the Studio palette learns the elements) and routes
namespaced nodes through `render()`.

## The catalog manifest — `catalogs/<lib>.catalog.yaml`

Declares what the library exposes so validation and tooling know it:

```yaml
catalog: "0.1"
library: ely-core          # id used in `use: alias: <id>`
name: Ely Components
version: 0.2.0
license: MIT               # surfaces in `forge license`
bases:
  gpui-kit: "^0.8"         # which base(s) this library mixes with

elements:
  Button:
    summary: Ely button.
    component: ely_core::buttons::Button   # doc-only: where it lives upstream
    content: label                         # positional content maps to `label` prop
    children: none                         # none | any | list
    props:
      label: string
      variant: {type: enum, values: [Primary, Secondary, Danger]}
      disabled: {type: bool, default: false}
    events:
      click:
```

Ship it with the adapter crate (`include_str!`) and add it to
`check::catalogs()` in `gpui-forge-runtime/src/check.rs` so `use:` imports
resolve during `forge check`.

## The render shim

`render()` reads the node's content/props (small helpers for `Value`) and
constructs the upstream widget:

```rust
"Button" => {
    let mut b = Button::new(key.to_string(), prop_str(node, "label"));
    if let Some(v) = prop_str_opt(node, "variant") { b = b.variant(...); }
    if prop_bool(node, "disabled") { b = b.disabled(true); }
    Some(b.into_any_element())
}
```

`key` is a stable per-node string — use it as the GPUI element id.

## Checklist for a new adapter

1. `cargo add` the upstream crate — it must link the same `gpui-pre` version
   as the base (the gpui-kit base pins `0.3.10`). Mismatches won't compile.
2. Write `catalogs/<lib>.catalog.yaml` per the manifest schema
   ([catalog manifest](catalog-manifest.md)).
3. Create `crates/forge-lib-<name>` implementing `ForgeLibrary`
   (see `crates/forge-lib-ely/` — ~200 lines).
4. Add the catalog to `check::catalogs()`. Add the crate to the workspace.
5. Register in the host app (`forge-studio` already registers `Ely`).
6. Add a demo `examples/demos/<lib>-demo.forge.yaml` and make `forge check` pass.
7. `forge license <demo>` should report the library's declared license —
   copyleft licenses (GPL/AGPL) stay opt-in, never linked into defaults.

## Current limits

- **Events are not wired.** `on click:` on adapter elements validates but the
  hook isn't invoked — adapters render visual-only elements today.
- **Stateful widgets need entities.** Components whose constructor takes a
  GPUI `Entity`/state (inputs, list models) need the adapter to own state —
  the v1 shim is stateless per render.
- **Children aren't mapped.** Elements that take child components
  (`children: any` in the catalog) render their own frame but don't yet
  receive the node's Forge children. Adapters that want them must walk
  `node.children` and `view.render_node` them (a helper is a follow-up).
- **License policy:** GPL-family libraries stay opt-in (separate crate,
  explicit `use:`), never linked into Studio or generated apps by default.

## Reference implementations

- `crates/forge-lib-ely` — Ely Components (`use: ely: ely-core`), 30+ elements
- `crates/forge-lib-hig` — Apple HIG (`use: hig: gpui-apple-hig`), vendored port
- `crates/forge-lib-base` — Base GPUI headless components (`use: base: base-gpui`)
- `crates/forge-lib-flow` — gpui-flow node editor (`use: gflow: gpui-flow`)
- `docs/design/multi-base.md` — the adapter RFC and multi-base design
