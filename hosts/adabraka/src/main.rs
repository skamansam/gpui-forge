//! `forge-preview-adabraka` — the out-of-process preview host for the adabraka
//! base (#95). Lives in a nested workspace (`hosts/adabraka/`) because
//! adabraka-ui's tree-sitter 0.25 `links`-conflicts with gpui-kit's 0.26 —
//! its own Cargo.lock resolves a graph without gpui-kit.
//!
//! Same JSON wire protocol as `forge-preview` (protocol.rs is shared
//! verbatim); Studio spawns this binary for `base: adabraka` documents.

use adabraka_gpui::prelude::*;
use adabraka_gpui::*;
use forge_spec::catalog::Catalog;
use gpui_forge_runtime::check;
use serde_json::json;
use std::sync::mpsc;
use view::ForgeView;

mod protocol;
mod view;
use protocol::{HostMsg, emit, spawn_reader};

/// The adabraka build's catalogs: its own library plus the Forge intrinsics —
/// the nested workspace can't reach `check::catalogs()` (no base feature).
fn catalogs() -> Vec<Catalog> {
    vec![
        forge_spec::catalog::from_yaml(include_str!("../../../catalogs/adabraka.catalog.yaml"))
            .expect("bundled adabraka catalog is valid"),
        forge_spec::catalog::from_yaml(include_str!("../../../catalogs/forge.catalog.yaml"))
            .expect("bundled forge catalog is valid"),
    ]
}

fn main() {
    // Control channel — stdin JSON lines → the render thread.
    let (tx, rx) = mpsc::channel::<HostMsg>();
    spawn_reader(tx);

    Application::new().run(move |cx| {
        let bounds = Bounds::centered(None, size(px(800.0), px(600.0)), cx);
        cx.open_window(
            WindowOptions {
                app_id: Some("forge-preview-adabraka".into()),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Forge Preview · adabraka".into()),
                    appears_transparent: true,
                    traffic_light_position: None,
                }),
                ..Default::default()
            },
            move |_, cx| {
                cx.new(move |cx| {
                    let mut view = ForgeView::new(catalogs());
                    view.set_dnd(true, cx);
                    // Fired hooks and state writes go out over stdout — same
                    // contract as the other hosts.
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
                                            "adabraka",
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
                                        &forge_spec::value::parse_path(
                                            "document.selected",
                                        )
                                        .unwrap(),
                                        serde_json::Value::String(path),
                                        cx,
                                    ),
                                    HostMsg::Theme { name, dark: _ } => {
                                        // adabraka_ui::init installs the theme —
                                        // dark/light + named themes map when the
                                        // bridge lands (#95).
                                        if let Some(name) = &name {
                                            eprintln!(
                                                "forge-preview-adabraka: named theme `{name}` not mapped"
                                            );
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
