//! JSON encoding of a `Document` — the machine-interchange format.

use crate::model::Document;

/// Parses a `Document` from JSON (the serde representation of the model).
pub fn from_str(src: &str) -> Result<Document, serde_json::Error> {
    serde_json::from_str(src)
}

/// Writes a `Document` as pretty JSON.
pub fn to_string(doc: &Document) -> String {
    serde_json::to_string_pretty(doc).unwrap_or_default()
}
