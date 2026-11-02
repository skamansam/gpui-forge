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
use gpui_forge_runtime::forge_spec::{Diagnostic, ElementRef, Node, Value, yaml};
use gpui_forge_runtime::render::{ForgeView, get_path, set_path};
use gpui_kit::component::theme::{Theme, ThemeMode, ThemeRegistry};
use gpui_kit::component::theme::ThemeSet;
use gpui_kit::component::WindowExt as _;
use gpui_forge_runtime::forge_spec::Document;
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
    /// Undo/redo — snapshots of `(document, editor source)` taken before each
    /// mutation, so a step back restores the editor text exactly.
    undo: Vec<(Document, String)>,
    redo: Vec<(Document, String)>,
}

type SharedState = Rc<RefCell<Shared>>;

fn main() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::AllAssets)
        .run(|cx| {
            gpui_kit::init(cx);
            cx.bind_keys([gpui::KeyBinding::new(
                "escape",
                gpui_forge_runtime::render::CancelPick,
                None,
            )]);
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
        undo: Vec::new(),
        redo: Vec::new(),
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
    // Load every vendored theme JSON — `default-theme.json` is only
    // light+dark; the full catalog lives in the workspace `themes/` dir.
    // Load every vendored theme JSON — `default-theme.json` is only
    // light+dark; the full catalog lives in the workspace `themes/` dir.
    let themes_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../themes");
    if let Ok(entries) = std::fs::read_dir(&themes_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|s| s.to_str()) == Some("json")
                && let Ok(content) = std::fs::read_to_string(entry.path())
            {
                let _ = ThemeRegistry::global_mut(cx).load_themes_from_str(&content);
            }
        }
    }
    // Watch the dir too — editing a theme JSON reloads the registry live.
    let _ = ThemeRegistry::watch_dir(themes_dir, cx, |_| {});
    let sorted = ThemeRegistry::global(cx).sorted_themes();
    let light_themes: Vec<String> = sorted
        .iter()
        .filter(|t| !t.mode.is_dark())
        .map(|t| t.name.to_string())
        .collect();
    let dark_themes: Vec<String> = sorted
        .iter()
        .filter(|t| t.mode.is_dark())
        .map(|t| t.name.to_string())
        .collect();
    let theme_name = Theme::global(cx).theme_name().to_string();
    view.set_state(
        json!({
            "studio": {
                "dark": dark,
                "light_themes": light_themes,
                "dark_themes": dark_themes,
                "theme": theme_name,
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
        "apply_theme_name",
        "apply_theme_json",
        "reset_theme",
        "source_changed",
        "copy_snippet",
        "insert_component",
        "place_component",
        "pick",
        "undo",
        "redo",
        "remove_component",
        "drop_component",
        "move_node",
        "cancel_place",
        "open_gallery",
        "select_node",
        "prop_changed",
        "style_changed",
        "start_from",
        "widget_set_changed",
        "test_error",
        "open_dialog",
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
    preview.update(cx, |preview, cx| {
        preview.set_dnd(true, cx);
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
/// `open_dialog('name')` — finds the document's `Dialog#name` declaration,
/// builds a child ForgeView from its contents, and opens it via `WindowExt`.
fn open_dialog(view: &mut ForgeView, args: &[String], shared: &SharedState, cx: &mut Context<ForgeView>) {
    let Some(name) = args
        .first()
        .map(|a| a.trim_matches(|c| c == '"' || c == '\'').to_owned())
    else {
        return;
    };
    let Some(node) = view
        .document()
        .and_then(|doc| doc.dialogs.iter().find(|n| n.id.as_deref() == Some(name.as_str())).cloned())
    else {
        view.report(format!("dialog `{name}` not found"));
        return;
    };
    let catalogs = view.catalogs.clone();
    let shell = cx.entity().downgrade();
    let shared = shared.clone();
    let child = cx.new(|cx| {
        let mut v = ForgeView::new(catalogs);
        let weak = shell.clone();
        let shared = shared.clone();
        v.set_sinks(
            Rc::new(move |call, _, cx| {
                let (name, args) = parse_call(call);
                let weak = weak.clone();
                let shared = shared.clone();
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
                                &parse_path("dialog.state").unwrap(),
                                Json::String(state.clone()),
                                cx,
                            );
                        })
                        .ok();
                });
            }),
        );
        v.set_document(
            Document {
                schema: None,
                version: "0.1".into(),
                base: None,
                imports: vec![],
                meta: vec![],
                dialogs: vec![],
                raw: None,
                root: Node {
                    element: ElementRef { namespace: None, name: "VStack".into() },
                    style: node.style.clone(),
                    children: node.children.clone(),
                    ..Default::default()
                },
            },
            cx,
        );
        v
    });
    // The hook runs inside the main window's own update cycle — `App::windows()`
    // handles can't be `update`d from in here because the window is taken out of
    // the slot map for the duration. A foreground task runs after the update
    // finishes, when the window is live again.
    let app = cx.to_async();
    let title = SharedString::from(name.clone());
    cx.foreground_executor()
        .spawn(async move {
            let opened = app.update(|cx| {
                for handle in cx.windows() {
                    let title = title.clone();
                    let child = child.clone();
                    if handle
                        .update(cx, move |_, window, cx| {
                            window.open_dialog(cx, move |dialog, _, _| {
                                dialog.title(title.clone()).child(child.clone())
                            });
                        })
                        .is_ok()
                    {
                        return true;
                    }
                }
                false
            });
            if !opened {
                eprintln!("open_dialog: no live window for `{name}`");
            }
        })
        .detach();
}

/// `apply_theme_name('Theme name')` — the titlebar `Select` writes `studio.theme`
/// and fires this; swaps the theme config + mode to match.
fn apply_theme_name(view: &mut ForgeView, args: &[String], cx: &mut Context<ForgeView>) {
    let Some(name) = args.first() else {
        return;
    };
    let Some(config) = ThemeRegistry::global(cx).themes().get(name.as_str()).cloned() else {
        view.report(format!("theme `{name}` not found"));
        return;
    };
    // `apply_config` installs the config into its mode's slot, switches mode,
    // and applies colors — `edit` sees it as already installed and skips the
    // reload that would otherwise re-apply the OLD config.
    Theme::update(cx, |theme| theme.apply_config(&config));
    // Show the theme's JSON in the Themes editor.
    if let Ok(json) = serde_json::to_string_pretty(&ThemeSet {
        name: config.name.clone(),
        themes: vec![(*config).clone()],
        ..Default::default()
    }) {
        view.update_state(&parse_path("studio.theme_json").unwrap(), Json::String(json), cx);
    }
}

/// `reset_theme()` — the Themes tab's reset icon — restores the registry's
/// default light/dark theme for the current mode.
fn reset_theme(view: &mut ForgeView, cx: &mut Context<ForgeView>) {
    let registry = ThemeRegistry::global(cx);
    let dark = Theme::global(cx).is_dark();
    let default = if dark {
        registry.default_dark_theme().clone()
    } else {
        registry.default_light_theme().clone()
    };
    let name = default.name.to_string();
    Theme::update(cx, |theme| theme.apply_config(&default));
    view.update_state(&parse_path("studio.theme").unwrap(), Json::String(name), cx);
}

/// `apply_theme_json()` — parse `studio.theme_json` (a `ThemeSet`) and apply the
/// first theme, so the user can edit a builtin theme's JSON and see it live.
fn apply_theme_json(view: &mut ForgeView, cx: &mut Context<ForgeView>) {
    let json = view
        .state()
        .get("studio")
        .and_then(|s| s.get("theme_json"))
        .and_then(|j| j.as_str())
        .map(|s| s.to_owned());
    let Some(json) = json else {
        return;
    };
    match serde_json::from_str::<ThemeSet>(&json) {
        Ok(set) => {
            let Some(config) = set.themes.into_iter().next() else {
                view.report("theme set has no themes".to_owned());
                return;
            };
            let name = config.name.to_string();
            Theme::update(cx, |theme| theme.apply_config(&Rc::new(config)));
            view.update_state(&parse_path("studio.theme").unwrap(), Json::String(name), cx);
        }
        Err(e) => view.report(format!("theme json: {e}")),
    }
}

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
        "apply_theme_name" => apply_theme_name(view, args, cx),
        "apply_theme_json" => apply_theme_json(view, cx),
        "reset_theme" => reset_theme(view, cx),
        "source_changed" => reload(view, shared, cx),
        "copy_snippet" => copy_snippet(view, args, cx),
        "insert_component" => insert_component(view, args, cx),
        "place_component" => place_component(view, args, shared, cx),
        "undo" => undo_action(view, shared, cx),
        "redo" => redo_action(view, shared, cx),
        "remove_component" => remove_component(view, shared, cx),
        "drop_component" => drop_component(view, args, shared, cx),
        "move_node" => move_node(view, args, shared, cx),
        "cancel_place" => {
            view.update_state(&parse_path("studio.placing").unwrap(), Json::Null, cx);
            status(view, "cancelled", cx);
            cx.notify();
        }
        "pick" => {
            view.report("pick mode — click an element in the preview".to_owned());
            view.preview("canvas", cx)
                .update(cx, |p, cx| p.set_pick(true, cx));
        }
        "open_dialog" => open_dialog(view, args, shared, cx),
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
        "prop_changed" => prop_changed(view, args, shared, cx),
        "style_changed" => style_changed(view, args, shared, cx),
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
    // A different document invalidates the mutation history.
    shared.borrow_mut().undo.clear();
    shared.borrow_mut().redo.clear();
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

/// Palette click — arms click-to-place: the next preview click inserts the
/// element's snippet under that node. Clicking the palette again cancels.
fn insert_component(view: &mut ForgeView, args: &[String], cx: &mut Context<ForgeView>) {
    let Some(tag) = args.first().cloned() else {
        return;
    };
    let placing = get_path(view.state(), &parse_path("studio.placing").unwrap())
        .and_then(|j| j.as_str())
        .map(str::to_owned);
    let preview = view.preview("canvas", cx);
    if placing.as_deref() == Some(tag.as_str()) {
        // Same palette item clicked while armed — cancel place mode.
        view.update_state(&parse_path("studio.placing").unwrap(), Json::Null, cx);
        preview.update(cx, |p, cx| p.set_placing(false, cx));
        status(view, "place cancelled", cx);
        return;
    }
    view.update_state(
        &parse_path("studio.placing").unwrap(),
        Json::String(tag.clone()),
        cx,
    );
    preview.update(cx, |p, cx| p.set_placing(true, cx));
    status(
        view,
        &format!("placing {tag} — click a container in the preview (palette again to cancel)"),
        cx,
    );
    cx.notify();
}

/// Fires when the preview reports a drop target (`studio.drop_target`) while a
/// palette item is armed (`studio.placing`): parse the element's snippet into a
/// node and insert it under the clicked node.
fn place_component(view: &mut ForgeView, args: &[String], shared: &SharedState, cx: &mut Context<ForgeView>) {
    let state = view.state().clone();
    let Some(tag) = get_path(&state, &parse_path("studio.placing").unwrap())
        .and_then(|j| j.as_str())
        .map(str::to_owned)
    else {
        return;
    };
    let target = args
        .first()
        .map(|t| t.trim_matches(|c| c == '"' || c == '\'').to_owned());
    let node = snippet_node(&tag);
    record_undo(view, shared, cx);
    let preview = view.preview("canvas", cx);
    let mut placed = None;
    preview.update(cx, |p, cx| {
        let Some(mut doc) = p.document().cloned() else {
            return;
        };
        let target = target
            .as_deref()
            .filter(|path| doc.node_at(path).is_some())
            .unwrap_or("0");
        let into = target == "0" || accepts_children(&shared.borrow().catalogs, &doc, target);
        // Container targets take the node inside; leaf targets take it as
        // the sibling right after.
        let result = if into {
            doc.insert_node(target, node, None)
                .then(|| format!("{target}.{}", doc.node_at(target).unwrap().children.len() - 1))
        } else {
            doc.insert_after(target, node)
        };
        if result.is_some() {
            placed = result;
            p.set_document(doc, cx);
        }
    });
    if let Some(path) = placed {
        view.update_state(
            &parse_path("document.selected").unwrap(),
            Json::String(path.clone()),
            cx,
        );
        view.update_state(&parse_path("studio.placing").unwrap(), Json::Null, cx);
        view.update_state(&parse_path("studio.dirty").unwrap(), Json::Bool(true), cx);
        status(view, &format!("placed {tag} at {path}"), cx);
        if let Some(doc) = preview.read(cx).document().cloned() {
            let src = yaml::to_string_preserving(&doc, &yaml::WriteOptions::default());
            view.update_state(
                &parse_path("editor.source").unwrap(),
                Json::String(src),
                cx,
            );
            view.reset_inputs("source");
        }
    }
    cx.notify();
}

/// A palette item dropped on a preview node or outline row —
/// `drop_component('<tag>', '<target-path>')`.
fn drop_component(view: &mut ForgeView, args: &[String], shared: &SharedState, cx: &mut Context<ForgeView>) {
    let Some(tag) = args
        .first()
        .map(|t| t.trim_matches(|c| c == '"' || c == '\'').to_owned())
    else {
        return;
    };
    let target = args
        .get(1)
        .map(|t| t.trim_matches(|c| c == '"' || c == '\'').to_owned());
    let node = snippet_node(&tag);
    insert_at(view, shared, node, &tag, target, cx);
}

/// A document node dragged onto a preview node or outline row —
/// `move_node('<src-path>', '<target-path>')`.
fn move_node(view: &mut ForgeView, args: &[String], shared: &SharedState, cx: &mut Context<ForgeView>) {
    let Some(src) = args
        .first()
        .map(|p| p.trim_matches(|c| c == '"' || c == '\'').to_owned())
    else {
        return;
    };
    let Some(target) = args
        .get(1)
        .map(|t| t.trim_matches(|c| c == '"' || c == '\'').to_owned())
    else {
        return;
    };
    record_undo(view, shared, cx);
    let preview = view.preview("canvas", cx);
    let mut moved = None;
    preview.update(cx, |p, cx| {
        let Some(mut doc) = p.document().cloned() else {
            return;
        };
        // Container targets take the node inside; leaf targets take it as
        // the sibling right after.
        let into = target == "0"
            || accepts_children(&shared.borrow().catalogs, &doc, &target);
        let result = if into {
            doc.reparent(&src, &target, None)
                .then(|| format!("{target}.{}", doc.node_at(&target).unwrap().children.len() - 1))
        } else {
            doc.move_after(&src, &target)
        };
        if result.is_some() {
            moved = result;
            p.set_document(doc, cx);
        }
    });
    if let Some(path) = moved {
        view.update_state(&parse_path("document.selected").unwrap(), Json::String(path.clone()), cx);
        view.update_state(&parse_path("studio.dirty").unwrap(), Json::Bool(true), cx);
        status(view, &format!("moved {src} under {target}"), cx);
        sync_source(view, &preview, cx);
    } else {
        status(view, &format!("cannot move {src} under {target}"), cx);
    }
    cx.notify();
}

/// Inserts `node` at `target` (or root), records undo, selects the new node,
/// and syncs the source pane. Shared by click-to-place and DnD.
fn insert_at(
    view: &mut ForgeView,
    shared: &SharedState,
    node: Node,
    tag: &str,
    target: Option<String>,
    cx: &mut Context<ForgeView>,
) {
    record_undo(view, shared, cx);
    let preview = view.preview("canvas", cx);
    let mut placed = None;
    preview.update(cx, |p, cx| {
        let Some(mut doc) = p.document().cloned() else {
            return;
        };
        let target = target
            .as_deref()
            .filter(|path| doc.node_at(path).is_some())
            .unwrap_or("0");
        let into = target == "0" || accepts_children(&shared.borrow().catalogs, &doc, target);
        // Container targets take the node inside; leaf targets take it as
        // the sibling right after.
        let result = if into {
            doc.insert_node(target, node, None)
                .then(|| format!("{target}.{}", doc.node_at(target).unwrap().children.len() - 1))
        } else {
            doc.insert_after(target, node)
        };
        if result.is_some() {
            placed = result;
            p.set_document(doc, cx);
        }
    });
    if let Some(path) = placed {
        view.update_state(&parse_path("document.selected").unwrap(), Json::String(path.clone()), cx);
        view.update_state(&parse_path("studio.dirty").unwrap(), Json::Bool(true), cx);
        status(view, &format!("placed {tag} at {path}"), cx);
        sync_source(view, &preview, cx);
    }
    cx.notify();
}

/// True when the node at `path` accepts children per its catalog entry.
fn accepts_children(catalogs: &[Catalog], doc: &Document, path: &str) -> bool {
    doc.node_at(path).is_some_and(|node| {
        catalogs.iter().any(|c| {
            c.element(&node.element.name)
                .is_some_and(|def| {
                    !matches!(
                        def.children,
                        gpui_forge_runtime::forge_spec::catalog::Children::None
                    )
                })
        })
    })
}

/// Re-writes `editor.source` from the preview document via the preserving writer.
fn sync_source(view: &mut ForgeView, preview: &Entity<ForgeView>, cx: &mut Context<ForgeView>) {
    if let Some(doc) = preview.read(cx).document().cloned() {
        let src = yaml::to_string_preserving(&doc, &yaml::WriteOptions::default());
        view.update_state(&parse_path("editor.source").unwrap(), Json::String(src), cx);
        view.reset_inputs("source");
    }
}

/// Parses a palette `snippet` into a `Node` by wrapping it as a document
/// children item; falls back to a bare `Element: Name` node.
fn snippet_node(tag: &str) -> Node {
    if let Some(snippet) = snippet(tag) {
        let body: String = snippet
            .lines()
            .map(|line| format!("    {line}"))
            .collect::<Vec<_>>()
            .join("\n");
        let src = format!("forge: \"0.1\"\nroot:\n  VStack:\n  children:\n{body}");
        if let Ok(doc) = yaml::from_str(&src)
            && let Some(node) = doc.root.children.into_iter().next()
        {
            return node;
        }
    }
    Node {
        element: ElementRef {
            name: tag.to_owned(),
            namespace: None,
        },
        content: Some(Value::String(tag.to_owned())),
        ..Default::default()
    }
}

/// Deletes the selected node (`document.selected`) — undoable, root-excluded.
fn remove_component(view: &mut ForgeView, shared: &SharedState, cx: &mut Context<ForgeView>) {
    let Some(path) = get_path(view.state(), &parse_path("document.selected").unwrap())
        .and_then(|j| j.as_str())
        .map(str::to_owned)
    else {
        status(view, "nothing selected", cx);
        return;
    };
    if path == "0" {
        status(view, "cannot remove the root node", cx);
        return;
    }
    record_undo(view, shared, cx);
    let preview = view.preview("canvas", cx);
    let mut removed = false;
    preview.update(cx, |p, cx| {
        let Some(mut doc) = p.document().cloned() else {
            return;
        };
        if doc.remove_node(&path).is_some() {
            removed = true;
            p.set_document(doc, cx);
        }
    });
    if !removed {
        status(view, &format!("cannot remove {path}"), cx);
        return;
    }
    // Select the parent so the inspector lands somewhere sensible.
    let parent = path.rsplit_once('.').map(|(p, _)| p.to_owned()).unwrap_or_else(|| "0".into());
    view.update_state(
        &parse_path("document.selected").unwrap(),
        Json::String(parent),
        cx,
    );
    view.update_state(&parse_path("studio.dirty").unwrap(), Json::Bool(true), cx);
    status(view, &format!("removed {path}"), cx);
    if let Some(doc) = preview.read(cx).document().cloned() {
        let src = yaml::to_string_preserving(&doc, &yaml::WriteOptions::default());
        view.update_state(&parse_path("editor.source").unwrap(), Json::String(src), cx);
        view.reset_inputs("source");
    }
    cx.notify();
}

/// Snapshots `(document, source)` before a mutation; clears the redo stack.
fn record_undo(view: &mut ForgeView, shared: &SharedState, cx: &mut Context<ForgeView>) {
    let preview = view.preview("canvas", cx);
    let Some(doc) = preview.read(cx).document().cloned() else {
        return;
    };
    let src = source(view);
    let mut shared = shared.borrow_mut();
    shared.undo.push((doc, src));
    if shared.undo.len() > 100 {
        shared.undo.remove(0);
    }
    shared.redo.clear();
}

/// Restores the snapshot at the top of `stack`, pushing the current state onto
/// the opposite stack.
fn restore(
    view: &mut ForgeView,
    shared: &SharedState,
    stack: impl Fn(&mut Shared) -> &mut Vec<(Document, String)>,
    other: impl Fn(&mut Shared) -> &mut Vec<(Document, String)>,
    cx: &mut Context<ForgeView>,
) {
    let Some((doc, src)) = stack(&mut shared.borrow_mut()).pop() else {
        status(view, "nothing to restore", cx);
        return;
    };
    let preview = view.preview("canvas", cx);
    let current = preview.read(cx).document().cloned();
    if let Some(current) = current {
        other(&mut shared.borrow_mut()).push((current, source(view)));
    }
    preview.update(cx, |p, cx| p.set_document(doc, cx));
    view.update_state(
        &parse_path("editor.source").unwrap(),
        Json::String(src),
        cx,
    );
    view.reset_inputs("source");
    view.update_state(&parse_path("studio.dirty").unwrap(), Json::Bool(true), cx);
}

fn undo_action(view: &mut ForgeView, shared: &SharedState, cx: &mut Context<ForgeView>) {
    status(view, "undo", cx);
    restore(view, shared, |s| &mut s.undo, |s| &mut s.redo, cx);
}

fn redo_action(view: &mut ForgeView, shared: &SharedState, cx: &mut Context<ForgeView>) {
    status(view, "redo", cx);
    restore(view, shared, |s| &mut s.redo, |s| &mut s.undo, cx);
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
        "Button" => "- Button: Label",
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
fn style_changed(view: &mut ForgeView, args: &[String], shared: &SharedState, cx: &mut Context<ForgeView>) {
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
    record_undo(view, shared, cx);
    let preview = view.preview("canvas", cx);
    preview.update(cx, |preview, cx| {
        preview.set_node_style(&path, name, &value, cx);
    });
}

fn prop_changed(view: &mut ForgeView, args: &[String], shared: &SharedState, cx: &mut Context<ForgeView>) {
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
    record_undo(view, shared, cx);
    let preview = view.preview("canvas", cx);
    preview.update(cx, |preview, cx| {
        preview.set_node_prop(&path, prop, &value, cx);
    });
    // Keep the source pane in sync with the mutated document — rebuild the editor
    // entity from the bound path so the new text shows.
    if let Some(doc) = preview.read(cx).document().cloned() {
        let src = yaml::to_string_preserving(&doc, &yaml::WriteOptions::default());
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
