//! `forge-term` — a real terminal emulator for Forge.
//!
//! [`Emulator`] folds a byte stream (PTY output, a TUI library's ANSI writes)
//! into a styled cell grid through `alacritty_terminal`'s `vte` parser — the
//! same engine alacritty, Zed, and bezel-terminal trust. [`TermView`] (the
//! `gpui` feature) paints that grid into a gpui canvas.
//!
//! The split mirrors Zed: emulation is pure state, the view only paints.

pub mod emulator;
pub mod palette;
#[cfg(feature = "gpui")]
pub mod view;

pub use emulator::{CellColor, CellSnapshot, CursorShape, CursorSnapshot, Emulator, GridSize};
#[cfg(feature = "gpui")]
pub use view::{TermStyle, TermView};
