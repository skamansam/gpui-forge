//! `forge-preview-ratatui` — the out-of-process preview host for the ratatui
//! base (#112). Same JSON wire protocol as the other hosts, but the render
//! target is a terminal, not a GPUI window: the view draws into a
//! `ratatui::Frame` over `/dev/tty` while stdout stays the protocol channel.

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::{execute, terminal};
use gpui_forge_runtime::check;
use gpui_forge_runtime::render::ForgeView;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::{TerminalOptions, Viewport};
use serde_json::json;
use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;
use std::sync::mpsc;

mod protocol;
use protocol::{HostMsg, emit, spawn_reader};

/// Collects the bytes a `CrosstermBackend` writes. The same ANSI stream a
/// real terminal would get. So it can ride the `frame` event to Studio,
/// where the card feeds it through `forge_term`'s emulator (#120).
#[derive(Clone)]
struct SinkWriter(Rc<RefCell<Vec<u8>>>);

impl Write for SinkWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn main() {
    gpui_forge_runtime::ensure_main_stack();
    // Control channel — stdin JSON lines → the render loop.
    let (tx, rx) = mpsc::channel::<HostMsg>();
    spawn_reader(tx);

    // Input channel — /dev/tty raw bytes → keys + SGR mouse events. Stdin is
    // the protocol, so the terminal input side reads the controlling
    // terminal directly (#112).
    let (etx, erx) = mpsc::channel::<TtyEvent>();
    std::thread::spawn(move || {
        use std::io::Read;
        let Ok(mut tty) = std::fs::File::open("/dev/tty") else {
            return;
        };
        // Mouse reports span reads, so bytes accumulate in `pending` and the
        // parser consumes whole sequences.
        let mut pending: Vec<u8> = Vec::new();
        let mut buf = [0u8; 64];
        while let Ok(n) = tty.read(&mut buf) {
            if n == 0 {
                break;
            }
            pending.extend_from_slice(&buf[..n]);
            while let Some((event, used)) = parse_tty(&pending) {
                pending.drain(..used);
                if let Some(event) = event
                    && etx.send(event).is_err()
                {
                    return;
                }
            }
            // A tail that can't complete (a stray ESC) must not grow forever.
            if pending.len() > 64 {
                pending.clear();
            }
        }
    });

    // The TUI draws on the controlling terminal when one exists — spawned
    // from Studio there usually isn't (stdout is the protocol channel).
    // Fall back to headless: the TestBackend grid below still streams
    // `frame` events to the canvas's `preview.frame`.
    let mut tty = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/tty")
        .ok();
    let mut term = tty.as_mut().and_then(|out| {
        let _ = terminal::enable_raw_mode();
        let _ = execute!(out, terminal::EnterAlternateScreen, EnableMouseCapture);
        out.try_clone()
            .ok()
            .and_then(|dup| Terminal::new(CrosstermBackend::new(dup)).ok())
    });
    if term.is_none() {
        eprintln!("forge-preview-ratatui: no controlling terminal — headless (frame stream only)");
    }
    // Headless grid size — FrameRect commands refine it from the canvas slot.
    let mut frame_size = (100u16, 30u16);
    // The streaming terminal: a CrosstermBackend writing into `sink` —
    // real escape bytes, not a TestBackend symbol dump.
    let sink = Rc::new(RefCell::new(Vec::new()));
    let mut stream_term = Terminal::with_options(
        CrosstermBackend::new(SinkWriter(sink.clone())),
        TerminalOptions {
            viewport: Viewport::Fixed(Rect::new(0, 0, frame_size.0, frame_size.1)),
        },
    )
    .expect("stream terminal");
    let mut frame_seq = 0u64;

    let mut view = ForgeView::new(check::catalogs());
    // Everything the view would tell an in-process host goes out over stdout.
    view.set_sinks(
        std::rc::Rc::new(|call, _| {
            emit(json!({"event":"call","call": call}));
        }),
        std::rc::Rc::new(|state, _| {
            emit(json!({"event":"state","state": state}));
        }),
    );

