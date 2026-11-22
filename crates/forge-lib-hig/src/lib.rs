//! Adapter for `gpui-apple-hig` (`use: hig: gpui-apple-hig` → `hig.*` elements)
//! — #90. MacOS-HIG components on the gpui-kit base. Render-only arms like the
//! Ely adapter (hook wiring is follow-up work).

use gpui_apple_hig::activity_rings::MacActivityRings;
use gpui_apple_hig::badge::MacBadge;
use gpui_apple_hig::button::MacButton;
use gpui_apple_hig::chart::{MacChart, MacChartType};
use gpui_apple_hig::checkbox::MacCheckbox;
use gpui_apple_hig::color_well::MacColorWell;
use gpui_apple_hig::column_view::{MacColumnEntry, MacColumnView};
use gpui_apple_hig::combo_box::MacComboBox;
use gpui_apple_hig::context_menu::MacContextMenu;
use gpui_apple_hig::dialog::MacDialog;
use gpui_apple_hig::digit_entry::MacDigitEntry;
use gpui_apple_hig::disclosure::MacDisclosure;
use gpui_apple_hig::gauge::MacGauge;
use gpui_apple_hig::image_view::{MacImageContentMode, MacImageView};
use gpui_apple_hig::image_well::MacImageWell;
use gpui_apple_hig::kbd::MacKbd;
use gpui_apple_hig::material::{MacMaterial, MacMaterialSurface, MacMaterialThickness};
use gpui_apple_hig::menu_bar::MacMenuBar;
use gpui_apple_hig::outline_view::{MacOutlineItem, MacOutlineView};
use gpui_apple_hig::page_control::MacPageControl;
use gpui_apple_hig::panel::MacPanel;
use gpui_apple_hig::path_control::{MacPathControl, MacPathItem};
use gpui_apple_hig::picker::{MacDatePicker, MacDatePickerStyle};
use gpui_apple_hig::popover::MacPopover;
use gpui_apple_hig::popup_button::{MacMenuItem, MacPopupButton, MacPullDownButton};
use gpui_apple_hig::progress::{MacProgress, MacSpinner};
use gpui_apple_hig::radio::MacRadio;
use gpui_apple_hig::rating_indicator::MacRatingIndicator;
use gpui_apple_hig::search_field::MacSearchField;
use gpui_apple_hig::segmented::{MacSegment, MacSegmentedControl};
use gpui_apple_hig::settings::{MacSettingsGroup, MacSettingsRow};
use gpui_apple_hig::sidebar::{MacSidebar, MacSidebarItem, MacSidebarSection};
use gpui_apple_hig::slider::MacSlider;
use gpui_apple_hig::split_view::MacSplitView;
use gpui_apple_hig::status_bar::MacStatusBar;
use gpui_apple_hig::stepper::MacStepper;
use gpui_apple_hig::switch::MacSwitch;
use gpui_apple_hig::tab_view::{MacTabBar, MacTabItem};
use gpui_apple_hig::table_view::MacTableView;
use gpui_apple_hig::text_field::{MacSecureField, MacTextField};
use gpui_apple_hig::text_view::MacTextView;
use gpui_apple_hig::theme::MacAccentColor;
use gpui_apple_hig::token_field::{MacToken, MacTokenField};
use gpui_apple_hig::toolbar::MacToolbar;
use gpui_apple_hig::window::MacWindow;
use gpui_forge_runtime::base::kit::assets::IconName;
use gpui_forge_runtime::base::kit::component::input::InputState;
use gpui_forge_runtime::base::kit::*;
use gpui_forge_runtime::forge_spec::catalog::{self, Catalog};
use gpui_forge_runtime::forge_spec::{Node, Path, Value};
use gpui_forge_runtime::render::{ForgeLibrary, ForgeView, get_path};
use serde_json::Value as Json;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::OnceLock;

thread_local! {
    /// `hig.*` element key → proxy view for children rendered through slots
    /// (`.child()`, `.control()`, `.leading()` …) — same lazy-proxy pattern as
    /// the Ely adapter, kept in a thread-local so `Hig` stays a unit struct.
    static VIEWS: RefCell<HashMap<String, AnyView>> = RefCell::new(HashMap::new());
    /// `hig.*` element key → `InputState` entity for the Mac text fields.
    static INPUTS: RefCell<HashMap<String, Entity<InputState>>> =
        RefCell::new(HashMap::new());
}

/// A `ForgeView`-hosted entity that re-renders one doc child inside an
/// adapter slot — avoids re-entering `ForgeView` mid-render.
pub struct HigProxy {
    view: WeakEntity<ForgeView>,
    node: Node,
    key: String,
}

