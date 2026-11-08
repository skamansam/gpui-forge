//! Opt-in zorite adapters for GPUI Forge — #78.
//!
//! One crate, three `use:`-able libraries, each behind a feature so a project
//! links only what it imports:
//!
//! - `pdf` — `use: pdf: gpui-pdf` → `pdf.Viewer` (hayro-rendered, off-thread)
//! - `whiteboard` — `use: wb: gpui-whiteboard` → `wb.Board`
//! - `math` — `use: math: ratex-gpui` → `math.Tex` / `math.Editor`
//!
//! All three target the gpui-kit base — the crates pin `gpui-pre`.
//!
//! Stateful widgets (`PdfView`, `WhiteboardView`, `MathEditor`) live in GPUI
//! entities, cached per node key and rebuilt when their source prop changes —
//! the doc re-renders every keystroke and a PDF viewer or pipeline can't be
//! recreated per frame.

use gpui_forge_runtime::base::kit::*;
use gpui_forge_runtime::forge_spec::{Node, Value};
use gpui_forge_runtime::render::{ForgeView, get_path};
use std::cell::RefCell;
use std::collections::HashMap;

#[cfg(feature = "pdf")]
pub use pdf::Pdf;
#[cfg(feature = "whiteboard")]
pub use whiteboard::Whiteboard;
#[cfg(feature = "math")]
pub use math::Math;

#[cfg(feature = "pdf")]
mod pdf {
    use super::*;
    use gpui_forge_runtime::forge_spec::catalog::{self, Catalog};
    use gpui_forge_runtime::render::ForgeLibrary;
    use std::sync::OnceLock;

    /// `pdf.*` adapter — register with `view.register_library(Pdf::new())`.
    /// `PdfView` entities are cached per node key; a `src` change releases the
    /// old viewer's atlas textures (`PdfView::release`) before rebuilding.
    #[derive(Default)]
    pub struct Pdf {
        viewers: RefCell<HashMap<String, (std::path::PathBuf, Entity<gpui_pdf::PdfView>)>>,
    }

    impl Pdf {
        pub fn new() -> Self {
            Self::default()
        }

        fn viewer(
            &self,
            key: &str,
            path: std::path::PathBuf,
            quality: f32,
            window: &mut Window,
            cx: &mut Context<ForgeView>,
        ) -> Entity<gpui_pdf::PdfView> {
            let make = |path: &std::path::PathBuf, cx: &mut Context<ForgeView>| {
                let style: gpui_pdf::PdfStyleFn = std::rc::Rc::new(gpui_pdf::PdfStyle::default);
                let quality_fn: gpui_pdf::PdfQualityFn = std::rc::Rc::new(move || quality);
                cx.new(|cx| gpui_pdf::PdfView::new(path.clone(), style, quality_fn, cx))
            };
            let mut viewers = self.viewers.borrow_mut();
            match viewers.entry(key.to_owned()) {
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    if entry.get().0 != path {
                        let old = entry.get().1.clone();
                        old.update(cx, |v, cx| v.release(window, cx));
                        entry.insert((path.clone(), make(&path, cx)));
                    }
                    entry.into_mut().1.clone()
                }
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert((path.clone(), make(&path, cx))).1.clone()
                }
            }
        }
    }

    impl ForgeLibrary for Pdf {
        fn alias(&self) -> &'static str {
            "pdf"
        }

        fn catalog(&self) -> Option<&Catalog> {
            static CAT: OnceLock<Catalog> = OnceLock::new();
            Some(CAT.get_or_init(|| {
                catalog::from_yaml(include_str!("../../../catalogs/pdf.catalog.yaml"))
                    .expect("pdf catalog parses")
            }))
        }

        fn render(
            &self,
            node: &Node,
            key: &str,
            view: &ForgeView,
            window: &mut Window,
            cx: &mut Context<ForgeView>,
        ) -> Option<AnyElement> {
            match node.element.name.as_str() {
                "Viewer" => {
                    let src = prop_str(node, "src", view)?;
                    let quality = prop_num(node, "quality", view).unwrap_or(1.5) as f32;
                    let viewer =
                        self.viewer(key, view.resolve_path(&src), quality, window, cx);
                    Some(viewer.into_any_element())
                }
                _ => None,
            }
        }
    }
}

#[cfg(feature = "whiteboard")]
mod whiteboard {
    use super::*;
    use gpui_forge_runtime::forge_spec::catalog::{self, Catalog};
    use gpui_forge_runtime::render::ForgeLibrary;
    use std::sync::OnceLock;

