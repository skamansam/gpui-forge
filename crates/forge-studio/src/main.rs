//! Forge Studio: a Forge document rendered by the Forge runtime.
//!
//! The shell is `studio.forge.yaml` — toolbar, palette, source editor, preview and
//! inspector are all declared in YAML and interpreted by `ForgeView`. Events in that
//! document call the hooks registered below.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use gpui_forge_runtime::base::BASE_ID;
use gpui_forge_runtime::check;
use gpui_forge_runtime::forge_spec::catalog::Catalog;
use gpui_forge_runtime::forge_spec::value::parse_path;
use gpui_forge_runtime::forge_spec::{Diagnostic, yaml};
use gpui_forge_runtime::render::{ForgeView, get_path, set_path};
use gpui_kit::component::theme::{Theme, ThemeMode};
use gpui_kit::{prelude::*, *};
use serde_json::{Value as Json, json};

const SHELL: &str = include_str!("../studio.forge.yaml");
const WELCOME: &str = include_str!("../welcome.forge.yaml");
const GALLERY: &str = include_str!("../gallery.forge.yaml");
const GALLERY_FORGE: &str = include_str!("../gallery.forge.forge.yaml");
const BLANK: &str = "forge: \"0.1\"\nbase: gpui-kit\n\nroot:\n  VStack:\n  style:\n    padding: 16px\n  children:\n    - Text: Hello, Forge!\n";

struct Shared {
    /// The file `editor.source` was loaded from; `None` for embedded documents.
    file: Option<PathBuf>,
    catalogs: Vec<Catalog>,
}

type SharedState = Rc<RefCell<Shared>>;

fn main() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::AllAssets)
        .run(|cx| {
            gpui_kit::init(cx);
            let bounds = Bounds::centered(None, size(px(1400.0), px(900.0)), cx);
            let options = WindowOptions {
                app_id: Some("gpui-forge".into()),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Forge Studio".into()),
                    appears_transparent: true,
                    traffic_light_position: None,
                }),
                app_owns_titlebar_drag: true,
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |_, cx| {
                // Default to the OS appearance — the theme switch still
                // overrides to explicit Light/Dark.
                use gpui_kit::WindowAppearance;
                Theme::change(
                    match cx.window_appearance() {
                        WindowAppearance::Dark | WindowAppearance::VibrantDark => {
                            ThemeMode::Dark
                        }
                        _ => ThemeMode::Light,
                    },
                    None,
                    cx,
                );
                cx.new(build_shell)
            })
            .expect("failed to open window");
        });
}

