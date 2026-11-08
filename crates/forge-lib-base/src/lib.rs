//! Adapter for Base GPUI (`use: base: base-gpui` → `base.*` elements) — #85.
//! Base GPUI is a headless Base UI port: components ship unstyled, so the
//! adapter supplies default chrome through `style_with_state` / `Styled`.
//! Render-only arms like the Ely adapter — event payloads are follow-up work.
//!
//! State lives inside each component (`window.use_keyed_state` entities keyed
//! by element id), so this adapter stays stateless: controlled props are fed
//! via `default_*` props and interactions update the component's own runtime.

use base_gpui::avatar::{AvatarFallback, AvatarImage, AvatarRoot};
use base_gpui::button::ButtonRoot;
use base_gpui::checkbox::{CheckboxIndicator, CheckboxRoot};
use base_gpui::input::Input;
use base_gpui::meter::{MeterIndicator, MeterLabel, MeterRoot, MeterTrack, MeterValue};
use base_gpui::progress::{
    ProgressIndicator, ProgressLabel, ProgressRoot, ProgressTrack, ProgressValue,
};
use base_gpui::separator::{Separator, SeparatorOrientation};
use base_gpui::slider::{
    SliderControl, SliderIndicator, SliderRoot, SliderThumb, SliderTrack, SliderValues,
};
use base_gpui::switch::{SwitchRoot, SwitchThumb};
use base_gpui::toggle::Toggle;
use gpui_forge_runtime::base::kit::*;
use gpui_forge_runtime::forge_spec::catalog::{self, Catalog};
use gpui_forge_runtime::forge_spec::{Node, Value};
use gpui_forge_runtime::render::{ForgeLibrary, ForgeView};
use std::cell::Cell;
use std::sync::OnceLock;

// Default chrome — flat Base-UI-flavoured palette (upstream ships unstyled).
const ACCENT: u32 = 0x2563eb;
const BORDER: u32 = 0xd4d4d8;
const TRACK: u32 = 0xe4e4e7;
const TEXT: u32 = 0x18181b;
const MUTED: u32 = 0xa1a1aa;
const WHITE: u32 = 0xffffff;

/// `base.*` adapter — register with `view.register_library(Base::new())`.
///
/// Calls `base_gpui::init` on first render: the component actions/key
/// bindings (Space toggles, slider arrows) are registered globally on the
/// `App`, once.
pub struct Base {
    inited: Cell<bool>,
}

impl Base {
    pub fn new() -> Self {
        Self {
            inited: Cell::new(false),
        }
    }
}

impl Default for Base {
    fn default() -> Self {
        Self::new()
    }
}

