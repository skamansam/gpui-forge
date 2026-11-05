//! The hook registry: `#[forge_hook]`-marked functions register themselves at
//! link time via `inventory`, so applications get hook dispatch without a
//! hand-written match.
//!
//! The consumer declares its context once: `type HookCtx = MyState;` — the
//! generated submit wrappers downcast `&dyn Any` to it.

use std::any::Any;

use gpui_forge_runtime::base::kit::prelude::Context;
use gpui_forge_runtime::render::ForgeView;

/// One registered hook — a `name` plus a wrapper that adapts the erased
/// context to the function's real signature.
pub struct HookDef {
    pub name: &'static str,
    pub call: fn(&mut ForgeView, &dyn Any, &[String], &mut Context<ForgeView>),
}

impl HookDef {
    pub const fn new(
        name: &'static str,
        call: fn(&mut ForgeView, &dyn Any, &[String], &mut Context<ForgeView>),
    ) -> Self {
        Self { name, call }
    }
}

inventory::collect!(HookDef);

/// Fires `name` with `args` against the registry. Returns true when a hook
/// handled it — callers fall back to `on_hook`/`open_dialog` resolution.
pub fn dispatch<Ctx: 'static>(
    ctx: &Ctx,
    view: &mut ForgeView,
    name: &str,
    args: &[String],
    cx: &mut Context<ForgeView>,
) -> bool {
    for h in inventory::iter::<HookDef> {
        if h.name == name {
            (h.call)(view, ctx, args, cx);
            return true;
        }
    }
    false
}
pub use inventory;

/// Typed context for hooks (#56) — wraps `&mut ForgeView` + `&mut Context`
/// with the helpers every handler reimplements today: state get/set by path
/// string, document and preview access. Opt-in: `HookCtx::new(view, cx)`
/// inside any `#[forge_hook]` or `register_hook` handler.
pub struct HookCtx<'a> {
    pub view: &'a mut ForgeView,
    pub cx: &'a mut Context<'a, ForgeView>,
}

impl<'a> HookCtx<'a> {
    pub fn new(view: &'a mut ForgeView, cx: &'a mut Context<'a, ForgeView>) -> Self {
        Self { view, cx }
    }

    /// `ctx.set("studio.dirty", json!(true))` — state write by path string.
    pub fn set(&mut self, path: &str, value: serde_json::Value) {
        self.view
            .update_state(&parse_path_lossy(path), value, self.cx);
    }

    /// `ctx.get("studio.dirty")` — state read by path string.
    pub fn get(&self, path: &str) -> Option<&serde_json::Value> {
        get_state_path(self.view.state(), path)
    }

    /// `ctx.get_str("studio.file")` — string-valued state.
    pub fn get_str(&self, path: &str) -> Option<&str> {
        self.get(path).and_then(|v| v.as_str())
    }

    /// The document this view hosts (the shell doc for Studio, the loaded
    /// doc for a preview view).
    pub fn document(&self) -> Option<&gpui_forge_runtime::forge_spec::Document> {
        self.view.document()
    }

    /// A named preview entity (e.g. `"canvas"`) — creating it on first use.
    pub fn preview(&mut self, id: &str) -> gpui_forge_runtime::base::kit::Entity<ForgeView> {
        self.view.preview(id, self.cx)
    }

    /// Mark the view dirty so listeners re-render.
    pub fn notify(&mut self) {
        self.cx.notify();
    }
}

/// `parse_path` that never fails the caller — unknown paths map to a root path
/// so a malformed hook arg can't panic the window.
fn parse_path_lossy(path: &str) -> gpui_forge_runtime::forge_spec::Path {
    gpui_forge_runtime::forge_spec::value::parse_path(path).unwrap_or_else(|_| {
        gpui_forge_runtime::forge_spec::value::parse_path("__invalid__").unwrap()
    })
}

/// Walks `state` by dotted path — `document.selected` → `state["document"]["selected"]`.
fn get_state_path<'a>(state: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut node = state;
    for seg in path.split('.') {
        node = node.get(seg)?;
    }
    Some(node)
}
