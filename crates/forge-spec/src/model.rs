//! The canonical, format-independent document model (`docs/spec/document-model.md`).

use std::fmt;

/// Ordered key/value pairs; order is preserved so hand-edited files diff cleanly.
pub type Entries = Vec<(String, Value)>;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct Document {
    /// JSON Schema URL for editors (`$schema`).
    pub schema: Option<String>,
    pub version: String,
    /// Base component system; inherited from `forge.toml` when absent.
    pub base: Option<String>,
    pub imports: Vec<Import>,
    pub meta: Vec<(String, String)>,
    /// Top-level `Dialog#name:` declarations, opened by `open_dialog(name)`.
    pub dialogs: Vec<Node>,
    pub root: Node,
    /// The YAML source and tree this document was read from, kept so the writer
    /// can splice unchanged regions verbatim (comments, anchors, formatting).
    /// `None` for documents built programmatically.
    #[serde(skip)]
    pub raw: Option<Raw>,
}

/// A parsed document's original text and marked YAML tree.
#[derive(Clone, Debug)]
pub struct Raw {
    pub source: String,
    pub(crate) tree: saphyr::MarkedYamlOwned,
}

// Provenance, not semantics — two equal documents may come from different text.
impl PartialEq for Raw {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Import {
    pub alias: String,
    pub library: String,
    pub version: Option<String>,
}

impl Document {
    /// Finds a node by index path (`"0.2.1"` → `root.children[2].children[1]`;
    /// `"0"` is the root). This is the same path shape `forge.Outline` reports for `select`.
    pub fn node_at(&self, path: &str) -> Option<&Node> {
        let mut indices = path.split('.');
        if indices.next()?.parse::<usize>().ok()? != 0 {
            return None;
        }
        let mut node = &self.root;
        for index in indices {
            let index: usize = index.parse().ok()?;
            node = node.children.get(index)?;
        }
        Some(node)
    }

    /// Mutable version of [`Document::node_at`].
    pub fn node_at_mut(&mut self, path: &str) -> Option<&mut Node> {
        let mut indices = path.split('.');
        if indices.next()?.parse::<usize>().ok()? != 0 {
            return None;
        }
        let mut node = &mut self.root;
        for index in indices {
            let index: usize = index.parse().ok()?;
            node = node.children.get_mut(index)?;
        }
        Some(node)
    }

    /// Sets or replaces a prop on the node at `path`; `None` removes it. Returns whether
    /// the path resolved — a `false` return means no change was made.
    pub fn set_prop(&mut self, path: &str, name: &str, value: Option<Value>) -> bool {
        let Some(node) = self.node_at_mut(path) else {
            return false;
        };
        match value {
            Some(value) => {
                match node.props.iter_mut().find(|(k, _)| k == name) {
                    Some((_, slot)) => *slot = value,
                    None => node.props.push((name.to_owned(), value)),
                }
            }
            None => node.props.retain(|(k, _)| k != name),
        }
        true
    }

    /// Sets or replaces a style entry on the node at `path`; `None` removes it.
    /// Returns whether the path resolved.
    pub fn set_style(&mut self, path: &str, name: &str, value: Option<Value>) -> bool {
        let Some(node) = self.node_at_mut(path) else {
            return false;
        };
        match value {
            Some(value) => {
                match node.style.iter_mut().find(|(k, _)| k == name) {
                    Some((_, slot)) => *slot = value,
                    None => node.style.push((name.to_owned(), value)),
                }
            }
            None => node.style.retain(|(k, _)| k != name),
        }
        true
    }

    /// Inserts `node` as a child of the node at `path` (`index` defaults to the end).
    /// Returns whether the path resolved.
    pub fn insert_node(&mut self, path: &str, node: Node, index: Option<usize>) -> bool {
        let Some(parent) = self.node_at_mut(path) else {
            return false;
        };
        let index = index.unwrap_or(parent.children.len()).min(parent.children.len());
        parent.children.insert(index, node);
        true
    }

    /// Removes the node at `path` (the root cannot be removed). Returns the removed node.
    pub fn remove_node(&mut self, path: &str) -> Option<Node> {
        let (parent_path, index) = path.rsplit_once('.')?;
        let index: usize = index.parse().ok()?;
        let parent = self.node_at_mut(parent_path)?;
        (index < parent.children.len())
            .then(|| parent.children.remove(index))
    }

    /// Inserts `node` as the sibling immediately after `after` — into its
    /// parent at `index + 1`. Returns the inserted node's path.
    pub fn insert_after(&mut self, after: &str, node: Node) -> Option<String> {
        let (parent_path, index) = after.rsplit_once('.')?;
        let index: usize = index.parse().ok()?;
        let parent = self.node_at_mut(parent_path)?;
        let at = (index + 1).min(parent.children.len());
        parent.children.insert(at, node);
        Some(format!("{parent_path}.{at}"))
    }

