//! Interprets a Forge document as gpui-kit elements.
//!
//! The gpui-kit adapter (#75). `ForgeView` renders a `Document`, keeps the JSON view
//! state that `bind` writes to, and dispatches `on <event>` bindings to registered
//! hooks or the hook sink.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use forge_spec::catalog::{Catalog, PropType};
use forge_spec::value::parse_path;
use forge_spec::{
    BindingMode, Color, Document, ElementRef, Entries, EventBinding, HandlerRef, Node, Path,
    PathSegment, Segment, Unit, Value,
};
use gpui_kit::component::accordion::Accordion;
use gpui_kit::component::alert::{Alert, AlertVariant};
use gpui_kit::component::avatar::{Avatar, AvatarGroup};
use gpui_kit::component::badge::Badge;
use gpui_kit::component::breadcrumb::{Breadcrumb, BreadcrumbItem};
use gpui_kit::component::button::{Button, ButtonVariant, ButtonVariants};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::collapsible::Collapsible;
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::notification::NotificationType;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::WindowExt;
use gpui_kit::component::color_picker::{ColorPicker, ColorPickerEvent, ColorPickerState};
use gpui_kit::component::Colorize;
use gpui_kit::component::description_list::{DescriptionItem, DescriptionList};
use gpui_kit::component::form::{Field, Form};
use gpui_kit::component::empty::{
    Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyMediaVariant, EmptyTitle,
};
use gpui_kit::component::group_box::{GroupBox, GroupBoxVariant, GroupBoxVariants};
use gpui_kit::component::input::{
    Editor, EditorState, Input, InputEvent, InputState, NumberInput, OtpEvent, OtpInput,
    OtpState, Textarea, TextareaState,
};
use gpui_kit::component::kbd::Kbd;
use gpui_kit::component::label::Label;
use gpui_kit::component::link::Link;
use gpui_kit::component::list::{List, ListDelegate, ListEvent, ListItem, ListState};
use gpui_kit::component::message::{
    Message, MessageAvatar, MessageContent, MessageFooter, MessageHeader,
};
use gpui_kit::component::table::{
    Table, TableBody, TableCaption, TableCell, TableFooter, TableHead, TableHeader,
};
use gpui_kit::component::time_field::{TimeField, TimeFieldState};
use gpui_kit::component::TitleBar;
use gpui_kit::component::pagination::Pagination;
use gpui_kit::component::progress::{Progress, ProgressCircle};
use gpui_kit::component::radio::{Radio, RadioGroup};
use gpui_kit::component::rating::Rating;
use gpui_kit::component::resizable::{ResizablePanel, h_resizable, resizable_panel, v_resizable};
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::separator::Separator;
use gpui_kit::component::sidebar::{Sidebar, SidebarCollapsible, SidebarMenu, SidebarMenuItem};
use gpui_kit::component::skeleton::Skeleton;
use gpui_kit::component::slider::{Slider, SliderEvent, SliderState, SliderValue};
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::status_bar::StatusBar;
use gpui_kit::component::stepper::{Stepper, StepperItem};
use gpui_kit::component::switch::Switch;
use gpui_kit::component::tab::{Tab, TabBar, TabVariant};
use gpui_kit::component::tag::{Tag, TagVariant};
use gpui_kit::component::calendar::{Calendar, CalendarState};
use gpui_kit::component::date_picker::{DatePicker, DatePickerState};
use gpui_kit::component::toolbar::Toolbar;
use gpui_kit::component::tree::{Tree, TreeItem, TreeState};
use gpui_kit::component::{ActiveTheme, Disableable, Icon, IndexPath, Sizable, Size, v_flex};
use gpui_kit::{prelude::*, *};
use serde_json::Value as Json;

/// Called with `hook(args)` when an event fires and no hook is registered for its name.
pub type HookSink = Rc<dyn Fn(&str, &mut ForgeView, &mut Context<ForgeView>)>;

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
            // The entity is leased (that's why the update panicked) — defer the
            // report so it lands after the lease releases and forces a repaint.
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
/// Called with the whole view state after a two-way binding writes to it.
pub type StateSink = Rc<dyn Fn(&Json, &mut ForgeView, &mut Context<ForgeView>)>;
/// Rust logic behind `on <event>: name(args)` — the hook registry (#52).
pub type HookHandler = Rc<dyn Fn(&[String], &mut ForgeView, &mut Context<ForgeView>)>;

type Options = Vec<SharedString>;

pub struct ForgeView {
    doc: Option<Document>,
    catalogs: Vec<Catalog>,
    state: Json,
    inputs: HashMap<String, Entity<InputState>>,
    textareas: HashMap<String, Entity<TextareaState>>,
    editors: HashMap<String, Entity<EditorState>>,
    sliders: HashMap<String, Entity<SliderState>>,
    color_pickers: HashMap<String, Entity<ColorPickerState>>,
    otps: HashMap<String, Entity<OtpState>>,
    date_pickers: HashMap<String, Entity<DatePickerState>>,
    calendars: HashMap<String, Entity<CalendarState>>,
    time_fields: HashMap<String, Entity<TimeFieldState>>,
    lists: HashMap<String, Entity<ListState<ForgeListDelegate>>>,
    trees: HashMap<String, Entity<TreeState>>,
    selects: HashMap<String, Entity<SelectState<Options>>>,
    tabs: HashMap<String, usize>,
    previews: HashMap<String, Entity<ForgeView>>,
    hooks: HashMap<String, HookHandler>,
    on_hook: Option<HookSink>,
    on_state: Option<StateSink>,
    subscriptions: Vec<Subscription>,
    /// Render-time problems surfaced for the host — `take_errors` drains them.
    errors: Vec<String>,
    /// The directory of the document this view renders — `src` paths on
    /// `Svg`/image elements resolve relative to it.
    base_dir: Option<std::path::PathBuf>,
    /// Shown while there is no document (e.g. inside `forge.Preview`).
    empty_text: Option<String>,
}

/// Walks `json` down a spec `Path` — the read side of `set_path`.
pub fn get_path<'a>(json: &'a Json, path: &Path) -> Option<&'a Json> {
    let mut current = json;
    for segment in &path.0 {
        current = match segment {
            PathSegment::Field(field) => current.get(field.as_str())?,
            PathSegment::Index(index) => current.get(index)?,
        };
    }
    Some(current)
}

/// A `ListDelegate` that renders `List` children on demand — `render_item` is called
/// several times per row per paint (measure then paint), so rows are rebuilt per call
/// through the owning view rather than handed out once.
struct ForgeListDelegate {
    view: WeakEntity<ForgeView>,
    key: String,
    children: Vec<Node>,
    selected: Rc<std::cell::Cell<Option<IndexPath>>>,
}

impl ListDelegate for ForgeListDelegate {
    type Item = ListItem;

    fn items_count(&self, _: usize, _: &gpui_kit::App) -> usize {
        self.children.len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let node = self.children.get(ix.row)?.clone();
        let key = format!("{}/{}", self.key, ix.row);
        try_update(&self.view, cx, "list render", |view, cx| {
            ListItem::new(ix).child(view.render_node(&node, &key, window, cx))
        })
        .ok()
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) {
        self.selected.set(ix);
    }
}

impl ForgeView {
    pub fn new(catalogs: Vec<Catalog>) -> Self {
        Self {
            doc: None,
            catalogs,
            state: Json::Object(Default::default()),
            inputs: HashMap::new(),
            textareas: HashMap::new(),
            editors: HashMap::new(),
            sliders: HashMap::new(),
            color_pickers: HashMap::new(),
            otps: HashMap::new(),
            date_pickers: HashMap::new(),
            calendars: HashMap::new(),
            time_fields: HashMap::new(),
            lists: HashMap::new(),
            trees: HashMap::new(),
            selects: HashMap::new(),
            tabs: HashMap::new(),
            previews: HashMap::new(),
            hooks: HashMap::new(),
            on_hook: None,
            on_state: None,
            subscriptions: Vec::new(),
            errors: Vec::new(),
            base_dir: None,
            empty_text: None,
        }
    }

    pub fn set_document(&mut self, doc: Document, cx: &mut Context<Self>) {
        self.doc = Some(doc);
        cx.notify();
    }

    pub fn set_state(&mut self, state: Json, cx: &mut Context<Self>) {
        self.state = state;
        // Inputs and selects copy their value once; rebuild them from the new state.
        self.inputs.clear();
        self.textareas.clear();
        self.editors.clear();
        self.sliders.clear();
        self.color_pickers.clear();
        self.otps.clear();
        self.date_pickers.clear();
        self.calendars.clear();
        self.time_fields.clear();
        self.lists.clear();
        self.trees.clear();
        self.selects.clear();
        self.subscriptions.clear();
        cx.notify();
    }

    /// The view state that bindings read from and write to.
    pub fn state(&self) -> &Json {
        &self.state
    }

    /// Writes `value` at `path` in the view state without rebuilding inputs.
    /// The parsed document this view renders, if any.
    pub fn document(&self) -> Option<&Document> {
        self.doc.as_ref()
    }

    /// Drops a cached input/editor state so the next render rebuilds it from bound
    /// state — pushes generated text into the source pane after a model edit.
    pub fn reset_inputs(&mut self, key: &str) {
        self.inputs.remove(key);
        self.textareas.remove(key);
        self.editors.remove(key);
    }

    pub fn update_state(&mut self, path: &Path, value: Json, cx: &mut Context<Self>) {
        self.write(path, value, cx);
    }

    /// Text shown while the view has no document (used by `forge.Preview`).
    pub fn set_empty_text(&mut self, text: Option<String>) {
        self.empty_text = text;
    }

    /// Registers `handler` for `on <event>: name(args)` bindings. Handlers may edit this
    /// view (state, previews) freely; unregistered names are reported to the hook sink.
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

