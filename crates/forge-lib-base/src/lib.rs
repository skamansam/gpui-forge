//! Adapter for Base GPUI (`use: base: base-gpui` → `base.*` elements) — #85.
//! Base GPUI is a headless Base UI port: components ship unstyled, so the
//! adapter supplies default chrome through `style_with_state` / `Styled`.
//! Render-only arms like the Ely adapter — event payloads are follow-up work.
//!
//! State lives inside each component (`window.use_keyed_state` entities keyed
//! by element id), so this adapter stays stateless: controlled props are fed
//! via `default_*` props and interactions update the component's own runtime.

use base_gpui::accordion::{
    AccordionHeader, AccordionItem, AccordionOrientation, AccordionPanel, AccordionRoot,
    AccordionTrigger,
};
use base_gpui::alert_dialog::{AlertDialogPopup, AlertDialogRoot, AlertDialogTrigger};
use base_gpui::autocomplete::{AutocompleteMode, AutocompleteRoot};
use base_gpui::avatar::{AvatarFallback, AvatarImage, AvatarRoot};
use base_gpui::button::ButtonRoot;
use base_gpui::checkbox::{CheckboxIndicator, CheckboxRoot};
use base_gpui::checkbox_group::CheckboxGroup;
use base_gpui::collapsible::{CollapsiblePanel, CollapsibleRoot, CollapsibleTrigger};
use base_gpui::combobox::{
    ComboboxArrow, ComboboxBackdrop, ComboboxClear, ComboboxEmpty, ComboboxGroup,
    ComboboxGroupLabel, ComboboxIcon, ComboboxInput, ComboboxInputGroup, ComboboxItem,
    ComboboxItemIndicator, ComboboxLabel, ComboboxList, ComboboxPopup, ComboboxPortal,
    ComboboxPositioner, ComboboxRoot, ComboboxSeparator, ComboboxStatus, ComboboxTrigger,
};
use base_gpui::context_menu::{ContextMenuRoot, ContextMenuTrigger};
use base_gpui::dialog::{
    DialogBackdrop, DialogClose, DialogDescription, DialogModalMode, DialogPopup, DialogPortal,
    DialogRoot, DialogTitle, DialogTrigger, DialogViewport,
};
use base_gpui::drawer::{
    DrawerBackdrop, DrawerContent, DrawerPopup, DrawerPortal, DrawerRoot, DrawerViewport,
};
use base_gpui::field::{FieldDescription, FieldError, FieldLabel, FieldRoot};
use base_gpui::fieldset::{FieldsetLegend, FieldsetRoot};
use base_gpui::form::{Form, FormValue};
use base_gpui::input::Input;
use base_gpui::menu::{
    MenuArrow, MenuBackdrop, MenuCheckboxItem, MenuCheckboxItemIndicator,
    MenuCheckboxItemStyleState, MenuGroup, MenuGroupChild, MenuGroupLabel, MenuItem,
    MenuItemStyleState, MenuLinkItem, MenuLinkItemStyleState, MenuPopup, MenuPopupChild,
    MenuPortal, MenuPositioner, MenuRadioGroup, MenuRadioGroupChild, MenuRadioItem,
    MenuRadioItemIndicator, MenuRadioItemStyleState, MenuRoot, MenuSeparator, MenuSubmenuRoot,
    MenuSubmenuTrigger, MenuTrigger,
};
use base_gpui::menubar::Menubar;
use base_gpui::meter::{MeterIndicator, MeterLabel, MeterRoot, MeterTrack, MeterValue};
use base_gpui::navigation_menu::{
    NavigationMenuArrow, NavigationMenuBackdrop, NavigationMenuContent, NavigationMenuItem,
    NavigationMenuList, NavigationMenuPopup, NavigationMenuPortal, NavigationMenuPositioner,
    NavigationMenuRoot, NavigationMenuTrigger, NavigationMenuViewport,
};
use base_gpui::number_field::{
    NumberFieldDecrement, NumberFieldGroup, NumberFieldIncrement, NumberFieldInput, NumberFieldRoot,
};
use base_gpui::otp_field::{OTPFieldInput, OTPFieldRoot};
use base_gpui::popover::{
    PopoverArrow, PopoverBackdrop, PopoverClose, PopoverDescription, PopoverPopup, PopoverPortal,
    PopoverPositioner, PopoverRoot, PopoverSide, PopoverTitle, PopoverTrigger,
};
use base_gpui::preview_card::{
    PreviewCardArrow, PreviewCardBackdrop, PreviewCardPopup, PreviewCardPortal,
    PreviewCardPositioner, PreviewCardRoot, PreviewCardSide, PreviewCardTrigger,
    PreviewCardViewport,
};
use base_gpui::progress::{
    ProgressIndicator, ProgressLabel, ProgressRoot, ProgressTrack, ProgressValue,
};
use base_gpui::radio_group::{RadioGroupIndicator, RadioGroupRadio, RadioGroupRoot};
use base_gpui::scroll_area::{
    ScrollAreaContent, ScrollAreaCorner, ScrollAreaOrientation, ScrollAreaRoot,
    ScrollAreaScrollbar, ScrollAreaThumb, ScrollAreaViewport,
};
use base_gpui::select::{
    SelectArrow, SelectBackdrop, SelectGroup, SelectGroupLabel, SelectIcon, SelectItem,
    SelectItemIndicator, SelectItemText, SelectLabel, SelectList, SelectPopup, SelectPortal,
    SelectPositioner, SelectRoot, SelectSeparator, SelectTrigger, SelectValue,
};
use base_gpui::separator::{Separator, SeparatorOrientation};
use base_gpui::slider::{
    SliderControl, SliderIndicator, SliderLabel, SliderRoot, SliderThumb, SliderTrack, SliderValue,
    SliderValues,
};
use base_gpui::switch::{SwitchRoot, SwitchThumb};
use base_gpui::tabs::{TabsIndicator, TabsList, TabsOrientation, TabsPanel, TabsRoot, TabsTab};
use base_gpui::toggle::Toggle;
use base_gpui::toggle_group::{ToggleGroup, ToggleGroupOrientation};
use base_gpui::toolbar::{
    ToolbarButton, ToolbarGroup, ToolbarInput, ToolbarLink, ToolbarOrientation, ToolbarRoot,
    ToolbarSeparator,
};
use base_gpui::tooltip::{
    TooltipPopup, TooltipPortal, TooltipPositioner, TooltipRoot, TooltipSide, TooltipTrigger,
};
use gpui_forge_runtime::base::kit::*;
use gpui_forge_runtime::forge_spec::catalog::{self, Catalog};
use gpui_forge_runtime::forge_spec::{Node, Path, Value};
use gpui_forge_runtime::render::{ForgeLibrary, ForgeView};
use serde_json::Value as Json;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
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
    views: RefCell<HashMap<String, AnyView>>,
}

/// Renders a doc node in a later pass — compound components hand these to
/// `AnyElement` child slots (panels, scroll content, field bodies) so the
/// render doesn't re-enter `ForgeView` while it is already borrowed.
struct BaseProxy {
    view: WeakEntity<ForgeView>,
    node: Node,
    key: String,
}

impl Render for BaseProxy {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.view
            .update(cx, |view, cx| {
                view.render_node(&self.node, &self.key, window, cx)
            })
            .unwrap_or_else(|_| div().into_any_element())
    }
}

impl Base {
    pub fn new() -> Self {
        Self {
            inited: Cell::new(false),
            views: RefCell::new(HashMap::new()),
        }
    }

    /// A proxy view rendering `child` in a later pass.
    fn child_view(&self, child: &Node, key: &str, cx: &mut Context<ForgeView>) -> AnyView {
        let mut views = self.views.borrow_mut();
        if let Some(v) = views.get(key) {
            return v.clone();
        }
        let node = child.clone();
        let k = key.to_owned();
        let weak = cx.entity().downgrade();
        let v: AnyView = cx
            .new(|_| BaseProxy {
                view: weak,
                node,
                key: k,
            })
            .into();
        views.insert(key.to_owned(), v.clone());
        v
    }

    /// Render every child of `node` as proxies, keyed `{key}/{i}`.
    fn child_views(&self, node: &Node, key: &str, cx: &mut Context<ForgeView>) -> Vec<AnyView> {
        node.children
            .iter()
            .enumerate()
            .map(|(i, c)| self.child_view(c, &format!("{key}/{i}"), cx))
            .collect()
    }

