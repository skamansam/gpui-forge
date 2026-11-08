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
use gpui_forge_runtime::forge_spec::Document;
use gpui_forge_runtime::forge_spec::catalog::Catalog;
use gpui_forge_runtime::forge_spec::value::parse_path;
use gpui_forge_runtime::forge_spec::{Diagnostic, ElementRef, Node, Value, yaml};
use gpui_forge_runtime::render::{ForgeView, get_path, set_path};
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::theme::ThemeSet;
use gpui_kit::component::theme::{Theme, ThemeMode, ThemeRegistry};
use gpui_kit::{prelude::*, *};
use serde_json::{Value as Json, json};

const SHELL: &str = include_str!("../studio.forge.yaml");
const WELCOME: &str = include_str!("../welcome.forge.yaml");
const GALLERY: &str = include_str!("../gallery.forge.yaml");
const GALLERY_FORGE: &str = include_str!("../gallery.forge.forge.yaml");
const BLANK: &str = "$schema: https://gpui-forge.rudeboy.dev/schema/0.1/gpui-kit.json\nforge: \"0.1\"\nbase: gpui-kit\n\nroot:\n  VStack:\n  style:\n    padding: 16px\n  children:\n    - Text: Hello, Forge!\n";

struct Shared {
    /// The file `editor.source` was loaded from; `None` for embedded documents.
    file: Option<PathBuf>,
    /// The directory the current document lives in — asset paths resolve
    /// here and it's forwarded to the preview host as `base_dir`.
    dir: Option<PathBuf>,
    catalogs: Vec<Catalog>,
    /// Undo/redo — snapshots of `(document, editor source)` taken before each
    /// mutation, so a step back restores the editor text exactly.
    undo: Vec<(Document, String)>,
    redo: Vec<(Document, String)>,
}

/// The running `forge-preview` process (#97) — stdin for commands, stdout
/// drained by a reader thread into the event channel the pump loop owns.
struct Host {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    /// Spawn generation — `Closed` events from a superseded host are ignored
    /// so a restart can't report the new host as dead.
    generation: u64,
}

/// A line the host wrote on stdout, decoded.
enum HostEvent {
    Ready {
        base: String,
    },
    Diagnostics(Vec<String>),
    /// A hook fired in the host's view, as `name(args)` — the `on_hook` sink.
    Call(String),
    /// The host's whole view state after a write — the `on_state` sink.
    State(Json),
    /// #120 Phase 2 — a TUI frame buffer (one line per row) from
    /// `TestBackend` — rendered as a monospace cell grid in the card.
    Frame {
        rows: Vec<String>,
    },
    /// #120 Phase 2 — an rgba screenshot from a pixel-capable host (iced).
    FrameRgba {
        w: u32,
        h: u32,
        data: String,
    },
    /// stdout reached EOF — the host exited (or was killed for a restart).
    Closed {
        generation: u64,
    },
}

static HOST: std::sync::Mutex<Option<Host>> = std::sync::Mutex::new(None);
static HOST_GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

