//! `ForgeView` for the guise base — crates.io `gpui 0.2.2` + guise (#96).
//!
//! Same public contract as the gpui-kit twin: hosts drive `set_document`,
//! `update_state`, `fire_call`, `preview`, pick/place/dnd flags and sinks
//! identically. Element rendering maps the guise catalog onto guise widgets
//! where their API is stable and gpui primitives elsewhere — `forge.*`
//! intrinsics and two-way input widgets are stubs until the adapter port.

use crate::render::{get_path, set_path};
use forge_spec::catalog::Catalog;
use forge_spec::{Document, Entries, Node, Path, Segment, Unit, Value};
use gpui::prelude::*;
use gpui::*;
use guise::prelude::{Button as GuiseButton, Title as GuiseTitle, Variant as GuiseVariant};
use std::collections::HashMap;
use std::rc::Rc;

/// Called with `hook(args)` when an event fires and no hook is registered for its name.
pub type HookSink = Rc<dyn Fn(&str, &mut ForgeView, &mut Context<ForgeView>)>;
/// Called with the whole view state after a two-way binding writes to it.
pub type StateSink = Rc<dyn Fn(&Json, &mut ForgeView, &mut Context<ForgeView>)>;
/// Rust logic behind `on <event>: name(args)` — the hook registry.
pub type HookHandler = Rc<dyn Fn(&[String], &mut ForgeView, &mut Context<ForgeView>)>;

use serde_json::Value as Json;

gpui::actions!(gpui_forge_runtime, [CancelPick]);

/// A drag payload: `Component` comes from palette items (`insert_component`
/// handlers), `Node` from existing document nodes (reparent drags).
pub enum ForgeDrag {
    Component(String),
    Node(String),
}

/// Panics caught inside `update` calls where the view itself is unreachable — a
/// double-leased entity can't be updated to report its own error. Drained by
/// `take_errors` on the next paint.
static PENDING_ERRORS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// `weak.update` wrapped in `catch_unwind` — a reentrant entity update panics
/// inside gpui, and this converts it into a reported error instead of a crash.
fn try_update<F, R>(weak: &WeakEntity<ForgeView>, cx: &mut App, what: &str, f: F) -> Result<R, ()>
where
    F: FnOnce(&mut ForgeView, &mut Context<ForgeView>) -> R,
{
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        weak.update(cx, f).map_err(|_| ())
    }));
    match result {
        Ok(result) => result,
        Err(payload) => {
            let detail = payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown panic".to_owned());
            let weak = weak.clone();
            let what = what.to_owned();
            cx.defer(move |cx| {
                weak.update(cx, |view, cx| {
                    view.report(format!("{what} failed: {detail}"));
                    cx.notify();
                })
                .ok();
            });
            Err(())
        }
    }
}

/// An external element library plugged into the runtime — alias + catalog + a
/// render shim for `alias.Name` elements (the adapter contract).
pub trait ForgeLibrary {
    /// The `use:` alias this library handles (e.g. "ely" for `ely.VideoPlayer`).
    fn alias(&self) -> &'static str;
    /// The library's catalog manifest, merged into the view's catalogs on
    /// registration so validation/props resolve.
    fn catalog(&self) -> Option<&Catalog> {
        None
    }
    /// Renders `alias.<element>`. `None` falls through to the unknown-element
    /// placeholder.
    fn render(
        &self,
        node: &Node,
        key: &str,
        view: &ForgeView,
        window: &mut Window,
        cx: &mut Context<ForgeView>,
    ) -> Option<AnyElement>;
}

// A fixed dark palette — guise's Theme installs a global, but primitive divs
// need literal colors. These match the zinc/blue scheme Studio uses.
const BG: u32 = 0x18181b;
const BORDER: u32 = 0x3f3f46;
const FG: u32 = 0xf4f4f5;
const MUTED: u32 = 0xa1a1aa;

pub struct ForgeView {
    doc: Option<Document>,
    pub catalogs: Vec<Catalog>,
    state: Json,
    previews: HashMap<String, Entity<ForgeView>>,
    pub libraries: Vec<Rc<dyn ForgeLibrary>>,
    hooks: HashMap<String, HookHandler>,
    on_hook: Option<HookSink>,
    on_state: Option<StateSink>,
    /// Render-time problems surfaced for the host — `take_errors` drains them.
    errors: Vec<String>,
    /// The directory of the document this view renders — `src` paths resolve
    /// relative to it.
    base_dir: Option<std::path::PathBuf>,
    /// Element-picker mode (staged — the overlay lands with the adapter port).
    pick: bool,
    /// Click-to-place mode.
    placing: bool,
    /// Drag & drop enabled on rendered nodes.
    dnd: bool,
    /// Shown while there is no document (e.g. inside `forge.Preview`).
    empty_text: Option<String>,
    /// Per-key stateful widget views — a doc's nodes re-render but the
    /// widget state persists between paints (same contract as the kit base).
    views: HashMap<String, AnyView>,
    subscriptions: Vec<Subscription>,
    /// guise requires `Theme::init` before any component paints — done lazily
    /// on first render, the first place a `&mut App` exists.
    theme_ready: bool,
}

impl ForgeView {
    pub fn new(catalogs: Vec<Catalog>) -> Self {
        Self {
            doc: None,
            catalogs,
            state: Json::Object(Default::default()),
            previews: HashMap::new(),
            libraries: Vec::new(),
            hooks: HashMap::new(),
            on_hook: None,
            on_state: None,
            errors: Vec::new(),
            base_dir: None,
            pick: false,
            placing: false,
            dnd: false,
            empty_text: None,
            views: HashMap::new(),
            subscriptions: Vec::new(),
            theme_ready: false,
        }
    }

    pub fn set_document(&mut self, doc: Document, cx: &mut Context<Self>) {
        self.doc = Some(doc);
        cx.notify();
    }

    pub fn set_state(&mut self, state: Json, cx: &mut Context<Self>) {
        self.state = state;
        cx.notify();
    }

    /// The view state that bindings read from and write to.
    pub fn state(&self) -> &Json {
        &self.state
    }

    /// The parsed document this view renders, if any.
    pub fn document(&self) -> Option<&Document> {
        self.doc.as_ref()
    }

    /// Drops a cached input so the next render rebuilds it from bound state —
    /// a no-op until input widgets land in the adapter port.
    pub fn reset_inputs(&mut self, _key: &str) {}

    /// Writes `value` at `path` in the view state.
    pub fn update_state(&mut self, path: &Path, value: Json, cx: &mut Context<Self>) {
        self.write(path, value, cx);
    }

    /// Text displayed while the view has no document (used by `forge.Preview`).
    pub fn set_empty_text(&mut self, text: Option<String>) {
        self.empty_text = text;
    }

    /// Registers `handler` for `on <event>: name(args)` bindings. Handlers may
    /// edit this view freely. Unregistered names report to the hook sink.
    pub fn set_hook(&mut self, name: impl Into<String>, handler: HookHandler) {
        self.hooks.insert(name.into(), handler);
    }

    /// The `forge.Preview` host for `id`, creating the nested view on first use.
    /// The child shares the parent's registered libraries so `alias.Name`
    /// elements render inside previews too.
    pub fn preview(&mut self, id: &str, cx: &mut Context<Self>) -> Entity<ForgeView> {
        let catalogs = self.catalogs.clone();
        let libraries = self.libraries.clone();
        self.previews
            .entry(id.to_owned())
            .or_insert_with(|| {
                cx.new(|_| {
                    let mut view = ForgeView::new(catalogs);
                    view.libraries = libraries;
                    view
                })
            })
            .clone()
    }

    /// Sets the document a `forge.Preview` renders.
    pub fn set_preview_document(&mut self, id: &str, doc: Document, cx: &mut Context<Self>) {
        self.preview(id, cx)
            .update(cx, |view, cx| view.set_document(doc, cx));
    }

    pub fn set_sinks(&mut self, on_hook: HookSink, on_state: StateSink) {
        self.on_hook = Some(on_hook);
        self.on_state = Some(on_state);
    }

    /// Inspector prop write-through — `doc.set_prop`.
    pub fn set_node_prop(&mut self, path: &str, name: &str, json: &Json, cx: &mut Context<Self>) {
        let value = if json.is_null() {
            None
        } else {
            Some(Value::String(
                json.as_str().unwrap_or(&json.to_string()).to_owned(),
            ))
        };
        if let Some(doc) = self.doc.as_mut() {
            doc.set_prop(path, name, value);
            cx.notify();
        }
    }

    /// `Style` tab write-through — `doc.set_style`.
    pub fn set_node_style(&mut self, path: &str, name: &str, json: &Json, cx: &mut Context<Self>) {
        let value = if json.is_null() {
            None
        } else {
            Some(Value::String(json.as_str().unwrap_or_default().to_owned()))
        };
        if let Some(doc) = self.doc.as_mut() {
            doc.set_style(path, name, value);
            cx.notify();
        }
    }

    /// Records a render-time problem — surfaced to the host next paint.
    pub fn report(&mut self, message: impl Into<String>) {
        self.errors.push(message.into());
    }

    /// Sets the directory the current document lives in — relative `src` paths
    /// resolve against it.
    pub fn set_base_dir(&mut self, dir: impl Into<std::path::PathBuf>) {
        self.base_dir = Some(dir.into());
    }

    /// Resolves `src` against `base_dir` when relative — adapters (forge-lib-*)
    /// use this so `src` props are document-relative, not cwd-relative.
    pub fn resolve_path(&self, src: &str) -> std::path::PathBuf {
        let path = std::path::Path::new(src);
        if path.is_relative()
            && let Some(base) = &self.base_dir
        {
            return base.join(path);
        }
        path.to_path_buf()
    }

