//! Per-base WASM canvases (#99): one crate, one `canvas-<base>.wasm` artifact
//! per wasm-capable base — `--features base-<id>` selects the impl.
//!
//! - `base-gpui-kit` — the original canvas (gpui_kit::web::WebPlatform)
//! - `base-iced` — iced::application over the twin's view()/update()

#![recursion_limit = "256"]

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
