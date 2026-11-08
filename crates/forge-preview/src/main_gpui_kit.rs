//! `forge-preview` — an out-of-process preview host for a Forge document.
//!
//! One host per component-system base (#97): Studio spawns the binary for the
//! document's base and talks JSON-RPC-ish lines on stdin/stdout — the base's
//! own GPUI graph renders in this process, so bases on foreign pins (vanilla
//! gpui 0.2, Bezel, adabraka) are each a host binary, not a linked adapter.
//!
//! Protocol (newline-delimited JSON on stdin):
//!   {"cmd":"load","doc":"<path>"}      → renders the document
//!   {"cmd":"source","src":"<yaml>"}    → live-synced source
//!   {"cmd":"pick","on":<bool>}         → element-picker mode
//!   {"cmd":"place","on":<bool>}        → click-to-place mode
//!   {"cmd":"dnd","on":<bool>}          → node drags / drop targets
//!   {"cmd":"select","path":"<path>"}   → mirrors Studio's outline selection
//!   {"cmd":"theme","name":"<name>"}    → applies a registry theme by name
//!   {"cmd":"theme","dark":<bool>}      → light/dark mode with current theme
//!   {"cmd":"raise"}                    → focuses the host window
//!   {"cmd":"quit"}                     → exits
//! stdout replies:
//!   {"event":"ready","base":"<id>"}
//!   {"event":"diagnostics","items":[...]}
//!   {"event":"call","call":"name(args)"}  — a hook fired in the previewed doc
//!   {"event":"state","state":{...}}       — the view state after a write

use gpui_forge_runtime::check;
use gpui_forge_runtime::render::ForgeView;
use gpui_kit::component::theme::{Theme, ThemeMode, ThemeRegistry};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::json;
use std::sync::mpsc;

mod protocol;
use protocol::{HostMsg, emit, spawn_reader};

