//! Document validation against catalog manifests.
//!
//! Checks that need no catalog (syntax, value micro-syntax, duplicate keys) happen while
//! reading; this pass checks what depends on the selected base and imported libraries.

use std::collections::HashMap;

use crate::catalog::{Catalog, Children, ElementDef, PropDef};
use crate::diagnostic::{Diagnostic, Pos};
use crate::model::{Document, ElementRef, Node, Value};
use crate::source_map::{NodeSpans, SourceMap};
use crate::value::is_two_way;

/// Validates `doc` for the project's `base` against the installed `catalogs`.
///
/// `base` is the project's base (from `forge.toml`); a document that declares a different
/// `base:` is reported and not checked further. The base library is the catalog whose `library`
/// id equals `base`. Without a source map,
/// diagnostics point at `1:1`.
pub fn validate(
    doc: &Document,
    base: &str,
    catalogs: &[Catalog],
    map: Option<&SourceMap>,
) -> Vec<Diagnostic> {
    let mut v = Validator {
        map,
        diags: Vec::new(),
        base: None,
        imports: HashMap::new(),
        ids: HashMap::new(),
    };
    v.document(doc, base, catalogs);
    v.diags.sort_by_key(|d| d.pos);
    v.diags
}

struct Validator<'a> {
    map: Option<&'a SourceMap>,
    diags: Vec<Diagnostic>,
    imports: HashMap<&'a str, &'a Catalog>,
    ids: HashMap<&'a str, Pos>,
    base: Option<&'a Catalog>,
}

impl<'a> Validator<'a> {
    fn error(&mut self, pos: Pos, message: impl Into<String>) {
        self.diags.push(Diagnostic::new(pos, message));
    }

    fn hint(&mut self, pos: Pos, message: impl Into<String>, hint: impl Into<String>) {
        self.diags
            .push(Diagnostic::new(pos, message).with_hint(hint));
    }

    fn document(&mut self, doc: &'a Document, base: &str, catalogs: &'a [Catalog]) {
        if let Some(declared) = &doc.base
            && declared != base
        {
            let pos = self.map.and_then(|m| m.base).unwrap_or_else(Pos::start);
            self.hint(
                pos,
                format!("document declares base `{declared}` but the project uses `{base}`"),
                "change `base:` in the document or the project's base in forge.toml",
            );
            return;
        }
        let Some(base_catalog) = catalogs.iter().find(|c| c.library == base) else {
            self.error(
                Pos::start(),
                format!("no catalog installed for base `{base}`"),
            );
            return;
        };
        for import in &doc.imports {
            let pos = self
                .map
                .and_then(|m| m.imports.get(&import.alias))
                .copied()
                .unwrap_or_else(Pos::start);
            match catalogs.iter().find(|c| c.library == import.library) {
                None => self.hint(
                    pos,
                    format!("unknown library `{}`", import.library),
                    "no catalog manifest is installed for it",
                ),
                Some(catalog) if !catalog.supports_base(base) => {
                    let bases: Vec<&str> = catalog.bases.iter().map(|(b, _)| b.as_str()).collect();
                    self.hint(
                        pos,
                        format!("`{}` does not support the `{base}` base", import.library),
                        format!("it supports: {}", bases.join(", ")),
                    );
                }
                Some(catalog) => {
                    if self.imports.insert(&import.alias, catalog).is_some() {
                        self.error(pos, format!("duplicate import alias `{}`", import.alias));
                    }
                }
            }
        }
        self.base = Some(base_catalog);
        self.node(&doc.root, base_catalog, &mut Vec::new());
    }