    fn fire(&mut self, node: &Node, event: &str, cx: &mut Context<Self>) {
        let Some(binding) = node.events.iter().find(|e| e.event == event) else {
            return;
        };
        let args: Vec<String> = binding.handler.args.iter().map(|a| self.text(a)).collect();
        let name = binding.handler.name.clone();
        // Hook handlers may `entity.update` — possibly reentrantly — so a panic is
        // caught and reported to the host instead of killing the window.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if let Some(handler) = self.hooks.get(&name).cloned() {
                handler(&args, self, cx);
            } else if let Some(sink) = self.on_hook.clone() {
                let call = format!("{}({})", name, args.join(", "));
                sink(&call, self, cx);
            }
        }));
        if let Err(payload) = result {
            let detail = payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown panic".to_owned());
            self.report(format!("hook `{name}` failed: {detail}"));
            cx.notify();
        }
    }

    fn write(&mut self, path: &Path, value: Json, cx: &mut Context<Self>) {
        set_path(&mut self.state, path, value);
        if let Some(sink) = self.on_state.clone() {
            let state = self.state.clone();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                sink(&state, self, cx);
            }));
            if let Err(payload) = result {
                let detail = payload
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_else(|| "unknown panic".to_owned());
                self.report(format!("state sink failed: {detail}"));
            }
        }
        cx.notify();
    }

    fn lookup(&self, path: &Path) -> Option<&Json> {
        path.0
            .iter()
            .try_fold(&self.state, |json, segment| match segment {
                PathSegment::Field(name) => json.get(name),
                PathSegment::Index(i) => json.get(i),
            })
    }

    fn text(&self, value: &Value) -> String {
        match value {
            Value::String(s) | Value::Enum(s) => s.clone(),
            Value::Interpolated(segments) => segments
                .iter()
                .map(|s| match s {
                    Segment::Text(t) => t.clone(),
                    Segment::Binding(p) => self.json_text(p),
                })
                .collect(),
            Value::Number { value, unit } => {
                format!("{value}{}", unit.map(Unit::as_str).unwrap_or_default())
            }
            Value::Bool(b) => b.to_string(),
            Value::Color(c) => c.to_string(),
            Value::Token(p) => format!("$theme.{p}"),
            Value::List(items) => items
                .iter()
                .map(|i| self.text(i))
                .collect::<Vec<_>>()
                .join(", "),
            Value::Map(entries) => {
                let items = entries
                    .iter()
                    .map(|(k, v)| format!("{k}: {}", self.text(v)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{items}}}")
            }
            Value::Binding { path, .. } => self.json_text(path),
        }
    }

    fn json_text(&self, path: &Path) -> String {
        match self.lookup(path) {
            Some(Json::String(s)) => s.clone(),
            Some(Json::Null) | None => String::new(),
            Some(other) => other.to_string(),
        }
    }

    fn truthy(&self, value: &Value) -> bool {
        match value {
            Value::Bool(b) => *b,
            Value::Binding { path, .. } => match self.lookup(path) {
                Some(Json::Bool(b)) => *b,
                Some(Json::Null) | None => false,
                Some(Json::String(s)) => !s.is_empty(),
                Some(Json::Number(n)) => n.as_f64() != Some(0.0),
                Some(_) => true,
            },
            _ => false,
        }
    }

    fn number(&self, value: &Value) -> Option<f64> {
        match value {
            Value::Number { value, .. } => Some(*value),
            Value::Binding { path, .. } => self.lookup(path).and_then(Json::as_f64),
            _ => None,
        }
    }

    fn list(&self, value: &Value) -> Vec<String> {
        match value {
            Value::List(items) => items.iter().map(|i| self.text(i)).collect(),
            Value::Binding { path, .. } => match self.lookup(path) {
                Some(Json::Array(items)) => items
                    .iter()
                    .map(|i| i.as_str().map_or_else(|| i.to_string(), str::to_owned))
                    .collect(),
                _ => Vec::new(),
            },
            other => vec![self.text(other)],
        }
    }

    /// A prop's value: the node's content when the catalog maps it to this prop, then the
    /// written prop, then the catalog default.
    fn prop<'a>(&'a self, node: &'a Node, name: &str) -> Option<&'a Value> {
        let def = self
            .catalogs
            .first()
            .and_then(|c| c.element(&node.element.name));
        if let Some(content) = &node.content
            && def.and_then(|d| d.content.as_deref()) == Some(name)
        {
            return Some(content);
        }
        node.props
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v)
            .or_else(|| def?.prop(name)?.default.as_ref())
    }

    /// Builds an `Avatar` from a node — used directly and per child of `AvatarGroup`.
    fn avatar(&self, node: &Node) -> Avatar {
        let mut avatar = Avatar::new();
        if let Some(name) = self.prop_text(node, "name") {
            avatar = avatar.name(name);
        }
        if let Some(src) = self.prop_text(node, "src") {
            avatar = avatar.src(src);
        }
        if let Some(name) = self.prop_text(node, "icon") {
            avatar = avatar.placeholder(icon(&name));
        }
        avatar
    }

    /// Renders a slot element's children — `MessageHeader`, `TableCaption`, etc.
    fn slot_children(
        &mut self,
        node: &Node,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        node.children
            .iter()
            .enumerate()
            .map(|(i, child)| self.render_node(child, &format!("{key}/{i}"), window, cx))
            .collect()
    }

    /// Builds a `TableHead` — shared by `Table` header cells and standalone nodes.
    fn table_head(&self, node: &Node) -> TableHead {
        let mut head = TableHead::new()
            .child(self.prop_text(node, "label").unwrap_or_default());
        if let Some(n) = self.prop(node, "col-span").and_then(|v| self.number(v)) {
            head = head.col_span(n as usize);
        }
        match self.prop_text(node, "text-align").as_deref() {
            Some("center") => head = head.text_center(),
            Some("right") => head = head.text_right(),
            _ => {}
        }
        head
    }

    /// Builds a `TableCell` — shared by `Table` body/footer cells and standalone nodes.
    fn table_cell(&self, node: &Node) -> TableCell {
        let mut cell = TableCell::new()
            .child(self.prop_text(node, "value").unwrap_or_default());
        if let Some(n) = self.prop(node, "col-span").and_then(|v| self.number(v)) {
            cell = cell.col_span(n as usize);
        }
        match self.prop_text(node, "text-align").as_deref() {
            Some("center") => cell = cell.text_center(),
            Some("right") => cell = cell.text_right(),
            _ => {}
        }
        cell
    }

    /// Writes a prop on the node at `path` in this view's document — inspector edits.
    /// `json` arrives from the bound field's state; the catalog prop type maps it to `Value`.
    /// `Json::Null` removes the prop.
    pub fn set_node_prop(&mut self, path: &str, name: &str, json: &Json, cx: &mut Context<Self>) {
        let Some(node) = self.doc.as_ref().and_then(|doc| doc.node_at(path)) else {
            return;
        };
        let (element, namespaced) = (node.element.name.clone(), node.element.namespace.is_some());
        let ty = if namespaced {
            None
        } else {
            self.catalogs
                .first()
                .and_then(|catalog| catalog.element(&element))
                .and_then(|def| def.prop(&name))
                .map(|prop| prop.ty.clone())
        };
        let value = if json.is_null() {
            None
        } else {
            ty.as_ref()
                .and_then(|ty| self.json_value(ty, json))
                .or_else(|| json.as_str().map(|s| Value::String(s.to_owned())))
        };
        if let Some(doc) = self.doc.as_mut() {
            doc.set_prop(path, name, value);
            cx.notify();
        }
    }

    /// `Style` tab write-through — `style_changed` → `doc.set_style`.
    pub fn set_node_style(
        &mut self,
        path: &str,
        name: &str,
        json: &Json,
        cx: &mut Context<Self>,
    ) {
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

    /// Records a render-time problem — surfaced to the host as a toast next paint.
    pub fn report(&mut self, message: impl Into<String>) {
        self.errors.push(message.into());
    }

    /// Sets the directory the current document lives in — relative `src` paths
    /// on `Svg`/image elements resolve against it.
    pub fn set_base_dir(&mut self, dir: impl Into<std::path::PathBuf>) {
        self.base_dir = Some(dir.into());
    }

    /// Resolves `src` against `base_dir` when relative — `std::fs` calls use this
    /// so `src` is document-relative, not cwd-relative.
    fn resolve_path(&self, src: &str) -> std::path::PathBuf {
        let path = std::path::Path::new(src);
        if path.is_relative()
            && let Some(base) = &self.base_dir
        {
            return base.join(path);
        }
        path.to_path_buf()
    }

    /// Drains accumulated errors — the host captures them to show or handle.
    pub fn take_errors(&mut self) -> Vec<String> {
        let mut errors = std::mem::take(&mut self.errors);
        errors.append(
            &mut PENDING_ERRORS
                .lock()
                .unwrap_or_else(|e| e.into_inner()),
        );
        errors
    }

    /// Maps a state `Json` to a spec `Value` via the catalog prop type.
    fn json_value(&self, ty: &PropType, json: &Json) -> Option<Value> {
        match json {
            Json::Bool(b) => Some(Value::Bool(*b)),
            Json::Number(n) => n.as_f64().map(|value| Value::Number { value, unit: None }),
            Json::String(s) => match ty {
                PropType::Enum(_) => Some(Value::Enum(s.clone())),
                PropType::Number => s
                    .parse()
                    .ok()
                    .map(|value| Value::Number { value, unit: None }),
                PropType::Bool => s.parse().ok().map(Value::Bool),
                PropType::Color => Color::parse_hex(s).map(Value::Color),
                _ => Some(Value::String(s.clone())),
            },
            _ => None,
        }
    }

    /// Maps a prop `Value` back to state `Json` for seeding bound fields.
    fn value_json(&self, value: &Value) -> Json {
        match value {
            Value::Bool(b) => Json::Bool(*b),
            Value::Number { value, .. } => Json::from(*value),
            Value::String(s) | Value::Enum(s) => Json::String(s.clone()),
            other => Json::String(self.text(other)),
        }
    }

    /// Renders `forge.Inspector` — a catalog-driven `Form` of the selected node's props.
    /// Each field binds `document.edits.<prop>`; `prop_changed` writes through to the doc.
    fn inspector(
        &mut self,
        target: &Node,
        path: &str,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        // Re-seed `document.edits.*` with the target's prop values whenever the selected
        // node changes — the guard keeps this to one extra paint per selection.
        let seed_path = parse_path("document.edits.__path").expect("static path");
        let seeded = get_path(self.state(), &seed_path)
            .and_then(|j| j.as_str())
            .unwrap_or_default()
            .to_owned();
        let props = self
            .catalogs
            .first()
            .and_then(|c| c.element(&target.element.name))
            .map(|def| def.props.clone())
            .unwrap_or_default();
        if seeded != *path {
            self.update_state(&seed_path, Json::String(path.to_owned()), cx);
            for prop in &props {
                let json = target
                    .props
                    .iter()
                    .find(|(k, _)| k == &prop.name)
                    .map(|(_, v)| self.value_json(v))
                    .unwrap_or(Json::Null);
                if let Ok(edit_path) = parse_path(&format!("document.edits.{}", prop.name)) {
                    self.update_state(&edit_path, json, cx);
                }
            }
            for (name, value) in &target.style {
                let json = self.value_json(value);
                match parse_path(&format!("document.edits.style.{name}")) {
                    Ok(edit_path) => self.update_state(&edit_path, json, cx),
                    Err(e) => self.report(format!("style `{name}` skipped: {e}")),
                }
            }
        }
        let mut rows = vec![
            DescriptionItem::new("element").value(target.element.to_string()),
            DescriptionItem::new("path").value(path.to_owned()),
        ];
        if let Some(id) = &target.id {
            rows.push(DescriptionItem::new("id").value(id.clone()));
        }
        let header = DescriptionList::vertical().children(rows);
        // One `FormField` per catalog prop, each binding `document.edits.<prop>` and
        // firing `prop_changed(<prop>)` — the app writes the new value into the doc.
        let fields: Vec<Node> = props
            .iter()
            .filter_map(|prop| {
                let path = match parse_path(&format!("document.edits.{}", prop.name)) {
                    Ok(path) => path,
                    Err(e) => {
                        self.report(format!("prop `{}` skipped: {e}", prop.name));
                        return None;
                    }
                };
                let bind = Value::Binding {
                    path,
                    mode: BindingMode::TwoWay,
                };
                let (element, bind_prop, extra) = match &prop.ty {
                    PropType::Bool => ("Switch", "checked", vec![]),
                    PropType::Enum(values) => (
                        "Select",
                        "value",
                        vec![(
                            "options".to_owned(),
                            Value::List(
                                values.iter().map(|v| Value::Enum(v.clone())).collect(),
                            ),
                        )],
                    ),
                    PropType::Number => ("NumberInput", "value", vec![]),
                    PropType::Color => ("ColorPicker", "value", vec![]),
                    _ => ("Input", "value", vec![]),
                };
                let mut props = vec![(bind_prop.to_owned(), bind)];
                props.extend(extra);
                let control = Node {
                    element: ElementRef { namespace: None, name: element.into() },
                    props,
                    events: vec![EventBinding {
                        event: "change".into(),
                        handler: HandlerRef {
                            name: "prop_changed".into(),
                            args: vec![Value::String(prop.name.clone())],
                        },
                    }],
                    ..Default::default()
                };
                Some(Node {
                    element: ElementRef { namespace: None, name: "FormField".into() },
                    props: vec![("label".to_owned(), Value::String(prop.name.clone()))],
                    children: vec![control],
                    ..Default::default()
                })
            })
            .collect();
        let prop_children = if fields.is_empty() {
            vec![Node {
                element: ElementRef { namespace: None, name: "Text".into() },
                content: Some(Value::String("no catalog props".into())),
                ..Default::default()
            }]
        } else {
            vec![Node {
                element: ElementRef { namespace: None, name: "Form".into() },
                props: vec![("label-layout".to_owned(), Value::Enum("Vertical".into()))],
                children: fields,
                ..Default::default()
            }]
        };
        // Style tab — one `Input` per `style:` entry, bound `document.edits.style.<name>`.
        let style_children = if target.style.is_empty() {
            vec![Node {
                element: ElementRef { namespace: None, name: "Text".into() },
                content: Some(Value::String("no style entries".into())),
                ..Default::default()
            }]
        } else {
            vec![Node {
                element: ElementRef { namespace: None, name: "Form".into() },
                props: vec![("label-layout".to_owned(), Value::Enum("Vertical".into()))],
                children: target
                    .style
                    .iter()
                    .filter_map(|(name, _)| {
                        let path = match parse_path(&format!("document.edits.style.{name}")) {
                            Ok(path) => path,
                            Err(e) => {
                                self.report(format!("style `{name}` skipped: {e}"));
                                return None;
                            }
                        };
                        let control = Node {
                            element: ElementRef { namespace: None, name: "Input".into() },
                            props: vec![(
                                "value".to_owned(),
                                Value::Binding {
                                    path,
                                    mode: BindingMode::TwoWay,
                                },
                            )],
                            events: vec![EventBinding {
                                event: "change".into(),
                                handler: HandlerRef {
                                    name: "style_changed".into(),
                                    args: vec![Value::String(name.clone())],
                                },
                            }],
                            ..Default::default()
                        };
                        Some(Node {
                            element: ElementRef {
                                namespace: None,
                                name: "FormField".into(),
                            },
                            props: vec![("label".to_owned(), Value::String(name.clone()))],
                            children: vec![control],
                            ..Default::default()
                        })
                    })
                    .collect(),
                ..Default::default()
            }]
        };
        let tabs = Node {
            element: ElementRef { namespace: None, name: "Tabs".into() },
            children: vec![
                Node {
                    element: ElementRef { namespace: None, name: "Tab".into() },
                    props: vec![("label".to_owned(), Value::String("Properties".into()))],
                    children: prop_children,
                    ..Default::default()
                },
                Node {
                    element: ElementRef { namespace: None, name: "Tab".into() },
                    props: vec![("label".to_owned(), Value::String("Style".into()))],
                    children: style_children,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let tabs_key = format!("{key}/{path}");
        let tabs = self.render_node(&tabs, &tabs_key, window, cx);
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(header)
            .child(tabs)
            .into_any_element()
    }

    /// Renders `forge.Outline` — a `Tree` of the document's nodes; rows carry a click
    /// handler that writes `bind selected` and fires `select` with the node index path.
    fn outline(
        &mut self,
        node: &Node,
        key: &str,
        doc: &Document,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let sel = self.two_way(node, "selected");
        let mut items = Vec::new();
        self.outline_data(&doc.root, "0".to_owned(), &mut items);
        let state = match self.trees.get(key) {
            Some(state) => {
                state.update(cx, |state, cx| {
                    *state = std::mem::replace(state, TreeState::new(cx)).items(items.clone());
                });
                state.clone()
            }
            None => {
                let state = cx.new(|cx| TreeState::new(cx).items(items));
                self.trees.insert(key.to_owned(), state.clone());
                state
            }
        };
        // `render_item` runs several times per row per paint (measure then paint), so the
        // row is rebuilt inside the closure — nothing is handed out once and consumed.
        let weak = cx.entity().downgrade();
        let outline = node.clone();
        Tree::new(&state, move |ix, entry, selected, _, _cx| {
            let mut item = ListItem::new(ix);
            let id = entry.item().id.to_string();
            let sel = sel.clone();
            let outline = outline.clone();
            let weak = weak.clone();
            let row = div()
                .flex()
                .flex_row()
                .items_center()
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    try_update(&weak, cx, "event", |this, cx| {
                        if let Some(path) = &sel {
                            this.write(path, Json::String(id.clone()), cx);
                        }
                        this.fire(&outline, "select", cx);
                    })
                    .ok();
                })
                .child(entry.item().label.to_string());
            item = item.child(row);
            item.selected(selected)
        })
        .into_any_element()
    }

    /// One `TreeItem` per document node, recursing through `node.children`. The row is
    /// rebuilt per paint from `entry.item()` — id is the node's index path.
    fn outline_data(&mut self, node: &Node, path: String, items: &mut Vec<TreeItem>) {
        let mut label = node.element.name.clone();
        if let Some(id) = &node.id {
            label = format!("{label} #{id}");
        }
        if let Some(content) = &node.content {
            let text = self.text(content);
            if !text.is_empty() {
                label = format!("{label} · {text}");
            }
        }
        let mut data = TreeItem::new(&path, &label).expanded(true);
        let mut subs = Vec::new();
        for (i, child) in node.children.iter().enumerate() {
            self.outline_data(child, format!("{path}.{i}"), &mut subs);
        }
        if !subs.is_empty() {
            data = data.children(subs);
        }
        items.push(data);
    }

    /// Builds `TreeItem` data recursively; each node's row content is pre-rendered into
    /// `map` under its id for the `render_item` closure to hand out per paint.
    fn tree_data(
        &mut self,
        node: &Node,
        key: &str,
        nodes: &mut HashMap<String, Node>,
    ) -> TreeItem {
        let id = node
            .id
            .clone()
            .unwrap_or_else(|| key.to_owned());
        let label = self.prop_text(node, "label").unwrap_or_default();
        let mut data = TreeItem::new(&id, &label);
        if self.prop_bool(node, "expanded") {
            data = data.expanded(true);
        }
        nodes.insert(id.clone(), node.clone());
        let mut subs = Vec::new();
        for (i, child) in node.children.iter().enumerate() {
            if child.element.name.as_str() == "TreeItem" {
                subs.push(self.tree_data(child, &format!("{key}/{i}"), nodes));
            }
        }
        if !subs.is_empty() {
            data = data.children(subs);
        }
        data
    }

    /// Builds a `SidebarMenuItem` — recursively nests `SidebarMenuItem` children.
    fn sidebar_item(
        &mut self,
        node: &Node,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> SidebarMenuItem {
        let kids: Vec<SidebarMenuItem> = node
            .children
            .iter()
            .enumerate()
            .filter(|(_, c)| c.element.name.as_str() == "SidebarMenuItem")
            .map(|(i, c)| self.sidebar_item(c, &format!("{key}/{i}"), window, cx))
            .collect();
        let mut item = SidebarMenuItem::new(self.prop_text(node, "label").unwrap_or_default())
            .active(self.prop_bool(node, "active"))
            .disable(self.prop_bool(node, "disabled"))
            .default_open(self.prop_bool(node, "open"));
        if let Some(name) = self.prop_text(node, "icon") {
            item = item.icon(icon(&name));
        }
        if !kids.is_empty() {
            item = item.children(kids);
        }
        let weak = cx.entity().downgrade();
        let clicked = node.clone();
        item.on_click(move |_, _, cx| {
            try_update(&weak, cx, "event", |this, cx| this.fire(&clicked, "click", cx)).ok();
        })
    }

    /// Builds a `Field` from a `FormField` node — used by `Form` and standalone fields.
    fn form_field(
        &mut self,
        node: &Node,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Field {
        let kids: Vec<AnyElement> = node
            .children
            .iter()
            .enumerate()
            .map(|(i, child)| self.render_node(child, &format!("{key}/{i}"), window, cx))
            .collect();
        let mut field = Field::new().children(kids);
        if let Some(label) = self.prop_text(node, "label") {
            field = field.label(label);
        }
        if let Some(description) = self.prop_text(node, "description") {
            field = field.description(description);
        }
        field = field
            .required(self.prop_bool(node, "required"))
            .visible(!matches!(self.prop(node, "visible"), Some(Value::Bool(false))))
            .label_indent(self.prop_bool(node, "label-indent"));
        match self.prop_text(node, "align").as_deref() {
            Some("center") => field = field.items_center(),
            Some("end") => field = field.items_end(),
            _ => {}
        }
        if let Some(n) = self.prop(node, "col-span").and_then(|v| self.number(v)) {
            field = field.col_span(n as u16);
        }
        if let Some(n) = self.prop(node, "col-start").and_then(|v| self.number(v)) {
            field = field.col_start(n as i16);
        }
        field
    }

    fn prop_text(&self, node: &Node, name: &str) -> Option<String> {
        self.prop(node, name).map(|v| self.text(v))
    }

    fn prop_bool(&self, node: &Node, name: &str) -> bool {
        self.prop(node, name).is_some_and(|v| self.truthy(v))
    }

    fn two_way(&self, node: &Node, name: &str) -> Option<Path> {
        match self.prop(node, name)? {
            Value::Binding {
                path,
                mode: BindingMode::TwoWay,
            } => Some(path.clone()),
            _ => None,
        }
    }

    fn render_node(
        &mut self,
        node: &Node,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if let Some(namespace) = &node.element.namespace {
            let library = self
                .doc
                .as_ref()
                .and_then(|doc| doc.imports.iter().find(|i| &i.alias == namespace))
                .map(|import| import.library.as_str());
            let element: AnyElement = match (library, node.element.name.as_str()) {
                (Some("forge"), "Preview") => {
                    let id = node.id.clone().unwrap_or_else(|| key.to_owned());
                    let empty_text = self.prop_text(node, "placeholder");
                    let catalogs = self.catalogs.clone();
                    self.previews
                        .entry(id)
                        .or_insert_with(|| {
                            cx.new(|_| {
                                let mut view = ForgeView::new(catalogs);
                                view.empty_text = empty_text;
                                view
                            })
                        })
                        .clone()
                        .into_any_element()
                }
                (Some("forge"), "Outline") => {
                    let of = self.prop_text(node, "of").unwrap_or_default();
                    let empty_text = self
                        .prop_text(node, "placeholder")
                        .unwrap_or_else(|| "no document".into());
                    let doc = self
                        .previews
                        .get(&of)
                        .and_then(|preview| preview.read(cx).doc.clone());
                    match doc {
                        Some(doc) => self.outline(node, key, &doc, cx),
                        None => placeholder(empty_text, cx),
                    }
                }
                (Some("forge"), "Inspector") => {
                    let of = self.prop_text(node, "of").unwrap_or_default();
                    let path = self.prop_text(node, "selected").unwrap_or_default();
                    let empty_text = self
                        .prop_text(node, "placeholder")
                        .unwrap_or_else(|| "nothing selected".into());
                    let target = self
                        .previews
                        .get(&of)
                        .and_then(|preview| preview.read(cx).doc.clone())
                        .and_then(|doc| doc.node_at(&path).cloned());
                    match target {
                        Some(target) => self.inspector(&target, &path, key, window, cx),
                        None => placeholder(empty_text, cx),
                    }
                }
                _ => placeholder(
                    format!("`{namespace}.{}` is not provided by this adapter yet", node.element.name),
                    cx,
                ),
            };
            return wrap(
            element,
            &node.style,
            &format!("forge-{key}"),
            self.prop_text(node, "tooltip"),
            cx,
        );
        }
        let id = SharedString::from(format!("forge-{key}"));
        let weak = cx.entity().downgrade();
        let children = |this: &mut Self, window: &mut Window, cx: &mut Context<Self>| {
            node.children
                .iter()
                .enumerate()
                .map(|(i, child)| this.render_node(child, &format!("{key}.{i}"), window, cx))
                .collect::<Vec<_>>()
        };
        let element: AnyElement = match node.element.name.as_str() {
            "Box" => {
                let kids = children(self, window, cx);
                return container(div(), &node.style, id, kids, cx);
            }
            "VStack" => {
                let kids = children(self, window, cx);
                return container(v_flex(), &node.style, id, kids, cx);
            }
            // `h_flex()` centers children on the cross axis; a Forge `HStack` is a plain
            // flex row, so children stretch (use `align: center` to center them).
            "HStack" => {
                let kids = children(self, window, cx);
                return container(div().flex().flex_row(), &node.style, id, kids, cx);
            }
            "Text" => {
                let text = self.prop_text(node, "text").unwrap_or_default();
                return style(div(), &node.style, cx).child(text).into_any_element();
            }
            "Label" => Label::new(self.prop_text(node, "text").unwrap_or_default())
                .when_some(self.prop_text(node, "secondary"), |l, s| l.secondary(s))
                .masked(self.prop_bool(node, "masked"))
                .into_any_element(),
            "Button" => {
                let clicked = node.clone();
                Button::new(id)
                    .label(self.prop_text(node, "label").unwrap_or_default())
                    .with_variant(button_variant(
                        &self.prop_text(node, "variant").unwrap_or_default(),
                    ))
                    .with_size(size(&self.prop_text(node, "size").unwrap_or_default()))
                    .when_some(self.prop_text(node, "icon"), |b, name| b.icon(icon(&name)))
                    .when_some(self.prop_text(node, "tooltip"), |b, t| b.tooltip(t))
                    .disabled(self.prop_bool(node, "disabled"))
                    .loading(self.prop_bool(node, "loading"))
                    .when(self.prop_bool(node, "outline"), |b| b.outline())
                    .when(self.prop_bool(node, "compact"), |b| b.compact())
                    .on_click(move |_, _, cx| {
                        try_update(&weak, cx, "event", |this, cx| this.fire(&clicked, "click", cx)).ok();
                    })
                    .into_any_element()
            }
            "DropdownMenu" => {
                // The popover builder runs inside a `Context<PopupMenu>` at open time,
                // so `MenuItem` props are baked here and clicks go back through `weak`.
                let weak2 = weak.clone();
                let mut button = Button::new(SharedString::from(format!("{id}-menu")))
                    .label(self.prop_text(node, "label").unwrap_or_default())
                    .with_variant(button_variant(
                        &self.prop_text(node, "variant").unwrap_or_default(),
                    ))
                    .with_size(size(&self.prop_text(node, "size").unwrap_or_default()))
                    .disabled(self.prop_bool(node, "disabled"));
                if let Some(name) = self.prop_text(node, "icon") {
                    button = button.icon(icon(&name));
                }
                let items: Vec<(String, Option<String>, bool, bool, Node)> = node
                    .children
                    .iter()
                    .map(|c| {
                        let label = self
                            .prop_text(c, "label")
                            .or_else(|| c.content.as_ref().map(|v| self.text(v)))
                            .unwrap_or_default();
                        (
                            label,
                            self.prop_text(c, "icon"),
                            self.prop_bool(c, "disabled"),
                            self.prop_bool(c, "checked"),
                            c.clone(),
                        )
                    })
                    .collect();
                button
                    .dropdown_menu(move |menu, _, _| {
                        let mut menu = menu;
                        for (label, icon_name, disabled, checked, child) in &items {
                            if child.element.name.as_str() == "MenuSeparator" {
                                menu = menu.separator();
                                continue;
                            }
                            let mut item = PopupMenuItem::new(label.clone());
                            if let Some(name) = icon_name {
                                item = item.icon(icon(name));
                            }
                            item = item.disabled(*disabled).checked(*checked);
                            let weak = weak2.clone();
                            let child = child.clone();
                            item = item.on_click(move |_, _, cx| {
                                try_update(&weak, cx, "event", |this, cx| this.fire(&child, "click", cx))
                                    .ok();
                            });
                            menu = menu.item(item);
                        }
                        menu
                    })
                    .into_any_element()
            }
            "Input" => {
                let placeholder = self.prop_text(node, "placeholder").unwrap_or_default();
                let masked = self.prop_bool(node, "masked");
                let state_key = format!("{key}|{placeholder}|{masked}");
                let state = match self.inputs.get(&state_key) {
                    Some(state) => state.clone(),
                    None => {
                        let value = self.prop_text(node, "value").unwrap_or_default();
                        let state = cx.new(|cx| {
                            InputState::new(window, cx)
                                .placeholder(placeholder)
                                .masked(masked)
                                .default_value(value)
                        });
                        let path = self.two_way(node, "value");
                        let changed = node.clone();
                        let subscription =
                            cx.subscribe(&state, move |this, state, event, cx| match event {
                                InputEvent::Change => {
                                    if let Some(path) = &path {
                                        let value = state.read(cx).value().to_string();
                                        this.write(path, Json::String(value), cx);
                                    }
                                    this.fire(&changed, "change", cx);
                                }
                                InputEvent::PressEnter { .. } => this.fire(&changed, "submit", cx),
                                _ => {}
                            });
                        self.subscriptions.push(subscription);
                        self.inputs.insert(state_key, state.clone());
                        state
                    }
                };
                Input::new(&state)
                    .cleanable(self.prop_bool(node, "cleanable"))
                    .disabled(self.prop_bool(node, "disabled"))
                    .into_any_element()
            }
            "Textarea" => {
                let placeholder = self.prop_text(node, "placeholder").unwrap_or_default();
                let state_key = format!("{key}|{placeholder}");
                let state = match self.textareas.get(&state_key) {
                    Some(state) => state.clone(),
                    None => {
                        let value = self.prop_text(node, "value").unwrap_or_default();
                        let rows = self.prop(node, "rows").and_then(|v| self.number(v));
                        let state = cx.new(|cx| {
                            let state = TextareaState::new(window, cx)
                                .placeholder(placeholder)
                                .default_value(value);
                            match rows {
                                Some(rows) => state.rows(rows as usize),
                                None => state,
                            }
                        });
                        let path = self.two_way(node, "value");
                        let changed = node.clone();
                        let subscription =
                            cx.subscribe(&state, move |this, state, event, cx| match event {
                                InputEvent::Change => {
                                    if let Some(path) = &path {
                                        let value = state.read(cx).value().to_string();
                                        this.write(path, Json::String(value), cx);
                                    }
                                    this.fire(&changed, "change", cx);
                                }
                                _ => {}
                            });
                        self.subscriptions.push(subscription);
                        self.textareas.insert(state_key, state.clone());
                        state
                    }
                };
                let textarea = Textarea::new(&state)
                    .disabled(self.prop_bool(node, "disabled"))
                    .readonly(self.prop_bool(node, "readonly"));
                match self.prop(node, "height").and_then(length) {
                    Some(height) => textarea.h(height).into_any_element(),
                    None => textarea.into_any_element(),
                }
            }
            "NumberInput" => {
                let placeholder = self.prop_text(node, "placeholder").unwrap_or_default();
                let state = match self.inputs.get(key) {
                    Some(state) => state.clone(),
                    None => {
                        let value = self.prop_text(node, "value").unwrap_or_default();
                        let state = cx.new(|cx| {
                            InputState::new(window, cx)
                                .placeholder(placeholder)
                                .default_value(value)
                        });
                        let path = self.two_way(node, "value");
                        let changed = node.clone();
                        let subscription =
                            cx.subscribe(&state, move |this, state, event, cx| match event {
                                InputEvent::Change => {
                                    if let Some(path) = &path {
                                        let value = state.read(cx).value().to_string();
                                        this.write(path, Json::String(value), cx);
                                    }
                                    this.fire(&changed, "change", cx);
                                }
                                _ => {}
                            });
                        self.subscriptions.push(subscription);
                        self.inputs.insert(key.to_owned(), state.clone());
                        state
                    }
                };
                NumberInput::new(&state)
                    .with_size(size(&self.prop_text(node, "size").unwrap_or_default()))
                    .appearance(!matches!(
                        self.prop(node, "appearance"),
                        Some(Value::Bool(false))
                    ))
                    .disabled(self.prop_bool(node, "disabled"))
                    .into_any_element()
            }
            "Editor" => {
                let placeholder = self.prop_text(node, "placeholder").unwrap_or_default();
                // `highlighting.default` overrides the flat `language` prop.
                let mut language = self.prop_text(node, "language").unwrap_or_default();
                if let Some(Value::Map(entries)) = self.prop(node, "highlighting")
                    && let Some((_, value)) = entries.iter().find(|(k, _)| k == "default")
                {
                    match value {
                        Value::String(s) | Value::Enum(s) => language = s.clone(),
                        _ => {}
                    }
                }
                let state_key = format!("{key}|{language}");
                let state = match self.editors.get(&state_key) {
                    Some(state) => state.clone(),
                    None => {
                        let value = self.prop_text(node, "value").unwrap_or_default();
                        let state = cx.new(|cx| {
                            let state = EditorState::new(window, cx)
                                .placeholder(placeholder)
                                .default_value(value);
                            if language.is_empty() {
                                state
                            } else {
                                state.language(language)
                            }
                        });
                        let path = self.two_way(node, "value");
                        let changed = node.clone();
                        let subscription =
                            cx.subscribe(&state, move |this, state, event, cx| match event {
                                InputEvent::Change => {
                                    if let Some(path) = &path {
                                        let value = state.read(cx).value().to_string();
                                        this.write(path, Json::String(value), cx);
                                    }
                                    this.fire(&changed, "change", cx);
                                }
                                _ => {}
                            });
                        self.subscriptions.push(subscription);
                        self.editors.insert(state_key, state.clone());
                        state
                    }
                };
                let editor = Editor::new(&state)
                    .disabled(self.prop_bool(node, "disabled"))
                    .readonly(self.prop_bool(node, "readonly"));
                match self.prop(node, "height").and_then(length) {
                    Some(height) => editor.h(height).into_any_element(),
                    None => editor.h_full().into_any_element(),
                }
            }
            "Checkbox" | "Switch" => {
                let label = self.prop_text(node, "label");
                let checked = self.prop_bool(node, "checked");
                let disabled = self.prop_bool(node, "disabled");
                let path = self.two_way(node, "checked");
                let changed = node.clone();
                let on_change = move |value: &bool, _: &mut Window, cx: &mut App| {
                    try_update(&weak, cx, "event", |this, cx| {
                        if let Some(path) = &path {
                            this.write(path, Json::Bool(*value), cx);
                        }
                        this.fire(&changed, "change", cx);
                    })
                    .ok();
                };
                if node.element.name == "Checkbox" {
                    Checkbox::new(id)
                        .when_some(label, |c, l| c.label(l))
                        .checked(checked)
                        .disabled(disabled)
                        .on_click(on_change)
                        .into_any_element()
                } else {
                    Switch::new(id)
                        .when_some(label, |s, l| s.label(l))
                        .checked(checked)
                        .disabled(disabled)
                        .on_click(on_change)
                        .into_any_element()
                }
            }
            "RadioGroup" => {
                let selected = self.prop(node, "selected").and_then(|v| self.number(v));
                let path = self.two_way(node, "selected");
                let changed = node.clone();
                let horizontal = self.prop_text(node, "layout").as_deref() == Some("Horizontal");
                let radios: Vec<Radio> = node
                    .children
                    .iter()
                    .enumerate()
                    .map(|(i, child)| {
                        Radio::new(SharedString::from(format!("forge-{key}.{i}")))
                            .label(self.prop_text(child, "label").unwrap_or_default())
                            .disabled(self.prop_bool(child, "disabled"))
                    })
                    .collect();
                RadioGroup::new(id)
                    .layout(if horizontal {
                        Axis::Horizontal
                    } else {
                        Axis::Vertical
                    })
                    .selected_index(selected.map(|n| n as usize))
                    .disabled(self.prop_bool(node, "disabled"))
                    .children(radios)
                    .on_click(move |index, _, cx| {
                        try_update(&weak, cx, "event", |this, cx| {
                            if let Some(path) = &path {
                                this.write(path, Json::from(*index), cx);
                            }
                            this.fire(&changed, "change", cx);
                        })
                        .ok();
                    })
                    .into_any_element()
            }
            "Select" => {
                let options: Options = self
                    .prop(node, "options")
                    .map(|v| self.list(v))
                    .unwrap_or_default()
                    .into_iter()
                    .map(SharedString::from)
                    .collect();
                let value = self.prop_text(node, "value").unwrap_or_default();
                let state_key = format!("{key}|{}", options.join("\u{1f}"));
                let state = match self.selects.get(&state_key) {
                    Some(state) => state.clone(),
                    None => {
                        let selected = options.iter().position(|o| *o == value).map(IndexPath::new);
                        let state = cx.new(|cx| SelectState::new(options, selected, window, cx));
                        let path = self.two_way(node, "value");
                        let changed = node.clone();
                        let subscription = cx.subscribe(
                            &state,
                            move |this, _, event: &SelectEvent<Options>, cx| {
                                let SelectEvent::Confirm(value) = event;
                                if let Some(path) = &path {
                                    let value = value.as_ref().map(|v| v.to_string());
                                    this.write(path, Json::from(value), cx);
                                }
                                this.fire(&changed, "change", cx);
                            },
                        );
                        self.subscriptions.push(subscription);
                        self.selects.insert(state_key, state.clone());
                        state
                    }
                };
                Select::new(&state)
                    .when_some(self.prop_text(node, "placeholder"), |s, p| s.placeholder(p))
                    .disabled(self.prop_bool(node, "disabled"))
                    .into_any_element()
            }
            "Tabs" => {
                let bound = self.prop(node, "selected").and_then(|v| self.number(v));
                let selected = bound
                    .map(|n| n as usize)
                    .or_else(|| self.tabs.get(key).copied())
                    .unwrap_or(0)
                    .min(node.children.len().saturating_sub(1));
                let path = self.two_way(node, "selected");
                let changed = node.clone();
                let tab_key = key.to_owned();
                let tabs: Vec<Tab> = node
                    .children
                    .iter()
                    .map(|child| {
                        Tab::new()
                            .label(self.prop_text(child, "label").unwrap_or_default())
                            .disabled(self.prop_bool(child, "disabled"))
                    })
                    .collect();
                let content = node.children.get(selected).map(|tab| {
                    tab.children
                        .iter()
                        .enumerate()
                        .map(|(i, child)| {
                            self.render_node(child, &format!("{key}.{selected}.{i}"), window, cx)
                        })
                        .collect::<Vec<_>>()
                });
                let bar = TabBar::new(id)
                    .with_variant(tab_variant(
                        &self.prop_text(node, "variant").unwrap_or_default(),
                    ))
                    .selected_index(selected)
                    .children(tabs)
                    .on_click(move |index: &usize, _, cx| {
                        let index = *index;
                        let _ = try_update(&weak, cx, "tab click", |this, cx| {
                            this.tabs.insert(tab_key.clone(), index);
                            if let Some(path) = &path {
                                this.write(path, Json::from(index), cx);
                            }
                            this.fire(&changed, "change", cx);
                            cx.notify();
                        });
                    });
                v_flex()
                    .gap_3()
                    .child(bar)
                    .child(
                        v_flex()
                            .flex_1()
                            .min_h_0()
                            .children(content.unwrap_or_default()),
                    )
                    .into_any_element()
            }
            "Separator" => {
                let separator = match self.prop_text(node, "orientation").as_deref() {
                    Some("Vertical") => Separator::vertical(),
                    _ => Separator::horizontal(),
                };
                separator
                    .when(self.prop_bool(node, "dashed"), |s| s.dashed())
                    .when_some(self.prop_text(node, "label"), |s, l| s.label(l))
                    .into_any_element()
            }
            "GroupBox" => {
                let kids = children(self, window, cx);
                return style(
                    GroupBox::new()
                        .id(id)
                        .with_variant(group_box_variant(
                            &self.prop_text(node, "variant").unwrap_or_default(),
                        ))
                        .when_some(self.prop_text(node, "title"), |g, t| g.title(t))
                        .when_some(self.prop_text(node, "footer"), |g, f| g.footer(f))
                        .children(kids),
                    &node.style,
                    cx,
                )
                .into_any_element();
            }
            "Empty" => {
                let kids = children(self, window, cx);
                let mut header = EmptyHeader::new();
                if let Some(name) = self.prop_text(node, "icon") {
                    header = header.media(
                        EmptyMedia::new()
                            .with_variant(EmptyMediaVariant::Icon)
                            .child(icon(&name)),
                    );
                }
                if let Some(title) = self.prop_text(node, "title") {
                    header = header.title(EmptyTitle::new().child(title));
                }
                if let Some(description) = self.prop_text(node, "description") {
                    header = header.description(EmptyDescription::new().child(description));
                }
                let mut empty = Empty::new().header(header);
                if !kids.is_empty() {
                    empty = empty.content(EmptyContent::new().children(kids));
                }
                empty.into_any_element()
            }
            "Link" => {
                let clicked = node.clone();
                Link::new(id)
                    .when_some(self.prop_text(node, "href"), |l, h| l.href(h))
                    .disabled(self.prop_bool(node, "disabled"))
                    .child(self.prop_text(node, "label").unwrap_or_default())
                    .on_click(move |_, _, cx| {
                        try_update(&weak, cx, "event", |this, cx| this.fire(&clicked, "click", cx)).ok();
                    })
                    .into_any_element()
            }
            "Badge" => {
                let count = self.prop(node, "count").and_then(|v| self.number(v));
                let max = self.prop(node, "max").and_then(|v| self.number(v));
                let color = self.prop(node, "color").and_then(|v| color(v, cx));
                let kids = children(self, window, cx);
                Badge::new()
                    .when_some(count, |b, n| b.count(n as usize))
                    .when_some(max, |b, n| b.max(n as usize))
                    .when(self.prop_bool(node, "dot"), |b| b.dot())
                    .when_some(color, |b, c| b.color(c))
                    .children(kids)
                    .into_any_element()
            }
            "Tag" => Tag::new()
                .with_variant(tag_variant(
                    &self.prop_text(node, "variant").unwrap_or_default(),
                ))
                .when(self.prop_bool(node, "outline"), |t| t.outline())
                .child(self.prop_text(node, "label").unwrap_or_default())
                .into_any_element(),
            "Icon" => {
                let color = self.prop(node, "color").and_then(|v| color(v, cx));
                let name = self.prop_text(node, "name").unwrap_or_default();
                let element = svg().path(format!("icons/{name}.svg"));
                // `svg()` has no intrinsic size — default to 16px like a Medium icon.
                let element = match self.prop(node, "size").and_then(length) {
                    Some(size) => element.w(size).h(size),
                    None => element.w_4().h_4(),
                };
                element
                    .when_some(color, |e, c| e.text_color(c))
                    .into_any_element()
            }
            "Image" => {
                let src = self.prop_text(node, "src").unwrap_or_default();
                // `img()` keeps a multicolor image's colors — `svg()` is a
                // single-color mask tinted with `text_color` for theme icons.
                // A `Path` source reads the file directly; a URL or a bare name
                // falls through as `ImageSource` (uri / embedded asset key).
                let path = self.resolve_path(&src);
                let element = if path.is_file() {
                    img(path)
                } else {
                    img(src.as_str())
                };
                let fit = match self.prop_text(node, "object-fit").as_deref() {
                    Some("Cover") => ObjectFit::Cover,
                    Some("Fill") => ObjectFit::Fill,
                    Some("ScaleDown") => ObjectFit::ScaleDown,
                    _ => ObjectFit::Contain,
                };
                let element = element.object_fit(fit);
                match self.prop(node, "size").and_then(length) {
                    Some(size) => element.w(size).h(size),
                    None => element,
                }
                .into_any_element()
            }
            "Spinner" => {
                let color = self.prop(node, "color").and_then(|v| color(v, cx));
                Spinner::new()
                    .when_some(self.prop_text(node, "icon"), |s, name| s.icon(icon(&name)))
                    .when_some(color, |s, c| s.color(c))
                    .into_any_element()
            }
            "Alert" => {
                let color = self.prop(node, "color").and_then(|v| color(v, cx));
                let message = self.prop_text(node, "message").unwrap_or_default();
                let mut alert = Alert::new(id, message).with_variant(alert_variant(
                    &self.prop_text(node, "variant").unwrap_or_default(),
                ));
                if let Some(title) = self.prop_text(node, "title") {
                    alert = alert.title(title);
                }
                alert
                    .when(self.prop_bool(node, "banner"), |a| a.banner())
                    .visible(!matches!(
                        self.prop(node, "visible"),
                        Some(Value::Bool(false))
                    ))
                    .when_some(self.prop_text(node, "icon"), |a, name| a.icon(icon(&name)))
                    .when_some(color, |a, c| a.icon(icon("info").text_color(c)))
                    .into_any_element()
            }
            "Kbd" => match Keystroke::parse(&self.prop_text(node, "keys").unwrap_or_default()) {
                Ok(stroke) => Kbd::new(stroke)
                    .appearance(!matches!(
                        self.prop(node, "appearance"),
                        Some(Value::Bool(false))
                    ))
                    .when(self.prop_bool(node, "outline"), |k| k.outline())
                    .into_any_element(),
                Err(_) => placeholder("invalid keystroke".to_owned(), cx),
            },
            "Skeleton" => Skeleton::new()
                .when(self.prop_bool(node, "secondary"), |s| s.secondary())
                .into_any_element(),
            "Breadcrumb" => {
                let items: Vec<BreadcrumbItem> = node
                    .children
                    .iter()
                    .map(|child| {
                        BreadcrumbItem::new(
                            self.prop_text(child, "label").unwrap_or_default(),
                        )
                        .disabled(self.prop_bool(child, "disabled"))
                    })
                    .collect();
                Breadcrumb::new().children(items).into_any_element()
            }
            "Pagination" => {
                let changed = node.clone();
                let path = self.two_way(node, "page");
                Pagination::new(id)
                    .current_page(
                        self.prop(node, "page")
                            .and_then(|v| self.number(v))
                            .unwrap_or_default() as usize,
                    )
                    .total_pages(
                        self.prop(node, "total")
                            .and_then(|v| self.number(v))
                            .unwrap_or_default() as usize,
                    )
                    .when(self.prop_bool(node, "compact"), |p| p.compact())
                    .when_some(
                        self.prop(node, "visible").and_then(|v| self.number(v)),
                        |p, n| p.visible_pages(n as usize),
                    )
                    .on_click(move |page, _, cx| {
                        try_update(&weak, cx, "event", |this, cx| {
                            if let Some(path) = &path {
                                this.write(path, Json::from(*page), cx);
                            }
                            this.fire(&changed, "change", cx);
                        })
                        .ok();
                    })
                    .into_any_element()
            }
            "Rating" => {
                let changed = node.clone();
                let path = self.two_way(node, "value");
                Rating::new(id)
                    .with_size(size(&self.prop_text(node, "size").unwrap_or_default()))
                    .disabled(self.prop_bool(node, "disabled"))
                    .value(
                        self.prop(node, "value")
                            .and_then(|v| self.number(v))
                            .unwrap_or_default() as usize,
                    )
                    .max(
                        self.prop(node, "max")
                            .and_then(|v| self.number(v))
                            .unwrap_or(5.0) as usize,
                    )
                    .on_click(move |value, _, cx| {
                        try_update(&weak, cx, "event", |this, cx| {
                            if let Some(path) = &path {
                                this.write(path, Json::from(*value), cx);
                            }
                            this.fire(&changed, "change", cx);
                        })
                        .ok();
                    })
                    .into_any_element()
            }
            "Toolbar" => {
                let kids = children(self, window, cx);
                Toolbar::new(id)
                    .disabled(self.prop_bool(node, "disabled"))
                    .contents(kids)
                    .into_any_element()
            }
            "Progress" => {
                let color = self.prop(node, "color").and_then(|v| color(v, cx));
                let mut bar = Progress::new(id)
                    .loading(self.prop_bool(node, "loading"))
                    .value(
                        self.prop(node, "value")
                            .and_then(|v| self.number(v))
                            .unwrap_or_default() as f32,
                    );
                if let Some(c) = color {
                    bar = bar.color(c);
                }
                bar.into_any_element()
            }
            "ProgressCircle" => {
                let color = self.prop(node, "color").and_then(|v| color(v, cx));
                let mut circle = ProgressCircle::new(id)
                    .loading(self.prop_bool(node, "loading"))
                    .value(
                        self.prop(node, "value")
                            .and_then(|v| self.number(v))
                            .unwrap_or_default() as f32,
                    );
                if let Some(c) = color {
                    circle = circle.color(c);
                }
                circle.into_any_element()
            }
            "DescriptionList" => {
                let items: Vec<DescriptionItem> = node
                    .children
                    .iter()
                    .map(|child| {
                        let mut item = DescriptionItem::new(
                            self.prop_text(child, "label").unwrap_or_default(),
                        )
                        .value(self.prop_text(child, "value").unwrap_or_default());
                        if let Some(span) =
                            self.prop(child, "span").and_then(|v| self.number(v))
                        {
                            item = item.span(span as usize);
                        }
                        item
                    })
                    .collect();
                let horizontal = self
                    .prop_text(node, "layout")
                    .is_some_and(|l| l.eq_ignore_ascii_case("horizontal"));
                let list = if horizontal {
                    DescriptionList::horizontal()
                } else {
                    DescriptionList::vertical()
                };
                list.children(items)
                    .bordered(self.prop_bool(node, "bordered"))
                    .when_some(
                        self.prop(node, "columns").and_then(|v| self.number(v)),
                        |l, n| l.columns(n as usize),
                    )
                    .when_some(
                        self.prop(node, "label-width").and_then(length),
                        |l, w| l.label_width(w),
                    )
                    .into_any_element()
            }
            "StatusBar" => {
                let mut kids = children(self, window, cx).into_iter();
                let mut bar = StatusBar::new();
                if let Some(left) = kids.next() {
                    bar = bar.left(left);
                }
                if let Some(right) = kids.next() {
                    bar = bar.right(right);
                }
                bar.into_any_element()
            }
            "Avatar" => self.avatar(node).into_any_element(),
            "AvatarGroup" => {
                let avatars: Vec<Avatar> =
                    node.children.iter().map(|child| self.avatar(child)).collect();
                AvatarGroup::new()
                    .children(avatars)
                    .when_some(
                        self.prop(node, "limit").and_then(|v| self.number(v)),
                        |g, n| g.limit(n as usize),
                    )
                    .when(self.prop_bool(node, "ellipsis"), |g| g.ellipsis())
                    .into_any_element()
            }
            "Stepper" => {
                let items: Vec<StepperItem> = node
                    .children
                    .iter()
                    .map(|child| {
                        let mut item = StepperItem::new()
                            .disabled(self.prop_bool(child, "disabled"));
                        if let Some(name) = self.prop_text(child, "icon") {
                            item = item.icon(icon(&name));
                        }
                        if let Some(label) = self.prop_text(child, "label") {
                            item = item.child(label);
                        }
                        item
                    })
                    .collect();
                let changed = node.clone();
                let path = self.two_way(node, "selected");
                Stepper::new(id)
                    .items(items)
                    .selected_index(
                        self.prop(node, "selected")
                            .and_then(|v| self.number(v))
                            .unwrap_or_default() as usize,
                    )
                    .text_center(self.prop_bool(node, "text-center"))
                    .disabled(self.prop_bool(node, "disabled"))
                    .when(
                        self.prop_text(node, "layout")
                            .is_some_and(|l| l.eq_ignore_ascii_case("vertical")),
                        |s| s.vertical(),
                    )
                    .on_click(move |index, _, cx| {
                        try_update(&weak, cx, "event", |this, cx| {
                            if let Some(path) = &path {
                                this.write(path, Json::from(*index), cx);
                            }
                            this.fire(&changed, "change", cx);
                        })
                        .ok();
                    })
                    .into_any_element()
            }
            "Slider" => {
                let state = match self.sliders.get(key) {
                    Some(state) => state.clone(),
                    None => {
                        let value = self
                            .prop(node, "value")
                            .and_then(|v| self.number(v))
                            .unwrap_or_default();
                        let mut state = SliderState::new()
                            .min(self.prop(node, "min").and_then(|v| self.number(v)).unwrap_or_default() as f32)
                            .max(self.prop(node, "max").and_then(|v| self.number(v)).unwrap_or(100.0) as f32);
                        if let Some(step) = self.prop(node, "step").and_then(|v| self.number(v)) {
                            state = state.step(step as f32);
                        }
                        state = state.default_value(SliderValue::Single(value as f32));
                        let state = cx.new(|_| state);
                        let path = self.two_way(node, "value");
                        let changed = node.clone();
                        let subscription = cx.subscribe(&state, move |this, _state, event, cx| {
                            let SliderEvent::Change(value) = event else {
                                return;
                            };
                            if let SliderValue::Single(value) = value {
                                if let Some(path) = &path {
                                    this.write(path, Json::from(*value), cx);
                                }
                                this.fire(&changed, "change", cx);
                            }
                        });
                        self.subscriptions.push(subscription);
                        self.sliders.insert(key.to_owned(), state.clone());
                        state
                    }
                };
                let slider = Slider::new(&state)
                    .disabled(self.prop_bool(node, "disabled"))
                    .when(self.prop_bool(node, "vertical"), |s| s.vertical())
                    .when(self.prop_bool(node, "reverse"), |s| s.reverse());
                slider.into_any_element()
            }
            "Accordion" => {
                let changed = node.clone();
                // The component Accordion is controlled: the trigger only reports the
                // new open set via `on_toggle_click`. Persist it in view state so each
                // re-render rebuilds items with the right `open` flags.
                let open_path = Path(vec![
                    PathSegment::Field("accordion".into()),
                    PathSegment::Field(key.to_owned()),
                    PathSegment::Field("open".into()),
                ]);
                let open_set: HashSet<usize> = match get_path(&self.state, &open_path) {
                    Some(Json::Array(list)) => list
                        .iter()
                        .filter_map(|v| v.as_u64())
                        .map(|v| v as usize)
                        .collect(),
                    _ => {
                        let seeded: Vec<usize> = node
                            .children
                            .iter()
                            .enumerate()
                            .filter(|(_, c)| self.prop_bool(c, "open"))
                            .map(|(i, _)| i)
                            .collect();
                        set_path(&mut self.state, &open_path, Json::from(seeded.clone()));
                        seeded.into_iter().collect()
                    }
                };
                let open_path_c = open_path.clone();
                let mut accordion = Accordion::new(id)
                    .multiple(self.prop_bool(node, "multiple"))
                    .bordered(self.prop_bool(node, "bordered"))
                    .disabled(self.prop_bool(node, "disabled"))
                    .on_toggle_click(move |indices: &[usize], _, cx| {
                        let open_path = open_path_c.clone();
                        let changed = changed.clone();
                        try_update(&weak, cx, "event", |this, cx| {
                            this.update_state(&open_path, Json::from(indices.to_vec()), cx);
                            this.fire(&changed, "change", cx);
                        })
                        .ok();
                    });
                for (ci, child) in node.children.iter().enumerate() {
                    let title = self.prop_text(child, "title").unwrap_or_default();
                    let kids: Vec<AnyElement> = child
                        .children
                        .iter()
                        .enumerate()
                        .map(|(i, kid)| {
                            self.render_node(kid, &format!("{key}.{ci}.{i}"), window, cx)
                        })
                        .collect();
                    let mut item = gpui_kit::component::accordion::AccordionItem::new()
                        .title(title)
                        .open(open_set.contains(&ci))
                        .disabled(self.prop_bool(child, "disabled"))
                        .child(v_flex().children(kids));
                    if let Some(name) = self.prop_text(child, "icon") {
                        item = item.icon(icon(&name));
                    }
                    accordion = accordion.item(move |_| item);
                }
                accordion.into_any_element()
            }
            "Collapsible" => {
                let kids = children(self, window, cx);
                Collapsible::new()
                    .open(self.prop_bool(node, "open"))
                    .motion_id(id)
                    .content(v_flex().children(kids))
                    .into_any_element()
            }
            "Form" => {
                let fields: Vec<Field> = node
                    .children
                    .iter()
                    .enumerate()
                    .map(|(i, child)| {
                        self.form_field(child, &format!("{key}/{i}"), window, cx)
                    })
                    .collect();
                let form = if self
                    .prop_text(node, "layout")
                    .is_some_and(|l| l.eq_ignore_ascii_case("horizontal"))
                {
                    Form::horizontal()
                } else {
                    Form::vertical()
                };
                form.children(fields)
                    .when_some(
                        self.prop(node, "columns").and_then(|v| self.number(v)),
                        |f, n| f.columns(n as usize),
                    )
                    .when_some(
                        self.prop(node, "label-width").and_then(|v| self.number(v)),
                        |f, n| f.label_width(px(n as f32)),
                    )
                    .when(
                        self.prop_text(node, "label-layout")
                            .is_some_and(|l| l.eq_ignore_ascii_case("vertical")),
                        |f| f.label_layout(Axis::Vertical),
                    )
                    .into_any_element()
            }
            "FormField" => self.form_field(node, key, window, cx).into_any_element(),
            "ColorPicker" => {
                let state = match self.color_pickers.get(key) {
                    Some(state) => state.clone(),
                    None => {
                        let state = cx.new(|cx| ColorPickerState::new(window, cx));
                        let path = self.two_way(node, "value");
                        let changed = node.clone();
                        let subscription = cx.subscribe(&state, move |this, _state, event, cx| {
                            let ColorPickerEvent::Change(value) = event;
                            if let Some(path) = &path {
                                let hex = value.map(|c| c.to_hex()).unwrap_or_default();
                                this.write(path, Json::String(hex), cx);
                            }
                            this.fire(&changed, "change", cx);
                        });
                        self.subscriptions.push(subscription);
                        self.color_pickers.insert(key.to_owned(), state.clone());
                        state
                    }
                };
                ColorPicker::new(&state).into_any_element()
            }
            "OtpInput" => {
                let state = match self.otps.get(key) {
                    Some(state) => state.clone(),
                    None => {
                        let length = self
                            .prop(node, "length")
                            .and_then(|v| self.number(v))
                            .unwrap_or(6.0) as usize;
                        let state = cx.new(|cx| OtpState::new(length, window, cx));
                        let path = self.two_way(node, "value");
                        let changed = node.clone();
                        let subscription = cx.subscribe(&state, move |this, state, event, cx| {
                            match event {
                                OtpEvent::Change => {
                                    if let Some(path) = &path {
                                        let value = state.read(cx).value().to_string();
                                        this.write(path, Json::String(value), cx);
                                    }
                                    this.fire(&changed, "change", cx);
                                }
                                OtpEvent::Complete => {
                                    this.fire(&changed, "complete", cx);
                                }
                                _ => {}
                            }
                        });
                        self.subscriptions.push(subscription);
                        self.otps.insert(key.to_owned(), state.clone());
                        state
                    }
                };
                OtpInput::new(&state).into_any_element()
            }
            "DatePicker" => {
                let state = match self.date_pickers.get(key) {
                    Some(state) => state.clone(),
                    None => {
                        let state = cx.new(|cx| DatePickerState::new(window, cx));
                        let path = self.two_way(node, "value");
                        let changed = node.clone();
                        let subscription = cx.subscribe(&state, move |this, _state, event, cx| {
                            let gpui_kit::component::date_picker::DatePickerEvent::Change(date) =
                                event;
                            if let Some(path) = &path {
                                this.write(path, Json::String(format!("{date:?}")), cx);
                            }
                            this.fire(&changed, "change", cx);
                        });
                        self.subscriptions.push(subscription);
                        self.date_pickers.insert(key.to_owned(), state.clone());
                        state
                    }
                };
                DatePicker::new(&state)
                    .disabled(self.prop_bool(node, "disabled"))
                    .into_any_element()
            }
            "Calendar" => {
                let state = match self.calendars.get(key) {
                    Some(state) => state.clone(),
                    None => {
                        let state = cx.new(|cx| CalendarState::new(window, cx));
                        self.calendars.insert(key.to_owned(), state.clone());
                        state
                    }
                };
                Calendar::new(&state).into_any_element()
            }
            "TimeField" => {
                let state = match self.time_fields.get(key) {
                    Some(state) => state.clone(),
                    None => {
                        let state = cx.new(|cx| TimeFieldState::new(window, cx));
                        let changed = node.clone();
                        let subscription =
                            cx.subscribe(&state, move |this, _state, _event, cx| {
                                this.fire(&changed, "change", cx);
                            });
                        self.subscriptions.push(subscription);
                        self.time_fields.insert(key.to_owned(), state.clone());
                        state
                    }
                };
                TimeField::new(&state)
                    .invalid(self.prop_bool(node, "invalid"))
                    .into_any_element()
            }
            "TitleBar" => TitleBar::new()
                .children(children(self, window, cx))
                .into_any_element(),
            "Message" => {
                let mut message = Message::new();
                if self
                    .prop_text(node, "alignment")
                    .is_some_and(|a| a.eq_ignore_ascii_case("end"))
                {
                    message =
                        message.alignment(gpui_kit::component::message::MessageAlignment::End);
                }
                if let Some(name) = self.prop_text(node, "avatar") {
                    message = message.avatar(icon(&name));
                }
                for (i, child) in node.children.iter().enumerate() {
                    let kids = self.slot_children(child, &format!("{key}/{i}"), window, cx);
                    match child.element.name.as_str() {
                        "MessageAvatar" => {
                            message = message
                                .avatar_slot(MessageAvatar::new().children(kids));
                        }
                        "MessageHeader" => {
                            message = message.header(MessageHeader::new().children(kids));
                        }
                        "MessageContent" => {
                            message = message.content(MessageContent::new().children(kids));
                        }
                        "MessageFooter" => {
                            message = message.footer(MessageFooter::new().children(kids));
                        }
                        _ => {}
                    }
                }
                message.into_any_element()
            }
            "Table" => {
                let mut table = Table::new();
                for (i, child) in node.children.iter().enumerate() {
                    let key = format!("{key}/{i}");
                    match child.element.name.as_str() {
                        "TableHeader" => {
                            let heads: Vec<TableHead> = child
                                .children
                                .iter()
                                .map(|h| self.table_head(h))
                                .collect();
                            table = table.child(TableHeader::new().children(heads));
                        }
                        "TableBody" | "TableFooter" => {
                            let rows: Vec<gpui_kit::component::table::TableRow> = child
                                .children
                                .iter()
                                .enumerate()
                                .map(|(_j, r)| {
                                    let cells: Vec<TableCell> = r
                                        .children
                                        .iter()
                                        .map(|c| self.table_cell(c))
                                        .collect();
                                    gpui_kit::component::table::TableRow::new()
                                        .children(cells)
                                })
                                .collect();
                            table = if child.element.name.as_str() == "TableFooter" {
                                table.child(TableFooter::new().children(rows))
                            } else {
                                table.child(TableBody::new().children(rows))
                            };
                            let _ = key;
                        }
                        "TableCaption" => {
                            let kids =
                                self.slot_children(child, &key, window, cx);
                            table = table.child(TableCaption::new().children(kids));
                        }
                        _ => {}
                    }
                }
                table.into_any_element()
            }
            "TableHead" => self.table_head(node).into_any_element(),
            "TableCell" => self.table_cell(node).into_any_element(),
            "Resizable" => {
                let panels: Vec<ResizablePanel> = node
                    .children
                    .iter()
                    .enumerate()
                    .map(|(i, child)| {
                        let kids =
                            self.slot_children(child, &format!("{key}/{i}"), window, cx);
                        let mut panel = resizable_panel().children(kids);
                        if let Some(n) =
                            self.prop(child, "size").and_then(|v| self.number(v))
                        {
                            panel = panel.size(px(n as f32));
                        }
                        match (
                            self.prop(child, "min-size").and_then(|v| self.number(v)),
                            self.prop(child, "max-size").and_then(|v| self.number(v)),
                        ) {
                            (Some(min), Some(max)) => {
                                panel = panel.size_range(px(min as f32)..px(max as f32));
                            }
                            _ => {}
                        }
                        panel
                            .when(self.prop_bool(child, "fixed"), |p| p.fixed())
                            .visible(!matches!(
                                self.prop(child, "visible"),
                                Some(Value::Bool(false))
                            ))
                    })
                    .collect();
                let group = if self
                    .prop_text(node, "layout")
                    .is_some_and(|l| l.eq_ignore_ascii_case("vertical"))
                {
                    v_resizable(id)
                } else {
                    h_resizable(id)
                };
                group.children(panels).into_any_element()
            }
            "Sidebar" => {
                let mut sidebar = Sidebar::<SidebarMenu>::new(id)
                    .collapsed(self.prop_bool(node, "collapsed"));
                if self
                    .prop_text(node, "side")
                    .is_some_and(|s| s.eq_ignore_ascii_case("right"))
                {
                    sidebar = sidebar.side(gpui_kit::base::Side::Right);
                }
                match self.prop_text(node, "collapsible").as_deref() {
                    Some("none") => {
                        sidebar = sidebar.collapsible(SidebarCollapsible::None);
                    }
                    Some("offcanvas") => {
                        sidebar = sidebar.collapsible(SidebarCollapsible::Offcanvas);
                    }
                    _ => {}
                }
                for (i, child) in node.children.iter().enumerate() {
                    let key = format!("{key}/{i}");
                    match child.element.name.as_str() {
                        "SidebarHeader" => {
                            let kids = self.slot_children(child, &key, window, cx);
                            sidebar = sidebar.header(v_flex().children(kids));
                        }
                        "SidebarFooter" => {
                            let kids = self.slot_children(child, &key, window, cx);
                            sidebar = sidebar.footer(v_flex().children(kids));
                        }
                        "SidebarMenu" => {
                            let items: Vec<SidebarMenuItem> = child
                                .children
                                .iter()
                                .enumerate()
                                .map(|(j, item)| {
                                    self.sidebar_item(item, &format!("{key}/{j}"), window, cx)
                                })
                                .collect();
                            sidebar = sidebar.child(SidebarMenu::new().children(items));
                        }
                        "SidebarMenuItem" => {
                            let item = self.sidebar_item(child, &key, window, cx);
                            sidebar = sidebar.child(SidebarMenu::new().child(item));
                        }
                        _ => {}
                    }
                }
                sidebar.into_any_element()
            }
            "List" => {
                let state = match self.lists.get(key) {
                    Some(state) => state.clone(),
                    None => {
                        // The weak view must come from the OUTER context — `cx` inside
                        // `cx.new` is the new ListState's context, not the ForgeView's.
                        let weak = cx.entity().downgrade();
                        let state: Entity<ListState<ForgeListDelegate>> = cx.new(|cx| {
                            ListState::new(
                                ForgeListDelegate {
                                    view: weak,
                                    key: key.to_owned(),
                                    children: node.children.clone(),
                                    selected: Rc::default(),
                                },
                                window,
                                cx,
                            )
                        });
                        let path = self.two_way(node, "selected");
                        let changed = node.clone();
                        let subscription =
                            cx.subscribe(&state, move |this, _state, event, cx| match event {
                                ListEvent::Select(ix) => {
                                    if let Some(path) = &path {
                                        this.write(path, Json::from(ix.row), cx);
                                    }
                                    this.fire(&changed, "select", cx);
                                }
                                ListEvent::Confirm(ix) => {
                                    let _ = ix;
                                    this.fire(&changed, "confirm", cx);
                                }
                                _ => {}
                            });
                        self.subscriptions.push(subscription);
                        self.lists.insert(key.to_owned(), state.clone());
                        state
                    }
                };
                List::new(&state).into_any_element()
            }
            "Tree" => {
                // Nodes live in `nodes` by their `TreeItem` id so the paint-time
                // `render_item` can rebuild rows — it runs several times per row.
                let mut nodes: HashMap<String, Node> = HashMap::new();
                let items: Vec<TreeItem> = node
                    .children
                    .iter()
                    .enumerate()
                    .map(|(i, child)| {
                        self.tree_data(child, &format!("{key}/{i}"), &mut nodes)
                    })
                    .collect();
                let state = match self.trees.get(key) {
                    Some(state) => {
                        state.update(cx, |state, cx| {
                            *state = std::mem::replace(state, TreeState::new(cx))
                                .items(items.clone());
                        });
                        state.clone()
                    }
                    None => {
                        let state = cx.new(|cx| TreeState::new(cx).items(items.clone()));
                        self.trees.insert(key.to_owned(), state.clone());
                        state
                    }
                };
                let weak = cx.entity().downgrade();
                let nodes = Rc::new(nodes);
                let key = key.to_owned();
                Tree::new(&state, move |ix, entry, selected, window, cx| {
                    let mut item = ListItem::new(ix);
                    let id = entry.item().id.to_string();
                    if let Some(node) = nodes.get(&id).cloned() {
                        let key = format!("{key}/{id}");
                        if let Some(row) = try_update(&weak, cx, "tree row", |view, cx| {
                                let mut row = div().flex().flex_row().gap_2();
                                if let Some(name) = view.prop_text(&node, "icon") {
                                    row = row.child(icon(&name));
                                }
                                if let Some(label) = view.prop_text(&node, "label") {
                                    row = row.child(label);
                                }
                                for (i, child) in node.children.iter().enumerate() {
                                    if child.element.name.as_str() != "TreeItem" {
                                        row = row.child(view.render_node(
                                            child,
                                            &format!("{key}/{i}"),
                                            window,
                                            cx,
                                        ));
                                    }
                                }
                                row
                            })
                            .ok()
                        {
                            item = item.child(row);
                        }
                    }
                    item.selected(selected)
                })
                .into_any_element()
            }
            other => placeholder(format!("`{other}` is not in the gpui-kit catalog"), cx),
        };
        wrap(
            element,
            &node.style,
            &format!("forge-{key}"),
            self.prop_text(node, "tooltip"),
            cx,
        )
    }
}

impl Render for ForgeView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let (background, foreground) = (theme.background, theme.foreground);
        let content = self
            .doc
            .clone()
            .map(|doc| self.render_node(&doc.root, "root", window, cx))
            .or_else(|| self.empty_text.clone().map(|text| empty(text, cx)));
        for message in self.take_errors() {
            window.push_notification((NotificationType::Error, message), cx);
        }
        div()
            .id("forge-canvas")
            .size_full()
            .overflow_y_scroll()
            .bg(background)
            .text_color(foreground)
            .children(content)
    }
}

/// Applies `style` to a container. `overflow: scroll` needs an element id, so a scrolling
/// container is stateful.
fn container(
    element: Div,
    entries: &Entries,
    id: SharedString,
    kids: Vec<AnyElement>,
    cx: &App,
) -> AnyElement {
    if scrollable(entries) {
        style(element.id(id), entries, cx)
            .overflow_y_scroll()
            .children(kids)
            .into_any_element()
    } else {
        style(element, entries, cx)
            .children(kids)
            .into_any_element()
    }
}

/// Applies `style` to a leaf element: it goes on a wrapper `div` (scrollable when
/// `overflow: scroll`, which likewise needs an element id).
fn wrap(
    element: AnyElement,
    entries: &Entries,
    id: &str,
    tooltip: Option<String>,
    cx: &App,
) -> AnyElement {
    if entries.is_empty() && tooltip.is_none() {
        element
    } else if scrollable(entries) {
        let mut wrapper = style(
            div().id(SharedString::from(format!("{id}-scroll"))),
            entries,
            cx,
        )
        .overflow_y_scroll();
        if let Some(tip) = tooltip {
            wrapper = wrapper.tooltip(move |_, cx| {
                cx.new(|_| Tooltip::new(tip.clone())).into()
            });
        }
        wrapper.child(element).into_any_element()
    } else if let Some(tip) = tooltip {
        // Tooltips need interactivity — the wrapper must be stateful.
        style(
            div()
                .id(SharedString::from(format!("{id}-tip")))
                .tooltip(move |_, cx| {
                    cx.new(|_| Tooltip::new(tip.clone())).into()
                }),
            entries,
            cx,
        )
        .child(element)
        .into_any_element()
    } else {
        style(div(), entries, cx)
            .child(element)
            .into_any_element()
    }
}

/// Whether a `style:` map asks for a scrolling element (`overflow: scroll`).
fn scrollable(entries: &Entries) -> bool {
    entries.iter().any(|(key, value)| {
        key == "overflow" && matches!(value, Value::Enum(word) | Value::String(word) if word == "scroll")
    })
}

/// Shown by a `forge.Preview` with no document.
fn empty(text: String, cx: &App) -> AnyElement {
    let theme = cx.theme();
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text)
        .into_any_element()
}

