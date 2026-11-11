//! Adapter for `gpui-apple-hig` (`use: hig: gpui-apple-hig` → `hig.*` elements)
//! — #90. macOS-HIG components on the gpui-kit base. Render-only arms like the
//! Ely adapter (hook wiring is follow-up work).

use gpui_apple_hig::badge::MacBadge;
use gpui_apple_hig::button::MacButton;
use gpui_apple_hig::checkbox::MacCheckbox;
use gpui_apple_hig::progress::{MacProgress, MacSpinner};
use gpui_apple_hig::slider::MacSlider;
use gpui_apple_hig::switch::MacSwitch;
use gpui_forge_runtime::base::kit::*;
use gpui_forge_runtime::forge_spec::catalog::{self, Catalog};
use gpui_forge_runtime::forge_spec::{Node, Value};
use gpui_forge_runtime::render::{ForgeLibrary, ForgeView, get_path};
use std::sync::OnceLock;

/// `hig.*` adapter — register with `view.register_library(Hig)`.
pub struct Hig;

impl ForgeLibrary for Hig {
    fn alias(&self) -> &'static str {
        "hig"
    }

    fn catalog(&self) -> Option<&Catalog> {
        static CAT: OnceLock<Catalog> = OnceLock::new();
        Some(CAT.get_or_init(|| {
            catalog::from_yaml(include_str!("../../../catalogs/hig.catalog.yaml"))
                .expect("hig catalog parses")
        }))
    }

    fn render(
        &self,
        node: &Node,
        key: &str,
        view: &ForgeView,
        _window: &mut Window,
        _cx: &mut Context<ForgeView>,
    ) -> Option<AnyElement> {
        match node.element.name.as_str() {
            "Button" => {
                let mut b = MacButton::new(key.to_string()).label(prop_str(node, "label", view));
                if prop_bool(node, "prominent", view, false) {
                    b = b.prominent();
                }
                if prop_bool(node, "borderless", view, false) {
                    b = b.borderless();
                }
                if prop_bool(node, "destructive", view, false) {
                    b = b.destructive();
                }
                if prop_bool(node, "disabled", view, false) {
                    b = b.disabled(true);
                }
                Some(b.into_any_element())
            }
            "Switch" => {
                let mut s = MacSwitch::new(key.to_string())
                    .checked(prop_bool(node, "checked", view, false));
                if prop_bool(node, "disabled", view, false) {
                    s = s.disabled(true);
                }
                if let Some(label) = prop_str_opt(node, "label", view) {
                    s = s.label(label);
                }
                Some(s.into_any_element())
            }
            "Checkbox" => {
                let mut c = MacCheckbox::new(key.to_string())
                    .checked(prop_bool(node, "checked", view, false))
                    .label(prop_str(node, "label", view));
                if prop_bool(node, "disabled", view, false) {
                    c = c.disabled(true);
                }
                Some(c.into_any_element())
            }
            "Slider" => {
                let value = prop_num(node, "value", view).unwrap_or(0.0) as f32;
                let mut s = MacSlider::new(key.to_string()).value(value);
                let (min, max) = (
                    prop_num(node, "min", view).unwrap_or(0.0) as f32,
                    prop_num(node, "max", view).unwrap_or(100.0) as f32,
                );
                s = s.range(min, max);
                if let Some(step) = prop_num(node, "step", view) {
                    s = s.step(step as f32);
                }
                Some(s.into_any_element())
            }
            "Progress" => Some(
                MacProgress::new(key.to_string())
                    .value(prop_num(node, "value", view).unwrap_or(0.0) as f32)
                    .into_any_element(),
            ),
            "Spinner" => Some(MacSpinner::new(key.to_string()).into_any_element()),
            "Badge" => Some(
                MacBadge::new(key.to_string(), prop_str(node, "text", view)).into_any_element(),
            ),
            _ => None,
        }
    }
}

fn prop<'a>(node: &'a Node, name: &str) -> Option<&'a Value> {
    node.props.iter().find(|(k, _)| k == name).map(|(_, v)| v)
}

fn prop_str_opt(node: &Node, name: &str, view: &ForgeView) -> Option<String> {
    match prop(node, name)? {
        Value::String(s) | Value::Enum(s) => Some(s.clone()),
        Value::Binding { path, .. } => get_path(view.state(), path).map(|j| match j {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        }),
        _ => None,
    }
}

fn prop_str(node: &Node, name: &str, view: &ForgeView) -> String {
    node.content
        .as_ref()
        .and_then(|v| match v {
            Value::String(s) | Value::Enum(s) => Some(s.clone()),
            _ => None,
        })
        .or_else(|| prop_str_opt(node, name, view))
        .unwrap_or_default()
}

fn prop_bool(node: &Node, name: &str, view: &ForgeView, fallback: bool) -> bool {
    match prop(node, name) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Binding { path, .. }) => get_path(view.state(), path)
            .and_then(|j| j.as_bool())
            .unwrap_or(fallback),
        _ => fallback,
    }
}

fn prop_num(node: &Node, name: &str, view: &ForgeView) -> Option<f64> {
    match prop(node, name) {
        Some(Value::Number { value, .. }) => Some(*value),
        Some(Value::Binding { path, .. }) => get_path(view.state(), path).and_then(|j| j.as_f64()),
        _ => None,
    }
}