fn build_shell(cx: &mut Context<ForgeView>) -> ForgeView {
    let shared: SharedState = Rc::new(RefCell::new(Shared {
        file: None,
        catalogs: check::catalogs(),
    }));

    let mut view = ForgeView::new(shared.borrow().catalogs.clone());
    let (doc, diagnostics) = check::check(SHELL, BASE_ID, &shared.borrow().catalogs);
    for d in &diagnostics {
        eprintln!("studio.forge.yaml:{}:{}: {}", d.pos.line, d.pos.col, d.message);
    }
    if let Some(doc) = doc {
        view.set_document(doc, cx);
    }

    view.set_base_dir(env!("CARGO_MANIFEST_DIR"));
    let dark = Theme::global(cx).is_dark();
    view.set_state(
        json!({
            "studio": {
                "dark": dark,
                "file": "welcome.forge.yaml",
                "diagnostics": "",
                "spec": gpui_forge_runtime::forge_spec::SPEC_VERSION,
                "base": BASE_ID,
            },
            "editor": { "source": WELCOME },
            "preview": { "state": "" },
            "status": { "message": "ready — every panel you see is a Forge document" },
        }),
        cx,
    );

    for name in [
        "new_document",
        "open_document",
        "save_document",
        "format_document",
        "apply_theme",
        "source_changed",
        "copy_snippet",
        "open_gallery",
        "select_node",
        "prop_changed",
        "style_changed",
        "start_from",
        "widget_set_changed",
        "test_error",
        "quit",
    ] {
        let shared = shared.clone();
        view.set_hook(
            name,
            Rc::new(move |args, view, cx| {
                dispatch(view, name, args, &shared, cx);
            }),
        );
    }

    // Hooks fired inside the preview bubble up to the shell dispatch; the preview's
    // view state shows in the inspector.
    let shell = cx.entity().downgrade();
    let preview = view.preview("canvas", cx);
    preview.update(cx, |preview, _| {
        let weak = shell.clone();
        let shared = shared.clone();
        preview.set_sinks(
            Rc::new(move |call, _, cx| {
                let (name, args) = parse_call(call);
                let weak = weak.clone();
                let shared = shared.clone();
                // Defer — a preview event fires while the preview entity is leased,
                // so touching it (or the shell, which then touches the preview)
                // must wait until the in-flight update releases.
                cx.defer(move |cx| {
                    weak.update(cx, |view, cx| dispatch(view, &name, &args, &shared, cx))
                        .ok();
                });
            }),
            Rc::new(move |state, _, cx| {
                let shell = shell.clone();
                let state = state.to_string();
                cx.defer(move |cx| {
                    shell
                        .update(cx, |view, cx| {
                            view.update_state(
                                &parse_path("preview.state").unwrap(),
                                Json::String(state.clone()),
                                cx,
                            );
                        })
                        .ok();
                });
            }),
        );
    });

    load_source(
        &mut view,
        "welcome.forge.yaml",
        WELCOME,
        Some(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))),
        &shared,
        cx,
    );
    view
}

/// Routes a fired hook (from the shell or a previewed document) to its implementation.
fn dispatch(
    view: &mut ForgeView,
    name: &str,
    args: &[String],
    shared: &SharedState,
    cx: &mut Context<ForgeView>,
) {
    match name {
        "new_document" => {
            shared.borrow_mut().file = None;
            start_from(view, &["blank".into()], shared, cx);
        }
        "open_document" => open_document(shared.clone(), cx),
        "save_document" => save_document(view, shared, cx),
        "format_document" => format_document(view, shared, cx),
        "apply_theme" => {
            let dark = args.first().is_some_and(|a| a == "true");
            Theme::change(if dark { ThemeMode::Dark } else { ThemeMode::Light }, None, cx);
        }
        "source_changed" => reload(view, shared, cx),
        "copy_snippet" => copy_snippet(view, args, cx),
        "open_gallery" => start_from(view, &["gallery".into()], shared, cx),
        "start_from" => start_from(view, args, shared, cx),
        "widget_set_changed" => {
            let name = match args.first().map(String::as_str) {
                Some("forge") => "gallery.forge.forge.yaml",
                _ => "gallery.forge.yaml",
            };
            start_from(view, &[name.to_owned()], shared, cx);
        }
        "select_node" => select_node(view, args, cx),
        "prop_changed" => prop_changed(view, args, cx),
        "style_changed" => style_changed(view, args, cx),
        "quit" => cx.quit(),
        "test_error" => {
            view.report("test toast — simulated error".to_owned());
            cx.notify();
        }
        other => status(view, &format!("unhandled hook `{other}`"), cx),
    }
}

/// `start_from('welcome'|'gallery'|'blank')` — loads a starter document and shows
/// the preview tab so the user sees the running app.
fn start_from(view: &mut ForgeView, args: &[String], shared: &SharedState, cx: &mut Context<ForgeView>) {
    let name = match args.first().map(String::as_str) {
        Some("gallery" | "gallery.forge.yaml") => "gallery.forge.yaml",
        Some("forge" | "gallery.forge.forge.yaml") => "gallery.forge.forge.yaml",
        Some("blank" | "blank.forge.yaml") => "blank.forge.yaml",
        _ => "welcome.forge.yaml",
    };
    // Read the real file from disk — `include_str!` would be stale the moment the
    // user edits `crates/forge-studio/*.forge.yaml`. Falls back to embedded.
    let src = std::fs::read_to_string(format!("{}/{name}", env!("CARGO_MANIFEST_DIR")))
        .ok()
        .or_else(|| match name {
            "gallery.forge.yaml" => Some(GALLERY.to_owned()),
            "gallery.forge.forge.yaml" => Some(GALLERY_FORGE.to_owned()),
            "welcome.forge.yaml" => Some(WELCOME.to_owned()),
            _ => Some(BLANK.to_owned()),
        });
    match src {
        Some(src) => {
            load_source(
                view,
                name,
                &src,
                Some(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))),
                shared,
                cx,
            );
            view.update_state(&parse_path("studio.tab").unwrap(), Json::from(2), cx);
        }
        None => status(view, &format!("no starter `{name}`"), cx),
    }
}

