//! Template Forge adapter — the minimal shape every `forge-lib-*` crate follows.
//!
//! To ship a real adapter: copy this crate, rename it, point `use:` at your
//! library's upstream crate, and fill in the `render()` match. See
//! `docs/format/adapters.md` for the full checklist.

use gpui_forge_runtime::base::kit::*;
use gpui_forge_runtime::forge_spec::catalog::{self, Catalog};
use gpui_forge_runtime::forge_spec::{Node, Value};
use gpui_forge_runtime::render::{ForgeLibrary, ForgeView};
use std::sync::OnceLock;

/// `mylib.*` adapter — register with `view.register_library(Template)`.
pub struct Template;

impl ForgeLibrary for Template {
    /// The namespace a document imports: `use: mylib: template-lib` → `mylib.Pill`.
    fn alias(&self) -> &'static str {
        "mylib"
    }

    /// The catalog is embedded so validation and the Studio palette learn the
    /// elements without a file lookup.
    fn catalog(&self) -> Option<&Catalog> {
        static CAT: OnceLock<Catalog> = OnceLock::new();
        Some(CAT.get_or_init(|| {
            catalog::from_yaml(include_str!("../../../catalogs/template.catalog.yaml"))
                .expect("template catalog parses")
        }))
    }

    /// Map `mylib.<name>` nodes onto widgets. Return `None` for elements the
    /// catalog lists but this shim doesn't render — they fall through to the
    /// built-in catalog.
    fn render(
        &self,
        node: &Node,
        key: &str,
        _view: &ForgeView,
        _window: &mut Window,
        _cx: &mut Context<ForgeView>,
    ) -> Option<AnyElement> {
        match node.element.name.as_str() {
            "Pill" => {
                let label = prop_str(node, "label").unwrap_or_else(|| key.to_string());
                let (bg, fg) = match prop_str_opt(node, "variant").as_deref() {
                    Some("Warning") => (rgb(0xf5a524), rgb(0x1a1a1a)),
                    _ => (rgb(0x0e639c), rgb(0xffffff)),
                };
                Some(
                    div()
                        .px_2()
                        .py_1()
                        .rounded_full()
                        .bg(bg)
                        .text_color(fg)
                        .text_sm()
                        .child(label)
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

fn prop_str_opt(node: &Node, name: &str) -> Option<String> {
    match prop(node, name)? {
        Value::String(s) | Value::Enum(s) => Some(s.clone()),
        _ => None,
    }
}

fn prop_str(node: &Node, name: &str) -> Option<String> {
    node.content
        .as_ref()
        .and_then(|v| match v {
            Value::String(s) | Value::Enum(s) => Some(s.clone()),
            _ => None,
        })
        .or_else(|| prop_str_opt(node, name))
}