    let mut quit = false;
    while !quit {
        while let Ok(msg) = rx.try_recv() {
            match msg {
                HostMsg::Source { src, dir } => {
                    if let Some(dir) = dir {
                        view.set_base_dir(dir);
                    }
                    let catalogs = view.catalogs.clone();
                    let (doc, diags) =
                        check::check(&src, gpui_forge_runtime::base::BASE_ID, &catalogs);
                    emit(json!({"event":"diagnostics","items":
                        check::diagnostics_json(&diags)}));
                    if let Some(doc) = doc {
                        view.set_document(doc);
                    }
                }
                HostMsg::Pick(on) => view.set_pick(on),
                HostMsg::Place(on) => view.set_placing(on),
                HostMsg::Dnd(on) => view.set_dnd(on),
                HostMsg::Select(path) => view.update_state(
                    &gpui_forge_runtime::forge_spec::value::parse_path("document.selected")
                        .unwrap(),
                    serde_json::Value::String(path),
                ),
                HostMsg::Theme { dark, colors, .. } => view.set_theme(dark, &colors),
                HostMsg::Raise => {
                    // The host's terminal is already focused — nothing to raise.
                }
                HostMsg::FrameRect {
                    w, h, cols, rows, ..
                } => {
                    // Headless mode has no window to float — but the canvas
                    // slot's px size is a sane cols/rows hint for the grid.
                    // A measured cols/rows beats the 8x18px guess: the
                    // sender's cell metrics decide the terminal pitch.
                    let next = match (cols, rows) {
                        (Some(c), Some(r)) => (c.clamp(2, 400), r.clamp(2, 200)),
                        _ => (
                            (w / 8.0).clamp(20.0, 400.0) as u16,
                            (h / 18.0).clamp(5.0, 200.0) as u16,
                        ),
                    };
                    // The resize must happen here — the loop's
                    // `(cols, rows) != frame_size` check can never trip
                    // headless (`cols` derives from `frame_size`), so the
                    // stream viewport would stay at its initial size while
                    // frames claim the new one.
                    if next != frame_size {
                        frame_size = next;
                        let _ = stream_term.resize(Rect::new(0, 0, next.0, next.1));
                    }
                }
                // Wire input — Studio forwards these when the host has no
                // /dev/tty of its own.
                HostMsg::Key(key) => view.on_key(&key),
                HostMsg::Mouse { kind, col, row } => match kind.as_str() {
                    // Designer DnD — the sender drags over the card; the
                    // view resolves the cell to a doc path. `zone` tells
                    // the sender what a drag started on (#156).
                    "zone" => emit(json!({
                        "event": "zone",
                        "path": view.zone_path_at(col, row).unwrap_or_default(),
                    })),
                    "drop-hover" => emit(json!({
                        "event": "drop_target",
                        "path": view.drop_hover(col, row).unwrap_or_default(),
                    })),
                    "drop" => emit(json!({
                        "event": "drop",
                        "path": view.drop_at(col, row).unwrap_or_default(),
                    })),
                    _ => {
                        if kind == "down" {
                            emit(json!({
                                "event": "zone",
                                "path": view.zone_path_at(col, row).unwrap_or_default(),
                            }));
                        }
                        view.on_mouse(&kind, col, row);
                    }
                },
                HostMsg::Quit => quit = true,
            }
        }
        while let Ok(event) = erx.try_recv() {
            match event {
                TtyEvent::Key(key) => view.on_key(&key),
                TtyEvent::Mouse { kind, col, row } => view.on_mouse(kind, col, row),
            }
        }
        let (cols, rows) = if let Some(term) = &mut term {
            let _ = term.draw(|f| view.render(f));
            term.size()
                .map(|s| (s.width, s.height))
                .unwrap_or(frame_size)
        } else {
            frame_size
        };
        // Stream the frame as the exact ANSI bytes a real terminal gets —
        // Studio emulates them into styled cells in the card (#120).
        if (cols, rows) != frame_size {
            let _ = stream_term.resize(Rect::new(0, 0, cols, rows));
        }
        frame_size = (cols, rows);
        sink.borrow_mut().clear();
        let _ = stream_term.draw(|f| view.render(f));
        frame_seq += 1;
        emit(json!({
            "event": "frame",
            "base": "ratatui",
            "encoding": "ansi",
            "seq": frame_seq,
            "cols": cols,
            "rows": rows,
            "data": String::from_utf8_lossy(&sink.borrow()),
        }));
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    if let Some(mut out) = tty {
        let _ = execute!(out, DisableMouseCapture, terminal::LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

/// One decoded input from the terminal — a logical key name or a mouse
/// event. Mouse `kind` is `down`, `up`, `scroll-up` or `scroll-down`, with
/// `col`/`row` in zero-based cells.
enum TtyEvent {
    Key(String),
    Mouse {
        kind: &'static str,
        col: u16,
        row: u16,
    },
}

/// The next complete input in `buf`, as `(event, bytes consumed)`. `None`
/// outer means the buffer ends mid-sequence — wait for more bytes. A `None`
/// event with a count is junk to drop. SGR mouse reports arrive as
/// `ESC [ < b ; x ; y` closed by `M` (press) or `m` (release).
fn parse_tty(buf: &[u8]) -> Option<(Option<TtyEvent>, usize)> {
    let b = *buf.first()?;
    Some(match b {
        b'\t' => (Some(TtyEvent::Key("Tab".to_owned())), 1),
        127 | 8 => (Some(TtyEvent::Key("Backspace".to_owned())), 1),
        b'\r' | b'\n' => (Some(TtyEvent::Key("Enter".to_owned())), 1),
        27 => {
            if buf.len() < 2 {
                return None;
            }
            if buf[1] != b'[' {
                return Some((None, 1)); // a lone ESC
            }
            if buf.len() < 3 {
                return None;
            }
            if buf[2] == b'<' {
                let Some(rel) = buf.iter().skip(3).position(|&c| c == b'M' || c == b'm') else {
                    return None;
                };
                let i = 3 + rel;
                let mut parts = std::str::from_utf8(&buf[3..i])
                    .unwrap_or("")
                    .split(';')
                    .map(|s| s.parse::<u16>().unwrap_or(0));
                let cb = parts.next().unwrap_or(0);
                let col = parts.next().unwrap_or(1).saturating_sub(1);
                let row = parts.next().unwrap_or(1).saturating_sub(1);
                if buf[i] != b'm' && cb & 32 != 0 {
                    return Some((None, i + 1)); // motion tracking — drop
                }
                let kind = if buf[i] == b'm' {
                    "up"
                } else if cb & 64 != 0 {
                    if cb & 1 == 0 {
                        "scroll-up"
                    } else {
                        "scroll-down"
                    }
                } else {
                    "down"
                };
                (Some(TtyEvent::Mouse { kind, col, row }), i + 1)
            } else {
                match buf[2] {
                    // CSI sequences — arrows (ESC [ A..D) + shift-tab (ESC [ Z).
                    b'A' => (Some(TtyEvent::Key("Up".to_owned())), 3),
                    b'B' => (Some(TtyEvent::Key("Down".to_owned())), 3),
                    b'C' => (Some(TtyEvent::Key("Right".to_owned())), 3),
                    b'D' => (Some(TtyEvent::Key("Left".to_owned())), 3),
                    b'Z' => (Some(TtyEvent::Key("BackTab".to_owned())), 3),
                    _ => (None, 2),
                }
            }
        }
        32..=126 => (Some(TtyEvent::Key((b as char).to_string())), 1),
        _ => (None, 1),
    })
}
