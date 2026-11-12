//! Renders Forge documents with gpui-kit inside a browser canvas.
//!
//! The page drives the canvas through the exported functions. The canvas reports fired
//! hooks and state changes back through `globalThis.forgeHook` and `globalThis.forgeState`.

pub use gpui_forge_runtime::{check, render};

use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;

use forge_spec::catalog::Catalog;
use gpui_forge_runtime::render::ForgeView;
use gpui_kit::component::theme::{Theme, ThemeMode};
use gpui_kit::{prelude::*, *};
use wasm_bindgen::prelude::*;

/// The base component system this canvas renders with.
pub const BASE: &str = gpui_forge_runtime::base::BASE_ID;

thread_local! {
    static APPLICATION: RefCell<Option<ApplicationHandle>> = const { RefCell::new(None) };
    static VIEW: RefCell<Option<Entity<ForgeView>>> = const { RefCell::new(None) };
    static CATALOGS: Vec<Catalog> = check::catalogs();
    /// A document rendered before the window exists, applied once it does.
    static PENDING: RefCell<Option<forge_spec::Document>> = const { RefCell::new(None) };
    /// View state set before the window exists, applied with `PENDING`.
    static PENDING_STATE: RefCell<Option<serde_json::Value>> = const { RefCell::new(None) };
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = globalThis, js_name = forgeHook)]
    fn forge_hook(call: &str);
    #[wasm_bindgen(js_namespace = globalThis, js_name = forgeState)]
    fn forge_state(json: &str);
    #[wasm_bindgen(js_namespace = globalThis, js_name = forgeReady)]
    fn forge_ready();
}

#[cfg(not(target_family = "wasm"))]
fn forge_hook(_: &str) {}
#[cfg(not(target_family = "wasm"))]
fn forge_state(_: &str) {}
#[cfg(not(target_family = "wasm"))]
fn forge_ready() {}

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
            cx.new(|cx| {
                let mut view = ForgeView::new(CATALOGS.with(Clone::clone));
                // The `use:` libraries the examples page demos — the Studio set
                // minus VideoPlayer (GStreamer is native-only) and Whiteboard/
                // Flow (their views don't paint on the web platform. The
                // elements fall back to "not provided by this adapter yet").
                view.register_library(forge_lib_ely::Ely::new());
                view.register_library(forge_lib_gpui_component::GpuiComponent);
                view.register_library(forge_lib_zorite::Pdf::with_asset_resolver(|src| {
                    // Doc-relative `src:` has no filesystem on wasm — bundled
                    // demo assets resolve by basename instead.
                    static ASSETS: &[(&str, &[u8])] = &[(
                        "sample.pdf",
                        include_bytes!("../../../examples/demos/sample.pdf"),
                    )];
                    let name = src.rsplit('/').next().unwrap_or(src);
                    ASSETS
                        .iter()
                        .find(|(n, _)| *n == name)
                        .map(|(_, b)| std::sync::Arc::new(b.to_vec()))
                }));
                view.register_library(forge_lib_zorite::Math::new());
                view.register_library(forge_lib_hig::Hig);
                view.register_library(forge_lib_base::Base::new());
                view.set_sinks(
                    Rc::new(|call, _, _| forge_hook(call)),
                    Rc::new(|state, _, _| forge_state(&state.to_string())),
                );
                if let Some(state) = PENDING_STATE.with(|p| p.borrow_mut().take()) {
                    view.set_state(state, cx);
                }
                if let Some(doc) = PENDING.with(|p| p.borrow_mut().take()) {
                    view.set_document(doc, cx);
                }
                view
            })
        })
        .expect("canvas window opens");
        VIEW.with(|v| *v.borrow_mut() = Some(view));
        cx.activate(true);
        // The browser's GPU setup is asynchronous, so the window (and the view `render`
        // updates) only exists from here on. Tell the page it can start sending documents.
        forge_ready();
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
    let (doc, diagnostics) = CATALOGS.with(|c| check::check(yaml, BASE, c));
    if let Some(doc) = doc {
        if VIEW.with(|v| v.borrow().is_none()) {
            PENDING.with(|p| *p.borrow_mut() = Some(doc));
        } else {
            with_view(|view, cx| view.update(cx, |view, cx| view.set_document(doc, cx)));
        }
    }
    check::to_json(&diagnostics)
}

/// Validates `yaml` without rendering it. Returns the diagnostics as JSON.
#[wasm_bindgen]
pub fn diagnostics(yaml: &str) -> String {
    CATALOGS.with(|c| check::to_json(&check::check(yaml, BASE, c).1))
}

/// Replaces the view state that bindings read from and write to.
#[wasm_bindgen]
pub fn set_state(json: &str) -> Result<(), JsValue> {
    let state = serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    if VIEW.with(|v| v.borrow().is_none()) {
        PENDING_STATE.with(|p| *p.borrow_mut() = Some(state));
    } else {
        with_view(|view, cx| view.update(cx, |view, cx| view.set_state(state, cx)));
    }
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
