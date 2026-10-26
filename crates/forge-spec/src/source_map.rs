//! Where each part of a document was written, so later passes can report `line:col`.

use std::collections::HashMap;

use crate::diagnostic::Pos;

/// A node's location: child indices from the root (`[]` is the root, `[0, 2]` its first child's third child).
pub type NodePath = Vec<usize>;

#[derive(Clone, Debug, Default)]
pub struct SourceMap {
    pub nodes: HashMap<NodePath, NodeSpans>,
    /// Position of the `base:` value.
    pub base: Option<Pos>,
    /// Import alias -> position of its key under `use:`.
    pub imports: HashMap<String, Pos>,
}

#[derive(Clone, Debug, Default)]
pub struct NodeSpans {
    /// The element key (`Button#save`).
    pub element: Pos,
    pub content: Option<Pos>,
    /// Prop name -> position of its key.
    pub props: HashMap<String, Pos>,
    /// Event name -> position of its `on <event>` key.
    pub events: HashMap<String, Pos>,
    /// Style key -> position of the key.
    pub style: HashMap<String, Pos>,
}
