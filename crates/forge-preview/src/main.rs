//! `forge-preview` — an out-of-process preview host for a Forge document.
//!
//! One host per component-system base (#97): Studio spawns the binary for the
//! document's base and talks JSON-RPC-ish lines on stdin/stdout — the base's
//! own GPUI graph renders in this process, so bases on foreign pins (vanilla
//! gpui 0.2, Bezel, adabraka) are each a host binary, not a linked adapter.
//!
//! Protocol (newline-delimited JSON on stdin):
//!   {"cmd":"load","doc":"<path>"}   → renders the document
//!   {"cmd":"source","src":"<yaml>"} → live-synced source
//!   {"cmd":"quit"}                  → exits
//! stdout replies:
//!   {"event":"ready","base":"<id>"}
//!   {"event":"diagnostics","items":[...]}
//!   {"event":"pick","path":"<node-path>"}

use gpui_forge_runtime::check;
use gpui_forge_runtime::render::ForgeView;
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::json;
use std::io::{BufRead, Write};
use std::sync::mpsc;

enum HostMsg {
    Source(String),
    Quit,
}

fn main() {
    // Control channel — stdin JSON lines → the render thread.
    let (tx, rx) = mpsc::channel::<HostMsg>();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut out = std::io::stdout();
        let _ = writeln!(
            out,
            "{}",
            json!({"event":"ready","base": gpui_forge_runtime::base::BASE_ID})
        );
        let _ = out.flush();
        for line in stdin.lock().lines().map_while(Result::ok) {
            let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
                continue;
            };
            match v["cmd"].as_str() {
                Some("load") => {
                    if let Some(p) = v["doc"].as_str() {
                        if let Ok(src) = std::fs::read_to_string(p) {
                            let _ = tx.send(HostMsg::Source(src));
                        }
                    }
                }
                Some("source") => {
                    if let Some(src) = v["src"].as_str() {
                        let _ = tx.send(HostMsg::Source(src.to_string()));
                    }
                }
                Some("quit") => {
                    let _ = tx.send(HostMsg::Quit);
                    return;
                }
                _ => {}
            }
        }
    });

    gpui_kit::application()
        .with_assets(gpui_kit::assets::AllAssets)
        .run(move |cx| {
            gpui_kit::init(cx);
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
                        // Control channel polled on a timer — doc syncs and
                        // quit commands arrive after window creation too.
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
                                        HostMsg::Source(src) => {
                                            let catalogs = view.catalogs.clone();
                                            let (doc, diags) = check::check(
                                                &src,
                                                gpui_forge_runtime::base::BASE_ID,
                                                &catalogs,
                                            );
                                            let mut out = std::io::stdout();
                                            let _ = writeln!(
                                                out,
                                                "{}",
                                                json!({"event":"diagnostics","items":
                                                    diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>()})
                                            );
                                            let _ = out.flush();
                                            if let Some(doc) = doc {
                                                view.set_document(doc, cx);
                                            }
                                        }
                                        HostMsg::Quit => quit = true,
                                    }
                                }
                            });
                            if quit {
                                cx.update(|cx| cx.quit());
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
