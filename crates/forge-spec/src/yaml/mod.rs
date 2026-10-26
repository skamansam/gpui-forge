//! Forge YAML (`docs/spec/forge-yaml.md`): reader and canonical writer.

mod read;
pub(crate) mod tree;
mod write;

pub use read::{from_str, from_str_with_source_map};
pub use write::{IdStyle, WriteOptions, to_string};
