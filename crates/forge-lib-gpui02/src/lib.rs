//! Adapters for gpui-0.2 libraries — the Vanilla GPUI 0.2 base (#86, #87, #88).
//!
//! One crate, three `use:`-able libraries, each behind a feature:
//!
//! - `ferrum` — `use: flow: ferrum-flow` → `flow.Canvas` (node editor)
//! - `rhythm` — `use: rhythm: rhythm-gpui` → `rhythm.Frame` (baseline grid)
//! - `symbols` — `use: sf: gpui-symbols` → `sf.Icon` (SF Symbols, macOS)
//!
//! All are off by default: their crates pin crates.io `gpui 0.2.x` and can only
//! link under the `base-gpui02` runtime arm, which the feature forwards.
//! `kit::gpui` re-exports that same gpui build, so the adapter's types always
//! unify with the runtime's.

#[cfg(feature = "ferrum")]
pub use ferrum::Ferrum;
#[cfg(feature = "rhythm")]
pub use rhythm::Rhythm;
#[cfg(feature = "symbols")]
pub use symbols::Symbols;

#[cfg(feature = "ferrum")]
mod ferrum {
    use gpui_forge_runtime::base::kit::gpui::*;
    use gpui_forge_runtime::forge_spec::Node;
    use gpui_forge_runtime::forge_spec::catalog::{self, Catalog};
    use gpui_forge_runtime::render::{ForgeLibrary, ForgeView};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::sync::OnceLock;

    /// `flow.*` adapter — `flow.Canvas` is a `FlowCanvas` entity cached per
    /// node key. Its `Graph` starts empty (a `scene` prop may follow).
    #[derive(Default)]
    pub struct Ferrum {
        canvases: RefCell<HashMap<String, Entity<ferrum_flow::FlowCanvas>>>,
    }

    impl Ferrum {
        pub fn new() -> Self {
            Self::default()
        }
    }

    impl ForgeLibrary for Ferrum {
        fn alias(&self) -> &'static str {
            "flow"
        }

        fn catalog(&self) -> Option<&Catalog> {
            static CAT: OnceLock<Catalog> = OnceLock::new();
            Some(CAT.get_or_init(|| {
                catalog::from_yaml(include_str!("../../../catalogs/ferrum.catalog.yaml"))
                    .expect("ferrum catalog parses")
            }))
        }

        fn render(
            &self,
            node: &Node,
            key: &str,
            _view: &ForgeView,
            window: &mut Window,
            cx: &mut Context<ForgeView>,
        ) -> Option<AnyElement> {
            match node.element.name.as_str() {
                "Canvas" => {
                    let canvas = self
                        .canvases
                        .borrow_mut()
                        .entry(key.to_owned())
                        .or_insert_with(|| {
                            cx.new(|cx| {
                                ferrum_flow::FlowCanvas::builder(
                                    ferrum_flow::Graph::new(),
                                    cx,
                                    window,
                                )
                                .default_plugins()
                                .build()
                            })
                        })
                        .clone();
                    Some(canvas.into_any_element())
                }
                _ => None,
            }
        }
    }
}

#[cfg(feature = "rhythm")]
mod rhythm {
    use gpui_forge_runtime::base::kit::gpui::*;
    use gpui_forge_runtime::forge_spec::catalog::{self, Catalog};
    use gpui_forge_runtime::forge_spec::{Node, Value};
    use gpui_forge_runtime::render::{ForgeLibrary, ForgeView, get_path};
    use std::sync::OnceLock;

    /// `rhythm.*` adapter — `rhythm.Frame` wraps its text content in a
    /// baseline-rhythm frame (`RhythmGrid` cell = `size`, `ratio` = x-height
    /// fraction of the cell).
    pub struct Rhythm;

    impl ForgeLibrary for Rhythm {
        fn alias(&self) -> &'static str {
            "rhythm"
        }

