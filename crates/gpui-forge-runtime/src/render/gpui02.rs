//! `ForgeView` for the gpui02 base — crates.io `gpui 0.2.2` + guise (#96).
//!
//! Same public contract as the gpui-kit twin: hosts drive `set_document`,
//! `update_state`, `fire_call`, `preview`, pick/place/dnd flags and sinks
//! identically. Element rendering maps the gpui02 catalog onto guise widgets
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
    /// Renders `alias.<element>`; `None` falls through to the unknown-element
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
// need literal colors; these match the zinc/blue scheme Studio uses.
const BG: u32 = 0x18181b;
const BORDER: u32 = 0x3f3f46;
const FG: u32 = 0xf4f4f5;
const MUTED: u32 = 0xa1a1aa;

pub struct ForgeView {
    doc: Option<Document>,
    pub catalogs: Vec<Catalog>,
    state: Json,
    previews: HashMap<String, Entity<ForgeView>>,
    pub libraries: Vec<Box<dyn ForgeLibrary>>,
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
    /// Per-key stateful input entities — a doc's nodes re-render but the
    /// widget state persists between paints (same contract as the kit base).
    inputs: HashMap<String, Entity<guise::input::TextInput>>,
    selects: HashMap<String, Entity<guise::input::Select>>,
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
            inputs: HashMap::new(),
            selects: HashMap::new(),
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

    /// Text shown while the view has no document (used by `forge.Preview`).
    pub fn set_empty_text(&mut self, text: Option<String>) {
        self.empty_text = text;
    }

    /// Registers `handler` for `on <event>: name(args)` bindings. Handlers may
    /// edit this view freely; unregistered names report to the hook sink.
    pub fn set_hook(&mut self, name: impl Into<String>, handler: HookHandler) {
        self.hooks.insert(name.into(), handler);
    }

    /// The `forge.Preview` host for `id`, creating the nested view on first use.
    pub fn preview(&mut self, id: &str, cx: &mut Context<Self>) -> Entity<ForgeView> {
        let catalogs = self.catalogs.clone();
        self.previews
            .entry(id.to_owned())
            .or_insert_with(|| cx.new(|_| ForgeView::new(catalogs)))
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
        self.libraries.push(Box::new(lib));
    }

    /// Drains accumulated errors — the host captures them to show or handle.
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

    /// Renders a document node — dispatch on `(namespace, element)`.
    fn render_node(
        &mut self,
        node: &Node,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if let Some(ns) = node.element.namespace.as_deref() {
            if ns == "forge" {
                // `forge.Preview` hosts a nested document — `set_preview_document`
                // fills `previews`; the other intrinsics stay stubs until the
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
                return placeholder(format!("forge.{} — gpui02 stub", node.element.name));
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
                if !self.inputs.contains_key(key) {
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
                    self.inputs.insert(key.to_owned(), entity);
                }
                let entity = self.inputs.get(key).unwrap().clone();
                div().child(entity).into_any_element()
            }
            "Select" => {
                if !self.selects.contains_key(key) {
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
                    self.selects.insert(key.to_owned(), entity);
                }
                let entity = self.selects.get(key).unwrap().clone();
                div().child(entity).into_any_element()
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
            _ => placeholder(format!("{} — no gpui02 renderer", node.element.name)),
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
