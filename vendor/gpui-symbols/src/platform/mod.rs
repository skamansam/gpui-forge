//! Platform-specific implementations for SF Symbol rendering.

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::render_sf_symbol;

/// Stub implementation for non-macOS platforms.
/// Vendored fix (#88): upstream's stub predates the weight/scale/mode args the
/// call site passes — widened to the macOS signature so the crate compiles.
#[cfg(not(target_os = "macos"))]
pub fn render_sf_symbol(
    _name: &str,
    _point_size: f32,
    _scale: f32,
    _color: u32,
    _weight: crate::SymbolWeight,
    _symbol_scale: crate::SymbolScale,
    _rendering_mode: crate::RenderingMode,
) -> Option<(u32, u32, Vec<u8>)> {
    None // SF Symbols only available on macOS
}
