//! The gpui half of the terminal: `TermView` paints an [`Emulator`]'s grid.
//!
//! The model is Zed's `terminal_view`: per row, background quads for the
//! cells' backgrounds, then one shaped line for the glyphs — `TextRun`s carry
//! per-cell fg/underline, the quad carries the cell-box-sized background
//! (a `TextRun::background_color` only covers the glyph's advance, not the
//! whole cell). The cursor is a quad in whatever shape the program asked for.

use std::{cell::RefCell, rc::Rc};

use gpui::{
    Bounds, Font, FontFeatures, FontStyle, FontWeight, Hsla, IntoElement, Pixels, Point,
    ShapedLine, SharedString, Styled, TextAlign, TextRun, UnderlineStyle, Window, canvas, fill,
    point, px, rgb, size,
};

use crate::{
    emulator::{CellSnapshot, CursorShape, Emulator},
    palette,
};

/// Style knobs for [`TermView`] — the theme fills these in.
#[derive(Clone)]
pub struct TermStyle {
    /// Monospace family for the grid.
    pub font_family: SharedString,
    pub font_size: Pixels,
    /// Terminal default foreground/background (cells that never set a color).
    pub default_fg: Hsla,
    pub default_bg: Hsla,
    /// Cursor fill.
    pub cursor: Hsla,
}

impl Default for TermStyle {
    fn default() -> Self {
        Self {
            font_family: "monospace".into(),
            font_size: px(12.0),
            default_fg: rgb(0xd0d0d0).into(),
            default_bg: rgb(0x101018).into(),
            cursor: rgb(0xd0d0d0).into(),
        }
    }
}

/// An element that paints an [`Emulator`]'s cell grid. Feed bytes into the
/// emulator wherever the byte stream lands; this re-reads the grid each paint.
pub struct TermView {
    emulator: Rc<RefCell<Emulator>>,
    style: TermStyle,
}

impl TermView {
    pub fn new(emulator: Rc<RefCell<Emulator>>, style: TermStyle) -> Self {
        Self { emulator, style }
    }
}

fn rgb8(hsla: Hsla) -> (u8, u8, u8) {
    let c = gpui::Rgba::from(hsla);
    ((c.r * 255.) as u8, (c.g * 255.) as u8, (c.b * 255.) as u8)
}

fn hsl(c: (u8, u8, u8)) -> Hsla {
    rgb(((c.0 as u32) << 16) | ((c.1 as u32) << 8) | c.2 as u32).into()
}

fn dim(c: (u8, u8, u8), toward: (u8, u8, u8)) -> (u8, u8, u8) {
    let m = |a: u8, b: u8| (a as u16 / 2 + b as u16 / 2) as u8;
    (m(c.0, toward.0), m(c.1, toward.1), m(c.2, toward.2))
}

pub struct RowPaint {
    line: Option<ShapedLine>,
    bgs: Vec<(Bounds<Pixels>, Hsla)>,
}

pub struct TermPrepaint {
    rows: Vec<RowPaint>,
    cursor: Option<(Bounds<Pixels>, bool)>, // bounds, hollow?
    origin: Point<Pixels>,
    cell_h: Pixels,
}

impl IntoElement for TermView {
    type Element = gpui::Canvas<TermPrepaint>;

    fn into_element(self) -> Self::Element {
        let emulator = self.emulator;
        let style = self.style;
        let font_base = move |weight, fstyle| Font {
            family: style.font_family.clone(),
            features: FontFeatures::default(),
            fallbacks: None,
            weight,
            style: fstyle,
        };

        canvas(
            move |bounds, _window, cx| {
                let emu = emulator.borrow();
                let ts = cx.text_system();
                let font_id = ts.resolve_font(&font_base(FontWeight::NORMAL, FontStyle::Normal));
                let cell_w = ts
                    .advance(font_id, style.font_size, ' ')
                    .map(|s| s.width)
                    .unwrap_or(style.font_size * 0.6);
                let cell_h =
                    ts.ascent(font_id, style.font_size) + ts.descent(font_id, style.font_size);
                let (def_fg, def_bg) = (rgb8(style.default_fg), rgb8(style.default_bg));
                let mut rows = Vec::with_capacity(emu.rows());
                for r in 0..emu.rows() {
                    let cells = emu.line(r);
                    rows.push(shape_row(
                        &cells,
                        bounds.origin.y + cell_h * r as f32,
                        bounds.origin.x,
                        cell_w,
                        cell_h,
                        def_fg,
                        def_bg,
                        style.font_size,
                        &font_base,
                        _window,
                    ));
                }
                let cursor = emu.cursor().map(|c| {
                    let origin = point(
                        bounds.origin.x + cell_w * c.col as f32,
                        bounds.origin.y + cell_h * c.row as f32,
                    );
                    let (bounds, hollow) = match c.shape {
                        CursorShape::Beam => (
                            Bounds {
                                origin,
                                size: size(px(1.5), cell_h),
                            },
                            false,
                        ),
                        CursorShape::Underline => (
                            Bounds {
                                origin: origin + point(px(0.), cell_h - px(1.5)),
                                size: size(cell_w, px(1.5)),
                            },
                            false,
                        ),
                        CursorShape::HollowBlock => (
                            Bounds {
                                origin,
                                size: size(cell_w, cell_h),
                            },
                            true,
                        ),
                        CursorShape::Block => (
                            Bounds {
                                origin,
                                size: size(cell_w, cell_h),
                            },
                            false,
                        ),
                    };
                    (bounds, hollow)
                });
                TermPrepaint {
                    rows,
                    cursor,
                    origin: bounds.origin,
                    cell_h,
                }
            },
            move |bounds, pre, window, cx| {
                window.paint_quad(fill(bounds, style.default_bg));
                for row in &pre.rows {
                    for (b, color) in &row.bgs {
                        window.paint_quad(fill(*b, *color));
                    }
                }
                for (r, row) in pre.rows.iter().enumerate() {
                    if let Some(line) = &row.line {
                        let _ = line.paint(
                            pre.origin + point(px(0.), pre.cell_h * r as f32),
                            pre.cell_h,
                            TextAlign::Left,
                            None,
                            window,
                            cx,
                        );
                    }
                }
                if let Some((b, hollow)) = pre.cursor {
                    if hollow {
                        let mut q = fill(b, gpui::transparent_black());
                        q.border_widths = px(1.).into();
                        q.border_color = style.cursor;
                        window.paint_quad(q);
                    } else {
                        let mut color = style.cursor;
                        color.a = 0.35;
                        window.paint_quad(fill(b, color));
                    }
                }
            },
        )
        .size_full()
    }
}