impl Render for HigProxy {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(view) = self.view.upgrade() else {
            return div().into_any_element();
        };
        view.update(cx, |view, cx| {
            view.render_node(&self.node, &self.key, window, cx)
        })
    }
}

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
        window: &mut Window,
        cx: &mut Context<ForgeView>,
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

            "Kbd" => {
                Some(MacKbd::new(key.to_string(), prop_str(node, "keys", view)).into_any_element())
            }
            "Gauge" => {
                let mut g = MacGauge::new(
                    key.to_string(),
                    prop_num(node, "value", view).unwrap_or(0.0) as f32,
                );
                if let Some(l) = prop_str_opt(node, "label", view) {
                    g = g.label(l);
                }
                Some(g.into_any_element())
            }
            "Chart" => {
                let mut c = MacChart::new(key.to_string());
                if prop_str(node, "type", view) == "Line" {
                    c = c.chart_type(MacChartType::Line);
                }
                if let Some(t) = prop_str_opt(node, "title", view) {
                    c = c.title(t);
                }
                for p in prop_list(node, "points") {
                    let mut parts = p.splitn(3, ':');
                    let (label, value) = (
                        parts.next().unwrap_or_default().to_string(),
                        parts
                            .next()
                            .and_then(|v| v.parse::<f32>().ok())
                            .unwrap_or(0.0),
                    );
                    c = match parts.next() {
                        Some(color) => c.point_colored(label, value, accent_of(color)),
                        None => c.point(label, value),
                    };
                }
                Some(c.into_any_element())
            }
            "ActivityRings" => {
                let mut a = MacActivityRings::new(key.to_string());
                for r in prop_list(node, "rings") {
                    let mut parts = r.splitn(3, ':');
                    let name = parts.next().unwrap_or_default().to_string();
                    let progress = parts
                        .next()
                        .and_then(|v| v.parse::<f32>().ok())
                        .unwrap_or(0.0);
                    a = a.ring(
                        name,
                        progress,
                        parts.next().map(accent_of).unwrap_or_default(),
                    );
                }
                Some(a.into_any_element())
            }
            "Stepper" => {
                let mut st = MacStepper::new(key.to_string());
                if prop_bool(node, "disabled", view, false) {
                    st = st.disabled(true);
                }
                if let Some(h) = handler(view, node, "increment", cx) {
                    st = st.on_increment(move |_e, w, cx| h(w, cx));
                }
                if let Some(h) = handler(view, node, "decrement", cx) {
                    st = st.on_decrement(move |_e, w, cx| h(w, cx));
                }
                Some(st.into_any_element())
            }
            "RatingIndicator" => {
                let mut r = MacRatingIndicator::new(
                    key.to_string(),
                    prop_num(node, "rating", view).unwrap_or(0.0).max(0.0) as usize,
                );
                if let Some(h) = fire_idx(view, node, "rate", cx) {
                    r = r.on_rate(move |v, w, cx| h(v, w, cx));
                }
                Some(r.into_any_element())
            }
            "PageControl" => {
                let mut p = MacPageControl::new(
                    key.to_string(),
                    prop_num(node, "pages", view).unwrap_or(1.0).max(1.0) as usize,
                )
                .current_page(prop_num(node, "page", view).unwrap_or(0.0).max(0.0) as usize);
                if let Some(h) = fire_idx(view, node, "change", cx) {
                    p = p.on_change(move |v, w, cx| h(v, w, cx));
                }
                Some(p.into_any_element())
            }
            "DigitEntry" => {
                let mut d = MacDigitEntry::new(
                    key.to_string(),
                    prop_num(node, "digits", view).unwrap_or(6.0).max(1.0) as usize,
                );
                let v = prop_str(node, "value", view);
                if !v.is_empty() {
                    d = d.value(v);
                }
                if let Some(h) = fire_str(view, node, "change", cx) {
                    d = d.on_change(move |v, w, cx| h(v.to_string(), w, cx));
                }
                if let Some(h) = fire_str(view, node, "complete", cx) {
                    d = d.on_complete(move |v, w, cx| h(v.to_string(), w, cx));
                }
                Some(d.into_any_element())
            }
            "ColorWell" => {
                let mut c =
                    MacColorWell::new(key.to_string(), hsla_of(&prop_str(node, "color", view)));
                if prop_bool(node, "disabled", view, false) {
                    c = c.disabled(true);
                }
                if let Some(h) = handler(view, node, "click", cx) {
                    c = c.on_click(move |_e, w, cx| h(w, cx));
                }
                Some(c.into_any_element())
            }
            "ImageView" => {
                let mut i = MacImageView::new(key.to_string());
                if let Some(c) = prop_str_opt(node, "caption", view) {
                    i = i.caption(c);
                }
                i = match prop_str(node, "mode", view).as_str() {
                    "AspectFill" | "Fill" => i.content_mode(MacImageContentMode::AspectFill),
                    _ => i,
                };
                let (w, h) = (
                    prop_num(node, "width", view).unwrap_or(200.0) as f32,
                    prop_num(node, "height", view).unwrap_or(150.0) as f32,
                );
                i = i.size(w, h);
                if let Some(r) = prop_num(node, "radius", view) {
                    i = i.corner_radius(r as f32);
                }
                Some(i.into_any_element())
            }
            "ImageWell" => {
                let mut i = MacImageWell::new(key.to_string());
                if let Some(c) = prop_str_opt(node, "caption", view) {
                    i = i.caption(c);
                }
                if let Some(h) = handler(view, node, "click", cx) {
                    i = i.on_click(move |_e, w, cx| h(w, cx));
                }
                Some(i.into_any_element())
            }
            "MaterialSurface" => {
                let thickness = match prop_str(node, "thickness", view).as_str() {
                    "UltraThin" => MacMaterialThickness::UltraThin,
                    "Thin" => MacMaterialThickness::Thin,
                    "Thick" => MacMaterialThickness::Thick,
                    "UltraThick" => MacMaterialThickness::UltraThick,
                    _ => MacMaterialThickness::Regular,
                };
                let child = self
                    .child_views(node, key, cx)
                    .into_iter()
                    .next()
                    .map(|v| v.into_any_element());
                let mut m =
                    MacMaterialSurface::new(key.to_string()).material(MacMaterial::new(thickness));
                if let Some(c) = child {
                    m = m.child(c);
                }
                Some(m.into_any_element())
            }
            "Panel" => {
                let mut p = MacPanel::new(key.to_string(), prop_str(node, "title", view));
                if let Some(w) = prop_num(node, "width", view) {
                    p = p.width(w as f32);
                }
                if let Some(c) = self.child_views(node, key, cx).into_iter().next() {
                    p = p.child(c);
                }
                if let Some(h) = handler(view, node, "close", cx) {
                    p = p.on_close(move |_e, w, cx| h(w, cx));
                }
                Some(p.into_any_element())
            }
            "Disclosure" => {
                let mut d = MacDisclosure::new(key.to_string(), prop_str(node, "title", view));
                if let Some(c) = self.child_views(node, key, cx).into_iter().next() {
                    d = d.child(c);
                }
                if let Some(h) = fire_str(view, node, "toggle", cx) {
                    d = d.on_toggle(move |v, w, cx| h(v.to_string(), w, cx));
                }
                Some(d.into_any_element())
            }
            "StatusBar" => {
                let mut sb = MacStatusBar::new(key.to_string());
                for (i, child) in node.children.iter().enumerate() {
                    let v = self
                        .child_view(child, &format!("{key}/{i}"), cx)
                        .into_any_element();
                    sb = match prop_str(child, "slot", view).as_str() {
                        "center" => sb.center(v),
                        "trailing" => sb.trailing(v),
                        _ => sb.leading(v),
                    };
                }
                Some(sb.into_any_element())
            }
            "Window" => {
                let mut w = MacWindow::new(key.to_string(), prop_str(node, "title", view));
                if let Some(s) = prop_str_opt(node, "subtitle", view) {
                    w = w.subtitle(s);
                }
                let (wd, ht) = (
                    prop_num(node, "width", view).unwrap_or(480.0) as f32,
                    prop_num(node, "height", view).unwrap_or(320.0) as f32,
                );
                w = w.size(wd, ht);
                if let Some(c) = self.child_views(node, key, cx).into_iter().next() {
                    w = w.child(c);
                }
                if let Some(h) = handler(view, node, "close", cx) {
                    w = w.on_close(move |_e, w, cx| h(w, cx));
                }
                Some(w.into_any_element())
            }
            "Dialog" => {
                let mut d = MacDialog::new(key.to_string(), prop_str(node, "title", view));
                match prop_str(node, "variant", view).as_str() {
                    "warning" => d = d.warning(),
                    "critical" => d = d.critical(),
                    _ => {}
                }
                if let Some(m) = prop_str_opt(node, "message", view) {
                    d = d.message(m);
                }
                if let Some(l) = prop_str_opt(node, "primary", view) {
                    d = d.primary_label(l);
                }
                if let Some(l) = prop_str_opt(node, "secondary", view) {
                    d = d.secondary_label(Some(l.into()));
                }
                if prop_bool(node, "destructive", view, false) {
                    d = d.destructive(true);
                }
                if let Some(c) = self.child_views(node, key, cx).into_iter().next() {
                    d = d.child(c);
                }
                if let Some(h) = handler(view, node, "primary", cx) {
                    d = d.on_primary(move |_e, w, cx| h(w, cx));
                }
                if let Some(h) = handler(view, node, "secondary", cx) {
                    d = d.on_secondary(move |_e, w, cx| h(w, cx));
                }
                Some(d.into_any_element())
            }
            "Popover" => {
                let mut p = MacPopover::new(key.to_string());
                if let Some(t) = prop_str_opt(node, "title", view) {
                    p = p.title(t);
                }
                if let Some(w) = prop_num(node, "width", view) {
                    p = p.width(w as f32);
                }
                if let Some(c) = self.child_views(node, key, cx).into_iter().next() {
                    p = p.child(c);
                }
                if let Some(h) = handler(view, node, "close", cx) {
                    p = p.on_close(move |_e, w, cx| h(w, cx));
                }
                Some(p.into_any_element())
            }
            "TextView" => {
                let mut t = MacTextView::new(key.to_string());
                let text = prop_str(node, "text", view);
                if !text.is_empty() {
                    t = t.text(text);
                }
                if let Some(p) = prop_str_opt(node, "placeholder", view) {
                    t = t.placeholder(p);
                }
                if prop_bool(node, "line_numbers", view, false) {
                    t = t.show_line_numbers(true);
                }
                let (w, h) = (
                    prop_num(node, "width", view).unwrap_or(400.0) as f32,
                    prop_num(node, "height", view).unwrap_or(200.0) as f32,
                );
                t = t.size(w, h);
                Some(t.into_any_element())
            }
            "Radio" => {
                let mut r = MacRadio::new(key.to_string())
                    .selected(prop_bool(node, "selected", view, false))
                    .disabled(prop_bool(node, "disabled", view, false));
                if let Some(l) = prop_str_opt(node, "label", view) {
                    r = r.label(l);
                }
                if let Some(d) = prop_str_opt(node, "description", view) {
                    r = r.description(d);
                }
                if let Some(h) = handler(view, node, "click", cx) {
                    r = r.on_click(move |w, cx| h(w, cx));
                }
                Some(r.into_any_element())
            }
            "ComboBox" => {
                let mut c = MacComboBox::new(key.to_string());
                let v = prop_str(node, "value", view);
                if !v.is_empty() {
                    c = c.value(v);
                }
                if let Some(p) = prop_str_opt(node, "placeholder", view) {
                    c = c.placeholder(p);
                }
                for o in prop_list(node, "options") {
                    c = c.option(o);
                }
                if let Some(h) = change_str(view, node, "change", cx) {
                    c = c.on_change(move |v, w, cx| h(v.to_string(), w, cx));
                }
                Some(c.into_any_element())
            }
            "DatePicker" => {
                let mut d = MacDatePicker::new(key.to_string());
                d = match prop_str(node, "style", view).as_str() {
                    "Graphical" => d.picker_style(MacDatePickerStyle::Graphical),
                    "Stepper" => d.picker_style(MacDatePickerStyle::Stepper),
                    _ => d,
                };
                if prop_bool(node, "time", view, false) {
                    d = d.show_time(true);
                }
                if let Some(h) = fire_str(view, node, "change", cx) {
                    d = d.on_change(move |y, m, day, w, cx| {
                        h(format!("{y:04}-{m:02}-{day:02}"), w, cx)
                    });
                }
                Some(d.into_any_element())
            }
            "ColumnView" => {
                let mut v = MacColumnView::new(key.to_string());
                if let Some(w) = prop_num(node, "column_width", view) {
                    v = v.column_width(w as f32);
                }
                for col in prop_list(node, "columns") {
                    let entries: Vec<MacColumnEntry> = col
                        .split(';')
                        .filter(|e| !e.is_empty())
                        .map(|e| {
                            let mut p = e.split('|');
                            MacColumnEntry {
                                title: p.next().unwrap_or_default().into(),
                                is_folder: p.next() == Some("folder"),
                                is_selected: p.next() == Some("sel"),
                            }
                        })
                        .collect();
                    v = v.column(entries);
                }
                Some(v.into_any_element())
            }
            "Toolbar" => {
                let mut t = MacToolbar::new(key.to_string());
                if let Some(title) = prop_str_opt(node, "title", view) {
                    t = t.title(title);
                }
                for (i, child) in node.children.iter().enumerate() {
                    let v = self
                        .child_view(child, &format!("{key}/{i}"), cx)
                        .into_any_element();
                    t = match prop_str(child, "slot", view).as_str() {
                        "center" => t.center(v),
                        "trailing" => t.trailing(v),
                        _ => t.leading(v),
                    };
                }
                Some(t.into_any_element())
            }
            "TabBar" => {
                let mut t = MacTabBar::new(key.to_string());
                for item in prop_list(node, "items") {
                    let mut parts = item.splitn(3, '|');
                    let mut tab = MacTabItem::new(parts.next().unwrap_or_default().to_string());
                    if let Some(b) = parts.next().and_then(|b| b.parse::<usize>().ok()) {
                        tab = tab.badge(b);
                    }
                    t = t.tab_item(tab);
                }
                for child in &node.children {
                    let mut tab = MacTabItem::new(prop_str(child, "title", view));
                    if let Some(i) = icon_named(&prop_str(child, "icon", view)) {
                        tab = MacTabItem::with_icon(prop_str(child, "title", view), i);
                    }
                    if let Some(b) = prop_num(child, "badge", view) {
                        tab = tab.badge(b as usize);
                    }
                    t = t.tab_item(tab);
                }
                if let Some(s) = prop_num(node, "selected", view) {
                    t = t.selected_index(s as usize);
                }
                if prop_bool(node, "disabled", view, false) {
                    t = t.disabled(true);
                }
                if let Some(h) = fire_idx(view, node, "select", cx) {
                    t = t.on_select(move |v, w, cx| h(v, w, cx));
                }
                Some(t.into_any_element())
            }
            "SegmentedControl" => {
                let mut sc = MacSegmentedControl::new(key.to_string());
                for (i, item) in prop_list(node, "items").into_iter().enumerate() {
                    let mut parts = item.splitn(2, '|');
                    let mut seg = MacSegment::new(format!("{key}/seg{i}"))
                        .label(parts.next().unwrap_or_default());
                    if let Some(icon) = parts.next().and_then(icon_named) {
                        seg = seg.icon(icon);
                    }
                    sc = sc.segment(seg);
                }
                if let Some(s) = prop_num(node, "selected", view) {
                    sc = sc.selected_index(s as usize);
                }
                if prop_bool(node, "disabled", view, false) {
                    sc = sc.disabled(true);
                }
                if let Some(h) = fire_idx(view, node, "change", cx) {
                    sc = sc.on_change(move |v, w, cx| h(*v, w, cx));
                }
                Some(sc.into_any_element())
            }
            "Sidebar" => {
                let mut sb = MacSidebar::new(key.to_string());
                let section_of = |child: &Node| {
                    let mut sec = MacSidebarSection::new();
                    if let Some(t) = prop_str_opt(child, "title", view) {
                        sec = sec.title(t);
                    }
                    for item in prop_list(child, "items") {
                        let mut parts = item.splitn(3, '|');
                        let mut si = MacSidebarItem::new(
                            format!("{}/item", key),
                            parts.next().unwrap_or_default().to_string(),
                        );
                        if let Some(i) = parts.next().and_then(icon_named) {
                            si = si.icon(i);
                        }
                        if let Some(b) = parts.next() {
                            si = si.badge(b.to_string());
                        }
                        sec = sec.item(si);
                    }
                    for (j, item) in child.children.iter().enumerate() {
                        let mut si = MacSidebarItem::new(
                            format!("{key}/item{j}"),
                            prop_str(item, "title", view),
                        );
                        if let Some(i) = icon_named(&prop_str(item, "icon", view)) {
                            si = si.icon(i);
                        }
                        if let Some(b) = prop_str_opt(item, "badge", view) {
                            si = si.badge(b);
                        }
                        if prop_bool(item, "selected", view, false) {
                            si = si.selected(true);
                        }
                        sec = sec.item(si);
                    }
                    sec
                };
                if node.children.is_empty() {
                    let mut sec = MacSidebarSection::new();
                    for item in prop_list(node, "items") {
                        let mut parts = item.splitn(3, '|');
                        let mut si = MacSidebarItem::new(
                            format!("{key}/item"),
                            parts.next().unwrap_or_default().to_string(),
                        );
                        if let Some(i) = parts.next().and_then(icon_named) {
                            si = si.icon(i);
                        }
                        if let Some(b) = parts.next() {
                            si = si.badge(b.to_string());
                        }
                        sec = sec.item(si);
                    }
                    sb = sb.section(sec);
                } else {
                    for child in &node.children {
                        sb = sb.section(section_of(child));
                    }
                }
                Some(sb.into_any_element())
            }
            "TableView" => {
                let mut t = MacTableView::new(key.to_string());
                for col in prop_list(node, "columns") {
                    let mut parts = col.splitn(2, '|');
                    t = t.column(
                        parts.next().unwrap_or_default().to_string(),
                        parts
                            .next()
                            .and_then(|w| w.parse::<f32>().ok())
                            .unwrap_or(120.0),
                    );
                }
                for row in prop_list(node, "rows") {
                    let cells: Vec<String> = row.split('|').map(str::to_string).collect();
                    t = t.row(cells);
                }
                if let Some(s) = prop_num(node, "selected", view) {
                    t = t.selected_row(Some(s as usize));
                }
                if let Some(h) = fire_idx(view, node, "select", cx) {
                    t = t.on_select_row(move |v, w, cx| h(v, w, cx));
                }
                Some(t.into_any_element())
            }
            "OutlineView" => {
                let mut o = MacOutlineView::new(key.to_string());
                let items: Vec<MacOutlineItem> = prop_list(node, "items")
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        let mut parts = item.splitn(4, '|');
                        let mut oi = MacOutlineItem::new(
                            format!("{key}/item{i}"),
                            parts.next().unwrap_or_default().to_string(),
                            parts
                                .next()
                                .and_then(|l| l.parse::<usize>().ok())
                                .unwrap_or(0),
                        );
                        if let Some(icon) = parts.next().and_then(icon_named) {
                            oi = oi.icon(icon);
                        }
                        if parts.next() == Some("open") {
                            oi = oi.expanded(true).has_children(true);
                        }
                        oi
                    })
                    .collect();
                o = o.items(items);
                if let Some(h) = fire_str(view, node, "select", cx) {
                    o = o.on_select(move |v, w, cx| h(format!("{v:?}"), w, cx));
                }
                if let Some(h) = fire_str(view, node, "toggle", cx) {
                    o = o.on_toggle(move |v, w, cx| h(format!("{v:?}"), w, cx));
                }
                Some(o.into_any_element())
            }
            "SplitView" => {
                let mut sv = MacSplitView::new(key.to_string());
                if let Some(w) = prop_num(node, "leading_width", view) {
                    sv = sv.leading_width(w as f32);
                }
                let mut kids = self.child_views(node, key, cx).into_iter();
                if let Some(l) = kids.next() {
                    sv = sv.leading(l);
                }
                if let Some(t) = kids.next() {
                    sv = sv.trailing(t);
                }
                Some(sv.into_any_element())
            }
            "PathControl" => {
                let mut p = MacPathControl::new(key.to_string());
                let items: Vec<MacPathItem> = prop_list(node, "items")
                    .iter()
                    .map(|item| {
                        let mut parts = item.splitn(2, '|');
                        let title = parts.next().unwrap_or_default().to_string();
                        match parts.next().and_then(icon_named) {
                            Some(i) => MacPathItem::with_icon(title, i),
                            None => MacPathItem::new(title),
                        }
                    })
                    .collect();
                p = p.items(items);
                if prop_bool(node, "disabled", view, false) {
                    p = p.disabled(true);
                }
                if let Some(h) = fire_idx(view, node, "select", cx) {
                    p = p.on_select(move |v, w, cx| h(v, w, cx));
                }
                Some(p.into_any_element())
            }
            "PopupButton" => {
                let mut b = MacPopupButton::new(key.to_string());
                let items: Vec<MacMenuItem> = prop_list(node, "items")
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        let mut parts = item.splitn(3, '|');
                        let mut mi = MacMenuItem::new(
                            format!("{key}/item{i}"),
                            parts.next().unwrap_or_default().to_string(),
                        );
                        if let Some(i) = parts.next().and_then(icon_named) {
                            mi = mi.icon(i);
                        }
                        if let Some(s) = parts.next() {
                            mi = mi.shortcut(s);
                        }
                        mi
                    })
                    .collect();
                b = b.items(items);
                if let Some(s) = prop_num(node, "selected", view) {
                    b = b.selected_index(s as usize);
                }
                if prop_bool(node, "disabled", view, false) {
                    b = b.disabled(true);
                }
                if let Some(h) = fire_idx(view, node, "change", cx) {
                    b = b.on_change(move |v, w, cx| h(*v, w, cx));
                }
                Some(b.into_any_element())
            }
            "PullDownButton" => {
                let mut b = MacPullDownButton::new(key.to_string(), prop_str(node, "title", view));
                for (i, item) in prop_list(node, "items").into_iter().enumerate() {
                    let mut parts = item.splitn(3, '|');
                    let mut mi = MacMenuItem::new(
                        format!("{key}/item{i}"),
                        parts.next().unwrap_or_default().to_string(),
                    );
                    if let Some(i) = parts.next().and_then(icon_named) {
                        mi = mi.icon(i);
                    }
                    if let Some(s) = parts.next() {
                        mi = mi.shortcut(s);
                    }
                    b = b.item(mi);
                }
                if prop_bool(node, "disabled", view, false) {
                    b = b.disabled(true);
                }
                if let Some(h) = fire_idx(view, node, "select", cx) {
                    b = b.on_select(move |v, w, cx| h(*v, w, cx));
                }
                Some(b.into_any_element())
            }
            "ContextMenu" => {
                let mut m = MacContextMenu::new(key.to_string());
                for item in prop_list(node, "items") {
                    let mut parts = item.splitn(2, '|');
                    m = match parts.next().and_then(icon_named) {
                        Some(i) => m.item_with_icon(parts.next().unwrap_or_default(), i),
                        None => m.item(item),
                    };
                }
                if let Some(w) = prop_num(node, "width", view) {
                    m = m.width(w as f32);
                }
                if let Some(h) = fire_idx(view, node, "select", cx) {
                    m = m.on_select(move |v, w, cx| h(v, w, cx));
                }
                Some(m.into_any_element())
            }
            "MenuBar" => {
                let mut m = MacMenuBar::new(key.to_string(), prop_str(node, "app", view));
                let menus = prop_list(node, "menus");
                if !menus.is_empty() {
                    m = m.menus(menus);
                }
                if let Some(h) = fire_idx(view, node, "select", cx) {
                    m = m.on_select_menu(move |v, w, cx| h(v, w, cx));
                }
                Some(m.into_any_element())
            }
            "SettingsGroup" => {
                let mut g = MacSettingsGroup::new(key.to_string());
                if let Some(t) = prop_str_opt(node, "title", view) {
                    g = g.title(t);
                }
                for (i, child) in node.children.iter().enumerate() {
                    let mut r = MacSettingsRow::new(
                        format!("{key}/row{i}"),
                        prop_str(child, "title", view),
                    );
                    if let Some(s) = prop_str_opt(child, "subtitle", view) {
                        r = r.subtitle(s);
                    }
                    if let Some(icon) = icon_named(&prop_str(child, "icon", view)) {
                        r = r.icon(icon);
                    }
                    if let Some(c) = prop_str_opt(child, "color", view) {
                        r = r.icon_color(hsla_of(&c));
                    }
                    if let Some(v) = self.child_view_opt(child, &format!("{key}/row{i}/ctl"), cx) {
                        r = r.control(v);
                    }
                    g = g.row(r);
                }
                Some(g.into_any_element())
            }
            "TextField" | "SecureField" | "SearchField" => {
                let state = self.input_state(key, node, view, window, cx);
                let el = match node.element.name.as_str() {
                    "SecureField" => {
                        let mut f = MacSecureField::new(key.to_string(), state);
                        if let Some(p) = prop_str_opt(node, "placeholder", view) {
                            f = f.placeholder(p);
                        }
                        f.into_any_element()
                    }
                    "SearchField" => {
                        let mut f = MacSearchField::new(key.to_string(), state);
                        if let Some(p) = prop_str_opt(node, "hint", view) {
                            f = f.shortcut_hint(p);
                        }
                        f.into_any_element()
                    }
                    _ => {
                        let mut f = MacTextField::new(key.to_string(), state);
                        if let Some(p) = prop_str_opt(node, "placeholder", view) {
                            f = f.placeholder(p);
                        }
                        f.into_any_element()
                    }
                };
                Some(el)
            }
            "TokenField" => {
                let mut t = MacTokenField::new(key.to_string());
                let toks: Vec<MacToken> = prop_list(node, "tokens")
                    .iter()
                    .enumerate()
                    .map(|(i, token)| {
                        let mut parts = token.splitn(2, '|');
                        let mut tok = MacToken::new(
                            format!("{key}/tok{i}"),
                            parts.next().unwrap_or_default().to_string(),
                        );
                        if let Some(c) = parts.next() {
                            tok = tok.color(accent_of(c));
                        }
                        tok
                    })
                    .collect();
                t = t.tokens(toks);
                if let Some(p) = prop_str_opt(node, "placeholder", view) {
                    t = t.placeholder(p);
                }
                if prop_bool(node, "disabled", view, false) {
                    t = t.disabled(true);
                }
                if let Some(h) = fire_idx(view, node, "remove", cx) {
                    t = t.on_remove(move |v, w, cx| h(v, w, cx));
                }
                Some(t.into_any_element())
            }
            _ => None,
        }
    }
}

