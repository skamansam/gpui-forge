//! `ForgeView` for the ratatui base — a terminal-UI view (#112).
//!
//! Same host contract as the GPUI twins (document/state/hooks/sinks, JSON
//! state paths, `fire_call`) minus the GPUI machinery: there is no
//! `Context<Entity>` in a crossterm event loop, so sinks take `&mut ForgeView`
//! directly and `render` draws into a `ratatui::Frame`. The host
//! (`forge-preview-ratatui`) owns the terminal.

use crate::render::{doc_meta, lookup_path, set_path, style_edit_value};
use forge_spec::catalog::Catalog;
use forge_spec::{Compare, Control, Document, Node, Path, PathSegment, Segment, Unit, Value};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::calendar::{CalendarEventStore, Monthly};
use ratatui::widgets::canvas::{Canvas, Map, MapResolution};
use ratatui::widgets::{
    Axis, BarChart, Block, BorderType, Borders, Chart, Clear, Dataset, Gauge, GraphType, List,
    ListItem, Paragraph, RatatuiLogo, Sparkline, Tabs, Wrap,
};
use serde_json::Value as Json;
use std::collections::HashMap;
use std::rc::Rc;

/// Called with `hook(args)` when an event fires and no hook is registered for
/// its name — no `Context` exists in a TUI loop.
pub type HookSink = Rc<dyn Fn(&str, &mut ForgeView)>;
/// Called with the whole view state after a two-way binding writes to it.
pub type StateSink = Rc<dyn Fn(&Json, &mut ForgeView)>;
/// Rust logic behind `on <event>: name(args)` — the hook registry.
pub type HookHandler = Rc<dyn Fn(&[String], &mut ForgeView)>;

/// An external element library plugged into the runtime — alias + catalog +
/// render shim for `alias.Name` elements (the adapter contract).
pub trait ForgeLibrary {
    /// The `use:` alias this library handles.
    fn alias(&self) -> &'static str;
    /// The library's catalog manifest, merged into the view's catalogs on
    /// registration.
    fn catalog(&self) -> Option<&Catalog> {
        None
    }
    /// Draws an `alias.Name` node — the custom-widget hook. `view` gives the
    /// library resolved props, state writes and `fire_call`. Return true when
    /// the node was handled so the caller skips the placeholder.
    fn render(&self, node: &Node, area: Rect, view: &mut ForgeView, frame: &mut Frame) -> bool {
        let _ = (node, area, view, frame);
        false
    }
}

pub struct ForgeView {
    doc: Option<Document>,
    /// The document's `meta:` map — `{$meta.k}` bindings resolve against it.
    meta: Json,
    pub catalogs: Vec<Catalog>,
    state: Json,
    pub libraries: Vec<Rc<dyn ForgeLibrary>>,
    hooks: HashMap<String, HookHandler>,
    on_hook: Option<HookSink>,
    on_state: Option<StateSink>,
    /// Render-time problems surfaced for the host — `take_errors` drains them.
    errors: Vec<String>,
    /// The directory of the document this view renders.
    base_dir: Option<std::path::PathBuf>,
    /// `forge.Preview` hosts — a nested `ForgeView` per id, rendered into
    /// the node's area (the TUI's flat model has no `Entity`, so the map
    /// holds views directly).
    previews: HashMap<String, ForgeView>,
    /// Focusable inputs — `(binding path, kind, resolved `on:` calls,
    /// drawn rect)` rebuilt each render. `Tab`/`Shift-Tab` cycles, typed keys
    /// write the focused binding, `Enter`/`Space` fire the node's event
    /// calls. The rect is the widget's drawn area, so `on_mouse` hit-tests.
    focusables: Vec<(Path, FocusKind, Calls, Rect)>,
    /// Every rendered node's `(doc path, drawn rect)`, rebuilt each render.
    /// The sender resolves pointer cells to doc paths through it for the
    /// designer's drop-hover and drop (#156).
    zones: Vec<(String, Rect)>,
    /// The highlighted drop target — `(doc path, rect)` — painted as an
    /// accent frame over the zone while a drag hovers it.
    drop_target: Option<(String, Rect)>,
    focused: usize,
    /// `for`/`with` scope vars — `(name, value)` frames pushed while control
    /// children expand (same contract as the GPUI twins).
    scopes: Vec<(String, Json)>,
    /// The resolved theme palette — semantic name → `Color`, set by
    /// `HostMsg::Theme`'s `colors` map. `{$theme.*}` style values resolve
    /// here. An empty map keeps the renderer's default colors.
    theme: HashMap<String, Color>,
    /// The wire's dark flag, kept for libraries that paint both modes.
    #[allow(dead_code)]
    dark: bool,
    /// Flags staged for protocol parity — a TUI has no element picker, but the
    /// commands are acknowledged so hosts don't special-case the base.
    #[allow(dead_code)]
    pick: bool,
    #[allow(dead_code)]
    placing: bool,
    #[allow(dead_code)]
    dnd: bool,
    /// Shown while there is no document.
    empty_text: Option<String>,
}

impl ForgeView {
    pub fn new(catalogs: Vec<Catalog>) -> Self {
        Self {
            doc: None,
            catalogs,
            meta: Json::Object(Default::default()),
            state: Json::Object(Default::default()),
            libraries: Vec::new(),
            hooks: HashMap::new(),
            on_hook: None,
            on_state: None,
            errors: Vec::new(),
            base_dir: None,
            previews: HashMap::new(),
            focusables: Vec::new(),
            zones: Vec::new(),
            drop_target: None,
            focused: 0,
            scopes: Vec::new(),
            theme: HashMap::new(),
            dark: false,
            pick: false,
            placing: false,
            dnd: false,
            empty_text: None,
        }
    }