type SharedState = Rc<RefCell<Shared>>;
/// The hook context type for `#[forge_hook]` wrappers.
type HookCtx = SharedState;

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
            // Reap the preview host on any quit path — the `quit` hook is only
            // one of them (window close, Cmd-Q…).
            cx.on_app_quit(|_| async move {
                if let Some(mut host) = HOST.lock().unwrap().take() {
                    let _ = host.child.kill();
                    let _ = host.child.wait();
                }
            })
            .detach();
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
                        WindowAppearance::Dark | WindowAppearance::VibrantDark => ThemeMode::Dark,
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
        dir: None,
        catalogs: check::catalogs(),
        undo: Vec::new(),
        redo: Vec::new(),
    }));

    let mut view = ForgeView::new(shared.borrow().catalogs.clone());
    let (doc, diagnostics) = check::check(SHELL, BASE_ID, &shared.borrow().catalogs);
    for d in &diagnostics {
        eprintln!(
            "studio.forge.yaml:{}:{}: {}",
            d.pos.line, d.pos.col, d.message
        );
    }
    if let Some(doc) = doc {
        view.set_document(doc, cx);
    }
    refresh_recent_menu(&mut view, &load_recents(), cx);

    view.register_library(forge_lib_ely::Ely);
    view.register_library(forge_lib_gpui_component::GpuiComponent);
    view.register_library(forge_lib_video::VideoPlayer::new());
    view.register_library(forge_lib_zorite::Pdf::new());
    view.register_library(forge_lib_zorite::Whiteboard::new());
    view.register_library(forge_lib_zorite::Math::new());
    view.register_library(forge_lib_hig::Hig);
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
    // #127 — one Select for all themes: separator rows group light/dark.
    let mut all_themes = Vec::with_capacity(light_themes.len() + dark_themes.len() + 2);
    all_themes.push("--- Light Themes ---".to_string());
    all_themes.extend(light_themes.iter().cloned());
    all_themes.push("--- Dark Themes ---".to_string());
    all_themes.extend(dark_themes.iter().cloned());
    let prefs = load_prefs();
    let theme_name = prefs
        .get("theme")
        .and_then(|j| j.as_str())
        .map(str::to_owned)
        .unwrap_or_else(|| Theme::global(cx).theme_name().to_string());
    let dark = prefs.get("dark").and_then(|j| j.as_bool()).unwrap_or(dark);
    if let Some(config) = ThemeRegistry::global(cx)
        .themes()
        .get(theme_name.as_str())
        .cloned()
    {
        Theme::update(cx, |theme| theme.apply_config(&config));
    }
    view.set_state(
        json!({
            "studio": {
                "dark": dark,
                "light_themes": light_themes,
                "dark_themes": dark_themes,
                "all_themes": all_themes,
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
        "open_project",
        "create_project",
        "preview_in_host",
        "raise_host",
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
                let state = state.clone();
                cx.defer(move |cx| {
                    shell
                        .update(cx, |view, cx| {
                            view.update_state(
                                &parse_path("preview.state").unwrap(),
                                Json::String(state.to_string()),
                                cx,
                            );
                            mirror_document_state(view, &state, cx);
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
fn open_dialog(
    view: &mut ForgeView,
    args: &[String],
    shared: &SharedState,
    cx: &mut Context<ForgeView>,
) {
    let Some(name) = args
        .first()
        .map(|a| a.trim_matches(|c| c == '"' || c == '\'').to_owned())
    else {
        return;
    };
    let Some(node) = view.document().and_then(|doc| {
        doc.dialogs
            .iter()
            .find(|n| n.id.as_deref() == Some(name.as_str()))
            .cloned()
    }) else {
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
                state: vec![],
                dialogs: vec![],
                raw: None,
                root: Node {
                    element: ElementRef {
                        namespace: None,
                        name: "VStack".into(),
                    },
                    style: node.style.clone(),
                    children: node.children.clone(),
                    ..Default::default()
                },
            },
            cx,
        );
        // Seed fields dialogs bind against — the base registry for pickers,
        // defaults for the New Project form.
        v.set_state(
            json!({
                "name": "app",
                "base": BASE_ID,
                "bases": check::bases()
                    .iter()
                    .map(|b| b.id.clone())
                    .collect::<Vec<String>>(),
            }),
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
#[forge_hooks_macros::forge_hook]
fn apply_theme_name(
    view: &mut ForgeView,
    _ctx: &HookCtx,
    args: &[String],
    cx: &mut Context<ForgeView>,
) {
    let Some(name) = args.first() else {
        return;
    };
    // #127 — separator rows in the merged all_themes list aren't themes.
    if name.starts_with("---") {
        return;
    }
    let Some(config) = ThemeRegistry::global(cx)
        .themes()
        .get(name.as_str())
        .cloned()
    else {
        view.report(format!("theme `{name}` not found"));
        return;
    };
    // `apply_config` installs the config into its mode's slot, switches mode,
    // and applies colors — `edit` sees it as already installed and skips the
    // reload that would otherwise re-apply the OLD config.
    Theme::update(cx, |theme| theme.apply_config(&config));
    save_prefs(view);
    // Show the theme's JSON in the Themes editor.
    if let Ok(json) = serde_json::to_string_pretty(&ThemeSet {
        name: config.name.clone(),
        themes: vec![(*config).clone()],
        ..Default::default()
    }) {
        view.update_state(
            &parse_path("studio.theme_json").unwrap(),
            Json::String(json),
            cx,
        );
    }
    host_send_theme(view);
}

/// `reset_theme()` — the Themes tab's reset icon — restores the registry's
/// default light/dark theme for the current mode.
#[forge_hooks_macros::forge_hook]
fn reset_theme(
    view: &mut ForgeView,
    _ctx: &HookCtx,
    _args: &[String],
    cx: &mut Context<ForgeView>,
) {
    let registry = ThemeRegistry::global(cx);
    let dark = Theme::global(cx).is_dark();
    // #123+#126 — reset loads the theme STORED in
    // `~/.gpui-forge/settings.toml` (the per-user prefs), not whatever's
    // selected in-session; falls back to the mode default.
    let stored = load_prefs()
        .get("theme")
        .and_then(|j| j.as_str())
        .map(|s| s.to_owned());
    let default = stored
        .and_then(|name| registry.themes().get(name.as_str()).cloned())
        .unwrap_or_else(|| {
            if dark {
                registry.default_dark_theme().clone()
            } else {
                registry.default_light_theme().clone()
            }
        });
    let name = default.name.to_string();
    Theme::update(cx, |theme| theme.apply_config(&default));
    view.update_state(&parse_path("studio.theme").unwrap(), Json::String(name), cx);
    host_send_theme(view);
}

/// `apply_theme_json()` — parse `studio.theme_json` (a `ThemeSet`) and apply the
/// first theme, so the user can edit a builtin theme's JSON and see it live.
#[forge_hooks_macros::forge_hook]
fn apply_theme_json(
    view: &mut ForgeView,
    _ctx: &HookCtx,
    _args: &[String],
    cx: &mut Context<ForgeView>,
) {
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
            host_send_theme(view);
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
    // Registry first — #[forge_hook] functions register via inventory and need
    // no arm here; anything unhandled falls through to the legacy match.
    if forge_hooks::dispatch::<SharedState>(shared, view, name, args, cx) {
        return;
    }
    match name {
        "new_document" => {
            shared.borrow_mut().file = None;
            start_from(view, &["blank".into()], shared, cx);
        }
        "open_document" => open_document(shared.clone(), cx),
        "open_project" => open_project(shared.clone(), cx),
        "create_project" => create_project(view, shared.clone(), cx),
        "preview_in_host" => preview_in_host(view, shared.clone(), cx),
        "raise_host" => host_send(&json!({"cmd":"raise"})),
        "open_recent" => {
            if let Some(path) = args.first().map(PathBuf::from) {
                load_file(path, view, shared.clone(), cx);
            }
        }
        "save_document" => save_document(view, shared, cx),
        "format_document" => format_document(view, shared, cx),
        "apply_theme" => {
            let dark = args.first().is_some_and(|a| a == "true");
            Theme::change(
                if dark {
                    ThemeMode::Dark
                } else {
                    ThemeMode::Light
                },
                None,
                cx,
            );
            view.update_state(&parse_path("studio.dark").unwrap(), Json::Bool(dark), cx);
            host_send(&json!({"cmd":"theme","dark": dark}));
        }
        // apply_theme_name/apply_theme_json/reset_theme: registry hooks
        "source_changed" => reload(view, shared, cx),
        // copy_snippet/insert_component: registry hooks
        "place_component" => place_component(view, args, shared, cx),
        "undo" => undo_action(view, shared, cx),
        "redo" => redo_action(view, shared, cx),
        "remove_component" => remove_component(view, shared, cx),
        "drop_component" => drop_component(view, args, shared, cx),
        "move_node" => move_node(view, args, shared, cx),
        "cancel_place" => {
            view.update_state(&parse_path("studio.placing").unwrap(), Json::Null, cx);
            host_send(&json!({"cmd":"pick","on":false}));
            host_send(&json!({"cmd":"place","on":false}));
            status(view, "cancelled", cx);
            cx.notify();
        }
        "pick" => {
            view.report("pick mode — click an element in the preview".to_owned());
            view.preview("canvas", cx)
                .update(cx, |p, cx| p.set_pick(true, cx));
            host_send(&json!({"cmd":"pick","on":true}));
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
        "quit" => {
            if let Some(mut host) = HOST.lock().unwrap().take() {
                let _ = host.child.kill();
                let _ = host.child.wait();
            }
            cx.quit();
        }
        "test_error" => {
            view.report("test toast — simulated error".to_owned());
            cx.notify();
        }
        other => status(view, &format!("unhandled hook `{other}`"), cx),
    }
}

/// `start_from('welcome'|'gallery'|'blank')` — loads a starter document and shows
/// the preview tab so the user sees the running app.
fn start_from(
    view: &mut ForgeView,
    args: &[String],
    shared: &SharedState,
    cx: &mut Context<ForgeView>,
) {
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
        view.update(cx, move |view, cx| {
            load_file(path, view, shared.clone(), cx);
        })
        .ok();
    })
    .detach();
}

/// `open_project()` — pick a `forge.toml`; the manifest's `forge.document`
/// path resolves relative to it and becomes the loaded document, while
/// `studio.project` tracks the project directory (#49).
fn open_project(shared: SharedState, cx: &mut Context<ForgeView>) {
    let receiver = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: false,
        prompt: Some("Open a Forge project (forge.toml)".into()),
    });
    cx.spawn(async move |view, cx| {
        let Ok(Ok(Some(paths))) = receiver.await else {
            return;
        };
        let Some(toml) = paths.into_iter().next() else {
            return;
        };
        view.update(cx, move |view, cx| {
            load_project(toml, view, shared.clone(), cx);
        })
        .ok();
    })
    .detach();
}

/// `create_project()` — the New Project dialog's Create button (#98): reads the
/// dialog's `name`/`base` fields (its state lands in `dialog.state` as JSON),
/// prompts for a parent folder, scaffolds `forge.toml` + `app.forge.yaml` like
/// `forge new`, then opens the project.
fn create_project(view: &mut ForgeView, shared: SharedState, cx: &mut Context<ForgeView>) {
    let fields = get_path(view.state(), &parse_path("dialog.state").unwrap())
        .and_then(|j| j.as_str())
        .and_then(|s| serde_json::from_str::<Json>(s).ok())
        .unwrap_or_default();
    let name = fields["name"]
        .as_str()
        .unwrap_or_default()
        .trim()
        .to_owned();
    let base = fields["base"]
        .as_str()
        .unwrap_or_default()
        .trim()
        .to_owned();
    let base = if base.is_empty() {
        BASE_ID.to_owned()
    } else {
        base
    };
    if name.is_empty() || name.contains(['/', '\\']) {
        status(view, "project name required (no slashes)", cx);
        return;
    }
    if !check::bases().iter().any(|b| b.id == base) {
        status(view, &format!("unknown base `{base}`"), cx);
        return;
    }
    let receiver = cx.prompt_for_paths(PathPromptOptions {
        files: false,
        directories: true,
        multiple: false,
        prompt: Some("Choose a parent folder for the project".into()),
    });
    cx.spawn(async move |view, cx| {
        let Ok(Ok(Some(paths))) = receiver.await else {
            return;
        };
        let Some(parent) = paths.into_iter().next() else {
            return;
        };
        view.update(cx, move |view, cx| {
            let dir = parent.join(&name);
            if dir.exists() {
                status(view, &format!("{} already exists", dir.display()), cx);
                return;
            }
            if let Err(e) = std::fs::create_dir_all(&dir) {
                status(view, &format!("cannot create {}: {e}", dir.display()), cx);
                return;
            }
            let toml = dir.join("forge.toml");
            let manifest = format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nid = \"dev.forge.{name}\"\n\n[forge]\nbase = \"{base}\"\ndocument = \"app.forge.yaml\"\n"
            );
            let doc = format!(
                "$schema: {}\nforge: \"0.1\"\nbase: {base}\n\nroot:\n  VStack:\n  style:\n    padding: 16px\n    gap: 8px\n  children:\n    - Text: Hello, Forge!\n",
                gpui_forge_runtime::forge_spec::yaml_schema::schema_url(&base)
            );
            if std::fs::write(&toml, &manifest).is_err()
                || std::fs::write(dir.join("app.forge.yaml"), &doc).is_err()
            {
                status(view, "project files could not be written", cx);
                return;
            }
            load_project(toml, view, shared.clone(), cx);
        })
        .ok();
    })
    .detach();
}

/// `preview_in_host()` — spawn the out-of-process preview host (#97) on the
/// current document: the host renders its own GPUI graph, Studio feeds it
/// the doc path over the JSON control channel and reads its events back —
/// `call`/`state` lines play the same role as the embedded preview's sinks.
fn preview_in_host(view: &mut ForgeView, shared: SharedState, cx: &mut Context<ForgeView>) {
    // `load` for file-backed docs (also seeds the host's base_dir); unsaved
    // documents go over `source` — the editor buffer is the document.
    let file = shared.borrow().file.clone();
    // One host binary per base — `forge-preview-gpui02` for gpui02 docs, and
    // the plain `forge-preview` stays the gpui-kit host (#96).
    let base = get_path(view.state(), &parse_path("studio.doc_base").unwrap())
        .and_then(Json::as_str)
        .unwrap_or(BASE_ID)
        .to_owned();
    let dir = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.to_path_buf()));
    let named = dir
        .as_ref()
        .map(|d| d.join(format!("forge-preview-{base}")));
    // Links-conflicted bases host out of a nested workspace — their binary
    // lands in hosts/<base>/target/<profile>/ rather than ./target (#95).
    let nested = dir.as_ref().and_then(|d| {
        let profile = d.file_name()?;
        let root = d.parent()?.parent()?;
        let p = root.join(format!(
            "hosts/{base}/target/{}/forge-preview-{base}",
            profile.to_string_lossy()
        ));
        p.exists().then_some(p)
    });
    let binary = named.filter(|p| p.exists()).or(nested).or_else(|| {
        if base == BASE_ID {
            dir.map(|d| d.join("forge-preview"))
                .or_else(|| Some(std::path::PathBuf::from("forge-preview")))
        } else {
            None
        }
    });
    let Some(binary) = binary else {
        status(
            view,
            &format!("no preview host for base `{base}` — build `forge-preview-{base}`"),
            cx,
        );
        return;
    };
    let spawned = std::process::Command::new(&binary)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(e) => {
            status(view, &format!("cannot spawn forge-preview: {e}"), cx);
            return;
        }
    };
    let Some(stdin) = child.stdin.take() else {
        status(view, "cannot spawn forge-preview: no stdin", cx);
        return;
    };
    let generation = HOST_GEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
    // stdout → HostEvent lines, one JSON value per line. EOF (host exit or a
    // restart's kill) reports `Closed` so the pump loop below can retire.
    let (tx, rx) = std::sync::mpsc::channel::<HostEvent>();
    if let Some(stdout) = child.stdout.take() {
        std::thread::spawn(move || {
            use std::io::BufRead;
            for line in std::io::BufReader::new(stdout)
                .lines()
                .map_while(Result::ok)
            {
                let Ok(v) = serde_json::from_str::<Json>(&line) else {
                    continue;
                };
                let event = match v["event"].as_str() {
                    Some("ready") => HostEvent::Ready {
                        base: v["base"].as_str().unwrap_or_default().to_owned(),
                    },
                    Some("diagnostics") => HostEvent::Diagnostics(
                        v["items"]
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .filter_map(|i| i.as_str().map(str::to_owned))
                                    .collect()
                            })
                            .unwrap_or_default(),
                    ),
                    Some("call") => {
                        HostEvent::Call(v["call"].as_str().unwrap_or_default().to_owned())
                    }
                    Some("state") => HostEvent::State(v["state"].clone()),
                    Some("frame_rgba") => HostEvent::FrameRgba {
                        w: v["w"].as_u64().unwrap_or(0) as u32,
                        h: v["h"].as_u64().unwrap_or(0) as u32,
                        data: v["data"].as_str().unwrap_or_default().to_owned(),
                    },
                    Some("frame") => HostEvent::Frame {
                        rows: v["rows"]
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .filter_map(|r| r.as_str().map(str::to_owned))
                                    .collect()
                            })
                            .unwrap_or_default(),
                    },
                    _ => continue,
                };
                if tx.send(event).is_err() {
                    return;
                }
            }
            let _ = tx.send(HostEvent::Closed { generation });
        });
    }
    // Restart the host against the fresh path — the killed host's reader hits
    // EOF and its pump retires on `Closed` (ignored for the new generation).
    {
        let mut guard = HOST.lock().unwrap();
        if let Some(mut old) = guard.take() {
            let _ = old.child.kill();
            let _ = old.child.wait();
        }
        *guard = Some(Host {
            child,
            stdin,
            generation,
        });
    }
    if let Some(path) = &file {
        host_send(&json!({"cmd":"load","doc":path.display().to_string()}));
    } else {
        host_send_source(view, &shared);
    }
    host_send(&json!({"cmd":"dnd","on":true}));
    host_send_theme(view);
    cx.spawn(async move |view, cx| {
        let mut last_rect: Option<String> = None;
        loop {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(50))
                .await;
            let mut events = Vec::new();
            while let Ok(event) = rx.try_recv() {
                let done = matches!(event, HostEvent::Closed { .. });
                events.push(event);
                if done {
                    break;
                }
            }
            // #120 Phase 1 — poll the canvas slot's bounds + send frame_rect
            // so the host can float its window over the Studio card
            // (fake-embed position tracking).
            if let Ok(Some((WindowBounds::Windowed(wb), b))) = view.update(cx, |view, cx| {
                view.preview_bounds("canvas")
                    .and_then(|cell| *cell.borrow())
                    .and_then(|b| {
                        let wb = cx
                            .windows()
                            .first()
                            .and_then(|h| h.update(cx, |_, w, _| w.window_bounds()).ok());
                        wb.map(|wb| (wb, b))
                    })
            }) {
                // window space → screen space: add the window's origin
                let rect = json!({
                    "cmd": "frame_rect",
                    "x": f64::from(wb.origin.x) + f64::from(b.origin.x),
                    "y": f64::from(wb.origin.y) + f64::from(b.origin.y),
                    "w": f64::from(b.size.width),
                    "h": f64::from(b.size.height),
                });
                // Send only on change — unconditional sends at 20Hz are
                // pure noise for an idle canvas.
                let key = rect.to_string();
                if last_rect.as_deref() != Some(key.as_str()) {
                    last_rect = Some(key);
                    host_send(&rect);
                }
            }
            if events.is_empty() {
                continue;
            }
            let mut done = false;
            let alive = view
                .update(cx, |view, cx| {
                    for event in events.drain(..) {
                        done = matches!(event, HostEvent::Closed { .. });
                        host_event(view, event, &shared, cx);
                    }
                })
                .is_ok();
            if done || !alive {
                return;
            }
        }
    })
    .detach();
    status(view, "preview host launched", cx);
}

