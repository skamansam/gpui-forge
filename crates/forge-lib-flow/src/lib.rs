//! Adapter for `gpui-flow` (`use: gflow: gpui-flow` → `gflow.*` elements) —
//! #89. React-Flow-style node editor on the gpui-kit base. Alias is `gflow`
//! so it can't be confused with the gpui02 base's `flow` (ferrum-flow).
//!
//! `gflow.Canvas` is an `Entity<FlowGraph>` cached per node key (same shape
//! as the ferrum adapter); its `FlowState` starts from the `graph` map prop.
//! Controls/minimap overlay the canvas when their props are on.

use gpui_flow::{
    BackgroundPattern, Controls, EdgeType, FlowEdge, FlowGraph, FlowNode, FlowState, HandleDef,
    HandlePosition, Minimap,
};
use gpui_forge_runtime::base::kit::*;
use gpui_forge_runtime::forge_spec::catalog::{self, Catalog};
use gpui_forge_runtime::forge_spec::{Node, Value};
use gpui_forge_runtime::render::{ForgeLibrary, ForgeView};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::OnceLock;

/// `gflow.*` adapter — register with `view.register_library(Flow::new())`.
#[derive(Default)]
pub struct Flow {
    graphs: RefCell<HashMap<String, FlowEntities>>,
}

struct FlowEntities {
    graph: Entity<FlowGraph>,
    controls: Entity<Controls>,
    minimap: Entity<Minimap>,
}

impl Flow {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ForgeLibrary for Flow {
    fn alias(&self) -> &'static str {
        "gflow"
    }

    fn catalog(&self) -> Option<&Catalog> {
        static CAT: OnceLock<Catalog> = OnceLock::new();
        Some(CAT.get_or_init(|| {
            catalog::from_yaml(include_str!("../../../catalogs/gflow.catalog.yaml"))
                .expect("gflow catalog parses")
        }))
    }

    fn render(
        &self,
        node: &Node,
        key: &str,
        _view: &ForgeView,
        _window: &mut Window,
        cx: &mut Context<ForgeView>,
    ) -> Option<AnyElement> {
        match node.element.name.as_str() {
            "Canvas" => {
                let (graph, controls, minimap) = {
                    let mut graphs = self.graphs.borrow_mut();
                    let flow = graphs.entry(key.to_owned()).or_insert_with(|| {
                        let (nodes, edges) = parse_graph(node);
                        let state = cx.new(|_| FlowState::new(nodes, edges));
                        FlowEntities {
                            graph: cx.new(|cx| {
                                let mut graph = FlowGraph::new(state.clone(), cx);
                                graph = match prop_str_opt(node, "pattern").as_deref() {
                                    Some("Lines") => graph.bg_pattern(BackgroundPattern::Lines),
                                    Some("Cross") => graph.bg_pattern(BackgroundPattern::Cross),
                                    _ => graph.bg_pattern(BackgroundPattern::Dots),
                                };
                                graph
                            }),
                            controls: cx.new(|_| Controls::new(state.clone())),
                            minimap: cx.new(|_| Minimap::new(state.clone())),
                        }
                    });
                    (
                        flow.graph.clone(),
                        flow.controls.clone(),
                        flow.minimap.clone(),
                    )
                };
                let mut canvas = div().size_full().relative().child(graph.into_any_element());
                if prop_bool_default(node, "controls", true) {
                    canvas = canvas.child(
                        div()
                            .absolute()
                            .bottom_4()
                            .left_4()
                            .child(controls.into_any_element()),
                    );
                }
                if prop_bool(node, "minimap") {
                    canvas = canvas.child(
                        div()
                            .absolute()
                            .bottom_4()
                            .right_4()
                            .child(minimap.into_any_element()),
                    );
                }
                Some(canvas.into_any_element())
            }
            _ => None,
        }
    }
}

/// Reads the `graph` map prop: `{nodes: [...], edges: [...]}`.
fn parse_graph(node: &Node) -> (Vec<FlowNode>, Vec<FlowEdge>) {
    let Some(Value::Map(graph)) = prop(node, "graph") else {
        return (Vec::new(), Vec::new());
    };
    let nodes = map_get(graph, "nodes").map(parse_nodes).unwrap_or_default();
    let edges = map_get(graph, "edges").map(parse_edges).unwrap_or_default();
    (nodes, edges)
}