fn placeholder(message: String, cx: &App) -> AnyElement {
    let theme = cx.theme();
    div()
        .px_2()
        .py_1()
        .border_1()
        .border_dashed()
        .border_color(theme.border)
        .rounded_md()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(message)
        .into_any_element()
}

fn icon(name: &str) -> Icon {
    Icon::empty().path(format!("icons/{name}.svg"))
}

fn button_variant(name: &str) -> ButtonVariant {
    match name {
        "Primary" => ButtonVariant::Primary,
        "Secondary" => ButtonVariant::Secondary,
        "Danger" => ButtonVariant::Danger,
        "Info" => ButtonVariant::Info,
        "Success" => ButtonVariant::Success,
        "Warning" => ButtonVariant::Warning,
        "Ghost" => ButtonVariant::Ghost,
        "Link" => ButtonVariant::Link,
        "Text" => ButtonVariant::Text,
        _ => ButtonVariant::Default,
    }
}

fn alert_variant(name: &str) -> AlertVariant {
    match name.to_lowercase().as_str() {
        "success" => AlertVariant::Success,
        "warning" => AlertVariant::Warning,
        "error" | "danger" => AlertVariant::Error,
        _ => AlertVariant::Info,
    }
}

fn tag_variant(name: &str) -> TagVariant {
    match name {
        "Primary" => TagVariant::Primary,
        "Danger" => TagVariant::Danger,
        "Success" => TagVariant::Success,
        "Warning" => TagVariant::Warning,
        "Info" => TagVariant::Info,
        _ => TagVariant::Secondary,
    }
}