    /// `wb.*` adapter — register with `view.register_library(Whiteboard::new())`.
    /// `WhiteboardView` entities are cached per node key and rebuilt when
    /// `scene`/`read_only` change.
    /// `key → (scene_json, read_only, board)` — rebuilt when either prop changes.
    type BoardCache = RefCell<HashMap<String, (String, bool, Entity<gpui_whiteboard::WhiteboardView>)>>;

    #[derive(Default)]
    pub struct Whiteboard {
        boards: BoardCache,
    }

    /// A neutral dark palette — the doc has no theme tokens, so the adapter
    /// ships one that reads on Studio's dark canvas.
    fn style() -> gpui_whiteboard::WhiteboardStyleFn {
        std::rc::Rc::new(|| gpui_whiteboard::WhiteboardStyle {
            bg: hsla(0.0, 0.0, 0.08, 1.0),
            grid: hsla(0.0, 0.0, 1.0, 0.08),
            text: hsla(0.0, 0.0, 1.0, 0.6),
            ink: hsla(0.0, 0.0, 0.95, 1.0),
            panel: hsla(0.0, 0.0, 0.12, 0.85),
            panel_strong: hsla(0.0, 0.0, 0.1, 0.95),
            accent: hsla(0.6, 0.7, 0.5, 0.35),
            selection: hsla(0.6, 0.9, 0.6, 1.0),
            swatches: vec![
                hsla(0.0, 0.0, 0.95, 1.0),
                hsla(0.0, 0.8, 0.55, 1.0),
                hsla(0.33, 0.7, 0.5, 1.0),
                hsla(0.6, 0.8, 0.6, 1.0),
                hsla(0.08, 0.9, 0.55, 1.0),
            ],
        })
    }

    impl Whiteboard {
        pub fn new() -> Self {
            Self::default()
        }
    }

    impl ForgeLibrary for Whiteboard {
        fn alias(&self) -> &'static str {
            "wb"
        }

        fn catalog(&self) -> Option<&Catalog> {
            static CAT: OnceLock<Catalog> = OnceLock::new();
            Some(CAT.get_or_init(|| {
                catalog::from_yaml(include_str!("../../../catalogs/whiteboard.catalog.yaml"))
                    .expect("whiteboard catalog parses")
            }))
        }

        fn render(
            &self,
            node: &Node,
            key: &str,
            view: &ForgeView,
            _window: &mut Window,
            cx: &mut Context<ForgeView>,
        ) -> Option<AnyElement> {
            match node.element.name.as_str() {
                "Board" => {
                    let scene_json = prop_str(node, "scene", view).unwrap_or_default();
                    let read_only = prop_bool(node, "read_only", view, false);
                    let mut boards = self.boards.borrow_mut();
                    let board = match boards.entry(key.to_owned()) {
                        std::collections::hash_map::Entry::Occupied(mut entry) => {
                            if entry.get().0 != scene_json || entry.get().1 != read_only {
                                let scene = gpui_whiteboard::Scene::from_json(&scene_json);
                                entry.insert((
                                    scene_json.clone(),
                                    read_only,
                                    cx.new(|cx| {
                                        if read_only {
                                            gpui_whiteboard::WhiteboardView::new_read_only(
                                                scene, style(), cx,
                                            )
                                        } else {
                                            gpui_whiteboard::WhiteboardView::new(
                                                scene, style(), cx,
                                            )
                                        }
                                    }),
                                ));
                            }
                            entry.into_mut().2.clone()
                        }
                        std::collections::hash_map::Entry::Vacant(entry) => {
                            let scene = gpui_whiteboard::Scene::from_json(&scene_json);
                            entry
                                .insert((
                                    scene_json.clone(),
                                    read_only,
                                    cx.new(|cx| {
                                        if read_only {
                                            gpui_whiteboard::WhiteboardView::new_read_only(
                                                scene, style(), cx,
                                            )
                                        } else {
                                            gpui_whiteboard::WhiteboardView::new(
                                                scene, style(), cx,
                                            )
                                        }
                                    }),
                                ))
                                .2
                                .clone()
                        }
                    };
                    Some(board.into_any_element())
                }
                _ => None,
            }
        }
    }
}

#[cfg(feature = "math")]
mod math {
    use super::*;
    use gpui_forge_runtime::forge_spec::catalog::{self, Catalog};
    use gpui_forge_runtime::render::ForgeLibrary;
    use std::sync::OnceLock;