fn parse_nodes(value: &Value) -> Vec<FlowNode> {
    let Value::List(items) = value else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let Value::Map(m) = item else { return None };
            let id = map_str(m, "id")?;
            let mut n = FlowNode::new(id, map_num(m, "x"), map_num(m, "y"));
            if let Some(label) = map_str(m, "label") {
                n = n.label(label);
            }
            if let Some(node_type) = map_str(m, "type") {
                n = n.node_type(node_type);
            }
            match map_get(m, "handles") {
                Some(v) => n = n.handles(parse_handles(v)),
                // Nodes default to no handles — give them the usual
                // source-right / target-left pair so edges can attach.
                None => {
                    n = n.handles(vec![
                        HandleDef::source(HandlePosition::Right),
                        HandleDef::target(HandlePosition::Left),
                    ])
                }
            }
            if let (Some(w), Some(h)) = (map_num_opt(m, "w"), map_num_opt(m, "h")) {
                n = n.size(w, h);
            }
            Some(n)
        })
        .collect()
}

fn parse_handles(value: &Value) -> Vec<HandleDef> {
    let Value::List(items) = value else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let Value::Map(m) = item else { return None };
            let position = match map_str(m, "position").as_deref() {
                Some("Left") => HandlePosition::Left,
                Some("Top") => HandlePosition::Top,
                Some("Bottom") => HandlePosition::Bottom,
                _ => HandlePosition::Right,
            };
            let mut def = match map_str(m, "type").as_deref() {
                Some("Target") => HandleDef::target(position),
                _ => HandleDef::source(position),
            };
            if let Some(id) = map_str(m, "id") {
                def = def.id(id);
            }
            Some(def)
        })
        .collect()
}

fn parse_edges(value: &Value) -> Vec<FlowEdge> {
    let Value::List(items) = value else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let Value::Map(m) = item else { return None };
            let mut e = FlowEdge::new(
                map_str(m, "id")?,
                map_str(m, "source")?,
                map_str(m, "target")?,
            );
            if let Some(label) = map_str(m, "label") {
                e = e.label(label);
            }
            e.edge_type = match map_str(m, "type").as_deref() {
                Some("Straight") => EdgeType::Straight,
                Some("SmoothStep") => EdgeType::SmoothStep {
                    border_radius: 4.0,
                    offset: 16.0,
                },
                _ => EdgeType::default(),
            };
            e.animated = map_bool(m, "animated");
            if let (Some(sh), Some(th)) = (map_str(m, "source_handle"), map_str(m, "target_handle"))
            {
                e.source_handle = Some(sh.into());
                e.target_handle = Some(th.into());
            }
            Some(e)
        })
        .collect()
}

fn map_get<'a>(map: &'a [(String, Value)], key: &str) -> Option<&'a Value> {
    map.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn map_str(map: &[(String, Value)], key: &str) -> Option<String> {
    match map_get(map, key)? {
        Value::String(s) | Value::Enum(s) => Some(s.clone()),
        Value::Number { value, .. } => Some(value.to_string()),
        _ => None,
    }
}

fn map_num(map: &[(String, Value)], key: &str) -> f32 {
    map_num_opt(map, key).unwrap_or(0.)
}

fn map_num_opt(map: &[(String, Value)], key: &str) -> Option<f32> {
    match map_get(map, key)? {
        Value::Number { value, .. } => Some(*value as f32),
        _ => None,
    }
}

fn map_bool(map: &[(String, Value)], key: &str) -> bool {
    matches!(map_get(map, key), Some(Value::Bool(true)))
}

fn prop<'a>(node: &'a Node, name: &str) -> Option<&'a Value> {
    node.props.iter().find(|(k, _)| k == name).map(|(_, v)| v)
}

fn prop_str_opt(node: &Node, name: &str) -> Option<String> {
    match prop(node, name)? {
        Value::String(s) | Value::Enum(s) => Some(s.clone()),
        _ => None,
    }
}

fn prop_bool(node: &Node, name: &str) -> bool {
    matches!(prop(node, name), Some(Value::Bool(true)))
}

fn prop_bool_default(node: &Node, name: &str, default: bool) -> bool {
    match prop(node, name) {
        Some(Value::Bool(b)) => *b,
        _ => default,
    }
}