impl Hig {
    fn child_view(&self, child: &Node, key: &str, cx: &mut Context<ForgeView>) -> AnyView {
        if let Some(v) = VIEWS.with(|views| views.borrow().get(key).cloned()) {
            return v;
        }
        let node = child.clone();
        let k = key.to_owned();
        let weak = cx.entity().downgrade();
        let v: AnyView = cx
            .new(|_| HigProxy {
                view: weak,
                node,
                key: k,
            })
            .into();
        VIEWS.with(|views| views.borrow_mut().insert(key.to_owned(), v.clone()));
        v
    }

    fn child_views(&self, node: &Node, key: &str, cx: &mut Context<ForgeView>) -> Vec<AnyView> {
        node.children
            .iter()
            .enumerate()
            .map(|(i, c)| self.child_view(c, &format!("{key}/{i}"), cx))
            .collect()
    }

    /// First child element of `child`, if it has any — for `.control()` slots.
    fn child_view_opt(
        &self,
        child: &Node,
        key: &str,
        cx: &mut Context<ForgeView>,
    ) -> Option<AnyElement> {
        child
            .children
            .first()
            .map(|c| self.child_view(c, key, cx).into_any_element())
    }

    /// The `InputState` entity a Mac text field binds to, created once per key.
    fn input_state(
        &self,
        key: &str,
        node: &Node,
        view: &ForgeView,
        window: &mut Window,
        cx: &mut Context<ForgeView>,
    ) -> Entity<InputState> {
        if let Some(e) = INPUTS.with(|i| i.borrow().get(key).cloned()) {
            return e;
        }
        let value = prop_str(node, "value", view);
        let state = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            if !value.is_empty() {
                s = s.default_value(value);
            }
            s
        });
        INPUTS.with(|i| i.borrow_mut().insert(key.to_owned(), state.clone()));
        state
    }
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