/// A line decoded from the host's stdout — the out-of-process twin of the
/// embedded preview's `on_hook`/`on_state` sinks.
fn host_event(
    view: &mut ForgeView,
    event: HostEvent,
    shared: &SharedState,
    cx: &mut Context<ForgeView>,
) {
    match event {
        HostEvent::Ready { base } => {
            view.update_state(
                &parse_path("studio.host").unwrap(),
                Json::String(format!("preview host: {base}")),
                cx,
            );
            status(view, &format!("preview host ready ({base})"), cx);
            // #120 — `FORGE_EMBED` / `?mode=embedded` marks the canvas
            // card embedded: the host floats over it via frame_rect.
            let mode = if std::env::var_os("FORGE_EMBED").is_some() {
                "embedded"
            } else {
                "remote"
            };
            view.update_state(
                &parse_path("preview.mode").unwrap(),
                Json::String(mode.to_owned()),
                cx,
            );
        }
        HostEvent::Diagnostics(items) => {
            // The host checks with ITS base's catalogs — for a foreign-base
            // doc these are the accurate diagnostics, not Studio's own.
            let text = if items.is_empty() {
                "no problems".to_owned()
            } else {
                items.join("\n")
            };
            view.update_state(
                &parse_path("studio.diagnostics").unwrap(),
                Json::String(text),
                cx,
            );
        }
        HostEvent::Call(call) => {
            let (name, args) = parse_call(&call);
            dispatch(view, &name, &args, shared, cx);
            // The dispatch may have mutated the preview doc (place/move/drop)
            // — push the synced source back so the host re-renders it.
            host_send_source(view, shared);
        }
        HostEvent::State(state) => {
            view.update_state(
                &parse_path("preview.state").unwrap(),
                Json::String(state.to_string()),
                cx,
            );
            mirror_document_state(view, &state, cx);
        }
        HostEvent::FrameRgba { w, h, data } => {
            view.update_state(
                &parse_path("preview.frame_rgba").unwrap(),
                Json::String(data),
                cx,
            );
            let _ = (w, h); // dims for the image arm when it lands
        }
        HostEvent::Frame { rows, .. } => {
            // #120 Phase 2 — the TUI's frame grid as `preview.frame`, a
            // `\n`-joined text the card renders monospace.
            view.update_state(
                &parse_path("preview.frame").unwrap(),
                Json::String(rows.join("\n")),
                cx,
            );
        }
        HostEvent::Closed { generation } => {
            let mut guard = HOST.lock().unwrap();
            if guard.as_ref().is_some_and(|h| h.generation == generation) {
                if let Some(mut host) = guard.take() {
                    let _ = host.child.kill();
                    let _ = host.child.wait();
                }
                drop(guard);
                view.update_state(&parse_path("studio.host").unwrap(), Json::Null, cx);
                status(view, "preview host exited", cx);
            }
        }
    }
}