        fn catalog(&self) -> Option<&Catalog> {
            static CAT: OnceLock<Catalog> = OnceLock::new();
            Some(CAT.get_or_init(|| {
                catalog::from_yaml(include_str!("../../../catalogs/rhythm.catalog.yaml"))
                    .expect("rhythm catalog parses")
            }))
        }

        fn render(
            &self,
            node: &Node,
            _key: &str,
            view: &ForgeView,
            _window: &mut Window,
            _cx: &mut Context<ForgeView>,
        ) -> Option<AnyElement> {
            match node.element.name.as_str() {
                "Frame" => {
                    let size = prop_num(node, "size", view).unwrap_or(8.0) as f32;
                    let ratio = prop_num(node, "ratio", view).unwrap_or(0.5) as f32;
                    let text = node
                        .content
                        .as_ref()
                        .and_then(|v| match v {
                            Value::String(s) => Some(s.clone()),
                            Value::Binding { path, .. } => {
                                get_path(view.state(), path).map(|j| match j {
                                    serde_json::Value::String(s) => s.clone(),
                                    other => other.to_string(),
                                })
                            }
                            _ => None,
                        })
                        .unwrap_or_default();
                    Some(
                        rhythm_gpui::rhythm_frame(rhythm_gpui::RhythmGrid::new(px(size)), ratio)
                            .child(text)
                            .into_any_element(),
                    )
                }
                _ => None,
            }
        }
    }

    fn prop<'a>(node: &'a Node, name: &str) -> Option<&'a Value> {
        node.props.iter().find(|(k, _)| k == name).map(|(_, v)| v)
    }

    fn prop_num(node: &Node, name: &str, view: &ForgeView) -> Option<f64> {
        match prop(node, name) {
            Some(Value::Number { value, .. }) => Some(*value),
            Some(Value::Binding { path, .. }) => {
                get_path(view.state(), path).and_then(|j| j.as_f64())
            }
            _ => None,
        }
    }
}

#[cfg(feature = "symbols")]
mod symbols {
    use gpui_forge_runtime::base::kit::gpui::*;
    use gpui_forge_runtime::forge_spec::catalog::{self, Catalog};
    use gpui_forge_runtime::forge_spec::{Node, Value};
    use gpui_forge_runtime::render::{ForgeLibrary, ForgeView, get_path};
    use std::sync::OnceLock;

    /// `sf.*` adapter — `sf.Icon` renders a named SF Symbol (macOS only —
    /// the crate resolves symbols through AppKit).
    pub struct Symbols;

    impl ForgeLibrary for Symbols {
        fn alias(&self) -> &'static str {
            "sf"
        }

        fn catalog(&self) -> Option<&Catalog> {
            static CAT: OnceLock<Catalog> = OnceLock::new();
            Some(CAT.get_or_init(|| {
                catalog::from_yaml(include_str!("../../../catalogs/symbols.catalog.yaml"))
                    .expect("symbols catalog parses")
            }))
        }

        fn render(
            &self,
            node: &Node,
            _key: &str,
            view: &ForgeView,
            _window: &mut Window,
            _cx: &mut Context<ForgeView>,
        ) -> Option<AnyElement> {
            match node.element.name.as_str() {
                "Icon" => {
                    let name = prop_str(node, "name", view)?;
                    Some(gpui_symbols::Icon::new(name).into_any_element())
                }
                _ => None,
            }
        }
    }

    fn prop<'a>(node: &'a Node, name: &str) -> Option<&'a Value> {
        node.props.iter().find(|(k, _)| k == name).map(|(_, v)| v)
    }

    fn prop_str(node: &Node, name: &str, view: &ForgeView) -> Option<String> {
        match prop(node, name)? {
            Value::String(s) | Value::Enum(s) => Some(s.clone()),
            Value::Binding { path, .. } => get_path(view.state(), path).map(|j| match j {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            }),
            _ => None,
        }
    }
}