fn group_box_variant(name: &str) -> GroupBoxVariant {
    match name {
        "Fill" => GroupBoxVariant::Fill,
        "Outline" => GroupBoxVariant::Outline,
        _ => GroupBoxVariant::Normal,
    }
}

fn tab_variant(name: &str) -> TabVariant {
    match name {
        "Outline" => TabVariant::Outline,
        "Pill" => TabVariant::Pill,
        "Segmented" => TabVariant::Segmented,
        "Underline" => TabVariant::Underline,
        "Folder" => TabVariant::Folder,
        _ => TabVariant::Tab,
    }
}

fn size(name: &str) -> Size {
    match name {
        "XSmall" => Size::XSmall,
        "Small" => Size::Small,
        "Large" => Size::Large,
        _ => Size::Medium,
    }
}

fn length(value: &Value) -> Option<DefiniteLength> {
    match value {
        Value::Number { value, unit } => Some(match unit {
            None | Some(Unit::Px) => px(*value as f32).into(),
            Some(Unit::Rem) => rems(*value as f32).into(),
            Some(Unit::Percent) => relative(*value as f32 / 100.0),
            Some(Unit::Fr) => return None,
        }),
        _ => None,
    }
}

fn color(value: &Value, cx: &App) -> Option<Hsla> {
    match value {
        Value::Color(c) => Some(
            Rgba {
                r: f32::from(c.r) / 255.0,
                g: f32::from(c.g) / 255.0,
                b: f32::from(c.b) / 255.0,
                a: f32::from(c.a) / 255.0,
            }
            .into(),
        ),
        Value::Token(path) => {
            let theme = cx.theme();
            let name = match path.0.last()? {
                PathSegment::Field(name) => name.as_str(),
                PathSegment::Index(_) => return None,
            };
            Some(match name {
                "primary" => theme.primary,
                "secondary" => theme.secondary,
                "background" => theme.background,
                "foreground" => theme.foreground,
                "muted" => theme.muted,
                "muted_foreground" | "muted-foreground" => theme.muted_foreground,
                "accent" => theme.accent,
                "border" => theme.border,
                "danger" => theme.danger,
                "success" => theme.success,
                "warning" => theme.warning,
                "info" => theme.info,
                _ => return None,
            })
        }
        _ => None,
    }
}

