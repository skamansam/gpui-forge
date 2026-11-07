//! `cargo run -p gpui-forge-runtime --no-default-features --features base-gpui02
//!   --example render_gpui02 -- <doc.forge.yaml>`
//!
//! Renders a Forge document on the gpui02 base — crates.io gpui 0.2.2 + guise —
//! the second-base proof: same ForgeView contract, a completely different GPUI
//! graph behind it (#96).

use gpui::prelude::*;
use gpui::{
    AppContext, Application, Bounds, Context, Render, Window, WindowBounds, WindowOptions, div, px,
    size,
};
use gpui_forge_runtime::check;
use gpui_forge_runtime::render::ForgeView;

struct Host {
    view: gpui::Entity<ForgeView>,
}

impl Render for Host {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.view.clone())
    }
}

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: render_gpui02 <doc.forge.yaml>");
        std::process::exit(2);
    };
    let src = std::fs::read_to_string(&path).expect("cannot read document");
    let dir = std::path::Path::new(&path)
        .parent()
        .map(|p| p.to_path_buf());
    Application::new().run(move |cx| {
        let bounds = Bounds::centered(None, size(px(800.0), px(600.0)), cx);
        let title = format!("Forge (gpui02 · guise) — {path}");
        cx.open_window(
            WindowOptions {
                app_id: Some("forge-render-gpui02".into()),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some(title.into()),
                    appears_transparent: true,
                    traffic_light_position: None,
                }),
                ..Default::default()
            },
            move |_, cx| {
                let view = cx.new(|cx| {
                    let mut view = ForgeView::new(check::catalogs());
                    let (doc, diagnostics) =
                        check::check(&src, gpui_forge_runtime::base::BASE_ID, &view.catalogs);
                    for d in &diagnostics {
                        eprintln!("{path}:{}:{}: {}", d.pos.line, d.pos.col, d.message);
                    }
                    if let Some(doc) = doc {
                        view.set_document(doc, cx);
                    }
                    if let Some(dir) = dir.clone() {
                        view.set_base_dir(dir);
                    }
                    view
                });
                cx.new(|_| Host { view })
            },
        )
        .expect("failed to open window");
    });
}
