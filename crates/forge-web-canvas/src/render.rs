//! Interprets a Forge document as gpui-kit elements.
//!
//! An early, web-only slice of the gpui-kit adapter (#75). Style keys are a provisional
//! subset until the style and layout model (#5) lands.

use std::collections::HashMap;
use std::rc::Rc;

use forge_spec::catalog::Catalog;
use forge_spec::{BindingMode, Document, Entries, Node, Path, PathSegment, Segment, Unit, Value};
use gpui_kit::component::badge::Badge;
use gpui_kit::component::button::{Button, ButtonVariant, ButtonVariants};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::label::Label;
use gpui_kit::component::link::Link;
use gpui_kit::component::radio::{Radio, RadioGroup};
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::separator::Separator;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::tab::{Tab, TabBar, TabVariant};
use gpui_kit::component::tag::{Tag, TagVariant};
use gpui_kit::component::{
    ActiveTheme, Disableable, Icon, IndexPath, Sizable, Size, h_flex, v_flex,
};
use gpui_kit::{prelude::*, *};
use serde_json::Value as Json;

/// Called with `hook(args)` whenever an event handler fires.
pub type HookSink = Rc<dyn Fn(&str)>;
/// Called with the whole view state after a two-way binding writes to it.
pub type StateSink = Rc<dyn Fn(&Json)>;

type Options = Vec<SharedString>;

pub struct ForgeView {
    doc: Option<Document>,
    catalogs: Vec<Catalog>,
    state: Json,
    inputs: HashMap<String, Entity<InputState>>,
    selects: HashMap<String, Entity<SelectState<Options>>>,
    tabs: HashMap<String, usize>,
    on_hook: Option<HookSink>,
    on_state: Option<StateSink>,
    subscriptions: Vec<Subscription>,
}