    /// Moves the node at `path` to the position immediately after `after`,
    /// keeping siblings in order when moving within the same parent.
    pub fn move_after(&mut self, path: &str, after: &str) -> Option<String> {
        if path == "0" || path == after || after.starts_with(&format!("{path}.")) {
            return None;
        }
        let (parent_path, index) = after.rsplit_once('.')?;
        let index: usize = index.parse().ok()?;
        let (src_parent, src_index) = path.rsplit_once('.')?;
        let src_index: usize = src_index.parse().ok()?;
        let node = self.remove_node(path)?;
        // A sibling removed above `after` shifts its slot left by one.
        let at = if src_parent == parent_path && src_index < index {
            index
        } else {
            index + 1
        };
        let parent = self.node_at_mut(parent_path)?;
        let at = at.min(parent.children.len());
        parent.children.insert(at, node);
        Some(format!("{parent_path}.{at}"))
    }

    /// Moves the node at `path` under `new_parent` (appended, or at `index`).
    /// Refuses to move the root, or a node into itself/descendants.
    pub fn reparent(&mut self, path: &str, new_parent: &str, index: Option<usize>) -> bool {
        if path == "0" || path == new_parent || new_parent.starts_with(&format!("{path}.")) {
            return false;
        }
        let Some(node) = self.remove_node(path) else {
            return false;
        };
        self.insert_node(new_parent, node, index)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Node {
    pub element: ElementRef,
    pub id: Option<String>,
    /// Positional value; the catalog maps it to the element's primary prop.
    pub content: Option<Value>,
    pub props: Entries,
    pub style: Entries,
    pub events: Vec<EventBinding>,
    pub children: Vec<Node>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct ElementRef {
    /// Import alias; `None` is the base library.
    pub namespace: Option<String>,
    pub name: String,
}

impl fmt::Display for ElementRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(namespace) = &self.namespace {
            write!(f, "{namespace}.")?;
        }
        f.write_str(&self.name)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct EventBinding {
    pub event: String,
    pub handler: HandlerRef,
}

/// A hook id resolved by the hook registry at runtime, with arguments.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct HandlerRef {
    pub name: String,
    pub args: Vec<Value>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub enum Value {
    String(String),
    Interpolated(Vec<Segment>),
    Number {
        value: f64,
        unit: Option<Unit>,
    },
    Bool(bool),
    /// A bare word; the catalog decides whether it is an enum value or plain text.
    Enum(String),
    Color(Color),
    /// Theme token path, written `$theme.<path>`.
    Token(Path),
    List(Vec<Value>),
    /// A YAML mapping — `highlighting: {languages: [yaml], default: yaml}`.
    Map(Entries),
    Binding {
        path: Path,
        mode: BindingMode,
    },
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub enum Segment {
    Text(String),
    Binding(Path),
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Px,
    Rem,
    Percent,
    Fr,
}

impl Unit {
    pub fn as_str(self) -> &'static str {
        match self {
            Unit::Px => "px",
            Unit::Rem => "rem",
            Unit::Percent => "%",
            Unit::Fr => "fr",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "px" => Unit::Px,
            "rem" => Unit::Rem,
            "%" => Unit::Percent,
            "fr" => Unit::Fr,
            _ => return None,
        })
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    /// Parses `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa`.
    pub fn parse_hex(text: &str) -> Option<Self> {
        let hex = text.strip_prefix('#')?;
        if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let short = |i: usize| u8::from_str_radix(&hex[i..=i], 16).ok().map(|v| v * 17);
        let long = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
        let (r, g, b, a) = match hex.len() {
            3 => (short(0)?, short(1)?, short(2)?, 255),
            4 => (short(0)?, short(1)?, short(2)?, short(3)?),
            6 => (long(0)?, long(2)?, long(4)?, 255),
            8 => (long(0)?, long(2)?, long(4)?, long(6)?),
            _ => return None,
        };
        Some(Self { r, g, b, a })
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)?;
        if self.a != 255 {
            write!(f, "{:02x}", self.a)?;
        }
        Ok(())
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingMode {
    OneWay,
    TwoWay,
}

/// A path into the view state, e.g. `form.items[0].name`.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Path(pub Vec<PathSegment>);

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum PathSegment {
    Field(String),
    Index(usize),
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, segment) in self.0.iter().enumerate() {
            match segment {
                PathSegment::Field(name) if i == 0 => f.write_str(name)?,
                PathSegment::Field(name) => write!(f, ".{name}")?,
                PathSegment::Index(index) => write!(f, "[{index}]")?,
            }
        }
        Ok(())
    }
}
