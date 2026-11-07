//! Adapter for `gpui-component` (`use: gc` → `gc.*` elements on base-gpui02).

use gpui_forge_runtime::base::kit::gpui::*;
use gpui_forge_runtime::forge_spec::catalog::{self, Catalog};
use gpui_forge_runtime::forge_spec::{Node, Value};
use gpui_forge_runtime::render::{ForgeLibrary, ForgeView};
use std::sync::OnceLock;

/// `gc.*` adapter — register with `view.register_library(GpuiComponent)`.
pub struct GpuiComponent;

impl ForgeLibrary for GpuiComponent {
    fn alias(&self) -> &'static str {
        "gc"
    }

    fn catalog(&self) -> Option<&Catalog> {
        static CAT: OnceLock<Catalog> = OnceLock::new();
        Some(CAT.get_or_init(|| {
            catalog::from_yaml(include_str!("../../../catalogs/gc.catalog.yaml"))
                .expect("gc catalog parses")
        }))
    }

    fn render(
        &self,
        node: &Node,
        key: &str,
        _view: &ForgeView,
        _window: &mut Window,
        _cx: &mut Context<ForgeView>,
    ) -> Option<AnyElement> {
        match node.element.name.as_str() {
            // `gc.*` arms are honest placeholders — `gpui-component`'s
            // widgets produce `gpui::AnyElement`, not gpui-pre's; real
            // widget arms need the adapter's gpui types unified with
            // gpui-component's (a deeper dep-resolution piece).
            name => Some(
                div()
                    .text_sm()
                    .child(format!("gc.{name} (gpui-component adapter)"))
                    .into_any_element(),
            ),
        }
    }
}

fn prop_str(node: &Node, name: &str) -> String {
    prop_str_opt(node, name).unwrap_or_default()
}

fn prop_str_opt(node: &Node, name: &str) -> Option<String> {
    node.props
        .iter()
        .find(|(k, _)| k == name)
        .and_then(|(_, v)| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        })
}
