//! `forge-preview-ratatui` — the out-of-process preview host for the ratatui
//! base (#112). Same JSON wire protocol as the other hosts, but the render
//! target is a terminal, not a GPUI window: the view draws into a
//! `ratatui::Frame` over `/dev/tty` while stdout stays the protocol channel.

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

/// Collects the bytes a `CrosstermBackend` writes — the same ANSI stream a
/// real terminal would get — so it can ride the `frame` event to Studio,
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
    // Control channel — stdin JSON lines → the render loop.
    let (tx, rx) = mpsc::channel::<HostMsg>();
    spawn_reader(tx);

    // Key channel — /dev/tty raw bytes → keys. stdin is the protocol, so the
    // terminal input side reads the controlling terminal directly (#112).
    let (ktx, krx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        use std::io::Read;
        let Ok(mut tty) = std::fs::File::open("/dev/tty") else {
            return;
        };
        let mut buf = [0u8; 8];
        while let Ok(n) = tty.read(&mut buf) {
            if n == 0 {
                break;
            }
            let mut i = 0;
            while i < n {
                let b = buf[i];
                let key = match b {
                    b'\t' => "Tab".to_owned(),
                    127 | 8 => "Backspace".to_owned(),
                    b'\r' | b'\n' => "Enter".to_owned(),
                    27 => {
                        // CSI sequences — arrows (ESC [ A..D) + shift-tab (ESC [ Z).
                        if i + 2 < n && buf[i + 1] == b'[' {
                            i += 2;
                            match buf[i] {
                                b'A' => "Up",
                                b'B' => "Down",
                                b'C' => "Right",
                                b'D' => "Left",
                                b'Z' => "BackTab",
                                _ => continue,
                            }
                            .to_owned()
                        } else {
                            i += 1;
                            continue;
                        }
                    }
                    32..=126 => (b as char).to_string(),
                    _ => {
                        i += 1;
                        continue;
                    }
                };
                i += 1;
                if ktx.send(key).is_err() {
                    return;
                }
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
        let _ = execute!(out, terminal::EnterAlternateScreen);
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
                HostMsg::Theme { .. } => {
                    // ratatui widgets style themselves per-widget in the Forge
                    // base renderer — there's no palette hook to map onto. The
                    // streamed ANSI frame shows the TUI's own colors as a real
                    // terminal would, which is the honest render.
                }
                HostMsg::Raise => {
                    // The host's terminal is already focused — nothing to raise.
                }
                HostMsg::FrameRect { w, h, .. } => {
                    // Headless mode has no window to float — but the canvas
                    // slot's px size is a sane cols/rows hint for the grid.
                    frame_size = (
                        (w / 8.0).clamp(20.0, 400.0) as u16,
                        (h / 18.0).clamp(5.0, 200.0) as u16,
                    );
                }
                HostMsg::Quit => quit = true,
            }
        }
        while let Ok(key) = krx.try_recv() {
            // Shift-Tab arrives as CSI Z (27 91 90) — handled as escape for now.
            // Plain Tab already cycles forward.
            view.on_key(&key);
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
        let _ = execute!(out, terminal::LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}
