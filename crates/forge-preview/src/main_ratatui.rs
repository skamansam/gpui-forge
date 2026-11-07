//! `forge-preview-ratatui` — the out-of-process preview host for the ratatui
//! base (#112). Same JSON wire protocol as the other hosts, but the render
//! target is a terminal, not a GPUI window: the view draws into a
//! `ratatui::Frame` over `/dev/tty` while stdout stays the protocol channel.

use crossterm::{execute, terminal};
use gpui_forge_runtime::check;
use gpui_forge_runtime::render::ForgeView;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use serde_json::json;
use std::sync::mpsc;

mod protocol;
use protocol::{HostMsg, emit, spawn_reader};

fn main() {
    // Control channel — stdin JSON lines → the render loop.
    let (tx, rx) = mpsc::channel::<HostMsg>();
    spawn_reader(tx);

    // The TUI draws on the controlling terminal — stdout stays protocol-clean.
    let mut out = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/tty")
        .expect("forge-preview-ratatui needs a controlling terminal");
    let _ = terminal::enable_raw_mode();
    let _ = execute!(out, terminal::EnterAlternateScreen);
    let backend = CrosstermBackend::new(out.try_clone().expect("tty dup"));
    let mut term = Terminal::new(backend).expect("terminal init");

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
                        diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>()}));
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
                HostMsg::Theme { name, dark } => {
                    if let Some(name) = &name {
                        eprintln!(
                            "forge-preview-ratatui: named theme `{name}` not mapped (dark={dark})"
                        );
                    }
                }
                HostMsg::Raise => {
                    // The host's terminal is already focused — nothing to raise.
                }
                HostMsg::Quit => quit = true,
            }
        }
        let _ = term.draw(|f| view.render(f));
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    let _ = execute!(out, terminal::LeaveAlternateScreen);
    let _ = terminal::disable_raw_mode();
}
