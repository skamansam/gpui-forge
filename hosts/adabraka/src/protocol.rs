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
        emit(json!({"event":"ready","base": "adabraka"}));
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
                    return;
                }
                _ => {}
            }
        }
    });
}

/// `a.b[0]` path read/write on serde_json::Value — the same helpers the
/// in-tree twins import from `render/mod.rs` (cfg'd away without a base).
pub mod paths {
    use forge_spec::{Path, PathSegment};
    use serde_json::Value;

    pub fn get_path<'v>(state: &'v Value, path: &Path) -> Option<&'v Value> {
        let mut cur = state;
        for seg in &path.0 {
            match seg {
                PathSegment::Field(f) => cur = cur.get(f)?,
                PathSegment::Index(i) => cur = cur.get(*i)?,
            }
        }
        Some(cur)
    }

    pub fn set_path(target: &mut Value, path: &Path, value: Value) {
        let mut cur = target;
        for (i, seg) in path.0.iter().enumerate() {
            let last = i == path.0.len() - 1;
            match seg {
                PathSegment::Field(f) => {
                    let entry = cur
                        .as_object_mut()
                        .map(|m| m.entry(f.clone()).or_insert(Value::Null));
                    let Some(node) = entry else { return };
                    if last {
                        *node = value;
                        return;
                    }
                    cur = node;
                }
                PathSegment::Index(i) => {
                    let idx = *i;
                    let arr = cur.as_array_mut().map(|a| {
                        while a.len() <= idx {
                            a.push(Value::Null);
                        }
                        &mut a[idx]
                    });
                    let Some(node) = arr else { return };
                    if last {
                        *node = value;
                        return;
                    }
                    cur = node;
                }
            }
        }
    }
}