type Handler = Rc<dyn Fn(&mut Window, &mut App)>;
type StrHandler = Rc<dyn Fn(String, &mut Window, &mut App)>;
type IdxHandler = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// A no-payload `on <event>:` handler, or `None` when the node has none.
fn handler(
    view: &ForgeView,
    node: &Node,
    event: &str,
    cx: &mut Context<ForgeView>,
) -> Option<Handler> {
    let calls = calls_for(view, node, event);
    if calls.is_empty() {
        return None;
    }
    let weak = cx.entity().downgrade();
    Some(Rc::new(move |_: &mut Window, cx: &mut App| {
        for (name, args) in &calls {
            let weak = weak.clone();
            let name = name.clone();
            let args = args.clone();
            weak.update(cx, |view, cx| view.fire_call(&name, args, cx))
                .ok();
        }
    }))
}

/// An `<event>` handler that fires calls with a string payload.
fn fire_str(
    view: &ForgeView,
    node: &Node,
    event: &str,
    cx: &mut Context<ForgeView>,
) -> Option<StrHandler> {
    let calls = calls_for(view, node, event);
    if calls.is_empty() {
        return None;
    }
    let weak = cx.entity().downgrade();
    Some(Rc::new(move |v: String, _: &mut Window, cx: &mut App| {
        for (name, args) in &calls {
            let weak = weak.clone();
            let name = name.clone();
            let mut args = args.clone();
            args.push(v.clone());
            weak.update(cx, |view, cx| view.fire_call(&name, args, cx))
                .ok();
        }
    }))
}