    fn spans(&self, path: &[usize]) -> Option<&'a NodeSpans> {
        self.map.and_then(|m| m.nodes.get(path))
    }

    fn node(&mut self, node: &'a Node, base: &'a Catalog, path: &mut Vec<usize>) {
        let spans = self.spans(path);
        let at = spans.map_or_else(Pos::start, |s| s.element);
        if let Some(id) = &node.id {
            if let Some(first) = self.ids.get(id.as_str()) {
                let message = format!("duplicate id `{id}` (first used at {first})");
                self.error(at, message);
            } else {
                self.ids.insert(id, at);
            }
        }
        if let Some((catalog, def)) = self.element(node, base, at) {
            self.check(node, catalog, def, spans, at, path);
        }
        for (i, child) in node.children.iter().enumerate() {
            path.push(i);
            self.node(child, base, path);
            path.pop();
        }
    }

    fn element(
        &mut self,
        node: &Node,
        base: &'a Catalog,
        at: Pos,
    ) -> Option<(&'a Catalog, &'a ElementDef)> {
        let catalog = match &node.element.namespace {
            None => base,
            Some(alias) => match self.imports.get(alias.as_str()) {
                Some(catalog) => catalog,
                None => {
                    self.hint(
                        at,
                        format!("unknown library alias `{alias}`"),
                        format!("import it under `use:`, e.g. `{alias}: <library-id>`"),
                    );
                    return None;
                }
            },
        };
        let def = catalog
            .element(&node.element.name)
            .map(|def| (catalog, def));
        if def.is_none() {
            self.error(
                at,
                format!(
                    "`{}` is not an element of `{}`",
                    node.element.name, catalog.library
                ),
            );
        }
        def
    }

    fn check(
        &mut self,
        node: &Node,
        catalog: &Catalog,
        def: &ElementDef,
        spans: Option<&NodeSpans>,
        at: Pos,
        path: &[usize],
    ) {
        let element = &node.element;
        let content_prop = def.content.as_deref().and_then(|c| def.prop(c));
        if let Some(content) = &node.content {
            let pos = spans.and_then(|s| s.content).unwrap_or(at);
            match content_prop {
                None => self.error(pos, format!("`{element}` takes no content")),
                Some(prop) => {
                    if node.props.iter().any(|(name, _)| *name == prop.name) {
                        self.error(
                            pos,
                            format!(
                                "`{element}` sets `{}` both as content and as a prop",
                                prop.name
                            ),
                        );
                    }
                    self.value(element, prop, content, pos);
                }
            }
        }
        for (name, value) in &node.props {
            let pos = spans.and_then(|s| s.props.get(name)).copied().unwrap_or(at);
            match def.prop(name) {
                Some(prop) => self.value(element, prop, value, pos),
                None => {
                    let known: Vec<&str> = def.props.iter().map(|p| p.name.as_str()).collect();
                    self.hint(
                        pos,
                        format!("`{element}` has no prop `{name}`"),
                        known_list("props", &known),
                    );
                }
            }
        }
        for prop in def.props.iter().filter(|p| p.required) {
            let via_content = node.content.is_some() && content_prop == Some(prop);
            if !via_content && !node.props.iter().any(|(name, _)| *name == prop.name) {
                self.error(at, format!("`{element}` requires `{}`", prop.name));
            }
        }
        for event in &node.events {
            if def.event(&event.event).is_none() {
                let pos = spans
                    .and_then(|s| s.events.get(&event.event))
                    .copied()
                    .unwrap_or(at);
                let known: Vec<&str> = def.events.iter().map(|e| e.name.as_str()).collect();
                self.hint(
                    pos,
                    format!("`{element}` has no event `{}`", event.event),
                    known_list("events", &known),
                );
            }
        }
        self.children(node, catalog, def, path);
    }

    /// Library id of the catalog an element reference resolves to.
    fn library_of(&self, element: &ElementRef) -> Option<&'a str> {
        let catalog = match &element.namespace {
            None => self.base,
            Some(alias) => self.imports.get(alias.as_str()).copied(),
        };
        catalog.map(|c| c.library.as_str())
    }

    fn children(&mut self, node: &Node, catalog: &Catalog, def: &ElementDef, path: &[usize]) {
        let map = self.map;
        let child_pos = |i: usize| {
            let mut child = path.to_vec();
            child.push(i);
            map.and_then(|m| m.nodes.get(&child))
                .map_or_else(Pos::start, |s| s.element)
        };
        match &def.children {
            Children::Any => {}
            Children::None => {
                if !node.children.is_empty() {
                    let pos = child_pos(0);
                    self.error(pos, format!("`{}` cannot have children", node.element));
                }
            }
            Children::Only(allowed) => {
                for (i, child) in node.children.iter().enumerate() {
                    let Some(library) = self.library_of(&child.element) else {
                        continue;
                    };
                    // Unqualified entries name the parent's own library; others are `<library-id>.Element`.
                    let ok = allowed.iter().any(|entry| match entry.rsplit_once('.') {
                        Some((lib, name)) => lib == library && name == child.element.name,
                        None => catalog.library == library && *entry == child.element.name,
                    });
                    if !ok {
                        let pos = child_pos(i);
                        self.hint(
                            pos,
                            format!("`{}` cannot contain `{}`", node.element, child.element),
                            format!("allowed children: {}", allowed.join(", ")),
                        );
                    }
                }
            }
        }
    }

    fn value(&mut self, element: &ElementRef, prop: &PropDef, value: &Value, pos: Pos) {
        if nested_two_way(value) {
            self.error(
                pos,
                format!(
                    "`{element}.{}` cannot contain two-way bindings inside a list",
                    prop.name
                ),
            );
        } else if is_two_way(value) && !prop.bindable {
            self.error(
                pos,
                format!(
                    "`{element}.{}` does not accept two-way bindings; use `$path` instead",
                    prop.name
                ),
            );
        } else if !prop.ty.accepts(value) {
            self.error(
                pos,
                format!("`{element}.{}` must be {}", prop.name, prop.ty),
            );
        }
    }
}

fn nested_two_way(value: &Value) -> bool {
    matches!(value, Value::List(items) if items.iter().any(|i| is_two_way(i) || nested_two_way(i)))
}

fn known_list(what: &str, names: &[&str]) -> String {
    if names.is_empty() {
        format!("it has no {what}")
    } else {
        format!("known {what}: {}", names.join(", "))
    }
}
