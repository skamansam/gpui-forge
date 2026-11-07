//! `ForgeView` for the iced base — the Elm-architecture twin (#113).
//!
//! Same host contract as the other bases (document/state/hooks/sinks, JSON
//! state paths, `fire_call`) with iced's split render model: `view()` produces
//! `iced::Element<Message>` and widget events come back through `update()`
//! as `Message`s — `on click:` bindings become `Message::Hooks`, two-way
//! inputs become `Message::Input`/`Toggle`/`Slide` writes.

use crate::render::{get_path, set_path};
use forge_spec::catalog::Catalog;
use forge_spec::{Document, Node, Path, Segment, Unit, Value};
use iced::widget::{
    button, checkbox, column, container, progress_bar, row, scrollable, slider, text, text_input,
    toggler,
};
use iced::{Element, Length};
use serde_json::Value as Json;
use std::collections::HashMap;
use std::rc::Rc;

/// Widget events routed back through `update` — built per-node in `view`.
#[derive(Clone, Debug)]
pub enum Message {
    /// `on <event>:` calls — each `(handler, args)` fires through `fire_call`.
    Hooks(Vec<(String, Vec<String>)>),
    /// Two-way input: the binding's state path + new text.
    Input(String, String),
    /// `checked: {{path}}` toggles.
    Toggle(String, bool),
    /// `value: {{path}}` slider moves.
    Slide(String, f32),
    /// A placeholder for events a scaffold arm can't express yet.
    Noop,
}

/// Called with `hook(args)` when an event fires and no hook is registered.
pub type HookSink = Rc<dyn Fn(&str, &mut ForgeView)>;
/// Called with the whole view state after a two-way binding writes to it.
pub type StateSink = Rc<dyn Fn(&Json, &mut ForgeView)>;
/// Rust logic behind `on <event>: name(args)` — the hook registry.
pub type HookHandler = Rc<dyn Fn(&[String], &mut ForgeView)>;

/// An external element library plugged into the runtime (the adapter contract).
pub trait ForgeLibrary {
    fn alias(&self) -> &'static str;
    fn catalog(&self) -> Option<&Catalog> {
        None
    }
}

pub struct ForgeView {
    doc: Option<Document>,
    pub catalogs: Vec<Catalog>,
    state: Json,
    pub libraries: Vec<Box<dyn ForgeLibrary>>,
    hooks: HashMap<String, HookHandler>,
    on_hook: Option<HookSink>,
    on_state: Option<StateSink>,
    errors: Vec<String>,
    base_dir: Option<std::path::PathBuf>,
    /// Flags staged for protocol parity — Elm has no pick/place/dnd overlays.
    #[allow(dead_code)]
    pick: bool,
    #[allow(dead_code)]
    placing: bool,
    #[allow(dead_code)]
    dnd: bool,
    empty_text: Option<String>,
}

impl ForgeView {
    pub fn new(catalogs: Vec<Catalog>) -> Self {
        Self {
            doc: None,
            catalogs,
            state: Json::Object(Default::default()),
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
        }
    }

    pub fn set_document(&mut self, doc: Document) {
        self.doc = Some(doc);
    }

    pub fn set_state(&mut self, state: Json) {
        self.state = state;
    }

    pub fn state(&self) -> &Json {
        &self.state
    }

    pub fn document(&self) -> Option<&Document> {
        self.doc.as_ref()
    }

    pub fn update_state(&mut self, path: &Path, value: Json) {
        self.write(path, value);
    }

    pub fn set_empty_text(&mut self, text: Option<String>) {
        self.empty_text = text;
    }

    pub fn set_hook(&mut self, name: impl Into<String>, handler: HookHandler) {
        self.hooks.insert(name.into(), handler);
    }

    pub fn set_sinks(&mut self, on_hook: HookSink, on_state: StateSink) {
        self.on_hook = Some(on_hook);
        self.on_state = Some(on_state);
    }

    /// Inspector prop write-through — `doc.set_prop`.
    pub fn set_node_prop(&mut self, path: &str, name: &str, json: &Json) {
        let value = if json.is_null() {
            None
        } else {
            Some(Value::String(
                json.as_str().unwrap_or(&json.to_string()).to_owned(),
            ))
        };
        if let Some(doc) = self.doc.as_mut() {
            doc.set_prop(path, name, value);
        }
    }

    /// `Style` tab write-through — `doc.set_style`.
    pub fn set_node_style(&mut self, path: &str, name: &str, json: &Json) {
        let value = if json.is_null() {
            None
        } else {
            Some(Value::String(json.as_str().unwrap_or_default().to_owned()))
        };
        if let Some(doc) = self.doc.as_mut() {
            doc.set_style(path, name, value);
        }
    }

    pub fn report(&mut self, message: impl Into<String>) {
        self.errors.push(message.into());
    }

    pub fn set_base_dir(&mut self, dir: impl Into<std::path::PathBuf>) {
        self.base_dir = Some(dir.into());
    }

    /// Protocol-parity no-ops.
    pub fn set_pick(&mut self, pick: bool) {
        self.pick = pick;
        self.placing = false;
    }
    pub fn set_placing(&mut self, placing: bool) {
        self.pick = placing;
        self.placing = placing;
    }
    pub fn set_dnd(&mut self, dnd: bool) {
        self.dnd = dnd;
    }

    /// Fires a registered hook by name, bubbling to the host's sink when this
    /// view doesn't handle it.
    pub fn fire_call(&mut self, name: &str, args: Vec<String>) {
        if let Some(handler) = self.hooks.get(name).cloned() {
            handler(&args, self);
        } else if let Some(sink) = self.on_hook.clone() {
            sink(&format!("{}({})", name, args.join(", ")), self);
        }
    }

