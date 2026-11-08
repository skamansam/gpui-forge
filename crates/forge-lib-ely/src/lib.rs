//! Adapter for Ely GPUI Components (`use: ely: ely-core` → `ely.*` elements).

use ely_core::buttons::{
    Button, ButtonVariant, CopyButton, IconButton, SegmentedControl, ToggleButton, ToggleItem,
};
use ely_core::data_display::{
    Avatar, Badge, KpiCard, Meter, QrCode, Statistic, Tag, TimelineItem, TrendIndicator, UsageBar,
};
use ely_core::feedback::{Alert, Banner, Callout, EmptyState, InlineMessage, StatusMessage};
use ely_core::forms::{Rating, Stepper, Switch};
use ely_core::layout::{AspectRatio, Container, ZStack};
use ely_core::motion::{Spinner, SpinnerStyle};
use ely_core::primitives::{Divider, Icon, IconName, Severity};
use ely_core::theme::{ContainerSize, ControlSize, IconSize};
use ely_core::typography::{Caption, Heading, Label, Overline, Paragraph, Subtitle, Title};
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
            "Spinner" => {
                let style = match prop_str_opt(node, "spinner").as_deref() {
                    Some("Dots") => SpinnerStyle::Dots,
                    Some("Wave") => SpinnerStyle::Wave,
                    Some("Bars") => SpinnerStyle::Bars,
                    Some("Pulse") => SpinnerStyle::Pulse,
                    _ => SpinnerStyle::Ring,
                };
                let size = match prop_str_opt(node, "size").as_deref() {
                    Some("Xs") => IconSize::Xs,
                    Some("Sm") => IconSize::Sm,
                    Some("Lg") => IconSize::Lg,
                    Some("Xl") => IconSize::Xl,
                    Some("Xxl") => IconSize::Xxl,
                    _ => IconSize::Md,
                };
                Some(
                    Spinner::new(key.to_string())
                        .style(style)
                        .size(size)
                        .into_any_element(),
                )
            }
            "Heading" => {
                let level = match prop(node, "level") {
                    Some(Value::Number { value, .. }) => *value as u8,
                    _ => 1,
                };
                Some(Heading::new(level, prop_str(node, "text")).into_any_element())
            }
            "Avatar" => {
                Some(Avatar::new(key.to_string(), prop_str(node, "name")).into_any_element())
            }
            "Paragraph" => Some(Paragraph::new(prop_str(node, "text")).into_any_element()),
            "Title" => Some(Title::new(prop_str(node, "text")).into_any_element()),
            "Divider" => Some(
                if prop_str_opt(node, "orientation").as_deref() == Some("vertical") {
                    Divider::vertical().into_any_element()
                } else {
                    Divider::horizontal().into_any_element()
                },
            ),
            "CopyButton" => {
                Some(CopyButton::new(key.to_string(), prop_str(node, "label")).into_any_element())
            }
            "TrendIndicator" => Some(
                TrendIndicator::new(prop_num(node, "change").unwrap_or(0.0)).into_any_element(),
            ),
            "Statistic" => Some(
                Statistic::new(
                    prop_str_opt(node, "label").unwrap_or_default(),
                    prop_str_opt(node, "value").unwrap_or_default(),
                    prop_num(node, "change").unwrap_or(0.0),
                )
                .into_any_element(),
            ),
            "KpiCard" => Some(
                KpiCard::new(Statistic::new(
                    prop_str_opt(node, "label").unwrap_or_default(),
                    prop_str_opt(node, "value").unwrap_or_default(),
                    prop_num(node, "change").unwrap_or(0.0),
                ))
                .into_any_element(),
            ),
            "Meter" => {
                // `value` is a share 0..=1; a number past 1 reads as a
                // percent (68 → 0.68). ely asserts on shares outside 0..=1,
                // so clamp rather than panic in the showcase.
                let v = prop_num(node, "value").unwrap_or(0.0) as f32;
                let share = if v > 1.0 { v / 100.0 } else { v }.clamp(0.0, 1.0);
                Some(
                    Meter::new(
                        key.to_string(),
                        prop_str_opt(node, "label").unwrap_or_default(),
                        share,
                    )
                    .into_any_element(),
                )
            }
            "UsageBar" => {
                Some(UsageBar::new(prop_num(node, "total").unwrap_or(100.0)).into_any_element())
            }
            "QrCode" => QrCode::new(prop_str_opt(node, "data").unwrap_or_default())
                .ok()
                .map(|q| q.into_any_element()),
            "TimelineItem" => Some(TimelineItem::new(prop_str(node, "text")).into_any_element()),
            "Banner" => Some(
                Banner::new(key.to_string(), severity_of(node), prop_str(node, "text"))
                    .into_any_element(),
            ),
            "InlineMessage" => Some(
                InlineMessage::new(severity_of(node), prop_str(node, "text")).into_any_element(),
            ),
            "Icon" => Some(Icon::new(icon_of(node)).into_any_element()),
            "Subtitle" => Some(Subtitle::new(prop_str(node, "text")).into_any_element()),
            "Caption" => Some(Caption::new(prop_str(node, "text")).into_any_element()),
            "Label" => Some(Label::new(prop_str(node, "text")).into_any_element()),
            "Overline" => Some(Overline::new(prop_str(node, "text")).into_any_element()),
            "Container" => {
                let size = match prop_str_opt(node, "size").as_deref() {
                    Some("Xs") => ContainerSize::Sm,
                    Some("Sm") => ContainerSize::Sm,
                    Some("Lg") => ContainerSize::Lg,
                    Some("Xl") => ContainerSize::Lg,
                    _ => ContainerSize::Md,
                };
                Some(Container::new(size).into_any_element())
            }
            "AspectRatio" => Some(
                AspectRatio::new(prop_num(node, "ratio").unwrap_or(1.0) as f32).into_any_element(),
            ),
            "ZStack" => Some(ZStack::new().into_any_element()),
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
            "Callout" => {
                let mut c = Callout::new(severity_of(node));
                if let Some(t) = prop_str_opt(node, "title") {
                    c = c.title(t);
                }
                Some(c.into_any_element())
            }
            "EmptyState" => Some(
                EmptyState::new(key.to_string(), icon_of(node), prop_str(node, "title"))
                    .into_any_element(),
            ),
            "Rating" => {
                let max = prop_num(node, "max").unwrap_or(5.0).max(1.0) as u8;
                let value = (prop_num(node, "value").unwrap_or(3.0) as u8).min(max);
                Some(
                    Rating::new(key.to_string(), value)
                        .max(max)
                        .into_any_element(),
                )
            }
            "SegmentedControl" => Some(
                SegmentedControl::new(key.to_string(), prop_str(node, "selected"))
                    .into_any_element(),
            ),
            "Stepper" => Some(
                Stepper::new(key.to_string(), prop_num(node, "value").unwrap_or(0.0))
                    .into_any_element(),
            ),
            "Switch" => {
                Some(Switch::new(key.to_string(), prop_bool(node, "checked")).into_any_element())
            }
            "Tag" => Some(Tag::new(key.to_string(), prop_str(node, "label")).into_any_element()),
            "ToggleButton" => Some(
                ToggleButton::new(
                    key.to_string(),
                    ToggleItem::new(prop_str(node, "label")).icon(icon_of(node)),
                    prop_bool(node, "checked"),
                )
                .into_any_element(),
            ),
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

fn prop_num(node: &Node, name: &str) -> Option<f64> {
    match prop(node, name)? {
        Value::Number { value, .. } => Some(*value),
        _ => None,
    }
}

fn severity_of(node: &Node) -> Severity {
    match prop_str_opt(node, "severity").as_deref() {
        Some("Success") => Severity::Success,
        Some("Warning") => Severity::Warning,
        Some("Danger") => Severity::Danger,
        _ => Severity::Info,
    }
}

fn icon_of(node: &Node) -> IconName {
    match prop_str_opt(node, "icon").as_deref() {
        Some("check") => IconName::Check,
        Some("circle-check") => IconName::CircleCheck,
        Some("file-plus") => IconName::FilePlus,
        Some("folder-plus") => IconName::FolderPlus,
        Some("menu") => IconName::Menu,
        Some("list-checks") => IconName::ListChecks,
        Some("square-check") => IconName::SquareCheck,
        Some("check-check") => IconName::CheckCheck,
        _ => IconName::Plus,
    }
}