    /// `math.*` adapter — register with `view.register_library(Math::new())`.
    /// `math.Tex` rasterizes LaTeX to an image; `math.Editor` is the
    /// MathQuill-style structural editor (entity-cached per node key).
    #[derive(Default)]
    pub struct Math {
        editors: RefCell<HashMap<String, Entity<ratex_gpui::MathEditor>>>,
    }

    impl Math {
        pub fn new() -> Self {
            Self::default()
        }
    }

    impl ForgeLibrary for Math {
        fn alias(&self) -> &'static str {
            "math"
        }

        fn catalog(&self) -> Option<&Catalog> {
            static CAT: OnceLock<Catalog> = OnceLock::new();
            Some(CAT.get_or_init(|| {
                catalog::from_yaml(include_str!("../../../catalogs/math.catalog.yaml"))
                    .expect("math catalog parses")
            }))
        }

        fn render(
            &self,
            node: &Node,
            key: &str,
            view: &ForgeView,
            window: &mut Window,
            cx: &mut Context<ForgeView>,
        ) -> Option<AnyElement> {
            match node.element.name.as_str() {
                "Tex" => {
                    let src = prop_str(node, "src", view)
                        .or_else(|| node_text(node, view))
                        .unwrap_or_default();
                    if src.trim().is_empty() {
                        return None;
                    }
                    let size = prop_num(node, "size", view).unwrap_or(16.0) as f32;
                    let color = prop_str(node, "color", view)
                        .and_then(|c| hex_color(&c))
                        .unwrap_or_else(|| hsla(0.0, 0.0, 0.95, 1.0));
                    let rendered = ratex_gpui::render::render_latex(
                        &src,
                        size,
                        window.scale_factor(),
                        color,
                    )?;
                    Some(
                        img(rendered.image)
                            .w(px(rendered.width))
                            .h(px(rendered.height))
                            .into_any_element(),
                    )
                }
                "Editor" => {
                    let editor = self
                        .editors
                        .borrow_mut()
                        .entry(key.to_owned())
                        .or_insert_with(|| cx.new(ratex_gpui::MathEditor::new))
                        .clone();
                    Some(editor.into_any_element())
                }
                _ => None,
            }
        }
    }

    /// `#rrggbb` → `Hsla` — the catalog's `color` prop arrives as a string.
    fn hex_color(s: &str) -> Option<Hsla> {
        let hex = s.trim().trim_start_matches('#');
        Some(rgb(u32::from_str_radix(hex, 16).ok()?).into())
    }
}

fn prop<'a>(node: &'a Node, name: &str) -> Option<&'a Value> {
    node.props.iter().find(|(k, _)| k == name).map(|(_, v)| v)
}

/// A `string`/`enum`/literal prop — `{{binding}}` values resolve against the
/// view's state one-way.
fn prop_str(node: &Node, name: &str, view: &ForgeView) -> Option<String> {
    match prop(node, name)? {
        Value::String(s) | Value::Enum(s) => Some(s.clone()),
        Value::Number { value, .. } => Some(value.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Binding { path, .. } => get_path(view.state(), path).map(|j| match j {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        }),
        _ => None,
    }
}

fn prop_bool(node: &Node, name: &str, view: &ForgeView, fallback: bool) -> bool {
    match prop(node, name) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Binding { path, .. }) => get_path(view.state(), path)
            .and_then(|j| j.as_bool())
            .unwrap_or(fallback),
        _ => fallback,
    }
}

fn prop_num(node: &Node, name: &str, view: &ForgeView) -> Option<f64> {
    match prop(node, name) {
        Some(Value::Number { value, .. }) => Some(*value),
        Some(Value::Binding { path, .. }) => get_path(view.state(), path).and_then(|j| j.as_f64()),
        _ => None,
    }
}

/// The node's text content — `content:` elements like `math.Tex`.
fn node_text(node: &Node, view: &ForgeView) -> Option<String> {
    match node.content.as_ref()? {
        Value::String(s) | Value::Enum(s) => Some(s.clone()),
        Value::Interpolated(segments) => Some(
            segments
                .iter()
                .map(|seg| match seg {
                    gpui_forge_runtime::forge_spec::Segment::Text(t) => t.clone(),
                    gpui_forge_runtime::forge_spec::Segment::Binding(p) => get_path(
                        view.state(),
                        p,
                    )
                    .map(|j| match j {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    })
                    .unwrap_or_default(),
                })
                .collect(),
        ),
        _ => None,
    }
}
