//! `forge-preview-gpui02` — the out-of-process preview host for the gpui02 base
//! (crates.io gpui 0.2.2 + guise). Same wire protocol as `forge-preview`.
//! Studio spawns this binary for `base: gpui02` documents (#96).

use gpui::prelude::*;
use gpui::*;
use gpui_forge_runtime::check;
use gpui_forge_runtime::render::ForgeView;
use serde_json::json;
use std::sync::mpsc;

mod protocol;
use protocol::{HostMsg, emit, spawn_reader};

fn main() {
    // Control channel — stdin JSON lines → the render thread.
    let (tx, rx) = mpsc::channel::<HostMsg>();
    spawn_reader(tx);

    Application::new().run(move |cx| {
        let bounds = Bounds::centered(None, size(px(800.0), px(600.0)), cx);
        cx.open_window(
            WindowOptions {
                app_id: Some("forge-preview-gpui02".into()),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Forge Preview · gpui02".into()),
                    appears_transparent: true,
                    traffic_light_position: None,
                }),
                ..Default::default()
            },
            move |_, cx| {
                cx.new(move |cx| {
                    let mut view = ForgeView::new(check::catalogs());
                    // gpui02-base adapters (#86-88) — namespaced elements on
                    // foreign-base docs render through these.
                    view.register_library(forge_lib_gpui02::Ferrum::new());
                    view.register_library(forge_lib_gpui02::Rhythm);
                    view.register_library(forge_lib_gpui02::Symbols);
                    view.set_dnd(true, cx);
                    // Fired hooks and state writes go out over stdout — same
                    // contract as the gpui-kit host.
                    view.set_sinks(
                        std::rc::Rc::new(|call, _, _| {
                            emit(json!({"event":"call","call": call}));
                        }),
                        std::rc::Rc::new(|state, _, _| {
                            emit(json!({"event":"state","state": state}));
                        }),
                    );
                    cx.spawn(async move |view, cx| loop {
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
                                            check::diagnostics_json(&diags)})
                                        );
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
                                        // guise installs its Theme as a global —
                                        // dark/light swap now, named themes
                                        // when the theme bridge lands (#96).
                                        if let Some(name) = &name {
                                            eprintln!(
                                                "forge-preview-gpui02: named theme `{name}` not mapped — using mode toggle"
                                            );
                                        }
                                        if dark {
                                            guise::theme::Theme::dark().init(cx);
                                        } else {
                                            guise::theme::Theme::light().init(cx);
                                        }
                                    }
                                    HostMsg::Raise => {
                                        cx.defer(|cx| {
                                            for handle in cx.windows() {
                                                let _ = handle.update(
                                                    cx,
                                                    |_, window, _| {
                                                        window.activate_window()
                                                    },
                                                );
                                            }
                                        });
                                    }
                                    HostMsg::FrameRect { x, y, w, h } => {
                                        // #120 Phase 1 — `PlatformWindow` only
                                        // exposes `resize`. Size the host
                                        // window to the canvas slot.
                                        cx.defer(move |cx| {
                                            for handle in cx.windows() {
                                                let _ = handle.update(
                                                    cx,
                                                    |_, window, _| {
                                                        window.resize(gpui::size(
                                                            gpui::px(w as f32),
                                                            gpui::px(h as f32),
                                                        ));
                                                        #[cfg(target_os = "linux")]
                                                        x11_move(window, x, y);
                                                    },
                                                );
                                            }
                                        });
                                    }
                                    HostMsg::Quit => quit = true,
                                }
                            }
                        });
                        if quit {
                            let _ = cx.update(|cx| cx.quit());
                            return;
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

/// #120 — the X11 position escape (PlatformWindow only has `resize`).
#[cfg(target_os = "linux")]
fn x11_move(window: &mut gpui::Window, x: f64, y: f64) {
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
