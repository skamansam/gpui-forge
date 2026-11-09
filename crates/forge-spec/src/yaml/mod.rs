//! Forge YAML (`docs/format/forge-yaml.md`): reader and canonical writer.

mod preserve;
mod read;
pub(crate) mod tree;
mod write;

pub use read::{from_str, from_str_with_source_map};
pub use write::{IdStyle, WriteOptions, to_string, to_string_preserving};
