//! Resolving [`CellColor`] to RGB — the standard xterm 256-color palette plus
//! caller-supplied defaults for the two named terminal colors.

use crate::emulator::CellColor;

/// The first 16 palette entries — the ANSI named colors (0-7) and their
/// bright variants (8-15), as xterm defines them.
const ANSI16: [[u8; 3]; 16] = [
    [0, 0, 0],
    [205, 0, 0],
    [0, 205, 0],
    [205, 205, 0],
    [0, 0, 238],
    [205, 0, 205],
    [0, 205, 205],
    [229, 229, 229],
    [127, 127, 127],
    [255, 0, 0],
    [0, 255, 0],
    [255, 255, 0],
    [92, 92, 255],
    [255, 0, 255],
    [0, 255, 255],
    [255, 255, 255],
];

/// 256-color index → RGB: 0-15 named, 16-231 the 6×6×6 cube, 232-255 the
/// 24-step grayscale ramp.
pub fn indexed_rgb(ix: u8) -> (u8, u8, u8) {
    match ix {
        0..=15 => ANSI16[ix as usize].into(),
        16..=231 => {
            let i = ix - 16;
            let level = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
            (level(i / 36), level((i % 36) / 6), level(i % 6))
        }
        232..=255 => {
            let g = 8 + 10 * (ix - 232);
            (g, g, g)
        }
    }
}

/// Resolve a cell color against the theme defaults.
pub fn resolve(
    color: CellColor,
    default_fg: (u8, u8, u8),
    default_bg: (u8, u8, u8),
) -> (u8, u8, u8) {
    match color {
        CellColor::Foreground => default_fg,
        CellColor::Background => default_bg,
        CellColor::Rgb(r, g, b) => (r, g, b),
        CellColor::Indexed(ix) => indexed_rgb(ix),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_and_ramp() {
        assert_eq!(indexed_rgb(16), (0, 0, 0));
        assert_eq!(indexed_rgb(231), (255, 255, 255));
        assert_eq!(indexed_rgb(196), (255, 0, 0)); // red corner of the cube
        assert_eq!(indexed_rgb(232), (8, 8, 8));
        assert_eq!(indexed_rgb(255), (238, 238, 238));
    }
}