    /// Every child of `node` as a proxy-backed `AnyElement`.
    fn child_elements(
        &self,
        node: &Node,
        key: &str,
        cx: &mut Context<ForgeView>,
    ) -> Vec<AnyElement> {
        self.child_views(node, key, cx)
            .into_iter()
            .map(IntoElement::into_any_element)
            .collect()
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
                let mut root = CheckboxRoot::new()
                    .id(key.to_string())
                    .default_checked(prop_bool(node, "checked"))
                    .indeterminate(prop_bool(node, "indeterminate"))
                    .disabled(prop_bool(node, "disabled"));
                if let Some(v) = prop_str_opt(node, "value") {
                    root = root.value(SharedString::from(v));
                }
                if let Some(cb) = value_change(view, node, "checked", "change", cx) {
                    root = root.on_checked_change(move |checked, _d, w, cx| {
                        cb(Json::Bool(checked), checked.to_string(), w, cx);
                    });
                }
                let root = root.child(indicator);
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
                let mut root = SwitchRoot::new()
                    .id(key.to_string())
                    .default_checked(prop_bool(node, "checked"))
                    .disabled(prop_bool(node, "disabled"));
                if let Some(cb) = value_change(view, node, "checked", "change", cx) {
                    root = root.on_checked_change(move |checked, _d, w, cx| {
                        cb(Json::Bool(checked), checked.to_string(), w, cx);
                    });
                }
                let root = root
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
                let mut t = Toggle::<SharedString>::new()
                    .id(key.to_string())
                    .default_pressed(prop_bool(node, "pressed"))
                    .disabled(prop_bool(node, "disabled"));
                if let Some(v) = prop_str_opt(node, "value") {
                    t = t.value(SharedString::from(v));
                }
                if let Some(cb) = value_change(view, node, "pressed", "change", cx) {
                    t = t.on_pressed_change(move |pressed, _d, w, cx| {
                        cb(Json::Bool(pressed), pressed.to_string(), w, cx);
                    });
                }
                let t = t
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
                let mut root = SliderRoot::new()
                    .id(key.to_string())
                    .default_value(SliderValues::Single(prop_num(node, "value").unwrap_or(50.)))
                    .min(prop_num(node, "min").unwrap_or(0.))
                    .max(prop_num(node, "max").unwrap_or(100.))
                    .step(prop_num(node, "step").unwrap_or(1.))
                    .disabled(prop_bool(node, "disabled"))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .w_full();
                if let Some(cb) = value_change(view, node, "value", "change", cx) {
                    root = root.on_value_change(move |values, _d, w, cx| {
                        let (json, text) = match values {
                            SliderValues::Single(v) => (serde_json::json!(v), v.to_string()),
                            SliderValues::Range(vs) => {
                                let json = serde_json::json!(vs);
                                (json.clone(), json.to_string())
                            }
                        };
                        cb(json, text, w, cx);
                    });
                }
                if let Some(label) = prop_str_opt(node, "label") {
                    root = root.child(
                        SliderLabel::new()
                            .text_sm()
                            .text_color(rgb(TEXT))
                            .child(SharedString::from(label)),
                    );
                }
                root = root.child(control);
                if prop_bool(node, "show_value") {
                    root = root.child(SliderValue::new().text_sm().text_color(rgb(MUTED)));
                }
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
            "Accordion" => {
                let mut root = AccordionRoot::<SharedString>::new()
                    .id(key.to_string())
                    .multiple(prop_bool(node, "multiple"))
                    .keep_mounted(prop_bool(node, "keep_mounted"))
                    .disabled(prop_bool(node, "disabled"))
                    .w_full();
                if let Some(cb) = value_change(view, node, "value", "change", cx) {
                    root = root.on_value_change(move |values, _d, w, cx| {
                        let strings: Vec<String> = values.iter().map(|v| v.to_string()).collect();
                        cb(
                            Json::from(strings.clone()),
                            Json::from(strings).to_string(),
                            w,
                            cx,
                        );
                    });
                }
                if matches!(
                    prop_str_opt(node, "orientation").as_deref(),
                    Some("Horizontal")
                ) {
                    root = root.orientation(AccordionOrientation::Horizontal);
                }
                let mut default_value: Vec<SharedString> = Vec::new();
                for (i, item) in node.children.iter().enumerate() {
                    if item.element.name != "AccordionItem" {
                        continue;
                    }
                    let label = prop_str(item, "label");
                    let value = SharedString::from(
                        prop_str_opt(item, "value").unwrap_or_else(|| format!("{key}/item-{i}")),
                    );
                    if prop_bool(item, "open") {
                        default_value.push(value.clone());
                    }
                    let trigger = AccordionTrigger::new()
                        .id(format!("{key}/trigger-{i}"))
                        .w_full()
                        .flex()
                        .items_center()
                        .justify_between()
                        .px_3()
                        .h_10()
                        .text_sm()
                        .text_color(rgb(TEXT))
                        .style_with_state(|state, el| {
                            if state.item.disabled {
                                el.text_color(rgb(MUTED))
                            } else {
                                el
                            }
                        })
                        .child(SharedString::from(label))
                        .child(div().text_color(rgb(MUTED)).child("⌄"));
                    let mut panel = AccordionPanel::new()
                        .id(format!("{key}/panel-{i}"))
                        .px_3()
                        .py_2()
                        .border_b_1()
                        .border_color(rgb(BORDER));
                    panel.extend(self.child_elements(item, &format!("{key}/item-{i}"), cx));
                    root = root.child(
                        AccordionItem::new(value)
                            .disabled(prop_bool(item, "disabled"))
                            .child(AccordionHeader::new().child(trigger))
                            .child(panel),
                    );
                }
                Some(root.default_value(default_value).into_any_element())
            }
            "Collapsible" => {
                let trigger = CollapsibleTrigger::new()
                    .id(format!("{key}/trigger"))
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .h_10()
                    .text_sm()
                    .text_color(rgb(TEXT))
                    .child(SharedString::from(prop_str(node, "label")))
                    .child(div().text_color(rgb(MUTED)).child("⌄"));
                let mut panel = CollapsiblePanel::new()
                    .keep_mounted(prop_bool(node, "keep_mounted"))
                    .px_3()
                    .py_2()
                    .border_t_1()
                    .border_color(rgb(BORDER));
                panel.extend(self.child_elements(node, key, cx));
                let mut root = CollapsibleRoot::new()
                    .id(key.to_string())
                    .default_open(prop_bool(node, "open"))
                    .disabled(prop_bool(node, "disabled"))
                    .w_full()
                    .border_1()
                    .border_color(rgb(BORDER))
                    .rounded_md()
                    .child(trigger)
                    .child(panel);
                if let Some(cb) = value_change(view, node, "open", "open", cx) {
                    root = root.on_open_change(move |open, _d, w, cx| {
                        cb(Json::Bool(open), open.to_string(), w, cx);
                    });
                }
                Some(root.into_any_element())
            }
            "Tabs" => {
                let mut list = TabsList::<SharedString>::new()
                    .relative()
                    .w_full()
                    .flex()
                    .flex_row()
                    .gap_1()
                    .border_b_1()
                    .border_color(rgb(BORDER));
                let mut panels = Vec::new();
                for (i, tab_node) in node.children.iter().enumerate() {
                    if tab_node.element.name != "Tab" {
                        continue;
                    }
                    let label = prop_str(tab_node, "label");
                    let value = SharedString::from(
                        prop_str_opt(tab_node, "value").unwrap_or_else(|| format!("{key}/tab-{i}")),
                    );
                    let tab = TabsTab::<SharedString>::new()
                        .id(format!("{key}/tab-{i}"))
                        .value(value.clone())
                        .disabled(prop_bool(tab_node, "disabled"))
                        .px_3()
                        .h_9()
                        .text_sm()
                        .style_with_state(|state, el| {
                            if state.disabled {
                                el.text_color(rgb(MUTED))
                            } else if state.active {
                                el.text_color(rgb(ACCENT))
                            } else {
                                el.text_color(rgb(TEXT))
                            }
                        })
                        .child(SharedString::from(label));
                    list = list.child(tab);
                    let mut panel = TabsPanel::<SharedString>::new()
                        .id(format!("{key}/panel-{i}"))
                        .value(value)
                        .keep_mounted(prop_bool(tab_node, "keep_mounted"))
                        .pt_3();
                    panel.extend(self.child_elements(
                        tab_node,
                        &format!("{key}/tab-panel-{i}"),
                        cx,
                    ));
                    panels.push(panel);
                }
                list = list.child(
                    TabsIndicator::<SharedString>::new()
                        .absolute()
                        .bg(rgb(ACCENT))
                        .style_with_state(|state, el| {
                            match (state.active_tab_position, state.active_tab_size) {
                                (Some(position), Some(size)) => match state.orientation {
                                    TabsOrientation::Horizontal => el
                                        .left(position.left)
                                        .top(position.bottom)
                                        .w(size.width)
                                        .h(px(2.)),
                                    TabsOrientation::Vertical => el
                                        .left(position.right)
                                        .top(position.top)
                                        .w(px(2.))
                                        .h(size.height),
                                },
                                _ => el,
                            }
                        }),
                );
                let mut root = TabsRoot::<SharedString>::new()
                    .id(key.to_string())
                    .w_full()
                    .child(list)
                    .children(panels);
                if let Some(v) = prop_str_opt(node, "value") {
                    root = root.default_value(Some(SharedString::from(v)));
                }
                if let Some(cb) = value_change(view, node, "value", "change", cx) {
                    root = root.on_value_change(move |value, w, cx| {
                        let (json, text) = opt_value(value);
                        cb(json, text, w, cx);
                    });
                }
                Some(root.into_any_element())
            }
            "RadioGroup" => {
                let horizontal = matches!(
                    prop_str_opt(node, "orientation").as_deref(),
                    Some("Horizontal")
                );
                let mut root = RadioGroupRoot::<SharedString>::new()
                    .id(key.to_string())
                    .disabled(prop_bool(node, "disabled"))
                    .flex()
                    .gap_2();
                root = if horizontal {
                    root.flex_row().gap_4()
                } else {
                    root.flex_col()
                };
                if let Some(v) = prop_str_opt(node, "value") {
                    root = root.default_value(Some(SharedString::from(v)));
                }
                if let Some(cb) = value_change(view, node, "value", "change", cx) {
                    root = root.on_value_change(move |value, _d, w, cx| {
                        let (json, text) = opt_value(value);
                        cb(json, text, w, cx);
                    });
                }
                for (i, radio_node) in node.children.iter().enumerate() {
                    if radio_node.element.name != "Radio" {
                        continue;
                    }
                    let label = prop_str(radio_node, "label");
                    let value = prop_str_opt(radio_node, "value").unwrap_or_else(|| label.clone());
                    let dot = RadioGroupIndicator::new()
                        .keep_mounted(true)
                        .size(px(14.))
                        .rounded_full()
                        .border_1()
                        .style_with_state(|state, el| {
                            if state.radio.disabled {
                                el.bg(rgb(TRACK)).border_color(rgb(BORDER))
                            } else if state.radio.checked {
                                el.bg(rgb(ACCENT)).border_color(rgb(ACCENT))
                            } else {
                                el.bg(rgb(WHITE)).border_color(rgb(BORDER))
                            }
                        });
                    let label_part = RadioGroupIndicator::new()
                        .keep_mounted(true)
                        .text_sm()
                        .text_color(rgb(TEXT))
                        .child(SharedString::from(label));
                    root = root.child(
                        RadioGroupRadio::new()
                            .id(format!("{key}/radio-{i}"))
                            .value(SharedString::from(value))
                            .disabled(prop_bool(radio_node, "disabled"))
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(dot)
                            .child(label_part),
                    );
                }
                Some(root.into_any_element())
            }
            "CheckboxGroup" => {
                let mut all_values: Vec<SharedString> = Vec::new();
                let mut default_value: Vec<SharedString> = Vec::new();
                for c in node.children.iter() {
                    if c.element.name != "Checkbox" {
                        continue;
                    }
                    let value = prop_str_opt(c, "value").unwrap_or_else(|| prop_str(c, "label"));
                    if prop_bool(c, "checked") {
                        default_value.push(SharedString::from(value.clone()));
                    }
                    all_values.push(SharedString::from(value));
                }
                let mut group = CheckboxGroup::new()
                    .id(key.to_string())
                    .disabled(prop_bool(node, "disabled"))
                    .all_values(all_values)
                    .default_value(default_value)
                    .flex()
                    .flex_col()
                    .gap_2();
                if let Some(cb) = value_change(view, node, "value", "change", cx) {
                    group = group.on_value_change(move |values, _d, w, cx| {
                        let strings: Vec<String> = values.iter().map(|v| v.to_string()).collect();
                        cb(
                            Json::from(strings.clone()),
                            Json::from(strings).to_string(),
                            w,
                            cx,
                        );
                    });
                }
                for v in self.child_views(node, key, cx) {
                    group = group.child(v);
                }
                Some(group.into_any_element())
            }
            "ToggleGroup" => {
                let vertical = matches!(
                    prop_str_opt(node, "orientation").as_deref(),
                    Some("Vertical")
                );
                let mut group = ToggleGroup::<SharedString>::new()
                    .id(key.to_string())
                    .multiple(prop_bool(node, "multiple"))
                    .disabled(prop_bool(node, "disabled"))
                    .orientation(if vertical {
                        ToggleGroupOrientation::Vertical
                    } else {
                        ToggleGroupOrientation::Horizontal
                    })
                    .flex()
                    .gap_1();
                group = if vertical {
                    group.flex_col().items_start()
                } else {
                    group.flex_row()
                };
                if let Some(Value::Bool(b)) = prop(node, "loop_focus") {
                    group = group.loop_focus(*b);
                }
                let mut default_value: Vec<SharedString> = Vec::new();
                for (i, c) in node.children.iter().enumerate() {
                    if c.element.name != "Toggle" {
                        continue;
                    }
                    let label = prop_str(c, "label");
                    let value = prop_str_opt(c, "value").unwrap_or_else(|| label.clone());
                    if prop_bool(c, "pressed") {
                        default_value.push(SharedString::from(value.clone()));
                    }
                    let t = Toggle::<SharedString>::new()
                        .id(format!("{key}/toggle-{i}"))
                        .value(SharedString::from(value))
                        .disabled(prop_bool(c, "disabled"))
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
                        .child(SharedString::from(label));
                    group = group.child(t);
                }
                if let Some(cb) = value_change(view, node, "value", "change", cx) {
                    group = group.on_value_change(move |values, _d, w, cx| {
                        let strings: Vec<String> = values.iter().map(|v| v.to_string()).collect();
                        cb(
                            Json::from(strings.clone()),
                            Json::from(strings).to_string(),
                            w,
                            cx,
                        );
                    });
                }
                Some(group.default_value(default_value).into_any_element())
            }
            "Fieldset" => {
                let mut root = FieldsetRoot::new()
                    .id(key.to_string())
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_3()
                    .border_1()
                    .border_color(rgb(BORDER))
                    .rounded_md();
                if let Some(legend) = prop_str_opt(node, "legend") {
                    root = root.child(
                        FieldsetLegend::new()
                            .text_sm()
                            .text_color(rgb(TEXT))
                            .child(SharedString::from(legend)),
                    );
                }
                for v in self.child_views(node, key, cx) {
                    root = root.child_any(v);
                }
                Some(root.into_any_element())
            }
            "Field" => {
                let mut root = FieldRoot::new()
                    .id(key.to_string())
                    .disabled(prop_bool(node, "disabled"))
                    .flex()
                    .flex_col()
                    .gap_1();
                if prop_bool(node, "invalid") || prop_str_opt(node, "error").is_some() {
                    root = root.invalid(true);
                }
                if let Some(label) = prop_str_opt(node, "label") {
                    root = root.child(
                        FieldLabel::new()
                            .text_sm()
                            .text_color(rgb(TEXT))
                            .child(SharedString::from(label)),
                    );
                }
                for v in self.child_views(node, key, cx) {
                    root = root.child_any(v);
                }
                if let Some(desc) = prop_str_opt(node, "description") {
                    root = root.child(
                        FieldDescription::new()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(SharedString::from(desc)),
                    );
                }
                if let Some(error) = prop_str_opt(node, "error") {
                    root = root.child(
                        FieldError::new()
                            .text_xs()
                            .text_color(rgb(0xdc2626))
                            .child(SharedString::from(error)),
                    );
                }
                Some(root.into_any_element())
            }
            "Form" => {
                let mut form = Form::new().id(key.to_string()).flex().flex_col().gap_3();
                if let Some(cb) = event_cb(view, node, "submit", cx) {
                    form = form.on_form_submit(move |values, _d, w, cx| {
                        let map: serde_json::Map<String, Json> = values
                            .iter()
                            .map(|(k, v)| (k.to_string(), form_value_json(v)))
                            .collect();
                        cb(Json::Object(map).to_string(), w, cx);
                    });
                }
                for v in self.child_views(node, key, cx) {
                    form = form.child(v);
                }
                Some(form.into_any_element())
            }
            "NumberField" => {
                let group = NumberFieldGroup::new()
                    .flex()
                    .flex_row()
                    .items_center()
                    .border_1()
                    .border_color(rgb(BORDER))
                    .rounded_md()
                    .overflow_hidden()
                    .child(
                        NumberFieldDecrement::new()
                            .w(px(28.))
                            .h_8()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_sm()
                            .text_color(rgb(TEXT))
                            .child(SharedString::from("−")),
                    )
                    .child(
                        NumberFieldInput::new()
                            .w(px(64.))
                            .h_8()
                            .text_sm()
                            .text_color(rgb(TEXT)),
                    )
                    .child(
                        NumberFieldIncrement::new()
                            .w(px(28.))
                            .h_8()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_sm()
                            .text_color(rgb(TEXT))
                            .child(SharedString::from("+")),
                    );
                let mut root = NumberFieldRoot::new()
                    .id(key.to_string())
                    .disabled(prop_bool(node, "disabled"))
                    .read_only(prop_bool(node, "read_only"))
                    .child(group);
                if let Some(v) = prop_num(node, "value") {
                    root = root.default_value(Some(v));
                }
                if let Some(v) = prop_num(node, "min") {
                    root = root.min(v);
                }
                if let Some(v) = prop_num(node, "max") {
                    root = root.max(v);
                }
                if let Some(v) = prop_num(node, "step") {
                    root = root.step(v);
                }
                if let Some(cb) = value_change(view, node, "value", "change", cx) {
                    root = root.on_value_change(move |value, _d, w, cx| {
                        let (json, text) = match value {
                            Some(v) => (serde_json::json!(v), v.to_string()),
                            None => (Json::Null, "null".to_string()),
                        };
                        cb(json, text, w, cx);
                    });
                }
                Some(root.into_any_element())
            }
            "OTPField" => {
                let length = prop_num(node, "length")
                    .map(|n| n.max(1.0) as usize)
                    .unwrap_or(6);
                let mut root = OTPFieldRoot::new()
                    .id(key.to_string())
                    .length(length)
                    .disabled(prop_bool(node, "disabled"))
                    .read_only(prop_bool(node, "read_only"))
                    .mask(prop_bool(node, "mask"))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2();
                if let Some(v) = prop_str_opt(node, "value") {
                    root = root.default_value(SharedString::from(v));
                }
                if let Some(cb) = value_change(view, node, "value", "change", cx) {
                    root = root.on_value_change(move |value, _d, w, cx| {
                        cb(Json::String(value.to_string()), value.to_string(), w, cx);
                    });
                }
                if let Some(cb) = event_cb(view, node, "complete", cx) {
                    root = root.on_value_complete(move |value, _d, w, cx| {
                        cb(value.to_string(), w, cx);
                    });
                }
                let separator = prop_bool(node, "separator");
                for i in 0..length {
                    if separator && length > 2 && i == length / 2 {
                        root = root.child(Separator::new().w(px(8.)).h(px(2.)).bg(rgb(BORDER)));
                    }
                    root = root.child(
                        OTPFieldInput::new()
                            .id(format!("{key}/slot-{i}"))
                            .size(px(34.))
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(BORDER))
                            .text_sm()
                            .text_color(rgb(TEXT))
                            .style_with_state(|state, el| {
                                if state.active {
                                    el.border_color(rgb(ACCENT))
                                } else {
                                    el
                                }
                            }),
                    );
                }
                Some(root.into_any_element())
            }
            "ScrollArea" => {
                let content = ScrollAreaContent::new()
                    .id(format!("{key}/content"))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_3()
                    .children(self.child_elements(node, key, cx));
                let mut viewport = ScrollAreaViewport::new()
                    .id(format!("{key}/viewport"))
                    .child(content);
                if let Some(label) = prop_str_opt(node, "aria_label") {
                    viewport = viewport.aria_label(label);
                }
                let vbar = ScrollAreaScrollbar::new()
                    .orientation(ScrollAreaOrientation::Vertical)
                    .keep_mounted(true)
                    .w(px(8.))
                    .child(ScrollAreaThumb::new());
                let mut root = ScrollAreaRoot::new()
                    .id(key.to_string())
                    .w(px(prop_num(node, "width").unwrap_or(280.) as f32))
                    .h(px(prop_num(node, "height").unwrap_or(200.) as f32))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .rounded_md()
                    .child(viewport)
                    .child(vbar);
                if matches!(
                    prop_str_opt(node, "orientation").as_deref(),
                    Some("Both") | Some("Horizontal")
                ) {
                    root = root.child(
                        ScrollAreaScrollbar::new()
                            .orientation(ScrollAreaOrientation::Horizontal)
                            .keep_mounted(true)
                            .h(px(8.))
                            .child(ScrollAreaThumb::new()),
                    );
                }
                if matches!(prop_str_opt(node, "orientation").as_deref(), Some("Both")) {
                    root = root.child(ScrollAreaCorner::new().w(px(8.)).h(px(8.)).bg(rgb(TRACK)));
                }
                Some(root.into_any_element())
            }
            "Toolbar" => {
                let vertical = matches!(
                    prop_str_opt(node, "orientation").as_deref(),
                    Some("Vertical")
                );
                let mut root = ToolbarRoot::new()
                    .id(key.to_string())
                    .orientation(if vertical {
                        ToolbarOrientation::Vertical
                    } else {
                        ToolbarOrientation::Horizontal
                    })
                    .disabled(prop_bool(node, "disabled"))
                    .flex()
                    .items_center()
                    .gap_1()
                    .p_1()
                    .border_1()
                    .border_color(rgb(BORDER))
                    .rounded_md();
                root = if vertical {
                    root.flex_col()
                } else {
                    root.flex_row()
                };
                for (i, c) in node.children.iter().enumerate() {
                    match c.element.name.as_str() {
                        "ToolbarButton" => {
                            root = root.child(toolbar_button(view, c, key, i, cx));
                        }
                        "ToolbarLink" => {
                            root = root.child(toolbar_link(view, c, key, i, cx));
                        }
                        "ToolbarInput" => {
                            root = root.child(toolbar_input(c, key, i));
                        }
                        "ToolbarSeparator" => {
                            root = root.child(ToolbarSeparator::new());
                        }
                        "ToolbarGroup" => {
                            root = root.child(toolbar_group(view, c, key, i, cx));
                        }
                        _ => {}
                    }
                }
                Some(root.into_any_element())
            }
            "Tooltip" => {
                let mut trigger = TooltipTrigger::<()>::new().id(format!("{key}/trigger"));
                trigger.extend(self.child_elements(node, key, cx));
                let popup = TooltipPopup::<()>::new()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .text_xs()
                    .text_color(rgb(WHITE))
                    .bg(rgb(0x27272a))
                    .shadow_md()
                    .child_any(SharedString::from(prop_str(node, "label")));
                let mut root = TooltipRoot::<()>::new()
                    .id(key.to_string())
                    .child(trigger)
                    .child(
                        TooltipPortal::<()>::new().child(
                            TooltipPositioner::<()>::new()
                                .side(tooltip_side(node))
                                .side_offset(px(4.))
                                .child(popup),
                        ),
                    );
                if let Some(cb) = event_cb(view, node, "open", cx) {
                    root = root.on_open_change(move |open, _d, w, cx| {
                        cb(open.to_string(), w, cx);
                    });
                }
                Some(root.into_any_element())
            }
            "Dialog" => {
                let mut popup = DialogPopup::<()>::new()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_4()
                    .min_w(px(280.))
                    .bg(rgb(WHITE))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .rounded_lg()
                    .shadow_lg();
                if let Some(title) = prop_str_opt(node, "title") {
                    popup = popup.child(
                        DialogTitle::<()>::new()
                            .text_base()
                            .text_color(rgb(TEXT))
                            .child(SharedString::from(title)),
                    );
                }
                if let Some(desc) = prop_str_opt(node, "description") {
                    popup = popup.child(
                        DialogDescription::<()>::new()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .child(SharedString::from(desc)),
                    );
                }
                for v in self.child_views(node, key, cx) {
                    popup = popup.child_any(v);
                }
                if prop_bool(node, "close") {
                    popup = popup.child(
                        DialogClose::<()>::new()
                            .px_3()
                            .h_8()
                            .rounded_md()
                            .bg(rgb(TRACK))
                            .text_sm()
                            .text_color(rgb(TEXT))
                            .child("Close"),
                    );
                }
                let mut root = DialogRoot::<()>::new()
                    .id(key.to_string())
                    .default_open(prop_bool(node, "open"))
                    .modal_mode(
                        if matches!(prop_str_opt(node, "modal").as_deref(), Some("false")) {
                            DialogModalMode::NonModal
                        } else {
                            DialogModalMode::Modal
                        },
                    )
                    .child(
                        DialogTrigger::<()>::new()
                            .id(format!("{key}/trigger"))
                            .flex()
                            .items_center()
                            .justify_center()
                            .px_3()
                            .h_8()
                            .min_w(px(64.))
                            .rounded_md()
                            .text_sm()
                            .bg(rgb(ACCENT))
                            .text_color(rgb(WHITE))
                            .child(SharedString::from(prop_str(node, "label"))),
                    )
                    .child(
                        DialogPortal::<()>::new()
                            .child(
                                DialogBackdrop::<()>::new()
                                    .absolute()
                                    .top_0()
                                    .left_0()
                                    .w_full()
                                    .h_full()
                                    .bg(rgba(0x18181b66)),
                            )
                            .child(
                                DialogViewport::<()>::new()
                                    .absolute()
                                    .top_0()
                                    .left_0()
                                    .w_full()
                                    .h_full()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(popup),
                            ),
                    );
                if let Some(cb) = value_change(view, node, "open", "open", cx) {
                    root = root.on_open_change(move |open, _d, w, cx| {
                        cb(Json::Bool(open), open.to_string(), w, cx);
                    });
                }
                Some(root.into_any_element())
            }
            "AlertDialog" => {
                let mut popup = AlertDialogPopup::<()>::new()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_4()
                    .min_w(px(280.))
                    .bg(rgb(WHITE))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .rounded_lg()
                    .shadow_lg();
                if let Some(title) = prop_str_opt(node, "title") {
                    popup = popup.child(
                        DialogTitle::<()>::new()
                            .text_base()
                            .text_color(rgb(TEXT))
                            .child(SharedString::from(title)),
                    );
                }
                if let Some(desc) = prop_str_opt(node, "description") {
                    popup = popup.child(
                        DialogDescription::<()>::new()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .child(SharedString::from(desc)),
                    );
                }
                for v in self.child_views(node, key, cx) {
                    popup = popup.child_any(v);
                }
                if prop_bool(node, "close") {
                    popup = popup.child(
                        DialogClose::<()>::new()
                            .px_3()
                            .h_8()
                            .rounded_md()
                            .bg(rgb(TRACK))
                            .text_sm()
                            .text_color(rgb(TEXT))
                            .child("Close"),
                    );
                }
                let mut root = AlertDialogRoot::<()>::new()
                    .id(key.to_string())
                    .default_open(prop_bool(node, "open"))
                    .child(
                        AlertDialogTrigger::<()>::new()
                            .id(format!("{key}/trigger"))
                            .flex()
                            .items_center()
                            .justify_center()
                            .px_3()
                            .h_8()
                            .min_w(px(64.))
                            .rounded_md()
                            .text_sm()
                            .bg(rgb(ACCENT))
                            .text_color(rgb(WHITE))
                            .child(SharedString::from(prop_str(node, "label"))),
                    )
                    .child(
                        DialogPortal::<()>::new()
                            .child(
                                DialogBackdrop::<()>::new()
                                    .absolute()
                                    .top_0()
                                    .left_0()
                                    .w_full()
                                    .h_full()
                                    .bg(rgba(0x18181b66)),
                            )
                            .child(
                                DialogViewport::<()>::new()
                                    .absolute()
                                    .top_0()
                                    .left_0()
                                    .w_full()
                                    .h_full()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(popup),
                            ),
                    );
                if let Some(cb) = value_change(view, node, "open", "open", cx) {
                    root = root.on_open_change(move |open, _d, w, cx| {
                        cb(Json::Bool(open), open.to_string(), w, cx);
                    });
                }
                Some(root.into_any_element())
            }
            "Popover" => {
                let mut popup = PopoverPopup::<()>::new()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_3()
                    .min_w(px(200.))
                    .bg(rgb(WHITE))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .rounded_md()
                    .shadow_lg();
                if let Some(title) = prop_str_opt(node, "title") {
                    popup = popup.child(
                        PopoverTitle::<()>::new()
                            .text_sm()
                            .text_color(rgb(TEXT))
                            .child(SharedString::from(title)),
                    );
                }
                if let Some(desc) = prop_str_opt(node, "description") {
                    popup = popup.child(
                        PopoverDescription::<()>::new()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(SharedString::from(desc)),
                    );
                }
                for v in self.child_views(node, key, cx) {
                    popup = popup.child_any(v);
                }
                if prop_bool(node, "close") {
                    popup = popup.child(
                        PopoverClose::<()>::new()
                            .px_3()
                            .h_8()
                            .rounded_md()
                            .bg(rgb(TRACK))
                            .text_sm()
                            .text_color(rgb(TEXT))
                            .child("Close"),
                    );
                }
                if prop_bool(node, "arrow") {
                    popup = popup.child(overlay_arrow(PopoverArrow::<()>::new()));
                }
                let mut portal = PopoverPortal::<()>::new();
                if prop_bool(node, "backdrop") {
                    portal = portal.child(overlay_backdrop(PopoverBackdrop::<()>::new()));
                }
                portal = portal.child(
                    PopoverPositioner::<()>::new()
                        .side(popover_side(node))
                        .side_offset(px(4.))
                        .child(popup),
                );
                let mut root = PopoverRoot::<()>::new()
                    .id(key.to_string())
                    .default_open(prop_bool(node, "open"))
                    .child(
                        PopoverTrigger::<()>::new()
                            .id(format!("{key}/trigger"))
                            .flex()
                            .items_center()
                            .justify_center()
                            .px_3()
                            .h_8()
                            .min_w(px(64.))
                            .rounded_md()
                            .text_sm()
                            .bg(rgb(ACCENT))
                            .text_color(rgb(WHITE))
                            .child(SharedString::from(prop_str(node, "label"))),
                    )
                    .child(portal);
                if let Some(cb) = value_change(view, node, "open", "open", cx) {
                    root = root.on_open_change(move |open, _d, w, cx| {
                        cb(Json::Bool(open), open.to_string(), w, cx);
                    });
                }
                Some(root.into_any_element())
            }
            "PreviewCard" => {
                let mut viewport = PreviewCardViewport::<()>::new();
                viewport.extend(self.child_elements(node, key, cx));
                let mut popup = PreviewCardPopup::<()>::new()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .p_3()
                    .min_w(px(220.))
                    .bg(rgb(WHITE))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .rounded_md()
                    .shadow_lg()
                    .child(viewport);
                if prop_bool(node, "arrow") {
                    popup = popup.child(overlay_arrow(PreviewCardArrow::<()>::new()));
                }
                let mut portal = PreviewCardPortal::<()>::new();
                if prop_bool(node, "backdrop") {
                    portal = portal.child(overlay_backdrop(PreviewCardBackdrop::<()>::new()));
                }
                portal = portal.child(
                    PreviewCardPositioner::<()>::new()
                        .side(preview_card_side(node))
                        .side_offset(px(4.))
                        .child(popup),
                );
                let mut root = PreviewCardRoot::<()>::new()
                    .id(key.to_string())
                    .child(
                        PreviewCardTrigger::<()>::new()
                            .id(format!("{key}/trigger"))
                            .text_sm()
                            .text_color(rgb(ACCENT))
                            .child(SharedString::from(prop_str(node, "label"))),
                    )
                    .child(portal);
                if let Some(cb) = event_cb(view, node, "open", cx) {
                    root = root.on_open_change(move |open, _d, w, cx| {
                        cb(open.to_string(), w, cx);
                    });
                }
                Some(root.into_any_element())
            }
            "Drawer" => {
                let mut popup = DrawerPopup::<()>::new()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_4()
                    .w_full()
                    .max_h(px(320.))
                    .bg(rgb(WHITE))
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .shadow_lg();
                if let Some(title) = prop_str_opt(node, "title") {
                    popup = popup.child(
                        DialogTitle::<()>::new()
                            .text_base()
                            .text_color(rgb(TEXT))
                            .child(SharedString::from(title)),
                    );
                }
                if let Some(desc) = prop_str_opt(node, "description") {
                    popup = popup.child(
                        DialogDescription::<()>::new()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .child(SharedString::from(desc)),
                    );
                }
                let mut content = DrawerContent::<()>::new().flex().flex_col().gap_2();
                content.extend(self.child_elements(node, key, cx));
                popup = popup.child(content);
                if prop_bool(node, "close") {
                    popup = popup.child(
                        DialogClose::<()>::new()
                            .px_3()
                            .h_8()
                            .rounded_md()
                            .bg(rgb(TRACK))
                            .text_sm()
                            .text_color(rgb(TEXT))
                            .child("Close"),
                    );
                }
                let mut root = DrawerRoot::<()>::new()
                    .id(key.to_string())
                    .default_open(prop_bool(node, "open"))
                    .child(
                        DialogTrigger::<()>::new()
                            .id(format!("{key}/trigger"))
                            .flex()
                            .items_center()
                            .justify_center()
                            .px_3()
                            .h_8()
                            .min_w(px(64.))
                            .rounded_md()
                            .text_sm()
                            .bg(rgb(ACCENT))
                            .text_color(rgb(WHITE))
                            .child(SharedString::from(prop_str(node, "label"))),
                    )
                    .child(
                        DrawerPortal::<()>::new()
                            .child(
                                DrawerBackdrop::<()>::new()
                                    .absolute()
                                    .top_0()
                                    .left_0()
                                    .w_full()
                                    .h_full()
                                    .bg(rgba(0x18181b66)),
                            )
                            .child(
                                DrawerViewport::<()>::new()
                                    .absolute()
                                    .top_0()
                                    .left_0()
                                    .w_full()
                                    .h_full()
                                    .flex()
                                    .flex_col()
                                    .justify_end()
                                    .child(popup),
                            ),
                    );
                if let Some(cb) = value_change(view, node, "open", "open", cx) {
                    root = root.on_open_change(move |open, _d, w, cx| {
                        cb(Json::Bool(open), open.to_string(), w, cx);
                    });
                }
                Some(root.into_any_element())
            }
            "Select" => {
                let mut list = SelectList::<SharedString>::new()
                    .flex()
                    .flex_col()
                    .gap(px(1.));
                for (i, c) in node.children.iter().enumerate() {
                    match c.element.name.as_str() {
                        "Option" => {
                            list = list.child(select_item(c, key, i));
                        }
                        "OptionGroup" => {
                            list = list.child(select_group(c, &format!("{key}/group-{i}")));
                        }
                        "OptionSeparator" => {
                            list = list.child(select_separator());
                        }
                        _ => {}
                    }
                }
                let mut popup = SelectPopup::<SharedString>::new()
                    .w(px(180.))
                    .p_1()
                    .bg(rgb(WHITE))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .rounded_md()
                    .shadow_lg();
                if prop_bool(node, "arrow") {
                    popup = popup.child(overlay_arrow(SelectArrow::<SharedString>::new()));
                }
                popup = popup.child(list);
                let mut portal = SelectPortal::<SharedString>::new();
                if prop_bool(node, "backdrop") {
                    portal = portal.child(overlay_backdrop(SelectBackdrop::<SharedString>::new()));
                }
                portal = portal.child(
                    SelectPositioner::<SharedString>::new()
                        .side_offset(px(4.))
                        .child(popup),
                );
                let mut root = SelectRoot::<SharedString>::new()
                    .id(key.to_string())
                    .disabled(prop_bool(node, "disabled"))
                    .item_to_string_value(|v| v.clone())
                    .label_resolver(|v| v.clone())
                    .flex()
                    .flex_col()
                    .gap_1();
                if let Some(label) = prop_str_opt(node, "label") {
                    root = root.child(
                        SelectLabel::<SharedString>::new()
                            .text_sm()
                            .text_color(rgb(TEXT))
                            .child(SharedString::from(label)),
                    );
                }
                root = root
                    .child(
                        SelectTrigger::<SharedString>::new()
                            .id(format!("{key}/trigger"))
                            .w(px(180.))
                            .h_8()
                            .px_2()
                            .flex()
                            .items_center()
                            .justify_between()
                            .border_1()
                            .border_color(rgb(BORDER))
                            .rounded_md()
                            .text_sm()
                            .text_color(rgb(TEXT))
                            .bg(rgb(WHITE))
                            .child(
                                SelectValue::<SharedString>::new()
                                    .placeholder(prop_str(node, "placeholder")),
                            )
                            .child(
                                SelectIcon::<SharedString>::new()
                                    .text_xs()
                                    .text_color(rgb(MUTED))
                                    .child("⌄"),
                            ),
                    )
                    .child(portal);
                if let Some(v) = prop_str_opt(node, "value") {
                    root = root.default_value(Some(SharedString::from(v)));
                }
                if let Some(cb) = value_change(view, node, "value", "change", cx) {
                    root = root.on_value_change(move |value, _d, w, cx| {
                        let (json, text) = opt_value(value);
                        cb(json, text, w, cx);
                    });
                }
                if let Some(cb) = value_change(view, node, "open", "open", cx) {
                    root = root.on_open_change(move |open, _d, w, cx| {
                        cb(Json::Bool(open), open.to_string(), w, cx);
                    });
                }
                Some(root.into_any_element())
            }
            "Menu" => {
                let root = self.build_menu(view, node, key, cx);
                Some(root.into_any_element())
            }
            "ContextMenu" => {
                let mut root = ContextMenuRoot::<()>::new().id(key.to_string());
                if let Some(cb) = event_cb(view, node, "open", cx) {
                    root = root.on_open_change(move |open, _d, w, cx| {
                        cb(open.to_string(), w, cx);
                    });
                }
                let mut items = Vec::new();
                for (i, c) in node.children.iter().enumerate() {
                    if c.element.name == "ContextMenuArea" {
                        let mut trigger = ContextMenuTrigger::<()>::new()
                            .id(format!("{key}/area"))
                            .w_full();
                        trigger.extend(self.child_elements(c, &format!("{key}/area"), cx));
                        root = root.child(trigger);
                    } else if let Some(item) =
                        self.menu_popup_child(view, c, &format!("{key}/i{i}"), cx)
                    {
                        items.push(item);
                    }
                }
                let mut popup = MenuPopup::<()>::new()
                    .min_w(px(160.))
                    .p_1()
                    .bg(rgb(WHITE))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .rounded_md()
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .gap(px(1.));
                for item in items {
                    popup = popup.child(item);
                }
                if prop_bool(node, "arrow") {
                    popup = popup.child(overlay_arrow(MenuArrow::<()>::new()));
                }
                let mut portal = MenuPortal::<()>::new();
                if prop_bool(node, "backdrop") {
                    portal = portal.child(overlay_backdrop(MenuBackdrop::<()>::new()));
                }
                portal = portal.child(MenuPositioner::<()>::new().side_offset(px(4.)).child(popup));
                root = root.child(portal);
                Some(root.into_any_element())
            }
            "Menubar" => {
                let mut bar = Menubar::new()
                    .id(key.to_string())
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .p_1()
                    .border_1()
                    .border_color(rgb(BORDER))
                    .rounded_md();
                for (i, c) in node.children.iter().enumerate() {
                    if c.element.name == "Menu" {
                        bar = bar.child(self.build_menu(view, c, &format!("{key}/menu-{i}"), cx));
                    }
                }
                Some(bar.into_any_element())
            }
            "NavigationMenu" => {
                let mut list = NavigationMenuList::<SharedString>::new()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1();
                for (i, c) in node.children.iter().enumerate() {
                    if c.element.name != "NavigationMenuItem" {
                        continue;
                    }
                    let label = prop_str(c, "label");
                    let value = SharedString::from(
                        prop_str_opt(c, "value").unwrap_or_else(|| format!("{key}/nav-{i}")),
                    );
                    let trigger = NavigationMenuTrigger::<SharedString>::new()
                        .px_2()
                        .h_7()
                        .rounded_sm()
                        .text_sm()
                        .text_color(rgb(TEXT))
                        .child_any(SharedString::from(label));
                    let mut content = NavigationMenuContent::<SharedString>::new()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .p_3();
                    content.extend(self.child_elements(c, &format!("{key}/nav-{i}"), cx));
                    list = list.child(
                        NavigationMenuItem::<SharedString>::new()
                            .value(value)
                            .child(trigger)
                            .child(content),
                    );
                }
                let mut popup = NavigationMenuPopup::<SharedString>::new()
                    .min_w(px(200.))
                    .bg(rgb(WHITE))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .rounded_md()
                    .shadow_lg()
                    .child(NavigationMenuViewport::<SharedString>::new());
                if prop_bool(node, "arrow") {
                    popup = popup.child(overlay_arrow(NavigationMenuArrow::<SharedString>::new()));
                }
                let mut portal = NavigationMenuPortal::<SharedString>::new();
                if prop_bool(node, "backdrop") {
                    portal = portal.child(overlay_backdrop(
                        NavigationMenuBackdrop::<SharedString>::new(),
                    ));
                }
                portal = portal.child(
                    NavigationMenuPositioner::<SharedString>::new()
                        .side_offset(px(4.))
                        .child(popup),
                );
                let mut root = NavigationMenuRoot::<SharedString>::new()
                    .id(key.to_string())
                    .child(list)
                    .child(portal);
                if let Some(cb) = value_change(view, node, "value", "change", cx) {
                    root = root.on_value_change(move |value, _d, w, cx| {
                        let (json, text) = match value {
                            Some(v) => (Json::String(v.to_string()), v.to_string()),
                            None => (Json::Null, "null".to_string()),
                        };
                        cb(json, text, w, cx);
                    });
                }
                Some(root.into_any_element())
            }
            "Combobox" => {
                let mut list = ComboboxList::<SharedString>::new()
                    .flex()
                    .flex_col()
                    .gap(px(1.));
                for (i, c) in node.children.iter().enumerate() {
                    match c.element.name.as_str() {
                        "Option" => {
                            list = list.child(combobox_item(c, key, i));
                        }
                        "OptionGroup" => {
                            list = list.child(combobox_group(c, &format!("{key}/group-{i}")));
                        }
                        "OptionSeparator" => {
                            list = list.child(combobox_separator());
                        }
                        _ => {}
                    }
                }
                if let Some(empty) = prop_str_opt(node, "empty") {
                    list = list.child(
                        ComboboxEmpty::<SharedString>::new()
                            .px_2()
                            .py_2()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .child(SharedString::from(empty)),
                    );
                }
                if let Some(status) = prop_str_opt(node, "status") {
                    list = list.child(
                        ComboboxStatus::<SharedString>::new()
                            .px_2()
                            .py_1()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(SharedString::from(status)),
                    );
                }
                let mut popup = ComboboxPopup::<SharedString>::new()
                    .w(px(220.))
                    .p_1()
                    .bg(rgb(WHITE))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .rounded_md()
                    .shadow_lg();
                if prop_bool(node, "arrow") {
                    popup = popup.child(overlay_arrow(ComboboxArrow::<SharedString>::new()));
                }
                popup = popup.child(list);
                let mut portal = ComboboxPortal::<SharedString>::new();
                if prop_bool(node, "backdrop") {
                    portal =
                        portal.child(overlay_backdrop(ComboboxBackdrop::<SharedString>::new()));
                }
                portal = portal.child(
                    ComboboxPositioner::<SharedString>::new()
                        .side_offset(px(4.))
                        .child(popup),
                );
                let mut root = ComboboxRoot::<SharedString>::new()
                    .id(key.to_string())
                    .item_to_string_value(|v| v.clone())
                    .disabled(prop_bool(node, "disabled"))
                    .flex()
                    .flex_col()
                    .gap_1();
                if let Some(label) = prop_str_opt(node, "label") {
                    root = root.child(
                        ComboboxLabel::<SharedString>::new()
                            .text_sm()
                            .text_color(rgb(TEXT))
                            .child(SharedString::from(label)),
                    );
                }
                root = root
                    .child(
                        ComboboxInputGroup::<SharedString>::new()
                            .w(px(220.))
                            .h_8()
                            .px_2()
                            .flex()
                            .items_center()
                            .gap_1()
                            .border_1()
                            .border_color(rgb(BORDER))
                            .rounded_md()
                            .bg(rgb(WHITE))
                            .child(
                                ComboboxInput::<SharedString>::new()
                                    .id(format!("{key}/input"))
                                    .placeholder(prop_str(node, "placeholder"))
                                    .flex_1()
                                    .text_sm()
                                    .text_color(rgb(TEXT)),
                            )
                            .child(ComboboxClear::<SharedString>::new())
                            .child(
                                ComboboxTrigger::<SharedString>::new()
                                    .id(format!("{key}/trigger"))
                                    .child(
                                        ComboboxIcon::<SharedString>::new()
                                            .text_xs()
                                            .text_color(rgb(MUTED))
                                            .child("⌄"),
                                    ),
                            ),
                    )
                    .child(portal);
                if let Some(v) = prop_str_opt(node, "value") {
                    root = root.default_value(Some(SharedString::from(v)));
                }
                if let Some(cb) = value_change(view, node, "value", "change", cx) {
                    root = root.on_value_change(move |value, _d, w, cx| {
                        let (json, text) = opt_value(value);
                        cb(json, text, w, cx);
                    });
                }
                if let Some(cb) = value_change(view, node, "input", "input", cx) {
                    root = root.on_input_value_change(move |text, _d, w, cx| {
                        cb(Json::String(text.to_string()), text.to_string(), w, cx);
                    });
                }
                if let Some(cb) = value_change(view, node, "open", "open", cx) {
                    root = root.on_open_change(move |open, _d, w, cx| {
                        cb(Json::Bool(open), open.to_string(), w, cx);
                    });
                }
                Some(root.into_any_element())
            }
            "Autocomplete" => {
                let mut list = ComboboxList::<SharedString>::new()
                    .flex()
                    .flex_col()
                    .gap(px(1.));
                for (i, c) in node.children.iter().enumerate() {
                    match c.element.name.as_str() {
                        "Option" => {
                            list = list.child(combobox_item(c, key, i));
                        }
                        "OptionGroup" => {
                            list = list.child(combobox_group(c, &format!("{key}/group-{i}")));
                        }
                        "OptionSeparator" => {
                            list = list.child(combobox_separator());
                        }
                        _ => {}
                    }
                }
                if let Some(empty) = prop_str_opt(node, "empty") {
                    list = list.child(
                        ComboboxEmpty::<SharedString>::new()
                            .px_2()
                            .py_2()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .child(SharedString::from(empty)),
                    );
                }
                if let Some(status) = prop_str_opt(node, "status") {
                    list = list.child(
                        ComboboxStatus::<SharedString>::new()
                            .px_2()
                            .py_1()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(SharedString::from(status)),
                    );
                }
                let mut popup = ComboboxPopup::<SharedString>::new()
                    .w(px(220.))
                    .p_1()
                    .bg(rgb(WHITE))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .rounded_md()
                    .shadow_lg();
                if prop_bool(node, "arrow") {
                    popup = popup.child(overlay_arrow(ComboboxArrow::<SharedString>::new()));
                }
                popup = popup.child(list);
                let mut portal = ComboboxPortal::<SharedString>::new();
                if prop_bool(node, "backdrop") {
                    portal =
                        portal.child(overlay_backdrop(ComboboxBackdrop::<SharedString>::new()));
                }
                portal = portal.child(
                    ComboboxPositioner::<SharedString>::new()
                        .side_offset(px(4.))
                        .child(popup),
                );
                let mut root = AutocompleteRoot::<SharedString>::new()
                    .id(key.to_string())
                    .item_to_string_value(|v| v.clone())
                    .mode(match prop_str_opt(node, "mode").as_deref() {
                        Some("Both") => AutocompleteMode::Both,
                        Some("Inline") => AutocompleteMode::Inline,
                        Some("None") => AutocompleteMode::None,
                        _ => AutocompleteMode::List,
                    })
                    .child(
                        ComboboxInputGroup::<SharedString>::new()
                            .w(px(220.))
                            .h_8()
                            .px_2()
                            .flex()
                            .items_center()
                            .gap_1()
                            .border_1()
                            .border_color(rgb(BORDER))
                            .rounded_md()
                            .bg(rgb(WHITE))
                            .child(
                                ComboboxInput::<SharedString>::new()
                                    .id(format!("{key}/input"))
                                    .placeholder(prop_str(node, "placeholder"))
                                    .flex_1()
                                    .text_sm()
                                    .text_color(rgb(TEXT)),
                            )
                            .child(ComboboxClear::<SharedString>::new()),
                    )
                    .child(portal);
                if let Some(v) = prop_str_opt(node, "value") {
                    root = root.default_value(SharedString::from(v));
                }
                if let Some(cb) = value_change(view, node, "value", "change", cx) {
                    root = root.on_value_change(move |text, _d, w, cx| {
                        cb(Json::String(text.to_string()), text.to_string(), w, cx);
                    });
                }
                if let Some(cb) = value_change(view, node, "open", "open", cx) {
                    root = root.on_open_change(move |open, _d, w, cx| {
                        cb(Json::Bool(open), open.to_string(), w, cx);
                    });
                }
                Some(root.into_any_element())
            }
            _ => None,
        }
    }
}

