//! The canonical, format-independent document model (`docs/format/document-model.md`).

use std::fmt;

/// Ordered key/value pairs. Order is preserved so hand-edited files diff cleanly.
pub type Entries = Vec<(String, Value)>;

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema, Clone, Debug, PartialEq)]
pub struct Document {
    /// JSON Schema URL for editors (`$schema`).
    pub schema: Option<String>,
    pub version: String,
    /// Base component system; inherited from `forge.toml` when absent.
    pub base: Option<String>,
    pub imports: Vec<Import>,
    pub meta: Vec<(String, String)>,
    /// Top-level `state:` — declared state vars: `path -> type` (e.g.
    /// `studio.theme: string`). `forge.Vars` uses the type to pick the
    /// editor widget; `forge check` validates bindings to undeclared paths.
    pub state: Vec<(String, String)>,
    /// Top-level `Dialog#name:` declarations, opened by `open_dialog(name)`.
    pub dialogs: Vec<Node>,
    /// Ref blocks — `name: &name` fragments merged into nodes via `<<: *name`.
    /// The fragment is a node's entry-bag (`children:`, props, `style:`…) with
    /// no element head — `element.name` is `""` by convention.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<(String, Node)>,
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

#[derive(
    serde::Serialize, serde::Deserialize, schemars::JsonSchema, Clone, Debug, PartialEq, Eq,
)]
pub struct Import {
    pub alias: String,
    pub library: String,
    pub version: Option<String>,
}

/// One step in a node path — into `children` (`0.2.1`), the `else:` list
/// (`0.2.e0`), or a `match` `case` arm (`0.2.c1.0` = case 1, child 0).
/// `0.2.e` and `0.2.c1` (bare, un-indexed) address the branch *lists* for
/// insert ops — they are not nodes and never resolve via `node_at`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PathStep {
    /// `children[i]`.
    Child(usize),
    /// `else:` child `i` — `e{i}`.
    Else(usize),
    /// `case[c]`'s child `i` — `c{c}.{i}`.
    Case(usize, usize),
    /// The `else:` list itself — `e`.
    ElseList,
    /// `case[c]`'s child list itself — `c{c}`.
    CaseList(usize),
}

impl std::fmt::Display for PathStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathStep::Child(i) => write!(f, "{i}"),
            PathStep::Else(i) => write!(f, "e{i}"),
            PathStep::Case(c, i) => write!(f, "c{c}.{i}"),
            PathStep::ElseList => f.write_str("e"),
            PathStep::CaseList(c) => write!(f, "c{c}"),
        }
    }
}

/// Parses a dotted node path — `0.2.1`, `0.2.e0`, `0.2.c1.0`, `0.2.e`, `0.2.c1`.
/// `c{n}` becomes a `Case` step only when followed by a plain index; otherwise
/// it is the `CaseList` container (valid only at the end of the path).
pub fn path_steps(path: &str) -> Option<Vec<PathStep>> {
    let mut out = Vec::new();
    let mut segments = path.split('.').peekable();
    while let Some(seg) = segments.next() {
        if let Some(idx) = seg.strip_prefix('e') {
            if idx.is_empty() {
                out.push(PathStep::ElseList);
            } else {
                out.push(PathStep::Else(idx.parse().ok()?));
            }
        } else if let Some(idx) = seg.strip_prefix('c') {
            let case: usize = idx.parse().ok()?;
            match segments.peek() {
                Some(next) if next.bytes().all(|b| b.is_ascii_digit()) => {
                    let child: usize = segments.next()?.parse().ok()?;
                    out.push(PathStep::Case(case, child));
                }
                None => out.push(PathStep::CaseList(case)),
                _ => return None,
            }
        } else {
            out.push(PathStep::Child(seg.parse().ok()?));
        }
    }
    Some(out)
}

/// Formats a step path back to its string form (inverse of `path_steps`).
pub fn path_string(steps: &[PathStep]) -> String {
    steps
        .iter()
        .map(PathStep::to_string)
        .collect::<Vec<_>>()
        .join(".")
}

impl Document {
    /// Finds a node by index path (`"0.2.1"` → `root.children[2].children[1]`;
    /// `"0"` is the root; `e`/`c` steps reach `else:`/`case` branch children).
    /// This is the same path shape `forge.Outline` reports for `select`.
    pub fn node_at(&self, path: &str) -> Option<&Node> {
        let mut segments = path_steps(path)?.into_iter();
        if segments.next()? != PathStep::Child(0) {
            return None;
        }
        let mut node = &self.root;
        for step in segments {
            node = node.step_child(step)?;
        }
        Some(node)
    }

