//! The gpui half of the terminal: `TermView` paints an [`Emulator`]'s grid.
//!
//! The model is Zed's `terminal_view`: per row, background quads for the
//! cells' backgrounds, then one mark per cell — a shaped glyph anchored at
//! the cell origin, or a quad for box-drawing and block cells. The quad
//! carries the cell-box-sized background (a `TextRun::background_color`
//! only covers the glyph's advance, not the whole cell). The cursor is a
//! quad in whatever shape the program asked for.

use std::{cell::RefCell, rc::Rc};

use gpui::{
    App, Bounds, Font, FontFeatures, FontStyle, FontWeight, Hsla, IntoElement, Pixels, ShapedLine,
    SharedString, Styled, TextAlign, TextRun, UnderlineStyle, Window, canvas, fill, point, px, rgb,
    size,
};

use crate::{
    emulator::{CellSnapshot, CursorShape, CursorSnapshot, Emulator},
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

/// Shared slot a [`TermView`] publishes its measured (`cell_w`, `cell_h`)
/// into each prepaint.
pub type CellMetrics = Rc<RefCell<Option<(Pixels, Pixels)>>>;

/// An element that paints an [`Emulator`]'s cell grid. Feed bytes into the
/// emulator wherever the byte stream lands. This re-reads the grid each paint.
pub struct TermView {
    emulator: Rc<RefCell<Emulator>>,
    style: TermStyle,
    metrics: Option<CellMetrics>,
}

impl TermView {
    pub fn new(emulator: Rc<RefCell<Emulator>>, style: TermStyle) -> Self {
        Self {
            emulator,
            style,
            metrics: None,
        }
    }

    /// Publishes the measured cell size (`cell_w`, `cell_h`) each prepaint.
    /// A caller streaming to a fixed terminal grid needs these to turn a px
    /// slot into matching cols/rows. The cell pitch lives here, not in a
    /// protocol constant.
    pub fn metrics_sink(mut self, sink: CellMetrics) -> Self {
        self.metrics = Some(sink);
        self
    }
}

/// An element that paints a caller-owned cell grid — rows of
/// [`CellSnapshot`]s swapped in each frame. Studio's in-process ratatui
/// canvas converts a `ratatui::Buffer` and paints it through the same
/// `shape_row`/quad pipeline [`TermView`] uses (#156).
pub struct GridView {
    rows: Rc<RefCell<Vec<Vec<CellSnapshot>>>>,
    style: TermStyle,
    metrics: Option<CellMetrics>,
}

impl GridView {
    pub fn new(rows: Rc<RefCell<Vec<Vec<CellSnapshot>>>>, style: TermStyle) -> Self {
        Self {
            rows,
            style,
            metrics: None,
        }
    }

    /// Same contract as [`TermView::metrics_sink`] — the cell pitch the grid
    /// paints with, published each prepaint.
    pub fn metrics_sink(mut self, sink: CellMetrics) -> Self {
        self.metrics = Some(sink);
        self
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

/// Centerline half-segments for a box-drawing cell: left, right, up, down
/// from the cell's midpoint to the matching edge.
const SEG_L: u8 = 1;
const SEG_R: u8 = 2;
const SEG_U: u8 = 4;
const SEG_D: u8 = 8;

#[derive(Clone, Copy)]
enum LineWt {
    Light,
    Heavy,
    Double,
}

enum CellInk {
    /// Shaped text painted at the cell's top-left.
    Glyph(Box<ShapedLine>),
    /// `SEG_*` half-segments drawn as quads. Box-drawing glyphs rarely
    /// fill the cell advance, so adjacent line cells leave seams when
    /// painted as text. Segments sized to the cell edge cannot gap.
    Bars(u8, LineWt),
    /// Fractional rects ((x0,y0,x1,y1) in cell units) for block elements
    /// like `█` and `▁`. Same seam argument as `Bars`.
    Fracs([(f32, f32, f32, f32); 2]),
}

struct CellPaint {
    bounds: Bounds<Pixels>,
    fg: Hsla,
    ink: CellInk,
}

pub struct RowPaint {
    /// One entry per non-space cell. Row-wide shaping misaligns whenever a
    /// glyph's advance differs from `cell_w` (font fallback on box-drawing
    /// chars or a proportional resolved font): each substitution
    /// cascade-shifts the row's tail. Per-cell origins keep every glyph on
    /// its column regardless of font.
    spans: Vec<CellPaint>,
    bgs: Vec<(Bounds<Pixels>, Hsla)>,
}

pub struct TermPrepaint {
    rows: Vec<RowPaint>,
    cursor: Option<(Bounds<Pixels>, bool)>, // bounds, hollow?
    cell_h: Pixels,
}

impl IntoElement for TermView {
    type Element = gpui::Canvas<TermPrepaint>;

    fn into_element(self) -> Self::Element {
        let emulator = self.emulator;
        let style = self.style;
        let metrics = self.metrics;
        let family = style.font_family.clone();
        let font_base = move |weight, fstyle| Font {
            family: family.clone(),
            features: FontFeatures::default(),
            fallbacks: None,
            weight,
            style: fstyle,
        };
        let paint_style = style.clone();

        canvas(
            move |bounds, window, cx| {
                let emu = emulator.borrow();
                prepaint_grid(
                    bounds,
                    &emu.lines(),
                    emu.cursor(),
                    &style,
                    &metrics,
                    &font_base,
                    window,
                    cx,
                )
            },
            move |bounds, pre, window, cx| paint_grid(bounds, &pre, &paint_style, window, cx),
        )
        .size_full()
    }
}

impl IntoElement for GridView {
    type Element = gpui::Canvas<TermPrepaint>;

    fn into_element(self) -> Self::Element {
        let rows = self.rows;
        let style = self.style;
        let metrics = self.metrics;
        let family = style.font_family.clone();
        let font_base = move |weight, fstyle| Font {
            family: family.clone(),
            features: FontFeatures::default(),
            fallbacks: None,
            weight,
            style: fstyle,
        };
        let paint_style = style.clone();

        canvas(
            move |bounds, window, cx| {
                let rows = rows.borrow();
                prepaint_grid(
                    bounds, &rows, None, &style, &metrics, &font_base, window, cx,
                )
            },
            move |bounds, pre, window, cx| paint_grid(bounds, &pre, &paint_style, window, cx),
        )
        .size_full()
    }
}

/// Shared prepaint for the grid painters — resolves the cell pitch, shapes
/// every row and maps the optional cursor into paint bounds.
#[allow(clippy::too_many_arguments)]
fn prepaint_grid(
    bounds: Bounds<Pixels>,
    rows: &[Vec<CellSnapshot>],
    cursor: Option<CursorSnapshot>,
    style: &TermStyle,
    metrics: &Option<CellMetrics>,
    font_base: &dyn Fn(FontWeight, FontStyle) -> Font,
    window: &mut Window,
    cx: &mut App,
) -> TermPrepaint {
    let ts = cx.text_system();
    let font_id = ts.resolve_font(&font_base(FontWeight::NORMAL, FontStyle::Normal));
    // '0' is the cell-width probe (alacritty's convention).
    // ' ' has a narrow advance in many fonts and would pack
    // every glyph at a fraction of the grid pitch.
    let cell_w = ts
        .advance(font_id, style.font_size, '0')
        .map(|s| s.width)
        .unwrap_or(style.font_size * 0.6);
    // ascent+descent can resolve below the em square on some
    // fonts. 1.2em is the floor or rows cram and glyphs touch.
    let cell_h = (ts.ascent(font_id, style.font_size) + ts.descent(font_id, style.font_size))
        .max(style.font_size * 1.2);
    if let Some(sink) = metrics {
        *sink.borrow_mut() = Some((cell_w, cell_h));
    }
    let (def_fg, def_bg) = (rgb8(style.default_fg), rgb8(style.default_bg));
    let mut shaped = Vec::with_capacity(rows.len());
    for (r, cells) in rows.iter().enumerate() {
        shaped.push(shape_row(
            cells,
            bounds.origin.y + cell_h * r as f32,
            bounds.origin.x,
            cell_w,
            cell_h,
            def_fg,
            def_bg,
            style.font_size,
            font_base,
            window,
        ));
    }
    let cursor = cursor.map(|c| {
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
        rows: shaped,
        cursor,
        cell_h,
    }
}

/// Shared paint — background fill, per-row bgs, then glyph/bar/frac marks
/// and the cursor overlay.
fn paint_grid(
    bounds: Bounds<Pixels>,
    pre: &TermPrepaint,
    style: &TermStyle,
    window: &mut Window,
    cx: &mut App,
) {
    window.paint_quad(fill(bounds, style.default_bg));
    for row in &pre.rows {
        for (b, color) in &row.bgs {
            window.paint_quad(fill(*b, *color));
        }
    }
    for row in &pre.rows {
        for span in &row.spans {
            match &span.ink {
                CellInk::Glyph(line) => {
                    let _ = line.paint(
                        span.bounds.origin,
                        pre.cell_h,
                        TextAlign::Left,
                        None,
                        window,
                        cx,
                    );
                }
                CellInk::Bars(mask, wt) => {
                    paint_bars(span.bounds, *mask, *wt, span.fg, window);
                }
                CellInk::Fracs(fracs) => {
                    let (b, w, h) = (span.bounds, span.bounds.size.width, span.bounds.size.height);
                    for &(x0, y0, x1, y1) in fracs {
                        if x1 <= x0 || y1 <= y0 {
                            continue;
                        }
                        window.paint_quad(fill(
                            Bounds {
                                origin: point(b.origin.x + w * x0, b.origin.y + h * y0),
                                size: size(w * (x1 - x0), h * (y1 - y0)),
                            },
                            span.fg,
                        ));
                    }
                }
            }
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
}

/// One grid row → background quads + a shaped line. `cells` is already
/// viewport-clipped. The row top is `row_y`.
#[allow(clippy::too_many_arguments)]
fn shape_row(
    cells: &[CellSnapshot],
    row_y: Pixels,
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
                        origin: point(row_x + cell_w * bg_start as f32, row_y),
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
                origin: point(row_x + cell_w * bg_start as f32, row_y),
                size: size(cell_w * (cells.len() - bg_start) as f32, cell_h),
            },
            hsl(bg_color),
        ));
    }

    // Pass 2 — one entry per non-space cell, anchored at the cell's grid
    // origin (see `RowPaint::spans`).
    let mut spans: Vec<CellPaint> = Vec::new();
    for (col, cell) in cells.iter().enumerate() {
        if cell.wide_spacer || cell.hidden || cell.ch == ' ' {
            continue;
        }
        let (fg, bg) = cell.display_colors();
        let mut fg_rgb = palette::resolve(fg, def_fg, def_bg);
        if cell.dim {
            fg_rgb = dim(fg_rgb, palette::resolve(bg, def_fg, def_bg));
        }
        let ch = cell.ch;
        let ink = if let Some((mask, wt)) = box_bars(ch) {
            CellInk::Bars(mask, wt)
        } else if let Some(fracs) = block_fracs(ch) {
            CellInk::Fracs(fracs)
        } else {
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
            let runs = [run(
                ch.len_utf8(),
                font_base(weight, style),
                hsl(fg_rgb),
                cell.underline,
                font_size,
            )];
            CellInk::Glyph(Box::new(window.text_system().shape_line(
                ch.to_string().into(),
                font_size,
                &runs,
                None,
            )))
        };
        spans.push(CellPaint {
            bounds: Bounds {
                origin: point(row_x + cell_w * col as f32, row_y),
                size: size(cell_w, cell_h),
            },
            fg: hsl(fg_rgb),
            ink,
        });
    }

    RowPaint { spans, bgs }
}

/// Box-drawing characters mapped to centerline half-segments. Mixed-weight
/// or dashed variants return `None` and fall back to glyph paint.
fn box_bars(ch: char) -> Option<(u8, LineWt)> {
    const ALL: u8 = SEG_L | SEG_R | SEG_U | SEG_D;
    Some(match ch {
        '─' | '╌' => (SEG_L | SEG_R, LineWt::Light),
        '━' => (SEG_L | SEG_R, LineWt::Heavy),
        '═' => (SEG_L | SEG_R, LineWt::Double),
        '│' | '╎' => (SEG_U | SEG_D, LineWt::Light),
        '┃' => (SEG_U | SEG_D, LineWt::Heavy),
        '║' => (SEG_U | SEG_D, LineWt::Double),
        '┌' | '╭' => (SEG_R | SEG_D, LineWt::Light),
        '┐' | '╮' => (SEG_L | SEG_D, LineWt::Light),
        '└' | '╰' => (SEG_R | SEG_U, LineWt::Light),
        '┘' | '╯' => (SEG_L | SEG_U, LineWt::Light),
        '├' => (SEG_R | SEG_U | SEG_D, LineWt::Light),
        '┤' => (SEG_L | SEG_U | SEG_D, LineWt::Light),
        '┬' => (SEG_L | SEG_R | SEG_D, LineWt::Light),
        '┴' => (SEG_L | SEG_R | SEG_U, LineWt::Light),
        '┼' => (ALL, LineWt::Light),
        '╶' => (SEG_R, LineWt::Light),
        '╴' => (SEG_L, LineWt::Light),
        '╵' => (SEG_U, LineWt::Light),
        '╷' => (SEG_D, LineWt::Light),
        '┏' => (SEG_R | SEG_D, LineWt::Heavy),
        '┓' => (SEG_L | SEG_D, LineWt::Heavy),
        '┗' => (SEG_R | SEG_U, LineWt::Heavy),
        '┛' => (SEG_L | SEG_U, LineWt::Heavy),
        '┣' => (SEG_R | SEG_U | SEG_D, LineWt::Heavy),
        '┫' => (SEG_L | SEG_U | SEG_D, LineWt::Heavy),
        '┳' => (SEG_L | SEG_R | SEG_D, LineWt::Heavy),
        '┻' => (SEG_L | SEG_R | SEG_U, LineWt::Heavy),
        '╋' => (ALL, LineWt::Heavy),
        '╺' => (SEG_R, LineWt::Heavy),
        '╸' => (SEG_L, LineWt::Heavy),
        '╹' => (SEG_U, LineWt::Heavy),
        '╻' => (SEG_D, LineWt::Heavy),
        '╔' => (SEG_R | SEG_D, LineWt::Double),
        '╗' => (SEG_L | SEG_D, LineWt::Double),
        '╚' => (SEG_R | SEG_U, LineWt::Double),
        '╝' => (SEG_L | SEG_U, LineWt::Double),
        '╠' => (SEG_R | SEG_U | SEG_D, LineWt::Double),
        '╣' => (SEG_L | SEG_U | SEG_D, LineWt::Double),
        '╦' => (SEG_L | SEG_R | SEG_D, LineWt::Double),
        '╩' => (SEG_L | SEG_R | SEG_U, LineWt::Double),
        '╬' => (ALL, LineWt::Double),
        _ => return None,
    })
}

/// Block elements (U+2580–U+259F) mapped to fractional cell rects. Shades
/// (`░▒▓`) return `None` — they need a dither, not a fill.
fn block_fracs(ch: char) -> Option<[(f32, f32, f32, f32); 2]> {
    let one = |r| [r, (0., 0., 0., 0.)];
    Some(match ch {
        '▀' => one((0., 0., 1., 0.5)),
        '▁' => one((0., 0.875, 1., 1.)),
        '▂' => one((0., 0.75, 1., 1.)),
        '▃' => one((0., 0.625, 1., 1.)),
        '▄' => one((0., 0.5, 1., 1.)),
        '▅' => one((0., 0.375, 1., 1.)),
        '▆' => one((0., 0.25, 1., 1.)),
        '▇' => one((0., 0.125, 1., 1.)),
        '█' => one((0., 0., 1., 1.)),
        '▉' => one((0., 0., 0.875, 1.)),
        '▊' => one((0., 0., 0.75, 1.)),
        '▋' => one((0., 0., 0.625, 1.)),
        '▌' => one((0., 0., 0.5, 1.)),
        '▍' => one((0., 0., 0.375, 1.)),
        '▎' => one((0., 0., 0.25, 1.)),
        '▏' => one((0., 0., 0.125, 1.)),
        '▐' => one((0.5, 0., 1., 1.)),
        '▔' => one((0., 0., 1., 0.125)),
        '▕' => one((0.875, 0., 1., 1.)),
        '▖' => one((0., 0.5, 0.5, 1.)),
        '▗' => one((0.5, 0.5, 1., 1.)),
        '▘' => one((0., 0., 0.5, 0.5)),
        '▙' => [(0., 0., 0.5, 1.), (0.5, 0.5, 1., 1.)],
        '▚' => [(0., 0., 0.5, 0.5), (0.5, 0.5, 1., 1.)],
        '▛' => [(0., 0., 1., 0.5), (0., 0.5, 0.5, 1.)],
        '▜' => [(0., 0., 1., 0.5), (0.5, 0.5, 1., 1.)],
        '▝' => one((0.5, 0., 1., 0.5)),
        '▞' => [(0.5, 0., 1., 0.5), (0., 0.5, 0.5, 1.)],
        '▟' => [(0.5, 0., 1., 0.5), (0., 0.5, 1., 1.)],
        _ => return None,
    })
}

/// Paints a `SEG_*` mask as edge-touching quads. Double lanes offset along
/// the axis perpendicular to their direction.
fn paint_bars(b: Bounds<Pixels>, mask: u8, wt: LineWt, fg: Hsla, window: &mut Window) {
    let (w, h) = (b.size.width, b.size.height);
    let (cx, cy) = (b.origin.x + w * 0.5, b.origin.y + h * 0.5);
    let t = match wt {
        LineWt::Heavy => h * 0.16,
        _ => px(1.).max(h * 0.06),
    };
    let (dx, dy) = match wt {
        LineWt::Double => (w * 0.22, h * 0.22),
        _ => (px(0.), px(0.)),
    };
    let offs = |d: Pixels| -> Vec<Pixels> {
        if d > px(0.) {
            vec![-d, d]
        } else {
            vec![px(0.)]
        }
    };
    let bar = |x0, y0, x1, y1| Bounds {
        origin: point(x0, y0),
        size: size(x1 - x0, y1 - y0),
    };
    for off in offs(dy) {
        if mask & SEG_L != 0 {
            window.paint_quad(fill(
                bar(b.origin.x, cy - t * 0.5 + off, cx, cy + t * 0.5 + off),
                fg,
            ));
        }
        if mask & SEG_R != 0 {
            window.paint_quad(fill(
                bar(cx, cy - t * 0.5 + off, b.origin.x + w, cy + t * 0.5 + off),
                fg,
            ));
        }
    }
    for off in offs(dx) {
        if mask & SEG_U != 0 {
            window.paint_quad(fill(
                bar(cx - t * 0.5 + off, b.origin.y, cx + t * 0.5 + off, cy),
                fg,
            ));
        }
        if mask & SEG_D != 0 {
            window.paint_quad(fill(
                bar(cx - t * 0.5 + off, cy, cx + t * 0.5 + off, b.origin.y + h),
                fg,
            ));
        }
    }
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
