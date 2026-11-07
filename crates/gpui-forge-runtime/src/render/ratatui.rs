//! `ForgeView` for the ratatui base — a terminal-UI view (#112).
//!
//! Same host contract as the GPUI twins (document/state/hooks/sinks, JSON
//! state paths, `fire_call`) minus the GPUI machinery: there is no
//! `Context<Entity>` in a crossterm event loop, so sinks take `&mut ForgeView`
//! directly and `render` draws into a `ratatui::Frame`. The host
//! (`forge-preview-ratatui`) owns the terminal.

use crate::render::{get_path, set_path};
use forge_spec::catalog::Catalog;
use forge_spec::{Document, Node, Path, Segment, Unit, Value};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Gauge, List, ListItem, Paragraph, Tabs, Wrap};
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
}

pub struct ForgeView {
    doc: Option<Document>,
    pub catalogs: Vec<Catalog>,
    state: Json,
    pub libraries: Vec<Box<dyn ForgeLibrary>>,
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
    /// Focusable inputs — `(node key, binding path)` rebuilt each render;
    /// `Tab`/`Shift-Tab` cycles, typed keys write the focused binding.
    focusables: Vec<(Path, FocusKind)>,
    focused: usize,
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
            state: Json::Object(Default::default()),
            libraries: Vec::new(),
            hooks: HashMap::new(),
            on_hook: None,
            on_state: None,
            errors: Vec::new(),
            base_dir: None,
            previews: HashMap::new(),
            focusables: Vec::new(),
            focused: 0,
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
    pub fn preview(&mut self, id: &str) -> &mut ForgeView {
        let catalogs = self.catalogs.clone();
        self.previews
            .entry(id.to_owned())
            .or_insert_with(|| ForgeView::new(catalogs))
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
        self.libraries.push(Box::new(lib));
    }

    /// Drains accumulated errors — the host captures them.
    pub fn take_errors(&mut self) -> Vec<String> {
        std::mem::take(&mut self.errors)
    }

    /// Terminal keys — the host reads the tty and forwards them (#112).
    /// `Tab`/`Shift-Tab` cycles focusable inputs; printable keys write into
    /// the focused input's binding; `Enter`/`Space` fires toggle/select.
    pub fn on_key(&mut self, key: &str) {
        let kind = self.focusables.get(self.focused).map(|(_, k)| *k);
        match (kind, key) {
            (_, "Tab") | (_, "BackTab") => {}
            (Some(FocusKind::Bool), "Enter") | (Some(FocusKind::Bool), " ") => {
                if let Some((path, _)) = self.focusables.get(self.focused).cloned() {
                    let cur = self.lookup(&path).and_then(Json::as_bool).unwrap_or(false);
                    self.write(&path, Json::Bool(!cur));
                }
                return;
            }
            (Some(FocusKind::Select), "Up") | (Some(FocusKind::Select), "k") => {
                if let Some((path, _)) = self.focusables.get(self.focused).cloned() {
                    let cur = self.lookup(&path).and_then(Json::as_u64).unwrap_or(0);
                    self.write(&path, Json::from(cur.saturating_sub(1)));
                }
                return;
            }
            (Some(FocusKind::Select), "Down") | (Some(FocusKind::Select), "j") => {
                if let Some((path, _)) = self.focusables.get(self.focused).cloned() {
                    let cur = self.lookup(&path).and_then(Json::as_u64).unwrap_or(0);
                    self.write(&path, Json::from(cur + 1));
                }
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
                // `on submit:` hooks on the focused node — the node's key
                // identifies it; events live on the node itself.
            }
            "Backspace" => {
                if let Some((path, _)) = self.focusables.get(self.focused).cloned() {
                    let mut cur = self.json_text(&path);
                    cur.pop();
                    self.write(&path, Json::String(cur));
                }
            }
            c if c.chars().count() == 1 => {
                if let Some((path, _)) = self.focusables.get(self.focused).cloned() {
                    let mut cur = self.json_text(&path);
                    cur.push_str(c);
                    self.write(&path, Json::String(cur));
                }
            }
            _ => {}
        }
    }

