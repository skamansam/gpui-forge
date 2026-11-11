//! Forge YAML reader: YAML tree -> document model, collecting every error it can.

use std::collections::HashMap;

use super::tree::{self, Scalar, YNode};
use crate::SPEC_VERSION;
use crate::diagnostic::{Diagnostic, Diagnostics, Pos};
use crate::model::{
    Case, Compare, Control, Document, ElementRef, EventBinding, Import, Node, Value,
};
use crate::source_map::{NodeSpans, SourceMap};
use crate::value::Quoting;
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
    let (tree, marked) = match tree::parse(src) {
        Ok(Some(parsed)) => parsed,
        Ok(None) => {
            return Err(Diagnostics(vec![
                Diagnostic::new(Pos::start(), "empty document")
                    .with_hint("start with `forge: \"0.1\"` and a `root:` node"),
            ]));
        }
        Err(d) => return Err(Diagnostics(vec![d])),
    };
    let lines: Vec<&str> = src.lines().collect();
    let mut reader = Reader {
        src,
        diags: Vec::new(),
        path: Vec::new(),
        map: SourceMap::default(),
        merges: tree::merge_names(&marked, &lines),
    };
    // Ref names must be unique — YAML spec shadowing (nearest wins) doesn't
    // apply since refs are addressed by name, not by position.
    let mut seen = std::collections::HashSet::new();
    for (name, at) in tree::anchor_names(&marked, &lines) {
        if !seen.insert(name.clone()) {
            reader.hint(
                at,
                format!("duplicate ref `&{name}`"),
                "ref names must be unique in a document",
            );
        }
    }
    let doc = reader.document(tree);
    reader.diags.sort_by_key(|d| d.pos);
    match doc {
        Some(mut doc) if reader.diags.is_empty() => {
            doc.raw = Some(crate::model::Raw {
                source: src.to_owned(),
                tree: marked,
            });
            Ok((doc, reader.map))
        }
        _ => Err(Diagnostics(reader.diags)),
    }
}

/// Re-parses a `YNode` subtree into a model `Node`; used by the preserving
/// writer to detect unchanged regions.
pub(crate) fn reparse_node(
    src: &str,
    val: YNode,
    merges: std::collections::HashMap<Pos, crate::yaml::tree::MergeInfo>,
) -> Option<Node> {
    Reader {
        src,
        diags: Vec::new(),
        path: Vec::new(),
        map: SourceMap::default(),
        merges,
    }
    .node(val)
}

/// Re-parses a ref block's entry-bag (a node body with no element head) — the
/// preserving writer's baseline for `name: &name` fragments.
pub(crate) fn reparse_fragment(
    src: &str,
    val: YNode,
    merges: std::collections::HashMap<Pos, crate::yaml::tree::MergeInfo>,
) -> Option<Node> {
    let YNode::Map(entries, at) = val else {
        return None;
    };
    let mut with_head = vec![(
        Scalar {
            text: "Ref".into(),
            quoting: Quoting::Plain,
            pos: at,
        },
        YNode::Scalar(Scalar {
            text: String::new(),
            quoting: Quoting::Plain,
            pos: at,
        }),
    )];
    with_head.extend(entries);
    let mut node = reparse_node(src, YNode::Map(with_head, at), merges)?;
    node.element.name.clear();
    Some(node)
}

/// Re-parses a scalar value; `allow_two_way` matches props vs style/content rules.
pub(crate) fn reparse_value(src: &str, val: YNode, allow_two_way: bool) -> Option<Value> {
    Reader {
        src,
        diags: Vec::new(),
        path: Vec::new(),
        map: SourceMap::default(),
        merges: Default::default(),
    }
    .typed(val, allow_two_way)
}

/// Re-parses a scalar as plain text (`forge`, `$schema`, `base`, ...).
pub(crate) fn reparse_text(src: &str, val: YNode) -> Option<String> {
    Reader {
        src,
        diags: Vec::new(),
        path: Vec::new(),
        map: SourceMap::default(),
        merges: Default::default(),
    }
    .text(val)
}

/// Re-parses a `use:` mapping into imports.
pub(crate) fn reparse_imports(src: &str, val: YNode) -> Vec<Import> {
    Reader {
        src,
        diags: Vec::new(),
        path: Vec::new(),
        map: SourceMap::default(),
        merges: Default::default(),
    }
    .imports(val)
}

/// Re-parses a `meta:` mapping into entries.
pub(crate) fn reparse_meta(src: &str, val: YNode) -> Vec<(String, String)> {
    Reader {
        src,
        diags: Vec::new(),
        path: Vec::new(),
        map: SourceMap::default(),
        merges: Default::default(),
    }
    .meta(val)
}