fn toolbar_button(
    view: &ForgeView,
    node: &Node,
    key: &str,
    i: usize,
    cx: &mut Context<ForgeView>,
) -> ToolbarButton {
    let mut button = ToolbarButton::new()
        .id(format!("{key}/button-{i}"))
        .disabled(prop_bool(node, "disabled"))
        .px_2()
        .h_7()
        .rounded_sm()
        .text_sm()
        .style_with_state(|state, el| {
            if state.disabled {
                el.text_color(rgb(MUTED))
            } else {
                el.text_color(rgb(TEXT))
            }
        })
        .child(SharedString::from(prop_str(node, "label")));
    if let Some(cb) = fire_cb(view, node, &["click"], cx) {
        button = button.on_click(move |_e, w, cx| cb(w, cx));
    }
    button
}

fn toolbar_link(
    view: &ForgeView,
    node: &Node,
    key: &str,
    i: usize,
    cx: &mut Context<ForgeView>,
) -> ToolbarLink {
    let mut link = ToolbarLink::new()
        .id(format!("{key}/link-{i}"))
        .px_2()
        .h_7()
        .text_sm()
        .text_color(rgb(ACCENT))
        .child(SharedString::from(prop_str(node, "label")));
    if let Some(cb) = fire_cb(view, node, &["click"], cx) {
        link = link.on_click(move |_e, w, cx| cb(w, cx));
    }
    link
}

