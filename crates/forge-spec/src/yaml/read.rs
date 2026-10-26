//! Forge YAML reader: YAML tree -> document model, collecting every error it can.

use std::collections::HashMap;

use super::tree::{self, Scalar, YNode};
use crate::SPEC_VERSION;
use crate::diagnostic::{Diagnostic, Diagnostics, Pos};
use crate::model::{Document, ElementRef, EventBinding, Import, Node, Value};
use crate::source_map::{NodeSpans, SourceMap};
use crate::value::{
    self, is_element_name, is_ident, is_namespace, is_prop_key, is_two_way, parse_handler,
};

const COLOR_HINT: &str = "quote colors, e.g. `\"#3b82f6\"` (an unquoted `#` starts a YAML comment)";

/// Reads a Forge YAML document.
pub fn from_str(src: &str) -> Result<Document, Diagnostics> {
    from_str_with_source_map(src).map(|(doc, _)| doc)
}

/// Reads a Forge YAML document, also returning where each node, prop and event was written.
pub fn from_str_with_source_map(src: &str) -> Result<(Document, SourceMap), Diagnostics> {
    let tree = match tree::parse(src) {
        Ok(Some(tree)) => tree,
        Ok(None) => {
            return Err(Diagnostics(vec![
                Diagnostic::new(Pos::start(), "empty document")
                    .with_hint("start with `forge: \"0.1\"` and a `root:` node"),
            ]));
        }
        Err(d) => return Err(Diagnostics(vec![d])),
    };
    let mut reader = Reader {
        src,
        diags: Vec::new(),
        path: Vec::new(),
        map: SourceMap::default(),
    };
    let doc = reader.document(tree);
    reader.diags.sort_by_key(|d| d.pos);
    match doc {
        Some(doc) if reader.diags.is_empty() => Ok((doc, reader.map)),
        _ => Err(Diagnostics(reader.diags)),
    }
}

struct Reader<'a> {
    src: &'a str,
    diags: Vec<Diagnostic>,
    /// Child indices from the root to the node being read.
    path: Vec<usize>,
    map: SourceMap,
}

