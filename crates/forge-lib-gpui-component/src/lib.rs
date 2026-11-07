//! Adapter for `gpui-component` (`use: gc` → `gc.*` elements on base-gpui02).

use gpui_component::alert::Alert;
use gpui_component::badge::Badge;
use gpui_component::button::Button;
use gpui_component::separator::Separator;

use gpui_component::label::Label;
use gpui_component::progress::Progress;
use gpui_component::spinner::Spinner;

use gpui_forge_runtime::base::kit::*;
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
            "Button" => Some(
                Button::new(key.to_string())
                    .label(prop_str(node, "label"))
                    .into(),
            ),
            "Badge" => Some(
                Badge::new()
                    .count(prop_str(node, "count").parse::<usize>().unwrap_or(0))
                    .into_element()
                    .into_any_element(),
            ),
            "Alert" => Some(
                Alert::new(key.to_string(), prop_str(node, "message"))
                    .title(prop_str(node, "title"))
                    .into_element()
                    .into_any_element(),
            ),
            "Spinner" => Some(Spinner::new().into_element().into_any_element()),
            "Progress" => Some(
                Progress::new(key.to_string())
                    .value(prop_f32(node, "value"))
                    .into_element()
                    .into_any_element(),
            ),
            "Divider" => Some(Separator::horizontal().into_element().into_any_element()),

            "Label" => Some(
                Label::new(prop_str(node, "label"))
                    .into_element()
                    .into_any_element(),
            ),

            _ => None,
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

fn prop_f32(node: &Node, name: &str) -> f32 {
    node.props
        .iter()
        .find(|(k, _)| k == name)
        .and_then(|(_, v)| match v {
            Value::Number { value, .. } => Some(*value as f32),
            _ => None,
        })
        .unwrap_or(0.0)
}