fn main() {
    // Control channel — stdin JSON lines → the render thread.
    let (tx, rx) = mpsc::channel::<HostMsg>();
    spawn_reader(tx);

    gpui_kit::application()
        .with_assets(gpui_kit::assets::AllAssets)
        .run(move |cx| {
            gpui_kit::init(cx);
            // Same theme catalog as Studio — `theme` commands resolve names
            // against the workspace `themes/` dir.
            let themes_dir =
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../themes");
            if let Ok(entries) = std::fs::read_dir(&themes_dir) {
                for entry in entries.flatten() {
                    if entry.path().extension().and_then(|s| s.to_str()) == Some("json")
                        && let Ok(content) = std::fs::read_to_string(entry.path())
                    {
                        let _ = ThemeRegistry::global_mut(cx).load_themes_from_str(&content);
                    }
                }
            }
            let bounds = Bounds::centered(None, size(px(800.0), px(600.0)), cx);
            gpui_kit::open_window(
                WindowOptions {
                    app_id: Some("forge-preview".into()),
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        title: Some("Forge Preview".into()),
                        appears_transparent: true,
                        traffic_light_position: None,
                    }),
                    ..Default::default()
                },
                cx,
                move |_, cx| {
                    cx.new(move |cx| {
                        let mut view = ForgeView::new(check::catalogs());
                        // Node drags/drop targets match Studio's embedded preview.
                        view.set_dnd(true, cx);
                        // Everything the view would tell an in-process host —
                        // fired hooks and state writes — goes out over stdout so
                        // Studio sees pick/drop/doc events exactly like the
                        // embedded `forge.Preview` sinks deliver them.
                        view.set_sinks(
                            std::rc::Rc::new(|call, _, _| {
                                emit(json!({"event":"call","call": call}));
                            }),
                            std::rc::Rc::new(|state, _, _| {
                                emit(json!({"event":"state","state": state}));
                            }),
                        );
                        // Control channel polled on a timer — doc syncs and
                        // quit commands arrive after window creation too.
                        cx.spawn(async move |view, cx| {
                            loop {
                                cx.background_executor()
                                    .timer(std::time::Duration::from_millis(50))
                                    .await;
                                let mut msgs = Vec::new();
                                while let Ok(m) = rx.try_recv() {
                                    msgs.push(m);
                                }
                                if msgs.is_empty() {
                                    continue;
                                }
                                let mut quit = false;
                                let _ = view.update(cx, |view: &mut ForgeView, cx| {
                                    for msg in msgs.drain(..) {
                                        match msg {
                                            HostMsg::Source { src, dir } => {
                                                if let Some(dir) = dir {
                                                    view.set_base_dir(dir);
                                                }
                                                let catalogs = view.catalogs.clone();
                                                let (doc, diags) = check::check(
                                                    &src,
                                                    gpui_forge_runtime::base::BASE_ID,
                                                    &catalogs,
                                                );
                                                emit(json!({"event":"diagnostics","items":
                                                check::diagnostics_json(&diags)}));
                                                if let Some(doc) = doc {
                                                    view.set_document(doc, cx);
                                                }
                                            }
                                            HostMsg::Pick(on) => view.set_pick(on, cx),
                                            HostMsg::Place(on) => view.set_placing(on, cx),
                                            HostMsg::Dnd(on) => view.set_dnd(on, cx),
                                            HostMsg::Select(path) => view.update_state(
                                                &gpui_forge_runtime::forge_spec::value::parse_path(
                                                    "document.selected",
                                                )
                                                .unwrap(),
                                                serde_json::Value::String(path),
                                                cx,
                                            ),
                                            HostMsg::Theme { name, dark } => {
                                                let config = name.as_deref().and_then(|n| {
                                                    ThemeRegistry::global(cx)
                                                        .themes()
                                                        .get(n)
                                                        .cloned()
                                                });
                                                match config {
                                                    Some(config) => Theme::update(cx, |t| {
                                                        t.apply_config(&config)
                                                    }),
                                                    None => Theme::change(
                                                        if dark {
                                                            ThemeMode::Dark
                                                        } else {
                                                            ThemeMode::Light
                                                        },
                                                        None,
                                                        cx,
                                                    ),
                                                }
                                            }
                                            HostMsg::Raise => {
                                                // Deferred — a window update from
                                                // inside the entity update would
                                                // hit the leased slot.
                                                cx.defer(|cx| {
                                                    for handle in cx.windows() {
                                                        let _ = handle
                                                            .update(cx, |_, window, _| {
                                                                window.activate_window()
                                                            });
                                                    }
                                                });
                                            }
                                            HostMsg::FrameRect { x, y, w, h } => {
                                                // #120 — the X11 escape for
                                                // position: `HasWindowHandle`
                                                // gives the XID, `x11rb` moves it.
                                                cx.defer(move |cx| {
                                                    for handle in cx.windows() {
                                                        let _ =
                                                            handle.update(cx, |_, window, _| {
                                                                window.resize(gpui_kit::size(
                                                                    gpui_kit::px(w as f32),
                                                                    gpui_kit::px(h as f32),
                                                                ));
                                                                #[cfg(target_os = "linux")]
                                                                x11_move(window, x, y);
                                                            });
                                                    }
                                                });
                                            }
                                            HostMsg::Quit => quit = true,
                                        }
                                    }
                                });
                                if quit {
                                    cx.update(|cx| cx.quit());
                                    return;
                                }
                            }
                        })
                        .detach();
                        view
                    })
                },
            )
            .expect("failed to open preview window");
        });
}

/// #120 Phase 1 — the X11 position escape: `PlatformWindow` has no
/// `set_position`, but `HasWindowHandle` gives the XID; `x11rb` moves it.
#[cfg(target_os = "linux")]
fn x11_move(window: &mut gpui_kit::Window, x: f64, y: f64) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{ConfigureWindowAux, ConnectionExt};
    let Ok(handle) = window.window_handle() else {
        return;
    };
    let Some(xid) = (match handle.as_raw() {
        RawWindowHandle::Xcb(h) => Some(h.window.get()),
        RawWindowHandle::Xlib(h) => Some(h.window as u32),
        _ => None,
    }) else {
        return;
    };
    if let Ok((conn, _)) = x11rb::connect(None) {
        let aux = ConfigureWindowAux::new().x(x as i32).y(y as i32);
        let _ = conn.configure_window(xid, &aux);
        let _ = conn.flush();
    }
}