impl ForgeLibrary for Base {
    fn alias(&self) -> &'static str {
        "base"
    }

    fn catalog(&self) -> Option<&Catalog> {
        static CAT: OnceLock<Catalog> = OnceLock::new();
        Some(CAT.get_or_init(|| {
            catalog::from_yaml(include_str!("../../../catalogs/base.catalog.yaml"))
                .expect("base catalog parses")
        }))
    }

    fn render(
        &self,
        node: &Node,
        key: &str,
        view: &ForgeView,
        _window: &mut Window,
        cx: &mut Context<ForgeView>,
    ) -> Option<AnyElement> {
        if !self.inited.replace(true) {
            base_gpui::init(cx);
        }
        match node.element.name.as_str() {
            "Button" => {
                let b = ButtonRoot::new()
                    .id(key.to_string())
                    .disabled(prop_bool(node, "disabled"))
                    .style_with_state(|state, el| {
                        let el = el
                            .flex()
                            .items_center()
                            .justify_center()
                            .px_3()
                            .h_8()
                            .min_w(px(64.))
                            .rounded_md()
                            .text_sm();
                        if state.disabled {
                            el.bg(rgb(TRACK)).text_color(rgb(MUTED))
                        } else {
                            el.bg(rgb(ACCENT)).text_color(rgb(WHITE))
                        }
                    })
                    .child(SharedString::from(prop_str(node, "label")));
                Some(b.into_any_element())
            }
            "Checkbox" => {
                let indicator = CheckboxIndicator::new()
                    .keep_mounted(true)
                    .size(px(16.))
                    .rounded_sm()
                    .border_1()
                    .style_with_state(|state, el| {
                        if state.root.disabled {
                            el.bg(rgb(TRACK)).border_color(rgb(BORDER))
                        } else if state.root.checked || state.root.indeterminate {
                            el.bg(rgb(ACCENT)).border_color(rgb(ACCENT))
                        } else {
                            el.bg(rgb(WHITE)).border_color(rgb(BORDER))
                        }
                    });
                let root = CheckboxRoot::new()
                    .id(key.to_string())
                    .default_checked(prop_bool(node, "checked"))
                    .indeterminate(prop_bool(node, "indeterminate"))
                    .disabled(prop_bool(node, "disabled"))
                    .child(indicator);
                let mut row = div()
                    .id(key.to_string())
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(root);
                let label = prop_str(node, "label");
                if !label.is_empty() {
                    row = row.child(
                        div()
                            .text_sm()
                            .text_color(rgb(TEXT))
                            .child(SharedString::from(label)),
                    );
                }
                Some(row.into_any_element())
            }
            "Switch" => {
                let thumb = SwitchThumb::new()
                    .size(px(16.))
                    .rounded_full()
                    .bg(rgb(WHITE))
                    .style_with_state(
                        |state, el| {
                            if state.root.checked { el.ml_auto() } else { el }
                        },
                    );
                let root = SwitchRoot::new()
                    .id(key.to_string())
                    .default_checked(prop_bool(node, "checked"))
                    .disabled(prop_bool(node, "disabled"))
                    .w(px(36.))
                    .h(px(20.))
                    .p(px(2.))
                    .rounded_full()
                    .style_with_state(|state, el| {
                        let el = el.flex().flex_row().items_center();
                        if state.disabled {
                            el.bg(rgb(TRACK))
                        } else if state.checked {
                            el.bg(rgb(ACCENT))
                        } else {
                            el.bg(rgb(MUTED))
                        }
                    })
                    .child(thumb);
                Some(root.into_any_element())
            }
            "Toggle" => {
                let t = Toggle::<SharedString>::new()
                    .id(key.to_string())
                    .default_pressed(prop_bool(node, "pressed"))
                    .disabled(prop_bool(node, "disabled"))
                    .style_with_state(|state, el| {
                        let el = el
                            .flex()
                            .items_center()
                            .justify_center()
                            .px_3()
                            .h_8()
                            .rounded_md()
                            .text_sm()
                            .border_1();
                        if state.disabled {
                            el.bg(rgb(TRACK))
                                .border_color(rgb(BORDER))
                                .text_color(rgb(MUTED))
                        } else if state.pressed {
                            el.bg(rgb(BORDER))
                                .border_color(rgb(BORDER))
                                .text_color(rgb(TEXT))
                        } else {
                            el.bg(rgb(WHITE))
                                .border_color(rgb(BORDER))
                                .text_color(rgb(TEXT))
                        }
                    })
                    .child(SharedString::from(prop_str(node, "label")));
                Some(t.into_any_element())
            }
            "Slider" => {
                let track = SliderTrack::new()
                    .w_full()
                    .h(px(4.))
                    .rounded_full()
                    .bg(rgb(TRACK))
                    .child(
                        SliderIndicator::new()
                            .h_full()
                            .rounded_full()
                            .bg(rgb(ACCENT)),
                    );
                let thumb = SliderThumb::new()
                    .size(px(16.))
                    .rounded_full()
                    .bg(rgb(WHITE))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .shadow_sm();
                let control = SliderControl::new()
                    .w_full()
                    .h(px(16.))
                    .flex()
                    .items_center()
                    .child(track)
                    .child(thumb);
                let root = SliderRoot::new()
                    .id(key.to_string())
                    .default_value(SliderValues::Single(prop_num(node, "value").unwrap_or(50.)))
                    .min(prop_num(node, "min").unwrap_or(0.))
                    .max(prop_num(node, "max").unwrap_or(100.))
                    .step(prop_num(node, "step").unwrap_or(1.))
                    .disabled(prop_bool(node, "disabled"))
                    .w_full()
                    .child(control);
                Some(root.into_any_element())
            }
            "Progress" => {
                let track = ProgressTrack::new()
                    .w_full()
                    .h(px(6.))
                    .rounded_full()
                    .bg(rgb(TRACK))
                    .child(
                        ProgressIndicator::new()
                            .h_full()
                            .rounded_full()
                            .bg(rgb(ACCENT)),
                    );
                let mut root = ProgressRoot::new()
                    .id(key.to_string())
                    .value(Some(prop_num(node, "value").unwrap_or(50.)))
                    .max(prop_num(node, "max").unwrap_or(100.))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .w_full()
                    .child(track);
                if let Some(label) = prop_str_opt(node, "label") {
                    root = root.child(
                        ProgressLabel::new()
                            .text_sm()
                            .text_color(rgb(TEXT))
                            .child(SharedString::from(label)),
                    );
                }
                if !matches!(prop(node, "show_value"), Some(Value::Bool(false))) {
                    root = root.child(ProgressValue::new().text_sm().text_color(rgb(MUTED)));
                }
                Some(root.into_any_element())
            }
            "Meter" => {
                let track = MeterTrack::new()
                    .w_full()
                    .h(px(6.))
                    .rounded_full()
                    .bg(rgb(TRACK))
                    .child(
                        MeterIndicator::new()
                            .h_full()
                            .rounded_full()
                            .bg(rgb(ACCENT)),
                    );
                let mut root = MeterRoot::new()
                    .id(key.to_string())
                    .value(prop_num(node, "value").unwrap_or(50.))
                    .min(prop_num(node, "min").unwrap_or(0.))
                    .max(prop_num(node, "max").unwrap_or(100.))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .w_full()
                    .child(track);
                if let Some(label) = prop_str_opt(node, "label") {
                    root = root.child(
                        MeterLabel::new()
                            .text_sm()
                            .text_color(rgb(TEXT))
                            .text(label),
                    );
                }
                if !matches!(prop(node, "show_value"), Some(Value::Bool(false))) {
                    root = root.child(MeterValue::new().text_sm().text_color(rgb(MUTED)));
                }
                Some(root.into_any_element())
            }
            "Separator" => {
                let vertical = matches!(
                    prop_str_opt(node, "orientation").as_deref(),
                    Some("Vertical")
                );
                let s = Separator::new()
                    .id(key.to_string())
                    .orientation(if vertical {
                        SeparatorOrientation::Vertical
                    } else {
                        SeparatorOrientation::Horizontal
                    });
                let s = if vertical {
                    s.w(px(1.)).h_full()
                } else {
                    s.h(px(1.)).w_full()
                };
                Some(s.bg(rgb(BORDER)).into_any_element())
            }
            "Input" => {
                let i = Input::new()
                    .id(key.to_string())
                    .placeholder(prop_str(node, "placeholder"))
                    .default_value(prop_str(node, "value"))
                    .disabled(prop_bool(node, "disabled"))
                    .w_full()
                    .h_8()
                    .px_2()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(WHITE))
                    .text_sm()
                    .text_color(rgb(TEXT));
                Some(i.into_any_element())
            }
            "Avatar" => {
                let mut root = AvatarRoot::new()
                    .id(key.to_string())
                    .size(px(32.))
                    .rounded_full()
                    .overflow_hidden()
                    .bg(rgb(TRACK))
                    .flex()
                    .items_center()
                    .justify_center();
                if let Some(src) = prop_str_opt(node, "src") {
                    root = root.child(AvatarImage::new(view.resolve_path(&src).as_path()));
                }
                let fallback = prop_str(node, "fallback");
                root = root.child(
                    AvatarFallback::new()
                        .text_sm()
                        .text_color(rgb(TEXT))
                        .child(SharedString::from(fallback)),
                );
                Some(root.into_any_element())
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

fn prop_num(node: &Node, name: &str) -> Option<f64> {
    match prop(node, name)? {
        Value::Number { value, .. } => Some(*value),
        _ => None,
    }
}
