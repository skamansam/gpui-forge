//! The terminal emulator core: `alacritty_terminal`'s `Term` + vte's ANSI
//! `Processor` wrapped as a pure state machine.
//!
//! Bytes in ([`Emulator::feed`]), styled cell snapshots out
//! ([`Emulator::lines`], [`Emulator::cursor`]). No gpui, no I/O — whoever owns
//! this feeds it a byte stream and paints what it answers. The model is the
//! same split Zed and `bezel-terminal` use: the emulator is a fold over PTY
//! bytes, the view is a painter of the current grid.

use std::sync::{Arc, Mutex};

use alacritty_terminal::{
    event::{Event, EventListener},
    grid::Dimensions,
    index::{Column, Line, Point},
    term::{Config, Term, cell::Flags},
    vte::ansi::{
        Color as AnsiColor, CursorShape as AnsiCursorShape, NamedColor, Processor, Rgb as AnsiRgb,
    },
};

/// Scrollback history kept client-side (lines).
const SCROLLBACK_LINES: usize = 10_000;

/// Viewport dimensions in cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridSize {
    pub cols: u16,
    pub rows: u16,
}

impl GridSize {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols: cols.max(2),
            rows: rows.max(1),
        }
    }
}

impl Dimensions for GridSize {
    fn total_lines(&self) -> usize {
        self.rows as usize
    }
    fn screen_lines(&self) -> usize {
        self.rows as usize
    }
    fn columns(&self) -> usize {
        self.cols as usize
    }
}

/// A cell's paint color, decoupled from any palette: the view resolves these
/// against defaults (default fg/bg, 256-color index, or direct RGB).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellColor {
    /// Default foreground.
    Foreground,
    /// Default background.
    Background,
    /// Indexed color: 0-15 ANSI, 16-231 color cube, 232-255 grayscale ramp.
    Indexed(u8),
    /// Direct 24-bit color.
    Rgb(u8, u8, u8),
}

fn map_color(color: AnsiColor) -> CellColor {
    match color {
        AnsiColor::Spec(AnsiRgb { r, g, b }) => CellColor::Rgb(r, g, b),
        AnsiColor::Indexed(ix) => CellColor::Indexed(ix),
        AnsiColor::Named(named) => {
            let ix = named as usize;
            if ix < 16 {
                return CellColor::Indexed(ix as u8);
            }
            match named {
                NamedColor::Background => CellColor::Background,
                // Dim named colors fold onto their base index; the DIM flag
                // still travels on the cell for paint-time dimming.
                NamedColor::DimBlack
                | NamedColor::DimRed
                | NamedColor::DimGreen
                | NamedColor::DimYellow
                | NamedColor::DimBlue
                | NamedColor::DimMagenta
                | NamedColor::DimCyan
                | NamedColor::DimWhite => {
                    CellColor::Indexed((ix - NamedColor::DimBlack as usize) as u8)
                }
                _ => CellColor::Foreground,
            }
        }
    }
}

/// One rendered cell: char + colors + the flags paint cares about.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellSnapshot {
    pub ch: char,
    pub fg: CellColor,
    pub bg: CellColor,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
    pub hidden: bool,
    /// A double-width char (occupies this cell plus the next spacer cell).
    pub wide: bool,
    /// The spacer half of a wide char — never shaped, only background-painted.
    pub wide_spacer: bool,
}

impl CellSnapshot {
    /// Effective paint colors after INVERSE/HIDDEN resolution.
    pub fn display_colors(&self) -> (CellColor, CellColor) {
        let (fg, bg) = if self.inverse {
            (self.bg, self.fg)
        } else {
            (self.fg, self.bg)
        };
        if self.hidden { (bg, bg) } else { (fg, bg) }
    }
}

/// The cursor, in viewport coordinates; `None` when hidden or scrolled out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CursorSnapshot {
    pub row: usize,
    pub col: usize,
    pub shape: CursorShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorShape {
    Block,
    Beam,
    Underline,
    HollowBlock,
}

/// Events `Term` raises while parsing — replies the program expects written
/// back on the PTY (device-attribute answers, OSC responses).
type Capture = Arc<Mutex<Vec<Vec<u8>>>>;

struct Listener {
    capture: Capture,
}

impl EventListener for Listener {
    fn send_event(&self, event: Event) {
        if let Event::PtyWrite(bytes) = event {
            self.capture.lock().unwrap().push(bytes.into_bytes());
        }
    }
}

/// The emulator: a pure fold of PTY bytes into a renderable grid.
pub struct Emulator {
    term: Term<Listener>,
    parser: Processor,
    capture: Capture,
}

impl Emulator {
    pub fn new(cols: u16, rows: u16) -> Self {
        let capture: Capture = Arc::new(Mutex::new(Vec::new()));
        let config = Config {
            scrolling_history: SCROLLBACK_LINES,
            ..Config::default()
        };
        let term = Term::new(
            config,
            &GridSize::new(cols, rows),
            Listener {
                capture: capture.clone(),
            },
        );
        Self {
            term,
            parser: Processor::new(),
            capture,
        }
    }