    pub fn set_document(&mut self, doc: Document) {
        self.meta = doc_meta(&doc);
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

    /// Writes `value` at `path` in the view state — the two-way binding write.
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
        let value = style_edit_value(json);
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

    /// `HostMsg::Theme` — the sender's resolved palette lands as the
    /// `{$theme.*}` lookup table. Colors arrive as `#rrggbb`/`#rrggbbaa`
    /// strings. Unparseable entries drop out.
    pub fn set_theme(&mut self, dark: bool, colors: &serde_json::Map<String, Json>) {
        self.dark = dark;
        self.theme = colors
            .iter()
            .filter_map(|(k, v)| {
                v.as_str()
                    .and_then(forge_spec::Color::parse_hex)
                    .map(|c| (k.clone(), Color::Rgb(c.r, c.g, c.b)))
            })
            .collect();
    }

    /// Protocol-parity no-ops — a TUI has no pick/place/dnd overlays.
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

    /// The `forge.Preview` host for `id`, creating the nested view on first use.
    /// The child shares the parent's registered libraries so `alias.Name`
    /// elements render inside previews too.
    pub fn preview(&mut self, id: &str) -> &mut ForgeView {
        let catalogs = self.catalogs.clone();
        let libraries = self.libraries.clone();
        self.previews.entry(id.to_owned()).or_insert_with(|| {
            let mut view = ForgeView::new(catalogs);
            view.libraries = libraries;
            view
        })
    }

    /// Sets the document a `forge.Preview` renders — the host forwards the
    /// protocol command here.
    pub fn set_preview_document(&mut self, id: &str, doc: Document) {
        self.preview(id).set_document(doc);
    }

    /// Registers a namespaced-element library (adapter contract).
    pub fn register_library(&mut self, lib: impl ForgeLibrary + 'static) {
        if let Some(cat) = lib.catalog().cloned() {
            self.catalogs.push(cat);
        }
        self.libraries.push(Rc::new(lib));
    }

    /// Drains accumulated errors — the host captures them.
    pub fn take_errors(&mut self) -> Vec<String> {
        std::mem::take(&mut self.errors)
    }

    /// Terminal keys — the host reads the tty and forwards them (#112).
    /// `Tab`/`Shift-Tab` cycles focusable inputs. Printable keys write into
    /// the focused input's binding. `Enter`/`Space` fires toggle/select.
    pub fn on_key(&mut self, key: &str) {
        let kind = self.focusables.get(self.focused).map(|(_, k, _, _)| *k);
        match (kind, key) {
            (_, "Tab") | (_, "BackTab") => {}
            (Some(FocusKind::Bool), "Enter") | (Some(FocusKind::Bool), " ") => {
                if let Some((path, _, _, _)) = self.focusables.get(self.focused).cloned() {
                    let cur = self.lookup(&path).and_then(Json::as_bool).unwrap_or(false);
                    self.write(&path, Json::Bool(!cur));
                }
                self.fire_focused("change");
                return;
            }
            (Some(FocusKind::Select), "Up") | (Some(FocusKind::Select), "k") => {
                if let Some((path, _, _, _)) = self.focusables.get(self.focused).cloned() {
                    let cur = self.lookup(&path).and_then(Json::as_u64).unwrap_or(0);
                    self.write(&path, Json::from(cur.saturating_sub(1)));
                }
                self.fire_focused("select");
                return;
            }
            (Some(FocusKind::Select), "Down") | (Some(FocusKind::Select), "j") => {
                if let Some((path, _, _, _)) = self.focusables.get(self.focused).cloned() {
                    let cur = self.lookup(&path).and_then(Json::as_u64).unwrap_or(0);
                    self.write(&path, Json::from(cur + 1));
                }
                self.fire_focused("select");
                return;
            }
            (Some(FocusKind::Input), "Backspace")
            | (Some(FocusKind::Input), "Enter")
            | (Some(FocusKind::Input), _)
            | (None, _) => {}
            (_, _) => {}
        }
        match key {
            "Tab" => {
                if !self.focusables.is_empty() {
                    self.focused = (self.focused + 1) % self.focusables.len();
                }
            }
            "BackTab" => {
                if !self.focusables.is_empty() {
                    self.focused =
                        (self.focused + self.focusables.len() - 1) % self.focusables.len();
                }
            }
            "Enter" => {
                self.fire_focused("submit");
            }
            "Backspace" => {
                if let Some((path, _, _, _)) = self.focusables.get(self.focused).cloned() {
                    let mut cur = self.json_text(&path);
                    cur.pop();
                    self.write(&path, Json::String(cur));
                }
                self.fire_focused("change");
            }
            c if c.chars().count() == 1 => {
                if let Some((path, _, _, _)) = self.focusables.get(self.focused).cloned() {
                    let mut cur = self.json_text(&path);
                    cur.push_str(c);
                    self.write(&path, Json::String(cur));
                }
                self.fire_focused("change");
            }
            _ => {}
        }
    }

    /// The node's `on:` bindings resolved to `(event, name, args)` — stored
    /// on the focusable so `on_key` can fire them without re-walking the doc.
    fn calls_for(&self, node: &Node) -> Calls {
        node.events
            .iter()
            .map(|e| {
                (
                    e.event.clone(),
                    e.handler.name.clone(),
                    e.handler.args.iter().map(|a| self.text_of(a)).collect(),
                )
            })
            .collect()
    }

    /// Fires the focused node's `on <event>:` calls through `fire_call`.
    fn fire_focused(&mut self, event: &str) {
        if let Some((_, _, calls, _)) = self.focusables.get(self.focused).cloned() {
            for (e, name, args) in calls {
                if e == event {
                    self.fire_call(&name, args);
                }
            }
        }
    }

    /// Whether `path` is the focused binding — the Input arm highlights it.
    fn is_focused(&self, path: &Path) -> bool {
        self.focusables
            .get(self.focused)
            .map(|(p, _, _, _)| p == path)
            .unwrap_or(false)
    }

    /// The `{{ path }}` binding behind a prop — two-way input wiring.
    fn binding_of(node: &Node, name: &str) -> Option<Path> {
        node.props
            .iter()
            .find(|(k, _)| k == name)
            .and_then(|(_, v)| match v {
                Value::Binding { path, .. } => Some(path.clone()),
                _ => None,
            })
    }

    // ── state + value helpers (same semantics as the GPUI twins) ────────────

    fn write(&mut self, path: &Path, value: Json) {
        set_path(&mut self.state, path, value);
        if let Some(sink) = self.on_state.clone() {
            let state = self.state.clone();
            sink(&state, self);
        }
    }

    fn lookup(&self, path: &Path) -> Option<&Json> {
        // `for`/`with` scope vars shadow state — innermost frame wins.
        if let Some(PathSegment::Field(name)) = path.0.first()
            && let Some((_, value)) = self.scopes.iter().rev().find(|(n, _)| n == name)
        {
            return path.0[1..]
                .iter()
                .try_fold(value, |json, segment| match segment {
                    PathSegment::Field(name) => json.get(name),
                    PathSegment::Index(i) => json.get(i),
                });
        }
        lookup_path(&self.meta, &self.state, path)
    }

    /// `{{ path }}` text — binding values render as JSON scalars.
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
            Value::Enum(s) => s.clone(),
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

    fn num(&self, node: &Node, name: &str, fallback: f64) -> f64 {
        match node.props.iter().find(|(k, _)| k == name).map(|(_, v)| v) {
            Some(Value::Number { value, .. }) => *value,
            Some(Value::Binding { path, .. }) => {
                self.lookup(path).and_then(Json::as_f64).unwrap_or(fallback)
            }
            _ => fallback,
        }
    }

    fn boolean(&self, node: &Node, name: &str, fallback: bool) -> bool {
        match node.props.iter().find(|(k, _)| k == name).map(|(_, v)| v) {
            Some(Value::Bool(b)) => *b,
            Some(Value::Binding { path, .. }) => self
                .lookup(path)
                .and_then(Json::as_bool)
                .unwrap_or(fallback),
            _ => fallback,
        }
    }

    /// A `list<number>` prop — a `[1, 2, 3]` literal or a bound state array.
    fn floats(&self, node: &Node, name: &str) -> Vec<f64> {
        match node.props.iter().find(|(k, _)| k == name).map(|(_, v)| v) {
            Some(Value::List(items)) => items
                .iter()
                .filter_map(|v| match v {
                    Value::Number { value, .. } => Some(*value),
                    _ => None,
                })
                .collect(),
            Some(Value::Binding { path, .. }) => self
                .lookup(path)
                .and_then(Json::as_array)
                .map(|a| a.iter().filter_map(Json::as_f64).collect())
                .unwrap_or_default(),
            _ => Vec::new(),
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

    // ── control flow + theme (same semantics as the GPUI twins) ────────────

    /// A `Value` as state `Json` — bindings look up, literals convert.
    fn resolve(&self, value: &Value) -> Json {
        match value {
            Value::Binding { path, .. } => self.lookup(path).cloned().unwrap_or(Json::Null),
            Value::Bool(b) => Json::Bool(*b),
            Value::Number { value, unit: None } => Json::from(*value),
            Value::List(items) => Json::Array(items.iter().map(|i| self.resolve(i)).collect()),
            Value::Map(entries) => Json::Object(
                entries
                    .iter()
                    .map(|(k, v)| (k.clone(), self.resolve(v)))
                    .collect(),
            ),
            other => Json::String(self.text_of(other)),
        }
    }

    /// `is:`/`not:`/`case` equality — numbers compare by value so a `u64`
    /// index write and a `4` literal agree.
    fn value_eq(&self, a: &Json, b: &Json) -> bool {
        match (a, b) {
            (Json::Number(x), Json::Number(y)) => x.as_f64() == y.as_f64(),
            _ => a == b,
        }
    }

    /// An `if:` condition — truthiness alone, or equality against `is:`/`not:`.
    fn condition(&self, when: &Value, test: &Option<Compare>) -> bool {
        let value = self.resolve(when);
        match test {
            None => json_truthy(&value),
            Some(Compare::Is(operand)) => self.value_eq(&value, &self.resolve(operand)),
            Some(Compare::Not(operand)) => !self.value_eq(&value, &self.resolve(operand)),
        }
    }

    /// `for:`'s source as items — a single (non-list) value iterates once.
    fn for_items(&self, source: &Value) -> Vec<Json> {
        match self.resolve(source) {
            Json::Array(items) => items,
            Json::Null => Vec::new(),
            value => vec![value],
        }
    }

    /// One node's expansion — `if` gates, `for` repeats, `with` scopes and
    /// `match` picks an arm. Scope bindings bake into the returned clones,
    /// so they render after the scope unwinds.
    fn expand(&mut self, node: &Node) -> Vec<Node> {
        let Some(control) = &node.control else {
            // No control — bake scope bindings into the clone. This is the
            // path bare `for:`/`with:` block children take.
            let mut node = node.clone();
            self.bake_scope(&mut node);
            return vec![node];
        };
        match control {
            Control::If {
                when,
                test,
                otherwise,
            } => {
                if self.condition(when, test) {
                    self.expand_target(node)
                } else {
                    self.expand_kids(otherwise)
                }
            }
            Control::For {
                source,
                each,
                index,
                otherwise,
                ..
            } => {
                let items = self.for_items(source);
                if items.is_empty() {
                    return self.expand_kids(otherwise);
                }
                let mut out = Vec::new();
                for (i, item) in items.into_iter().enumerate() {
                    self.scopes.push((each.clone(), item));
                    if let Some(index) = index {
                        self.scopes.push((index.clone(), Json::from(i)));
                    }
                    out.extend(self.expand_target(node));
                    let pop = 1 + usize::from(index.is_some());
                    self.scopes.truncate(self.scopes.len() - pop);
                }
                out
            }
            Control::With { source, name } => {
                let value = self.resolve(source);
                self.scopes.push((name.clone(), value));
                let out = self.expand_target(node);
                self.scopes.pop();
                out
            }
            Control::Match {
                source,
                cases,
                otherwise,
            } => {
                let value = self.resolve(source);
                match cases
                    .iter()
                    .find(|case| self.value_eq(&self.resolve(&case.value), &value))
                {
                    Some(case) => self.expand_kids(&case.children),
                    None => self.expand_kids(otherwise),
                }
            }
        }
    }

    fn expand_kids(&mut self, nodes: &[Node]) -> Vec<Node> {
        let mut out = Vec::new();
        for node in nodes {
            out.extend(self.expand(node));
        }
        out
    }

    /// The control node's payload. A bare block expands its `children`. A
    /// modified element becomes one baked clone.
    fn expand_target(&mut self, node: &Node) -> Vec<Node> {
        if node.is_block() {
            self.expand_kids(&node.children)
        } else {
            let mut node = node.clone();
            node.control = None;
            self.bake_scope(&mut node);
            vec![node]
        }
    }

    /// Whether `path` is rooted in a live scope var.
    fn scoped(&self, path: &Path) -> bool {
        matches!(path.0.first(), Some(PathSegment::Field(name))
            if self.scopes.iter().rev().any(|(n, _)| n == name))
    }

    /// Rewrites scope-rooted bindings to their resolved literals across the
    /// node's whole subtree. Props, style, content, events and nested control
    /// sources all bake, so expanded clones render with no live scope.
    fn bake_scope(&self, node: &mut Node) {
        if self.scopes.is_empty() {
            return;
        }
        for (_, v) in &mut node.props {
            self.bake_value(v);
        }
        for (_, v) in &mut node.style {
            self.bake_value(v);
        }
        if let Some(content) = &mut node.content {
            self.bake_value(content);
        }
        for event in &mut node.events {
            for arg in &mut event.handler.args {
                self.bake_value(arg);
            }
        }
        for child in &mut node.children {
            self.bake_scope(child);
        }
        match &mut node.control {
            Some(Control::If {
                when,
                test,
                otherwise,
            }) => {
                self.bake_value(when);
                if let Some(Compare::Is(v) | Compare::Not(v)) = test {
                    self.bake_value(v);
                }
                for child in otherwise {
                    self.bake_scope(child);
                }
            }
            Some(Control::For {
                source, otherwise, ..
            }) => {
                self.bake_value(source);
                for child in otherwise {
                    self.bake_scope(child);
                }
            }
            Some(Control::With { source, .. }) => self.bake_value(source),
            Some(Control::Match {
                source,
                cases,
                otherwise,
            }) => {
                self.bake_value(source);
                for case in cases {
                    self.bake_value(&mut case.value);
                    for child in &mut case.children {
                        self.bake_scope(child);
                    }
                }
                for child in otherwise {
                    self.bake_scope(child);
                }
            }
            None => {}
        }
    }

    fn bake_value(&self, value: &mut Value) {
        match value {
            Value::Binding { path, .. } if self.scoped(path) => {
                *value = literal(self.lookup(path).cloned().unwrap_or(Json::Null));
            }
            Value::Interpolated(segments)
                if segments
                    .iter()
                    .any(|s| matches!(s, Segment::Binding(p) if self.scoped(p))) =>
            {
                let text = self.text_of(value);
                *value = Value::String(text);
            }
            Value::List(items) => {
                for item in items {
                    self.bake_value(item);
                }
            }
            Value::Map(entries) => {
                for (_, v) in entries {
                    self.bake_value(v);
                }
            }
            _ => {}
        }
    }

    /// A `style:` color value → ratatui `Color`. `{$theme.*}` tokens hit
    /// the palette. `Value::Color` and `#hex` parse directly, ANSI names map
    /// to the 16 named colors. Bindings resolve then parse the same way.
    fn style_color(&self, value: &Value) -> Option<Color> {
        let named = |s: &str| {
            ansi_color(s)
                .or_else(|| forge_spec::Color::parse_hex(s).map(|c| Color::Rgb(c.r, c.g, c.b)))
                .or_else(|| self.theme.get(s).copied())
        };
        match value {
            Value::Color(c) => Some(Color::Rgb(c.r, c.g, c.b)),
            Value::Token(path) => self.theme_color(path),
            Value::String(s) | Value::Enum(s) => named(s),
            Value::Binding { path, .. } => self.lookup(path).and_then(Json::as_str).and_then(named),
            _ => None,
        }
    }

    /// `{$theme.<name>}` → palette `Color` — the last path segment names the
    /// semantic slot (`accent`, `muted_foreground`, …).
    fn theme_color(&self, path: &Path) -> Option<Color> {
        let name = match path.0.last() {
            Some(PathSegment::Field(n)) => n.as_str(),
            _ => return None,
        };
        self.theme
            .get(name)
            .or_else(|| match name {
                "muted_foreground" => self.theme.get("muted-foreground"),
                "muted-foreground" => self.theme.get("muted_foreground"),
                _ => None,
            })
            .copied()
    }

    /// A `style:` entry's truth — `true`/`yes`/`1` strings count.
    fn truthy(&self, value: &Value) -> bool {
        match value {
            Value::Bool(b) => *b,
            Value::String(s) | Value::Enum(s) => matches!(s.as_str(), "true" | "yes" | "1"),
            Value::Binding { path, .. } => self.lookup(path).map(json_truthy).unwrap_or(false),
            _ => false,
        }
    }

    /// `style:` entries → a `Style` patch. `color`/`background` resolve
    /// through `style_color`, `weight`/`italic`/`underline`/`dim` set
    /// modifiers. Patch it over a widget's default style.
    fn node_style(&self, node: &Node) -> Style {
        let mut style = Style::default();
        for (key, value) in &node.style {
            match key.as_str() {
                "color" => {
                    if let Some(c) = self.style_color(value) {
                        style = style.fg(c);
                    }
                }
                "background" => {
                    if let Some(c) = self.style_color(value) {
                        style = style.bg(c);
                    }
                }
                "weight" if self.text_of(value) == "bold" => {
                    style = style.add_modifier(Modifier::BOLD)
                }
                "italic" if self.truthy(value) => style = style.add_modifier(Modifier::ITALIC),
                "underline" if self.truthy(value) => {
                    style = style.add_modifier(Modifier::UNDERLINED)
                }
                "dim" if self.truthy(value) => style = style.add_modifier(Modifier::DIM),
                _ => {}
            }
        }
        style
    }

    /// `border-color` from `style:` — blocks apply it to `border_style`.
    fn border_color(&self, node: &Node) -> Option<Color> {
        node.style
            .iter()
            .find(|(k, _)| k == "border-color")
            .and_then(|(_, v)| self.style_color(v))
    }

    /// Mouse hit-test — `kind` is `down`, `up`, `scroll-up` or `scroll-down`.
    /// A click focuses the zone's widget. Bool zones toggle, select zones
    /// pick the clicked row and scroll moves the selection.
    pub fn on_mouse(&mut self, kind: &str, col: u16, row: u16) {
        let pos = Position::new(col, row);
        let Some(i) = self
            .focusables
            .iter()
            .position(|(_, _, _, r)| r.contains(pos))
        else {
            return;
        };
        self.focused = i;
        let (path, focus_kind, _, rect) = self.focusables[i].clone();
        match (focus_kind, kind) {
            (FocusKind::Bool, "down") => {
                let cur = self.lookup(&path).and_then(Json::as_bool).unwrap_or(false);
                self.write(&path, Json::Bool(!cur));
                self.fire_focused("change");
            }
            (FocusKind::Select, "down") => {
                let idx = row.saturating_sub(rect.y + 1);
                self.write(&path, Json::from(idx));
                self.fire_focused("select");
            }
            (FocusKind::Select, "scroll-up") | (FocusKind::Select, "scroll-down") => {
                let cur = self.lookup(&path).and_then(Json::as_u64).unwrap_or(0);
                let next = if kind == "scroll-up" {
                    cur.saturating_sub(1)
                } else {
                    cur + 1
                };
                self.write(&path, Json::from(next));
                self.fire_focused("select");
            }
            _ => {}
        }
    }

    /// The deepest node under a cell — the smallest containing zone wins,
    /// so a drop targets the inner widget, not its frame.
    fn zone_at(&self, col: u16, row: u16) -> Option<(String, Rect)> {
        let pos = Position::new(col, row);
        self.zones
            .iter()
            .filter(|(_, r)| r.contains(pos))
            .min_by_key(|(_, r)| r.area() as u32)
            .map(|(p, r)| (p.clone(), *r))
    }

    /// The doc path of the deepest node under a cell — the drag source
    /// the sender reports on `down` (#156).
    pub fn zone_path_at(&self, col: u16, row: u16) -> Option<String> {
        self.zone_at(col, row).map(|(p, _)| p)
    }

    /// A streamed `drop-hover` — marks the zone the next frame highlights
    /// and returns its doc path for the sender to echo.
    pub fn drop_hover(&mut self, col: u16, row: u16) -> Option<String> {
        let found = self.zone_at(col, row);
        self.drop_target = found.clone();
        found.map(|(p, _)| p)
    }

    /// A streamed `drop` — resolves the target cell to a doc path and
    /// clears the highlight. `None` means the drop missed every zone.
    pub fn drop_at(&mut self, col: u16, row: u16) -> Option<String> {
        let found = self.zone_at(col, row).map(|(p, _)| p);
        self.drop_target = None;
        found
    }

    // ── rendering ────────────────────────────────────────────────────────────

    /// Draws the document into `frame` — called by the host's event loop.
    pub fn render(&mut self, frame: &mut Frame) {
        self.focusables.clear();
        self.zones.clear();
        let area = frame.area();
        if let Some(doc) = self.doc.clone() {
            for node in self.expand(&doc.root) {
                self.render_node(&node, "0", area, frame);
            }
        } else if let Some(text) = self.empty_text.clone() {
            frame.render_widget(
                Paragraph::new(text)
                    .style(Style::default().fg(Color::DarkGray))
                    .wrap(Wrap { trim: true }),
                area,
            );
        }
        // The sender's drop hover — an accent frame over the target zone,
        // the TUI twin of the gpui canvas's accent outline.
        if let Some((_, r)) = self.drop_target {
            frame.render_widget(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
                r,
            );
        }
    }

    fn render_node(&mut self, node: &Node, path: &str, area: Rect, frame: &mut Frame) {
        // Map doc path → drawn rect so the sender resolves pointer cells
        // to drop targets and drag sources.
        self.zones.push((path.to_owned(), area));
        if node.element.namespace.as_deref() == Some("forge") {
            if node.element.name == "Preview" {
                let id = self.prop_text(node, "id");
                let id = if id.is_empty() {
                    "preview".to_owned()
                } else {
                    id
                };
                if let Some(view) = self.previews.get_mut(&id) {
                    let doc = view.doc.clone();
                    if let Some(doc) = doc {
                        for node in view.expand(&doc.root) {
                            view.render_node(&node, "0", area, frame);
                        }
                    }
                }
            }
            return;
        }
        if let Some(ns) = node.element.namespace.as_deref() {
            if let Some(lib) = self.libraries.iter().find(|l| l.alias() == ns).cloned()
                && lib.render(node, area, self, frame)
            {
                return;
            }
            // No library claimed `ns.Name` — the placeholder keeps the gap
            // visible instead of dropping it.
            frame.render_widget(
                Paragraph::new(format!("{ns}.{} — no ratatui library", node.element.name))
                    .style(Style::default().fg(Color::DarkGray))
                    .block(Block::default().borders(Borders::ALL)),
                area,
            );
            return;
        }
        match node.element.name.as_str() {
            "VStack" => self.render_children(node, path, area, frame, Direction::Vertical),
            "HStack" => self.render_children(node, path, area, frame, Direction::Horizontal),
            "Block" | "Paper" | "Card" => {
                let title = self.prop_text(node, "title");
                let mut block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded);
                if let Some(c) = self.border_color(node) {
                    block = block.border_style(c);
                }
                if !title.is_empty() {
                    block = block.title(title);
                }
                let inner = block.inner(area);
                frame.render_widget(block, area);
                if node.children.is_empty() {
                    let text = self.node_text(node);
                    if !text.is_empty() {
                        frame.render_widget(
                            Paragraph::new(text).style(self.node_style(node)),
                            inner,
                        );
                    }
                } else {
                    self.render_children(node, path, inner, frame, Direction::Vertical);
                }
            }
            "Text" | "Title" => {
                let mut style = Style::default().patch(self.node_style(node));
                if node.element.name == "Title" {
                    style = style.add_modifier(Modifier::BOLD);
                }
                frame.render_widget(
                    Paragraph::new(Line::from(Span::styled(self.node_text(node), style)))
                        .wrap(Wrap { trim: true }),
                    area,
                );
            }
            "Divider" => {
                frame.render_widget(
                    Block::default().borders(Borders::TOP),
                    Rect { height: 1, ..area },
                );
            }
            "Clear" => {
                frame.render_widget(Clear, area);
            }
            "Button" => {
                let label = format!("[ {} ]", self.node_text(node));
                frame.render_widget(
                    Paragraph::new(label).style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                            .patch(self.node_style(node)),
                    ),
                    area,
                );
            }
            "List" | "Select" | "Tabs" => {
                let binding = Self::binding_of(node, "selected");
                if let Some(path) = &binding {
                    let calls = self.calls_for(node);
                    self.focusables
                        .push((path.clone(), FocusKind::Select, calls, area));
                }
                let focused = binding
                    .as_ref()
                    .map(|p| self.is_focused(p))
                    .unwrap_or(false);
                if node.element.name == "Tabs" {
                    let titles: Vec<Line> = if node.children.is_empty() {
                        self.prop_text(node, "titles")
                            .lines()
                            .map(|l| Line::from(l.to_string()))
                            .collect()
                    } else {
                        self.expand_kids(&node.children)
                            .iter()
                            .map(|c| Line::from(self.node_text(c)))
                            .collect()
                    };
                    let selected = self.num(node, "selected", 0.0) as usize;
                    let mut tabs = Tabs::new(titles)
                        .select(selected)
                        .block(Block::default().borders(Borders::ALL));
                    if focused {
                        tabs = tabs.highlight_style(
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        );
                    }
                    frame.render_widget(tabs, area);
                } else {
                    let items: Vec<ListItem> = if node.children.is_empty() {
                        self.prop_text(node, "items")
                            .lines()
                            .map(|l| ListItem::new(l.to_string()))
                            .collect()
                    } else {
                        self.expand_kids(&node.children)
                            .iter()
                            .map(|c| ListItem::new(self.node_text(c)))
                            .collect()
                    };
                    let title = self.prop_text(node, "title");
                    let mut list =
                        List::new(items).block(Block::default().borders(Borders::ALL).title(title));
                    if node.element.name == "Select" {
                        list = list.highlight_symbol("> ");
                    }
                    if focused {
                        list = list.highlight_style(
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        );
                    }
                    list = list.style(self.node_style(node));
                    frame.render_widget(list, area);
                }
            }
            "Gauge" | "Progress" => {
                let value = self.num(node, "value", 0.0).clamp(0.0, 100.0);
                let label = self.prop_text(node, "label");
                frame.render_widget(
                    Gauge::default()
                        .block({
                            let mut b = Block::default().borders(Borders::ALL);
                            if let Some(c) = self.border_color(node) {
                                b = b.border_style(c);
                            }
                            b
                        })
                        .gauge_style(
                            Style::default()
                                .fg(Color::Green)
                                .patch(self.node_style(node)),
                        )
                        .label(if label.is_empty() {
                            format!("{value:.0}%")
                        } else {
                            label
                        })
                        .percent(value as u16),
                    Rect {
                        height: 3.min(area.height),
                        ..area
                    },
                );
            }
            "Input" | "TextInput" => {
                let label = self.prop_text(node, "label");
                let placeholder = self.prop_text(node, "placeholder");
                let value = self.prop_text(node, "value");
                let binding = Self::binding_of(node, "value");
                let empty = value.is_empty();
                let text = if empty { placeholder } else { value };
                let drawn = Rect {
                    height: 3.min(area.height),
                    ..area
                };
                if let Some(path) = &binding {
                    let calls = self.calls_for(node);
                    self.focusables
                        .push((path.clone(), FocusKind::Input, calls, drawn));
                }
                let focused = binding
                    .as_ref()
                    .map(|p| self.is_focused(p))
                    .unwrap_or(false);
                let mut block = Block::default().borders(Borders::ALL);
                if !label.is_empty() {
                    block = block.title(label);
                }
                if let Some(c) = self.border_color(node) {
                    block = block.border_style(c);
                }
                if focused {
                    block = block
                        .border_style(Style::default().fg(Color::Cyan))
                        .title("▸ edit");
                }
                frame.render_widget(
                    Paragraph::new(if text.is_empty() { " " } else { text.as_str() })
                        .block(block)
                        .style(
                            if empty {
                                Style::default().fg(Color::DarkGray)
                            } else {
                                Style::default()
                            }
                            .patch(self.node_style(node)),
                        ),
                    drawn,
                );
            }
            "Checkbox" | "Switch" => {
                let binding = Self::binding_of(node, "checked");
                if let Some(path) = &binding {
                    let calls = self.calls_for(node);
                    self.focusables
                        .push((path.clone(), FocusKind::Bool, calls, area));
                }
                let focused = binding
                    .as_ref()
                    .map(|p| self.is_focused(p))
                    .unwrap_or(false);
                let checked = match node.props.iter().find(|(k, _)| k == "checked") {
                    Some((_, Value::Bool(b))) => *b,
                    Some((_, Value::Binding { path, .. })) => {
                        self.lookup(path).and_then(Json::as_bool).unwrap_or(false)
                    }
                    _ => false,
                };
                let label = self.node_text(node);
                let mut p =
                    Paragraph::new(format!("{} {label}", if checked { "[x]" } else { "[ ]" }))
                        .style(self.node_style(node));
                if focused {
                    p = p.style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    );
                }
                frame.render_widget(p, area);
            }
            "Table" => {
                let title = self.prop_text(node, "title");
                let cell = |s: &str| ratatui::widgets::Cell::from(s.trim().to_string());
                let header = self
                    .prop_text(node, "header")
                    .split('|')
                    .map(cell)
                    .collect::<Vec<_>>();
                let rows: Vec<ratatui::widgets::Row> = self
                    .prop_text(node, "rows")
                    .lines()
                    .map(|line| {
                        ratatui::widgets::Row::new(line.split('|').map(cell).collect::<Vec<_>>())
                    })
                    .collect();
                let widths = vec![Constraint::Fill(1); header.len().max(1)];
                frame.render_widget(
                    ratatui::widgets::Table::new(rows, widths)
                        .header(
                            ratatui::widgets::Row::new(header)
                                .style(Style::default().add_modifier(Modifier::BOLD)),
                        )
                        .block(Block::default().borders(Borders::ALL).title(title)),
                    area,
                );
            }
            "Sparkline" => {
                let values: Vec<u64> = self
                    .floats(node, "data")
                    .iter()
                    .map(|v| v.max(0.0) as u64)
                    .collect();
                let mut spark = Sparkline::default().data(&values).style(
                    Style::default()
                        .fg(Color::Yellow)
                        .patch(self.node_style(node)),
                );
                let max = self.num(node, "max", 0.0);
                if max > 0.0 {
                    spark = spark.max(max as u64);
                }
                frame.render_widget(spark, area);
            }
            "BarChart" => {
                let data = self.floats(node, "data");
                let declared = self.prop_text(node, "labels");
                let labels: Vec<String> = if declared.is_empty() {
                    (0..data.len()).map(|i| i.to_string()).collect()
                } else {
                    declared.lines().map(|l| l.to_string()).collect()
                };
                let entries: Vec<(&str, u64)> = data
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        (
                            labels.get(i).map(String::as_str).unwrap_or(""),
                            v.max(0.0) as u64,
                        )
                    })
                    .collect();
                let mut chart = BarChart::default().data(&entries).bar_style(
                    Style::default()
                        .fg(Color::Green)
                        .patch(self.node_style(node)),
                );
                let width = self.num(node, "bar_width", 0.0);
                if width > 0.0 {
                    chart = chart.bar_width(width as u16);
                }
                let max = self.num(node, "max", 0.0);
                if max > 0.0 {
                    chart = chart.max(max as u64);
                }
                frame.render_widget(chart, area);
            }
            "Chart" => {
                let data = self.floats(node, "data");
                let points: Vec<(f64, f64)> = data
                    .iter()
                    .enumerate()
                    .map(|(i, v)| (i as f64, *v))
                    .collect();
                let (mut y_lo, mut y_hi) = data
                    .iter()
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                        (lo.min(*v), hi.max(*v))
                    });
                if y_lo >= y_hi {
                    (y_lo, y_hi) = (0.0, 1.0);
                }
                let x_hi = data.len().saturating_sub(1).max(1) as f64;
                let name = self.prop_text(node, "name");
                let mut ds = Dataset::default()
                    .marker(symbols::Marker::Braille)
                    .graph_type(GraphType::Line)
                    .style(
                        Style::default()
                            .fg(Color::Cyan)
                            .patch(self.node_style(node)),
                    )
                    .data(&points);
                if !name.is_empty() {
                    ds = ds.name(name);
                }
                frame.render_widget(
                    Chart::new(vec![ds])
                        .x_axis(
                            Axis::default().bounds([
                                self.num(node, "x_min", 0.0),
                                self.num(node, "x_max", x_hi),
                            ]),
                        )
                        .y_axis(Axis::default().bounds([
                            self.num(node, "y_min", y_lo),
                            self.num(node, "y_max", y_hi),
                        ])),
                    area,
                );
            }
            "Calendar" => {
                let declared = self.prop_text(node, "date");
                let date =
                    parse_date(&declared).unwrap_or_else(|| time::OffsetDateTime::now_utc().date());
                let events = CalendarEventStore::today(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                );
                let header = Style::default().add_modifier(Modifier::BOLD);
                frame.render_widget(
                    Monthly::new(date, events)
                        .show_month_header(header)
                        .show_weekdays_header(header)
                        .block(Block::default().borders(Borders::ALL)),
                    area,
                );
            }
            "Canvas" => {
                let map = self.boolean(node, "map", true);
                frame.render_widget(
                    Canvas::default()
                        .x_bounds([
                            self.num(node, "x_min", -180.0),
                            self.num(node, "x_max", 180.0),
                        ])
                        .y_bounds([
                            self.num(node, "y_min", -90.0),
                            self.num(node, "y_max", 90.0),
                        ])
                        .paint(move |ctx| {
                            if map {
                                ctx.draw(&Map {
                                    resolution: MapResolution::High,
                                    color: Color::White,
                                });
                            }
                        }),
                    area,
                );
            }
            "Logo" => {
                let logo = if self.prop_text(node, "size") == "tiny" {
                    RatatuiLogo::tiny()
                } else {
                    RatatuiLogo::small()
                };
                frame.render_widget(logo, area);
            }
            _ => {
                frame.render_widget(
                    Paragraph::new(format!("{} — no ratatui renderer", node.element.name))
                        .style(Style::default().fg(Color::DarkGray)),
                    area,
                );
            }
        }
    }

    /// A style entry's numeric value and unit — `"40px"`, `30%`, `2fr` or a
    /// bare number all parse. The unit picks the `Constraint` kind.
    fn style_num(entries: &forge_spec::Entries, name: &str) -> Option<(f64, Option<Unit>)> {
        entries
            .iter()
            .find(|(k, _)| k == name)
            .and_then(|(_, v)| match v {
                Value::Number { value, unit } => Some((*value, *unit)),
                Value::String(s) => {
                    let s = s.trim();
                    let (num, unit) = if let Some(n) = s.strip_suffix("px") {
                        (n, Some(Unit::Px))
                    } else if let Some(n) = s.strip_suffix('%') {
                        (n, Some(Unit::Percent))
                    } else if let Some(n) = s.strip_suffix("fr") {
                        (n, Some(Unit::Fr))
                    } else {
                        (s, None)
                    };
                    num.trim().parse::<f64>().ok().map(|v| (v, unit))
                }
                _ => None,
            })
    }

    /// The `Constraint` one child earns along `dir`: an explicit `width` or
    /// `height` fixes it. `min`/`max` bound it and `flex` shares the
    /// leftover. A child with none of them takes an equal `Fill` share.
    fn child_constraint(node: &Node, dir: Direction) -> Constraint {
        let (size, min, max) = if dir == Direction::Horizontal {
            ("width", "min-width", "max-width")
        } else {
            ("height", "min-height", "max-height")
        };
        let fill = |n: f64| Constraint::Fill(n.max(1.0) as u16);
        if let Some((v, unit)) = Self::style_num(&node.style, size) {
            return match unit {
                Some(Unit::Percent) => Constraint::Percentage(v as u16),
                Some(Unit::Fr) => fill(v),
                _ => Constraint::Length(v as u16),
            };
        }
        if let Some((v, _)) = Self::style_num(&node.style, min) {
            return Constraint::Min(v as u16);
        }
        if let Some((v, _)) = Self::style_num(&node.style, max) {
            return Constraint::Max(v as u16);
        }
        if let Some((v, _)) = Self::style_num(&node.style, "flex") {
            return fill(v);
        }
        Constraint::Fill(1)
    }

    /// Splits `area` into one chunk per child along `direction` and renders
    /// each child, honoring `width`/`height`/`flex`/`min-*`/`max-*` style
    /// entries — children without one take an equal `Fill` share.
    fn render_children(
        &mut self,
        node: &Node,
        path: &str,
        area: Rect,
        frame: &mut Frame,
        dir: Direction,
    ) {
        let expanded = self.expand_kids(&node.children);
        let constraints: Vec<Constraint> = expanded
            .iter()
            .map(|c| Self::child_constraint(c, dir))
            .collect();
        let chunks = Layout::default()
            .direction(dir)
            .constraints(constraints)
            .split(area);
        for (i, (child, chunk)) in expanded.iter().zip(chunks.iter()).enumerate() {
            self.render_node(child, &format!("{path}.{i}"), *chunk, frame);
        }
    }
}