fn toolbar_input(node: &Node, key: &str, i: usize) -> ToolbarInput {
    ToolbarInput::new()
        .id(format!("{key}/input-{i}"))
        .placeholder(prop_str(node, "placeholder"))
        .disabled(prop_bool(node, "disabled"))
}

fn toolbar_group(
    view: &ForgeView,
    node: &Node,
    key: &str,
    i: usize,
    cx: &mut Context<ForgeView>,
) -> ToolbarGroup {
    let mut group = ToolbarGroup::new()
        .disabled(prop_bool(node, "disabled"))
        .flex()
        .flex_row()
        .items_center()
        .gap_1()
        .px_1();
    for (j, c) in node.children.iter().enumerate() {
        let gi = i * 100 + j;
        match c.element.name.as_str() {
            "ToolbarButton" => group = group.child(toolbar_button(view, c, key, gi, cx)),
            "ToolbarLink" => group = group.child(toolbar_link(view, c, key, gi, cx)),
            "ToolbarInput" => group = group.child(toolbar_input(c, key, gi)),
            _ => {}
        }
    }
    group
}

impl Base {
    /// Children of a `base.Menu` / `base.Submenu` popup or a
    /// `base.ContextMenu`: menu item parts or arbitrary proxied content.
    fn menu_popup_child(
        &self,
        view: &ForgeView,
        node: &Node,
        key: &str,
        cx: &mut Context<ForgeView>,
    ) -> Option<MenuPopupChild<()>> {
        match node.element.name.as_str() {
            "MenuItem" => Some(menu_item(view, node, key, cx).into()),
            "MenuLinkItem" => Some(menu_link_item(view, node, key, cx).into()),
            "MenuCheckboxItem" => Some(menu_checkbox_item(view, node, key, cx).into()),
            "MenuRadioGroup" => Some(self.menu_radio_group(view, node, key, cx).into()),
            "MenuGroup" => Some(self.menu_group(view, node, key, cx).into()),
            "MenuGroupLabel" => Some(menu_group_label(node, key).into()),
            "MenuSeparator" => Some(MenuSeparator::new().into()),
            "Submenu" => Some(self.submenu(view, node, key, cx).into()),
            _ => Some(MenuPopupChild::Any(
                self.child_view(node, key, cx).into_any_element(),
            )),
        }
    }