/// `open_document()` → "open_document", `apply_theme(true)` → ("apply_theme", ["true"]).
fn parse_call(call: &str) -> (String, Vec<String>) {
    match call.split_once('(') {
        Some((name, rest)) => {
            let args = rest.trim_end_matches(')');
            let args = if args.trim().is_empty() {
                Vec::new()
            } else {
                args.split(", ").map(|a| a.trim().to_owned()).collect()
            };
            (name.trim().to_owned(), args)
        }
        None => (call.trim().to_owned(), Vec::new()),
    }
}

fn open_document(shared: SharedState, cx: &mut Context<ForgeView>) {
    let receiver = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: false,
        prompt: Some("Open a Forge document".into()),
    });
    cx.spawn(async move |view, cx| {
        let Ok(Ok(Some(paths))) = receiver.await else {
            return;
        };
        let Some(path) = paths.into_iter().next() else {
            return;
        };
        view.update(cx, move |view, cx| match std::fs::read_to_string(&path) {
            Ok(src) => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.display().to_string());
                let dir = path.parent().map(|p| p.to_path_buf());
                shared.borrow_mut().file = Some(path);
                load_source(view, &name, &src, dir.as_deref(), &shared, cx);
                view.update_state(&parse_path("studio.tab").unwrap(), Json::from(2), cx);
            }
            Err(e) => status(view, &format!("cannot read {}: {e}", path.display()), cx),
        })
        .ok();
    })
    .detach();
}

/// Loads `src` into the editor and the preview, reporting diagnostics in the inspector.
fn load_source(
    view: &mut ForgeView,
    name: &str,
    src: &str,
    dir: Option<&std::path::Path>,
    shared: &SharedState,
    cx: &mut Context<ForgeView>,
) {
    let catalogs = shared.borrow().catalogs.clone();
    let (doc, diagnostics) = check::check(src, BASE_ID, &catalogs);
    let mut state = view.state().clone();
    set_path(&mut state, &parse_path("editor.source").unwrap(), Json::String(src.to_owned()));
    set_path(&mut state, &parse_path("studio.file").unwrap(), Json::String(name.to_owned()));
    set_path(
        &mut state,
        &parse_path("studio.diagnostics").unwrap(),
        Json::String(problems(&diagnostics)),
    );
    set_path(
        &mut state,
        &parse_path("status.message").unwrap(),
        Json::String(problem_count(&diagnostics)),
    );
    view.set_state(state, cx);
    if let Some(doc) = doc {
        view.set_preview_document("canvas", doc, cx);
        if let Some(dir) = dir {
            view.preview("canvas", cx)
                .update(cx, |p, _| p.set_base_dir(dir.to_path_buf()));
        }
    }
}

/// `on change: source_changed()` — re-check the source on every edit and keep the
/// preview showing the last valid document.
fn reload(view: &mut ForgeView, shared: &SharedState, cx: &mut Context<ForgeView>) {
    let src = source(view);
    let catalogs = shared.borrow().catalogs.clone();
    let (doc, diagnostics) = check::check(&src, BASE_ID, &catalogs);
    view.update_state(
        &parse_path("studio.diagnostics").unwrap(),
        Json::String(problems(&diagnostics)),
        cx,
    );
    view.update_state(
        &parse_path("status.message").unwrap(),
        Json::String(problem_count(&diagnostics)),
        cx,
    );
    if let Some(doc) = doc {
        view.set_preview_document("canvas", doc, cx);
    }
}

