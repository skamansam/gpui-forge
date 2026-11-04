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