    /// Element-picker mode — rendered nodes report `document.selected` (staged).
    pub fn set_pick(&mut self, pick: bool, cx: &mut Context<Self>) {
        self.pick = pick;
        self.placing = false;
        cx.notify();
    }

    /// Click-to-place mode — the click reports `studio.drop_target` and fires
    /// `place_component` (staged).
    pub fn set_placing(&mut self, placing: bool, cx: &mut Context<Self>) {
        self.pick = placing;
        self.placing = placing;
        cx.notify();
    }

    /// Enables drag & drop on rendered nodes (staged).
    pub fn set_dnd(&mut self, dnd: bool, cx: &mut Context<Self>) {
        self.dnd = dnd;
        cx.notify();
    }

    /// Fires a registered hook by name, bubbling to the host's sink when this
    /// view doesn't handle it — the shared path for events and drops. Public
    /// so hosts can trigger doc events programmatically (tests, IPC).
    pub fn fire_call(&mut self, name: &str, args: Vec<String>, cx: &mut Context<Self>) {
        if let Some(handler) = self.hooks.get(name).cloned() {
            handler(&args, self, cx);
        } else if let Some(sink) = self.on_hook.clone() {
            sink(&format!("{}({})", name, args.join(", ")), self, cx);
        }
    }

    /// Registers a namespaced-element library: its catalog is merged for
    /// validation/props and `alias.<name>` elements render through it.
    pub fn register_library(&mut self, lib: impl ForgeLibrary + 'static) {
        if let Some(cat) = lib.catalog().cloned() {
            self.catalogs.push(cat);
        }
        self.libraries.push(Rc::new(lib));
    }

    /// Drains accumulated errors — the host captures them to display or handle.
    pub fn take_errors(&mut self) -> Vec<String> {
        let mut errors = std::mem::take(&mut self.errors);
        errors.append(&mut PENDING_ERRORS.lock().unwrap_or_else(|e| e.into_inner()));
        errors
    }

    // ── state + value helpers ────────────────────────────────────────────────

    fn write(&mut self, path: &Path, value: Json, cx: &mut Context<Self>) {
        set_path(&mut self.state, path, value);
        if let Some(sink) = self.on_state.clone() {
            let state = self.state.clone();
            sink(&state, self, cx);
        }
        cx.notify();
    }

    fn lookup(&self, path: &Path) -> Option<&Json> {
        get_path(&self.state, path)
    }

    /// `{{ path }}` / `$state.x` text — binding values render as JSON scalars.
    fn json_text(&self, path: &Path) -> String {
        self.lookup(path)
            .map(|j| match j {
                Json::String(s) => s.clone(),
                other => other.to_string(),
            })
            .unwrap_or_default()
    }