impl Reader<'_> {
    fn error(&mut self, pos: Pos, message: impl Into<String>) {
        self.diags.push(Diagnostic::new(pos, message));
    }

    fn hint(&mut self, pos: Pos, message: impl Into<String>, hint: impl Into<String>) {
        self.diags
            .push(Diagnostic::new(pos, message).with_hint(hint));
    }

    fn document(&mut self, tree: YNode) -> Option<Document> {
        let YNode::Map(entries, _) = tree else {
            self.hint(
                tree.pos(),
                "a Forge document must be a YAML mapping",
                "start with `forge: \"0.1\"` and a `root:` node",
            );
            return None;
        };
        self.check_duplicates(&entries);
        let (mut schema, mut version, mut base, mut root) = (None, None, None, None);
        let (mut imports, mut meta) = (Vec::new(), Vec::new());
        let (mut saw_version, mut saw_root) = (false, false);
        for (key, val) in entries {
            match key.text.as_str() {
                "$schema" => schema = self.text(val),
                "forge" => {
                    saw_version = true;
                    let at = val.pos();
                    version = self.text(val).filter(|v| {
                        let ok = v == SPEC_VERSION;
                        if !ok {
                            self.error(
                                at,
                                format!("unsupported spec version `{v}`; this build reads `{SPEC_VERSION}`"),
                            );
                        }
                        ok
                    });
                }
                "base" => base = self.text(val),
                "use" => imports = self.imports(val),
                "meta" => meta = self.meta(val),
                "root" => {
                    saw_root = true;
                    root = self.node(val);
                }
                other => self.hint(
                    key.pos,
                    format!("unknown top-level key `{other}`"),
                    "expected `$schema`, `forge`, `base`, `use`, `meta` or `root`",
                ),
            }
        }
        if !saw_version {
            self.hint(Pos::start(), "missing `forge` key", "add `forge: \"0.1\"`");
        }
        if !saw_root {
            self.hint(Pos::start(), "missing `root` key", "add a `root:` node");
        }
        Some(Document {
            schema,
            version: version?,
            base,
            imports,
            meta,
            root: root?,
        })
    }

    fn text(&mut self, val: YNode) -> Option<String> {
        match val {
            YNode::Scalar(s) if !s.is_null() => Some(s.text),
            YNode::Scalar(s) => {
                self.error(s.pos, "expected a value");
                None
            }
            other => {
                self.error(other.pos(), "expected text, found a list or mapping");
                None
            }
        }
    }

    fn mapping(&mut self, val: YNode, what: &str) -> Vec<(Scalar, YNode)> {
        match val {
            YNode::Map(entries, _) => {
                self.check_duplicates(&entries);
                entries
            }
            YNode::Scalar(s) if s.is_null() => Vec::new(),
            other => {
                self.error(other.pos(), format!("{what} must be a mapping"));
                Vec::new()
            }
        }
    }

    fn imports(&mut self, val: YNode) -> Vec<Import> {
        let mut imports = Vec::new();
        for (key, val) in self.mapping(val, "`use`") {
            if !is_namespace(&key.text) {
                self.hint(
                    key.pos,
                    format!("invalid import alias `{}`", key.text),
                    "aliases are lower-case identifiers like `ely`",
                );
                continue;
            }
            let at = val.pos();
            let Some(text) = self.text(val) else { continue };
            let mut parts = text.split_whitespace();
            match (parts.next(), parts.next(), parts.next()) {
                (Some(library), version, None) => {
                    self.map.imports.insert(key.text.clone(), key.pos);
                    imports.push(Import {
                        alias: key.text,
                        library: library.to_owned(),
                        version: version.map(str::to_owned),
                    });
                }
                _ => self.hint(
                    at,
                    format!("invalid import `{text}`"),
                    "write `<library-id> [<version-req>]`, e.g. `ely-gpui-components ^0.4`",
                ),
            }
        }
        imports
    }

    fn meta(&mut self, val: YNode) -> Vec<(String, String)> {
        let mut meta = Vec::new();
        for (key, val) in self.mapping(val, "`meta`") {
            if let Some(text) = self.text(val) {
                meta.push((key.text, text));
            }
        }
        meta
    }

    fn node(&mut self, val: YNode) -> Option<Node> {
        let (entries, at) = match val {
            YNode::Map(entries, at) => (entries, at),
            other => {
                self.hint(
                    other.pos(),
                    "expected a node",
                    "nodes are mappings like `Button: Sign in` or `VStack#root:`",
                );
                return None;
            }
        };
        self.check_duplicates(&entries);
        let mut node = Node::default();
        let mut spans = NodeSpans::default();
        let mut element: Option<ElementRef> = None;
        for (key, val) in entries {
            let name = key.text.as_str();
            if name == "style" {
                node.style = self.style(val);
            } else if name == "children" {
                node.children = self.children(val);
            } else if let Some(event) = name.strip_prefix("on ") {
                if let Some(binding) = self.event(event.trim(), &key, val) {
                    spans.events.insert(binding.event.clone(), key.pos);
                    node.events.push(binding);
                }
            } else if let Some((el, id)) = element_key(name) {
                if let Some(first) = &element {
                    self.hint(
                        key.pos,
                        format!("node has two element keys (`{first}`, `{el}`)"),
                        "split them into separate list items under `children:`",
                    );
                    continue;
                }
                spans.element = key.pos;
                spans.content = Some(val.pos());
                node.id = id;
                node.content = self.content(val);
                element = Some(el);
            } else if is_prop_key(name) {
                if let Some(value) = self.value(val, &key, true) {
                    spans.props.insert(key.text.clone(), key.pos);
                    node.props.push((key.text, value));
                }
            } else {
                self.hint(
                    key.pos,
                    format!("`{name}` is not a valid key"),
                    "elements are PascalCase (`Button`, `ely.Badge`), props are lower-case (`variant`), events start with `on `",
                );
            }
        }
        let Some(element) = element else {
            self.hint(
                at,
                "node has no element key",
                "start the node with an element, e.g. `Button:` or `VStack#root:`",
            );
            return None;
        };
        node.element = element;
        self.map.nodes.insert(self.path.clone(), spans);
        Some(node)
    }

    fn content(&mut self, val: YNode) -> Option<Value> {
        match val {
            YNode::Scalar(s) if s.is_null() => None,
            YNode::Map(_, at) => {
                self.hint(
                    at,
                    "an element key holds the node's content, not its props",
                    "write props on the following lines at the same indentation as the element key",
                );
                None
            }
            other => self.typed(other, false),
        }
    }

    fn style(&mut self, val: YNode) -> Vec<(String, Value)> {
        let mut style = Vec::new();
        for (key, val) in self.mapping(val, "`style`") {
            if !is_prop_key(&key.text) {
                self.error(
                    key.pos,
                    format!("`{}` is not a valid style property", key.text),
                );
            } else if let Some(value) = self.value(val, &key, false) {
                style.push((key.text, value));
            }
        }
        style
    }

    fn children(&mut self, val: YNode) -> Vec<Node> {
        match val {
            YNode::Seq(items, _) => {
                let mut nodes = Vec::new();
                for item in items {
                    self.path.push(nodes.len());
                    if let Some(node) = self.node(item) {
                        nodes.push(node);
                    }
                    self.path.pop();
                }
                nodes
            }
            YNode::Scalar(s) if s.is_null() => Vec::new(),
            other => {
                self.hint(
                    other.pos(),
                    "`children` must be a list of nodes",
                    "start each child with `- `",
                );
                Vec::new()
            }
        }
    }

    fn event(&mut self, event: &str, key: &Scalar, val: YNode) -> Option<EventBinding> {
        if !is_prop_key(event) {
            self.error(key.pos, format!("invalid event name `{event}`"));
            return None;
        }
        let at = val.pos();
        let text = self.text(val)?;
        match parse_handler(&text) {
            Ok(handler) => Some(EventBinding {
                event: event.to_owned(),
                handler,
            }),
            Err(e) => {
                self.error(at, e);
                None
            }
        }
    }

    /// A prop or style value; empty values get a hint (often an unquoted color).
    fn value(&mut self, val: YNode, key: &Scalar, allow_two_way: bool) -> Option<Value> {
        if let YNode::Scalar(s) = &val
            && s.is_null()
        {
            let line = self.src.lines().nth(key.pos.line.saturating_sub(1));
            let hint = if line.is_some_and(|l| l.contains('#')) {
                COLOR_HINT
            } else {
                "give it a value or remove the key"
            };
            self.hint(key.pos, format!("`{}` has no value", key.text), hint);
            return None;
        }
        self.typed(val, allow_two_way)
    }

    fn typed(&mut self, val: YNode, allow_two_way: bool) -> Option<Value> {
        match val {
            YNode::Scalar(s) => match value::parse_value(&s.text, s.quoting) {
                Ok(v) if !allow_two_way && is_two_way(&v) => {
                    self.error(s.pos, "`bind` (two-way binding) is only allowed on props");
                    None
                }
                Ok(v) => Some(v),
                Err(e) => {
                    self.error(s.pos, e);
                    None
                }
            },
            YNode::Seq(items, _) => {
                let len = items.len();
                let values: Vec<_> = items
                    .into_iter()
                    .filter_map(|item| self.typed(item, allow_two_way))
                    .collect();
                (values.len() == len).then_some(Value::List(values))
            }
            YNode::Map(_, at) => {
                self.hint(
                    at,
                    "expected a value, found a mapping",
                    "text starting with `{` is a YAML mapping; quote it, e.g. `\"{$user.name}!\"`",
                );
                None
            }
        }
    }

    fn check_duplicates(&mut self, entries: &[(Scalar, YNode)]) {
        let mut seen: HashMap<&str, Pos> = HashMap::new();
        for (key, _) in entries {
            if let Some(first) = seen.get(key.text.as_str()) {
                self.diags.push(Diagnostic::new(
                    key.pos,
                    format!("duplicate key `{}` (first defined at {first})", key.text),
                ));
            } else {
                seen.insert(&key.text, key.pos);
            }
        }
    }
}

/// Parses `[alias.]Element[#id]` or `[alias.]Element .id`.
fn element_key(key: &str) -> Option<(ElementRef, Option<String>)> {
    let (path, id) = match key.split_once('#').or_else(|| key.split_once(" .")) {
        Some((path, id)) => (path, Some(id)),
        None => (key, None),
    };
    if id.is_some_and(|id| !is_ident(id)) {
        return None;
    }
    let (namespace, name) = match path.rsplit_once('.') {
        Some((namespace, name)) => (Some(namespace), name),
        None => (None, path),
    };
    if !is_element_name(name) || namespace.is_some_and(|ns| !is_namespace(ns)) {
        return None;
    }
    Some((
        ElementRef {
            namespace: namespace.map(str::to_owned),
            name: name.to_owned(),
        },
        id.map(str::to_owned),
    ))
}
