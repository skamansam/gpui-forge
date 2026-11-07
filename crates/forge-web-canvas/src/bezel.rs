//! WASM canvas for the bezel base — `bezel::gpui` (Zed f16f96) on
//! `bezel-gpui-web`'s `WebPlatform`, the same JS contract as the
//! gpui-kit canvas (#99).

use forge_spec::catalog::Catalog;
use gpui_forge_runtime::check;
use gpui_forge_runtime::render::ForgeView;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

use bezel::gpui::*;

pub const BASE: &str = "bezel";

thread_local! {
    static APPLICATION: RefCell<Option<ApplicationHandle>> = const { RefCell::new(None) };
    static VIEW: RefCell<Option<Entity<ForgeView>>> = const { RefCell::new(None) };
    static CATALOGS: Vec<Catalog> = check::catalogs();
    static PENDING: RefCell<Option<forge_spec::Document>> = const { RefCell::new(None) };
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

/// Starts the bezel canvas — `WebPlatform` drives the frame loop on wasm.
#[wasm_bindgen]
pub fn start(dark: bool, _assets: String) {
    console_error_panic_hook::set_once();
    #[cfg(target_family = "wasm")]
    let app = Application::with_platform(Rc::new(gpui_web::WebPlatform::new(false)));
    #[cfg(not(target_family = "wasm"))]
    let app = Application::with_platform(bezel::gpui_platform::current_platform(false));
    let launch = move |cx: &mut App| {
        apply_theme(dark, cx);
        cx.open_window(
            WindowOptions {
                app_id: Some("forge-web-canvas-bezel".into()),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|cx| {
                    let mut view = ForgeView::new(CATALOGS.with(|c| c.clone()));
                    view.set_sinks(
                        Rc::new(|call, _, _| forge_hook(call)),
                        Rc::new(|state, _, _| {
                            forge_state(&serde_json::to_string(state).unwrap_or_default())
                        }),
                    );
                    if let Some(doc) = PENDING.with(|p| p.borrow_mut().take()) {
                        view.set_document(doc, cx);
                    }
                    if let Some(state) = PENDING_STATE.with(|p| p.borrow_mut().take()) {
                        view.set_state(state, cx);
                    }
                    view
                })
            },
        )
        .expect("canvas window");
        forge_ready();
    };
    #[cfg(target_family = "wasm")]
    APPLICATION.with(|a| *a.borrow_mut() = Some(app.run_embedded(launch)));
    #[cfg(not(target_family = "wasm"))]
    app.run(launch);
}

fn apply_theme(dark: bool, cx: &mut App) {
    bezel::theme::appearance::init(
        if dark {
            bezel::theme::appearance::AppearanceMode::Dark
        } else {
            bezel::theme::appearance::AppearanceMode::Light
        },
        cx,
    );
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

/// Reads, validates and (when it parses) renders `yaml`. Diagnostics as JSON.
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

/// Redraws the canvas — the page calls this after a render.
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
    include_str!("../../../catalogs/bezel.catalog.yaml").to_owned()
}