    /// Whether `path` is the focused binding — the Input arm highlights it.
    fn is_focused(&self, path: &Path) -> bool {
        self.focusables
            .get(self.focused)
            .map(|(p, _)| p == path)
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
        get_path(&self.state, path)
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

    // ── rendering ────────────────────────────────────────────────────────────

    /// Draws the document into `frame` — called by the host's event loop.
    pub fn render(&mut self, frame: &mut Frame) {
        self.focusables.clear();
        let area = frame.area();
        if let Some(doc) = self.doc.clone() {
            self.render_node(&doc.root, area, frame);
        } else if let Some(text) = self.empty_text.clone() {
            frame.render_widget(
                Paragraph::new(text)
                    .style(Style::default().fg(Color::DarkGray))
                    .wrap(Wrap { trim: true }),
                area,
            );
        }
    }

    fn render_node(&mut self, node: &Node, area: Rect, frame: &mut Frame) {
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
                        view.render_node(&doc.root, area, frame);
                    }
                }
            }
            return;
        }
        match node.element.name.as_str() {
            "VStack" => self.render_children(node, area, frame, Direction::Vertical),
            "HStack" => self.render_children(node, area, frame, Direction::Horizontal),
            "Block" | "Paper" | "Card" => {
                let title = self.prop_text(node, "title");
                let mut block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded);
                if !title.is_empty() {
                    block = block.title(title);
                }
                let inner = block.inner(area);
                frame.render_widget(block, area);
                if node.children.is_empty() {
                    let text = self.node_text(node);
                    if !text.is_empty() {
                        frame.render_widget(Paragraph::new(text), inner);
                    }
                } else {
                    self.render_children(node, inner, frame, Direction::Vertical);
                }
            }
            "Text" | "Title" => {
                let mut line = Line::from(Span::raw(self.node_text(node)));
                if node.element.name == "Title" {
                    line = Line::from(
                        Span::raw(line.to_string())
                            .style(Style::default().add_modifier(Modifier::BOLD)),
                    );
                }
                frame.render_widget(Paragraph::new(line).wrap(Wrap { trim: true }), area);
            }
            "Divider" => {
                frame.render_widget(
                    Block::default().borders(Borders::TOP),
                    Rect { height: 1, ..area },
                );
            }
            "Button" => {
                let label = format!("[ {} ]", self.node_text(node));
                frame.render_widget(
                    Paragraph::new(label).style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    area,
                );
            }
            "List" | "Select" | "Tabs" => {
                let binding = Self::binding_of(node, "selected");
                if let Some(path) = &binding {
                    self.focusables.push((path.clone(), FocusKind::Select));
                }
                let focused = binding
                    .as_ref()
                    .map(|p| self.is_focused(p))
                    .unwrap_or(false);
                if node.element.name == "Tabs" {
                    let titles: Vec<Line> = node
                        .children
                        .iter()
                        .map(|c| Line::from(self.node_text(c)))
                        .collect();
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
                    let items: Vec<ListItem> = node
                        .children
                        .iter()
                        .map(|c| ListItem::new(self.node_text(c)))
                        .collect();
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
                    frame.render_widget(list, area);
                }
            }
            "Gauge" | "Progress" => {
                let value = self.num(node, "value", 0.0).clamp(0.0, 100.0);
                let label = self.prop_text(node, "label");
                frame.render_widget(
                    Gauge::default()
                        .block(Block::default().borders(Borders::ALL))
                        .gauge_style(Style::default().fg(Color::Green))
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
                if let Some(path) = &binding {
                    self.focusables.push((path.clone(), FocusKind::Input));
                }
                let focused = binding
                    .as_ref()
                    .map(|p| self.is_focused(p))
                    .unwrap_or(false);
                let mut block = Block::default().borders(Borders::ALL);
                if !label.is_empty() {
                    block = block.title(label);
                }
                if focused {
                    block = block
                        .border_style(Style::default().fg(Color::Cyan))
                        .title("▸ edit");
                }
                frame.render_widget(
                    Paragraph::new(if text.is_empty() { " " } else { text.as_str() })
                        .block(block)
                        .style(if empty {
                            Style::default().fg(Color::DarkGray)
                        } else {
                            Style::default()
                        }),
                    Rect {
                        height: 3.min(area.height),
                        ..area
                    },
                );
            }
            "Checkbox" | "Switch" => {
                let binding = Self::binding_of(node, "checked");
                if let Some(path) = &binding {
                    self.focusables.push((path.clone(), FocusKind::Bool));
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
                    Paragraph::new(format!("{} {label}", if checked { "[x]" } else { "[ ]" }));
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
                frame.render_widget(
                    Paragraph::new("table — rows pending the port")
                        .block(Block::default().borders(Borders::ALL).title(title)),
                    area,
                );
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

    /// Splits `area` into one chunk per child along `direction` and renders
    /// each child — equal `Fill` shares for the scaffold; `flex`/`width`
    /// style entries refine this in the port.
    fn render_children(&mut self, node: &Node, area: Rect, frame: &mut Frame, dir: Direction) {
        let constraints: Vec<Constraint> =
            node.children.iter().map(|_| Constraint::Fill(1)).collect();
        let chunks = Layout::default()
            .direction(dir)
            .constraints(constraints)
            .split(area);
        for (child, chunk) in node.children.iter().zip(chunks.iter()) {
            self.render_node(child, *chunk, frame);
        }
    }
}

/// The interactivity a focused element accepts — Input = text editing,
/// Bool = toggle (Checkbox/Switch), Select = arrow navigation (List/Tabs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusKind {
    Input,
    Bool,
    Select,
}