/// One grid row → background quads + a shaped line. `cells` is already
/// viewport-clipped; the row top is `row_y`.
#[allow(clippy::too_many_arguments)]
fn shape_row(
    cells: &[CellSnapshot],
    _row_y: Pixels,
    row_x: Pixels,
    cell_w: Pixels,
    cell_h: Pixels,
    def_fg: (u8, u8, u8),
    def_bg: (u8, u8, u8),
    font_size: Pixels,
    font_base: &dyn Fn(FontWeight, FontStyle) -> Font,
    window: &mut Window,
) -> RowPaint {
    // Pass 1 — background quads: consecutive cells sharing a display bg.
    let mut bgs: Vec<(Bounds<Pixels>, Hsla)> = Vec::new();
    let mut bg_start = 0usize;
    let mut bg_color = palette::resolve(
        cells
            .first()
            .map(|c| c.display_colors().1)
            .unwrap_or(crate::emulator::CellColor::Background),
        def_fg,
        def_bg,
    );
    for (i, cell) in cells.iter().enumerate().skip(1) {
        let c = palette::resolve(cell.display_colors().1, def_fg, def_bg);
        if c != bg_color {
            if bg_color != def_bg {
                bgs.push((
                    Bounds {
                        origin: point(row_x + cell_w * bg_start as f32, _row_y),
                        size: size(cell_w * (i - bg_start) as f32, cell_h),
                    },
                    hsl(bg_color),
                ));
            }
            bg_start = i;
            bg_color = c;
        }
    }
    if bg_color != def_bg {
        bgs.push((
            Bounds {
                origin: point(row_x + cell_w * bg_start as f32, _row_y),
                size: size(cell_w * (cells.len() - bg_start) as f32, cell_h),
            },
            hsl(bg_color),
        ));
    }

    // Pass 2 — text: skip spacers and spaces-only rows; group runs by
    // (fg, weight, style, underline) since TextRun carries one Font.
    let mut text = String::with_capacity(cells.len());
    let mut runs: Vec<TextRun> = Vec::new();
    let mut cur: Option<(Hsla, FontWeight, FontStyle, bool, usize)> = None;
    for cell in cells {
        if cell.wide_spacer {
            continue; // covered by the wide glyph's own advance
        }
        let (fg, bg) = cell.display_colors();
        let mut fg_rgb = palette::resolve(fg, def_fg, def_bg);
        if cell.dim {
            fg_rgb = dim(fg_rgb, palette::resolve(bg, def_fg, def_bg));
        }
        let fg_h = hsl(fg_rgb);
        let weight = if cell.bold {
            FontWeight::BOLD
        } else {
            FontWeight::NORMAL
        };
        let style = if cell.italic {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        };
        let ch = if cell.hidden { ' ' } else { cell.ch };
        let wlen = ch.len_utf8();
        match &mut cur {
            Some((cf, cw, cs, cu, len))
                if *cf == fg_h && *cw == weight && *cs == style && *cu == cell.underline =>
            {
                *len += wlen;
            }
            prev => {
                if let Some((cf, cw, cs, cu, len)) = prev.take() {
                    runs.push(run(len, font_base(cw, cs), cf, cu, font_size));
                }
                cur = Some((fg_h, weight, style, cell.underline, wlen));
            }
        }
        text.push(ch);
    }
    if let Some((cf, cw, cs, cu, len)) = cur.take() {
        runs.push(run(len, font_base(cw, cs), cf, cu, font_size));
    }

    let line = if text.trim().is_empty() {
        None
    } else {
        Some(
            window
                .text_system()
                .shape_line(text.into(), font_size, &runs, None),
        )
    };
    RowPaint { line, bgs }
}

fn run(len: usize, font: Font, color: Hsla, underline: bool, _font_size: Pixels) -> TextRun {
    TextRun {
        len,
        font,
        color,
        background_color: None,
        underline: underline.then(|| UnderlineStyle {
            thickness: px(1.),
            color: None,
            wavy: false,
        }),
        strikethrough: None,
    }
}