/// State paths the preview (embedded or hosted) mirrors into the shell so
/// outline/inspector interactions work identically either way. Writes are
/// skipped when the value already matches — the host echoes `document.selected`
/// back on `select`, and without the guard that echo would loop forever.
fn mirror_document_state(view: &mut ForgeView, state: &Json, cx: &mut Context<ForgeView>) {
    let selected = parse_path("document.selected").unwrap();
    let value = get_path(state, &selected).cloned().unwrap_or(Json::Null);
    if get_path(view.state(), &selected) != Some(&value) {
        view.update_state(&selected, value, cx);
        if let Some(path) = get_path(view.state(), &selected).and_then(|j| j.as_str()) {
            let path = path.to_owned();
            view.update_state(
                &parse_path("status.message").unwrap(),
                Json::String(format!("selected {path}")),
                cx,
            );
        }
    }
    let drop_target = parse_path("studio.drop_target").unwrap();
    let value = get_path(state, &drop_target).cloned().unwrap_or(Json::Null);
    if get_path(view.state(), &drop_target) != Some(&value) {
        view.update_state(&drop_target, value, cx);
    }
}

/// Sends one JSON line to the running host's stdin — a no-op without a host.
fn host_send(value: &Json) {
    use std::io::Write;
    let mut guard = HOST.lock().unwrap();
    if let Some(host) = guard.as_mut() {
        let _ = writeln!(host.stdin, "{value}");
        let _ = host.stdin.flush();
    }
}

