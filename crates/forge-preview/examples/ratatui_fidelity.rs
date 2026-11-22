//! `cargo run -p forge-preview --no-default-features --features base-ratatui
//!   --example ratatui_fidelity`
//!
//! Fidelity probe for "render the TUI widgets through a terminal emulator".
//! Two legs share one pipeline — a `CrosstermBackend` ANSI stream folded
//! through `forge-term`'s `Emulator` into a cell grid, dumped as text:
//!
//!   1. Forge ratatui documents → `ForgeView` (what `mode: stream` paints
//!      today) — first the showcase atoms, then every `[x]` catalog widget
//!      including Sparkline/BarChart/Chart/Calendar/Canvas/Logo/Mascot.
//!   2. The same widgets constructed as real `ratatui::widgets` — what the
//!      emulator conveys when the render arm actually emits the widget.

use gpui_forge_runtime::check;
use gpui_forge_runtime::render::ForgeView;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::canvas::{Canvas, Map, MapResolution};
use ratatui::widgets::{
    Axis, BarChart, Block, Borders, Chart, Dataset, Gauge, GraphType, Sparkline,
};
use ratatui::{Frame, Terminal, TerminalOptions, Viewport};
use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

#[derive(Clone)]
struct Sink(Rc<RefCell<Vec<u8>>>);

