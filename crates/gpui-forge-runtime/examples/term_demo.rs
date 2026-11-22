//! `cargo run -p gpui-forge-runtime --example term_demo [-- <doc.forge.yaml>]`
//!
//! A resizable window hosting `TermView` fed by a live
//! `forge-preview-ratatui` ANSI stream — the same pipeline Studio's stream
//! card uses, minus Studio. Resize the window to exercise the
//! `frame_rect` → host relayout → emulator resize path. Click and type to
//! forward input (#156).

use std::cell::RefCell;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::rc::Rc;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use forge_term::{Emulator, TermStyle, TermView};
use gpui_kit::prelude::*;
use gpui_kit::*;

struct Frame {
    seq: u64,
    cols: u16,
    rows: u16,
    data: String,
}

/// Field puller for the flat JSON the host emits — `"key":value` on one line.
fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{key}\":");
    let start = line.find(&pat)? + pat.len();
    let rest = line[start..].trim_start();
    if let Some(quoted) = rest.strip_prefix('"') {
        let mut end = 0;
        let mut esc = false;
        for (i, c) in quoted.char_indices() {
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == '"' {
                end = i;
                break;
            }
        }
        return Some(&quoted[..end]);
    }
    let end = rest.find([',', '}']).unwrap_or(rest.len());
    Some(&rest[..end])
}

/// Unescape a JSON string body (the host only emits \" \\ and \uXXXX).
fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                if let Some(c) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    out.push(c);
                }
            }
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

fn key_name(ks: &Keystroke) -> Option<String> {
    if ks.modifiers.control || ks.modifiers.platform {
        return None;
    }
    if ks.key.chars().count() == 1 {
        return Some(ks.key_char.clone().unwrap_or_else(|| ks.key.clone()));
    }
    let name = match ks.key.as_str() {
        "tab" => {
            if ks.modifiers.shift {
                "BackTab"
            } else {
                "Tab"
            }
        }
        "enter" => "Enter",
        "backspace" => "Backspace",
        "space" => " ",
        "up" => "Up",
        "down" => "Down",
        "left" => "Left",
        "right" => "Right",
        _ => return None,
    };
    Some(name.to_string())
}

struct Demo {
    emu: Rc<RefCell<Emulator>>,
    metrics: forge_term::CellMetrics,
    focus: FocusHandle,
    rx: mpsc::Receiver<Frame>,
    host_in: Arc<Mutex<ChildStdin>>,
    last_seq: u64,
    last_rect: Option<String>,
    child: Option<Child>,
}

impl Demo {
    fn send(&self, line: &str) {
        if let Ok(mut stdin) = self.host_in.lock() {
            let _ = writeln!(stdin, "{line}");
            let _ = stdin.flush();
        }
    }
}