    fn menu_group_child(
        &self,
        view: &ForgeView,
        node: &Node,
        key: &str,
        cx: &mut Context<ForgeView>,
    ) -> Option<MenuGroupChild<()>> {
        match node.element.name.as_str() {
            "MenuItem" => Some(menu_item(view, node, key, cx).into()),
            "MenuLinkItem" => Some(menu_link_item(view, node, key, cx).into()),
            "MenuCheckboxItem" => Some(menu_checkbox_item(view, node, key, cx).into()),
            "MenuGroupLabel" => Some(menu_group_label(node, key).into()),
            "MenuSeparator" => Some(MenuSeparator::new().into()),
            _ => Some(MenuGroupChild::Any(
                self.child_view(node, key, cx).into_any_element(),
            )),
        }
    }

    fn menu_radio_group_child(
        &self,
        node: &Node,
        key: &str,
        cx: &mut Context<ForgeView>,
    ) -> Option<MenuRadioGroupChild<(), SharedString>> {
        match node.element.name.as_str() {
            "MenuRadioItem" | "RadioItem" => Some(menu_radio_item(node, key).into()),
            "MenuGroupLabel" => Some(menu_group_label(node, key).into()),
            "MenuSeparator" => Some(MenuSeparator::new().into()),
            _ => Some(MenuRadioGroupChild::Any(
                self.child_view(node, key, cx).into_any_element(),
            )),
        }
    }