/// Applies a node's `style:` map (`docs/spec/style-layout.md`).
fn style<E: Styled>(element: E, entries: &Entries, cx: &App) -> E {
    entries.iter().fold(element, |e, (key, value)| {
        let word = match value {
            Value::Enum(s) | Value::String(s) => s.as_str(),
            _ => "",
        };
        let flag = matches!(value, Value::Bool(true));
        match (key.as_str(), length(value), color(value, cx)) {
            ("gap", Some(l), _) => e.gap(l),
            ("gap-x", Some(l), _) => e.gap_x(l),
            ("gap-y", Some(l), _) => e.gap_y(l),
            ("width", Some(l), _) => e.w(l),
            ("height", Some(l), _) => e.h(l),
            ("min-width", Some(l), _) => e.min_w(l),
            ("max-width", Some(l), _) => e.max_w(l),
            ("min-height", Some(l), _) => e.min_h(l),
            ("max-height", Some(l), _) => e.max_h(l),
            ("basis", Some(l), _) => e.flex_basis(l),
            ("line-height", Some(l), _) => e.line_height(l),
            ("width", _, _) if word == "full" => e.w_full(),
            ("height", _, _) if word == "full" => e.h_full(),
            ("padding", _, _) => match edges(value) {
                Some([t, r, b, l]) => e.pt(t).pr(r).pb(b).pl(l),
                None => e,
            },
            ("margin", _, _) => match edges(value) {
                Some([t, r, b, l]) => e.mt(t).mr(r).mb(b).ml(l),
                None => e,
            },
            ("background", _, Some(c)) => e.bg(c),
            ("color", _, Some(c)) => e.text_color(c),
            ("border-color", _, Some(c)) => e.border_color(c),
            ("border", Some(_), _) => e.border_1(),
            ("border-style", _, _) if word == "dashed" => e.border_dashed(),
            ("radius", Some(_), _) | ("radius", _, _) => match (word, value) {
                ("sm", _) => e.rounded_sm(),
                ("md", _) => e.rounded_md(),
                ("lg", _) => e.rounded_lg(),
                ("full", _) => e.rounded_full(),
                (
                    _,
                    Value::Number {
                        value,
                        unit: None | Some(Unit::Px),
                    },
                ) => e.rounded(px(*value as f32)),
                (
                    _,
                    Value::Number {
                        value,
                        unit: Some(Unit::Rem),
                    },
                ) => e.rounded(rems(*value as f32)),
                _ => e,
            },
            ("direction", _, _) => match word {
                "row" => e.flex_row(),
                "column" => e.flex_col(),
                "row-reverse" => e.flex_row_reverse(),
                "column-reverse" => e.flex_col_reverse(),
                _ => e,
            },
            ("align", _, _) => match word {
                "start" => e.items_start(),
                "center" => e.items_center(),
                "end" => e.items_end(),
                "stretch" => e.items_stretch(),
                "baseline" => e.items_baseline(),
                _ => e,
            },
            ("align-self", _, _) => match word {
                "start" => e.self_start(),
                "center" => e.self_center(),
                "end" => e.self_end(),
                "stretch" => e.self_stretch(),
                "baseline" => e.self_baseline(),
                _ => e,
            },
            ("justify", _, _) => match word {
                "start" => e.justify_start(),
                "center" => e.justify_center(),
                "end" => e.justify_end(),
                "between" => e.justify_between(),
                "around" => e.justify_around(),
                "evenly" => e.justify_evenly(),
                _ => e,
            },
            ("wrap", _, _) => {
                if flag {
                    e.flex_wrap()
                } else {
                    e.flex_nowrap()
                }
            }
            ("grow", _, _) => match value {
                Value::Number { value, .. } => e.flex_grow(*value as f32),
                Value::Bool(b) => e.flex_grow(if *b { 1.0 } else { 0.0 }),
                _ => e,
            },
            ("shrink", _, _) => match value {
                Value::Number { value, .. } => e.flex_shrink(*value as f32),
                Value::Bool(b) => e.flex_shrink(if *b { 1.0 } else { 0.0 }),
                _ => e,
            },
            // `scroll` needs an element id; `container`/`wrap` apply it instead.
            ("overflow", _, _) if !matches!(word, "visible" | "scroll") => e.overflow_hidden(),
            ("opacity", _, _) => match value {
                Value::Number { value, .. } => e.opacity(*value as f32),
                _ => e,
            },
            ("shadow", _, _) => match word {
                "sm" => e.shadow_sm(),
                "md" => e.shadow_md(),
                "lg" => e.shadow_lg(),
                "xl" => e.shadow_xl(),
                _ => e,
            },
            ("size", _, _) => match word {
                "xs" => e.text_xs(),
                "sm" => e.text_sm(),
                "base" | "md" => e.text_base(),
                "lg" => e.text_lg(),
                "xl" => e.text_xl(),
                "2xl" => e.text_2xl(),
                "3xl" => e.text_3xl(),
                _ => match value {
                    Value::Number { value, .. } => e.text_size(px(*value as f32)),
                    _ => e,
                },
            },
            ("weight", _, _) => e.font_weight(match word {
                "thin" => FontWeight::THIN,
                "light" => FontWeight::LIGHT,
                "medium" => FontWeight::MEDIUM,
                "semibold" => FontWeight::SEMIBOLD,
                "bold" => FontWeight::BOLD,
                "black" => FontWeight::BLACK,
                _ => FontWeight::NORMAL,
            }),
            ("text-align", _, _) => match word {
                "left" => e.text_left(),
                "center" => e.text_center(),
                "right" => e.text_right(),
                _ => e,
            },
            ("italic", _, _) if flag => e.italic(),
            ("underline", _, _) if flag => e.underline(),
            ("truncate", _, _) if flag => e.truncate(),
            _ => e,
        }
    })
}

