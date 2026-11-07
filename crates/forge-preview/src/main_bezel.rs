//! `forge-preview-bezel` — the out-of-process preview host for the gpui02 base
//! (crates.io bezel-gpui + bezel-ui). Same wire protocol as `forge-preview`;
//! Studio spawns this binary for `base: gpui02` documents (#96).

use bezel::gpui::prelude::*;
use bezel::gpui::*;
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

    Application::with_platform(bezel::gpui_platform::current_platform(false)).run(move |cx| {
        let bounds = Bounds::centered(None, size(px(800.0), px(600.0)), cx);
        cx.open_window(
            WindowOptions {
                app_id: Some("forge-preview-bezel".into()),
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
                                            diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>()})
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
                                        // bezel's appearance is a global —
                                        // dark/light swap now, named themes
                                        // when the theme bridge lands (#94).
                                        if let Some(name) = &name {
                                            eprintln!(
                                                "forge-preview-bezel: named theme `{name}` not mapped — using mode toggle"
                                            );
                                        }
                                        if dark {
                                            bezel::theme::appearance::init(bezel::theme::appearance::AppearanceMode::Dark, cx);
                                        } else {
                                            bezel::theme::appearance::init(bezel::theme::appearance::AppearanceMode::Light, cx);
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