/// Pushes the current `editor.source` to the host — after doc mutations and
/// on every edit, so the out-of-process preview stays live-synced.
fn host_send_source(view: &ForgeView, shared: &SharedState) {
    let dir = shared.borrow().dir.clone();
    host_send(&json!({"cmd":"source","src": source(view),"dir": dir}));
}

/// Sends the current theme to the host — the registry name when `studio.theme`
/// names a config, else just the light/dark mode.
fn host_send_theme(view: &ForgeView) {
    match get_path(view.state(), &parse_path("studio.theme").unwrap())
        .and_then(|j| j.as_str())
        .map(str::to_owned)
    {
        Some(name) => host_send(&json!({"cmd":"theme","name": name})),
        None => {
            let dark = get_path(view.state(), &parse_path("studio.dark").unwrap())
                .and_then(|j| j.as_bool())
                .unwrap_or_default();
            host_send(&json!({"cmd":"theme","dark": dark}));
        }
    }
}

/// Reads `key = "value"` lines out of a manifest's `[forge]` section — the
/// manifest format is flat enough that a scan beats a TOML dependency.
fn manifest_value(src: &str, key: &str) -> Option<String> {
    let forge = src.split("[forge]").nth(1)?.split('[').next()?;
    forge.lines().map(str::trim).find_map(|l| {
        l.strip_prefix(key)?
            .trim_start_matches('=')
            .trim()
            .strip_prefix('"')?
            .strip_suffix('"')
            .map(str::to_owned)
    })
}

