//! Per-base WASM canvases (#99): one crate, one `canvas-<base>.wasm` artifact
//! per wasm-capable base — `--features base-<id>` selects the impl.
//!
//! - `base-gpui-kit` — the original canvas (gpui_kit::web::WebPlatform)
//! - `base-iced` — iced::application over the twin's view()/update()

#![recursion_limit = "256"]

// Feature-forwarding only — pulls `instant`'s wasm-bindgen backend so parking_lot's
// `now()` resolves on wasm32 (see Cargo.toml).
#[cfg(target_arch = "wasm32")]
use instant as _;

#[cfg(feature = "base-bezel")]
pub mod bezel;
#[cfg(feature = "base-gpui-kit")]
mod gpui_kit_impl;
#[cfg(feature = "base-iced")]
pub mod iced;
#[cfg(feature = "base-ratatui")]
pub mod ratatui_impl;
#[cfg(feature = "base-gpui-kit")]
pub use gpui_kit_impl::*;