    fn menu_group(
        &self,
        view: &ForgeView,
        node: &Node,
        key: &str,
        cx: &mut Context<ForgeView>,
    ) -> MenuGroup<()> {
        let mut group = MenuGroup::<()>::new().id(format!("{key}-group"));
        for (i, c) in node.children.iter().enumerate() {
            if let Some(child) = self.menu_group_child(view, c, &format!("{key}/g{i}"), cx) {
                group = group.child(child);
            }
        }
        group
    }

    fn menu_radio_group(
        &self,
        view: &ForgeView,
        node: &Node,
        key: &str,
        cx: &mut Context<ForgeView>,
    ) -> MenuRadioGroup<(), SharedString> {
        let mut group = MenuRadioGroup::<(), SharedString>::new().id(format!("{key}-rg"));
        if let Some(v) = prop_str_opt(node, "value") {
            group = group.default_value(Some(SharedString::from(v)));
        }
        for (i, c) in node.children.iter().enumerate() {
            if let Some(child) = self.menu_radio_group_child(c, &format!("{key}/r{i}"), cx) {
                group = group.child(child);
            }
        }
        if let Some(cb) = value_change(view, node, "value", "change", cx) {
            group = group.on_value_change(move |value, _d, w, cx| {
                cb(Json::String(value.to_string()), value.to_string(), w, cx);
            });
        }
        group
    }

