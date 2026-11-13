//! `cargo run -p gpui-forge-runtime --example render -- <doc.forge.yaml> [key=value]…`
//!
//! Extra `key=value` args write `studio.tab=4`-style state after the document
//! loads — handy for forcing a selected tab. `FORGE_BOUNDS=1` logs every
//! element's laid-out bounds to stderr for layout debugging.
//!
//! Renders a Forge document in a window (#21) — the smallest complete host:
//! parse + check, `ForgeView` inside a gpui-kit window, hooks logged to stderr.

use gpui_forge_runtime::check;
use gpui_forge_runtime::render::ForgeView;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::rc::Rc;

fn main() {
    gpui_forge_runtime::ensure_main_stack();
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: render <doc.forge.yaml>");
        std::process::exit(2);
    };
    let src = std::fs::read_to_string(&path).expect("cannot read document");
    let dir = std::path::Path::new(&path)
        .parent()
        .map(|p| p.to_path_buf());
    gpui_kit::application()
        .with_assets(gpui_kit::assets::AllAssets)
        .run(move |cx| {
            gpui_kit::init(cx);
            let bounds = Bounds::centered(None, size(px(800.0), px(600.0)), cx);
            let title = format!("Forge — {path}");
            gpui_kit::open_window(
                WindowOptions {
                    app_id: Some("forge-render".into()),
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        title: Some(title.into()),
                        appears_transparent: true,
                        traffic_light_position: None,
                    }),
                    ..Default::default()
                },
                cx,
                move |_, cx| {
                    cx.new(move |cx| {
                        let mut view = ForgeView::new(check::catalogs());
                        view.set_sinks(
                            Rc::new(|call, _, _| eprintln!("hook: {call}")),
                            Rc::new(|state, _, _| {
                                eprintln!(
                                    "state: {}",
                                    state.to_string().chars().take(200).collect::<String>()
                                )
                            }),
                        );
                        let (doc, diagnostics) =
                            check::check(&src, gpui_forge_runtime::base::BASE_ID, &view.catalogs);
                        for d in &diagnostics {
                            eprintln!("{path}:{}:{}: {}", d.pos.line, d.pos.col, d.message);
                        }
                        if let Some(doc) = doc {
                            view.set_document(doc, cx);
                        }
                        for arg in std::env::args().skip(2) {
                            if let Some((k, v)) = arg.split_once('=') {
                                let value = v
                                    .parse::<f64>()
                                    .map(serde_json::Value::from)
                                    .unwrap_or_else(|_| serde_json::Value::from(v));
                                let path = gpui_forge_runtime::forge_spec::model::Path(
                                    k.split('.')
                                        .map(|seg| {
                                            gpui_forge_runtime::forge_spec::model::PathSegment::Field(
                                                seg.to_string(),
                                            )
                                        })
                                        .collect(),
                                );
                                view.update_state(&path, value, cx);
                            }
                        }
                        if let Some(dir) = dir.clone() {
                            view.set_base_dir(dir);
                        }
                        view
                    })
                },
            )
            .expect("failed to open window");
        });
}
