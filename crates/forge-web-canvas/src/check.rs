//! Parsing and validation, kept free of GPUI so it is testable on any target.

use forge_spec::catalog::{self, Catalog};
use forge_spec::{Diagnostic, Document, validate, yaml};

/// The project base the web canvas renders with.
pub const BASE: &str = "gpui-kit";

pub fn catalogs() -> Vec<Catalog> {
    vec![
        catalog::from_yaml(include_str!("../../../catalogs/gpui-kit.catalog.yaml"))
            .expect("bundled gpui-kit catalog is valid"),
    ]
}

/// Reads and validates `src`. The document is returned whenever it parses, even with
/// validation errors, so the canvas can keep showing as much as it can.
pub fn check(src: &str, catalogs: &[Catalog]) -> (Option<Document>, Vec<Diagnostic>) {
    match yaml::from_str_with_source_map(src) {
        Ok((doc, map)) => {
            let diagnostics = validate::validate(&doc, BASE, catalogs, Some(&map));
            (Some(doc), diagnostics)
        }
        Err(errors) => (None, errors.0),
    }
}

/// Diagnostics as a JSON array of `{line, col, message, hint}` for the page.
pub fn to_json(diagnostics: &[Diagnostic]) -> String {
    serde_json::Value::Array(
        diagnostics
            .iter()
            .map(|d| {
                serde_json::json!({
                    "line": d.pos.line,
                    "col": d.pos.col,
                    "message": d.message,
                    "hint": d.hint,
                })
            })
            .collect(),
    )
    .to_string()
}
