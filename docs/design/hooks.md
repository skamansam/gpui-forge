# The Bridge — hook system design (#53)

Forge documents declare behavior, not code: `on click: hook('arg')` on an
element. The bridge resolves each hook name to a Rust closure at runtime.

## Document syntax

```yaml
- Button: Save
  on click: save_file()                # event → hook call
  value: bind $form.name               # two-way state binding
- forge.Outline#structure:
  of: canvas
  on select: select_node()             # intrinsic events fire the same way
```

Args are single-quoted strings. `bind $path` two-way-syncs a prop with
`document.state`. `$state.path` interpolation reads state in text/props.

## Resolution order

When a node fires an event, `ForgeView::fire` looks up the handler:

1. **Hook registry** — `ForgeView::register_hook(name, handler)` handlers and
   `#[forge_hook]`-registered `HookDef`s (collected at link time via
   `inventory`) come first. A `HookDef` wraps a
   `fn(&[String], &mut ForgeView, &mut Context<ForgeView>)` — args, the view
   (state writes, document mutation, preview handles), and the GPUI context.
2. **`on_hook` sink** — if no registry handler claims the call, a raw
   `"name(arg, arg)"` string goes to the host-provided sink. Studio uses this
   as the fallback dispatcher for its bespoke hooks.

`fire` is panic-safe: handler panics are caught and surfaced as view errors
(`forge.Preview` shows them), never kill the window.

## Authoring a hook

```rust
#[forge_hook]
fn open_url(args: &[String], view: &mut ForgeView, cx: &mut Context<ForgeView>) {
    if let Some(url) = args.first() {
        view.update_state(&parse_path("app.url").unwrap(), Json::String(url.clone()), cx);
    }
}
```

The `forge-hooks` crate collects every `#[forge_hook]` fn into the
inventory registry. The host app calls `forge_hooks::dispatch` before its
own fallback. `forge hooks <doc>` emits the JSON manifest of hooks a
document requires — the contract a host must satisfy.

## State writes

Hooks mutate state through `view.update_state(&Path, Json, cx)` — the same
path `bind $` writes to. `studio.*` keys are Studio-internal conventions
(`palette_lib`, `drop_target`, `dirty`, `selected`). `document.*` is the
host doc's state. Document structure mutations go through
`Document::{insert_node, remove_node, reparent, move_after}` on the
preview's document — Studio's `record_undo` snapshots before each.

## Open gaps

- **Async hooks** — `HookCtx::spawn` launches a GPUI `Task` from a handler
  (file I/O, network, timers). The task updates the view through its weak
  entity when it finishes.
- **No structured context** — hooks get `&mut ForgeView` today. A typed
  `HookCtx` (state reader/writer + preview handles + diagnostics) is #56.
- **Library-element events unwired** — adapter `on click:` validates but
  `ForgeLibrary::render` can't reach the fire path yet.