    /// Mutable version of [`Document::node_at`].
    pub fn node_at_mut(&mut self, path: &str) -> Option<&mut Node> {
        let mut segments = path_steps(path)?.into_iter();
        if segments.next()? != PathStep::Child(0) {
            return None;
        }
        let mut node = &mut self.root;
        for step in segments {
            node = node.step_child_mut(step)?;
        }
        Some(node)
    }

    /// The child list `path` addresses — a node's `children`, or a branch
    /// list when the path ends in `e`/`c{n}` (`0.2.e` → `else:`, `0.2.c0` →
    /// case 0's children).
    fn list_at_mut(&mut self, path: &str) -> Option<&mut Vec<Node>> {
        let mut segments = path_steps(path)?.into_iter().peekable();
        if segments.next()? != PathStep::Child(0) {
            return None;
        }
        let mut node = &mut self.root;
        while let Some(step) = segments.next() {
            if segments.peek().is_none() {
                return match step {
                    PathStep::ElseList => node.otherwise_mut(),
                    PathStep::CaseList(c) => node.case_mut(c),
                    step => node.step_child_mut(step).map(|n| &mut n.children),
                };
            }
            node = node.step_child_mut(step)?;
        }
        Some(&mut node.children)
    }

    /// Sets or replaces a prop on the node at `path`; `None` removes it. Returns whether
    /// the path resolved — a `false` return means no change was made.
    pub fn set_prop(&mut self, path: &str, name: &str, value: Option<Value>) -> bool {
        let Some(node) = self.node_at_mut(path) else {
            return false;
        };
        match value {
            Some(value) => match node.props.iter_mut().find(|(k, _)| k == name) {
                Some((_, slot)) => *slot = value,
                None => node.props.push((name.to_owned(), value)),
            },
            None => node.props.retain(|(k, _)| k != name),
        }
        self.sync_refs();
        true
    }

    /// Sets or clears the `<<: *name` merge on the node at `path`. Assigning a
    /// new name gives the node a ref block covering its own facets. Clearing
    /// flattens merged entries back into the node on write.
    pub fn set_refname(&mut self, path: &str, refname: Option<String>) -> bool {
        let Some(node) = self.node_at_mut(path) else {
            return false;
        };
        node.refname = refname;
        self.sync_refs();
        true
    }

    /// Sets or replaces a style entry on the node at `path`; `None` removes it.
    /// Returns whether the path resolved.
    pub fn set_style(&mut self, path: &str, name: &str, value: Option<Value>) -> bool {
        let Some(node) = self.node_at_mut(path) else {
            return false;
        };
        match value {
            Some(value) => match node.style.iter_mut().find(|(k, _)| k == name) {
                Some((_, slot)) => *slot = value,
                None => node.style.push((name.to_owned(), value)),
            },
            None => node.style.retain(|(k, _)| k != name),
        }
        self.sync_refs();
        true
    }

    /// Splits `path` into parent steps and the final step — `None` for the
    /// root (no parent) or for `e`/`c{n}` list paths (they have no final
    /// *node* step).
    fn split_last(path: &str) -> Option<(Vec<PathStep>, PathStep)> {
        let mut steps = path_steps(path)?;
        let last = steps.pop()?;
        match last {
            PathStep::ElseList | PathStep::CaseList(_) => None,
            last => Some((steps, last)),
        }
    }

    /// Inserts `node` into the list `path` addresses (`index` defaults to the
    /// end) — `0.2` → that node's `children`, `0.2.e` → its `else:` list,
    /// `0.2.c0` → case 0's children. Returns whether the path resolved.
    pub fn insert_node(&mut self, path: &str, node: Node, index: Option<usize>) -> bool {
        let Some(list) = self.list_at_mut(path) else {
            return false;
        };
        let index = index.unwrap_or(list.len()).min(list.len());
        list.insert(index, node);
        self.sync_refs();
        true
    }

    /// Removes the node at `path` (the root cannot be removed). Returns the removed node.
    pub fn remove_node(&mut self, path: &str) -> Option<Node> {
        let (parent_steps, step) = Self::split_last(path)?;
        let parent = self.node_at_mut(&path_string(&parent_steps))?;
        let (list, index) = parent.step_list_mut(step)?;
        let removed = (index < list.len()).then(|| list.remove(index));
        if removed.is_some() {
            self.sync_refs();
        }
        removed
    }