fn load_project(
    toml: PathBuf,
    view: &mut ForgeView,
    shared: SharedState,
    cx: &mut Context<ForgeView>,
) {
    let src = match std::fs::read_to_string(&toml) {
        Ok(s) => s,
        Err(e) => {
            status(view, &format!("cannot read {}: {e}", toml.display()), cx);
            return;
        }
    };
    let Some(rel) = manifest_value(&src, "document") else {
        status(view, "forge.toml has no forge.document", cx);
        return;
    };
    let dir = toml.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let doc_path = dir.join(&rel);
    view.update_state(
        &parse_path("studio.project").unwrap(),
        Json::String(dir.display().to_string()),
        cx,
    );
    load_file(doc_path, view, shared, cx);
}

/// Reads `path` into the editor, updates recents + the menu, starts the
/// external-change watcher.
fn load_file(
    path: PathBuf,
    view: &mut ForgeView,
    shared: SharedState,
    cx: &mut Context<ForgeView>,
) {
    match std::fs::read_to_string(&path) {
        Ok(src) => {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string());
            let dir = path.parent().map(|p| p.to_path_buf());
            shared.borrow_mut().file = Some(path.clone());
            load_source(view, &name, &src, dir.as_deref(), &shared, cx);
            let recents = record_recent(&path);
            refresh_recent_menu(view, &recents, cx);
            watch_file(path, shared.clone(), cx);
            view.update_state(&parse_path("studio.tab").unwrap(), Json::from(2), cx);
        }
        Err(e) => status(view, &format!("cannot read {}: {e}", path.display()), cx),
    }
}

/// Recent-files list, persisted one absolute path per line under
/// `~/.local/share/gpui-forge/recent.txt` (first = most recent).
fn recents_file() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".local/share/gpui-forge/recent.txt"))
}

fn load_recents() -> Vec<PathBuf> {
    recents_file()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .map(|s| s.lines().map(PathBuf::from).collect())
        .unwrap_or_default()
}

/// Prepends `path` to the recent list, deduping and capping at 8, then
/// rewrites the store.
/// #123 — persisted preferences at `~/.gpui-forge/settings.toml` —
/// `theme` + `dark` per user, in TOML like a real settings file.
fn prefs_file() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".gpui-forge/settings.toml"))
}

fn load_prefs() -> Json {
    let s = prefs_file()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .unwrap_or_default();
    // tiny TOML-ish parse: `key = value` lines; strings quoted, bool bare.
    let mut m = serde_json::Map::new();
    for line in s.lines() {
        let line = line.trim();
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let (k, v) = (k.trim(), v.trim());
        if v == "true" || v == "false" {
            m.insert(k.to_owned(), Json::Bool(v == "true"));
        } else {
            m.insert(k.to_owned(), Json::String(v.trim_matches('"').to_owned()));
        }
    }
    Json::Object(m)
}

fn save_prefs(view: &ForgeView) {
    let theme = view
        .state()
        .get("studio")
        .and_then(|s| s.get("theme"))
        .and_then(|j| j.as_str())
        .unwrap_or_default()
        .to_owned();
    let dark = view
        .state()
        .get("studio")
        .and_then(|s| s.get("dark"))
        .and_then(|j| j.as_bool())
        .unwrap_or(false);
    if let Some(f) = prefs_file() {
        let _ = std::fs::create_dir_all(f.parent().unwrap());
        let _ = std::fs::write(f, format!("theme = \"{theme}\"\ndark = {dark}\n"));
    }
}

fn record_recent(path: &PathBuf) -> Vec<PathBuf> {
    let recents: Vec<PathBuf> = std::iter::once(path.clone())
        .chain(load_recents().into_iter().filter(|p| p != path))
        .take(8)
        .collect();
    if let Some(f) = recents_file() {
        let _ = std::fs::create_dir_all(f.parent().unwrap());
        let mut body = recents
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("\n");
        body.push('\n');
        let _ = std::fs::write(f, body);
    }
    recents
}

/// Path of the first node with `id == needle` — used to find named shell
/// elements like `file-menu` without hardcoding layout indexes.
fn find_by_id(node: &Node, path: &str, needle: &str) -> Option<String> {
    if node.id.as_deref() == Some(needle) {
        return Some(path.to_string());
    }
    for (i, child) in node.children.iter().enumerate() {
        if let Some(p) = find_by_id(child, &format!("{path}.{i}"), needle) {
            return Some(p);
        }
    }
    None
}

/// Rebuilds the "Open Recent" section of the File menu by mutating the
/// studio's own shell document — dogfooding the same Document mutation the
/// preview editing uses.
fn refresh_recent_menu(view: &mut ForgeView, recents: &[PathBuf], cx: &mut Context<ForgeView>) {
    use gpui_forge_runtime::forge_spec::{ElementRef, EventBinding, HandlerRef, Node, Value};
    let Some(mut doc) = view.document().cloned() else {
        return;
    };
    let Some(menu_path) = find_by_id(&doc.root, "0", "file-menu") else {
        return;
    };
    let Some(menu) = doc.node_at_mut(&menu_path) else {
        return;
    };
    menu.children
        .retain(|c| !c.id.as_deref().is_some_and(|id| id.starts_with("recent-")));
    if recents.is_empty() {
        view.set_document(doc, cx);
        return;
    }
    let sep = Node {
        element: ElementRef {
            namespace: None,
            name: "MenuSeparator".into(),
        },
        ..Default::default()
    };
    let items: Vec<Node> = recents
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let label = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.display().to_string());
            Node {
                element: ElementRef {
                    namespace: None,
                    name: "MenuItem".into(),
                },
                id: Some(format!("recent-{i}")),
                content: Some(Value::String(label)),
                events: vec![EventBinding {
                    event: "click".into(),
                    handler: HandlerRef {
                        name: "open_recent".into(),
                        args: vec![Value::String(p.display().to_string())],
                    },
                }],
                ..Default::default()
            }
        })
        .collect();
    // Drop the recent block in right after "Open…" (index 1).
    menu.children
        .splice(2..2, std::iter::once(sep).chain(items));
    view.set_document(doc, cx);
}