struct Reader<'a> {
    src: &'a str,
    diags: Vec<Diagnostic>,
    /// Path steps from the root to the node being read.
    path: Vec<crate::model::PathStep>,
    map: SourceMap,
    /// `<<` merge info by map position (`<<: *name` / `<<: [*a, *b]`).
    merges: std::collections::HashMap<Pos, crate::yaml::tree::MergeInfo>,
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
        let (mut imports, mut meta, mut state, mut dialogs) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut refs = Vec::new();
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
                "base" => {
                    self.map.base = Some(val.pos());
                    base = self.text(val);
                }
                "use" => imports = self.imports(val),
                "meta" => meta = self.meta(val),
                "state" => state = self.meta(val),
                "root" => {
                    saw_root = true;
                    root = self.node(val);
                }
                other => {
                    // `Dialog#name:` declares a dialog; container values are
                    // fragment slots for `&anchor` declarations; unknown scalar
                    // keys are likely typos.
                    let is_dialog = key.text == "Dialog"
                        || key.text.starts_with("Dialog#")
                        || key.text.starts_with("Dialog.");
                    if is_dialog {
                        if let Some(node) = self.dialog_node(key, val) {
                            dialogs.push(node);
                        }
                    } else if let YNode::Map(entries, at) = &val
                        && let Some(name) = self.anchor_at(*at)
                    {
                        // `name: &name` — a ref block: a node's entry-bag with
                        // no element head. Parse it as a node body so merges
                        // splice `children:`/`style:`/props correctly.
                        let mut with_head = vec![(
                            Scalar {
                                text: "Ref".into(),
                                quoting: Quoting::Plain,
                                pos: *at,
                            },
                            YNode::Scalar(Scalar {
                                text: String::new(),
                                quoting: Quoting::Plain,
                                pos: *at,
                            }),
                        )];
                        with_head.extend(entries.iter().cloned());
                        if let Some(mut node) = self.node(YNode::Map(with_head, *at)) {
                            node.element.name.clear();
                            refs.push((name, node));
                        }
                    } else if matches!(val, YNode::Scalar(_)) {
                        self.hint(
                            key.pos,
                            format!("unknown top-level key `{other}`"),
                            "expected `$schema`, `forge`, `base`, `use`, `meta` or `root`",
                        );
                    }
                }
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
            state,
            dialogs,
            refs,
            root: root?,
            raw: None,
        })
    }

    /// Parses a top-level `Dialog#name:` declaration — the element key lives on
    /// the key itself, so we synthesize `{Dialog#name: null, ..body}` for `node()`.
    fn dialog_node(&mut self, key: Scalar, val: YNode) -> Option<Node> {
        let YNode::Map(body, at) = val else {
            self.error(key.pos, "`Dialog` must be a mapping");
            return None;
        };
        let pos = key.pos;
        let mut entries = vec![(
            key,
            YNode::Scalar(Scalar {
                text: String::new(),
                quoting: Quoting::Plain,
                pos,
            }),
        )];
        entries.extend(body);
        self.node(YNode::Map(entries, at))
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

    /// The `&name` anchor at a node's position, if the value carries one —
    /// the anchor token sits at or just before the node's own span (saphyr
    /// starts anchored nodes' spans at the name, after `&`).
    fn anchor_at(&self, at: Pos) -> Option<String> {
        let line = self.src.lines().nth(at.line.saturating_sub(1))?;
        let text: String = line.chars().skip(at.col).collect();
        let text = text.trim_start();
        let rest = if let Some(rest) = text.strip_prefix('&') {
            rest
        } else if at.col > 0 && line.chars().nth(at.col - 1) == Some('&') {
            text
        } else {
            return None;
        };
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect();
        (!name.is_empty()).then_some(name)
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
        if let Some(merge) = self.merges.get(&at) {
            node.refname = merge.names.first().cloned();
            node.merged = merge.keys.clone();
        }
        let mut spans = NodeSpans::default();
        let mut element: Option<ElementRef> = None;
        let mut head: Option<(Scalar, ControlHead)> = None;
        let mut ctl = Ctl::default();
        for (key, val) in entries {
            let name = key.text.as_str();
            if name == "style" {
                node.style = self.style(val, &mut spans);
            } else if name == "children" {
                node.children = self.children(val);
            } else if let Some(event) = name.strip_prefix("on ") {
                if let Some(binding) = self.event(event.trim(), &key, val) {
                    spans.events.insert(binding.event.clone(), key.pos);
                    node.events.push(binding);
                }
            } else if matches!(name, "if" | "for" | "with" | "match") {
                if let Some((first, _)) = &head {
                    self.hint(
                        key.pos,
                        format!("node has two control keys (`{}`, `{name}`)", first.text),
                        "only one of `if`, `for`, `with` or `match` per node",
                    );
                    continue;
                }
                let content_pos = val.pos();
                if let Some(value) = self.control_value(name, &key, val) {
                    let kind = match name {
                        "if" => ControlHead::If(value),
                        "for" => ControlHead::For(value),
                        "with" => ControlHead::With(value),
                        _ => ControlHead::Match(value),
                    };
                    spans.element = key.pos;
                    spans.content = Some(content_pos);
                    head = Some((key, kind));
                }
            } else if name == "else" {
                ctl.otherwise = Some((
                    key,
                    self.branch_children(val, crate::model::PathStep::Else),
                ));
            } else if name == "case" {
                self.hint(
                    key.pos,
                    "`case` needs a value like `case gpui:`",
                    "`case` arms belong to `match`",
                );
            } else if let Some(operand) = name.strip_prefix("case ") {
                match value::parse_arg(operand.trim()) {
                    Ok(operand) => {
                        let case = ctl.cases.len();
                        let children =
                            self.branch_children(val, move |i| {
                                crate::model::PathStep::Case(case, i)
                            });
                        ctl.cases.push((key, operand, children));
                    }
                    Err(e) => self.error(key.pos, format!("invalid `case` operand: {e}")),
                }
            } else if name == "is" || name == "not" {
                if let Some(v) = self.value(val, &key, false) {
                    let slot = if name == "is" {
                        &mut ctl.is
                    } else {
                        &mut ctl.not
                    };
                    *slot = Some((key, v));
                }
            } else if name == "key" {
                // `key: item.id` — member path rooted in the `each` var.
                let text = self.text(val).unwrap_or_default();
                match crate::value::parse_path(&text) {
                    Ok(path) => ctl.key = Some((key, path)),
                    Err(e) => self.hint(key.pos, format!("invalid `key` path `{text}`"), &e),
                }
            } else if matches!(name, "each" | "index" | "as") {
                if let Some(text) = self.ident_arg(&key, val) {
                    let slot = match name {
                        "each" => &mut ctl.each,
                        "index" => &mut ctl.index,
                        _ => &mut ctl.as_,
                    };
                    *slot = Some((key, text));
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
        if let Some((key, head)) = head {
            let sentinel = head.sentinel_name();
            node.control = self.assemble(&key, head, ctl, element.is_some(), &node);
            if element.is_none() {
                // A bare control block — the element slot carries a `forge.X`
                // sentinel that renderers never reach element dispatch for.
                element = Some(ElementRef::control_sentinel(sentinel));
            }
        } else {
            self.stray_control(&ctl);
        }
        let Some(element) = element else {
            self.hint(
                at,
                "node has no element key",
                "start the node with an element, e.g. `Button:` or `VStack#root:`",
            );
            return None;
        };
        if element.is_control_sentinel() {
            self.block_only(&node, &spans, at);
        }
        node.element = element;
        self.map.nodes.insert(self.path.clone(), spans);
        Some(node)
    }

    /// The value a control head key carries — `if: <cond>`, `for: <list>`,
    /// `with: <binding>` or `match: <binding>`.
    fn control_value(&mut self, kind: &str, key: &Scalar, val: YNode) -> Option<Value> {
        let value = self.value(val, key, false)?;
        let ok = match kind {
            "for" => matches!(value, Value::Binding { .. } | Value::List(_)),
            "with" | "match" => matches!(value, Value::Binding { .. }),
            _ => true,
        };
        if !ok {
            self.hint(
                key.pos,
                format!("`{kind}` needs a binding like `$items`"),
                "control keys take a one-way `$path` binding, not `bind`",
            );
            return None;
        }
        Some(value)
    }

    /// A scoped-var name for `each:`/`index:`/`as:` — a plain identifier.
    fn ident_arg(&mut self, key: &Scalar, val: YNode) -> Option<String> {
        let text = self.text(val)?;
        if is_ident(&text) {
            Some(text)
        } else {
            self.hint(
                key.pos,
                format!("`{}` must be an identifier like `item`", key.text),
                "it names the variable children bind as `$item`",
            );
            None
        }
    }

    /// A node list under `else:` or `case <v>:` — addressable via `e`/`c`
    /// path steps (see `model::PathStep`).
    fn branch_children(
        &mut self,
        val: YNode,
        step: impl Fn(usize) -> crate::model::PathStep,
    ) -> Vec<Node> {
        match val {
            YNode::Seq(items, _) => {
                let mut nodes = Vec::new();
                for item in items {
                    self.path.push(step(nodes.len()));
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
                    "`else` and `case` arms must be lists of nodes",
                    "start each child with `- `",
                );
                Vec::new()
            }
        }
    }

    /// Builds the node's `Control` from its control key and pending sub-keys;
    /// misplaced sub-keys produce targeted errors.
    fn assemble(
        &mut self,
        head: &Scalar,
        kind: ControlHead,
        ctl: Ctl,
        has_element: bool,
        node: &Node,
    ) -> Option<Control> {
        let mut ctl = ctl;
        match kind {
            ControlHead::If(when) => {
                self.stray(&ctl.each, "for");
                self.stray(&ctl.index, "for");
                self.stray(&ctl.as_, "with");
                self.stray_cases(&ctl.cases);
                let test = match (ctl.is, ctl.not) {
                    (Some(_), Some((key, _))) => {
                        self.hint(
                            key.pos,
                            "node has both `is` and `not`",
                            "pick one — `is: x` renders on equality, `not: x` on inequality",
                        );
                        None
                    }
                    (Some((_, operand)), None) => Some(Compare::Is(operand)),
                    (None, Some((_, operand))) => Some(Compare::Not(operand)),
                    (None, None) => None,
                };
                Some(Control::If {
                    when,
                    test,
                    otherwise: ctl
                        .otherwise
                        .take()
                        .map(|(_, children)| children)
                        .unwrap_or_default(),
                })
            }
            ControlHead::For(source) => {
                self.stray(&ctl.is, "if");
                self.stray(&ctl.not, "if");
                self.stray(&ctl.as_, "with");
                self.stray_cases(&ctl.cases);
                let Some((_, each)) = ctl.each else {
                    self.hint(
                        head.pos,
                        "`for` needs `each: <name>`",
                        "name the scoped var children bind, e.g. `each: item`",
                    );
                    return None;
                };
                if let Some((key, index)) = &ctl.index
                    && index == &each
                {
                    self.error(key.pos, "`each` and `index` must name different vars");
                }
                // `key: item.id` must be a member path rooted in the `each` var.
                if let Some((k, path)) = &ctl.key
                    && !path.0.first().is_some_and(|s| {
                        matches!(s, crate::model::PathSegment::Field(field) if field == &each)
                    })
                {
                    self.hint(
                        k.pos,
                        format!("`key` must start with the `each` var `{each}`"),
                        "identify items by a member path, e.g. `key: item.id`",
                    );
                }
                Some(Control::For {
                    source,
                    each,
                    index: ctl.index.take().map(|(_, name)| name),
                    key: ctl.key.take().map(|(_, path)| path),
                    otherwise: ctl
                        .otherwise
                        .take()
                        .map(|(_, children)| children)
                        .unwrap_or_default(),
                })
            }
            ControlHead::With(source) => {
                self.stray(&ctl.is, "if");
                self.stray(&ctl.not, "if");
                self.stray(&ctl.each, "for");
                self.stray(&ctl.index, "for");
                self.stray_cases(&ctl.cases);
                if let Some((key, _)) = &ctl.otherwise {
                    self.hint(
                        key.pos,
                        "`else` needs `if`, `for` or `match`",
                        "`with` always scopes its value — test it with `if` inside",
                    );
                }
                let Some((_, name)) = ctl.as_ else {
                    self.hint(
                        head.pos,
                        "`with` needs `as: <name>`",
                        "name the scoped var children bind, e.g. `as: tab`",
                    );
                    return None;
                };
                Some(Control::With { source, name })
            }
            ControlHead::Match(source) => {
                self.stray(&ctl.is, "if");
                self.stray(&ctl.not, "if");
                self.stray(&ctl.each, "for");
                self.stray(&ctl.index, "for");
                self.stray(&ctl.as_, "with");
                if has_element {
                    self.hint(
                        head.pos,
                        "`match` cannot modify an element",
                        "wrap the element in a bare `match` block",
                    );
                }
                if !node.children.is_empty() {
                    self.hint(
                        head.pos,
                        "`match` uses `case` arms, not `children:`",
                        "write `case <value>:` arms, e.g. `case gpui:`",
                    );
                }
                if ctl.cases.is_empty() {
                    self.hint(
                        head.pos,
                        "`match` needs at least one `case`",
                        "write `case <value>:` arms, e.g. `case gpui:`",
                    );
                }
                Some(Control::Match {
                    source,
                    cases: ctl
                        .cases
                        .into_iter()
                        .map(|(_, value, children)| Case { value, children })
                        .collect(),
                    otherwise: ctl
                        .otherwise
                        .take()
                        .map(|(_, children)| children)
                        .unwrap_or_default(),
                })
            }
        }
    }

    /// Errors for control sub-keys found without their control head.
    fn stray_control(&mut self, ctl: &Ctl) {
        if let Some((key, _)) = &ctl.otherwise {
            self.hint(
                key.pos,
                "`else` needs `if`, `for` or `match`",
                "control blocks look like `- if: $cond`, `- for: $items` or `- match: $value`",
            );
        }
        self.stray_cases(&ctl.cases);
        self.stray(&ctl.is, "if");
        self.stray(&ctl.not, "if");
        self.stray(&ctl.each, "for");
        self.stray(&ctl.index, "for");
        self.stray(&ctl.as_, "with");
    }

    fn stray(&mut self, slot: &Option<(Scalar, impl std::fmt::Debug)>, needs: &str) {
        if let Some((key, _)) = slot {
            self.hint(
                key.pos,
                format!("`{}` needs `{needs}`", key.text),
                "control sub-keys only apply to their control key",
            );
        }
    }

    fn stray_cases(&mut self, cases: &[(Scalar, Value, Vec<Node>)]) {
        for (key, _, _) in cases {
            self.hint(
                key.pos,
                format!("`{}` needs `match`", key.text),
                "match blocks look like `- match: $value` with `case <v>:` arms",
            );
        }
    }

    /// Bare control blocks carry no element — style, props and events have
    /// nothing to attach to.
    fn block_only(&mut self, node: &Node, spans: &NodeSpans, at: Pos) {
        if let Some(pos) = spans.style.values().next() {
            self.hint(
                *pos,
                "`style` needs an element",
                "control blocks are transparent — style a child or a wrapper element",
            );
        }
        for (name, _) in &node.props {
            let pos = spans.props.get(name).copied().unwrap_or(at);
            self.hint(
                pos,
                format!("prop `{name}` needs an element"),
                "control blocks are transparent — put props on a child element",
            );
        }
        for event in &node.events {
            let pos = spans.events.get(&event.event).copied().unwrap_or(at);
            self.hint(
                pos,
                format!("`on {}` needs an element", event.event),
                "control blocks emit no events — put the handler on a child element",
            );
        }
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

    fn style(&mut self, val: YNode, spans: &mut NodeSpans) -> Vec<(String, Value)> {
        let mut style = Vec::new();
        for (key, val) in self.mapping(val, "`style`") {
            if !is_prop_key(&key.text) {
                self.error(
                    key.pos,
                    format!("`{}` is not a valid style property", key.text),
                );
            } else if let Some(value) = self.value(val, &key, false) {
                spans.style.insert(key.text.clone(), key.pos);
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
                    self.path.push(crate::model::PathStep::Child(nodes.len()));
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
            YNode::Map(items, _) => {
                let len = items.len();
                let entries: Vec<(String, Value)> = items
                    .into_iter()
                    .filter_map(|(key, val)| Some((key.text, self.typed(val, allow_two_way)?)))
                    .collect();
                (entries.len() == len).then_some(Value::Map(entries))
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

/// A control head key's parsed value — `if: <cond>` holds the condition,
/// `for:`/`with:`/`match:` their source binding.
enum ControlHead {
    If(Value),
    For(Value),
    With(Value),
    Match(Value),
}

impl ControlHead {
    fn sentinel_name(&self) -> &'static str {
        match self {
            ControlHead::If(_) => "If",
            ControlHead::For(_) => "For",
            ControlHead::With(_) => "With",
            ControlHead::Match(_) => "Match",
        }
    }
}

/// Control sub-keys collected while scanning a node's entries, consumed by
/// [`Reader::assemble`].
#[derive(Default)]
struct Ctl {
    is: Option<(Scalar, Value)>,
    not: Option<(Scalar, Value)>,
    each: Option<(Scalar, String)>,
    index: Option<(Scalar, String)>,
    key: Option<(Scalar, crate::model::Path)>,
    as_: Option<(Scalar, String)>,
    otherwise: Option<(Scalar, Vec<Node>)>,
    cases: Vec<(Scalar, Value, Vec<Node>)>,
}

/// Parses `[alias.]Element[#id]` or `[alias.]Element .id`.
pub(crate) fn element_key(key: &str) -> Option<(ElementRef, Option<String>)> {
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
