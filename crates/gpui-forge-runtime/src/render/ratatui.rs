//! `ForgeView` for the ratatui base — a terminal-UI view (#112).
//!
//! Same host contract as the GPUI twins (document/state/hooks/sinks, JSON
//! state paths, `fire_call`) minus the GPUI machinery: there is no
//! `Context<Entity>` in a crossterm event loop, so sinks take `&mut ForgeView`
//! directly and `render` draws into a `ratatui::Frame`. The host
//! (`forge-preview-ratatui`) owns the terminal.

use crate::render::{doc_meta, lookup_path, set_path, style_edit_value};
use ansi_to_tui::IntoText;
use forge_spec::catalog::Catalog;
use forge_spec::{Compare, Control, Document, Node, Path, PathSegment, Segment, Unit, Value};
use ratatui::Frame;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Size;
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::calendar::{CalendarEventStore, Monthly};
use ratatui::widgets::canvas::{Canvas, Map, MapResolution};
use ratatui::widgets::{
    Axis, BarChart, Block, BorderType, Borders, Chart, Clear, Dataset, Fill, Gauge, GraphType,
    LineGauge, List, ListItem, Paragraph, RatatuiLogo, RatatuiMascot, Scrollbar,
    ScrollbarOrientation, ScrollbarState, Sparkline, Tabs, Wrap,
};
// Native-only deps (Cargo.toml target block): tree-sitter grammars and
// ratatui's crossterm backend have no wasm32 build. On the web canvas the
// arms below fall through to the placeholder.
use ratatui::buffer::Buffer as TuiBuffer;
use ratatui::widgets::WidgetRef;
#[cfg(not(target_arch = "wasm32"))]
use ratatui_code_editor::editor::Editor as CodeEditorWidget;
#[cfg(not(target_arch = "wasm32"))]
use ratatui_markdown::ThemeConfig;
#[cfg(not(target_arch = "wasm32"))]
use ratatui_markdown::markdown::MarkdownRenderer;
use ratatui_toaster::{Toast, ToastType};
use serde_json::Value as Json;
use std::collections::HashMap;
use std::rc::Rc;
use throbber_widgets_tui::symbols::throbber::{self, WhichUse};
use throbber_widgets_tui::{Throbber, ThrobberState};
#[cfg(not(target_arch = "wasm32"))]
use tui_big_text::{BigText, PixelSize};
#[cfg(not(target_arch = "wasm32"))]
use tui_piechart::{PieChart, PieSlice};
use tui_scrollview::{ScrollView, ScrollViewState, ScrollbarVisibility};
#[cfg(not(target_arch = "wasm32"))]
use tui_slider::Slider;
use tui_tree_widget::{Tree as TuiTree, TreeItem, TreeState};

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
    /// `binding path → scroll limits` for Scroll focusables — rebuilt each
    /// render alongside `focusables` so stale maxima cannot linger. A Vec
    /// because `Path` has no `Hash` — scroll regions are few.
    scroll_meta: Vec<(Path, ScrollMeta)>,
    /// `binding path → range` for Slider focusables — rebuilt each render
    /// with `focusables`. Keys and pointer cells resolve against it.
    sliders: Vec<(Path, SliderMeta)>,
    /// `bound path string → yes_selected` — the dialog's highlighted button
    /// persists between renders. Keyed by string like `trees`.
    dialog_sel: HashMap<String, bool>,
    /// Every rendered node's `(doc path, drawn rect)`, rebuilt each render.
    /// The sender resolves pointer cells to doc paths through it for the
    /// designer's drop-hover and drop (#156).
    zones: Vec<(String, Rect)>,
    /// The highlighted drop target — `(doc path, rect)` — painted as an
    /// accent frame over the zone while a drag hovers it.
    drop_target: Option<(String, Rect)>,
    focused: usize,
    /// `bound path string → TreeState` — expansion and selection persist
    /// between renders. Keyed by string because `Path` has no `Hash`.
    trees: HashMap<String, TreeState<String>>,
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
    /// The canvas flag — a `dnd` view paints `document.selected` as an accent
    /// frame over the node's zone (the gpui canvas's selection outline).
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
            scroll_meta: Vec::new(),
            sliders: Vec::new(),
            dialog_sel: HashMap::new(),
            zones: Vec::new(),
            drop_target: None,
            focused: 0,
            trees: HashMap::new(),
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

    /// Arrow handling for a focused `Tree` — the `TreeState` methods move
    /// the selection or fold/unfold a node, then the bound `selected` path
    /// gets the slash-joined identifier list for `if:`/`Text:` reads.
    fn tree_step(&mut self, key: &str) {
        let Some((path, ..)) = self.focusables.get(self.focused).cloned() else {
            return;
        };
        let selected = self.trees.get_mut(&path.to_string()).map(|state| {
            match key {
                "up" => {
                    state.key_up();
                }
                "down" => {
                    state.key_down();
                }
                "left" => {
                    state.key_left();
                }
                "right" => {
                    state.key_right();
                }
                _ => {
                    state.toggle_selected();
                }
            }
            state.selected().join("/")
        });
        if let Some(selected) = selected {
            self.write(&path, Json::String(selected));
        }
    }

    /// Terminal keys — the host reads the tty and forwards them (#112).
    /// `Tab`/`Shift-Tab` cycles focusable inputs. Printable keys write into
    /// the focused input's binding. `Enter`/`Space` fires toggle/select.
    pub fn on_key(&mut self, key: &str) {
        let kind = self.focusables.get(self.focused).map(|(_, k, _, _)| *k);
        match (kind, key) {
            (_, "Tab") | (_, "BackTab") => {}
            (Some(FocusKind::Action), "Enter") | (Some(FocusKind::Action), " ") => {
                self.fire_focused("click");
                return;
            }
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
            (Some(FocusKind::Tree), "Up") | (Some(FocusKind::Tree), "k") => {
                self.tree_step("up");
                self.fire_focused("select");
                return;
            }
            (Some(FocusKind::Tree), "Down") | (Some(FocusKind::Tree), "j") => {
                self.tree_step("down");
                self.fire_focused("select");
                return;
            }
            (Some(FocusKind::Tree), "Left") | (Some(FocusKind::Tree), "h") => {
                self.tree_step("left");
                self.fire_focused("select");
                return;
            }
            (Some(FocusKind::Tree), "Right") | (Some(FocusKind::Tree), "l") => {
                self.tree_step("right");
                self.fire_focused("select");
                return;
            }
            (Some(FocusKind::Tree), "Enter") | (Some(FocusKind::Tree), " ") => {
                self.tree_step("toggle");
                self.fire_focused("select");
                return;
            }
            (Some(FocusKind::Scroll), "Up") | (Some(FocusKind::Scroll), "k") => {
                self.scroll_by(-1);
                return;
            }
            (Some(FocusKind::Scroll), "Down") | (Some(FocusKind::Scroll), "j") => {
                self.scroll_by(1);
                return;
            }
            (Some(FocusKind::Scroll), "PageUp") => {
                self.scroll_by(-10);
                return;
            }
            (Some(FocusKind::Scroll), "PageDown") => {
                self.scroll_by(10);
                return;
            }
            (Some(FocusKind::Scroll), "Home") => {
                self.scroll_to(0);
                return;
            }
            (Some(FocusKind::Scroll), "End") => {
                if let Some((path, _, _, _)) = self.focusables.get(self.focused).cloned() {
                    let max = self
                        .scroll_meta
                        .iter()
                        .find(|(p, _)| p == &path)
                        .map(|(_, m)| m.max)
                        .unwrap_or(0);
                    self.scroll_to(max);
                }
                return;
            }
            (Some(FocusKind::Slider), "Left")
            | (Some(FocusKind::Slider), "h")
            | (Some(FocusKind::Slider), "Down")
            | (Some(FocusKind::Slider), "j") => {
                self.slider_step(-1.0);
                self.fire_focused("change");
                return;
            }
            (Some(FocusKind::Slider), "Right")
            | (Some(FocusKind::Slider), "l")
            | (Some(FocusKind::Slider), "Up")
            | (Some(FocusKind::Slider), "k") => {
                self.slider_step(1.0);
                self.fire_focused("change");
                return;
            }
            (Some(FocusKind::Dialog), "Left")
            | (Some(FocusKind::Dialog), "Right")
            | (Some(FocusKind::Dialog), "h")
            | (Some(FocusKind::Dialog), "l") => {
                if let Some((path, _, _, _)) = self.focusables.get(self.focused).cloned() {
                    let sel = self.dialog_sel.entry(path.to_string()).or_insert(true);
                    *sel = !*sel;
                }
                return;
            }
            (Some(FocusKind::Dialog), "Enter") | (Some(FocusKind::Dialog), " ") => {
                self.dialog_choose();
                return;
            }
            (Some(FocusKind::Dialog), "y") | (Some(FocusKind::Dialog), "Y") => {
                if let Some((path, _, _, _)) = self.focusables.get(self.focused).cloned() {
                    self.dialog_sel.insert(path.to_string(), true);
                }
                self.dialog_choose();
                return;
            }
            (Some(FocusKind::Dialog), "n") | (Some(FocusKind::Dialog), "N") => {
                if let Some((path, _, _, _)) = self.focusables.get(self.focused).cloned() {
                    self.dialog_sel.insert(path.to_string(), false);
                }
                self.dialog_choose();
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
            (FocusKind::Action, "down") => {
                self.fire_focused("click");
            }
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
            (FocusKind::Tree, "down") => {
                let selected = self.trees.get_mut(&path.to_string()).map(|state| {
                    state.click_at(pos);
                    state.selected().join("/")
                });
                if let Some(selected) = selected {
                    self.write(&path, Json::String(selected));
                }
                self.fire_focused("select");
            }
            (FocusKind::Tree, "scroll-up") => {
                self.tree_step("up");
            }
            (FocusKind::Tree, "scroll-down") => {
                self.tree_step("down");
            }
            (FocusKind::Scroll, "scroll-up") | (FocusKind::Scroll, "scroll-down") => {
                self.scroll_by(if kind == "scroll-up" { -1 } else { 1 });
            }
            (FocusKind::Slider, "down") => {
                // A click maps its column to a value, snapped to `step`.
                if let Some(meta) = self
                    .sliders
                    .iter()
                    .find(|(p, _)| p == &path)
                    .map(|(_, m)| *m)
                {
                    let frac = col.saturating_sub(meta.x) as f64 / meta.width.max(1) as f64;
                    let raw = meta.min + frac.clamp(0.0, 1.0) * (meta.max - meta.min);
                    let snapped = (raw / meta.step).round() * meta.step;
                    self.write(&path, Json::from(snapped.clamp(meta.min, meta.max)));
                    self.fire_focused("change");
                }
            }
            (FocusKind::Slider, "scroll-up") => {
                self.slider_step(1.0);
                self.fire_focused("change");
            }
            (FocusKind::Slider, "scroll-down") => {
                self.slider_step(-1.0);
                self.fire_focused("change");
            }
            (FocusKind::Scroll, "down") => {
                // On a track (Scrollbar) a click jumps the thumb to the
                // clicked fraction. On a Log `down` only takes focus.
                if let Some(meta) = self
                    .scroll_meta
                    .iter()
                    .find(|(p, _)| p == &path)
                    .map(|(_, m)| *m)
                    && meta.track
                {
                    let frac = if meta.horizontal {
                        col.saturating_sub(rect.x) as f64
                            / rect.width.saturating_sub(1).max(1) as f64
                    } else {
                        row.saturating_sub(rect.y) as f64
                            / rect.height.saturating_sub(1).max(1) as f64
                    };
                    let pos = (frac.clamp(0.0, 1.0) * meta.max as f64).round() as u64;
                    self.write(&path, Json::from(pos));
                    self.fire_focused("change");
                }
            }
            _ => {}
        }
    }

    /// Moves the focused scroll binding by `delta`, clamped to its `max`.
    fn scroll_by(&mut self, delta: i64) {
        if let Some((path, FocusKind::Scroll, _, _)) = self.focusables.get(self.focused).cloned() {
            let cur = self.lookup(&path).and_then(Json::as_u64).unwrap_or(0);
            let max = self
                .scroll_meta
                .iter()
                .find(|(p, _)| p == &path)
                .map(|(_, m)| m.max)
                .unwrap_or(u64::MAX);
            let next = if delta < 0 {
                cur.saturating_sub(delta.unsigned_abs())
            } else {
                cur.saturating_add(delta as u64).min(max)
            };
            self.write(&path, Json::from(next));
            self.fire_focused("change");
        }
    }

    /// Sets the focused scroll binding to an absolute offset.
    fn scroll_to(&mut self, pos: u64) {
        if let Some((path, FocusKind::Scroll, _, _)) = self.focusables.get(self.focused).cloned() {
            self.write(&path, Json::from(pos));
            self.fire_focused("change");
        }
    }

    /// Moves the focused slider by `dir` × `step`, clamped to its range.
    fn slider_step(&mut self, dir: f64) {
        if let Some((path, FocusKind::Slider, _, _)) = self.focusables.get(self.focused).cloned() {
            let meta = self
                .sliders
                .iter()
                .find(|(p, _)| p == &path)
                .map(|(_, m)| *m)
                .unwrap_or(SliderMeta {
                    min: 0.0,
                    max: 100.0,
                    step: 1.0,
                    x: 0,
                    width: 1,
                });
            let cur = self
                .lookup(&path)
                .and_then(Json::as_f64)
                .unwrap_or(meta.min);
            self.write(
                &path,
                Json::from((cur + dir * meta.step).clamp(meta.min, meta.max)),
            );
        }
    }

    /// Writes the dialog's highlighted button (`"yes"`/`"no"`) to the
    /// `choice` binding, then fires `choose`.
    fn dialog_choose(&mut self) {
        if let Some((path, _, _, _)) = self.focusables.get(self.focused).cloned() {
            let yes = self
                .dialog_sel
                .get(&path.to_string())
                .copied()
                .unwrap_or(true);
            self.write(
                &path,
                Json::String(if yes { "yes" } else { "no" }.to_owned()),
            );
        }
        self.fire_focused("choose");
    }

    /// The deepest node under a cell — the smallest containing zone wins,
    /// so a drop targets the inner widget, not its frame.
    fn zone_at(&self, col: u16, row: u16) -> Option<(String, Rect)> {
        let pos = Position::new(col, row);
        self.zones
            .iter()
            .filter(|(_, r)| r.contains(pos))
            .min_by_key(|(_, r)| r.area())
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
        self.scroll_meta.clear();
        self.sliders.clear();
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
        // `document.selected` — the designer's selected node gets the same
        // treatment, only where a canvas flagged `dnd` (#156).
        if self.dnd
            && let Ok(path) = forge_spec::value::parse_path("document.selected")
            && let Some(selected) = self.lookup(&path).and_then(Json::as_str)
            && let Some((_, r)) = self.zones.iter().find(|(p, _)| p == selected)
        {
            frame.render_widget(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
                *r,
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
                let mut p = Paragraph::new(Line::from(Span::styled(self.node_text(node), style)));
                if self.prop_text(node, "wrap") != "false" {
                    p = p.wrap(Wrap { trim: true });
                }
                frame.render_widget(p, area);
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
                // Buttons take focus by doc path — `on click:` fires on
                // Enter/Space or a cell click, like the gpui twin.
                let node_path = Path(
                    path.split('.')
                        .filter_map(|s| s.parse::<usize>().ok().map(PathSegment::Index))
                        .collect(),
                );
                let calls = self.calls_for(node);
                self.focusables
                    .push((node_path.clone(), FocusKind::Action, calls, area));
                let focused = self.is_focused(&node_path);
                let label = format!("[ {} ]", self.node_text(node));
                let mut style = Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
                    .patch(self.node_style(node));
                if focused {
                    style = style.fg(Color::Black).bg(Color::Cyan);
                }
                frame.render_widget(Paragraph::new(label).style(style), area);
            }
            "List" | "Select" | "Tabs" | "Menu" => {
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
                    if matches!(node.element.name.as_str(), "Select" | "Menu") {
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
            "LineGauge" => {
                let value = self.num(node, "value", 0.0).clamp(0.0, 100.0);
                let label = self.prop_text(node, "label");
                let set = match self.prop_text(node, "set").as_str() {
                    "thick" => symbols::line::THICK,
                    "double" => symbols::line::DOUBLE,
                    _ => symbols::line::NORMAL,
                };
                frame.render_widget(
                    LineGauge::default()
                        .block({
                            let mut b = Block::default().borders(Borders::ALL);
                            if let Some(c) = self.border_color(node) {
                                b = b.border_style(c);
                            }
                            b
                        })
                        .filled_symbol(set.horizontal)
                        .unfilled_symbol(set.horizontal)
                        .filled_style(
                            Style::default()
                                .fg(Color::Green)
                                .patch(self.node_style(node)),
                        )
                        .label(if label.is_empty() {
                            format!("{value:.0}%")
                        } else {
                            label
                        })
                        .ratio(value / 100.0),
                    area,
                );
            }
            "Scrollbar" => {
                let orient = match self.prop_text(node, "orientation").as_str() {
                    "horizontal" => ScrollbarOrientation::HorizontalBottom,
                    _ => ScrollbarOrientation::VerticalRight,
                };
                let content = self.num(node, "content_length", 10.0).max(1.0) as usize;
                let pos = self.num(node, "position", 0.0).max(0.0) as usize;
                // A bound `position` makes the track interactive — the wheel
                // steps it, a track click jumps to the clicked fraction.
                if let Some(path) = Self::binding_of(node, "position") {
                    let calls = self.calls_for(node);
                    self.focusables
                        .push((path.clone(), FocusKind::Scroll, calls, area));
                    self.scroll_meta.push((
                        path,
                        ScrollMeta {
                            max: content.saturating_sub(1) as u64,
                            horizontal: matches!(orient, ScrollbarOrientation::HorizontalBottom),
                            track: true,
                        },
                    ));
                }
                let mut state = ScrollbarState::new(content).position(pos);
                frame.render_stateful_widget(Scrollbar::new(orient), area, &mut state);
            }
            "Log" => {
                // A scrollable text view — `offset` is the first drawn row,
                // `wrap` reflows instead of clipping. For log tails and
                // console panes where `Text` would clip. A bound `offset`
                // scrolls on the wheel and arrow keys.
                let text = self.node_text(node);
                let off = self.num(node, "offset", 0.0).max(0.0) as u16;
                if let Some(path) = Self::binding_of(node, "offset") {
                    let calls = self.calls_for(node);
                    self.focusables
                        .push((path.clone(), FocusKind::Scroll, calls, area));
                    let last = text.lines().count().saturating_sub(area.height as usize);
                    self.scroll_meta.push((
                        path,
                        ScrollMeta {
                            max: last as u64,
                            horizontal: false,
                            track: false,
                        },
                    ));
                }
                let mut p = Paragraph::new(text)
                    .scroll((off, 0))
                    .style(self.node_style(node));
                if self.prop_text(node, "wrap") != "false" {
                    p = p.wrap(Wrap { trim: false });
                }
                frame.render_widget(p, area);
            }
            "Spacer" => {
                // Empty area — the `size` prop feeds `child_constraint`,
                // nothing paints.
            }
            "Fill" => {
                // One glyph across the whole rect — backgrounds and
                // separator runs without a hand-rolled loop.
                let symbol = self.prop_text(node, "symbol");
                frame.render_widget(
                    Fill::new(if symbol.is_empty() {
                        " "
                    } else {
                        symbol.as_str()
                    })
                    .style(self.node_style(node)),
                    area,
                );
            }
            "Mascot" => {
                frame.render_widget(RatatuiMascot::new(), area);
            }
            "Logo" => {
                let logo = if self.prop_text(node, "size") == "tiny" {
                    RatatuiLogo::tiny()
                } else {
                    RatatuiLogo::small()
                };
                frame.render_widget(logo, area);
            }
            "Tree" => {
                // `items` indents text (two spaces or a tab per depth).
                // `TreeItem` children build the same structure in the doc.
                // A bound `selected` — the slash-joined identifier list —
                // makes arrows, wheel and clicks interactive.
                let title = self.prop_text(node, "title");
                let (items, opens) = self.tree_items(node);
                let binding = Self::binding_of(node, "selected");
                if let Some(path) = &binding {
                    let calls = self.calls_for(node);
                    self.focusables
                        .push((path.clone(), FocusKind::Tree, calls, area));
                }
                let focused = binding
                    .as_ref()
                    .map(|p| self.is_focused(p))
                    .unwrap_or(false);
                let expanded = self.prop_text(node, "expanded") != "false";
                let selected: Vec<String> = binding
                    .as_ref()
                    .and_then(|p| self.lookup(p))
                    .and_then(Json::as_str)
                    .map(|s| s.split('/').map(str::to_owned).collect())
                    .unwrap_or_default();
                match TuiTree::new(&items) {
                    Ok(tree) => {
                        let state = match binding.as_ref().map(|p| p.to_string()) {
                            Some(k) => match self.trees.entry(k) {
                                std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
                                std::collections::hash_map::Entry::Vacant(e) => {
                                    let s = e.insert(TreeState::default());
                                    if expanded {
                                        for p in &opens {
                                            s.open(p.clone());
                                        }
                                    }
                                    s
                                }
                            },
                            None => {
                                // Unbound trees hold no state — render a
                                // throwaway all-open one.
                                let mut s = TreeState::default();
                                if expanded {
                                    for p in &opens {
                                        s.open(p.clone());
                                    }
                                }
                                if !selected.is_empty() {
                                    s.select(selected);
                                }
                                let t = tree
                                    .block(Block::default().borders(Borders::ALL).title(title))
                                    .highlight_symbol("> ");
                                frame.render_stateful_widget(t, area, &mut s);
                                return;
                            }
                        };
                        if !selected.is_empty() {
                            state.select(selected);
                        }
                        let mut t = tree
                            .block(Block::default().borders(Borders::ALL).title(title))
                            .highlight_symbol("> ");
                        if focused {
                            t = t.highlight_style(
                                Style::default()
                                    .fg(Color::Cyan)
                                    .add_modifier(Modifier::BOLD),
                            );
                        }
                        frame.render_stateful_widget(t, area, state);
                    }
                    Err(e) => {
                        frame.render_widget(
                            Paragraph::new(format!("Tree: {e}"))
                                .style(Style::default().fg(Color::DarkGray)),
                            area,
                        );
                    }
                }
            }
            "TreeItem" => {
                // Alone it is just its label line — the `Tree` arm consumes
                // the structured form.
                frame.render_widget(Paragraph::new(self.node_text(node)), area);
            }
            "Spinner" => {
                // throbber-widgets-tui — `kind` picks the symbol set,
                // `step` is the frame step (bind it and tick to animate —
                // 0 or absent picks a random frame each draw).
                let set = match self.prop_text(node, "kind").as_str() {
                    "ascii" => throbber::ASCII,
                    "box-drawing" => throbber::BOX_DRAWING,
                    "arrow" => throbber::ARROW,
                    "double-arrow" => throbber::DOUBLE_ARROW,
                    "vertical-block" => throbber::VERTICAL_BLOCK,
                    "horizontal-block" => throbber::HORIZONTAL_BLOCK,
                    "quadrant-block" => throbber::QUADRANT_BLOCK,
                    "quadrant-block-crack" => throbber::QUADRANT_BLOCK_CRACK,
                    "white-square" => throbber::WHITE_SQUARE,
                    "white-circle" => throbber::WHITE_CIRCLE,
                    "black-circle" => throbber::BLACK_CIRCLE,
                    "clock" => throbber::CLOCK,
                    "braille-one" => throbber::BRAILLE_ONE,
                    "braille-double" => throbber::BRAILLE_DOUBLE,
                    "braille-six-double" => throbber::BRAILLE_SIX_DOUBLE,
                    "braille-eight" => throbber::BRAILLE_EIGHT,
                    "braille-eight-double" => throbber::BRAILLE_EIGHT_DOUBLE,
                    "ogham-a" => throbber::OGHAM_A,
                    "ogham-b" => throbber::OGHAM_B,
                    "ogham-c" => throbber::OGHAM_C,
                    "parenthesis" => throbber::PARENTHESIS,
                    "canadian" => throbber::CANADIAN,
                    _ => throbber::BRAILLE_SIX,
                };
                let mut state = ThrobberState::default();
                state.calc_step(self.num(node, "step", 0.0) as i8);
                let label = self.prop_text(node, "label");
                let label = if label.is_empty() {
                    self.node_text(node)
                } else {
                    label
                };
                let use_type = match self.prop_text(node, "symbol").as_str() {
                    "full" => WhichUse::Full,
                    "empty" => WhichUse::Empty,
                    _ => WhichUse::Spin,
                };
                frame.render_stateful_widget(
                    Throbber::default()
                        .label(label)
                        .throbber_set(set)
                        .use_type(use_type)
                        .style(self.node_style(node)),
                    area,
                    &mut state,
                );
            }
            #[cfg(not(target_arch = "wasm32"))]
            "Markdown" => {
                // ratatui-markdown — parse blocks, render lines at `width`
                // (default the rect's), then a Paragraph clips them. A
                // bound `offset` scrolls like `Log`.
                let text = self.node_text(node);
                let width = self.num(node, "width", area.width as f64).max(1.0) as usize;
                let off = self.num(node, "offset", 0.0).max(0.0) as u16;
                let renderer = MarkdownRenderer::new(width);
                let blocks = renderer.parse(&text);
                let lines = renderer.render(&blocks, &ThemeConfig::default());
                if let Some(path) = Self::binding_of(node, "offset") {
                    let calls = self.calls_for(node);
                    self.focusables
                        .push((path.clone(), FocusKind::Scroll, calls, area));
                    let last = lines.len().saturating_sub(area.height as usize);
                    self.scroll_meta.push((
                        path,
                        ScrollMeta {
                            max: last as u64,
                            horizontal: false,
                            track: false,
                        },
                    ));
                }
                let title = self.prop_text(node, "title");
                let mut p = Paragraph::new(lines)
                    .scroll((off, 0))
                    .style(self.node_style(node));
                if !title.is_empty() {
                    p = p.block(Block::default().borders(Borders::ALL).title(title));
                }
                frame.render_widget(p, area);
            }
            "AnsiText" => {
                // ansi-to-tui — escape codes become styled spans.
                let text = self.node_text(node);
                let parsed = text
                    .as_str()
                    .into_text()
                    .unwrap_or_else(|_| ratatui::text::Text::from(text));
                frame.render_widget(Paragraph::new(parsed).style(self.node_style(node)), area);
            }
            "Terminal" => {
                // A bordered ANSI screen — same parse as `AnsiText` plus
                // the terminal frame. A bound `offset` scrolls scrollback.
                let text = self.node_text(node);
                let off = self.num(node, "offset", 0.0).max(0.0) as u16;
                let parsed = text
                    .as_str()
                    .into_text()
                    .unwrap_or_else(|_| ratatui::text::Text::from(text));
                if let Some(path) = Self::binding_of(node, "offset") {
                    let calls = self.calls_for(node);
                    self.focusables
                        .push((path.clone(), FocusKind::Scroll, calls, area));
                    let last = parsed.lines.len().saturating_sub(area.height as usize);
                    self.scroll_meta.push((
                        path,
                        ScrollMeta {
                            max: last as u64,
                            horizontal: false,
                            track: false,
                        },
                    ));
                }
                let title = self.prop_text(node, "title");
                frame.render_widget(
                    Paragraph::new(parsed)
                        .block(Block::default().borders(Borders::ALL).title(title))
                        .scroll((off, 0)),
                    area,
                );
            }
            "ScrollView" => {
                // tui-scrollview — children paint into a virtual buffer
                // `width`×`height` cells, the viewport clips it and draws
                // scrollbars. A bound `offset` scrolls vertically.
                let vw = self.num(node, "width", area.width as f64).max(1.0) as u16;
                let vh = self.num(node, "height", area.height as f64).max(1.0) as u16;
                let mut sv = ScrollView::new(Size::new(vw, vh));
                sv = match self.prop_text(node, "scrollbars").as_str() {
                    "always" => sv
                        .horizontal_scrollbar_visibility(ScrollbarVisibility::Always)
                        .vertical_scrollbar_visibility(ScrollbarVisibility::Always),
                    "never" => sv
                        .horizontal_scrollbar_visibility(ScrollbarVisibility::Never)
                        .vertical_scrollbar_visibility(ScrollbarVisibility::Never),
                    "vertical" => sv.horizontal_scrollbar_visibility(ScrollbarVisibility::Never),
                    "horizontal" => sv.vertical_scrollbar_visibility(ScrollbarVisibility::Never),
                    _ => sv,
                };
                // Children paint into a headless frame of the virtual size,
                // then the cells copy into the scroll buffer — the same
                // render path, no widget shim. Hit zones inside the view
                // use virtual coordinates, so the arm drops them: children
                // of a ScrollView are display-only for now.
                let mut term = Terminal::new(TestBackend::new(vw, vh)).unwrap();
                let before = self.focusables.len();
                let virtual_area = Rect::new(0, 0, vw, vh);
                let _ = term.draw(|f| {
                    self.render_children(node, path, virtual_area, f, Direction::Vertical);
                });
                self.focusables.truncate(before);
                let src = term.backend().buffer();
                let dst = sv.buf_mut();
                for y in 0..vh {
                    for x in 0..vw {
                        if let Some(cell) = src.cell((x, y))
                            && let Some(d) = dst.cell_mut((x, y))
                        {
                            *d = cell.clone();
                        }
                    }
                }
                if let Some(path) = Self::binding_of(node, "offset") {
                    let calls = self.calls_for(node);
                    self.focusables
                        .push((path.clone(), FocusKind::Scroll, calls, area));
                    self.scroll_meta.push((
                        path,
                        ScrollMeta {
                            max: vh.saturating_sub(area.height) as u64,
                            horizontal: false,
                            track: false,
                        },
                    ));
                }
                let off = self.num(node, "offset", 0.0).max(0.0) as u16;
                let mut state = ScrollViewState::with_offset(Position::new(0, off));
                frame.render_stateful_widget(sv, area, &mut state);
            }
            #[cfg(not(target_arch = "wasm32"))]
            "CodeEditor" => {
                // ratatui-code-editor — tree-sitter highlighting for the
                // catalog's language list. `original` diffs against `text`.
                // Editing needs the crate's crossterm driver — the
                // doc widget renders and scrolls.
                let lang = self.prop_text(node, "language");
                let text = self.node_text(node);
                match CodeEditorWidget::new(&lang, &text, ratatui_code_editor::theme::vesper()) {
                    Ok(mut ed) => {
                        if self.prop_text(node, "folding") == "false" {
                            ed.set_code_folding_enabled(false);
                        }
                        let original = self.prop_text(node, "original");
                        if !original.is_empty() {
                            let _ = ed.set_original_code(&original);
                            ed.set_diff_enabled(true);
                        }
                        let off = self.num(node, "offset", 0.0).max(0.0) as usize;
                        for _ in 0..off.min(500) {
                            ed.scroll_down(area.height as usize);
                        }
                        if let Some(path) = Self::binding_of(node, "offset") {
                            let calls = self.calls_for(node);
                            self.focusables
                                .push((path.clone(), FocusKind::Scroll, calls, area));
                            self.scroll_meta.push((
                                path,
                                ScrollMeta {
                                    max: 500,
                                    horizontal: false,
                                    track: false,
                                },
                            ));
                        }
                        frame.render_widget(&ed, area);
                    }
                    Err(_) => {
                        frame.render_widget(Paragraph::new(text), area);
                    }
                }
            }
            "Toast" => {
                // ratatui-toaster's card — a bordered message tinted by
                // `type` (info/success/warning/error).
                let ty = match self.prop_text(node, "type").as_str() {
                    "success" => ToastType::Success,
                    "warning" | "warn" => ToastType::Warning,
                    "error" => ToastType::Error,
                    _ => ToastType::Info,
                };
                Toast::new(&self.node_text(node), ty).render_ref(area, frame.buffer_mut());
            }
            "Popup" => {
                // tui-popup — children draw offscreen, then the popup
                // floats them centered over `area` in a bordered surface.
                // `width`/`height` size the body (default: area minus 4).
                let w = self
                    .num(node, "width", f64::from(area.width.saturating_sub(4)))
                    .max(1.0) as u16;
                let h = self
                    .num(node, "height", f64::from(area.height.saturating_sub(4)))
                    .max(1.0) as u16;
                let zstart = self.zones.len();
                let fstart = self.focusables.len();
                let buf = self.children_buffer(node, path, w, h);
                let mut popup = tui_popup::Popup::new(BufWidget::new(buf));
                let title = self.prop_text(node, "title");
                if !title.is_empty() {
                    popup.title = Line::from(title);
                }
                popup.style = popup.style.patch(self.node_style(node));
                if let Some(c) = self.border_color(node) {
                    popup.border_style = Style::default().fg(c);
                }
                // `popup_area` centers the bordered surface — precompute it
                // so offscreen child zones land on the real cells.
                let surface = area.centered(
                    Constraint::Length(w.saturating_add(2)),
                    Constraint::Length(h.saturating_add(2)),
                );
                let mut state = tui_popup::PopupState::default();
                frame.render_stateful_widget(popup, area, &mut state);
                self.translate_zones(zstart, fstart, surface.x + 1, surface.y + 1);
            }
            "WidgetList" => {
                // tui-widget-list — children are items of mixed height.
                // Each draws offscreen at its `height` style (default 3)
                // and the list scrolls through them. A bound `selected`
                // gets arrow/wheel/click navigation.
                let kids = self.expand_kids(&node.children);
                let mut items: Vec<TuiBuffer> = Vec::new();
                let mut heights: Vec<u16> = Vec::new();
                let mut y = area.y;
                for (i, kid) in kids.iter().enumerate() {
                    let h = Self::style_num(&kid.style, "height")
                        .map(|(v, _)| v.max(1.0) as u16)
                        .unwrap_or(3);
                    let zstart = self.zones.len();
                    let fstart = self.focusables.len();
                    let buf = self.child_buffer(kid, &format!("{path}.{i}"), area.width, h);
                    self.translate_zones(zstart, fstart, area.x, y);
                    y = y.saturating_add(h);
                    items.push(buf);
                    heights.push(h);
                }
                let count = items.len();
                let builder = tui_widget_list::ListBuilder::new(move |ctx| {
                    let i = ctx.index.min(count.saturating_sub(1));
                    let buf = items
                        .get(i)
                        .cloned()
                        .unwrap_or_else(|| TuiBuffer::empty(Rect::new(0, 0, 1, 1)));
                    (BufWidget::new(buf), heights.get(i).copied().unwrap_or(1))
                });
                let mut view = tui_widget_list::ListView::new(builder, count);
                if self.prop_text(node, "direction") == "horizontal" {
                    view = view.scroll_axis(tui_widget_list::ScrollAxis::Horizontal);
                }
                let mut state = tui_widget_list::ListState::default();
                let binding = Self::binding_of(node, "selected");
                if let Some(path) = &binding {
                    let calls = self.calls_for(node);
                    self.focusables
                        .push((path.clone(), FocusKind::Select, calls, area));
                    state.select(Some(
                        self.lookup(path)
                            .and_then(Json::as_u64)
                            .unwrap_or(0)
                            .min(count.saturating_sub(1) as u64) as usize,
                    ));
                }
                frame.render_stateful_widget(view, area, &mut state);
            }
            "Nodes" => {
                // tui-nodes — `GraphNode` children are the boxes. The
                // `edges` prop wires ports "from.port>to.port" (child
                // indices, comma-separated). Placement is automatic.
                let kids = self.expand_kids(&node.children);
                let titles: Vec<String> = kids.iter().map(|k| self.node_text(k)).collect();
                let mut layouts = Vec::new();
                for (i, kid) in kids.iter().enumerate() {
                    let w = self.num(kid, "width", 12.0).max(4.0) as u16;
                    let h = self.num(kid, "height", 3.0).max(3.0) as u16;
                    layouts.push(
                        tui_nodes::NodeLayout::new((w, h))
                            .with_title(titles[i].as_str())
                            .with_border_type(BorderType::Rounded),
                    );
                }
                let line_type = match self.prop_text(node, "lines").as_str() {
                    "rounded" => tui_nodes::LineType::Rounded,
                    "double" => tui_nodes::LineType::Double,
                    "thick" => tui_nodes::LineType::Thick,
                    _ => tui_nodes::LineType::Plain,
                };
                let edges: Vec<tui_nodes::Connection> = self
                    .prop_text(node, "edges")
                    .split(',')
                    .filter_map(|e| {
                        let (from, to) = e.trim().split_once('>')?;
                        let (fn_, fp) = from.trim().split_once('.')?;
                        let (tn, tp) = to.trim().split_once('.')?;
                        Some(
                            tui_nodes::Connection::new(
                                fn_.trim().parse().ok()?,
                                fp.trim().parse().ok()?,
                                tn.trim().parse().ok()?,
                                tp.trim().parse().ok()?,
                            )
                            .with_line_type(line_type),
                        )
                    })
                    .collect();
                let mut graph = tui_nodes::NodeGraph::new(
                    layouts,
                    edges,
                    area.width as usize,
                    area.height as usize,
                );
                graph.calculate();
                let mut st = ();
                frame.render_stateful_widget(graph, area, &mut st);
            }
            "Shimmer" => {
                // tui-shimmer — a highlight sweeps the text over a 4s
                // cycle. `phase` (0..1) pins a frame. Bind it to animate.
                let phase = self.num(node, "phase", 0.0) as f32;
                let spans = tui_shimmer::shimmer_spans_with_style_at_phase(
                    &self.node_text(node),
                    self.node_style(node),
                    phase,
                );
                frame.render_widget(Paragraph::new(Line::from(spans)), area);
            }
            "Dialog" => {
                // tui_confirm_dialog — `open` draws the centered box.
                // `choice` binds the answer. Left/Right picks a button and
                // Enter/y/n commits. `yes-label`/`no-label` use "(Y)es"
                // markup — the parenthesized char is the hotkey.
                if self.boolean(node, "open", false) {
                    let title = self.prop_text(node, "title");
                    let text = self.node_text(node);
                    let mut st = tui_confirm_dialog::ConfirmDialogState::new(0, title, text);
                    st.modal(true).open();
                    if let Some(path) = Self::binding_of(node, "choice") {
                        let calls = self.calls_for(node);
                        self.focusables
                            .push((path.clone(), FocusKind::Dialog, calls, area));
                        let sel = self
                            .dialog_sel
                            .get(&path.to_string())
                            .copied()
                            .unwrap_or(true);
                        st.with_yes_button_selected(sel);
                    }
                    let yes = self.prop_text(node, "yes-label");
                    if let Ok(l) = tui_confirm_dialog::ButtonLabel::try_from(yes.as_str()) {
                        st.with_yes_button(l);
                    }
                    let no = self.prop_text(node, "no-label");
                    if no == "none" {
                        st.without_no_button();
                    } else if let Ok(l) = tui_confirm_dialog::ButtonLabel::try_from(no.as_str()) {
                        st.with_no_button(l);
                    }
                    let mut w = tui_confirm_dialog::ConfirmDialog::new()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded);
                    if let Some(c) = self.border_color(node) {
                        w = w.border_style(Style::default().fg(c));
                    }
                    frame.render_stateful_widget(w, area, &mut st);
                }
            }
            "Fx" => {
                // tachyonfx — children paint first, then the effect
                // post-processes their cells. `progress` (0..1) scrubs
                // through `duration` (ms). A fresh Effect each render
                // makes the frame deterministic.
                self.render_children(node, path, area, frame, Direction::Vertical);
                let ms = self.num(node, "duration", 800.0).max(1.0) as u32;
                let elapsed = tachyonfx::Duration::from_millis(
                    (self.num(node, "progress", 1.0).clamp(0.0, 1.0) * f64::from(ms)) as u32,
                );
                let timer = (ms, tachyonfx::Interpolation::Linear);
                use tachyonfx::fx::ExpandDirection;
                use tachyonfx::{Motion, fx};
                let mut effect = match self.prop_text(node, "kind").as_str() {
                    "coalesce" => fx::coalesce(timer),
                    "dissolve-to" => {
                        fx::dissolve_to(Style::default().fg(Color::Black).bg(Color::Black), timer)
                    }
                    "coalesce-from" => {
                        fx::coalesce_from(Style::default().fg(Color::Black).bg(Color::Black), timer)
                    }
                    "sweep-in" => fx::sweep_in(Motion::LeftToRight, 10, 0, Color::Black, timer),
                    "sweep-out" => fx::sweep_out(Motion::LeftToRight, 10, 0, Color::Black, timer),
                    "slide-in" => fx::slide_in(Motion::UpToDown, 5, 0, Color::Black, timer),
                    "slide-out" => fx::slide_out(Motion::UpToDown, 5, 0, Color::Black, timer),
                    "explode" => fx::explode(2.0, 0.5, timer),
                    "expand" => fx::expand(ExpandDirection::Horizontal, Style::default(), timer),
                    "expand-v" => fx::expand(ExpandDirection::Vertical, Style::default(), timer),
                    "stretch" => fx::stretch(Motion::RightToLeft, Style::default(), timer),
                    "paint" => fx::paint_fg(Color::White, timer),
                    "fade-to" => fx::fade_to_fg(Color::Black, timer),
                    "hsl-shift" => fx::hsl_shift_fg([30.0, 0.0, 0.0], timer),
                    _ => fx::dissolve(timer),
                };
                effect.process(elapsed, frame.buffer_mut(), area);
            }
            "QrCode" => {
                // `qrcode` — unicode half-block rendering of `data`.
                let data = {
                    let d = self.prop_text(node, "data");
                    if d.is_empty() {
                        self.node_text(node)
                    } else {
                        d
                    }
                };
                match qrcode::QrCode::new(data.as_bytes()) {
                    Ok(code) => {
                        let img = code
                            .render::<qrcode::render::unicode::Dense1x2>()
                            .quiet_zone(false)
                            .module_dimensions(2, 1)
                            .build();
                        frame.render_widget(Paragraph::new(img), area);
                    }
                    Err(_) => {
                        frame.render_widget(
                            Paragraph::new("invalid QR data")
                                .style(Style::default().fg(Color::DarkGray)),
                            area,
                        );
                    }
                }
            }
            "Image" => {
                // `image` decode → halfblock raster: each cell is ▀ with
                // fg = upper pixel, bg = lower. `src` resolves on
                // `base_dir` like every other asset reference.
                let src = {
                    let s = self.prop_text(node, "src");
                    if s.is_empty() {
                        self.prop_text(node, "path")
                    } else {
                        s
                    }
                };
                let path = self
                    .base_dir
                    .as_ref()
                    .map(|d| d.join(&src))
                    .unwrap_or_else(|| std::path::PathBuf::from(&src));
                match image::open(&path) {
                    Ok(img) => {
                        let w = u32::from(area.width.max(1));
                        let h = u32::from(area.height.max(1)) * 2;
                        let rgba = image::imageops::resize(
                            &img,
                            w,
                            h,
                            image::imageops::FilterType::Triangle,
                        );
                        let mut lines = Vec::new();
                        for y in (0..h).step_by(2) {
                            let mut spans = Vec::new();
                            for x in 0..w {
                                let t = rgba.get_pixel(x, y).0;
                                let b = if y + 1 < h {
                                    rgba.get_pixel(x, y + 1).0
                                } else {
                                    [0, 0, 0, 0]
                                };
                                spans.push(Span::styled(
                                    "\u{2580}",
                                    Style::default()
                                        .fg(Color::Rgb(t[0], t[1], t[2]))
                                        .bg(Color::Rgb(b[0], b[1], b[2])),
                                ));
                            }
                            lines.push(Line::from(spans));
                        }
                        frame.render_widget(Paragraph::new(lines), area);
                    }
                    Err(_) => {
                        frame.render_widget(
                            Paragraph::new(format!("image: {src}"))
                                .style(Style::default().fg(Color::DarkGray)),
                            area,
                        );
                    }
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            "Slider" => {
                // tui-slider — a bound `value` takes Left/Right `step`
                // writes, wheel ticks, and click-to-position.
                let min = self.num(node, "min", 0.0);
                let max = self.num(node, "max", 100.0).max(min);
                let step = self.num(node, "step", 1.0).max(f64::EPSILON);
                let value = self.num(node, "value", min).clamp(min, max);
                let binding = Self::binding_of(node, "value");
                if let Some(path) = &binding {
                    let calls = self.calls_for(node);
                    self.focusables
                        .push((path.clone(), FocusKind::Slider, calls, area));
                    self.sliders.push((
                        path.clone(),
                        SliderMeta {
                            min,
                            max,
                            step,
                            x: area.x,
                            width: area.width,
                        },
                    ));
                }
                let focused = binding
                    .as_ref()
                    .map(|p| self.is_focused(p))
                    .unwrap_or(false);
                let mut slider = Slider::new(value, min, max)
                    .focused(focused)
                    .show_value(self.boolean(node, "show-value", true));
                let label = self.prop_text(node, "label");
                if !label.is_empty() {
                    slider = slider.label(label);
                }
                if self.prop_text(node, "orientation") == "vertical" {
                    slider = slider.orientation(tui_slider::SliderOrientation::Vertical);
                }
                frame.render_widget(slider, area);
            }
            #[cfg(not(target_arch = "wasm32"))]
            "PieChart" => {
                // tui-piechart — `data` numbers, newline `labels`, and a
                // fixed slice palette. `hole` (0..1) makes it a donut.
                let data = self.floats(node, "data");
                let labels: Vec<String> = self
                    .prop_text(node, "labels")
                    .lines()
                    .map(|l| l.trim().to_owned())
                    .collect();
                const PALETTE: [Color; 8] = [
                    Color::Red,
                    Color::Green,
                    Color::Yellow,
                    Color::Blue,
                    Color::Magenta,
                    Color::Cyan,
                    Color::LightRed,
                    Color::LightBlue,
                ];
                let slices: Vec<PieSlice> = data
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        PieSlice::new(
                            labels.get(i).map(String::as_str).unwrap_or(""),
                            *v,
                            PALETTE[i % PALETTE.len()],
                        )
                    })
                    .collect();
                let mut chart = PieChart::new(slices);
                let title = self.prop_text(node, "title");
                if !title.is_empty() {
                    chart = chart.block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .title(title),
                    );
                }
                chart = chart
                    .hole(self.num(node, "hole", 0.0).clamp(0.0, 0.9))
                    .show_legend(self.boolean(node, "legend", true))
                    .legend_position(match self.prop_text(node, "legend-position").as_str() {
                        "left" => tui_piechart::LegendPosition::Left,
                        "top" => tui_piechart::LegendPosition::Top,
                        "bottom" => tui_piechart::LegendPosition::Bottom,
                        _ => tui_piechart::LegendPosition::Right,
                    });
                frame.render_widget(chart, area);
            }
            #[cfg(not(target_arch = "wasm32"))]
            "BigText" => {
                // tui-big-text — an 8x8 font drawn as block pixels.
                // `size` is pixels-per-cell: full, half, quadrant…
                let size = match self.prop_text(node, "size").as_str() {
                    "half" | "half-height" => PixelSize::HalfHeight,
                    "half-width" => PixelSize::HalfWidth,
                    "quadrant" => PixelSize::Quadrant,
                    "sextant" => PixelSize::Sextant,
                    "third" => PixelSize::ThirdHeight,
                    "quarter" => PixelSize::QuarterHeight,
                    "octant" => PixelSize::Octant,
                    _ => PixelSize::Full,
                };
                let mut builder = BigText::builder();
                builder
                    .lines(vec![Line::from(self.node_text(node))])
                    .pixel_size(size)
                    .style(self.node_style(node));
                match self.prop_text(node, "align").as_str() {
                    "right" => {
                        builder.right_aligned();
                    }
                    "center" => {
                        builder.centered();
                    }
                    _ => {
                        builder.left_aligned();
                    }
                }
                frame.render_widget(builder.build(), area);
            }
            #[cfg(not(target_arch = "wasm32"))]
            "Globe" => {
                // tui-globe — an embedded coastline mesh under a camera.
                // A bound `yaw` rotates it like a slider (-180..180).
                static DATA: std::sync::OnceLock<tui_globe::MapData> = std::sync::OnceLock::new();
                let data = DATA.get_or_init(tui_globe::MapData::embedded);
                if let Some(path) = Self::binding_of(node, "yaw") {
                    let calls = self.calls_for(node);
                    self.focusables
                        .push((path.clone(), FocusKind::Slider, calls, area));
                    self.sliders.push((
                        path,
                        SliderMeta {
                            min: -180.0,
                            max: 180.0,
                            step: 10.0,
                            x: 0,
                            width: 0,
                        },
                    ));
                }
                let camera = tui_globe::Camera {
                    yaw: self.num(node, "yaw", 0.0) as f32,
                    pitch: self.num(node, "pitch", 0.0) as f32,
                    zoom: self.num(node, "zoom", 1.0).max(0.1) as f32,
                };
                frame.render_widget(tui_globe::Globe::new(data, camera), area);
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

    /// Renders a node's children into an offscreen buffer through a
    /// scratch TestBackend terminal — the same path as the live frame.
    /// Popup bodies and WidgetList items draw this way.
    fn children_buffer(&mut self, node: &Node, path: &str, w: u16, h: u16) -> TuiBuffer {
        let mut term = Terminal::new(TestBackend::new(w.max(1), h.max(1))).expect("test backend");
        let _ = term.draw(|f| self.render_children(node, path, f.area(), f, Direction::Vertical));
        term.backend().buffer().clone()
    }

    /// One child into an offscreen buffer — the WidgetList item path.
    fn child_buffer(&mut self, node: &Node, path: &str, w: u16, h: u16) -> TuiBuffer {
        let mut term = Terminal::new(TestBackend::new(w.max(1), h.max(1))).expect("test backend");
        let _ = term.draw(|f| self.render_node(node, path, f.area(), f));
        term.backend().buffer().clone()
    }

    /// Re-anchors zones and focusable rects captured during an offscreen
    /// child render — `zfrom`/`ffrom` are the vec lengths at capture time.
    fn translate_zones(&mut self, zfrom: usize, ffrom: usize, dx: u16, dy: u16) {
        for (_, r) in self.zones.iter_mut().skip(zfrom) {
            r.x = r.x.saturating_add(dx);
            r.y = r.y.saturating_add(dy);
        }
        for (_, _, _, r) in self.focusables.iter_mut().skip(ffrom) {
            r.x = r.x.saturating_add(dx);
            r.y = r.y.saturating_add(dy);
        }
    }

    /// `Tree` content → `TreeItem`s plus every identifier path (for
    /// all-open seeding). `TreeItem` children nest structurally. An `items`
    /// prop parses indented text — two spaces or a tab per depth.
    /// Identifiers are `id` props, else the row text.
    fn tree_items(&mut self, node: &Node) -> (Vec<TreeItem<'static, String>>, Vec<Vec<String>>) {
        let mut opens = Vec::new();
        let kids = self.expand_kids(&node.children);
        if kids.iter().any(|c| c.element.name == "TreeItem") {
            let items = kids
                .iter()
                .filter(|c| c.element.name == "TreeItem")
                .map(|c| self.tree_item(c, &mut Vec::new(), &mut opens))
                .collect();
            return (items, opens);
        }
        let lines: Vec<(usize, String)> = self
            .prop_text(node, "items")
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                let depth: usize = l
                    .chars()
                    .take_while(|c| c.is_whitespace())
                    .map(|c| if c == '\t' { 2 } else { 1 })
                    .sum::<usize>()
                    / 2;
                (depth, l.trim().to_string())
            })
            .collect();
        let mut i = 0;
        let items = Self::tree_level(&lines, &mut i, 0, &mut Vec::new(), &mut opens);
        (items, opens)
    }

    /// One depth of the indented `items` lines → `TreeItem`s — deeper lines
    /// attach under the row above them.
    fn tree_level(
        lines: &[(usize, String)],
        i: &mut usize,
        depth: usize,
        prefix: &mut Vec<String>,
        opens: &mut Vec<Vec<String>>,
    ) -> Vec<TreeItem<'static, String>> {
        let mut out = Vec::new();
        while let Some((d, text)) = lines.get(*i) {
            if *d < depth {
                break;
            }
            *i += 1;
            prefix.push(text.clone());
            opens.push(prefix.clone());
            let kids = if lines.get(*i).is_some_and(|(d2, _)| *d2 > depth) {
                Self::tree_level(lines, i, depth + 1, prefix, opens)
            } else {
                Vec::new()
            };
            prefix.pop();
            let item = if kids.is_empty() {
                TreeItem::new_leaf(text.clone(), text.clone())
            } else {
                TreeItem::new(text.clone(), text.clone(), kids)
                    .unwrap_or_else(|_| TreeItem::new_leaf(text.clone(), text.clone()))
            };
            out.push(item);
        }
        out
    }

    /// A `TreeItem` element → the widget item — `id` or the text is the
    /// identifier, nested `TreeItem` children attach under it.
    fn tree_item(
        &mut self,
        node: &Node,
        prefix: &mut Vec<String>,
        opens: &mut Vec<Vec<String>>,
    ) -> TreeItem<'static, String> {
        let text = self.node_text(node);
        let id = self.prop_text(node, "id");
        let id = if id.is_empty() { text.clone() } else { id };
        prefix.push(id.clone());
        opens.push(prefix.clone());
        let kids: Vec<TreeItem<String>> = {
            let expanded = self.expand_kids(&node.children);
            expanded
                .iter()
                .filter(|c| c.element.name == "TreeItem")
                .map(|c| self.tree_item(c, prefix, opens))
                .collect()
        };
        prefix.pop();
        if kids.is_empty() {
            TreeItem::new_leaf(id, text)
        } else {
            TreeItem::new(id.clone(), text.clone(), kids)
                .unwrap_or_else(|_| TreeItem::new_leaf(id, text))
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
        // `Spacer`'s `size` prop is its extent — no style entry needed.
        if node.element.name == "Spacer"
            && let Some((_, Value::Number { value, .. })) =
                node.props.iter().find(|(k, _)| k == "size")
        {
            return Constraint::Length((*value).max(0.0) as u16);
        }
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

/// An offscreen-rendered child buffer as a `Widget` — Popup bodies and
/// WidgetList items draw through `render_node` first, then blit.
struct BufWidget {
    buf: TuiBuffer,
}

impl BufWidget {
    fn new(buf: TuiBuffer) -> Self {
        Self { buf }
    }
}

impl tui_popup::KnownSize for BufWidget {
    fn width(&self) -> usize {
        self.buf.area.width as usize
    }
    fn height(&self) -> usize {
        self.buf.area.height as usize
    }
}

impl ratatui::widgets::Widget for BufWidget {
    fn render(self, area: Rect, buf: &mut TuiBuffer) {
        for y in 0..self.buf.area.height.min(area.height) {
            for x in 0..self.buf.area.width.min(area.width) {
                if let (Some(s), Some(d)) = (
                    self.buf.cell((x, y)),
                    buf.cell_mut((area.x + x, area.y + y)),
                ) {
                    *d = s.clone();
                }
            }
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

/// The interactivity a focused element accepts. Input = text editing.
/// Bool = toggle (Checkbox/Switch). Select = arrow navigation (List/Tabs).
/// Action = fire `click` on Enter/Space/down (Button).
/// Scroll = wheel/keys/track-click move a bound offset (Scrollbar/Log).
/// Tree = arrows navigate/fold, Enter toggles, clicks pick a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusKind {
    Input,
    Bool,
    Select,
    Action,
    Scroll,
    Tree,
    Slider,
    Dialog,
}

/// Per-path range for a `FocusKind::Slider` zone — `min`/`max`/`step`
/// drive arrow writes, `x`/`width` map a clicked cell to a value.
#[derive(Clone, Copy)]
struct SliderMeta {
    min: f64,
    max: f64,
    step: f64,
    x: u16,
    width: u16,
}

/// Per-path scroll behavior for a `FocusKind::Scroll` zone — `max` clamps
/// writes, `horizontal` picks the click axis, `track` turns `down` into a
/// jump-to-fraction (a scrollbar's track, not a Log's text).
#[derive(Clone, Copy)]
struct ScrollMeta {
    max: u64,
    horizontal: bool,
    track: bool,
}

/// A resolved `on <event>:` call — `(event name, handler name, args)`.
/// Args resolve at render time, so a fired call sees the document as of
/// the last draw.
type Calls = Vec<(String, String, Vec<String>)>;