/// An `<event>` handler that fires calls with an index payload.
fn fire_idx(
    view: &ForgeView,
    node: &Node,
    event: &str,
    cx: &mut Context<ForgeView>,
) -> Option<IdxHandler> {
    let calls = calls_for(view, node, event);
    if calls.is_empty() {
        return None;
    }
    let weak = cx.entity().downgrade();
    Some(Rc::new(move |v: usize, _: &mut Window, cx: &mut App| {
        for (name, args) in &calls {
            let weak = weak.clone();
            let name = name.clone();
            let mut args = args.clone();
            args.push(v.to_string());
            weak.update(cx, |view, cx| view.fire_call(&name, args, cx))
                .ok();
        }
    }))
}

/// The state path a `{{…}}`-bound prop points at.
fn binding_of(node: &Node, name: &str) -> Option<Path> {
    match prop(node, name) {
        Some(Value::Binding { path, .. }) => Some(path.clone()),
        _ => None,
    }
}

/// Update the entity through a weak handle without panicking mid-render.
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

/// A `change` handler that writes `{{value}}` bindings and fires calls.
fn change_str(
    view: &ForgeView,
    node: &Node,
    event: &str,
    cx: &mut Context<ForgeView>,
) -> Option<StrHandler> {
    let path = binding_of(node, "value");
    let calls = calls_for(view, node, event);
    if path.is_none() && calls.is_empty() {
        return None;
    }
    let weak = cx.entity().downgrade();
    Some(Rc::new(move |v: String, _: &mut Window, cx: &mut App| {
        if let Some(path) = path.clone() {
            let value = Json::String(v.clone());
            try_update(&weak, cx, |view, cx| view.update_state(&path, value, cx));
        }
        for (name, args) in &calls {
            let weak = weak.clone();
            let name = name.clone();
            let mut args = args.clone();
            args.push(v.clone());
            try_update(&weak, cx, |view, cx| view.fire_call(&name, args, cx));
        }
    }))
}