impl ForgeView {
    pub fn new(catalogs: Vec<Catalog>) -> Self {
        Self {
            doc: None,
            catalogs,
            state: Json::Object(Default::default()),
            inputs: HashMap::new(),
            selects: HashMap::new(),
            tabs: HashMap::new(),
            on_hook: None,
            on_state: None,
            subscriptions: Vec::new(),
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
        self.selects.clear();
        self.subscriptions.clear();
        cx.notify();
    }

    pub fn set_sinks(&mut self, on_hook: HookSink, on_state: StateSink) {
        self.on_hook = Some(on_hook);
        self.on_state = Some(on_state);
    }

    fn fire(&self, node: &Node, event: &str) {
        let Some(binding) = node.events.iter().find(|e| e.event == event) else {
            return;
        };
        let args: Vec<String> = binding.handler.args.iter().map(|a| self.text(a)).collect();
        let call = format!("{}({})", binding.handler.name, args.join(", "));
        if let Some(sink) = &self.on_hook {
            sink(&call);
        }
    }

    fn write(&mut self, path: &Path, value: Json, cx: &mut Context<Self>) {
        set_path(&mut self.state, path, value);
        if let Some(sink) = &self.on_state {
            sink(&self.state);
        }
        cx.notify();
    }

    fn lookup(&self, path: &Path) -> Option<&Json> {
        path.0.iter().try_fold(&self.state, |json, segment| match segment {
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
            return placeholder(
                format!("{namespace}.{} is not available in the web preview yet", node.element.name),
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
                return style(div(), &node.style, cx).children(kids).into_any_element();
            }
            "VStack" => {
                let kids = children(self, window, cx);
                return style(v_flex(), &node.style, cx).children(kids).into_any_element();
            }
            "HStack" => {
                let kids = children(self, window, cx);
                return style(h_flex(), &node.style, cx).children(kids).into_any_element();
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
                    .with_variant(button_variant(&self.prop_text(node, "variant").unwrap_or_default()))
                    .with_size(size(&self.prop_text(node, "size").unwrap_or_default()))
                    .when_some(self.prop_text(node, "icon"), |b, name| b.icon(icon(&name)))
                    .when_some(self.prop_text(node, "tooltip"), |b, t| b.tooltip(t))
                    .disabled(self.prop_bool(node, "disabled"))
                    .loading(self.prop_bool(node, "loading"))
                    .when(self.prop_bool(node, "outline"), |b| b.outline())
                    .when(self.prop_bool(node, "compact"), |b| b.compact())
                    .on_click(move |_, _, cx| {
                        weak.update(cx, |this, _| this.fire(&clicked, "click")).ok();
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
                        let subscription = cx.subscribe(&state, move |this, state, event, cx| {
                            match event {
                                InputEvent::Change => {
                                    if let Some(path) = &path {
                                        let value = state.read(cx).value().to_string();
                                        this.write(path, Json::String(value), cx);
                                    }
                                    this.fire(&changed, "change");
                                }
                                InputEvent::PressEnter { .. } => this.fire(&changed, "submit"),
                                _ => {}
                            }
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
            "Checkbox" | "Switch" => {
                let label = self.prop_text(node, "label");
                let checked = self.prop_bool(node, "checked");
                let disabled = self.prop_bool(node, "disabled");
                let path = self.two_way(node, "checked");
                let changed = node.clone();
                let on_change = move |value: &bool, _: &mut Window, cx: &mut App| {
                    weak.update(cx, |this, cx| {
                        if let Some(path) = &path {
                            this.write(path, Json::Bool(*value), cx);
                        }
                        this.fire(&changed, "change");
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
                    .layout(if horizontal { Axis::Horizontal } else { Axis::Vertical })
                    .selected_index(selected.map(|n| n as usize))
                    .disabled(self.prop_bool(node, "disabled"))
                    .children(radios)
                    .on_click(move |index, _, cx| {
                        weak.update(cx, |this, cx| {
                            if let Some(path) = &path {
                                this.write(path, Json::from(*index), cx);
                            }
                            this.fire(&changed, "change");
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
                                this.fire(&changed, "change");
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
                let selected = self
                    .tabs
                    .get(key)
                    .copied()
                    .or(bound.map(|n| n as usize))
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
                    .with_variant(tab_variant(&self.prop_text(node, "variant").unwrap_or_default()))
                    .selected_index(selected)
                    .children(tabs)
                    .on_click(move |index: &usize, _, cx| {
                        let index = *index;
                        weak.update(cx, |this, cx| {
                            this.tabs.insert(tab_key.clone(), index);
                            if let Some(path) = &path {
                                this.write(path, Json::from(index), cx);
                            }
                            this.fire(&changed, "change");
                            cx.notify();
                        })
                        .ok();
                    });
                v_flex()
                    .gap_3()
                    .child(bar)
                    .children(content.unwrap_or_default())
                    .into_any_element()
            }
            "Separator" => Separator::horizontal()
                .when(self.prop_bool(node, "dashed"), |s| s.dashed())
                .when_some(self.prop_text(node, "label"), |s, l| s.label(l))
                .into_any_element(),
            "Link" => {
                let clicked = node.clone();
                Link::new(id)
                    .when_some(self.prop_text(node, "href"), |l, h| l.href(h))
                    .disabled(self.prop_bool(node, "disabled"))
                    .child(self.prop_text(node, "label").unwrap_or_default())
                    .on_click(move |_, _, cx| {
                        weak.update(cx, |this, _| this.fire(&clicked, "click")).ok();
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
                .with_variant(tag_variant(&self.prop_text(node, "variant").unwrap_or_default()))
                .when(self.prop_bool(node, "outline"), |t| t.outline())
                .child(self.prop_text(node, "label").unwrap_or_default())
                .into_any_element(),
            "Icon" => {
                let color = self.prop(node, "color").and_then(|v| color(v, cx));
                icon(&self.prop_text(node, "name").unwrap_or_default())
                    .with_size(size(&self.prop_text(node, "size").unwrap_or_default()))
                    .when_some(color, |i, c| i.text_color(c))
                    .into_any_element()
            }
            other => placeholder(format!("`{other}` is not in the gpui-kit catalog"), cx),
        };
        if node.style.is_empty() {
            element
        } else {
            style(div(), &node.style, cx).child(element).into_any_element()
        }
    }
}

impl Render for ForgeView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let (background, foreground) = (theme.background, theme.foreground);
        let content = self
            .doc
            .clone()
            .map(|doc| self.render_node(&doc.root, "root", window, cx));
        div()
            .id("forge-canvas")
            .size_full()
            .overflow_y_scroll()
            .bg(background)
            .text_color(foreground)
            .children(content)
    }
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

/// Applies the provisional style keys documented on the docs site.
fn style<E: Styled>(element: E, entries: &Entries, cx: &App) -> E {
    entries.iter().fold(element, |e, (key, value)| {
        let word = match value {
            Value::Enum(s) | Value::String(s) => s.as_str(),
            _ => "",
        };
        match (key.as_str(), length(value), color(value, cx)) {
            ("gap", Some(l), _) => e.gap(l),
            ("padding", Some(l), _) => e.p(l),
            ("width", Some(l), _) => e.w(l),
            ("height", Some(l), _) => e.h(l),
            ("max-width", Some(l), _) => e.max_w(l),
            ("background", _, Some(c)) => e.bg(c),
            ("color", _, Some(c)) => e.text_color(c),
            ("border", _, Some(c)) => e.border_1().border_color(c),
            ("radius", _, _) => match value {
                Value::Number { value, unit: None | Some(Unit::Px) } => e.rounded(px(*value as f32)),
                Value::Number { value, unit: Some(Unit::Rem) } => e.rounded(rems(*value as f32)),
                _ => e,
            },
            ("grow", _, _) if matches!(value, Value::Bool(true)) => e.flex_grow(1.0),
            ("wrap", _, _) if matches!(value, Value::Bool(true)) => e.flex_wrap(),
            ("align", _, _) => match word {
                "start" => e.items_start(),
                "center" => e.items_center(),
                "end" => e.items_end(),
                "stretch" => e.items_stretch(),
                _ => e,
            },
            ("justify", _, _) => match word {
                "start" => e.justify_start(),
                "center" => e.justify_center(),
                "end" => e.justify_end(),
                "between" => e.justify_between(),
                _ => e,
            },
            ("size", _, _) => match word {
                "xs" => e.text_xs(),
                "sm" => e.text_sm(),
                "md" => e.text_base(),
                "lg" => e.text_lg(),
                "xl" => e.text_xl(),
                "2xl" => e.text_2xl(),
                "3xl" => e.text_3xl(),
                _ => match value {
                    Value::Number { value, .. } => e.text_size(px(*value as f32)),
                    _ => e,
                },
            },
            ("weight", _, _) => match word {
                "medium" => e.font_weight(FontWeight::MEDIUM),
                "semibold" => e.font_weight(FontWeight::SEMIBOLD),
                "bold" => e.font_weight(FontWeight::BOLD),
                _ => e.font_weight(FontWeight::NORMAL),
            },
            _ => e,
        }
    })
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