    /// A spec `Value` as display text — bindings resolve through state.
    fn text_of(&self, value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Number { value, unit } => match unit {
                Some(Unit::Px) => format!("{value}px"),
                Some(Unit::Percent) => format!("{value}%"),
                Some(Unit::Fr) => format!("{value}fr"),
                Some(Unit::Rem) => format!("{value}rem"),
                None => format!("{value}"),
            },
            Value::Bool(b) => b.to_string(),
            Value::Binding { path, .. } => self.json_text(path),
            Value::Interpolated(segments) => segments
                .iter()
                .map(|seg| match seg {
                    Segment::Text(t) => t.clone(),
                    Segment::Binding(p) => self.json_text(p),
                })
                .collect(),
            _ => String::new(),
        }
    }

    fn prop_text(&self, node: &Node, name: &str) -> String {
        node.props
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| self.text_of(v))
            .unwrap_or_default()
    }

    /// The state path a prop binds to, if it's a `{{...}}` binding — used by
    /// two-way input widgets.
    fn binding_of(node: &Node, name: &str) -> Option<Path> {
        node.props
            .iter()
            .find(|(k, _)| k == name)
            .and_then(|(_, v)| match v {
                Value::Binding { path, .. } => Some(path.clone()),
                _ => None,
            })
    }

    fn prop_bool(&self, node: &Node, name: &str, fallback: bool) -> bool {
        match node.props.iter().find(|(k, _)| k == name).map(|(_, v)| v) {
            Some(Value::Bool(b)) => *b,
            Some(Value::Binding { path, .. }) => self
                .lookup(path)
                .and_then(Json::as_bool)
                .unwrap_or(fallback),
            _ => fallback,
        }
    }

    fn num(&self, node: &Node, name: &str, fallback: f64) -> f64 {
        match node.props.iter().find(|(k, _)| k == name).map(|(_, v)| v) {
            Some(Value::Number { value, .. }) => *value,
            Some(Value::Binding { path, .. }) => {
                self.lookup(path).and_then(Json::as_f64).unwrap_or(fallback)
            }
            _ => fallback,
        }
    }

    /// A `variant:` prop as a guise `Variant` — shared across Button, Badge,
    /// and Alert.
    fn variant_of(node: &Node, name: &str) -> Option<GuiseVariant> {
        match &node.props.iter().find(|(k, _)| k == name)?.1 {
            Value::String(s) => match s.as_str() {
                "Light" => Some(GuiseVariant::Light),
                "Outline" => Some(GuiseVariant::Outline),
                "Subtle" => Some(GuiseVariant::Subtle),
                "Transparent" => Some(GuiseVariant::Transparent),
                "White" => Some(GuiseVariant::White),
                "Default" => Some(GuiseVariant::Default),
                "Filled" => Some(GuiseVariant::Filled),
                _ => None,
            },
            _ => None,
        }
    }

    /// `size:` prop as a guise `Size` — shared across widgets.
    fn size_of(&self, node: &Node, name: &str) -> guise::Size {
        match self.prop_text(node, name).as_str() {
            "Xs" => guise::Size::Xs,
            "Sm" => guise::Size::Sm,
            "Lg" => guise::Size::Lg,
            "Xl" => guise::Size::Xl,
            _ => guise::Size::Md,
        }
    }

    /// `color:` prop as a guise `ColorName` — the Mantine palette names.
    fn color_of(&self, node: &Node, name: &str) -> Option<guise::ColorName> {
        use guise::ColorName as C;
        Some(match self.prop_text(node, name).as_str() {
            "Dark" => C::Dark,
            "Gray" => C::Gray,
            "Red" => C::Red,
            "Pink" => C::Pink,
            "Grape" => C::Grape,
            "Violet" => C::Violet,
            "Indigo" => C::Indigo,
            "Blue" => C::Blue,
            "Cyan" => C::Cyan,
            "Teal" => C::Teal,
            "Green" => C::Green,
            "Lime" => C::Lime,
            "Yellow" => C::Yellow,
            "Orange" => C::Orange,
            _ => return None,
        })
    }

    /// Comma-separated option list — shared by every widget that takes
    /// `IntoIterator<Item = impl Into<SharedString>>`.
    fn options_of(&self, node: &Node, name: &str) -> Vec<SharedString> {
        self.prop_text(node, name)
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| SharedString::from(s.to_owned()))
            .collect()
    }

    /// Comma-separated usize list — multi-select initial values.
    fn indices_of(&self, node: &Node, name: &str) -> Vec<usize> {
        self.prop_text(node, name)
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect()
    }

    /// Kebab- or PascalCase lucide name → `IconName` — `IconName::all` is a
    /// ~2k entry name table, scanned once per render.
    fn icon_of(s: &str) -> Option<guise::IconName> {
        let norm = |t: &str| t.replace(['-', '_'], "").to_lowercase();
        let want = norm(s);
        guise::IconName::all()
            .iter()
            .copied()
            .find(|n| norm(n.name()) == want)
    }

    /// `#RRGGBB` / `#RRGGBBAA` hex → `Hsla` — the ColorInput value prop.
    fn hsla_of(s: &str) -> Option<Hsla> {
        let h = s.trim().trim_start_matches('#');
        if h.len() != 6 && h.len() != 8 {
            return None;
        }
        let rgb = u32::from_str_radix(&h[..6], 16).ok()?;
        let a = if h.len() == 8 {
            u32::from_str_radix(&h[6..], 16).ok()? as f32 / 255.0
        } else {
            1.0
        };
        let mut c = Hsla::from(gpui::rgb(rgb));
        c.a = a;
        Some(c)
    }

    /// The shared `on click:` wiring — handler calls through `try_update`,
    /// same as the Button arm.
    fn click_handler(
        &self,
        node: &Node,
        cx: &mut Context<Self>,
    ) -> Option<impl Fn(&ClickEvent, &mut Window, &mut App) + 'static> {
        let calls = self.events_for(node, "click");
        if calls.is_empty() {
            return None;
        }
        let weak = cx.entity().downgrade();
        Some(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
            for (name, args) in &calls {
                let weak = weak.clone();
                let name = name.clone();
                let args = args.clone();
                try_update(&weak, cx, "click", |view, cx| {
                    view.fire_call(&name, args, cx)
                })
                .ok();
            }
        })
    }

    /// The shared `on change:` wiring for click-style toggles — fires the
    /// bindings and writes the flipped value back to a `{{...}}` bound prop.
    fn change_handler(
        &self,
        node: &Node,
        prop: &str,
        current: bool,
        cx: &mut Context<Self>,
    ) -> Option<impl Fn(&ClickEvent, &mut Window, &mut App) + 'static> {
        let calls = self.events_for(node, "change");
        let binding = Self::binding_of(node, prop);
        if calls.is_empty() && binding.is_none() {
            return None;
        }
        let weak = cx.entity().downgrade();
        Some(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
            for (name, args) in &calls {
                let weak = weak.clone();
                let name = name.clone();
                let args = args.clone();
                try_update(&weak, cx, "change", |view, cx| {
                    view.fire_call(&name, args, cx)
                })
                .ok();
            }
            if let Some(path) = &binding {
                try_update(&weak, cx, "change", |view, cx| {
                    view.write(path, Json::Bool(!current), cx)
                })
                .ok();
            }
        })
    }

    /// `on <event>` bindings for a node — `(handler, args)` per binding of
    /// `event`.
    fn events_for(&self, node: &Node, event: &str) -> Vec<(String, Vec<String>)> {
        node.events
            .iter()
            .filter(|e| e.event == event)
            .map(|e| {
                (
                    e.handler.name.clone(),
                    e.handler.args.iter().map(|a| self.text_of(a)).collect(),
                )
            })
            .collect()
    }

    // ── style entries ────────────────────────────────────────────────────────

    fn pixels_of(value: &Value) -> Option<f32> {
        match value {
            Value::Number { value, .. } => Some(*value as f32),
            Value::String(s) => s.trim_end_matches("px").parse().ok(),
            _ => None,
        }
    }

    fn style_px(entries: &Entries, name: &str) -> Option<f32> {
        entries
            .iter()
            .find(|(k, _)| k == name)
            .and_then(|(_, v)| Self::pixels_of(v))
    }

    /// Applies the style entries layout primitives understand: padding, gap,
    /// width/height, margin.
    fn apply_style<D: Styled>(d: D, entries: &Entries) -> D {
        let mut d = d;
        if let Some(v) = Self::style_px(entries, "padding") {
            d = d.p(px(v));
        }
        if let Some(v) = Self::style_px(entries, "gap") {
            d = d.gap(px(v));
        }
        if let Some(v) = Self::style_px(entries, "width") {
            d = d.w(px(v));
        }
        if let Some(v) = Self::style_px(entries, "height") {
            d = d.h(px(v));
        }
        d
    }

    // ── rendering ────────────────────────────────────────────────────────────

    /// Renders a document node — the pick-mode wrap lives here so every
    /// element reports `document.selected` on click, matching the kit base.
    fn render_node(
        &mut self,
        node: &Node,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let el = self.render_node_inner(node, key, window, cx);
        if !self.pick && !self.dnd {
            return el;
        }
        // Same contract as the kit base: a transparent overlay carries pick
        // clicks (select, or place → `studio.drop_target` + `place_component`)
        // and the drag/drop protocol (`studio.drop_target` → `drop_component` /
        // `move_node`).
        let path = key.to_owned();
        let weak = cx.entity().downgrade();
        let cancel_weak = cx.entity().downgrade();
        let drop_weak = cx.entity().downgrade();
        let label = node.element.name.clone();
        let mut overlay = div()
            .id(SharedString::from(format!("pick-{key}")))
            .absolute()
            .inset_0()
            .cursor_pointer();
        if self.pick {
            let pick_path = path.clone();
            overlay = overlay
                .hover(|s| {
                    s.bg(rgba(0x3b82f659))
                        .border_2()
                        .border_color(rgba(0x3b82f6ff))
                })
                .on_click(move |_, _, cx| {
                    try_update(&weak, cx, "pick", |p, cx| {
                        p.pick = false;
                        if p.placing {
                            p.placing = false;
                            p.write(
                                &forge_spec::value::parse_path("studio.drop_target").unwrap(),
                                Json::String(pick_path.clone()),
                                cx,
                            );
                            p.fire_call("place_component", vec![pick_path.clone()], cx);
                        } else {
                            p.write(
                                &forge_spec::value::parse_path("document.selected").unwrap(),
                                Json::String(pick_path.clone()),
                                cx,
                            );
                        }
                        cx.notify();
                    })
                    .ok();
                })
                .on_mouse_down(MouseButton::Right, move |_, _, cx| {
                    try_update(&cancel_weak, cx, "cancel-place", |p, cx| {
                        p.pick = false;
                        p.placing = false;
                        p.fire_call("cancel_place", vec![], cx);
                        cx.notify();
                    })
                    .ok();
                });
        }
        if self.dnd {
            let drag_id = path.clone();
            let drop_id = path.clone();
            overlay = overlay
                .on_drag(ForgeDrag::Node(drag_id), move |_, _, _, cx| {
                    cx.new(|_| DragGhost(SharedString::from(label.clone())))
                })
                .on_drop::<ForgeDrag>(move |drag, _, cx| {
                    try_update(&drop_weak, cx, "drop", |p, cx| {
                        p.write(
                            &forge_spec::value::parse_path("studio.drop_target").unwrap(),
                            Json::String(drop_id.clone()),
                            cx,
                        );
                        match drag {
                            ForgeDrag::Component(tag) => p.fire_call(
                                "drop_component",
                                vec![tag.clone(), drop_id.clone()],
                                cx,
                            ),
                            ForgeDrag::Node(src) => {
                                p.fire_call("move_node", vec![src.clone(), drop_id.clone()], cx)
                            }
                        }
                        cx.notify();
                    })
                    .ok();
                });
        }
        div().relative().child(el).child(overlay).into_any_element()
    }

    /// Renders a document node — dispatch on `(namespace, element)`.
    fn render_node_inner(
        &mut self,
        node: &Node,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if let Some(ns) = node.element.namespace.as_deref() {
            if ns == "forge" {
                // `forge.Preview` hosts a nested document — `set_preview_document`
                // fills `previews`. The other intrinsics stay stubs until the
                // Studio-parity port lands.
                if node.element.name == "Preview" {
                    let id = self.prop_text(node, "id");
                    let id = if id.is_empty() { "preview" } else { &id };
                    let empty = self.prop_text(node, "placeholder");
                    let view = self.preview(id, cx);
                    if !empty.is_empty() {
                        view.update(cx, |v, _| v.set_empty_text(Some(empty)));
                    }
                    return div().size_full().child(view).into_any_element();
                }
                return placeholder(format!("forge.{} — guise stub", node.element.name));
            }
            for lib in &self.libraries {
                if lib.alias() == ns
                    && let Some(el) = lib.render(node, key, self, window, cx)
                {
                    return el;
                }
            }
            return placeholder(format!(
                "{ns}.{} — no library registered",
                node.element.name
            ));
        }
        match node.element.name.as_str() {
            "VStack" => {
                let mut col = div().flex().flex_col();
                col = Self::apply_style(col, &node.style);
                for (i, child) in node.children.iter().enumerate() {
                    let k = format!("{key}/{i}");
                    col = col.child(self.render_node(child, &k, window, cx));
                }
                col.into_any_element()
            }
            "HStack" => {
                let mut row = div().flex().flex_row();
                row = Self::apply_style(row, &node.style);
                for (i, child) in node.children.iter().enumerate() {
                    let k = format!("{key}/{i}");
                    row = row.child(self.render_node(child, &k, window, cx));
                }
                row.into_any_element()
            }
            "Center" => {
                let mut d = div().flex().size_full().items_center().justify_center();
                d = Self::apply_style(d, &node.style);
                for (i, child) in node.children.iter().enumerate() {
                    let k = format!("{key}/{i}");
                    d = d.child(self.render_node(child, &k, window, cx));
                }
                d.into_any_element()
            }
            "Container" => {
                let mut d = div().flex().flex_col().mx_auto().w_full().max_w(px(640.0));
                d = Self::apply_style(d, &node.style);
                for (i, child) in node.children.iter().enumerate() {
                    let k = format!("{key}/{i}");
                    d = d.child(self.render_node(child, &k, window, cx));
                }
                d.into_any_element()
            }
            "Space" => {
                let px_v = match self.prop_text(node, "size").as_str() {
                    "Xs" => 4.0,
                    "Sm" => 8.0,
                    "Lg" => 16.0,
                    "Xl" => 24.0,
                    _ => 12.0,
                };
                div().h(px(px_v)).into_any_element()
            }
            "Divider" => {
                let mut row = div().flex().flex_row().items_center().gap_2();
                row = row.child(div().flex_1().h(px(1.0)).bg(rgb(BORDER)));
                let label = self.prop_text(node, "label");
                if !label.is_empty() {
                    row = row
                        .child(div().text_xs().text_color(rgb(MUTED)).child(label))
                        .child(div().flex_1().h(px(1.0)).bg(rgb(BORDER)));
                }
                row.into_any_element()
            }
            "Paper" | "Card" => {
                let mut paper = guise::Paper::new()
                    .with_border(true)
                    .shadow(guise::prelude::Size::Xs);
                for (i, child) in node.children.iter().enumerate() {
                    let k = format!("{key}/{i}");
                    paper.extend([self.render_node(child, &k, window, cx)]);
                }
                paper.into_any_element()
            }
            "Title" => {
                let order: u8 = match self.prop_text(node, "order").as_str() {
                    "H1" => 1,
                    "H2" => 2,
                    "H4" => 4,
                    "H5" => 5,
                    "H6" => 6,
                    _ => 3,
                };
                let text = self.node_text(node);
                GuiseTitle::new(text).order(order).into_any_element()
            }
            "Text" => {
                let mut d = div().child(self.node_text(node));
                d = Self::apply_style(d, &node.style);
                d.into_any_element()
            }
            "Badge" => {
                let mut badge = guise::Badge::new(self.node_text(node));
                if let Some(variant) = Self::variant_of(node, "variant") {
                    badge = badge.variant(variant);
                }
                badge.into_any_element()
            }
            "Button" => {
                let label = self.node_text(node);
                let variant = match self.prop_text(node, "variant").as_str() {
                    "Light" => GuiseVariant::Light,
                    "Outline" => GuiseVariant::Outline,
                    "Subtle" => GuiseVariant::Subtle,
                    "Transparent" => GuiseVariant::Transparent,
                    "White" => GuiseVariant::White,
                    "Default" => GuiseVariant::Default,
                    _ => GuiseVariant::Filled,
                };
                let mut btn =
                    GuiseButton::new(SharedString::from(key.to_owned()), label).variant(variant);
                if self.prop_bool(node, "disabled", false) {
                    btn = btn.disabled(true);
                }
                let weak = cx.entity().downgrade();
                for (name, args) in self.events_for(node, "click") {
                    let weak = weak.clone();
                    btn = btn.on_click(move |_, _, cx| {
                        let args = args.clone();
                        let name = name.clone();
                        try_update(&weak, cx, "click", |view, cx| {
                            view.fire_call(&name, args, cx)
                        })
                        .ok();
                    });
                }
                btn.into_any_element()
            }
            "TextInput" | "Input" => {
                if !self.views.contains_key(key) {
                    let value = self.prop_text(node, "value");
                    let placeholder = self.prop_text(node, "placeholder");
                    let label = self.prop_text(node, "label");
                    let description = self.prop_text(node, "description");
                    let error = self.prop_text(node, "error");
                    let entity = cx.new(|cx| {
                        let mut input = guise::input::TextInput::new(cx)
                            .value(&value)
                            .placeholder(placeholder)
                            .label(label);
                        if !description.is_empty() {
                            input = input.description(description);
                        }
                        if !error.is_empty() {
                            input = input.error(error);
                        }
                        input
                    });
                    let calls = self.events_for(node, "change");
                    let binding = Self::binding_of(node, "value");
                    self.subscriptions.push(cx.subscribe(
                        &entity,
                        move |view, _input, event: &guise::input::TextInputEvent, cx| {
                            let text = match event {
                                guise::input::TextInputEvent::Change(t) => t.clone(),
                                _ => return,
                            };
                            for (name, args) in &calls {
                                view.fire_call(name, args.clone(), cx);
                            }
                            if let Some(path) = &binding {
                                view.write(path, Json::String(text.clone()), cx);
                            }
                        },
                    ));
                    self.views.insert(key.to_owned(), entity.into());
                }
                div()
                    .child(self.views.get(key).unwrap().clone())
                    .into_any_element()
            }
            "Select" => {
                if !self.views.contains_key(key) {
                    let options: Vec<String> = self
                        .prop_text(node, "options")
                        .split(',')
                        .map(|s| s.trim().to_owned())
                        .filter(|s| !s.is_empty())
                        .collect();
                    let label = self.prop_text(node, "label");
                    let selected = options
                        .iter()
                        .position(|o| *o == self.prop_text(node, "value"))
                        .unwrap_or(0);
                    let shared: Vec<SharedString> = options
                        .iter()
                        .map(|o| SharedString::from(o.clone()))
                        .collect();
                    let entity = cx.new(|cx| {
                        let mut s = guise::input::Select::new(cx).data(shared.clone());
                        if !label.is_empty() {
                            s = s.label(label);
                        }
                        s.selected(selected)
                    });
                    let calls = self.events_for(node, "change");
                    let binding = Self::binding_of(node, "value");
                    self.subscriptions.push(cx.subscribe(
                        &entity,
                        move |view, _select, event: &guise::input::SelectEvent, cx| {
                            let value = options.get(event.0).cloned().unwrap_or_default();
                            for (name, args) in &calls {
                                view.fire_call(name, args.clone(), cx);
                            }
                            if let Some(path) = &binding {
                                view.write(path, Json::String(value.clone()), cx);
                            }
                        },
                    ));
                    self.views.insert(key.to_owned(), entity.into());
                }
                div()
                    .child(self.views.get(key).unwrap().clone())
                    .into_any_element()
            }
            "Checkbox" | "Switch" => {
                let checked = self.prop_bool(node, "checked", false);
                let label = self.node_text(node);
                let weak = cx.entity().downgrade();
                let calls = self.events_for(node, "change");
                // Two-way: `checked: {{path}}` writes the toggle back to state.
                let binding = node
                    .props
                    .iter()
                    .find(|(k, _)| k == "checked")
                    .and_then(|(_, v)| match v {
                        Value::Binding { path, .. } => Some(path.clone()),
                        _ => None,
                    });
                let has_handler = !calls.is_empty() || binding.is_some();
                let on_change = move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                    for (name, args) in &calls {
                        try_update(&weak, cx, "change", |view, cx| {
                            view.fire_call(name, args.clone(), cx)
                        })
                        .ok();
                    }
                    if let Some(path) = &binding {
                        try_update(&weak, cx, "change", |view, cx| {
                            view.write(path, Json::Bool(!checked), cx)
                        })
                        .ok();
                    }
                };
                if node.element.name == "Checkbox" {
                    let mut c = guise::input::Checkbox::new(SharedString::from(key.to_owned()))
                        .checked(checked)
                        .label(label);
                    if has_handler {
                        c = c.on_change(on_change);
                    }
                    c.into_any_element()
                } else {
                    let mut s = guise::input::Switch::new(SharedString::from(key.to_owned()))
                        .checked(checked)
                        .label(label);
                    if has_handler {
                        s = s.on_change(on_change);
                    }
                    s.into_any_element()
                }
            }
            "Progress" => {
                let value = self.num(node, "value", 0.0).clamp(0.0, 100.0) as f32;
                guise::feedback::Progress::new(value).into_any_element()
            }
            "Alert" => {
                let title = self.prop_text(node, "title");
                let text = self.node_text(node);
                let mut alert = guise::feedback::Alert::new(text);
                if !title.is_empty() {
                    alert = alert.title(title);
                }
                if let Some(variant) = Self::variant_of(node, "variant") {
                    alert = alert.variant(variant);
                }
                alert.into_any_element()
            }
            "Icon" => {
                let name = self.prop_text(node, "name");
                match Self::icon_of(&name) {
                    Some(icon) => {
                        let mut i = guise::Icon::new(icon).size(self.size_of(node, "size"));
                        if let Some(c) = self.color_of(node, "color") {
                            i = i.color(c);
                        }
                        i.into_any_element()
                    }
                    None => placeholder(format!("Icon — unknown lucide name '{name}'")),
                }
            }
            "ThemeIcon" => {
                let glyph = Self::icon_of(&self.prop_text(node, "name"))
                    .map(guise::Glyph::from)
                    .unwrap_or_else(|| {
                        guise::Glyph::from(SharedString::from(self.prop_text(node, "name")))
                    });
                let mut t = guise::ThemeIcon::new(glyph).size(self.size_of(node, "size"));
                if let Some(v) = Self::variant_of(node, "variant") {
                    t = t.variant(v);
                }
                if let Some(c) = self.color_of(node, "color") {
                    t = t.color(c);
                }
                t.into_any_element()
            }
            "ActionIcon" => {
                let glyph = Self::icon_of(&self.prop_text(node, "name"))
                    .map(guise::Glyph::from)
                    .unwrap_or_else(|| {
                        guise::Glyph::from(SharedString::from(self.prop_text(node, "name")))
                    });
                let mut a = guise::ActionIcon::new(SharedString::from(key.to_owned()), glyph)
                    .size(self.size_of(node, "size"));
                let label = self.prop_text(node, "label");
                if !label.is_empty() {
                    a = a.label(label);
                }
                if let Some(v) = Self::variant_of(node, "variant") {
                    a = a.variant(v);
                }
                if let Some(c) = self.color_of(node, "color") {
                    a = a.color(c);
                }
                if self.prop_bool(node, "disabled", false) {
                    a = a.disabled(true);
                }
                if let Some(h) = self.click_handler(node, cx) {
                    a = a.on_click(h);
                }
                a.into_any_element()
            }
            "CloseButton" => {
                let mut b = guise::CloseButton::new(SharedString::from(key.to_owned()))
                    .size(self.size_of(node, "size"));
                if let Some(h) = self.click_handler(node, cx) {
                    b = b.on_click(h);
                }
                b.into_any_element()
            }
            "Chip" => {
                let checked = self.prop_bool(node, "checked", false);
                let mut c =
                    guise::Chip::new(SharedString::from(key.to_owned()), self.node_text(node))
                        .checked(checked)
                        .size(self.size_of(node, "size"));
                if let Some(col) = self.color_of(node, "color") {
                    c = c.color(col);
                }
                if let Some(h) = self.change_handler(node, "checked", checked, cx) {
                    c = c.on_change(h);
                }
                c.into_any_element()
            }
            "Kbd" => guise::Kbd::new(self.node_text(node)).into_any_element(),
            "Code" => {
                let mut c = guise::Code::new(self.node_text(node));
                if let Some(col) = self.color_of(node, "color") {
                    c = c.color(col);
                }
                c.into_any_element()
            }
            "Mark" => {
                let mut m = guise::Mark::new(self.node_text(node)).size(self.size_of(node, "size"));
                if let Some(c) = self.color_of(node, "color") {
                    m = m.color(c);
                }
                m.into_any_element()
            }
            "Blockquote" => {
                let mut b = guise::Blockquote::new().text(self.node_text(node));
                let cite = self.prop_text(node, "cite");
                if !cite.is_empty() {
                    b = b.cite(cite);
                }
                if let Some(c) = self.color_of(node, "color") {
                    b = b.color(c);
                }
                if let Some(i) = Self::icon_of(&self.prop_text(node, "icon")) {
                    b = b.icon(i);
                }
                b.into_any_element()
            }
            "Skeleton" => {
                let w = self.num(node, "width", 0.0) as f32;
                let h = self.num(node, "height", 0.0) as f32;
                let mut s = guise::Skeleton::new();
                if self.prop_bool(node, "circle", false) {
                    s = s.circle(if w > 0.0 { w } else { 40.0 });
                } else {
                    if w > 0.0 {
                        s = s.width(w);
                    }
                    if h > 0.0 {
                        s = s.height(h);
                    }
                }
                s.into_any_element()
            }
            "Indicator" => {
                let child = node
                    .children
                    .first()
                    .map(|c| self.render_node(c, &format!("{key}/0"), window, cx))
                    .unwrap_or_else(|| div().into_any_element());
                let mut i = guise::Indicator::new(child);
                let label = self.prop_text(node, "label");
                if !label.is_empty() {
                    i = i.label(label);
                }
                if let Some(c) = self.color_of(node, "color") {
                    i = i.color(c);
                }
                if self.prop_bool(node, "disabled", false) {
                    i = i.disabled(true);
                }
                i.into_any_element()
            }
            "Image" => {
                let src = self.prop_text(node, "src");
                if src.is_empty() {
                    return placeholder("Image — src required".to_owned());
                }
                let mut img = if src.starts_with("http://") || src.starts_with("https://") {
                    guise::Image::new(src.clone())
                } else {
                    guise::Image::new(self.resolve_path(&src))
                };
                let w = self.num(node, "width", 0.0) as f32;
                let h = self.num(node, "height", 0.0) as f32;
                if w > 0.0 {
                    img = img.width(w);
                }
                if h > 0.0 {
                    img = img.height(h);
                }
                if self.prop_bool(node, "circle", false) {
                    img = img.circle();
                }
                img = img.fit(match self.prop_text(node, "fit").as_str() {
                    "Contain" => guise::ObjectFit::Contain,
                    "Fill" => guise::ObjectFit::Fill,
                    "None" => guise::ObjectFit::None,
                    "ScaleDown" => guise::ObjectFit::ScaleDown,
                    _ => guise::ObjectFit::Cover,
                });
                img.into_any_element()
            }
            "Panel" => {
                let mut p = guise::Panel::new();
                let title = self.prop_text(node, "title");
                let description = self.prop_text(node, "description");
                if !title.is_empty() {
                    p = p.title(title);
                }
                if !description.is_empty() {
                    p = p.description(description);
                }
                for (i, child) in node.children.iter().enumerate() {
                    let k = format!("{key}/{i}");
                    p.extend([self.render_node(child, &k, window, cx)]);
                }
                p.into_any_element()
            }
            "Spoiler" => {
                let mut s = guise::Spoiler::new(SharedString::from(key.to_owned()))
                    .max_height(self.num(node, "max_height", 96.0) as f32)
                    .expanded(self.prop_bool(node, "expanded", false))
                    .size(self.size_of(node, "size"));
                let show = self.prop_text(node, "show_label");
                let hide = self.prop_text(node, "hide_label");
                if !show.is_empty() {
                    s = s.show_label(show);
                }
                if !hide.is_empty() {
                    s = s.hide_label(hide);
                }
                if let Some(c) = self.color_of(node, "color") {
                    s = s.color(c);
                }
                for (i, child) in node.children.iter().enumerate() {
                    let k = format!("{key}/{i}");
                    s.extend([self.render_node(child, &k, window, cx)]);
                }
                s.into_any_element()
            }
            "CopyButton" => {
                if !self.views.contains_key(key) {
                    let mut b = guise::CopyButton::new(self.prop_text(node, "text"));
                    let label = self.prop_text(node, "label");
                    if !label.is_empty() {
                        b = b.label(label);
                    }
                    let entity = cx.new(|_| b);
                    self.views.insert(key.to_owned(), entity.into());
                }
                div()
                    .child(self.views.get(key).unwrap().clone())
                    .into_any_element()
            }
            "Slider" => {
                if !self.views.contains_key(key) {
                    let entity = cx.new(|cx| {
                        let mut w = guise::input::Slider::new(cx)
                            .value(self.num(node, "value", 0.0))
                            .min(self.num(node, "min", 0.0))
                            .max(self.num(node, "max", 100.0))
                            .step(self.num(node, "step", 1.0));
                        if let Some(c) = self.color_of(node, "color") {
                            w = w.color(c);
                        }
                        if self.prop_bool(node, "disabled", false) {
                            w = w.disabled(true);
                        }
                        w
                    });
                    let calls = self.events_for(node, "change");
                    let binding = Self::binding_of(node, "value");
                    self.subscriptions.push(cx.subscribe(
                        &entity,
                        move |view, _w, e: &guise::input::SliderEvent, cx| {
                            for (name, args) in &calls {
                                view.fire_call(name, args.clone(), cx);
                            }
                            if let Some(p) = &binding {
                                view.write(p, Json::from(e.0), cx);
                            }
                        },
                    ));
                    self.views.insert(key.to_owned(), entity.into());
                }
                div()
                    .child(self.views.get(key).unwrap().clone())
                    .into_any_element()
            }
            "RangeSlider" => {
                if !self.views.contains_key(key) {
                    let parts: Vec<f64> = self
                        .prop_text(node, "value")
                        .split(',')
                        .filter_map(|s| s.trim().parse().ok())
                        .collect();
                    let lo = parts.first().copied().unwrap_or(20.0);
                    let hi = parts.get(1).copied().unwrap_or(80.0);
                    let entity = cx.new(|cx| {
                        let mut w = guise::input::RangeSlider::new(cx)
                            .value((lo, hi))
                            .min(self.num(node, "min", 0.0))
                            .max(self.num(node, "max", 100.0))
                            .step(self.num(node, "step", 1.0))
                            .min_gap(self.num(node, "min_gap", 0.0));
                        if let Some(c) = self.color_of(node, "color") {
                            w = w.color(c);
                        }
                        if self.prop_bool(node, "disabled", false) {
                            w = w.disabled(true);
                        }
                        w
                    });
                    let calls = self.events_for(node, "change");
                    let binding = Self::binding_of(node, "value");
                    self.subscriptions.push(cx.subscribe(
                        &entity,
                        move |view, _w, e: &guise::input::RangeSliderEvent, cx| {
                            for (name, args) in &calls {
                                view.fire_call(name, args.clone(), cx);
                            }
                            if let Some(p) = &binding {
                                view.write(
                                    p,
                                    Json::Array(vec![Json::from(e.0.0), Json::from(e.0.1)]),
                                    cx,
                                );
                            }
                        },
                    ));
                    self.views.insert(key.to_owned(), entity.into());
                }
                div()
                    .child(self.views.get(key).unwrap().clone())
                    .into_any_element()
            }
            "NumberInput" => {
                if !self.views.contains_key(key) {
                    let entity = cx.new(|cx| {
                        let mut w = guise::input::NumberInput::new(cx)
                            .value(self.num(node, "value", 0.0))
                            .min(self.num(node, "min", f64::MIN))
                            .max(self.num(node, "max", f64::MAX))
                            .step(self.num(node, "step", 1.0))
                            .size(self.size_of(node, "size"));
                        for (m, p) in [
                            (guise::input::NumberInput::label as fn(_, _) -> _, "label"),
                            (
                                guise::input::NumberInput::description as fn(_, _) -> _,
                                "description",
                            ),
                            (guise::input::NumberInput::error as fn(_, _) -> _, "error"),
                        ] {
                            let v = self.prop_text(node, p);
                            if !v.is_empty() {
                                w = m(w, v);
                            }
                        }
                        if self.prop_bool(node, "disabled", false) {
                            w = w.disabled(true);
                        }
                        w
                    });
                    let calls = self.events_for(node, "change");
                    let binding = Self::binding_of(node, "value");
                    self.subscriptions.push(cx.subscribe(
                        &entity,
                        move |view, _w, e: &guise::input::NumberInputEvent, cx| {
                            for (name, args) in &calls {
                                view.fire_call(name, args.clone(), cx);
                            }
                            if let Some(p) = &binding {
                                view.write(p, Json::from(e.0), cx);
                            }
                        },
                    ));
                    self.views.insert(key.to_owned(), entity.into());
                }
                div()
                    .child(self.views.get(key).unwrap().clone())
                    .into_any_element()
            }
            "PasswordInput" => {
                if !self.views.contains_key(key) {
                    let entity = cx.new(|cx| {
                        let mut w =
                            guise::input::PasswordInput::new(cx).size(self.size_of(node, "size"));
                        for (m, p) in [
                            ("value", "value"),
                            ("placeholder", "placeholder"),
                            ("label", "label"),
                            ("description", "description"),
                            ("error", "error"),
                        ] {
                            let v = self.prop_text(node, p);
                            if !v.is_empty() {
                                w = match m {
                                    "value" => w.value(&v),
                                    "placeholder" => w.placeholder(&v),
                                    "label" => w.label(v),
                                    "description" => w.description(v),
                                    _ => w.error(v),
                                };
                            }
                        }
                        if self.prop_bool(node, "disabled", false) {
                            w = w.disabled(true);
                        }
                        w
                    });
                    let calls = self.events_for(node, "change");
                    let binding = Self::binding_of(node, "value");
                    self.subscriptions.push(cx.subscribe(
                        &entity,
                        move |view, _w, e: &guise::input::PasswordInputEvent, cx| {
                            let text = match e {
                                guise::input::PasswordInputEvent::Change(t)
                                | guise::input::PasswordInputEvent::Submit(t) => t.clone(),
                            };
                            for (name, args) in &calls {
                                view.fire_call(name, args.clone(), cx);
                            }
                            if let Some(p) = &binding {
                                view.write(p, Json::String(text), cx);
                            }
                        },
                    ));
                    self.views.insert(key.to_owned(), entity.into());
                }
                div()
                    .child(self.views.get(key).unwrap().clone())
                    .into_any_element()
            }
            "PinInput" => {
                if !self.views.contains_key(key) {
                    let entity = cx.new(|cx| {
                        let mut w = guise::input::PinInput::new(cx)
                            .length(self.num(node, "length", 4.0) as usize)
                            .size(self.size_of(node, "size"));
                        let v = self.prop_text(node, "value");
                        if !v.is_empty() {
                            w = w.value(&v);
                        }
                        if self.prop_bool(node, "mask", false) {
                            w = w.mask(true);
                        }
                        if self.prop_bool(node, "disabled", false) {
                            w = w.disabled(true);
                        }
                        w
                    });
                    let calls = self.events_for(node, "change");
                    let binding = Self::binding_of(node, "value");
                    self.subscriptions.push(cx.subscribe(
                        &entity,
                        move |view, _w, e: &guise::input::PinInputEvent, cx| {
                            let text = match e {
                                guise::input::PinInputEvent::Change(t)
                                | guise::input::PinInputEvent::Complete(t) => t.clone(),
                            };
                            for (name, args) in &calls {
                                view.fire_call(name, args.clone(), cx);
                            }
                            if let Some(p) = &binding {
                                view.write(p, Json::String(text), cx);
                            }
                        },
                    ));
                    self.views.insert(key.to_owned(), entity.into());
                }
                div()
                    .child(self.views.get(key).unwrap().clone())
                    .into_any_element()
            }
            "TextArea" => {
                if !self.views.contains_key(key) {
                    let entity = cx.new(|cx| {
                        let mut w =
                            guise::input::TextArea::new(cx).size(self.size_of(node, "size"));
                        for (m, p) in [
                            ("value", "value"),
                            ("placeholder", "placeholder"),
                            ("label", "label"),
                            ("description", "description"),
                            ("error", "error"),
                        ] {
                            let v = self.prop_text(node, p);
                            if !v.is_empty() {
                                w = match m {
                                    "value" => w.value(&v),
                                    "placeholder" => w.placeholder(&v),
                                    "label" => w.label(v),
                                    "description" => w.description(v),
                                    _ => w.error(v),
                                };
                            }
                        }
                        let rows = self.num(node, "rows", 0.0) as usize;
                        if rows > 0 {
                            w = w.rows(rows);
                        }
                        let max_rows = self.num(node, "max_rows", 0.0) as usize;
                        if max_rows > 0 {
                            w = w.max_rows(max_rows);
                        }
                        if self.prop_bool(node, "submit_on_enter", false) {
                            w = w.submit_on_enter(true);
                        }
                        if self.prop_bool(node, "disabled", false) {
                            w = w.disabled(true);
                        }
                        w
                    });
                    let calls = self.events_for(node, "change");
                    let binding = Self::binding_of(node, "value");
                    self.subscriptions.push(cx.subscribe(
                        &entity,
                        move |view, _w, e: &guise::input::TextAreaEvent, cx| {
                            for (name, args) in &calls {
                                view.fire_call(name, args.clone(), cx);
                            }
                            if let Some(p) = &binding {
                                view.write(p, Json::String(e.0.clone()), cx);
                            }
                        },
                    ));
                    self.views.insert(key.to_owned(), entity.into());
                }
                div()
                    .child(self.views.get(key).unwrap().clone())
                    .into_any_element()
            }
            "TagsInput" => {
                if !self.views.contains_key(key) {
                    let tags = self.options_of(node, "options");
                    let entity = cx.new(|cx| {
                        let mut w =
                            guise::input::TagsInput::new(cx).size(self.size_of(node, "size"));
                        for (m, p) in [
                            ("placeholder", "placeholder"),
                            ("label", "label"),
                            ("description", "description"),
                            ("error", "error"),
                        ] {
                            let v = self.prop_text(node, p);
                            if !v.is_empty() {
                                w = match m {
                                    "placeholder" => w.placeholder(v),
                                    "label" => w.label(v),
                                    "description" => w.description(v),
                                    _ => w.error(v),
                                };
                            }
                        }
                        let max = self.num(node, "max_tags", 0.0) as usize;
                        if max > 0 {
                            w = w.max_tags(max);
                        }
                        if self.prop_bool(node, "disabled", false) {
                            w = w.disabled(true);
                        }
                        w
                    });
                    if !tags.is_empty() {
                        entity.update(cx, |t, cx| {
                            t.set_tags(tags.iter().map(|s| s.to_string()).collect(), cx)
                        });
                    }
                    let calls = self.events_for(node, "change");
                    let binding = Self::binding_of(node, "value");
                    self.subscriptions.push(cx.subscribe(
                        &entity,
                        move |view, _w, e: &guise::input::TagsInputEvent, cx| {
                            for (name, args) in &calls {
                                view.fire_call(name, args.clone(), cx);
                            }
                            if let Some(p) = &binding {
                                view.write(
                                    p,
                                    Json::Array(
                                        e.0.iter().map(|s| Json::String(s.clone())).collect(),
                                    ),
                                    cx,
                                );
                            }
                        },
                    ));
                    self.views.insert(key.to_owned(), entity.into());
                }
                div()
                    .child(self.views.get(key).unwrap().clone())
                    .into_any_element()
            }
            "Combobox" => {
                if !self.views.contains_key(key) {
                    let options = self.options_of(node, "options");
                    let strings: Vec<String> = options.iter().map(|s| s.to_string()).collect();
                    let entity = cx.new(|cx| {
                        let mut w = guise::input::Combobox::new(cx)
                            .data(options.clone())
                            .size(self.size_of(node, "size"));
                        for (m, p) in [("placeholder", "placeholder"), ("label", "label")] {
                            let v = self.prop_text(node, p);
                            if !v.is_empty() {
                                w = if m == "placeholder" {
                                    w.placeholder(v)
                                } else {
                                    w.label(v)
                                };
                            }
                        }
                        let selected = self.indices_of(node, "value");
                        if !selected.is_empty() {
                            w = w.selected(selected);
                        }
                        if self.prop_bool(node, "multiple", false) {
                            w = w.multiple(true);
                        }
                        if self.prop_bool(node, "disabled", false) {
                            w = w.disabled(true);
                        }
                        w
                    });
                    let calls = self.events_for(node, "change");
                    let binding = Self::binding_of(node, "value");
                    self.subscriptions.push(cx.subscribe(
                        &entity,
                        move |view, _w, e: &guise::input::ComboboxEvent, cx| {
                            let value = strings.get(e.0).cloned().unwrap_or_default();
                            for (name, args) in &calls {
                                view.fire_call(name, args.clone(), cx);
                            }
                            if let Some(p) = &binding {
                                view.write(p, Json::String(value), cx);
                            }
                        },
                    ));
                    self.views.insert(key.to_owned(), entity.into());
                }
                div()
                    .child(self.views.get(key).unwrap().clone())
                    .into_any_element()
            }
            "Autocomplete" => {
                if !self.views.contains_key(key) {
                    let suggestions = self.options_of(node, "options");
                    let entity = cx.new(|cx| {
                        let mut w = guise::input::Autocomplete::new(cx)
                            .suggestions(suggestions.clone())
                            .size(self.size_of(node, "size"));
                        for (m, p) in [
                            ("value", "value"),
                            ("placeholder", "placeholder"),
                            ("label", "label"),
                        ] {
                            let v = self.prop_text(node, p);
                            if !v.is_empty() {
                                w = match m {
                                    "value" => w.value(v),
                                    "placeholder" => w.placeholder(v),
                                    _ => w.label(v),
                                };
                            }
                        }
                        let max = self.num(node, "max_shown", 0.0) as usize;
                        if max > 0 {
                            w = w.max_shown(max);
                        }
                        if self.prop_bool(node, "disabled", false) {
                            w = w.disabled(true);
                        }
                        w
                    });
                    let calls = self.events_for(node, "change");
                    let binding = Self::binding_of(node, "value");
                    self.subscriptions.push(cx.subscribe(
                        &entity,
                        move |view, _w, e: &guise::input::AutocompleteEvent, cx| {
                            let text = match e {
                                guise::input::AutocompleteEvent::Change(t)
                                | guise::input::AutocompleteEvent::Commit(t) => t.clone(),
                            };
                            for (name, args) in &calls {
                                view.fire_call(name, args.clone(), cx);
                            }
                            if let Some(p) = &binding {
                                view.write(p, Json::String(text), cx);
                            }
                        },
                    ));
                    self.views.insert(key.to_owned(), entity.into());
                }
                div()
                    .child(self.views.get(key).unwrap().clone())
                    .into_any_element()
            }
            "ColorInput" => {
                if !self.views.contains_key(key) {
                    let entity = cx.new(|cx| {
                        let mut w =
                            guise::input::ColorInput::new(cx).size(self.size_of(node, "size"));
                        for (m, p) in [
                            ("label", "label"),
                            ("description", "description"),
                            ("error", "error"),
                        ] {
                            let v = self.prop_text(node, p);
                            if !v.is_empty() {
                                w = match m {
                                    "label" => w.label(v),
                                    "description" => w.description(v),
                                    _ => w.error(v),
                                };
                            }
                        }
                        if let Some(h) = Self::hsla_of(&self.prop_text(node, "value")) {
                            w = w.value(h);
                        }
                        if self.prop_bool(node, "disabled", false) {
                            w = w.disabled(true);
                        }
                        w
                    });
                    let calls = self.events_for(node, "change");
                    let binding = Self::binding_of(node, "value");
                    self.subscriptions.push(cx.subscribe(
                        &entity,
                        move |view, _w, e: &guise::input::ColorInputEvent, cx| {
                            for (name, args) in &calls {
                                view.fire_call(name, args.clone(), cx);
                            }
                            if let Some(p) = &binding {
                                view.write(p, Json::String(e.0.to_string()), cx);
                            }
                        },
                    ));
                    self.views.insert(key.to_owned(), entity.into());
                }
                div()
                    .child(self.views.get(key).unwrap().clone())
                    .into_any_element()
            }
            "DatePicker" => {
                if !self.views.contains_key(key) {
                    let entity = cx.new(|cx| {
                        let mut w =
                            guise::input::DatePicker::new(cx).size(self.size_of(node, "size"));
                        for (m, p) in [("placeholder", "placeholder"), ("label", "label")] {
                            let v = self.prop_text(node, p);
                            if !v.is_empty() {
                                w = if m == "placeholder" {
                                    w.placeholder(v)
                                } else {
                                    w.label(v)
                                };
                            }
                        }
                        for (m, p) in [("value", "value"), ("min", "min"), ("max", "max")] {
                            if let Some(d) = guise::input::Date::parse_iso(&self.prop_text(node, p))
                            {
                                w = match m {
                                    "value" => w.value(d),
                                    "min" => w.min(d),
                                    _ => w.max(d),
                                };
                            }
                        }
                        if self.prop_bool(node, "range", false) {
                            w = w.range_mode();
                        }
                        if self.prop_bool(node, "disabled", false) {
                            w = w.disabled(true);
                        }
                        w
                    });
                    let calls = self.events_for(node, "change");
                    let binding = Self::binding_of(node, "value");
                    self.subscriptions.push(cx.subscribe(
                        &entity,
                        move |view, _w, e: &guise::input::DatePickerEvent, cx| {
                            let value = match e {
                                guise::input::DatePickerEvent::Selected(d) => {
                                    Json::String(format!("{}", d))
                                }
                                guise::input::DatePickerEvent::Range(a, b) => Json::Array(vec![
                                    Json::String(format!("{}", a)),
                                    Json::String(format!("{}", b)),
                                ]),
                            };
                            for (name, args) in &calls {
                                view.fire_call(name, args.clone(), cx);
                            }
                            if let Some(p) = &binding {
                                view.write(p, value, cx);
                            }
                        },
                    ));
                    self.views.insert(key.to_owned(), entity.into());
                }
                div()
                    .child(self.views.get(key).unwrap().clone())
                    .into_any_element()
            }
            "TimePicker" => {
                if !self.views.contains_key(key) {
                    let entity = cx.new(|cx| {
                        let mut w =
                            guise::input::TimePicker::new(cx).size(self.size_of(node, "size"));
                        for (m, p) in [("placeholder", "placeholder"), ("label", "label")] {
                            let v = self.prop_text(node, p);
                            if !v.is_empty() {
                                w = if m == "placeholder" {
                                    w.placeholder(v)
                                } else {
                                    w.label(v)
                                };
                            }
                        }
                        if let Some(t) = guise::input::Time::parse(&self.prop_text(node, "value")) {
                            w = w.value(t);
                        }
                        if self.prop_bool(node, "hour24", true) {
                            w = w.twenty_four_hour();
                        }
                        let step = self.num(node, "minute_step", 0.0) as u32;
                        if step > 0 {
                            w = w.minute_step(step);
                        }
                        if self.prop_bool(node, "disabled", false) {
                            w = w.disabled(true);
                        }
                        w
                    });
                    let calls = self.events_for(node, "change");
                    let binding = Self::binding_of(node, "value");
                    self.subscriptions.push(cx.subscribe(
                        &entity,
                        move |view, _w, e: &guise::input::TimePickerEvent, cx| {
                            for (name, args) in &calls {
                                view.fire_call(name, args.clone(), cx);
                            }
                            if let Some(p) = &binding {
                                view.write(p, Json::String(e.0.format_24()), cx);
                            }
                        },
                    ));
                    self.views.insert(key.to_owned(), entity.into());
                }
                div()
                    .child(self.views.get(key).unwrap().clone())
                    .into_any_element()
            }
            "FileInput" => {
                if !self.views.contains_key(key) {
                    let entity = cx.new(|cx| {
                        let mut w =
                            guise::input::FileInput::new(cx).size(self.size_of(node, "size"));
                        for (m, p) in [("placeholder", "placeholder"), ("label", "label")] {
                            let v = self.prop_text(node, p);
                            if !v.is_empty() {
                                w = if m == "placeholder" {
                                    w.placeholder(v)
                                } else {
                                    w.label(v)
                                };
                            }
                        }
                        if self.prop_bool(node, "multiple", false) {
                            w = w.multiple();
                        }
                        if self.prop_bool(node, "directories", false) {
                            w = w.directories();
                        }
                        if self.prop_bool(node, "disabled", false) {
                            w = w.disabled(true);
                        }
                        w
                    });
                    let calls = self.events_for(node, "change");
                    let binding = Self::binding_of(node, "value");
                    self.subscriptions.push(cx.subscribe(
                        &entity,
                        move |view, _w, e: &guise::input::FileInputEvent, cx| {
                            let joined =
                                e.0.iter()
                                    .map(|p| p.display().to_string())
                                    .collect::<Vec<_>>()
                                    .join(";");
                            for (name, args) in &calls {
                                view.fire_call(name, args.clone(), cx);
                            }
                            if let Some(p) = &binding {
                                view.write(p, Json::String(joined), cx);
                            }
                        },
                    ));
                    self.views.insert(key.to_owned(), entity.into());
                }
                div()
                    .child(self.views.get(key).unwrap().clone())
                    .into_any_element()
            }
            "SegmentedControl" => {
                if !self.views.contains_key(key) {
                    let options = self.options_of(node, "options");
                    let strings: Vec<String> = options.iter().map(|s| s.to_string()).collect();
                    let entity = cx.new(|cx| {
                        let mut w = guise::input::SegmentedControl::new(cx)
                            .data(options.clone())
                            .size(self.size_of(node, "size"));
                        let selected = self.indices_of(node, "value");
                        if let Some(&i) = selected.first() {
                            w = w.selected(i);
                        }
                        w
                    });
                    let calls = self.events_for(node, "change");
                    let binding = Self::binding_of(node, "value");
                    self.subscriptions.push(cx.subscribe(
                        &entity,
                        move |view, _w, e: &guise::input::SegmentedControlEvent, cx| {
                            let value = strings.get(e.0).cloned().unwrap_or_default();
                            for (name, args) in &calls {
                                view.fire_call(name, args.clone(), cx);
                            }
                            if let Some(p) = &binding {
                                view.write(p, Json::String(value), cx);
                            }
                        },
                    ));
                    self.views.insert(key.to_owned(), entity.into());
                }
                div()
                    .child(self.views.get(key).unwrap().clone())
                    .into_any_element()
            }
            "Transfer" => {
                if !self.views.contains_key(key) {
                    let items = self.options_of(node, "options");
                    let strings: Vec<String> = items.iter().map(|s| s.to_string()).collect();
                    let entity = cx.new(|cx| {
                        let mut w = guise::input::Transfer::new(cx).data(items.clone());
                        let chosen = self.indices_of(node, "value");
                        if !chosen.is_empty() {
                            w = w.chosen(chosen);
                        }
                        let left = self.prop_text(node, "left_title");
                        let right = self.prop_text(node, "right_title");
                        if !left.is_empty() || !right.is_empty() {
                            w = w.titles(left, right);
                        }
                        let h = self.num(node, "height", 0.0) as f32;
                        if h > 0.0 {
                            w = w.height(h);
                        }
                        if self.prop_bool(node, "disabled", false) {
                            w = w.disabled(true);
                        }
                        w
                    });
                    let calls = self.events_for(node, "change");
                    let binding = Self::binding_of(node, "value");
                    self.subscriptions.push(cx.subscribe(
                        &entity,
                        move |view, _w, e: &guise::input::TransferEvent, cx| {
                            let names: Vec<Json> = e
                                .0
                                .iter()
                                .map(|i| {
                                    Json::String(
                                        strings.get(*i).cloned().unwrap_or_else(|| i.to_string()),
                                    )
                                })
                                .collect();
                            for (name, args) in &calls {
                                view.fire_call(name, args.clone(), cx);
                            }
                            if let Some(p) = &binding {
                                view.write(p, Json::Array(names), cx);
                            }
                        },
                    ));
                    self.views.insert(key.to_owned(), entity.into());
                }
                div()
                    .child(self.views.get(key).unwrap().clone())
                    .into_any_element()
            }
            "Rating" => {
                let mut r = guise::input::Rating::new(SharedString::from(key.to_owned()))
                    .value(self.num(node, "value", 0.0) as f32)
                    .size(self.size_of(node, "size"));
                let count = self.num(node, "count", 0.0) as usize;
                if count > 0 {
                    r = r.count(count);
                }
                if let Some(c) = self.color_of(node, "color") {
                    r = r.color(c);
                }
                if self.prop_bool(node, "readonly", false) {
                    r = r.readonly(true);
                }
                let calls = self.events_for(node, "change");
                let binding = Self::binding_of(node, "value");
                if !calls.is_empty() || binding.is_some() {
                    let weak = cx.entity().downgrade();
                    r = r.on_change(move |v: &f32, _w, cx| {
                        let v = *v as f64;
                        for (name, args) in &calls {
                            let weak = weak.clone();
                            let name = name.clone();
                            let args = args.clone();
                            try_update(&weak, cx, "change", |view, cx| {
                                view.fire_call(&name, args, cx)
                            })
                            .ok();
                        }
                        if let Some(p) = &binding {
                            try_update(&weak, cx, "change", |view, cx| {
                                view.write(p, Json::from(v), cx)
                            })
                            .ok();
                        }
                    });
                }
                r.into_any_element()
            }
            "Radio" => {
                let checked = self.prop_bool(node, "checked", false);
                let label = self.node_text(node);
                let mut r = guise::input::Radio::new(SharedString::from(key.to_owned()))
                    .checked(checked)
                    .size(self.size_of(node, "size"));
                if !label.is_empty() {
                    r = r.label(label);
                }
                if let Some(c) = self.color_of(node, "color") {
                    r = r.color(c);
                }
                if self.prop_bool(node, "disabled", false) {
                    r = r.disabled(true);
                }
                if let Some(h) = self.change_handler(node, "checked", checked, cx) {
                    r = r.on_change(h);
                }
                r.into_any_element()
            }
            "RadioGroup" => {
                let options = self.options_of(node, "options");
                let strings: Vec<String> = options.iter().map(|s| s.to_string()).collect();
                let mut g = guise::input::RadioGroup::new()
                    .options(options)
                    .size(self.size_of(node, "size"));
                let selected = self.indices_of(node, "value");
                if let Some(&i) = selected.first() {
                    g = g.value(i);
                }
                let label = self.prop_text(node, "label");
                if !label.is_empty() {
                    g = g.label(label);
                }
                if let Some(c) = self.color_of(node, "color") {
                    g = g.color(c);
                }
                let calls = self.events_for(node, "change");
                let binding = Self::binding_of(node, "value");
                if !calls.is_empty() || binding.is_some() {
                    let weak = cx.entity().downgrade();
                    g = g.on_change(move |i: usize, _w, cx| {
                        let value = strings.get(i).cloned().unwrap_or_default();
                        for (name, args) in &calls {
                            let weak = weak.clone();
                            let name = name.clone();
                            let args = args.clone();
                            try_update(&weak, cx, "change", |view, cx| {
                                view.fire_call(&name, args, cx)
                            })
                            .ok();
                        }
                        if let Some(p) = &binding {
                            let value = value.clone();
                            try_update(&weak, cx, "change", |view, cx| {
                                view.write(p, Json::String(value), cx)
                            })
                            .ok();
                        }
                    });
                }
                g.into_any_element()
            }
            "CheckboxGroup" => {
                let options = self.options_of(node, "options");
                let strings: Vec<String> = options.iter().map(|s| s.to_string()).collect();
                let mut g = guise::input::CheckboxGroup::new()
                    .options(options)
                    .size(self.size_of(node, "size"));
                let selected = self.indices_of(node, "value");
                if !selected.is_empty() {
                    g = g.value(selected);
                }
                let label = self.prop_text(node, "label");
                if !label.is_empty() {
                    g = g.label(label);
                }
                if let Some(c) = self.color_of(node, "color") {
                    g = g.color(c);
                }
                let calls = self.events_for(node, "change");
                let binding = Self::binding_of(node, "value");
                if !calls.is_empty() || binding.is_some() {
                    let weak = cx.entity().downgrade();
                    g = g.on_change(move |indices: Vec<usize>, _w, cx| {
                        let names: Vec<Json> = indices
                            .iter()
                            .map(|i| {
                                Json::String(
                                    strings.get(*i).cloned().unwrap_or_else(|| i.to_string()),
                                )
                            })
                            .collect();
                        for (name, args) in &calls {
                            let weak = weak.clone();
                            let name = name.clone();
                            let args = args.clone();
                            try_update(&weak, cx, "change", |view, cx| {
                                view.fire_call(&name, args, cx)
                            })
                            .ok();
                        }
                        if let Some(p) = &binding {
                            let names = names.clone();
                            try_update(&weak, cx, "change", |view, cx| {
                                view.write(p, Json::Array(names), cx)
                            })
                            .ok();
                        }
                    });
                }
                g.into_any_element()
            }
            "Calendar" => {
                let mut c = guise::input::Calendar::new(SharedString::from(key.to_owned()));
                let month = self.prop_text(node, "month");
                let mut parts = month.split('-');
                let y: i32 = parts
                    .next()
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);
                let m: u32 = parts
                    .next()
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);
                if y != 0 && m != 0 {
                    c = c.month(y, m);
                }
                if let Some(d) = guise::input::Date::parse_iso(&self.prop_text(node, "value")) {
                    c = c.value(Some(d));
                }
                let calls = self.events_for(node, "change");
                let binding = Self::binding_of(node, "value");
                if !calls.is_empty() || binding.is_some() {
                    let weak = cx.entity().downgrade();
                    c = c.on_select(move |d, _w, cx| {
                        let value = format!("{}", d);
                        for (name, args) in &calls {
                            let weak = weak.clone();
                            let name = name.clone();
                            let args = args.clone();
                            try_update(&weak, cx, "change", |view, cx| {
                                view.fire_call(&name, args, cx)
                            })
                            .ok();
                        }
                        if let Some(p) = &binding {
                            let value = value.clone();
                            try_update(&weak, cx, "change", |view, cx| {
                                view.write(p, Json::String(value), cx)
                            })
                            .ok();
                        }
                    });
                }
                c.into_any_element()
            }
            "Dropzone" => {
                let mut z = guise::input::Dropzone::new(SharedString::from(key.to_owned()));
                let label = self.prop_text(node, "label");
                if !label.is_empty() {
                    z = z.label(label);
                }
                let hint = self.prop_text(node, "hint");
                if !hint.is_empty() {
                    z = z.hint(hint);
                }
                if let Some(i) = Self::icon_of(&self.prop_text(node, "icon")) {
                    z = z.icon(i);
                }
                let accepts = self.options_of(node, "accept");
                if !accepts.is_empty() {
                    z = z.accept(accepts);
                }
                let h = self.num(node, "height", 0.0) as f32;
                if h > 0.0 {
                    z = z.height(h);
                }
                if self.prop_bool(node, "single", false) {
                    z = z.single();
                }
                if self.prop_bool(node, "no_click", false) {
                    z = z.no_click();
                }
                let calls = self.events_for(node, "change");
                let binding = Self::binding_of(node, "value");
                if !calls.is_empty() || binding.is_some() {
                    let weak = cx.entity().downgrade();
                    z = z.on_files(move |paths: Vec<std::path::PathBuf>, cx: &mut App| {
                        let joined = paths
                            .iter()
                            .map(|p| p.display().to_string())
                            .collect::<Vec<_>>()
                            .join(";");
                        for (name, args) in &calls {
                            let weak = weak.clone();
                            let name = name.clone();
                            let args = args.clone();
                            try_update(&weak, cx, "change", |view, cx| {
                                view.fire_call(&name, args, cx)
                            })
                            .ok();
                        }
                        if let Some(p) = &binding {
                            let joined = joined.clone();
                            try_update(&weak, cx, "change", |view, cx| {
                                view.write(p, Json::String(joined), cx)
                            })
                            .ok();
                        }
                    });
                }
                z.into_any_element()
            }
            "Field" => {
                let mut f = guise::input::Field::new();
                let label = self.prop_text(node, "label");
                if !label.is_empty() {
                    f = f.label(label);
                }
                let description = self.prop_text(node, "description");
                if !description.is_empty() {
                    f = f.description(description);
                }
                let error = self.prop_text(node, "error");
                if !error.is_empty() {
                    f = f.error(error);
                }
                let mut inner = div().flex().flex_col().gap_1();
                for (i, child) in node.children.iter().enumerate() {
                    let k = format!("{key}/{i}");
                    inner = inner.child(self.render_node(child, &k, window, cx));
                }
                f.child(inner).into_any_element()
            }
            _ => placeholder(format!("{} — no guise renderer", node.element.name)),
        }
    }

    /// The node's display text — content prop first, then `text`/`label`.
    fn node_text(&self, node: &Node) -> String {
        if let Some(v) = &node.content {
            let text = self.text_of(v);
            if !text.is_empty() {
                return text;
            }
        }
        let label = self.prop_text(node, "label");
        if !label.is_empty() {
            return label;
        }
        self.prop_text(node, "text")
    }
}

/// A dashed-border box for elements without a renderer yet.
fn placeholder(message: String) -> AnyElement {
    div()
        .px_2()
        .py_1()
        .border_1()
        .border_color(rgb(BORDER))
        .border_dashed()
        .rounded_md()
        .text_xs()
        .text_color(rgb(MUTED))
        .child(message)
        .into_any_element()
}

/// The floating chip rendered while dragging — mirrors the kit base's ghost.
struct DragGhost(SharedString);

impl Render for DragGhost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .rounded_md()
            .bg(rgb(BG))
            .border_1()
            .border_color(rgb(BORDER))
            .text_xs()
            .child(self.0.clone())
    }
}

impl Render for ForgeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.theme_ready {
            guise::theme::Theme::dark().init(cx);
            self.theme_ready = true;
        }
        let mut root = div()
            .id(SharedString::from("forge-root"))
            .size_full()
            .bg(rgb(BG))
            .text_color(rgb(FG));
        if let Some(doc) = self.doc.clone() {
            root = root.child(self.render_node(&doc.root, "0", _window, cx));
        } else if let Some(text) = self.empty_text.clone() {
            root = root.child(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(div().text_color(rgb(MUTED)).child(text)),
            );
        }
        root
    }
}