/// Watches `path` for external edits — polls mtime, reloads when the on-disk
/// contents differ from the editor (so our own saves don't trigger a reload).
/// Stops when a different file is opened.
fn watch_file(path: PathBuf, shared: SharedState, cx: &mut Context<ForgeView>) {
    cx.spawn(async move |view, cx| {
        let mut mtime = std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok());
        loop {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(1))
                .await;
            if shared.borrow().file.as_deref() != Some(path.as_path()) {
                return;
            }
            let new = std::fs::metadata(&path)
                .ok()
                .and_then(|m| m.modified().ok());
            if new.is_none() || new == mtime {
                continue;
            }
            mtime = new;
            let Ok(src) = std::fs::read_to_string(&path) else {
                continue;
            };
            let path = path.clone();
            let shared = shared.clone();
            view.update(cx, move |view, cx| {
                if source(view) != src {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.display().to_string());
                    let dir = path.parent().map(|p| p.to_path_buf());
                    load_source(view, &name, &src, dir.as_deref(), &shared, cx);
                    status(view, "reloaded — file changed on disk", cx);
                }
            })
            .ok();
        }
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
    set_path(
        &mut state,
        &parse_path("editor.source").unwrap(),
        Json::String(src.to_owned()),
    );
    set_path(
        &mut state,
        &parse_path("studio.file").unwrap(),
        Json::String(name.to_owned()),
    );
    set_path(
        &mut state,
        &parse_path("studio.dirty").unwrap(),
        Json::Bool(false),
    );
    set_path(
        &mut state,
        &parse_path("studio.diagnostics").unwrap(),
        Json::String(problems(&diagnostics)),
    );
    set_path(
        &mut state,
        &parse_path("studio.doc_base").unwrap(),
        Json::String(
            doc.as_ref()
                .and_then(|d| d.base.clone())
                .unwrap_or_else(|| BASE_ID.to_owned()),
        ),
    );
    set_path(
        &mut state,
        &parse_path("status.message").unwrap(),
        Json::String(problem_count(&diagnostics)),
    );
    view.set_state(state, cx);
    // Sync the editor's text without firing `source_changed` — a fresh load
    // is clean, not a user edit.
    view.reset_inputs("source");
    if let Some(dir) = dir {
        shared.borrow_mut().dir = Some(dir.to_path_buf());
    }
    if let Some(doc) = doc {
        view.set_preview_document("canvas", doc, cx);
        if let Some(dir) = dir {
            view.preview("canvas", cx)
                .update(cx, |p, _| p.set_base_dir(dir.to_path_buf()));
        }
    }
    host_send_source(view, shared);
}

