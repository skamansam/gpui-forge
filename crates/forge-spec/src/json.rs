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

/// The JSON Schema for `Document` — generated from the serde model and kept in
/// lockstep automatically. Emit with `forge schema`.
pub fn schema() -> String {
    serde_json::to_string_pretty(&schemars::schema_for!(Document)).unwrap_or_default()
}