/// `list<string>` prop as owned strings.
fn prop_list(node: &Node, name: &str) -> Vec<String> {
    match prop(node, name) {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|v| match v {
                Value::String(s) | Value::Enum(s) => Some(s.clone()),
                _ => None,
            })
            .collect(),
        Some(Value::String(s)) | Some(Value::Enum(s)) => s
            .split(';')
            .filter(|x| !x.is_empty())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

/// Lucide-style kebab name → `IconName`, matched via asset paths.
fn icon_named(name: &str) -> Option<IconName> {
    if name.is_empty() {
        return None;
    }
    let want = format!("icons/{name}.svg");
    IconName::ALL.iter().copied().find(|i| i.path() == want)
}

/// `macOS accent name → MacAccentColor`.
fn accent_of(name: &str) -> MacAccentColor {
    match name {
        "Purple" => MacAccentColor::Purple,
        "Pink" => MacAccentColor::Pink,
        "Red" => MacAccentColor::Red,
        "Orange" => MacAccentColor::Orange,
        "Yellow" => MacAccentColor::Yellow,
        "Green" => MacAccentColor::Green,
        "Graphite" => MacAccentColor::Graphite,
        _ => MacAccentColor::Blue,
    }
}

/// `#rrggbb`/`#rrggbbaa` → `Hsla`.
fn hsla_of(s: &str) -> Hsla {
    let hex = s.trim_start_matches('#');
    let parse = |i: usize| -> f32 {
        hex.get(i..i + 2)
            .and_then(|h| u8::from_str_radix(h, 16).ok())
            .unwrap_or(0) as f32
            / 255.0
    };
    let (r, g, b) = (parse(0), parse(2), parse(4));
    // sRGB → HSL for the gpui Hsla fields.
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let (h, s) = if max == min {
        (0.0, 0.0)
    } else {
        let d = max - min;
        let s = if l > 0.5 {
            d / (2.0 - max - min)
        } else {
            d / (max + min)
        };
        let h = if max == r {
            (g - b) / d + if g < b { 6.0 } else { 0.0 }
        } else if max == g {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        };
        (h / 6.0, s)
    };
    Hsla { h, s, l, a: 1.0 }
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