/// `on change: source_changed()` — re-check the source on every edit and keep the
/// preview showing the last valid document.
fn reload(view: &mut ForgeView, shared: &SharedState, cx: &mut Context<ForgeView>) {
    view.update_state(&parse_path("studio.dirty").unwrap(), Json::Bool(true), cx);
    let src = source(view);
    let catalogs = shared.borrow().catalogs.clone();
    let (doc, diagnostics) = check::check(&src, BASE_ID, &catalogs);
    view.update_state(
        &parse_path("studio.diagnostics").unwrap(),
        Json::String(problems(&diagnostics)),
        cx,
    );
    view.update_state(
        &parse_path("studio.doc_base").unwrap(),
        Json::String(
            doc.as_ref()
                .and_then(|d| d.base.clone())
                .unwrap_or_else(|| BASE_ID.to_owned()),
        ),
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
    host_send_source(view, shared);
}

fn save_document(view: &mut ForgeView, shared: &SharedState, cx: &mut Context<ForgeView>) {
    let Some(path) = shared.borrow().file.clone() else {
        save_as(shared.clone(), cx);
        return;
    };
    match std::fs::write(&path, source(view)) {
        Ok(()) => {
            view.update_state(&parse_path("studio.dirty").unwrap(), Json::Bool(false), cx);
            status(view, &format!("saved {}", path.display()), cx);
        }
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
                    view.update_state(&parse_path("studio.dirty").unwrap(), Json::Bool(false), cx);
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.display().to_string());
                    shared.borrow_mut().file = Some(path.clone());
                    let recents = record_recent(&path);
                    refresh_recent_menu(view, &recents, cx);
                    watch_file(path.clone(), shared.clone(), cx);
                    view.update_state(&parse_path("studio.file").unwrap(), Json::String(name), cx);
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
#[forge_hooks_macros::forge_hook]
fn insert_component(
    view: &mut ForgeView,
    _ctx: &HookCtx,
    args: &[String],
    cx: &mut Context<ForgeView>,
) {
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
        host_send(&json!({"cmd":"place","on":false}));
        status(view, "place cancelled", cx);
        return;
    }
    view.update_state(
        &parse_path("studio.placing").unwrap(),
        Json::String(tag.clone()),
        cx,
    );
    preview.update(cx, |p, cx| p.set_placing(true, cx));
    host_send(&json!({"cmd":"place","on":true}));
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
fn place_component(
    view: &mut ForgeView,
    args: &[String],
    shared: &SharedState,
    cx: &mut Context<ForgeView>,
) {
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
        // Namespaced tag — make sure the `use:` import for the alias exists.
        if let Some(ns) = node.element.namespace.as_ref()
            && !doc.imports.iter().any(|i| &i.alias == ns)
        {
            let library = view
                .libraries
                .iter()
                .find(|l| l.alias() == ns.as_str())
                .and_then(|l| l.catalog().map(|c| c.library.clone()))
                .unwrap_or_else(|| ns.clone());
            doc.imports.push(gpui_forge_runtime::forge_spec::Import {
                alias: ns.clone(),
                library,
                version: None,
            });
        }
        let target = target
            .as_deref()
            .filter(|path| doc.node_at(path).is_some())
            .unwrap_or("0");
        let into = target == "0" || accepts_children(&shared.borrow().catalogs, &doc, target);
        // Container targets take the node inside; leaf targets take it as
        // the sibling right after.
        let result = if into {
            doc.insert_node(target, node, None).then(|| {
                format!(
                    "{target}.{}",
                    doc.node_at(target).unwrap().children.len() - 1
                )
            })
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
            view.update_state(&parse_path("editor.source").unwrap(), Json::String(src), cx);
            view.reset_inputs("source");
        }
    }
    cx.notify();
}

/// A palette item dropped on a preview node or outline row —
/// `drop_component('<tag>', '<target-path>')`.
fn drop_component(
    view: &mut ForgeView,
    args: &[String],
    shared: &SharedState,
    cx: &mut Context<ForgeView>,
) {
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
fn move_node(
    view: &mut ForgeView,
    args: &[String],
    shared: &SharedState,
    cx: &mut Context<ForgeView>,
) {
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
        let into = target == "0" || accepts_children(&shared.borrow().catalogs, &doc, &target);
        let result = if into {
            doc.reparent(&src, &target, None).then(|| {
                format!(
                    "{target}.{}",
                    doc.node_at(&target).unwrap().children.len() - 1
                )
            })
        } else {
            doc.move_after(&src, &target)
        };
        if result.is_some() {
            moved = result;
            p.set_document(doc, cx);
        }
    });
    if let Some(path) = moved {
        view.update_state(
            &parse_path("document.selected").unwrap(),
            Json::String(path.clone()),
            cx,
        );
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
        // Namespaced tag — make sure the `use:` import for the alias exists.
        if let Some(ns) = node.element.namespace.as_ref()
            && !doc.imports.iter().any(|i| &i.alias == ns)
        {
            let library = view
                .libraries
                .iter()
                .find(|l| l.alias() == ns.as_str())
                .and_then(|l| l.catalog().map(|c| c.library.clone()))
                .unwrap_or_else(|| ns.clone());
            doc.imports.push(gpui_forge_runtime::forge_spec::Import {
                alias: ns.clone(),
                library,
                version: None,
            });
        }
        let target = target
            .as_deref()
            .filter(|path| doc.node_at(path).is_some())
            .unwrap_or("0");
        let into = target == "0" || accepts_children(&shared.borrow().catalogs, &doc, target);
        // Container targets take the node inside; leaf targets take it as
        // the sibling right after.
        let result = if into {
            doc.insert_node(target, node, None).then(|| {
                format!(
                    "{target}.{}",
                    doc.node_at(target).unwrap().children.len() - 1
                )
            })
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
            c.element(&node.element.name).is_some_and(|def| {
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
    let (namespace, name) = match tag.split_once('.') {
        Some((ns, n)) => (Some(ns.to_owned()), n.to_owned()),
        None => (None, tag.to_owned()),
    };
    Node {
        element: ElementRef { name, namespace },
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
    let parent = path
        .rsplit_once('.')
        .map(|(p, _)| p.to_owned())
        .unwrap_or_else(|| "0".into());
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
    view.update_state(&parse_path("editor.source").unwrap(), Json::String(src), cx);
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

#[forge_hooks_macros::forge_hook]
fn copy_snippet(
    view: &mut ForgeView,
    _ctx: &HookCtx,
    args: &[String],
    cx: &mut Context<ForgeView>,
) {
    let Some(name) = args.first() else {
        return;
    };
    let Some(snippet) = snippet(name) else {
        status(view, &format!("no snippet for `{name}`"), cx);
        return;
    };
    cx.write_to_clipboard(ClipboardItem::new_string(snippet.to_owned()));
    status(
        view,
        &format!("copied a `{name}` snippet to the clipboard"),
        cx,
    );
}

/// A node snippet for each palette element.
fn snippet(name: &str) -> Option<&'static str> {
    Some(match name {
        "Button" => "- Button: Label",
        "Input" => "- Input:\n  value: bind $form.field\n  placeholder: Text",
        "Textarea" => "- Textarea:\n  value: bind $form.text\n  rows: 3",
        "Checkbox" => "- Checkbox: Label\n  checked: bind $form.flag",
        "Switch" => "- Switch: Label\n  checked: bind $form.flag",
        "RadioGroup" => {
            "- RadioGroup:\n  selected: bind $form.choice\n  children:\n    - Radio: One\n    - Radio: Two"
        }
        "Select" => "- Select:\n  options: [One, Two]\n  value: bind $form.choice",
        "Tabs" => {
            "- Tabs:\n  children:\n    - Tab: One\n      children:\n        - Text: first panel\n    - Tab: Two\n      children:\n        - Text: second panel"
        }
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
fn style_changed(
    view: &mut ForgeView,
    args: &[String],
    shared: &SharedState,
    cx: &mut Context<ForgeView>,
) {
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

fn prop_changed(
    view: &mut ForgeView,
    args: &[String],
    shared: &SharedState,
    cx: &mut Context<ForgeView>,
) {
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
        view.update_state(&parse_path("editor.source").unwrap(), Json::String(src), cx);
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
    host_send(&json!({"cmd":"select","path": path}));
    view.update_state(
        &parse_path("status.message").unwrap(),
        Json::String(format!("selected {path}")),
        cx,
    );
}
