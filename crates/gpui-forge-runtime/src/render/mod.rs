//! Per-base `ForgeView` — the module is selected by the `base-*` feature so
//! each base compiles its own element renderer against its own UI toolkit.
//! JSON state-path helpers are toolkit-agnostic and shared below.

#[cfg(feature = "base-gpui-kit")]
mod gpui_kit;
#[cfg(feature = "base-gpui-kit")]
pub use gpui_kit::*;

#[cfg(all(feature = "base-gpui02", not(feature = "base-gpui-kit")))]
mod gpui02;
#[cfg(all(feature = "base-gpui02", not(feature = "base-gpui-kit")))]
pub use gpui02::*;

#[cfg(all(
    feature = "base-bezel",
    not(feature = "base-gpui-kit"),
    not(feature = "base-gpui02"),
    not(feature = "base-adabraka")
))]
mod bezel;
#[cfg(all(
    feature = "base-bezel",
    not(feature = "base-gpui-kit"),
    not(feature = "base-gpui02"),
    not(feature = "base-adabraka")
))]
pub use bezel::*;

#[cfg(all(
    feature = "base-ratatui",
    not(feature = "base-gpui-kit"),
    not(feature = "base-gpui02"),
    not(feature = "base-bezel"),
    not(feature = "base-adabraka")
))]
mod ratatui;
#[cfg(all(
    feature = "base-ratatui",
    not(feature = "base-gpui-kit"),
    not(feature = "base-gpui02"),
    not(feature = "base-bezel"),
    not(feature = "base-adabraka")
))]
pub use ratatui::*;

#[cfg(all(
    feature = "base-iced",
    not(feature = "base-gpui-kit"),
    not(feature = "base-gpui02"),
    not(feature = "base-bezel"),
    not(feature = "base-adabraka"),
    not(feature = "base-ratatui")
))]
mod iced;
#[cfg(all(
    feature = "base-iced",
    not(feature = "base-gpui-kit"),
    not(feature = "base-gpui02"),
    not(feature = "base-bezel"),
    not(feature = "base-adabraka"),
    not(feature = "base-ratatui")
))]
pub use iced::*;

use forge_spec::{Path, PathSegment};
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