    /// Inserts `node` as the sibling immediately after `after` — into its
    /// parent's list at `index + 1`. Returns the inserted node's path.
    pub fn insert_after(&mut self, after: &str, node: Node) -> Option<String> {
        let (parent_steps, step) = Self::split_last(after)?;
        let parent_path = path_string(&parent_steps);
        let parent = self.node_at_mut(&parent_path)?;
        let (list, index) = parent.step_list_mut(step)?;
        let at = (index + 1).min(list.len());
        list.insert(at, node);
        self.sync_refs();
        Some(format!("{parent_path}.{}", step.at_index(at)))
    }

    /// Moves the node at `path` to the position immediately after `after`,
    /// keeping siblings in order when moving within the same parent.
    pub fn move_after(&mut self, path: &str, after: &str) -> Option<String> {
        if path == "0" || path == after || after.starts_with(&format!("{path}.")) {
            return None;
        }
        let (parent_steps, step) = Self::split_last(after)?;
        let parent_path = path_string(&parent_steps);
        let (src_steps, src_step) = Self::split_last(path)?;
        let src_index = src_step.index()?;
        let index = step.index()?;
        let same_list = src_steps == parent_steps && src_step.list_kind() == step.list_kind();
        let node = self.remove_node(path)?;
        // A sibling removed above `after` shifts its slot left by one.
        let at = if same_list && src_index < index {
            index
        } else {
            index + 1
        };
        let parent = self.node_at_mut(&parent_path)?;
        let (list, _) = parent.step_list_mut(step)?;
        let at = at.min(list.len());
        list.insert(at, node);
        self.sync_refs();
        Some(format!("{parent_path}.{}", step.at_index(at)))
    }

    /// Keeps `refs` and merge sites consistent after edits: each `*name` node's
    /// merged facets flow into the `name:` ref block, then the fragment is
    /// applied back to every merge site (a shared ref means shared content).
    /// Newly-assigned refnames get a ref block covering the node's own facets.
    pub fn sync_refs(&mut self) {
        let mut acc: Vec<(String, Node)> = Vec::new();
        fn visit(node: &mut Node, acc: &mut Vec<(String, Node)>) {
            if let Some(name) = node.refname.clone() {
                // Fresh refname — the node's own facets become the ref block.
                if node.merged.is_empty() {
                    node.merged = node.facet_keys();
                }
                let merged = node.merged_facets();
                match acc.iter_mut().find(|(n, _)| n == &name) {
                    Some((_, acc)) => acc.apply_facets(&node.merged, &merged),
                    None => acc.push((name, merged)),
                }
            }
            for child in &mut node.children {
                visit(child, acc);
            }
            match &mut node.control {
                Some(Control::Match {
                    cases, otherwise, ..
                }) => {
                    for case in cases {
                        for child in &mut case.children {
                            visit(child, acc);
                        }
                    }
                    for child in otherwise {
                        visit(child, acc);
                    }
                }
                Some(
                    Control::If { otherwise, .. }
                    | Control::For { otherwise, .. },
                ) => {
                    for child in otherwise {
                        visit(child, acc);
                    }
                }
                _ => {}
            }
        }
        visit(&mut self.root, &mut acc);
        for dialog in &mut self.dialogs {
            visit(dialog, &mut acc);
        }
        for (_, fragment) in &mut self.refs {
            visit(fragment, &mut acc);
        }
        for (name, fragment) in &acc {
            match self.refs.iter_mut().find(|(n, _)| n == name) {
                Some((_, slot)) => *slot = fragment.clone(),
                None => self.refs.push((name.clone(), fragment.clone())),
            }
        }
        let apply = |node: &mut Node, acc: &Vec<(String, Node)>| {
            let mut visit = |node: &mut Node| {
                if let Some((_, fragment)) =
                    node.refname.as_ref().and_then(|n| acc.iter().find(|(k, _)| k == n))
                {
                    let fragment = fragment.clone();
                    node.apply_facets(&node.merged.clone(), &fragment);
                }
            };
            fn deep(node: &mut Node, visit: &mut impl FnMut(&mut Node)) {
                visit(node);
                for child in &mut node.children {
                    deep(child, visit);
                }
                match &mut node.control {
                    Some(Control::Match {
                        cases, otherwise, ..
                    }) => {
                        for case in cases {
                            for child in &mut case.children {
                                deep(child, visit);
                            }
                        }
                        for child in otherwise {
                            deep(child, visit);
                        }
                    }
                    Some(Control::If { otherwise, .. } | Control::For { otherwise, .. }) => {
                        for child in otherwise {
                            deep(child, visit);
                        }
                    }
                    _ => {}
                }
            }
            deep(node, &mut visit);
        };
        apply(&mut self.root, &acc);
        for dialog in &mut self.dialogs {
            apply(dialog, &acc);
        }
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

#[derive(
    serde::Serialize, serde::Deserialize, schemars::JsonSchema, Clone, Debug, Default, PartialEq,
)]
pub struct Node {
    pub element: ElementRef,
    pub id: Option<String>,
    /// Positional value. The catalog maps it to the element's primary prop.
    pub content: Option<Value>,
    pub props: Entries,
    pub style: Entries,
    pub events: Vec<EventBinding>,
    pub children: Vec<Node>,
    /// Control flow — `if`, `for`, `with` or `match`. On a bare block
    /// (`- if: $cond`) it is the node's whole meaning and `element` is a
    /// `forge.X` sentinel; on an element it modifies that element.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control: Option<Control>,
    /// Name of the `&anchor` block this node's entries merge from
    /// (`<<: *name` in YAML). Written through the preserving writer — edits
    /// splice into the `name:` ref block, not the merge site.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refname: Option<String>,
    /// Entry keys the `<<` merges contributed — the writer skips these so
    /// merged facets aren't re-emitted after the `<<` line. Read-time
    /// bookkeeping, not part of the serialized model.
    #[serde(skip)]
    pub merged: Vec<String>,
}