/// `if:`'s truthiness — numbers, strings, arrays and nulls all have a
/// sense in the doc language (the same contract as the GPUI twin).
fn json_truthy(v: &Json) -> bool {
    match v {
        Json::Bool(b) => *b,
        Json::Null => false,
        Json::Number(n) => n.as_f64() != Some(0.0),
        Json::String(s) => !s.is_empty(),
        Json::Array(a) => !a.is_empty(),
        Json::Object(o) => !o.is_empty(),
    }
}

/// A resolved `Json` back to a `Value` literal — the shape `bake_scope`
/// writes when a scope binding becomes concrete.
fn literal(v: Json) -> Value {
    match v {
        Json::Bool(b) => Value::Bool(b),
        Json::Number(n) => Value::Number {
            value: n.as_f64().unwrap_or_default(),
            unit: None,
        },
        Json::Array(items) => Value::List(items.into_iter().map(literal).collect()),
        Json::Object(map) => Value::Map(map.into_iter().map(|(k, v)| (k, literal(v))).collect()),
        _ => Value::String(v.as_str().unwrap_or_default().to_owned()),
    }
}

/// ANSI color names → ratatui `Color`. Keeps the GPUI base's named-hue set
/// where a terminal color exists.
fn ansi_color(word: &str) -> Option<Color> {
    Some(match word {
        "black" => Color::Black,
        "white" => Color::White,
        "red" => Color::Red,
        "orange" => Color::LightRed,
        "yellow" => Color::Yellow,
        "green" => Color::Green,
        "teal" | "cyan" => Color::Cyan,
        "blue" | "indigo" => Color::Blue,
        "magenta" | "purple" | "violet" => Color::Magenta,
        "pink" => Color::LightMagenta,
        "gray" | "grey" => Color::Gray,
        "darkgray" | "dark_gray" | "dark-grey" => Color::DarkGray,
        _ => return None,
    })
}

/// `date` prop — `"YYYY-MM"` or `"YYYY-MM-DD"`. Day defaults to the 1st.
fn parse_date(text: &str) -> Option<time::Date> {
    let mut parts = text.split('-');
    let year: i32 = parts.next()?.parse().ok()?;
    let month: u8 = parts.next()?.parse().ok()?;
    let day: u8 = parts.next().map_or(Ok(1), |d| d.parse()).ok()?;
    time::Date::from_calendar_date(year, time::Month::try_from(month).ok()?, day).ok()
}

/// The interactivity a focused element accepts — Input = text editing,
/// Bool = toggle (Checkbox/Switch), Select = arrow navigation (List/Tabs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusKind {
    Input,
    Bool,
    Select,
}

/// A resolved `on <event>:` call — `(event name, handler name, args)`.
/// Args resolve at render time, so a fired call sees the document as of
/// the last draw.
type Calls = Vec<(String, String, Vec<String>)>;
