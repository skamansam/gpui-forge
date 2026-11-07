//! The base component system this runtime build targets.

/// Identifier of the active base, as written in a project's `forge.toml`.
#[cfg(feature = "base-gpui-kit")]
pub const BASE_ID: &str = "gpui-kit";

/// The active base's component crate, which also re-exports its GPUI build.
#[cfg(feature = "base-gpui-kit")]
pub use gpui_kit as kit;

/// Identifier of the active base, as written in a project's `forge.toml`.
#[cfg(all(feature = "base-gpui02", not(feature = "base-gpui-kit")))]
pub const BASE_ID: &str = "gpui02";

/// The gpui02 base's component crate (guise on crates.io gpui 0.2.2).
#[cfg(all(feature = "base-gpui02", not(feature = "base-gpui-kit")))]
pub use guise as kit;

/// Identifier of the active base, as written in a project's `forge.toml`.
#[cfg(all(
    feature = "base-bezel",
    not(feature = "base-gpui-kit"),
    not(feature = "base-gpui02"),
    not(feature = "base-adabraka")
))]
pub const BASE_ID: &str = "bezel";

/// The Bezel base's component crate (SwiftUI-flavored on bezel-gpui).
#[cfg(all(
    feature = "base-bezel",
    not(feature = "base-gpui-kit"),
    not(feature = "base-gpui02"),
    not(feature = "base-adabraka")
))]
pub use bezel as kit;
