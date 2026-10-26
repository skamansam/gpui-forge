//! The base component system this runtime build targets.

/// Identifier of the active base, as written in a project's `forge.toml`.
#[cfg(feature = "base-gpui-kit")]
pub const BASE_ID: &str = "gpui-kit";

/// The active base's component crate, which also re-exports its GPUI build.
#[cfg(feature = "base-gpui-kit")]
pub use gpui_kit as kit;

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "base-gpui-kit")]
    fn default_base_is_gpui_kit() {
        assert_eq!(super::BASE_ID, "gpui-kit");
    }
}