impl Node {
    /// True for bare control blocks — `element` is a `forge.X` sentinel that
    /// never renders; `control` is the node's whole meaning.
    pub fn is_block(&self) -> bool {
        self.control.is_some() && self.element.is_control_sentinel()
    }

    /// The facets a `<<` merge contributed — `ref` blocks hold exactly these
    /// (a node's entry-bag with no element head).
    fn merged_facets(&self) -> Node {
        let has = |key: &str| self.merged.iter().any(|k| k == key);
        Node {
            props: self
                .props
                .iter()
                .filter(|(k, _)| has(k))
                .cloned()
                .collect(),
            style: if has("style") { self.style.clone() } else { vec![] },
            events: self
                .events
                .iter()
                .filter(|e| has(&format!("on {}", e.event)))
                .cloned()
                .collect(),
            children: if has("children") {
                self.children.clone()
            } else {
                vec![]
            },
            control: self.control.clone().filter(|_| {
                has("if") || has("for") || has("with") || has("match")
            }),
            ..Default::default()
        }
    }

    /// Overwrites `keys`-marked facets with `other`'s — `other` carries the
    /// ref block's (or a sibling merge site's) latest facet values.
    fn apply_facets(&mut self, keys: &[String], other: &Node) {
        let has = |key: &str| keys.iter().any(|k| k == key);
        self.props.retain(|(k, _)| !has(k));
        self.props.extend(other.props.iter().cloned());
        if has("style") {
            self.style = other.style.clone();
        }
        self.events
            .retain(|e| !has(&format!("on {}", e.event)));
        self.events.extend(other.events.iter().cloned());
        if has("children") {
            self.children = other.children.clone();
        }
        if (has("if") || has("for") || has("with") || has("match")) && other.control.is_some() {
            self.control = other.control.clone();
        }
    }