fn save_document(view: &mut ForgeView, shared: &SharedState, cx: &mut Context<ForgeView>) {
    let Some(path) = shared.borrow().file.clone() else {
        save_as(shared.clone(), cx);
        return;
    };
    match std::fs::write(&path, source(view)) {
        Ok(()) => status(view, &format!("saved {}", path.display()), cx),
        Err(e) => status(view, &format!("cannot save {}: {e}", path.display()), cx),
    }
}

/// Save with no open file: prompt for a path, then write the source there.
fn save_as(shared: SharedState, cx: &mut Context<ForgeView>) {
    let directory = std::env::current_dir().unwrap_or_default();
    let receiver = cx.prompt_for_new_path(&directory, Some("untitled.forge.yaml"));
    cx.spawn(async move |view, cx| {
        let Ok(Ok(Some(path))) = receiver.await else {
            return;
        };
        view.update(cx, move |view, cx| {
            match std::fs::write(&path, source(view)) {
                Ok(()) => {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.display().to_string());
                    shared.borrow_mut().file = Some(path.clone());
                    view.update_state(
                        &parse_path("studio.file").unwrap(),
                        Json::String(name),
                        cx,
                    );
                    status(view, &format!("saved {}", path.display()), cx);
                }
                Err(e) => status(view, &format!("cannot save {}: {e}", path.display()), cx),
            }
        })
        .ok();
    })
    .detach();
}

fn format_document(view: &mut ForgeView, shared: &SharedState, cx: &mut Context<ForgeView>) {
    let (doc, _) = check::check(&source(view), BASE_ID, &shared.borrow().catalogs);
    match doc {
        Some(doc) => {
            let formatted = yaml::to_string(&doc, &yaml::WriteOptions::default());
            let mut state = view.state().clone();
            set_path(
                &mut state,
                &parse_path("editor.source").unwrap(),
                Json::String(formatted),
            );
            view.set_state(state, cx);
            reload(view, shared, cx);
        }
        None => status(view, "cannot format: the document does not parse", cx),
    }
}

fn copy_snippet(view: &mut ForgeView, args: &[String], cx: &mut Context<ForgeView>) {
    let Some(name) = args.first() else {
        return;
    };
    let Some(snippet) = snippet(name) else {
        status(view, &format!("no snippet for `{name}`"), cx);
        return;
    };
    cx.write_to_clipboard(ClipboardItem::new_string(snippet.to_owned()));
    status(view, &format!("copied a `{name}` snippet to the clipboard"), cx);
}

/// A node snippet for each palette element.
fn snippet(name: &str) -> Option<&'static str> {
    Some(match name {
        "Button" => "- Button: Label\n  on click:",
        "Input" => "- Input:\n  value: bind $form.field\n  placeholder: Text",
        "Textarea" => "- Textarea:\n  value: bind $form.text\n  rows: 3",
        "Checkbox" => "- Checkbox: Label\n  checked: bind $form.flag",
        "Switch" => "- Switch: Label\n  checked: bind $form.flag",
        "RadioGroup" => "- RadioGroup:\n  selected: bind $form.choice\n  children:\n    - Radio: One\n    - Radio: Two",
        "Select" => "- Select:\n  options: [One, Two]\n  value: bind $form.choice",
        "Tabs" => "- Tabs:\n  children:\n    - Tab: One\n      children:\n        - Text: first panel\n    - Tab: Two\n      children:\n        - Text: second panel",
        "VStack" => "- VStack:\n  children:\n    - Text: Hello",
        "HStack" => "- HStack:\n  children:\n    - Text: Hello",
        "GroupBox" => "- GroupBox: Title\n  variant: Outline\n  children:\n    - Text: Hello",
        "Text" => "- Text: Hello",
        "Label" => "- Label: Title\n  secondary: supporting text",
        "Tag" => "- Tag: Tag\n  variant: Primary",
        "Icon" => "- Icon: star\n  size: Small",
        "Badge" => "- Badge: 3\n  children:\n    - Icon: bell\n    size: Small",
        "Separator" => "- Separator:\n  label: Section",
        "Link" => "- Link: Label\n  href: https://",
        "Empty" => "- Empty: Nothing here\n  description: an empty state",
        "Preview" => "- forge.Preview:\n  placeholder: document goes here",
        _ => return None,
    })
}

