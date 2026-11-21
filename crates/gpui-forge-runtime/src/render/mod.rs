//! Per-base `ForgeView` — the module is selected by the `base-*` feature so
//! each base compiles its own element renderer against its own UI toolkit.
//! JSON state-path helpers are toolkit-agnostic and shared below.

#[cfg(feature = "base-gpui-kit")]
mod gpui_kit;
#[cfg(feature = "base-gpui-kit")]
pub use gpui_kit::*;
#[cfg(all(test, feature = "base-gpui-kit"))]
mod bind_salt_tests;

#[cfg(all(feature = "base-guise", not(feature = "base-gpui-kit")))]
mod guise;
#[cfg(all(feature = "base-guise", not(feature = "base-gpui-kit")))]
pub use guise::*;

#[cfg(all(
    feature = "base-bezel",
    not(feature = "base-gpui-kit"),
    not(feature = "base-guise"),
    not(feature = "base-adabraka")
))]
mod bezel;
#[cfg(all(
    feature = "base-bezel",
    not(feature = "base-gpui-kit"),
    not(feature = "base-guise"),
    not(feature = "base-adabraka")
))]
pub use bezel::*;

#[cfg(all(
    feature = "base-ratatui",
    not(feature = "base-gpui-kit"),
    not(feature = "base-guise"),
    not(feature = "base-bezel"),
    not(feature = "base-adabraka")
))]
mod ratatui;
#[cfg(all(
    feature = "base-ratatui",
    not(feature = "base-gpui-kit"),
    not(feature = "base-guise"),
    not(feature = "base-bezel"),
    not(feature = "base-adabraka")
))]
pub use ratatui::*;

#[cfg(all(
    feature = "base-iced",
    not(feature = "base-gpui-kit"),
    not(feature = "base-guise"),
    not(feature = "base-bezel"),
    not(feature = "base-adabraka"),
    not(feature = "base-ratatui")
))]
mod iced;
#[cfg(all(
    feature = "base-iced",
    not(feature = "base-gpui-kit"),
    not(feature = "base-guise"),
    not(feature = "base-bezel"),
    not(feature = "base-adabraka"),
    not(feature = "base-ratatui")
))]
pub use iced::*;

use forge_spec::{Path, PathSegment, Value};
use serde_json::Value as Json;

/// Reads `path` out of a JSON state tree — `a.b` walks objects, `a.0` indexes
/// arrays (the binding path syntax the document writes).
pub fn get_path<'a>(json: &'a Json, path: &Path) -> Option<&'a Json> {
    let mut current = json;
    for segment in &path.0 {
        current = match segment {
            PathSegment::Field(field) => current.get(field.as_str())?,
            PathSegment::Index(index) => current.get(index)?,
        };
    }
    Some(current)
}

/// Binding lookup with the `meta` namespace: `meta.*` paths read the document
/// meta object; anything else reads view state.
pub fn lookup_path<'a>(meta: &'a Json, state: &'a Json, path: &Path) -> Option<&'a Json> {
    let (root, segments) = match path.0.first() {
        Some(PathSegment::Field(name)) if name == "meta" => (meta, &path.0[1..]),
        _ => (state, &path.0[..]),
    };
    segments.iter().try_fold(root, |j, s| match s {
        PathSegment::Field(f) => j.get(f.as_str()),
        PathSegment::Index(i) => j.get(i),
    })
}

/// A document's `meta:` map as a JSON object — `{$meta.k}` bindings resolve
/// against it.
pub fn doc_meta(doc: &forge_spec::Document) -> Json {
    Json::Object(
        doc.meta
            .iter()
            .map(|(k, v)| (k.clone(), Json::String(v.clone())))
            .collect(),
    )
}

/// A `style_changed` write's `Json` as a style `Value` — `null`, `""` and
/// the `[none]` Select sentinel all remove the entry (`None`); bools/numbers
/// map to real `Value`s instead of being flattened to strings.
pub fn style_edit_value(json: &Json) -> Option<Value> {
    if json.is_null() || json.as_str().is_some_and(|s| s.is_empty() || s == "[none]") {
        return None;
    }
    Some(match json {
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => Value::Number {
            value: n.as_f64().unwrap_or_default(),
            unit: None,
        },
        _ => Value::String(json.as_str().unwrap_or_default().to_owned()),
    })
}

/// Writes `value` at `path` in a JSON state tree, creating objects/arrays as
/// needed — the inverse of `get_path`.
pub fn set_path(state: &mut Json, path: &Path, value: Json) {
    let mut target = state;
    for segment in &path.0 {
        target = match segment {
            PathSegment::Field(name) => {
                if !target.is_object() {
                    *target = Json::Object(Default::default());
                }
                target
                    .as_object_mut()
                    .expect("just made an object")
                    .entry(name.clone())
                    .or_insert(Json::Null)
            }
            PathSegment::Index(i) => {
                if !target.is_array() {
                    *target = Json::Array(Vec::new());
                }
                let items = target.as_array_mut().expect("just made an array");
                if items.len() <= *i {
                    items.resize(*i + 1, Json::Null);
                }
                &mut items[*i]
            }
        };
    }
    *target = value;
}
