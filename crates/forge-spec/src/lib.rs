//! The Forge Schema.
//!
//! Pure data: this crate must never depend on GPUI or any base component system,
//! so documents can be read and validated by every tool regardless of base.

pub mod catalog;
pub mod diagnostic;
pub mod model;
pub mod source_map;
pub mod validate;
pub mod value;
pub mod yaml;

pub use diagnostic::{Diagnostic, Diagnostics, Pos};
pub use model::*;

/// Version of the Forge document format this crate reads and writes.
pub const SPEC_VERSION: &str = "0.1";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_version_is_set() {
        assert!(!SPEC_VERSION.is_empty());
    }
}
