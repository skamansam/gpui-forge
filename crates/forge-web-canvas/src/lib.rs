//! Renders Forge documents with gpui-kit inside a browser canvas.
//!
//! The page drives the canvas through the exported functions; the canvas reports fired
//! hooks and state changes back through `globalThis.forgeHook` and `globalThis.forgeState`.

#![recursion_limit = "256"]

pub mod check;
pub mod render;

use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;

use forge_spec::catalog::Catalog;
use gpui_kit::component::theme::{Theme, ThemeMode};
use gpui_kit::{prelude::*, *};
use render::ForgeView;
use wasm_bindgen::prelude::*;

thread_local! {
    static APPLICATION: RefCell<Option<ApplicationHandle>> = const { RefCell::new(None) };
    static VIEW: RefCell<Option<Entity<ForgeView>>> = const { RefCell::new(None) };
    static CATALOGS: Vec<Catalog> = check::catalogs();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = globalThis, js_name = forgeHook)]
    fn forge_hook(call: &str);
    #[wasm_bindgen(js_namespace = globalThis, js_name = forgeState)]
    fn forge_state(json: &str);
}

#[cfg(not(target_family = "wasm"))]
fn forge_hook(_: &str) {}
#[cfg(not(target_family = "wasm"))]
fn forge_state(_: &str) {}

/// Starts the canvas. `assets` is the URL that serves `assets/icons/*.svg`.
#[wasm_bindgen]
pub fn start(dark: bool, assets: String) {
    console_error_panic_hook::set_once();
    #[cfg(target_family = "wasm")]
    gpui_kit::platform::web_init();
    #[cfg(target_family = "wasm")]
    let app = Application::with_platform(Rc::new(gpui_kit::web::WebPlatform::new(false)));
    #[cfg(not(target_family = "wasm"))]
    let app = gpui_kit::application();
    let app = app.with_assets(gpui_kit::assets::Assets::new(assets));
    let launch = move |cx: &mut App| {
        gpui_kit::init(cx);
        // The web platform has no system fonts, and resolves `.SystemUIFont` to IBM Plex Sans.
        cx.text_system()
            .add_fonts(vec![
                Cow::Borrowed(include_bytes!("../fonts/Inter-Regular.ttf").as_slice()),
                Cow::Borrowed(include_bytes!("../fonts/IBMPlexSans-Regular.ttf").as_slice()),
                Cow::Borrowed(include_bytes!("../fonts/JetBrainsMono-Regular.ttf").as_slice()),
            ])
            .expect("bundled fonts load");
        apply_theme(dark, cx);
        let (_, view) = gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| {
                let mut view = ForgeView::new(CATALOGS.with(Clone::clone));
                view.set_sinks(
                    Rc::new(forge_hook),
                    Rc::new(|state| forge_state(&state.to_string())),
                );
                view
            })
        })
        .expect("canvas window opens");
        VIEW.with(|v| *v.borrow_mut() = Some(view));
        cx.activate(true);
    };
    #[cfg(target_family = "wasm")]
    APPLICATION.with(|a| *a.borrow_mut() = Some(app.run_embedded(launch)));
    #[cfg(not(target_family = "wasm"))]
    app.run(launch);
}

fn apply_theme(dark: bool, cx: &mut App) {
    let mode = if dark {
        ThemeMode::Dark
    } else {
        ThemeMode::Light
    };
    Theme::change(mode, None, cx);
    Theme::global_mut(cx).font_family = "Inter".into();
    Theme::global_mut(cx).mono_font_family = "JetBrains Mono".into();
}

fn with_view(f: impl FnOnce(&Entity<ForgeView>, &mut App)) {
    APPLICATION.with(|a| {
        if let Some(app) = a.borrow().as_ref() {
            app.update(|cx| {
                if let Some(view) = VIEW.with(|v| v.borrow().clone()) {
                    f(&view, cx);
                }
            });
        }
    });
}

/// Reads, validates and (when it parses) renders `yaml`. Returns the diagnostics as JSON.
#[wasm_bindgen]
pub fn render(yaml: &str) -> String {
    let (doc, diagnostics) = CATALOGS.with(|c| check::check(yaml, c));
    if let Some(doc) = doc {
        with_view(|view, cx| view.update(cx, |view, cx| view.set_document(doc, cx)));
    }
    check::to_json(&diagnostics)
}

/// Validates `yaml` without rendering it. Returns the diagnostics as JSON.
#[wasm_bindgen]
pub fn diagnostics(yaml: &str) -> String {
    CATALOGS.with(|c| check::to_json(&check::check(yaml, c).1))
}

/// Replaces the view state that bindings read from and write to.
#[wasm_bindgen]
pub fn set_state(json: &str) -> Result<(), JsValue> {
    let state = serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    with_view(|view, cx| view.update(cx, |view, cx| view.set_state(state, cx)));
    Ok(())
}

/// Redraws the canvas. GPUI only repaints on change, so the page calls this after a render
/// to pick up icons that gpui-kit's web assets finish downloading later.
#[wasm_bindgen]
pub fn refresh() {
    with_view(|view, cx| view.update(cx, |_, cx| cx.notify()));
}

#[wasm_bindgen]
pub fn set_theme(dark: bool) {
    with_view(|view, cx| {
        apply_theme(dark, cx);
        view.update(cx, |_, cx| cx.notify());
    });
}

/// The bundled catalog, for palettes and inspectors on the page.
#[wasm_bindgen]
pub fn catalog_yaml() -> String {
    include_str!("../../../catalogs/gpui-kit.catalog.yaml").to_owned()
}