impl Write for Sink {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Draw into a fixed-viewport `Terminal` whose backend emits ANSI into
/// memory — the same bytes a real terminal (or `mode: stream`) would see.
fn ansi(cols: u16, rows: u16, draw: impl FnOnce(&mut Frame)) -> Vec<u8> {
    let sink = Sink(Rc::new(RefCell::new(Vec::new())));
    let mut term = Terminal::with_options(
        CrosstermBackend::new(sink.clone()),
        TerminalOptions {
            viewport: Viewport::Fixed(Rect::new(0, 0, cols, rows)),
        },
    )
    .expect("terminal");
    // `Terminal::draw` renders + diffs + emits. `Terminal::flush` would diff
    // the *next* (empty) buffer and erase the frame — don't call it.
    term.draw(|f| draw(f)).expect("draw");
    let bytes = sink.0.borrow().clone();
    bytes
}

/// Feed the ANSI stream through the alacritty-backed emulator and print the
/// resulting grid. Glyph text plus a styled-cell count so color fidelity
/// shows even in a text dump.
fn show(cols: u16, rows: u16, label: &str, bytes: &[u8]) {
    let mut emu = forge_term::Emulator::new(cols, rows);
    emu.feed(bytes);
    println!(
        "=== {label} ({cols}x{rows}, {} ANSI bytes) ===",
        bytes.len()
    );
    let mut styled = 0usize;
    for line in emu.lines() {
        let mut s = String::new();
        for cell in &line {
            if cell.wide_spacer {
                continue;
            }
            if cell.fg != forge_term::CellColor::Foreground
                || cell.bg != forge_term::CellColor::Background
                || cell.bold
            {
                styled += 1;
            }
            s.push(if cell.ch == '\0' { ' ' } else { cell.ch });
        }
        println!("|{}", s.trim_end());
    }
    println!("--- styled cells: {styled} ---\n");
}

/// Render a Forge document source through the ratatui `ForgeView` and return
/// the frame's ANSI bytes.
fn forge_doc(cols: u16, rows: u16, src: &str) -> (Vec<u8>, usize) {
    let mut view = ForgeView::new(check::catalogs());
    let (doc, diags) = check::check(src, gpui_forge_runtime::base::BASE_ID, &view.catalogs);
    for d in &diags {
        eprintln!("  diag {}:{}: {}", d.pos.line, d.pos.col, d.message);
    }
    view.set_document(doc.expect("document parses"));
    let diags = diags.len();
    (ansi(cols, rows, |f| view.render(f)), diags)
}

const BASIC: &str = r#"
forge: "0.1"
state:
  q: string
  tab: number
base: ratatui
root:
  VStack:
  children:
    - Text: "atoms showcase"
    - Tabs:
      titles: "One\nTwo\nThree"
      selected: 1
    - HStack:
      children:
        - Block:
          title: "gauge"
          children:
            - Gauge:
              value: 72
        - Block:
          title: "list"
          children:
            - List:
              items: "alpha\nbeta\ngamma"
        - Block:
          title: "input"
          children:
            - VStack:
              children:
                - Input:
                  placeholder: "type here"
                  value: $q
                - Checkbox:
                  checked: true
    - Table:
      title: "table"
      header: "name | status | load"
      rows: "web-1 | up | 0.62\nweb-2 | up | 0.41\ndb-1 | deg | 0.91"
"#;

const EXOTIC: &str = r#"
forge: "0.1"
base: ratatui
root:
  VStack:
  children:
    - HStack:
      children:
        - Block:
          title: "sparkline"
          children:
            - Sparkline:
              data: [1, 5, 2, 8, 3, 9, 4, 7]
        - Block:
          title: "barchart"
          children:
            - BarChart:
              data: [4, 9, 2, 7]
              labels: "a\nb\nc\nd"
    - HStack:
      children:
        - Block:
          title: "chart"
          children:
            - Chart:
              data: [1, 3, 2, 8, 5, 9, 4, 7, 6]
              name: "s"
        - Block:
          title: "calendar"
          children:
            - Calendar:
              date: "2026-11"
    - HStack:
      children:
        - Block:
          title: "canvas"
          children:
            - Canvas:
        - Block:
          title: "logo"
          children:
            - Logo:
              size: small
    - Clear:
"#;

fn main() {
    let (cols, rows) = (112u16, 44u16);

    // Leg 1a — the atoms the ratatui arm does handle.
    let (bytes, diags) = forge_doc(cols, rows, BASIC);
    println!("forge atoms — {diags} diagnostics");
    show(cols, rows, "ForgeView atoms", &bytes);

    // Leg 1b — every `[x]` catalog widget the match arm never names.
    let (bytes, diags) = forge_doc(cols, rows, EXOTIC);
    println!("forge exotic — {diags} diagnostics");
    show(cols, rows, "ForgeView exotic widgets", &bytes);

    // Leg 2 — real ratatui widgets through the same ANSI → emulator pipe.
    let bytes = ansi(cols, rows, |f| {
        let rows_l = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1); 3])
            .split(f.area());
        let top = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Fill(1); 2])
            .split(rows_l[0]);
        f.render_widget(
            Sparkline::default()
                .data(&[1, 5, 2, 8, 3, 9, 4, 7])
                .block(Block::default().borders(Borders::ALL).title("sparkline"))
                .style(Style::default().fg(Color::Yellow)),
            top[0],
        );
        f.render_widget(
            BarChart::default()
                .data(&[("a", 4), ("b", 9), ("c", 2), ("d", 7)])
                .block(Block::default().borders(Borders::ALL).title("barchart"))
                .bar_width(7)
                .bar_style(Style::default().fg(Color::Green)),
            top[1],
        );
        let mid = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Fill(1); 2])
            .split(rows_l[1]);
        let data: Vec<(f64, f64)> = (0..50)
            .map(|i| {
                let x = i as f64 / 5.0;
                (x, (x * 0.8).sin() * 4.0 + 5.0)
            })
            .collect();
        let ds = Dataset::default()
            .name("sin")
            .marker(ratatui::symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(&data);
        f.render_widget(
            Chart::new(vec![ds])
                .block(Block::default().borders(Borders::ALL).title("chart"))
                .x_axis(Axis::default().bounds([0.0, 10.0]))
                .y_axis(Axis::default().bounds([0.0, 10.0])),
            mid[0],
        );
        f.render_widget(
            Canvas::default()
                .block(Block::default().borders(Borders::ALL).title("canvas map"))
                .x_bounds([-180.0, 180.0])
                .y_bounds([-90.0, 90.0])
                .paint(|ctx| {
                    ctx.draw(&Map {
                        resolution: MapResolution::High,
                        color: Color::White,
                    });
                }),
            mid[1],
        );
        f.render_widget(
            Gauge::default()
                .block(Block::default().borders(Borders::ALL).title("gauge"))
                .gauge_style(Style::default().fg(Color::Magenta))
                .percent(72),
            rows_l[2],
        );
    });
    show(cols, rows, "real ratatui widgets", &bytes);
}