    /// Entry keys this node would contribute to a `<<` merge — used when a
    /// `refname` is newly assigned and the whole node becomes the ref block.
    fn facet_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.props.iter().map(|(k, _)| k.clone()).collect();
        if !self.style.is_empty() {
            keys.push("style".into());
        }
        keys.extend(self.events.iter().map(|e| format!("on {}", e.event)));
        if !self.children.is_empty() {
            keys.push("children".into());
        }
        if let Some(control) = &self.control {
            keys.push(control.keyword().to_owned());
        }
        keys
    }

    /// The child reached by `step` — `children`, `else:` or a `case` arm.
    fn step_child(&self, step: PathStep) -> Option<&Node> {
        match step {
            PathStep::Child(i) => self.children.get(i),
            PathStep::Else(i) => self.control.as_ref()?.otherwise().get(i),
            PathStep::Case(c, i) => match &self.control {
                Some(Control::Match { cases, .. }) => cases.get(c)?.children.get(i),
                _ => None,
            },
            PathStep::ElseList | PathStep::CaseList(_) => None,
        }
    }

    fn step_child_mut(&mut self, step: PathStep) -> Option<&mut Node> {
        match step {
            PathStep::Child(i) => self.children.get_mut(i),
            PathStep::Else(i) => self.otherwise_mut()?.get_mut(i),
            PathStep::Case(c, i) => self.case_mut(c)?.get_mut(i),
            PathStep::ElseList | PathStep::CaseList(_) => None,
        }
    }

    /// The child list `step` indexes into, plus the step's index — for
    /// sibling ops (insert/remove/move) on branch children.
    fn step_list_mut(&mut self, step: PathStep) -> Option<(&mut Vec<Node>, usize)> {
        let index = step.index()?;
        let list = match step {
            PathStep::Child(_) => &mut self.children,
            PathStep::Else(_) => self.otherwise_mut()?,
            PathStep::Case(c, _) => self.case_mut(c)?,
            PathStep::ElseList | PathStep::CaseList(_) => return None,
        };
        Some((list, index))
    }

    /// The `else:` child list, when the control has one.
    fn otherwise_mut(&mut self) -> Option<&mut Vec<Node>> {
        match &mut self.control {
            Some(
                Control::If { otherwise, .. }
                | Control::For { otherwise, .. }
                | Control::Match { otherwise, .. },
            ) => Some(otherwise),
            _ => None,
        }
    }

    /// `case[c]`'s child list, on a `match` node.
    fn case_mut(&mut self, case: usize) -> Option<&mut Vec<Node>> {
        match &mut self.control {
            Some(Control::Match { cases, .. }) => Some(&mut cases.get_mut(case)?.children),
            _ => None,
        }
    }
}

impl PathStep {
    /// The index within its list — `None` for bare `e`/`c{n}` list steps.
    fn index(self) -> Option<usize> {
        match self {
            PathStep::Child(i) | PathStep::Else(i) | PathStep::Case(_, i) => Some(i),
            PathStep::ElseList | PathStep::CaseList(_) => None,
        }
    }

    /// Which sibling list a step belongs to — `same_list` in `move_after`.
    fn list_kind(self) -> u8 {
        match self {
            PathStep::Child(_) => 0,
            PathStep::Else(_) => 1,
            PathStep::Case(c, _) => 2 + (c as u8).min(200),
            PathStep::ElseList | PathStep::CaseList(_) => 255,
        }
    }

    /// The step for index `at` in this step's list — keeps `e`/`c` markers.
    fn at_index(self, at: usize) -> PathStep {
        match self {
            PathStep::Child(_) => PathStep::Child(at),
            PathStep::Else(_) | PathStep::ElseList => PathStep::Else(at),
            PathStep::Case(c, _) | PathStep::CaseList(c) => PathStep::Case(c, at),
        }
    }
}

/// Control-flow modifier — `if`, `for`, `with` or `match` (`docs/format/forge-yaml.md`).
///
/// `children` always holds the primary branch (the then-body, loop body or
/// scoped body); `match` uses `cases` instead and carries no `children`.
#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema, Clone, Debug, PartialEq)]
pub enum Control {
    /// `if:` — renders `children` (or the element) when `when` is truthy,
    /// or compares equal/not-equal to the `is:`/`not:` operand; `otherwise`
    /// (`else:`) renders when the condition fails.
    If {
        when: Value,
        test: Option<Compare>,
        otherwise: Vec<Node>,
    },
    /// `for:` — repeats `children` (or the element) once per item in `source`;
    /// a single (non-list) value repeats once. `each` names the scoped item
    /// var, `index:` optionally names the index; `otherwise` (`else:`)
    /// renders when the list is empty.
    For {
        source: Value,
        each: String,
        index: Option<String>,
        /// `key: item.id` — member path rooted in `each` giving items stable
        /// render identity (per-item state survives reorders).
        key: Option<Path>,
        otherwise: Vec<Node>,
    },
    /// `with:` — scopes `source` under `name` (`as:`) for `children` (or the
    /// element) — a single-value `for`.
    With { source: Value, name: String },
    /// `match:` — renders the first `case` arm whose operand equals the
    /// resolved `source`; `otherwise` (`else:`) when none match.
    Match {
        source: Value,
        cases: Vec<Case>,
        otherwise: Vec<Node>,
    },
}

