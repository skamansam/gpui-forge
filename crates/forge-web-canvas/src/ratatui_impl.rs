//! base-ratatui — the terminal twin compiled to WASM (#137). Renders a Forge
//! document into a `TestBackend` and hands the frame back as text rows; the
//! site's `TuiCanvas` component draws them as a monospace grid. No iframe —
//! it's a pure function call, unlike the GPUI canvases.

use gpui_forge_runtime::{check, render::ForgeView};
use ratatui::{Terminal, backend::TestBackend};
use wasm_bindgen::prelude::*;

/// `render_frame(yaml, state_json, width, height)` → a JSON string of
/// `{rows: [String], diagnostics: [{line,col,message,hint}]}` — one terminal
/// frame of the document plus the same check diagnostics every canvas reports.
/// Cell styling is dropped for now (monochrome glyphs).
#[wasm_bindgen]
pub fn render_frame(yaml: &str, state_json: &str, width: u16, height: u16) -> String {
    console_error_panic_hook::set_once();
    let catalogs = check::catalogs();
    let (doc, diagnostics) = check::check(yaml, gpui_forge_runtime::base::BASE_ID, &catalogs);
    let mut view = ForgeView::new(catalogs);
    if let Some(doc) = doc {
        view.set_document(doc);
    }
    if let Ok(state) = serde_json::from_str::<serde_json::Value>(state_json)
        && !state.is_null()
    {
        view.set_state(state);
    }
    let mut term =
        Terminal::new(TestBackend::new(width.max(20), height.max(5))).expect("test backend");
    let _ = term.draw(|f| view.render(f));
    let buffer = term.backend().buffer();
    let rows: Vec<String> = (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect();
    serde_json::json!({
        "rows": rows,
        "diagnostics": check::diagnostics_json(&diagnostics),
    })
    .to_string()
}