fn source(view: &ForgeView) -> String {
    view.state()
        .get("editor")
        .and_then(|e| e.get("source"))
        .and_then(Json::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn status(view: &mut ForgeView, message: &str, cx: &mut Context<ForgeView>) {
    view.update_state(
        &parse_path("status.message").unwrap(),
        Json::String(message.to_owned()),
        cx,
    );
}

/// Diagnostics formatted for the inspector: one `line:col: message — hint` per line.
fn problems(diagnostics: &[Diagnostic]) -> String {
    if diagnostics.is_empty() {
        return "no problems".into();
    }
    diagnostics
        .iter()
        .map(|d| match &d.hint {
            Some(hint) => format!("{}:{}: {} — {hint}", d.pos.line, d.pos.col, d.message),
            None => format!("{}:{}: {}", d.pos.line, d.pos.col, d.message),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn problem_count(diagnostics: &[Diagnostic]) -> String {
    match diagnostics.len() {
        0 => "ok".into(),
        1 => "1 problem".into(),
        n => format!("{n} problems"),
    }
}

/// `prop_changed(prop)` — an inspector field wrote `document.edits.<prop>`; push the
/// new value into the preview's document at `document.selected` and re-render.
/// `style_changed('<style-prop>')` — the Inspector's Style tab writes the bound
/// `document.edits.style.<name>` value back into the node's `style` entries.
fn style_changed(view: &mut ForgeView, args: &[String], cx: &mut Context<ForgeView>) {
    let Some(name) = args.first() else {
        return;
    };
    let state = view.state();
    let Some(path) = get_path(state, &parse_path("document.selected").unwrap())
        .and_then(|j| j.as_str())
        .map(str::to_owned)
    else {
        return;
    };
    let value = get_path(
        state,
        &parse_path(&format!("document.edits.style.{name}")).unwrap(),
    )
    .cloned()
    .unwrap_or(Json::Null);
    let preview = view.preview("canvas", cx);
    preview.update(cx, |preview, cx| {
        preview.set_node_style(&path, name, &value, cx);
    });
}

fn prop_changed(view: &mut ForgeView, args: &[String], cx: &mut Context<ForgeView>) {
    let Some(prop) = args.first() else {
        return;
    };
    let state = view.state();
    let Some(path) = get_path(state, &parse_path("document.selected").unwrap())
        .and_then(|j| j.as_str())
        .map(str::to_owned)
    else {
        return;
    };
    let value = get_path(
        state,
        &parse_path(&format!("document.edits.{prop}")).unwrap(),
    )
    .cloned()
    .unwrap_or(Json::Null);
    let preview = view.preview("canvas", cx);
    preview.update(cx, |preview, cx| {
        preview.set_node_prop(&path, prop, &value, cx);
    });
    // Keep the source pane in sync with the mutated document — rebuild the editor
    // entity from the bound path so the new text shows.
    if let Some(doc) = preview.read(cx).document().cloned() {
        let src = yaml::to_string(&doc, &yaml::WriteOptions::default());
        view.update_state(
            &parse_path("editor.source").unwrap(),
            Json::String(src),
            cx,
        );
        view.reset_inputs("source");
    }
}

/// Selection in the `forge.Outline` — the row's `bind selected` write already landed
/// the path in `document.selected`; this just surfaces it in the status bar.
fn select_node(view: &mut ForgeView, _args: &[String], cx: &mut Context<ForgeView>) {
    let path = get_path(view.state(), &parse_path("document.selected").unwrap())
        .and_then(|j| j.as_str())
        .unwrap_or_default()
        .to_owned();
    view.update_state(
        &parse_path("status.message").unwrap(),
        Json::String(format!("selected {path}")),
        cx,
    );
}