    pub fn register_library(&mut self, lib: impl ForgeLibrary + 'static) {
        if let Some(cat) = lib.catalog().cloned() {
            self.catalogs.push(cat);
        }
        self.libraries.push(Box::new(lib));
    }

    pub fn take_errors(&mut self) -> Vec<String> {
        std::mem::take(&mut self.errors)
    }

    // ── state + value helpers (same semantics as the other twins) ───────────

    fn write(&mut self, path: &Path, value: Json) {
        set_path(&mut self.state, path, value);
        if let Some(sink) = self.on_state.clone() {
            let state = self.state.clone();
            sink(&state, self);
        }
    }

    fn lookup(&self, path: &Path) -> Option<&Json> {
        get_path(&self.state, path)
    }

    fn json_text(&self, path: &Path) -> String {
        self.lookup(path)
            .map(|j| match j {
                Json::String(s) => s.clone(),
                other => other.to_string(),
            })
            .unwrap_or_default()
    }

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

    fn binding_of(node: &Node, name: &str) -> Option<Path> {
        node.props
            .iter()
            .find(|(k, _)| k == name)
            .and_then(|(_, v)| match v {
                Value::Binding { path, .. } => Some(path.clone()),
                _ => None,
            })
    }

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

    /// `on <event>` bindings for a node — `(handler, args)` per binding.
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

    // ── Elm render ───────────────────────────────────────────────────────────

    /// iced `update` — a widget message routes into hooks or a state write.
    pub fn update(&mut self, msg: Message) {
        match msg {
            Message::Hooks(calls) => {
                for (name, args) in calls {
                    self.fire_call(&name, args);
                }
            }
            Message::Input(path, value) => {
                if let Ok(path) = forge_spec::value::parse_path(&path) {
                    self.write(&path, Json::String(value));
                }
            }
            Message::Toggle(path, checked) => {
                if let Ok(path) = forge_spec::value::parse_path(&path) {
                    self.write(&path, Json::Bool(checked));
                }
            }
            Message::Slide(path, value) => {
                if let Ok(path) = forge_spec::value::parse_path(&path) {
                    self.write(&path, Json::from(value));
                }
            }
            Message::Noop => {}
        }
    }

    /// iced `view` — the document as an `Element<Message>` tree.
    pub fn view(&self) -> Element<'static, Message> {
        if let Some(doc) = &self.doc {
            self.render_node(&doc.root)
        } else if let Some(label) = &self.empty_text {
            container(text(label.clone()))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        } else {
            column![].into()
        }
    }

    fn render_node(&self, node: &Node) -> Element<'static, Message> {
        match node.element.name.as_str() {
            "VStack" => {
                let children = node.children.iter().map(|c| self.render_node(c));
                column(children).spacing(8).padding(10).into()
            }
            "HStack" => {
                let children = node.children.iter().map(|c| self.render_node(c));
                row(children).spacing(8).padding(10).into()
            }
            "Container" => {
                let children = node.children.iter().map(|c| self.render_node(c));
                container(column(children).spacing(8)).padding(10).into()
            }
            "Scrollable" => {
                let children = node.children.iter().map(|c| self.render_node(c));
                scrollable(column(children).spacing(8)).into()
            }
            "Text" | "Title" => {
                let mut t = text(self.node_text(node));
                if node.element.name == "Title" {
                    t = t.size(24);
                }
                t.into()
            }
            "Button" => {
                let calls = self.events_for(node, "click");
                let mut btn = button(text(self.node_text(node)));
                if !calls.is_empty() {
                    btn = btn.on_press(Message::Hooks(calls));
                }
                btn.into()
            }
            "Input" | "TextInput" => {
                let placeholder = self.prop_text(node, "placeholder");
                let value = self.prop_text(node, "value");
                let binding = Self::binding_of(node, "value");
                let mut input = text_input(&placeholder, &value);
                if let Some(path) = binding {
                    input = input.on_input(move |v| Message::Input(path_to_string(&path), v));
                }
                input.into()
            }
            "Checkbox" | "Switch" => {
                let checked = self.prop_bool(node, "checked", false);
                let label = self.node_text(node);
                let binding = Self::binding_of(node, "checked");
                if node.element.name == "Checkbox" {
                    let mut cb = checkbox(label, checked);
                    if let Some(path) = binding {
                        cb = cb.on_toggle(move |v| Message::Toggle(path_to_string(&path), v));
                    }
                    cb.into()
                } else {
                    let mut t = toggler(checked).label(label);
                    if let Some(path) = binding {
                        t = t.on_toggle(move |v| Message::Toggle(path_to_string(&path), v));
                    }
                    t.into()
                }
            }
            "Slider" => {
                let value = self.num(node, "value", 0.0) as f32;
                let binding = Self::binding_of(node, "value");
                let mut s = slider(0.0..=100.0, value, move |v| {
                    Message::Slide(binding.as_ref().map(path_to_string).unwrap_or_default(), v)
                });
                let label = self.prop_text(node, "label");
                if !label.is_empty() {
                    return column![text(label), s].into();
                }
                let _ = &mut s;
                s.into()
            }
            "Progress" => {
                let value = self.num(node, "value", 0.0) as f32;
                progress_bar(0.0..=100.0, value).into()
            }
            _ => {
                let msg = format!("{} — no iced renderer", node.element.name);
                text(msg).into()
            }
        }
    }
}

/// `Path` → the `a.b` string a `Message` carries — `parse_path` reads it back.
fn path_to_string(path: &Path) -> String {
    path.0
        .iter()
        .map(|s| match s {
            forge_spec::PathSegment::Field(f) => f.clone(),
            forge_spec::PathSegment::Index(i) => i.to_string(),
        })
        .collect::<Vec<_>>()
        .join(".")
}
