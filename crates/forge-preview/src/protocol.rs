//! Shared wire protocol for every `forge-preview-<base>` host — newline-delimited
//! JSON on stdin/stdout, identical regardless of which GPUI graph renders.
//! Each host main maps `HostMsg` onto its own view/theme/window calls.

use serde_json::json;
use std::io::{BufRead, Write};
use std::sync::mpsc;

/// Commands Studio sends the host (see each binary's doc comment for the wire
/// shape).
pub enum HostMsg {
    Source {
        src: String,
        dir: Option<std::path::PathBuf>,
    },
    Pick(bool),
    Place(bool),
    Dnd(bool),
    Select(String),
    Theme {
        name: Option<String>,
        dark: bool,
    },
    Raise,
    /// #120 Phase 1 — the canvas slot's screen-space bounds; the host
    /// floats its window over them (fake-embed).
    #[allow(dead_code)]
    FrameRect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    },
    Quit,
}

/// One JSON line on stdout — each `writeln!` locks stdout once, so lines from
/// the reader thread and the UI thread never interleave.
pub fn emit(value: serde_json::Value) {
    let mut out = std::io::stdout();
    let _ = writeln!(out, "{value}");
    let _ = out.flush();
}

/// Spawns the stdin reader thread: `ready` first, then JSON lines → `HostMsg`
/// on the channel (a host polls the channel on its render thread).
pub fn spawn_reader(tx: mpsc::Sender<HostMsg>) {
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        emit(json!({"event":"ready","base": gpui_forge_runtime::base::BASE_ID}));
        for line in stdin.lock().lines().map_while(Result::ok) {
            let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
                continue;
            };
            let on = v["on"].as_bool().unwrap_or(true);
            match v["cmd"].as_str() {
                Some("load") => {
                    if let Some(p) = v["doc"].as_str()
                        && let Ok(src) = std::fs::read_to_string(p)
                    {
                        let dir = std::path::Path::new(p).parent().map(|d| d.to_path_buf());
                        let _ = tx.send(HostMsg::Source { src, dir });
                    }
                }
                Some("source") => {
                    if let Some(src) = v["src"].as_str() {
                        let dir = v["dir"].as_str().map(std::path::PathBuf::from);
                        let _ = tx.send(HostMsg::Source {
                            src: src.to_string(),
                            dir,
                        });
                    }
                }
                Some("pick") => {
                    let _ = tx.send(HostMsg::Pick(on));
                }
                Some("place") => {
                    let _ = tx.send(HostMsg::Place(on));
                }
                Some("dnd") => {
                    let _ = tx.send(HostMsg::Dnd(on));
                }
                Some("select") => {
                    if let Some(path) = v["path"].as_str() {
                        let _ = tx.send(HostMsg::Select(path.to_string()));
                    }
                }
                Some("theme") => {
                    let _ = tx.send(HostMsg::Theme {
                        name: v["name"].as_str().map(str::to_owned),
                        dark: v["dark"].as_bool().unwrap_or_default(),
                    });
                }
                Some("raise") => {
                    let _ = tx.send(HostMsg::Raise);
                }
                Some("quit") => {
                    let _ = tx.send(HostMsg::Quit);
                }
                Some("frame_rect") => {
                    let _ = tx.send(HostMsg::FrameRect {
                        x: v["x"].as_f64().unwrap_or(0.0),
                        y: v["y"].as_f64().unwrap_or(0.0),
                        w: v["w"].as_f64().unwrap_or(0.0),
                        h: v["h"].as_f64().unwrap_or(0.0),
                    });
                }
                _ => {}
            }
        }
    });
}