/// `padding`/`margin` as `[top, right, bottom, left]`.
fn edges(value: &Value) -> Option<[DefiniteLength; 4]> {
    match value {
        Value::List(items) => {
            let l: Vec<DefiniteLength> = items.iter().filter_map(length).collect();
            match l.as_slice() {
                [a] => Some([*a; 4]),
                [y, x] => Some([*y, *x, *y, *x]),
                [t, r, b, l] => Some([*t, *r, *b, *l]),
                _ => None,
            }
        }
        other => length(other).map(|l| [l; 4]),
    }
}

/// Writes `value` at `path`, creating objects and arrays along the way.
pub fn set_path(state: &mut Json, path: &Path, value: Json) {
    let mut target = state;
    for segment in &path.0 {
        target = match segment {
            PathSegment::Field(name) => {
                if !target.is_object() {
                    *target = Json::Object(Default::default());
                }
                target
                    .as_object_mut()
                    .expect("just made an object")
                    .entry(name.clone())
                    .or_insert(Json::Null)
            }
            PathSegment::Index(i) => {
                if !target.is_array() {
                    *target = Json::Array(Vec::new());
                }
                let items = target.as_array_mut().expect("just made an array");
                if items.len() <= *i {
                    items.resize(*i + 1, Json::Null);
                }
                &mut items[*i]
            }
        };
    }
    *target = value;
}