impl Drop for Demo {
    fn drop(&mut self) {
        self.send("{\"cmd\":\"quit\"}");
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Render for Demo {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Feed every frame in seq order. A seq regression means a
        // respawned host. A grid change means a relayout. Both get a
        // fresh emulator — sparse diffs only land right on a blank grid.
        // See feed_term_frame in gpui_kit.
        while let Ok(f) = self.rx.try_recv() {
            let mut emu = self.emu.borrow_mut();
            if f.seq < self.last_seq
                || emu.cols() != f.cols as usize
                || emu.rows() != f.rows as usize
            {
                *emu = Emulator::new(f.cols, f.rows);
            }
            self.last_seq = f.seq;
            emu.feed(f.data.as_bytes());
        }

        // frame_rect on viewport change — cols/rows ride the view's own
        // measured cell metrics so the host grid matches the painted grid.
        let size = window.viewport_size();
        let (w, h) = (f64::from(size.width), f64::from(size.height));
        let mut cmd = format!("{{\"cmd\":\"frame_rect\",\"x\":0.0,\"y\":0.0,\"w\":{w},\"h\":{h}");
        if let Some((cw, ch)) = *self.metrics.borrow() {
            let cols = (w / f64::from(cw)).floor().max(2.) as u64;
            let rows = (h / f64::from(ch)).floor().max(2.) as u64;
            cmd.push_str(&format!(",\"cols\":{cols},\"rows\":{rows}"));
        }
        cmd.push('}');
        if self.last_rect.as_deref() != Some(cmd.as_str()) {
            eprintln!("frame_rect: {cmd} metrics={:?}", *self.metrics.borrow());
            self.last_rect = Some(cmd.clone());
            self.send(&cmd);
        }

        let stdin = self.host_in.clone();
        let send_key = move |ev: &KeyDownEvent| {
            if let Some(key) = key_name(&ev.keystroke)
                && let Ok(mut s) = stdin.lock()
            {
                let _ = writeln!(s, "{{\"cmd\":\"key\",\"key\":\"{key}\"}}");
            }
        };
        let stdin = self.host_in.clone();
        let metrics = self.metrics.clone();
        let send_mouse = move |kind: &'static str, pos: gpui::Point<Pixels>| {
            // The view fills the window, so the grid origin is (0,0) — and
            // its own published metrics decide the pitch.
            if let Some((cw, ch)) = *metrics.borrow()
                && let Ok(mut s) = stdin.lock()
            {
                let col = (pos.x / cw) as u64;
                let row = (pos.y / ch) as u64;
                let _ = writeln!(
                    s,
                    "{{\"cmd\":\"mouse\",\"kind\":\"{kind}\",\"col\":{col},\"row\":{row}}}"
                );
            }
        };
        let focus_d = self.focus.clone();
        let down = send_mouse.clone();
        let up = send_mouse.clone();
        let scroll = send_mouse;

        div()
            .size_full()
            .bg(rgb(0x101018))
            .track_focus(&self.focus)
            .on_key_down(move |ev, _w, _cx| send_key(ev))
            .on_mouse_down(MouseButton::Left, move |ev, window, cx| {
                focus_d.focus(window, cx);
                down("down", ev.position);
            })
            .on_mouse_up(MouseButton::Left, move |ev, _w, _cx| up("up", ev.position))
            .on_scroll(move |ev: &ScrollEvent, _w, _cx| {
                let kind = if f32::from(ev.delta.pixel_delta(px(20.)).y) < 0. {
                    "scroll-down"
                } else {
                    "scroll-up"
                };
                scroll(kind, ev.position);
            })
            .child(
                TermView::new(
                    self.emu.clone(),
                    TermStyle {
                        font_family: SharedString::from("monospace"),
                        font_size: px(12.),
                        default_fg: rgb(0xd0d0d0).into(),
                        default_bg: rgb(0x101018).into(),
                        cursor: rgb(0xd0d0d0).into(),
                    },
                )
                .metrics_sink(self.metrics.clone()),
            )
    }
}

fn main() {
    gpui_forge_runtime::ensure_main_stack();
    let doc = std::env::args().nth(1).unwrap_or_else(|| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/demos/ratatui-dashboard.forge.yaml"
        )
        .to_owned()
    });
    let exe = std::env::current_exe().unwrap();
    let host = exe
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("forge-preview-ratatui");
    let mut child = Command::new(&host)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e} — build forge-preview first", host.display()));
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    writeln!(stdin, "{{\"cmd\":\"load\",\"doc\":\"{doc}\"}}").unwrap();
    stdin.flush().unwrap();

    let (tx, rx) = mpsc::channel::<Frame>();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if field(&line, "event") != Some("frame") {
                eprintln!("host: {line}");
                continue;
            }
            let _ = tx.send(Frame {
                seq: field(&line, "seq").and_then(|s| s.parse().ok()).unwrap(),
                cols: field(&line, "cols").and_then(|s| s.parse().ok()).unwrap(),
                rows: field(&line, "rows").and_then(|s| s.parse().ok()).unwrap(),
                data: unescape(field(&line, "data").unwrap_or("")),
            });
        }
    });

    let stdin = Arc::new(Mutex::new(stdin));
    gpui_kit::application()
        .with_assets(gpui_kit::assets::AllAssets)
        .run(move |cx| {
            gpui_kit::init(cx);
            let bounds = Bounds::centered(None, size(px(900.0), px(560.0)), cx);
            gpui_kit::open_window(
                WindowOptions {
                    app_id: Some("forge-term-demo".into()),
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        title: Some("forge-term demo".into()),
                        appears_transparent: true,
                        traffic_light_position: None,
                    }),
                    ..Default::default()
                },
                cx,
                move |_, cx| {
                    cx.new(|cx| {
                        cx.spawn(async move |this: WeakEntity<Demo>, cx| {
                            loop {
                                cx.background_executor()
                                    .timer(Duration::from_millis(50))
                                    .await;
                                if this.update(cx, |_, cx| cx.notify()).is_err() {
                                    break;
                                }
                            }
                        })
                        .detach();
                        Demo {
                            emu: Rc::new(RefCell::new(Emulator::new(80, 24))),
                            metrics: Rc::new(RefCell::new(None)),
                            focus: cx.focus_handle(),
                            rx,
                            host_in: stdin,
                            last_seq: 0,
                            last_rect: None,
                            child: Some(child),
                        }
                    })
                },
            )
            .expect("failed to open window");
        });
}
