//! Parsing and validation, kept free of GPUI so it is testable on any target.

use forge_spec::base::Base;
use forge_spec::catalog::{self, Catalog};
use forge_spec::{Diagnostic, Document, validate, yaml};

/// The catalogs bundled with this build: the active base's library plus the
/// Forge runtime library (`forge.` elements such as `forge.Preview`).
#[cfg(feature = "base-gpui-kit")]
pub fn catalogs() -> Vec<Catalog> {
    vec![
        catalog::from_yaml(include_str!("../../../catalogs/gpui-kit.catalog.yaml"))
            .expect("bundled gpui-kit catalog is valid"),
        catalog::from_yaml(include_str!("../../../catalogs/forge.catalog.yaml"))
            .expect("bundled forge catalog is valid"),
        catalog::from_yaml(include_str!("../../../catalogs/ely.catalog.yaml"))
            .expect("bundled ely catalog is valid"),
        catalog::from_yaml(include_str!("../../../catalogs/gpui02.catalog.yaml"))
            .expect("bundled gpui02 catalog is valid"),
    ]
}

/// Reads and validates `src` for `base`. The document is returned whenever it parses, even with
/// validation errors, so a canvas can keep showing as much as it can.
/// The bases shipped with this build — the registry `base:` validates against
/// and the CLI/Studio/compiler pick from (#91).
pub fn bases() -> Vec<Base> {
    vec![
        Base::from_yaml(include_str!("../../../bases/gpui-kit.base.yaml"))
            .expect("bundled gpui-kit base manifest is valid"),
        Base::from_yaml(include_str!("../../../bases/gpui02.base.yaml"))
            .expect("bundled gpui02 base manifest is valid"),
    ]
}

pub fn check(src: &str, base: &str, catalogs: &[Catalog]) -> (Option<Document>, Vec<Diagnostic>) {
    match yaml::from_str_with_source_map(src) {
        Ok((doc, map)) => {
            let diagnostics = validate::validate(&doc, base, catalogs, Some(&map));
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