    /// Advance the state machine over decoded PTY output. Returns bytes the
    /// terminal wants written back (DSR/DA query responses) — for a headless
    /// byte stream there's nowhere to send them; a PTY host would.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<u8> {
        self.parser.advance(&mut self.term, bytes);
        let mut out = Vec::new();
        for reply in self.capture.lock().unwrap().drain(..) {
            out.extend_from_slice(&reply);
        }
        out
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        if self.cols() == cols as usize && self.rows() == rows as usize {
            return;
        }
        self.term.resize(GridSize::new(cols, rows));
    }

    pub fn cols(&self) -> usize {
        self.term.columns()
    }

    pub fn rows(&self) -> usize {
        self.term.screen_lines()
    }

    /// Rows scrolled out of the viewport, for a future scrollbar.
    pub fn history_lines(&self) -> usize {
        self.term.grid().history_size()
    }

    /// Snapshot one viewport row (0 = top) honoring the scrollback offset.
    pub fn line(&self, viewport_row: usize) -> Vec<CellSnapshot> {
        let offset = self.term.grid().display_offset() as i32;
        let line = Line(viewport_row as i32 - offset);
        let grid = self.term.grid();
        let row = &grid[line];
        (0..self.cols())
            .map(|col| {
                let cell = &row[Column(col)];
                CellSnapshot {
                    ch: cell.c,
                    fg: map_color(cell.fg),
                    bg: map_color(cell.bg),
                    bold: cell.flags.intersects(Flags::BOLD),
                    dim: cell.flags.intersects(Flags::DIM),
                    italic: cell.flags.intersects(Flags::ITALIC),
                    underline: cell.flags.intersects(Flags::ALL_UNDERLINES),
                    inverse: cell.flags.intersects(Flags::INVERSE),
                    hidden: cell.flags.intersects(Flags::HIDDEN),
                    wide: cell.flags.intersects(Flags::WIDE_CHAR),
                    wide_spacer: cell
                        .flags
                        .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER),
                }
            })
            .collect()
    }

    /// All viewport rows, top to bottom.
    pub fn lines(&self) -> Vec<Vec<CellSnapshot>> {
        (0..self.rows()).map(|r| self.line(r)).collect()
    }

    /// Cursor in viewport coordinates; `None` when hidden or scrolled out.
    pub fn cursor(&self) -> Option<CursorSnapshot> {
        let content = self.term.renderable_content();
        let shape = match content.cursor.shape {
            AnsiCursorShape::Hidden => return None,
            AnsiCursorShape::Block => CursorShape::Block,
            AnsiCursorShape::HollowBlock => CursorShape::HollowBlock,
            AnsiCursorShape::Beam => CursorShape::Beam,
            AnsiCursorShape::Underline => CursorShape::Underline,
        };
        let Point { line, column } = content.cursor.point;
        let row = line.0 + self.term.grid().display_offset() as i32;
        if row < 0 || row >= self.rows() as i32 {
            return None;
        }
        Some(CursorSnapshot {
            row: row as usize,
            col: column.0,
            shape,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(line: &[CellSnapshot]) -> String {
        line.iter().map(|c| c.ch).collect()
    }

    #[test]
    fn plain_text_lands() {
        let mut emu = Emulator::new(10, 3);
        emu.feed(b"hello");
        assert_eq!(text(&emu.line(0))[..5], *"hello");
    }

    #[test]
    fn sgr_colors_stick() {
        let mut emu = Emulator::new(10, 3);
        emu.feed(b"\x1b[31;44mhi\x1b[0m");
        let line = emu.line(0);
        assert_eq!(line[0].fg, CellColor::Indexed(1)); // red
        assert_eq!(line[1].bg, CellColor::Indexed(4)); // blue
        // The reset after `hi` leaves trailing cells on defaults.
        assert_eq!(line[5].fg, CellColor::Foreground);
    }

    #[test]
    fn cursor_positioning_writes() {
        let mut emu = Emulator::new(10, 3);
        // CUP: row 2, col 4 (1-based) then 'x'.
        emu.feed(b"\x1b[2;4Hx");
        assert_eq!(emu.line(1)[3].ch, 'x');
        assert_eq!(emu.line(0)[3].ch, ' ');
    }

    #[test]
    fn clear_screen_wipes() {
        let mut emu = Emulator::new(10, 3);
        emu.feed(b"junk\x1b[2J");
        assert!(text(&emu.line(0)).trim().is_empty());
    }

    #[test]
    fn bold_and_underline_flags() {
        let mut emu = Emulator::new(10, 3);
        emu.feed(b"\x1b[1;4mx");
        let cell = emu.line(0)[0];
        assert!(cell.bold && cell.underline);
    }

    #[test]
    fn inverse_swaps_display_colors() {
        let mut emu = Emulator::new(10, 3);
        emu.feed(b"\x1b[7m\x1b[31mx");
        let cell = emu.line(0)[0];
        let (fg, bg) = cell.display_colors();
        assert_eq!(bg, CellColor::Indexed(1));
        assert_eq!(fg, CellColor::Background);
    }

    #[test]
    fn wrap_scrolls_into_history() {
        let mut emu = Emulator::new(5, 2);
        emu.feed(b"aaaaaaaaaaBBBBB"); // two full rows then 5 more
        assert!(emu.history_lines() > 0);
        assert_eq!(emu.line(1)[0].ch, 'B');
    }

    #[test]
    fn crossterm_style_stream_parses() {
        // What ratatui's CrosstermBackend actually emits for a styled draw:
        // position + set colors + text, repeated per run.
        let mut emu = Emulator::new(20, 5);
        emu.feed(b"\x1b[1;1H\x1b[38;5;196m\x1b[48;5;21mERR\x1b[39m\x1b[49m");
        let line = emu.line(0);
        assert_eq!(&text(&line)[..3], "ERR");
        assert_eq!(line[0].fg, CellColor::Indexed(196));
        assert_eq!(line[2].bg, CellColor::Indexed(21));
    }
}