impl Control {
    /// The YAML control key — `if`, `for`, `with` or `match`.
    pub fn keyword(&self) -> &'static str {
        match self {
            Control::If { .. } => "if",
            Control::For { .. } => "for",
            Control::With { .. } => "with",
            Control::Match { .. } => "match",
        }
    }

    /// The value the control key carries — the `if:` condition or the
    /// `for:`/`with:`/`match:` source binding.
    pub fn source(&self) -> &Value {
        match self {
            Control::If { when, .. } => when,
            Control::For { source, .. }
            | Control::With { source, .. }
            | Control::Match { source, .. } => source,
        }
    }

    /// The `else:` children — `with` has none.
    pub fn otherwise(&self) -> &[Node] {
        match self {
            Control::If { otherwise, .. }
            | Control::For { otherwise, .. }
            | Control::Match { otherwise, .. } => otherwise,
            Control::With { .. } => &[],
        }
    }
}

/// An `is:`/`not:` operand on `if` — equality tested against the resolved `when`.
#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema, Clone, Debug, PartialEq)]
pub enum Compare {
    /// `is:` — render when `when` equals the operand.
    Is(Value),
    /// `not:` — render when `when` does not equal the operand.
    Not(Value),
}

/// One `case <value>:` arm of a `match` block.
#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema, Clone, Debug, PartialEq)]
pub struct Case {
    /// The `case` operand — compared by value equality against `match`'s source.
    pub value: Value,
    pub children: Vec<Node>,
}

#[derive(
    serde::Serialize, serde::Deserialize, schemars::JsonSchema, Clone, Debug, Default, PartialEq, Eq,
)]
pub struct ElementRef {
    /// Import alias; `None` is the base library.
    pub namespace: Option<String>,
    pub name: String,
}

impl ElementRef {
    /// `forge.If`/`forge.For`/`forge.With`/`forge.Match` — placeholder
    /// elements the reader puts on bare control blocks. Never rendered or
    /// catalog-validated; `control` carries the meaning.
    pub fn is_control_sentinel(&self) -> bool {
        self.namespace.as_deref() == Some("forge")
            && matches!(self.name.as_str(), "If" | "For" | "With" | "Match")
    }

    /// The sentinel element for a bare control block, e.g. `forge.If`.
    pub fn control_sentinel(name: &str) -> Self {
        Self {
            namespace: Some("forge".into()),
            name: name.into(),
        }
    }
}

impl fmt::Display for ElementRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(namespace) = &self.namespace {
            write!(f, "{namespace}.")?;
        }
        f.write_str(&self.name)
    }
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema, Clone, Debug, PartialEq)]
pub struct EventBinding {
    pub event: String,
    pub handler: HandlerRef,
}

/// A hook id resolved by the hook registry at runtime, with arguments.
#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema, Clone, Debug, PartialEq)]
pub struct HandlerRef {
    pub name: String,
    pub args: Vec<Value>,
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema, Clone, Debug, PartialEq)]
pub enum Value {
    String(String),
    Interpolated(Vec<Segment>),
    Number {
        value: f64,
        unit: Option<Unit>,
    },
    Bool(bool),
    /// A bare word. The catalog decides whether it is an enum value or plain text.
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

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema, Clone, Debug, PartialEq)]
pub enum Segment {
    Text(String),
    Binding(Path),
}

#[derive(
    serde::Serialize, serde::Deserialize, schemars::JsonSchema, Clone, Copy, Debug, PartialEq, Eq,
)]
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

#[derive(
    serde::Serialize, serde::Deserialize, schemars::JsonSchema, Clone, Copy, Debug, PartialEq, Eq,
)]
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

#[derive(
    serde::Serialize, serde::Deserialize, schemars::JsonSchema, Clone, Copy, Debug, PartialEq, Eq,
)]
pub enum BindingMode {
    OneWay,
    TwoWay,
}

/// A path into the view state, e.g. `form.items[0].name`.
#[derive(
    serde::Serialize, serde::Deserialize, schemars::JsonSchema, Clone, Debug, PartialEq, Eq,
)]
pub struct Path(pub Vec<PathSegment>);

#[derive(
    serde::Serialize, serde::Deserialize, schemars::JsonSchema, Clone, Debug, PartialEq, Eq,
)]
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
