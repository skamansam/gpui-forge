//! Where each part of a document was written, so later passes can report `line:col`.

use std::collections::HashMap;

use crate::diagnostic::Pos;
use crate::model::PathStep;

/// A node's location: path steps from the root (`[]` is the root,
/// `[Child(0), Child(2)]` its first child's third child. `Else`/`Case`
/// steps reach `else:`/`case` branch children).
pub type NodePath = Vec<PathStep>;

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