    fn submenu(
        &self,
        view: &ForgeView,
        node: &Node,
        key: &str,
        cx: &mut Context<ForgeView>,
    ) -> MenuSubmenuRoot<()> {
        let mut items = Vec::new();
        for (i, c) in node.children.iter().enumerate() {
            if let Some(item) = self.menu_popup_child(view, c, &format!("{key}/s{i}"), cx) {
                items.push(item);
            }
        }
        let mut popup = menu_popup();
        for item in items {
            popup = popup.child(item);
        }
        if prop_bool(node, "arrow") {
            popup = popup.child(overlay_arrow(MenuArrow::<()>::new()));
        }
        let mut portal = MenuPortal::<()>::new();
        if prop_bool(node, "backdrop") {
            portal = portal.child(overlay_backdrop(MenuBackdrop::<()>::new()));
        }
        portal = portal.child(MenuPositioner::<()>::new().side_offset(px(4.)).child(popup));
        let mut sub = MenuSubmenuRoot::<()>::new()
            .id(format!("{key}-sub"))
            .child(
                MenuSubmenuTrigger::<()>::new()
                    .id(format!("{key}/trigger"))
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_2()
                    .h_7()
                    .rounded_sm()
                    .text_sm()
                    .text_color(rgb(TEXT))
                    .child(SharedString::from(prop_str(node, "label")))
                    .child(div().text_xs().text_color(rgb(MUTED)).child("›")),
            )
            .child(portal);
        if let Some(cb) = event_cb(view, node, "open", cx) {
            sub = sub.on_open_change(move |open, _d, w, cx| {
                cb(open.to_string(), w, cx);
            });
        }
        sub
    }

    /// `base.Menu` (including each `base.Menu` hosted by `base.Menubar`)
    /// → `MenuRoot { trigger, portal { positioner { popup { items } } } }`.
    fn build_menu(
        &self,
        view: &ForgeView,
        node: &Node,
        key: &str,
        cx: &mut Context<ForgeView>,
    ) -> MenuRoot<()> {
        let mut items = Vec::new();
        for (i, c) in node.children.iter().enumerate() {
            if let Some(item) = self.menu_popup_child(view, c, &format!("{key}/i{i}"), cx) {
                items.push(item);
            }
        }
        let mut popup = menu_popup();
        for item in items {
            popup = popup.child(item);
        }
        if prop_bool(node, "arrow") {
            popup = popup.child(overlay_arrow(MenuArrow::<()>::new()));
        }
        let mut portal = MenuPortal::<()>::new();
        if prop_bool(node, "backdrop") {
            portal = portal.child(overlay_backdrop(MenuBackdrop::<()>::new()));
        }
        portal = portal.child(MenuPositioner::<()>::new().side_offset(px(4.)).child(popup));
        let mut root = MenuRoot::<()>::new()
            .id(format!("{key}-menu"))
            .child(
                MenuTrigger::<()>::new()
                    .id(format!("{key}/trigger"))
                    .flex()
                    .items_center()
                    .px_2()
                    .h_7()
                    .rounded_sm()
                    .text_sm()
                    .text_color(rgb(TEXT))
                    .child(SharedString::from(prop_str(node, "label"))),
            )
            .child(portal);
        if let Some(cb) = event_cb(view, node, "open", cx) {
            root = root.on_open_change(move |open, _d, w, cx| {
                cb(open.to_string(), w, cx);
            });
        }
        root
    }
}

fn menu_popup() -> MenuPopup<()> {
    MenuPopup::<()>::new()
        .min_w(px(160.))
        .p_1()
        .bg(rgb(WHITE))
        .border_1()
        .border_color(rgb(BORDER))
        .rounded_md()
        .shadow_lg()
        .flex()
        .flex_col()
        .gap(px(1.))
}

fn menu_item(
    view: &ForgeView,
    node: &Node,
    key: &str,
    cx: &mut Context<ForgeView>,
) -> MenuItem<()> {
    let label = prop_str(node, "label");
    let mut item = MenuItem::<()>::new()
        .id(format!("{key}-item"))
        .disabled(prop_bool(node, "disabled"))
        .label(SharedString::from(label.clone()))
        .flex()
        .items_center()
        .px_2()
        .h_7()
        .rounded_sm()
        .text_sm()
        .style_with_state(|state: MenuItemStyleState, el| {
            if state.disabled {
                el.text_color(rgb(MUTED))
            } else if state.highlighted {
                el.bg(rgb(TRACK)).text_color(rgb(TEXT))
            } else {
                el.text_color(rgb(TEXT))
            }
        })
        .child(SharedString::from(label));
    if let Some(cb) = fire_cb(view, node, &["click", "activate"], cx) {
        item = item.on_click(move |w, cx| cb(w, cx));
    }
    item
}

fn menu_link_item(
    view: &ForgeView,
    node: &Node,
    key: &str,
    cx: &mut Context<ForgeView>,
) -> MenuLinkItem<()> {
    let label = prop_str(node, "label");
    let mut item = MenuLinkItem::<()>::new()
        .id(format!("{key}-link"))
        .label(SharedString::from(label.clone()))
        .flex()
        .items_center()
        .px_2()
        .h_7()
        .rounded_sm()
        .text_sm()
        .style_with_state(|state: MenuLinkItemStyleState, el| {
            if state.highlighted {
                el.bg(rgb(TRACK)).text_color(rgb(TEXT))
            } else {
                el.text_color(rgb(TEXT))
            }
        });
    item.extend(vec![SharedString::from(label).into_any_element()]);
    if let Some(cb) = fire_cb(view, node, &["activate", "click"], cx) {
        item = item.on_activate(move |w, cx| cb(w, cx));
    }
    item
}

fn menu_checkbox_item(
    view: &ForgeView,
    node: &Node,
    key: &str,
    cx: &mut Context<ForgeView>,
) -> MenuCheckboxItem<()> {
    let label = prop_str(node, "label");
    let mut item = MenuCheckboxItem::<()>::new()
        .id(format!("{key}-cb"))
        .disabled(prop_bool(node, "disabled"))
        .default_checked(prop_bool(node, "checked"))
        .label(SharedString::from(label.clone()))
        .flex()
        .items_center()
        .px_2()
        .h_7()
        .rounded_sm()
        .text_sm()
        .style_with_state(|state: MenuCheckboxItemStyleState, el| {
            if state.disabled {
                el.text_color(rgb(MUTED))
            } else if state.highlighted {
                el.bg(rgb(TRACK)).text_color(rgb(TEXT))
            } else {
                el.text_color(rgb(TEXT))
            }
        })
        .child(
            MenuCheckboxItemIndicator::<()>::new()
                .keep_mounted(true)
                .w(px(14.))
                .text_xs()
                .text_color(rgb(ACCENT))
                .child("✓"),
        )
        .child_any(SharedString::from(label));
    if let Some(cb) = value_change(view, node, "checked", "change", cx) {
        item = item.on_checked_change(move |checked, _d, w, cx| {
            cb(Json::Bool(checked), checked.to_string(), w, cx);
        });
    }
    item
}

fn menu_radio_item(node: &Node, key: &str) -> MenuRadioItem<(), SharedString> {
    let label = prop_str(node, "label");
    let value = prop_str_opt(node, "value").unwrap_or_else(|| label.clone());
    MenuRadioItem::<(), SharedString>::new()
        .id(format!("{key}-radio"))
        .disabled(prop_bool(node, "disabled"))
        .value(SharedString::from(value))
        .label(SharedString::from(label.clone()))
        .flex()
        .items_center()
        .px_2()
        .h_7()
        .rounded_sm()
        .text_sm()
        .style_with_state(|state: MenuRadioItemStyleState, el| {
            if state.disabled {
                el.text_color(rgb(MUTED))
            } else if state.highlighted {
                el.bg(rgb(TRACK)).text_color(rgb(TEXT))
            } else {
                el.text_color(rgb(TEXT))
            }
        })
        .child(
            MenuRadioItemIndicator::<(), SharedString>::new()
                .keep_mounted(true)
                .w(px(14.))
                .text_xs()
                .text_color(rgb(ACCENT))
                .child("●"),
        )
        .child_any(SharedString::from(label))
}

fn menu_group_label(node: &Node, _key: &str) -> MenuGroupLabel<()> {
    MenuGroupLabel::<()>::new()
        .label(SharedString::from(prop_str(node, "label")))
        .px_2()
        .py_1()
        .text_xs()
        .text_color(rgb(MUTED))
}

/// Shared styling for the little anchor arrow inside overlay popups.
fn overlay_arrow<E: Styled>(el: E) -> E {
    el.w(px(12.)).h(px(6.)).bg(rgb(WHITE))
}

