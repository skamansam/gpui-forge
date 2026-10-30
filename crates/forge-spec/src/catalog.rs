//! Catalog manifests (`docs/spec/catalog-manifest.md`): the elements a library offers.

use std::collections::HashMap;
use std::fmt;

use crate::diagnostic::{Diagnostic, Diagnostics, Pos};
use crate::model::Value;
use crate::value::{self, is_element_name, is_namespace, is_prop_key};
use crate::yaml::tree::{self, Scalar, YNode};

/// Version of the catalog manifest format this crate reads.
pub const CATALOG_VERSION: &str = "0.1";

#[derive(Clone, Debug, PartialEq)]
pub struct Catalog {
    /// Library id, e.g. `gpui-kit`, `ely-gpui-components`.
    pub library: String,
    pub name: Option<String>,
    /// Version of the library this manifest describes.
    pub version: String,
    pub license: Option<String>,
    /// Supported bases and the base version requirement for each.
    pub bases: Vec<(String, String)>,
    pub elements: Vec<ElementDef>,
}

impl Catalog {
    pub fn element(&self, name: &str) -> Option<&ElementDef> {
        self.elements.iter().find(|e| e.name == name)
    }

    pub fn supports_base(&self, base: &str) -> bool {
        self.bases.iter().any(|(id, _)| id == base)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ElementDef {
    pub name: String,
    pub summary: Option<String>,
    /// The library type that implements the element, for docs and adapter authors.
    pub component: Option<String>,
    /// Prop that receives the node's positional content (`Button: Sign in`).
    pub content: Option<String>,
    pub props: Vec<PropDef>,
    pub events: Vec<EventDef>,
    pub children: Children,
}

impl ElementDef {
    pub fn prop(&self, name: &str) -> Option<&PropDef> {
        self.props.iter().find(|p| p.name == name)
    }

    pub fn event(&self, name: &str) -> Option<&EventDef> {
        self.events.iter().find(|e| e.name == name)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PropDef {
    pub name: String,
    pub ty: PropType,
    pub default: Option<Value>,
    pub required: bool,
    /// Whether the prop accepts a two-way binding (`bind $path`).
    pub bindable: bool,
    pub summary: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropType {
    String,
    Number,
    /// A number with an optional unit (`12px`, `1.5rem`, `50%`).
    Length,
    Bool,
    Color,
    Icon,
    Enum(Vec<String>),
    List(Box<PropType>),
    /// A YAML mapping; nested keys are the element's own contract.
    Map,
}

impl PropType {
    /// Whether `value` can be assigned to a prop of this type. Bindings are checked at runtime.
    pub fn accepts(&self, value: &Value) -> bool {
        use PropType as T;
        match (self, value) {
            (_, Value::Binding { .. }) => true,
            (T::Color | T::Length | T::Number, Value::Token(_)) => true,
            (
                T::String,
                Value::String(_)
                | Value::Interpolated(_)
                | Value::Enum(_)
                | Value::Number { .. }
                | Value::Bool(_),
            ) => true,
            (T::Number, Value::Number { unit: None, .. }) => true,
            (T::Length, Value::Number { .. }) => true,
            (T::Bool, Value::Bool(_)) => true,
            (T::Color, Value::Color(_)) => true,
            (T::Icon, Value::Enum(_) | Value::String(_)) => true,
            (T::Enum(values), Value::Enum(v)) => values.contains(v),
            (T::List(item), Value::List(items)) => items.iter().all(|i| item.accepts(i)),
            (T::Map, Value::Map(_)) => true,
            _ => false,
        }
    }
}

impl fmt::Display for PropType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PropType::String => f.write_str("string"),
            PropType::Number => f.write_str("number"),
            PropType::Length => f.write_str("length"),
            PropType::Bool => f.write_str("bool"),
            PropType::Color => f.write_str("color"),
            PropType::Icon => f.write_str("icon"),
            PropType::Enum(values) => write!(f, "one of {}", values.join(", ")),
            PropType::List(item) => write!(f, "list<{item}>"),
            PropType::Map => write!(f, "map"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventDef {
    pub name: String,
    /// Type of the value passed to the handler, if any (`bool`, `string`, `number`).
    pub payload: Option<String>,
    pub summary: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Children {
    #[default]
    None,
    Any,
    /// Only these elements, e.g. `Tabs` takes `Tab`.
    Only(Vec<String>),
}

/// Reads a catalog manifest written in YAML.
pub fn from_yaml(src: &str) -> Result<Catalog, Diagnostics> {
    let tree = match tree::parse(src) {
        Ok(Some(tree)) => tree,
        Ok(None) => {
            return Err(Diagnostics(vec![Diagnostic::new(
                Pos::start(),
                "empty catalog manifest",
            )]));
        }
        Err(d) => return Err(Diagnostics(vec![d])),
    };
    let mut loader = Loader { diags: Vec::new() };
    let catalog = loader.catalog(tree);
    loader.diags.sort_by_key(|d| d.pos);
    match catalog {
        Some(catalog) if loader.diags.is_empty() => Ok(catalog),
        _ => Err(Diagnostics(loader.diags)),
    }
}

struct Loader {
    diags: Vec<Diagnostic>,
}

impl Loader {
    fn error(&mut self, pos: Pos, message: impl Into<String>) {
        self.diags.push(Diagnostic::new(pos, message));
    }

    fn mapping(&mut self, node: YNode, what: &str) -> Vec<(Scalar, YNode)> {
        match node {
            YNode::Map(entries, _) => {
                let mut seen: HashMap<String, Pos> = HashMap::new();
                for (key, _) in &entries {
                    if let Some(first) = seen.get(&key.text) {
                        let message =
                            format!("duplicate key `{}` (first defined at {first})", key.text);
                        self.error(key.pos, message);
                    } else {
                        seen.insert(key.text.clone(), key.pos);
                    }
                }
                entries
            }
            YNode::Scalar(s) if s.is_null() => Vec::new(),
            other => {
                self.error(other.pos(), format!("{what} must be a mapping"));
                Vec::new()
            }
        }
    }

    fn scalar(&mut self, node: YNode) -> Option<Scalar> {
        match node {
            YNode::Scalar(s) if !s.is_null() => Some(s),
            other => {
                self.error(other.pos(), "expected a value");
                None
            }
        }
    }

    fn text(&mut self, node: YNode) -> Option<String> {
        self.scalar(node).map(|s| s.text)
    }

    fn flag(&mut self, node: YNode) -> bool {
        match self.scalar(node) {
            Some(s) if s.text == "true" => true,
            Some(s) if s.text == "false" => false,
            Some(s) => {
                self.error(s.pos, "expected `true` or `false`");
                false
            }
            None => false,
        }
    }

    fn unknown(&mut self, key: &Scalar, expected: &str) {
        self.diags.push(
            Diagnostic::new(key.pos, format!("unknown key `{}`", key.text))
                .with_hint(format!("expected {expected}")),
        );
    }

    fn catalog(&mut self, tree: YNode) -> Option<Catalog> {
        let at = tree.pos();
        let (mut library, mut version, mut format) = (None, None, None);
        let (mut name, mut license) = (None, None);
        let (mut bases, mut elements) = (Vec::new(), Vec::new());
        for (key, val) in self.mapping(tree, "a catalog manifest") {
            match key.text.as_str() {
                "$schema" => {}
                "catalog" => format = self.scalar(val),
                "library" => library = self.text(val),
                "name" => name = self.text(val),
                "version" => version = self.text(val),
                "license" => license = self.text(val),
                "bases" => bases = self.bases(val),
                "elements" => elements = self.elements(val),
                _ => self.unknown(
                    &key,
                    "`catalog`, `library`, `name`, `version`, `license`, `bases` or `elements`",
                ),
            }
        }
        match &format {
            Some(f) if f.text != CATALOG_VERSION => self.error(
                f.pos,
                format!(
                    "unsupported catalog format `{}`; this build reads `{CATALOG_VERSION}`",
                    f.text
                ),
            ),
            None => self.error(at, "missing `catalog` key"),
            _ => {}
        }
        if library.is_none() {
            self.error(at, "missing `library` key");
        }
        if version.is_none() {
            self.error(at, "missing `version` key");
        }
        if bases.is_empty() {
            self.error(
                at,
                "a catalog must list at least one supported base under `bases`",
            );
        }
        Some(Catalog {
            library: library?,
            name,
            version: version?,
            license,
            bases,
            elements,
        })
    }

    fn bases(&mut self, node: YNode) -> Vec<(String, String)> {
        let mut bases = Vec::new();
        for (key, val) in self.mapping(node, "`bases`") {
            if !is_namespace(&key.text) {
                self.error(key.pos, format!("invalid base id `{}`", key.text));
            } else if let Some(req) = self.text(val) {
                bases.push((key.text, req));
            }
        }
        bases
    }

    fn elements(&mut self, node: YNode) -> Vec<ElementDef> {
        let entries = self.mapping(node, "`elements`");
        entries
            .into_iter()
            .filter_map(|(key, val)| self.element(key, val))
            .collect()
    }

    fn element(&mut self, key: Scalar, node: YNode) -> Option<ElementDef> {
        if !is_element_name(&key.text) {
            self.error(
                key.pos,
                format!("element names are PascalCase, found `{}`", key.text),
            );
            return None;
        }
        let mut element = ElementDef {
            name: key.text,
            summary: None,
            component: None,
            content: None,
            props: Vec::new(),
            events: Vec::new(),
            children: Children::None,
        };
        let mut content_pos = Pos::start();
        for (key, val) in self.mapping(node, "an element") {
            match key.text.as_str() {
                "summary" => element.summary = self.text(val),
                "component" => element.component = self.text(val),
                "content" => {
                    content_pos = val.pos();
                    element.content = self.text(val);
                }
                "props" => {
                    let entries = self.mapping(val, "`props`");
                    element.props = entries
                        .into_iter()
                        .filter_map(|(k, v)| self.prop(k, v))
                        .collect();
                }
                "events" => element.events = self.events(val),
                "children" => element.children = self.children(val),
                _ => self.unknown(
                    &key,
                    "`summary`, `component`, `content`, `props`, `events` or `children`",
                ),
            }
        }
        if let Some(content) = &element.content
            && element.prop(content).is_none()
        {
            let message = format!("content prop `{content}` is not defined under `props`");
            self.error(content_pos, message);
        }
        Some(element)
    }

    fn prop(&mut self, key: Scalar, node: YNode) -> Option<PropDef> {
        if !is_prop_key(&key.text) {
            self.error(key.pos, format!("invalid prop name `{}`", key.text));
            return None;
        }
        let mut prop = PropDef {
            name: key.text,
            ty: PropType::String,
            default: None,
            required: false,
            bindable: false,
            summary: None,
        };
        let (ty, values, default) = match node {
            // Shorthand: `disabled: bool`.
            YNode::Scalar(s) => (Some(s), None, None),
            node => {
                let (mut ty, mut values, mut default) = (None, None, None);
                for (key, val) in self.mapping(node, "a prop") {
                    match key.text.as_str() {
                        "type" => ty = self.scalar(val),
                        "values" => values = Some(self.words(val)),
                        "default" => default = self.scalar(val),
                        "required" => prop.required = self.flag(val),
                        "bindable" => prop.bindable = self.flag(val),
                        "summary" => prop.summary = self.text(val),
                        _ => self.unknown(
                            &key,
                            "`type`, `values`, `default`, `required`, `bindable` or `summary`",
                        ),
                    }
                }
                (ty, values, default)
            }
        };
        let Some(ty) = ty.filter(|t| !t.is_null()) else {
            self.error(key.pos, format!("prop `{}` has no `type`", prop.name));
            return None;
        };
        prop.ty = self.prop_type(&ty, values)?;
        if let Some(default) = default {
            match value::parse_value(&default.text, default.quoting) {
                Ok(v) if prop.ty.accepts(&v) => prop.default = Some(v),
                Ok(_) => {
                    let message = format!("default for `{}` must be {}", prop.name, prop.ty);
                    self.error(default.pos, message);
                }
                Err(e) => self.error(default.pos, e),
            }
        }
        Some(prop)
    }

    fn prop_type(&mut self, ty: &Scalar, values: Option<Vec<String>>) -> Option<PropType> {
        let text = ty.text.trim();
        let pos = ty.pos;
        let ty = match text {
            "string" => PropType::String,
            "number" => PropType::Number,
            "length" => PropType::Length,
            "bool" => PropType::Bool,
            "color" => PropType::Color,
            "icon" => PropType::Icon,
            "map" => PropType::Map,
            "enum" => match values {
                Some(values) if !values.is_empty() => return Some(PropType::Enum(values)),
                _ => {
                    self.error(ty.pos, "`enum` props need a non-empty `values` list");
                    return None;
                }
            },
            _ => match text.strip_prefix("list<").and_then(|t| t.strip_suffix('>')) {
                Some(item) => {
                    let item = Scalar {
                        text: item.to_owned(),
                        quoting: ty.quoting,
                        pos: ty.pos,
                    };
                    return Some(PropType::List(Box::new(self.prop_type(&item, values)?)));
                }
                None => {
                    let message = format!(
                        "unknown prop type `{text}`; expected string, number, length, bool, color, icon, enum, list<T> or map"
                    );
                    self.error(ty.pos, message);
                    return None;
                }
            },
        };
        if values.is_some() {
            self.error(pos, "`values` only applies to `enum` props");
        }
        Some(ty)
    }

    fn words(&mut self, node: YNode) -> Vec<String> {
        let YNode::Seq(items, _) = node else {
            self.error(node.pos(), "`values` must be a list");
            return Vec::new();
        };
        let mut words = Vec::new();
        for item in items {
            if let Some(word) = self.scalar(item) {
                if value::is_ident(&word.text) {
                    words.push(word.text);
                } else {
                    let message = format!("enum values are bare words, found `{}`", word.text);
                    self.error(word.pos, message);
                }
            }
        }
        words
    }

    fn events(&mut self, node: YNode) -> Vec<EventDef> {
        let mut events = Vec::new();
        for (key, val) in self.mapping(node, "`events`") {
            if !is_prop_key(&key.text) {
                self.error(key.pos, format!("invalid event name `{}`", key.text));
                continue;
            }
            let mut event = EventDef {
                name: key.text,
                payload: None,
                summary: None,
            };
            match val {
                YNode::Scalar(s) if s.is_null() => {}
                // Shorthand: `change: bool`.
                YNode::Scalar(s) => event.payload = Some(s.text),
                val => {
                    for (key, val) in self.mapping(val, "an event") {
                        match key.text.as_str() {
                            "payload" => event.payload = self.text(val),
                            "summary" => event.summary = self.text(val),
                            _ => self.unknown(&key, "`payload` or `summary`"),
                        }
                    }
                }
            }
            events.push(event);
        }
        events
    }

    fn children(&mut self, node: YNode) -> Children {
        match node {
            YNode::Scalar(s) if s.text == "any" => Children::Any,
            YNode::Scalar(s) if s.text == "none" => Children::None,
            YNode::Seq(items, _) => {
                let mut names = Vec::new();
                for item in items {
                    let Some(name) = self.scalar(item) else {
                        continue;
                    };
                    let element = name.text.rsplit('.').next().unwrap_or_default();
                    if is_element_name(element) {
                        names.push(name.text);
                    } else {
                        self.error(name.pos, format!("invalid element name `{}`", name.text));
                    }
                }
                Children::Only(names)
            }
            other => {
                self.error(
                    other.pos(),
                    "`children` must be `any`, `none` or a list of element names",
                );
                Children::None
            }
        }
    }
}
