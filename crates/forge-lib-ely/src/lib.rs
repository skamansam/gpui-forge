//! Adapter for Ely GPUI Components (`use: ely: ely-core` → `ely.*` elements).

use ely_core::buttons::{Button, ButtonVariant, IconButton};
use ely_core::data_display::Badge;
use ely_core::feedback::{Alert, StatusMessage};
use ely_core::primitives::IconName;
use ely_core::primitives::{Divider, Severity};
use ely_core::theme::ControlSize;
use gpui_forge_runtime::base::kit::*;
use gpui_forge_runtime::forge_spec::catalog::{self, Catalog};
use gpui_forge_runtime::forge_spec::{Node, Value};
use gpui_forge_runtime::render::{ForgeLibrary, ForgeView};
use std::sync::OnceLock;

/// `ely.*` adapter — register with `view.register_library(Ely)`.
pub struct Ely;

impl ForgeLibrary for Ely {
    fn alias(&self) -> &'static str {
        "ely"
    }

    fn catalog(&self) -> Option<&Catalog> {
        static CAT: OnceLock<Catalog> = OnceLock::new();
        Some(CAT.get_or_init(|| {
            catalog::from_yaml(include_str!("../../../catalogs/ely.catalog.yaml"))
                .expect("ely catalog parses")
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
            "Button" => {
                let mut b = Button::new(key.to_string(), prop_str(node, "label"));
                if let Some(v) = prop_str_opt(node, "variant") {
                    b = b.variant(match v.as_str() {
                        "Secondary" => ButtonVariant::Secondary,
                        "Outline" => ButtonVariant::Outline,
                        "Ghost" => ButtonVariant::Ghost,
                        "Subtle" => ButtonVariant::Subtle,
                        "Success" => ButtonVariant::Success,
                        "Danger" => ButtonVariant::Danger,
                        "Link" => ButtonVariant::Link,
                        _ => ButtonVariant::Primary,
                    });
                }
                if let Some(s) = prop_str_opt(node, "size") {
                    b = b.size(match s.as_str() {
                        "Sm" => ControlSize::Sm,
                        "Lg" => ControlSize::Lg,
                        _ => ControlSize::Md,
                    });
                }
                if prop_bool(node, "disabled") {
                    b = b.disabled(true);
                }
                Some(b.into_any_element())
            }
            "Badge" => Some(Badge::new(prop_str(node, "label")).into_any_element()),
            "Alert" => {
                let severity = match prop_str_opt(node, "severity").as_deref() {
                    Some("Success") => Severity::Success,
                    Some("Warning") => Severity::Warning,
                    Some("Danger") => Severity::Danger,
                    _ => Severity::Info,
                };
                let mut a = Alert::new(key.to_string(), severity, prop_str(node, "title"));
                if let Some(body) = prop_str_opt(node, "body") {
                    a = a.body(body);
                }
                Some(a.into_any_element())
            }
            "StatusMessage" => {
                let mut m = StatusMessage::new(key.to_string(), prop_str(node, "text"));
                if prop_str_opt(node, "icon").is_some() {
                    m = m.icon(IconName::Check);
                }
                Some(m.into_any_element())
            }
            "Divider" => Some(
                if prop_str_opt(node, "orientation").as_deref() == Some("vertical") {
                    Divider::vertical().into_any_element()
                } else {
                    Divider::horizontal().into_any_element()
                },
            ),
            "IconButton" => {
                let icon = match prop_str_opt(node, "icon").as_deref() {
                    Some("check") => IconName::Check,
                    Some("circle-check") => IconName::CircleCheck,
                    Some("file-plus") => IconName::FilePlus,
                    Some("folder-plus") => IconName::FolderPlus,
                    Some("menu") => IconName::Menu,
                    Some("list-checks") => IconName::ListChecks,
                    Some("square-check") => IconName::SquareCheck,
                    Some("check-check") => IconName::CheckCheck,
                    _ => IconName::Plus,
                };
                let mut b = IconButton::new(key.to_string(), icon);
                if let Some(t) = prop_str_opt(node, "tooltip") {
                    b = b.tooltip(t);
                }
                if prop_bool(node, "disabled") {
                    b = b.disabled(true);
                }
                Some(b.into_any_element())
            }
            _ => None,
        }
    }
}

fn prop<'a>(node: &'a Node, name: &str) -> Option<&'a Value> {
    node.props.iter().find(|(k, _)| k == name).map(|(_, v)| v)
}

fn prop_str_opt(node: &Node, name: &str) -> Option<String> {
    match prop(node, name)? {
        Value::String(s) | Value::Enum(s) => Some(s.clone()),
        _ => None,
    }
}

fn prop_str(node: &Node, name: &str) -> String {
    node.content
        .as_ref()
        .and_then(|v| match v {
            Value::String(s) | Value::Enum(s) => Some(s.clone()),
            _ => None,
        })
        .or_else(|| prop_str_opt(node, name))
        .unwrap_or_default()
}

fn prop_bool(node: &Node, name: &str) -> bool {
    matches!(prop(node, name), Some(Value::Bool(true)))
}