/// Shared styling for dimming backdrops inside overlay portals.
fn overlay_backdrop<E: Styled>(el: E) -> E {
    el.absolute()
        .top_0()
        .left_0()
        .w_full()
        .h_full()
        .bg(rgba(0x18181b66))
}

fn select_item(node: &Node, key: &str, i: usize) -> SelectItem<SharedString> {
    let label = prop_str(node, "label");
    let value = SharedString::from(prop_str_opt(node, "value").unwrap_or_else(|| label.clone()));
    SelectItem::<SharedString>::new()
        .id(format!("{key}/option-{i}"))
        .value(value)
        .label(SharedString::from(label.clone()))
        .disabled(prop_bool(node, "disabled"))
        .px_2()
        .h_7()
        .rounded_sm()
        .flex()
        .items_center()
        .gap_2()
        .text_sm()
        .style_with_state(|state, el| {
            if state.disabled {
                el.text_color(rgb(MUTED))
            } else if state.highlighted || state.selected {
                el.bg(rgb(TRACK)).text_color(rgb(TEXT))
            } else {
                el.text_color(rgb(TEXT))
            }
        })
        .child(
            SelectItemIndicator::<SharedString>::new()
                .keep_mounted(true)
                .w(px(14.))
                .text_xs()
                .text_color(rgb(ACCENT))
                .child("✓"),
        )
        .child(SelectItemText::<SharedString>::new().text(label))
}

fn select_group(node: &Node, key: &str) -> SelectGroup<SharedString> {
    let mut group = SelectGroup::<SharedString>::new()
        .flex()
        .flex_col()
        .gap(px(1.));
    if let Some(label) = prop_str_opt(node, "label") {
        group = group.child(
            SelectGroupLabel::<SharedString>::new()
                .px_2()
                .py_1()
                .text_xs()
                .text_color(rgb(MUTED))
                .text(label),
        );
    }
    for (i, c) in node.children.iter().enumerate() {
        match c.element.name.as_str() {
            "Option" => group = group.child(select_item(c, key, i)),
            "OptionSeparator" => group = group.child(select_separator()),
            _ => {}
        }
    }
    group
}

fn select_separator() -> SelectSeparator {
    SelectSeparator::new()
        .h(px(1.))
        .w_full()
        .my_1()
        .bg(rgb(BORDER))
}

fn combobox_item(node: &Node, key: &str, i: usize) -> ComboboxItem<SharedString> {
    let label = prop_str(node, "label");
    let value = SharedString::from(prop_str_opt(node, "value").unwrap_or_else(|| label.clone()));
    ComboboxItem::<SharedString>::new()
        .id(format!("{key}/option-{i}"))
        .value(value)
        .label(SharedString::from(label.clone()))
        .disabled(prop_bool(node, "disabled"))
        .px_2()
        .h_7()
        .rounded_sm()
        .flex()
        .items_center()
        .gap_2()
        .text_sm()
        .style_with_state(|state, el| {
            if state.disabled {
                el.text_color(rgb(MUTED))
            } else if state.highlighted || state.selected {
                el.bg(rgb(TRACK)).text_color(rgb(TEXT))
            } else {
                el.text_color(rgb(TEXT))
            }
        })
        .child(
            ComboboxItemIndicator::<SharedString>::new()
                .keep_mounted(true)
                .w(px(14.))
                .text_xs()
                .text_color(rgb(ACCENT))
                .child("✓"),
        )
        .child_any(div().child(SharedString::from(label)))
}

fn combobox_group(node: &Node, key: &str) -> ComboboxGroup<SharedString> {
    let mut group = ComboboxGroup::<SharedString>::new()
        .flex()
        .flex_col()
        .gap(px(1.));
    if let Some(label) = prop_str_opt(node, "label") {
        group = group.child(
            ComboboxGroupLabel::<SharedString>::new()
                .px_2()
                .py_1()
                .text_xs()
                .text_color(rgb(MUTED))
                .text(label),
        );
    }
    for (i, c) in node.children.iter().enumerate() {
        match c.element.name.as_str() {
            "Option" => group = group.child(combobox_item(c, key, i)),
            "OptionSeparator" => group = group.child(combobox_separator()),
            _ => {}
        }
    }
    group
}

fn combobox_separator() -> ComboboxSeparator {
    ComboboxSeparator::new()
        .h(px(1.))
        .w_full()
        .my_1()
        .bg(rgb(BORDER))
}

fn tooltip_side(node: &Node) -> TooltipSide {
    match prop_str_opt(node, "side").as_deref() {
        Some("Bottom") => TooltipSide::Bottom,
        Some("Left") => TooltipSide::Left,
        Some("Right") => TooltipSide::Right,
        Some("InlineStart") => TooltipSide::InlineStart,
        Some("InlineEnd") => TooltipSide::InlineEnd,
        _ => TooltipSide::Top,
    }
}

fn popover_side(node: &Node) -> PopoverSide {
    match prop_str_opt(node, "side").as_deref() {
        Some("Top") => PopoverSide::Top,
        Some("Left") => PopoverSide::Left,
        Some("Right") => PopoverSide::Right,
        Some("InlineStart") => PopoverSide::InlineStart,
        Some("InlineEnd") => PopoverSide::InlineEnd,
        _ => PopoverSide::Bottom,
    }
}

fn preview_card_side(node: &Node) -> PreviewCardSide {
    match prop_str_opt(node, "side").as_deref() {
        Some("Top") => PreviewCardSide::Top,
        Some("Left") => PreviewCardSide::Left,
        Some("Right") => PreviewCardSide::Right,
        Some("InlineStart") => PreviewCardSide::InlineStart,
        Some("InlineEnd") => PreviewCardSide::InlineEnd,
        _ => PreviewCardSide::Bottom,
    }
}

// === Event wiring ===
// `on <event>:` bindings reach these widgets through base-gpui callbacks that
// run during the widget's own update, so they bounce through a WeakEntity and
// tolerate a held borrow (try_update) rather than re-entering ForgeView.

/// The state path a `{{…}}`-bound prop points at.
fn binding_of(node: &Node, name: &str) -> Option<Path> {
    match prop(node, name) {
        Some(Value::Binding { path, .. }) => Some(path.clone()),
        _ => None,
    }
}

/// Update the entity through a weak handle without panicking if a render pass
/// holds a borrow — mirrors the base renderer's `try_update`.
fn try_update(
    weak: &WeakEntity<ForgeView>,
    cx: &mut App,
    f: impl FnOnce(&mut ForgeView, &mut Context<ForgeView>),
) {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    let weak = weak.clone();
    let _ = catch_unwind(AssertUnwindSafe(|| {
        weak.update(cx, |v, cx| f(v, cx)).ok();
    }));
}

/// `on <event>:` calls on a node, resolved to `(handler, args)` pairs.
fn calls_for(view: &ForgeView, node: &Node, event: &str) -> Vec<(String, Vec<String>)> {
    node.events
        .iter()
        .filter(|e| e.event == event)
        .map(|e| {
            (
                e.handler.name.clone(),
                e.handler
                    .args
                    .iter()
                    .map(|a| {
                        view.resolve(a)
                            .as_str()
                            .map(str::to_owned)
                            .unwrap_or_default()
                    })
                    .collect(),
            )
        })
        .collect()
}

type ValueCb = Rc<dyn Fn(Json, String, &mut Window, &mut App)>;
type EventCb = Rc<dyn Fn(String, &mut Window, &mut App)>;
type FireCb = Rc<dyn Fn(&mut Window, &mut App)>;

/// A `{{binding}}` on `prop_name` plus `on <event>:` calls → callback taking the
/// new value as `Json` (for the state write-back) and `String` (appended to
/// each hook call's args).
fn value_change(
    view: &ForgeView,
    node: &Node,
    prop_name: &str,
    event: &str,
    cx: &mut Context<ForgeView>,
) -> Option<ValueCb> {
    let path = binding_of(node, prop_name);
    let calls = calls_for(view, node, event);
    if path.is_none() && calls.is_empty() {
        return None;
    }
    let weak = cx.entity().downgrade();
    Some(Rc::new(move |json: Json, payload: String, _w, cx| {
        if let Some(path) = path.clone() {
            let json = json.clone();
            try_update(&weak, cx, |v, cx| v.update_state(&path, json, cx));
        }
        for (name, args) in &calls {
            let (name, mut args) = (name.clone(), args.clone());
            args.push(payload.clone());
            try_update(&weak, cx, |v, cx| v.fire_call(&name, args, cx));
        }
    }))
}

/// `on <event>:` calls with a payload string appended when non-empty — for
/// callbacks that don't own a bound prop (`submit`, `input`, `complete`).
fn event_cb(
    view: &ForgeView,
    node: &Node,
    event: &str,
    cx: &mut Context<ForgeView>,
) -> Option<EventCb> {
    let calls = calls_for(view, node, event);
    if calls.is_empty() {
        return None;
    }
    let weak = cx.entity().downgrade();
    Some(Rc::new(move |payload: String, _w, cx| {
        for (name, args) in &calls {
            let (name, mut args) = (name.clone(), args.clone());
            if !payload.is_empty() {
                args.push(payload.clone());
            }
            try_update(&weak, cx, |v, cx| v.fire_call(&name, args, cx));
        }
    }))
}

/// No-payload `on <event>:` calls across several event names — menu/toolbar
/// activation, which Forge exposes as `activate` (and tolerates `click`).
fn fire_cb(
    view: &ForgeView,
    node: &Node,
    events: &[&str],
    cx: &mut Context<ForgeView>,
) -> Option<FireCb> {
    let calls: Vec<(String, Vec<String>)> = events
        .iter()
        .flat_map(|e| calls_for(view, node, e))
        .collect();
    if calls.is_empty() {
        return None;
    }
    let weak = cx.entity().downgrade();
    Some(Rc::new(move |_w, cx| {
        for (name, args) in &calls {
            let (name, args) = (name.clone(), args.clone());
            try_update(&weak, cx, |v, cx| v.fire_call(&name, args, cx));
        }
    }))
}

/// FormValues map → JSON object for the `submit` hook payload.
fn form_value_json(v: &FormValue) -> Json {
    match v {
        FormValue::Empty => Json::Null,
        FormValue::Present => Json::Bool(true),
        FormValue::Bool(b) => Json::Bool(*b),
        FormValue::Text(s) => Json::String(s.to_string()),
        FormValue::List(l) => Json::Array(l.iter().map(|s| Json::String(s.to_string())).collect()),
        FormValue::Number(n) => serde_json::json!(n),
    }
}

/// `T = SharedString` option payload → (Json, arg string).
fn opt_value(v: Option<&SharedString>) -> (Json, String) {
    match v {
        Some(v) => (Json::String(v.to_string()), v.to_string()),
        None => (Json::Null, "null".to_string()),
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
