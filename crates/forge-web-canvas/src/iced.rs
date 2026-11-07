//! WASM canvas for the iced base — `iced::application` with the twin's
//! `view()`/`update()` split, the same `forgeHook`/`forgeState` JS contract
//! as the gpui-kit canvas (#99).

use forge_spec::catalog::Catalog;
use gpui_forge_runtime::check;
use gpui_forge_runtime::render::{ForgeView, Message as ForgeMessage};
use iced::{Element, Task};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

pub const BASE: &str = "iced";

thread_local! {
    static VIEW: RefCell<Option<ForgeView>> = const { RefCell::new(None) };
    static CATALOGS: Vec<Catalog> = check::catalogs();
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

#[derive(Debug, Clone)]
enum Message {
    Forge(ForgeMessage),
    Tick,
}

struct App {
    view: ForgeView,
}

impl App {
    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Forge(m) => self.view.update(m),
            Message::Tick => {}
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        self.view.view().map(Message::Forge)
    }
}

/// Starts the iced canvas — iced renders into its own wgpu surface on wasm.
#[wasm_bindgen]
pub fn start(_dark: bool, _assets: String) {
    console_error_panic_hook::set_once();
    let mut view = ForgeView::new(CATALOGS.with(|c| c.clone()));
    view.set_sinks(
        Rc::new(|call, _| forge_hook(call)),
        Rc::new(|state, _| forge_state(&serde_json::to_string(state).unwrap_or_default())),
    );
    VIEW.with(|v| *v.borrow_mut() = Some(view));
    iced::application("Forge Preview · iced", App::update, App::view)
        .run_with(|| {
            let view = VIEW.with(|v| v.borrow_mut().take()).expect("view");
            (App { view }, Task::done(Message::Tick))
        })
        .ok();
    forge_ready();
}

/// Loads a Forge YAML document — same contract as the other canvases.
#[wasm_bindgen]
pub fn document(src: &str) -> bool {
    VIEW.with(|v| {
        v.borrow_mut()
            .as_mut()
            .map(|view| {
                let catalogs = view.catalogs.clone();
                let (doc, _diags) = check::check(src, BASE, &catalogs);
                doc.map(|d| view.set_document(d)).is_some()
            })
            .unwrap_or(false)
    })
}

/// Replaces the view's JSON state — the host pushes `state:` changes here.
#[wasm_bindgen]
pub fn state(json: &str) {
    VIEW.with(|v| {
        let mut vm = v.borrow_mut();
        if let Some(view) = vm.as_mut() {
            if let Ok(state) = serde_json::from_str(json) {
                view.set_state(state);
            }
        }
    })
}

/// Fires a `on click:`/`on change:` call by name — the host's inspector.
#[wasm_bindgen]
pub fn fire(name: &str, args: &str) {
    VIEW.with(|v| {
        let mut vm = v.borrow_mut();
        if let Some(view) = vm.as_mut() {
            view.fire_call(name, args.split(',').map(|s| s.trim().to_owned()).collect());
        }
    })
}
