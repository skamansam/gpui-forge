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
use gpui_forge_runtime::forge_spec::{
    Control, Diagnostic, ElementRef, Node, Value, path_steps, path_string, yaml,
};
use gpui_forge_runtime::render::{ForgeView, get_path, pane_state_path, set_path};
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::theme::ActiveTheme as _;
use gpui_kit::component::theme::ThemeSet;
use gpui_kit::component::theme::{Theme, ThemeMode, ThemeRegistry};
use gpui_kit::{prelude::*, *};
use serde_json::{Value as Json, json};

const SHELL: &str = include_str!("../studio.forge.yaml");
const WELCOME: &str = include_str!("../welcome.forge.yaml");
const GALLERY: &str = include_str!("../gallery.forge.yaml");
const GALLERY_FORGE: &str = include_str!("../gallery.forge.forge.yaml");
const BLANK: &str = "$schema: https://gpui-forge.rudeboy.dev/schema/0.1/gpui-kit.json\nforge: \"0.1\"\nbase: gpui-kit\n\nroot:\n  VStack:\n  style:\n    padding: 16px\n  children:\n    - Text: Hello, Forge!\n";

// One demo per widget set, shared with `examples/` and the docs site (#129).
const EXAMPLE_ELY: &str = include_str!("../../../examples/demos/ely-demo.forge.yaml");
const EXAMPLE_GC: &str = include_str!("../../../examples/showcases/gc-showcase.forge.yaml");
const EXAMPLE_GFLOW: &str = include_str!("../../../examples/demos/gflow-demo.forge.yaml");
const EXAMPLE_BASE: &str = include_str!("../../../examples/demos/base-demo.forge.yaml");
const EXAMPLE_VIDEO: &str = include_str!("../../../examples/demos/video-demo.forge.yaml");
const EXAMPLE_PDF: &str = include_str!("../../../examples/demos/pdf-demo.forge.yaml");
const EXAMPLE_WB: &str = include_str!("../../../examples/demos/whiteboard-demo.forge.yaml");
const EXAMPLE_MATH: &str = include_str!("../../../examples/demos/math-demo.forge.yaml");
const EXAMPLE_HIG: &str = include_str!("../../../examples/demos/hig-demo.forge.yaml");
// Foreign-base showcases — they render in the base's own preview host.
const EXAMPLE_GPUI02: &str = include_str!("../../../examples/showcases/gpui02-showcase.forge.yaml");
const EXAMPLE_BEZEL: &str = include_str!("../../../examples/showcases/bezel-showcase.forge.yaml");
const EXAMPLE_RATATUI: &str =
    include_str!("../../../examples/showcases/ratatui-showcase.forge.yaml");
const EXAMPLE_ICED: &str = include_str!("../../../examples/showcases/iced-showcase.forge.yaml");
const EXAMPLE_ADABRAKA: &str =
    include_str!("../../../examples/showcases/adabraka-showcase.forge.yaml");

/// Widget sets the Welcome tab offers in its showcase picker (#129): the
/// gpui-kit base, the `forge` intrinsics, every registered `forge-lib-*`.
/// Then the foreign-base showcases (they launch their preview host).
const WIDGET_SETS: &[&str] = &[
    "gpui-kit", "forge", "ely", "gc", "gflow", "base", "video", "hig", "pdf", "wb", "math",
    "gpui02", "bezel", "ratatui", "iced", "adabraka",
];

struct Shared {
    /// The file `editor.source` was loaded from. `None` for embedded documents.
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
    /// `check` items from the host — `{line, col, message, hint?}` maps (older
    /// hosts sent bare strings. Both are handled).
    Diagnostics(Vec<Json>),
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
            // Reap the preview hosts on any quit path — the `quit` hook is
            // only one of them (window close, Cmd-Q…). Pane hosts live in
            // HOSTS — without this they'd orphan and survive Studio's exit.
            cx.on_app_quit(|_| async move {
                if let Some(mut host) = HOST.lock().unwrap().take() {
                    let _ = host.child.kill();
                    let _ = host.child.wait();
                }
                for (_, (_, mut host)) in HOSTS.lock().unwrap().drain() {
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

    view.register_library(forge_lib_ely::Ely::new());
    view.register_library(forge_lib_gpui_component::GpuiComponent);
    view.register_library(forge_lib_video::VideoPlayer::new());
    view.register_library(forge_lib_zorite::Pdf::new());
    view.register_library(forge_lib_zorite::Whiteboard::new());
    view.register_library(forge_lib_zorite::Math::new());
    view.register_library(forge_lib_hig::Hig);
    view.register_library(forge_lib_base::Base::new());
    view.register_library(forge_lib_flow::Flow::new());
    view.set_base_dir(env!("CARGO_MANIFEST_DIR"));
    let dark = Theme::global(cx).is_dark();
    // Load every vendored theme JSON — `default-theme.json` is only
    // light+dark. The full catalog lives in the workspace `themes/` dir.
    // Load every vendored theme JSON — `default-theme.json` is only
    // light+dark. The full catalog lives in the workspace `themes/` dir.
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
    let mut theme_json = String::new();
    if let Some(config) = ThemeRegistry::global(cx)
        .themes()
        .get(theme_name.as_str())
        .cloned()
    {
        Theme::update(cx, |theme| theme.apply_config(&config));
        theme_json = serde_json::to_string_pretty(&ThemeSet {
            name: config.name.clone(),
            themes: vec![(*config).clone()],
            ..Default::default()
        })
        .unwrap_or_default();
    }
    view.set_state(
        json!({
            "studio": {
                "dark": dark,
                "light_themes": light_themes,
                "dark_themes": dark_themes,
                "all_themes": all_themes,
                "theme": theme_name,
                "theme_json": theme_json,
                "theme_preview": BASE_ID,
                "file": "welcome.forge.yaml",
                "diagnostics": "",
                "widget_sets": WIDGET_SETS,
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
        "refname_changed",
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

    // Hooks fired inside the preview bubble up to the shell dispatch. The preview's
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
                // Defer — a preview event fires while the preview entity is leased.
                // Touching it (or the shell, which then touches the preview)
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
    // Declared panes in the shell document (the Themes-tab theme preview)
    // spawn hosts from startup — no need to wait for a foreign-base open.
    pane_sync(shared.clone(), cx);
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
    let libraries = view.libraries.clone();
    let shell = cx.entity().downgrade();
    let shared = shared.clone();
    let child = cx.new(|cx| {
        let mut v = ForgeView::new(catalogs);
        v.libraries = libraries;
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
                refs: vec![],
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
    // The hook runs inside the main window's own update cycle. `App::windows()`
    // Handles can't be `update`d from in here because the window is taken out of
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
/// and fires this. Swaps the theme config + mode to match.
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
    // and applies colors. `edit` sees it as already installed and skips the
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
    host_send_theme(view, cx);
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
    // selected in-session. Falls back to the mode default.
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
    host_send_theme(view, cx);
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
            host_send_theme(view, cx);
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
    // no arm here. Anything unhandled falls through to the legacy match.
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
            let alias = args.first().map(String::as_str).unwrap_or("gpui-kit");
            start_from(view, &[alias.to_owned()], shared, cx);
        }
        "select_node" => select_node(view, args, cx),
        "prop_changed" => prop_changed(view, args, shared, cx),
        "style_changed" => style_changed(view, args, shared, cx),
        "refname_changed" => refname_changed(view, shared, cx),
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

/// `start_from('welcome'|'gallery'|'blank'|<widget-set>)` — loads a starter
/// document and shows the preview tab so the user sees the running app.
/// Widget-set aliases resolve to their demo in `examples/{demos,showcases}/` (#129). The paths
/// are relative to the crate so doc-relative `src:` props resolve too.
fn starter(name: &str) -> Option<(&'static str, &'static str)> {
    Some(match name {
        "welcome" | "welcome.forge.yaml" => ("welcome.forge.yaml", WELCOME),
        "gallery" | "gpui-kit" | "gallery.forge.yaml" => ("gallery.forge.yaml", GALLERY),
        "forge" | "gallery.forge.forge.yaml" => ("gallery.forge.forge.yaml", GALLERY_FORGE),
        "blank" | "blank.forge.yaml" => ("blank.forge.yaml", BLANK),
        "ely" | "ely-demo.forge.yaml" => ("../../examples/demos/ely-demo.forge.yaml", EXAMPLE_ELY),
        "gc" | "gc-showcase.forge.yaml" => (
            "../../examples/showcases/gc-showcase.forge.yaml",
            EXAMPLE_GC,
        ),
        "gflow" | "gflow-demo.forge.yaml" => {
            ("../../examples/demos/gflow-demo.forge.yaml", EXAMPLE_GFLOW)
        }
        "base" | "base-demo.forge.yaml" => {
            ("../../examples/demos/base-demo.forge.yaml", EXAMPLE_BASE)
        }
        "video" | "video-demo.forge.yaml" => {
            ("../../examples/demos/video-demo.forge.yaml", EXAMPLE_VIDEO)
        }
        "pdf" | "pdf-demo.forge.yaml" => ("../../examples/demos/pdf-demo.forge.yaml", EXAMPLE_PDF),
        "wb" | "whiteboard-demo.forge.yaml" => (
            "../../examples/demos/whiteboard-demo.forge.yaml",
            EXAMPLE_WB,
        ),
        "math" | "math-demo.forge.yaml" => {
            ("../../examples/demos/math-demo.forge.yaml", EXAMPLE_MATH)
        }
        "hig" | "hig-demo.forge.yaml" => ("../../examples/demos/hig-demo.forge.yaml", EXAMPLE_HIG),
        "gpui02" | "gpui02-showcase.forge.yaml" => (
            "../../examples/showcases/gpui02-showcase.forge.yaml",
            EXAMPLE_GPUI02,
        ),
        "bezel" | "bezel-showcase.forge.yaml" => (
            "../../examples/showcases/bezel-showcase.forge.yaml",
            EXAMPLE_BEZEL,
        ),
        "ratatui" | "ratatui-showcase.forge.yaml" => (
            "../../examples/showcases/ratatui-showcase.forge.yaml",
            EXAMPLE_RATATUI,
        ),
        "iced" | "iced-showcase.forge.yaml" => (
            "../../examples/showcases/iced-showcase.forge.yaml",
            EXAMPLE_ICED,
        ),
        "adabraka" | "adabraka-showcase.forge.yaml" => (
            "../../examples/showcases/adabraka-showcase.forge.yaml",
            EXAMPLE_ADABRAKA,
        ),
        _ => return None,
    })
}

fn start_from(
    view: &mut ForgeView,
    args: &[String],
    shared: &SharedState,
    cx: &mut Context<ForgeView>,
) {
    let name = args.first().map(String::as_str).unwrap_or("welcome");
    let Some((path, fallback)) = starter(name) else {
        status(view, &format!("no starter `{name}`"), cx);
        return;
    };
    // Read the real file from disk — `include_str!` would be stale the moment the
    // user edits it. Falls back to embedded.
    let full = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    let src = std::fs::read_to_string(&full)
        .ok()
        .unwrap_or_else(|| fallback.to_owned());
    load_source(
        view,
        path.rsplit('/').next().unwrap_or(path),
        &src,
        full.parent(),
        shared,
        cx,
    );
    view.update_state(&parse_path("studio.tab").unwrap(), Json::from(2), cx);
    // A foreign-base demo can't render in the embedded (gpui-kit) canvas —
    // spawn the base's preview host so the showcase actually shows (#97).
    let doc_base = get_path(view.state(), &parse_path("studio.doc_base").unwrap())
        .and_then(Json::as_str)
        .unwrap_or(BASE_ID)
        .to_owned();
    if doc_base != BASE_ID {
        preview_in_host(view, shared.clone(), cx);
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

/// `open_project()` — pick a `forge.toml`. The manifest's `forge.document`
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

/// `create_project()` — the New Project dialog's Create button (#98). It reads the
/// dialog's `name`/`base` fields (its state lands in `dialog.state` as JSON)
/// and loads a fresh `app.forge.yaml`-style document into the editor.
///
/// #133 — no folder prompt: a project is one YAML file, so nothing needs a
/// path until an operation does. `save_document` → `save_as` asks where to
/// put it. `forge.toml` scaffolding waits for `forge new`/`forge build`.
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
    let doc = format!(
        "$schema: {}\nforge: \"0.1\"\nbase: {base}\n\nroot:\n  VStack:\n  style:\n    padding: 16px\n    gap: 8px\n  children:\n    - Text: Hello, Forge!\n",
        gpui_forge_runtime::forge_spec::yaml_schema::schema_url(&base)
    );
    shared.borrow_mut().file = None;
    shared.borrow_mut().dir = None;
    load_source(view, &format!("{name}.forge.yaml"), &doc, None, &shared, cx);
    view.update_state(
        &parse_path("studio.project").unwrap(),
        Json::String(String::new()),
        cx,
    );
    view.update_state(&parse_path("studio.dirty").unwrap(), Json::Bool(true), cx);
    view.update_state(&parse_path("studio.tab").unwrap(), Json::from(2), cx);
    status(
        view,
        &format!("new project `{name}` — unsaved; pick a folder on first save"),
        cx,
    );
}

/// `preview_in_host()` — spawn the out-of-process preview host (#97) on the
/// current document. The host renders its own GPUI graph. Studio feeds it
/// the doc path over the JSON control channel and reads its events back.
/// `call`/`state` lines play the same role as the embedded preview's sinks.
fn preview_in_host(view: &mut ForgeView, shared: SharedState, cx: &mut Context<ForgeView>) {
    // `load` for file-backed docs (also seeds the host's base_dir). Unsaved
    // documents go over `source` — the editor buffer is the document.
    let file = shared.borrow().file.clone();
    // One host binary per base — `forge-preview-gpui02` for gpui02 docs, and
    // the plain `forge-preview` stays the gpui-kit host (#96).
    let base = get_path(view.state(), &parse_path("studio.doc_base").unwrap())
        .and_then(Json::as_str)
        .unwrap_or(BASE_ID)
        .to_owned();
    let Some(binary) = find_preview_binary(&base) else {
        status(
            view,
            &format!("no preview host for base `{base}` — build `forge-preview-{base}`"),
            cx,
        );
        return;
    };
    let spawned = preview_command(&binary, &base)
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
                    Some("diagnostics") => {
                        HostEvent::Diagnostics(v["items"].as_array().cloned().unwrap_or_default())
                    }
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
    host_send_theme(view, cx);
    pane_sync(shared.clone(), cx);
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
                items
                    .iter()
                    .map(|i| {
                        i.as_str().map(str::to_owned).unwrap_or_else(|| {
                            let line = i["line"].as_u64().unwrap_or(0);
                            let col = i["col"].as_u64().unwrap_or(0);
                            let msg = i["message"].as_str().unwrap_or_default();
                            match i["hint"].as_str() {
                                Some(hint) => format!("{line}:{col}: {msg} — {hint}"),
                                None => format!("{line}:{col}: {msg}"),
                            }
                        })
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            // Squiggle markers for the source editor — objects only. Bare
            // strings carry no position.
            let markers = Json::Array(items.iter().filter(|i| i.is_object()).cloned().collect());
            view.update_state(
                &parse_path("studio.diagnostics").unwrap(),
                Json::String(text),
                cx,
            );
            view.update_state(&parse_path("studio.markers").unwrap(), markers, cx);
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
/// The `theme` command payload — registry name when `studio.theme` names a
/// theme, else just the light/dark flag. Shared by the main host and pane
/// hosts so a theme preview applies to every live preview.
/// The `theme` command: name and/or dark flag plus the resolved palette.
/// Foreign hosts can't look Forge theme names up, but `colors` gives every
/// host the actual hex values to paint with — `Hsla` serializes to
/// `#rrggbbaa`, so `ThemeColor`'s `Serialize` produces `{field: "#hex"}`.
fn theme_cmd(view: &ForgeView, theme: &Theme) -> Json {
    let colors = serde_json::to_value(theme.colors).unwrap_or_default();
    match get_path(view.state(), &parse_path("studio.theme").unwrap())
        .and_then(|j| j.as_str())
        .map(str::to_owned)
    {
        Some(name) => json!({"cmd":"theme","name": name,"colors": colors}),
        None => {
            let dark = get_path(view.state(), &parse_path("studio.dark").unwrap())
                .and_then(|j| j.as_bool())
                .unwrap_or_default();
            json!({"cmd":"theme","dark": dark,"colors": colors})
        }
    }
}

fn host_send_theme(view: &ForgeView, cx: &App) {
    host_send(&theme_cmd(view, cx.theme()));
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

/// The `forge-preview{,-base}` binary for `base` — next to the running
/// executable, or in the nested `hosts/<base>` workspace for links-conflicted
/// bases (#95). `None` when no host exists for the base.
fn find_preview_binary(base: &str) -> Option<PathBuf> {
    let dir = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.to_path_buf()));
    let named = dir
        .as_ref()
        .map(|d| d.join(format!("forge-preview-{base}")));
    let nested = dir.as_ref().and_then(|d| {
        let profile = d.file_name()?;
        let root = d.parent()?.parent()?;
        let p = root.join(format!(
            "hosts/{base}/target/{}/forge-preview-{base}",
            profile.to_string_lossy()
        ));
        p.exists().then_some(p)
    });
    named.filter(|p| p.exists()).or(nested).or_else(|| {
        (base == BASE_ID).then(|| {
            dir.map(|d| d.join("forge-preview"))
                .unwrap_or_else(|| PathBuf::from("forge-preview"))
        })
    })
}

// ── Declared remote preview panes ────────────────────────────────────
// A `forge.Preview` with `remote:` + `id:` declares its own preview host —
// `base:` picks the host binary (empty → the document's base) and `doc:`
// picks the document (a doc-relative path, a named ref's head-less body, or
// the parent document when absent). `pane_sync` diffs declared panes against
// running hosts, spawns/kills deltas, streams each pane's document, and
// `frame_rect`s each host window over its tracked slot.

/// A remote `forge.Preview` pane declared in the open document.
struct PaneSpec {
    /// The pane's `id:` — also the `preview_bounds` key its slot tracks.
    id: String,
    base: String,
    /// `doc:` — doc-relative path, ref name, or none (the parent document).
    doc: Option<String>,
    /// `mode:` — `embedded` floats the host window over the slot; `stream`
    /// routes the host's `frame` events into `preview.panes.<id>.frame`.
    mode: String,
}

impl PaneSpec {
    /// Panes that want the slot rect — windowed hosts float on it, streaming
    /// hosts use it to size their headless grid.
    fn wants_frame_rect(&self) -> bool {
        self.mode == "embedded" || self.mode == "stream"
    }
}

static HOSTS: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<String, (String, Host)>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));
static PANE_SYNC: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn pane_send(id: &str, value: &Json) {
    use std::io::Write;
    let mut guard = HOSTS.lock().unwrap();
    if let Some((_, host)) = guard.get_mut(id) {
        let _ = writeln!(host.stdin, "{value}");
        let _ = host.stdin.flush();
    }
}

/// Does any top-level X11 window carry `_NET_WM_PID == pid`? `None` when the
/// check can't run (no X connection — e.g. Wayland-only) — the caller treats
/// that as "can't tell", not "no window".
#[cfg(target_os = "linux")]
fn x11_pid_has_window(pid: u32) -> Option<bool> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _};
    let (conn, screen_num) = x11rb::connect(None).ok()?;
    let root = conn.setup().roots.get(screen_num)?.root;
    let pid_atom = conn
        .intern_atom(false, b"_NET_WM_PID")
        .ok()?
        .reply()
        .ok()?
        .atom;
    let tree = conn.query_tree(root).ok()?.reply().ok()?;
    for win in tree.children {
        let ours = conn
            .get_property(false, win, pid_atom, AtomEnum::CARDINAL, 0, 1)
            .ok()
            .and_then(|c| c.reply().ok())
            .and_then(|r| r.value32().and_then(|mut v| v.next()))
            == Some(pid);
        if ours {
            return Some(true);
        }
    }
    Some(false)
}

#[cfg(not(target_os = "linux"))]
fn x11_pid_has_window(_pid: u32) -> Option<bool> {
    None
}

/// Literal string prop — pane declarations are static metadata, not bindings.
fn pane_prop<'a>(node: &'a Node, name: &str) -> Option<&'a str> {
    node.props
        .iter()
        .find(|(k, _)| k == name)
        .and_then(|(_, v)| match v {
            Value::String(s) | Value::Enum(s) => Some(s.as_str()),
            _ => None,
        })
}

/// Every `forge.Preview` with a non-empty `remote:` in the *rendered* tree —
/// control flow evaluates against view state, so a `match`/`if` arm's pane
/// only spawns while that branch is live. Panes without `id:` can't be
/// addressed — counted in `skipped`.
fn remote_panes(view: &ForgeView, node: &Node, out: &mut Vec<PaneSpec>, skipped: &mut usize) {
    if node.element.namespace.as_deref() == Some("forge")
        && node.element.name == "Preview"
        && pane_prop(node, "remote").is_some_and(|s| !s.is_empty())
    {
        match node.id.clone() {
            Some(id) => out.push(PaneSpec {
                id,
                base: pane_prop(node, "base").unwrap_or_default().to_owned(),
                doc: pane_prop(node, "doc").map(str::to_owned),
                mode: pane_prop(node, "mode").unwrap_or_default().to_owned(),
            }),
            None => *skipped += 1,
        }
    }
    // Which children render decides which panes exist — walk the branch the
    // control flow actually selects, never a hidden arm.
    match &node.control {
        Some(Control::If {
            when,
            test,
            otherwise,
        }) => {
            let arm = if view.condition(when, test) {
                &node.children
            } else {
                otherwise
            };
            for child in arm {
                remote_panes(view, child, out, skipped);
            }
        }
        Some(Control::For {
            source, otherwise, ..
        }) => {
            let arm = if view.for_items(source).is_empty() {
                otherwise
            } else {
                &node.children
            };
            for child in arm {
                remote_panes(view, child, out, skipped);
            }
        }
        Some(Control::Match {
            source,
            cases,
            otherwise,
            ..
        }) => {
            let resolved = view.resolve(source);
            let arm = cases
                .iter()
                .find(|c| view.value_eq(&view.resolve(&c.value), &resolved))
                .map(|c| &c.children)
                .unwrap_or(otherwise);
            for child in arm {
                remote_panes(view, child, out, skipped);
            }
        }
        _ => {
            for child in &node.children {
                remote_panes(view, child, out, skipped);
            }
        }
    }
}

/// Spawns `forge-preview-{base}` for a declared pane — stdin feeds commands,
/// stdout drains on a thread (pane events stay inside their own document).
/// Pane host stdout events — `(pane_id, event)` — drained each pane-sync
/// tick so streaming bases' frames land in `preview.panes.<id>.*` state.
type PaneEvent = (String, HostEvent);
static PANE_EVENTS: std::sync::LazyLock<(
    std::sync::mpsc::Sender<PaneEvent>,
    std::sync::Mutex<std::sync::mpsc::Receiver<PaneEvent>>,
)> = std::sync::LazyLock::new(|| {
    let (tx, rx) = std::sync::mpsc::channel();
    (tx, std::sync::Mutex::new(rx))
});

/// The `Command` for a preview host — older-gpui bases can't talk to newer
/// Wayland compositors (`UnsupportedVersion`), so non-default hosts get X11
/// via XWayland. Streaming bases (iced/ratatui) have no window — the env is
/// inert there.
fn preview_command(binary: &PathBuf, base: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new(binary);
    if base != BASE_ID {
        cmd.env("WAYLAND_DISPLAY", "")
            .env("XDG_SESSION_TYPE", "x11");
    }
    cmd
}

fn spawn_pane(base: &str, id: &str) -> Option<Host> {
    let binary = find_preview_binary(base)?;
    let mut child = preview_command(&binary, base)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        // Drained, not inherited — a pane's panics/debug dumps would spam
        // Studio's own stderr (and crash-looping hosts spam it per respawn).
        .stderr(std::process::Stdio::piped())
        .spawn()
        .ok()?;
    if let Some(stderr) = child.stderr.take() {
        std::thread::spawn(move || {
            use std::io::BufRead;
            for _line in std::io::BufReader::new(stderr).lines() {}
        });
    }
    if let Some(stdout) = child.stdout.take() {
        let tx = PANE_EVENTS.0.clone();
        let id = id.to_owned();
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
                    Some("diagnostics") => {
                        HostEvent::Diagnostics(v["items"].as_array().cloned().unwrap_or_default())
                    }
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
                    Some("frame_rgba") => HostEvent::FrameRgba {
                        w: v["w"].as_u64().unwrap_or(0) as u32,
                        h: v["h"].as_u64().unwrap_or(0) as u32,
                        data: v["data"].as_str().unwrap_or_default().to_owned(),
                    },
                    _ => continue,
                };
                if tx.send((id.clone(), event)).is_err() {
                    return;
                }
            }
            let _ = tx.send((id, HostEvent::Closed { generation: 0 }));
        });
    }
    let stdin = child.stdin.take()?;
    Some(Host {
        child,
        stdin,
        generation: HOST_GEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1,
    })
}

/// The `source` command for a pane — a named ref becomes a document clone
/// with the ref body as root; no `doc:` previews the parent buffer.
fn pane_doc_source(view: &ForgeView, pane: &PaneSpec, dir: &str) -> Json {
    match pane.doc.as_deref() {
        Some(name) => {
            let Some(mut doc) = view.document().cloned() else {
                return json!({"cmd":"source","src":"","dir":dir});
            };
            doc.root = view
                .document()
                .and_then(|d| {
                    d.refs
                        .iter()
                        .find(|(n, _)| n == name)
                        .map(|(_, b)| b.clone())
                })
                .unwrap_or_default();
            if !pane.base.is_empty() {
                doc.base = Some(pane.base.clone());
            }
            json!({
                "cmd": "source",
                "src": yaml::to_string(&doc, &yaml::WriteOptions::default()),
                "dir": dir,
            })
        }
        None => json!({"cmd": "source", "src": source(view), "dir": dir}),
    }
}

/// Starts the pane-sync task once — diffs declared `remote:` panes against
/// running hosts every 200ms, spawns/kills deltas, streams each pane's doc
/// on change, and floats each host window over its `preview_bounds(id)` slot.
fn pane_sync(shared: SharedState, cx: &mut Context<ForgeView>) {
    if PANE_SYNC.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    cx.spawn(async move |view, cx| {
        let mut fed: std::collections::HashMap<String, String> = Default::default();
        let mut rects: std::collections::HashMap<String, String> = Default::default();
        let mut themes: std::collections::HashMap<String, String> = Default::default();
        let mut deaths: std::collections::HashMap<String, u32> = Default::default();
        let mut gave_up: std::collections::HashSet<String> = Default::default();
        // Pane id → (spawn time, child pid) for windowed hosts not yet seen to
        // map a window. Probed once after a grace period — a live process with
        // no window (e.g. old-gpui on WSLg) is as useless as a dead one.
        let mut pending: std::collections::HashMap<String, (std::time::Instant, u32)> =
            Default::default();
        let mut last_skipped = 0usize;
        loop {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(200))
                .await;
            let Ok(work) = view.update(cx, |view, cx| {
                let mut panes = Vec::new();
                let mut skipped = 0usize;
                if let Some(doc) = view.document() {
                    remote_panes(view, &doc.root, &mut panes, &mut skipped);
                }
                // Pane host events — a `mode: stream` pane's frames land in
                // `preview.panes.<id>.*` so its card renders the grid. Other
                // panes' stdout (iced's debug grid) stays drained.
                let streams: std::collections::HashSet<&str> = panes
                    .iter()
                    .filter(|p| p.mode == "stream")
                    .map(|p| p.id.as_str())
                    .collect();
                while let Ok((pid, ev)) = PANE_EVENTS.1.lock().unwrap().try_recv() {
                    match ev {
                        HostEvent::Frame { rows } if streams.contains(pid.as_str()) => {
                            view.update_state(
                                &pane_state_path(&pid, "frame"),
                                Json::String(rows.join("\n")),
                                cx,
                            );
                        }
                        HostEvent::FrameRgba { data, .. } if streams.contains(pid.as_str()) => {
                            view.update_state(
                                &pane_state_path(&pid, "frame_rgba"),
                                Json::String(data),
                                cx,
                            );
                        }
                        HostEvent::Ready { base } => {
                            status(view, &format!("preview pane `{pid}` ready ({base})"), cx)
                        }
                        HostEvent::Diagnostics(items) if !items.is_empty() => {
                            status(
                                view,
                                &format!("preview pane `{pid}`: {} diagnostic(s)", items.len()),
                                cx,
                            )
                        }
                        _ => {}
                    }
                }
                let wb = cx
                    .windows()
                    .first()
                    .and_then(|h| h.update(cx, |_, w, _| w.window_bounds()).ok())
                    .and_then(|b| match b {
                        WindowBounds::Windowed(wb) => Some(wb),
                        _ => None,
                    });
                let slots: std::collections::HashMap<String, Option<Bounds<Pixels>>> = panes
                    .iter()
                    .filter_map(|p| {
                        view.preview_bounds(&p.id)
                            .map(|cell| (p.id.clone(), *cell.borrow()))
                    })
                    .collect();
                let doc_base = get_path(view.state(), &parse_path("studio.doc_base").unwrap())
                    .and_then(Json::as_str)
                    .unwrap_or(BASE_ID)
                    .to_owned();
                let theme = theme_cmd(view, cx.theme());
                // No open file → pane `doc:` paths resolve against the shell
                // doc's own directory (`studio.forge.yaml` lives in the crate).
                let dir = shared
                    .borrow()
                    .dir
                    .clone()
                    .or_else(|| Some(env!("CARGO_MANIFEST_DIR").into()));
                (panes, slots, wb, doc_base, dir, skipped, theme)
            }) else {
                return;
            };
            let (panes, slots, wb, doc_base, dir, skipped, theme) = work;
            let theme_key = theme.to_string();
            if skipped != last_skipped {
                last_skipped = skipped;
                if skipped > 0 {
                    let _ = view.update(cx, |view, cx| {
                        status(
                            view,
                            &format!(
                                "{skipped} remote forge.Preview pane(s) have no `id:` — no host spawned"
                            ),
                            cx,
                        )
                    });
                }
            }

            // Kill panes no longer declared.
            let stale: Vec<String> = {
                let guard = HOSTS.lock().unwrap();
                guard
                    .keys()
                    .filter(|id| !panes.iter().any(|p| &p.id == *id))
                    .cloned()
                    .collect()
            };
            for id in stale {
                if let Some((_, mut old)) = HOSTS.lock().unwrap().remove(&id) {
                    let _ = old.child.kill();
                    let _ = old.child.wait();
                }
                fed.remove(&id);
                rects.remove(&id);
                themes.remove(&id);
                deaths.remove(&id);
                gave_up.remove(&id);
                pending.remove(&id);
            }

            // A dead pane host respawns on the next tick — report, then drop it
            // so the spec check below spawns a replacement. Two deaths marks
            // the host unusable — a host that can't window here (e.g. old-gpui
            // on WSL2's GPU) must not flash-crash on every tick.
            let dead: Vec<String> = {
                let mut guard = HOSTS.lock().unwrap();
                guard
                    .iter_mut()
                    .filter_map(|(id, (_, h))| h.child.try_wait().ok().flatten().map(|_| id.clone()))
                    .collect()
            };
            for id in dead {
                HOSTS.lock().unwrap().remove(&id);
                fed.remove(&id);
                let tries = deaths.entry(id.clone()).or_insert(0);
                *tries += 1;
                if *tries > 1 {
                    if gave_up.insert(id.clone()) {
                        let _ = view.update(cx, |view, cx| {
                            status(
                                view,
                                &format!("preview host for `{id}` keeps exiting — giving up"),
                                cx,
                            )
                        });
                    }
                } else {
                    let _ = view.update(cx, |view, cx| {
                        status(view, &format!("preview host for `{id}` exited — restarting"), cx)
                    });
                }
            }

            // A host that stays alive but never maps a window (old-gpui keeps
            // running even when its window creation fails) still claims the
            // card's "separate window" is open. Probe X11 once, three seconds
            // after spawn; a live child with no `_NET_WM_PID` window is killed
            // and reported instead of sitting as a zombie.
            let unmapped: Vec<String> = pending
                .iter()
                .filter(|(_, (t0, _))| t0.elapsed() > std::time::Duration::from_secs(3))
                .filter(|(id, (_, pid))| {
                    HOSTS
                        .lock()
                        .unwrap()
                        .get(*id)
                        .is_some_and(|(_, h)| h.child.id() == *pid)
                })
                .filter(|(_, (_, pid))| x11_pid_has_window(*pid) == Some(false))
                .map(|(id, _)| id.clone())
                .collect();
            for id in unmapped {
                if let Some((_, mut h)) = HOSTS.lock().unwrap().remove(&id) {
                    let _ = h.child.kill();
                    let _ = h.child.wait();
                }
                fed.remove(&id);
                if gave_up.insert(id.clone()) {
                    let _ = view.update(cx, |view, cx| {
                        status(
                            view,
                            &format!(
                                "preview host for `{id}` opened no window — can't render here"
                            ),
                            cx,
                        )
                    });
                }
            }
            // Expired entries were probed (or couldn't be — Wayland) once;
            // drop them either way.
            pending.retain(|_, (t0, _)| t0.elapsed() <= std::time::Duration::from_secs(3));

            for pane in &panes {
                if gave_up.contains(&pane.id) {
                    continue;
                }
                let base = if pane.base.is_empty() {
                    &doc_base
                } else {
                    &pane.base
                };
                let spec = format!("{base}\u{1}{}", pane.doc.as_deref().unwrap_or(""));
                let running = HOSTS.lock().unwrap().get(&pane.id).map(|(s, _)| s.clone());
                if running.as_deref() != Some(spec.as_str()) {
                    if let Some((_, mut old)) = HOSTS.lock().unwrap().remove(&pane.id) {
                        let _ = old.child.kill();
                        let _ = old.child.wait();
                    }
                    themes.remove(&pane.id);
                    deaths.remove(&pane.id);
                    gave_up.remove(&pane.id);
                    match spawn_pane(base, &pane.id) {
                        Some(host) => {
                            if pane.mode != "stream" {
                                pending.insert(
                                    pane.id.clone(),
                                    (std::time::Instant::now(), host.child.id()),
                                );
                            }
                            HOSTS.lock().unwrap().insert(pane.id.clone(), (spec, host));
                            fed.remove(&pane.id);
                        }
                        None => continue,
                    }
                }

                // Feed the pane its document — `load` for real paths (sent
                // once per spawn), `source` for refs/parent docs (on change).
                match pane.doc.as_deref() {
                    Some(rel) if !rel.is_empty() => {
                        let path = dir
                            .as_ref()
                            .map(|d| d.join(rel))
                            .unwrap_or_else(|| PathBuf::from(rel));
                        // `load` sends once per spawn AND on file change — the
                        // mtime folds into the fed key so edits outside Studio
                        // (and to non-buffer files) still live-reload.
                        let mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
                        let cmd = json!({"cmd":"load","doc":path.display().to_string()});
                        let key = format!("{cmd}|{mtime:?}");
                        if fed.get(&pane.id) != Some(&key) {
                            pane_send(&pane.id, &cmd);
                            fed.insert(pane.id.clone(), key);
                        }
                    }
                    _ => {
                        let d = dir
                            .as_ref()
                            .map(|d| d.display().to_string())
                            .unwrap_or_default();
                        let cmd = view
                            .update(cx, |view, _| pane_doc_source(view, pane, &d))
                            .unwrap_or_else(|_| json!({"cmd":"source","src":"","dir":""}));
                        let key = cmd.to_string();
                        if fed.get(&pane.id) != Some(&key) {
                            pane_send(&pane.id, &cmd);
                            fed.insert(pane.id.clone(), key);
                        }
                    }
                }

                // Theme applies to pane hosts too — a theme preview shows the
                // same theme everywhere it can.
                if themes.get(&pane.id) != Some(&theme_key) {
                    pane_send(&pane.id, &theme);
                    themes.insert(pane.id.clone(), theme_key.clone());
                }

                // Float the host window over the pane's tracked slot — or, for
                // a streaming host, size its headless grid to the slot.
                let slot = pane
                    .wants_frame_rect()
                    .then(|| slots.get(&pane.id).copied().flatten())
                    .flatten();
                if let (Some(wb), Some(b)) = (wb, slot) {
                    let rect = json!({
                        "cmd": "frame_rect",
                        "x": f64::from(wb.origin.x) + f64::from(b.origin.x),
                        "y": f64::from(wb.origin.y) + f64::from(b.origin.y),
                        "w": f64::from(b.size.width),
                        "h": f64::from(b.size.height),
                    });
                    let key = rect.to_string();
                    if rects.get(&pane.id) != Some(&key) {
                        pane_send(&pane.id, &rect);
                        rects.insert(pane.id.clone(), key);
                    }
                }
            }
        }
    })
    .detach();
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

/// Reads `path` into the editor, updates recents + the menu, launches the
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
    // tiny TOML-ish parse: `key = value` lines. Strings quoted, bool bare.
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
        &parse_path("studio.markers").unwrap(),
        markers(&diagnostics),
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
        &parse_path("studio.markers").unwrap(),
        markers(&diagnostics),
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
/// palette item is armed (`studio.placing`). It parses the element's snippet into a
/// node and inserts it under the clicked node.
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
        // Container targets take the node inside. Leaf targets take it as
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
        // Container targets take the node inside. Leaf targets take it as
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
        // Container targets take the node inside. Leaf targets take it as
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
/// children item. Falls back to a bare `Element: Name` node.
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
    // Select the parent so the inspector lands somewhere sensible — for
    // `e`/`c` branch paths the parent is the control node itself.
    let parent = path_steps(&path)
        .and_then(|steps| {
            let parent = &steps[..steps.len().saturating_sub(1)];
            (!parent.is_empty()).then(|| path_string(parent))
        })
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

/// Snapshots `(document, source)` before a mutation. Clears the redo stack.
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

/// Diagnostics as a `studio.markers` JSON list — `{line, col, message, hint?}`
/// maps the source `Editor`'s `diagnostics:` prop renders as squiggles (#128).
fn markers(diagnostics: &[Diagnostic]) -> Json {
    check::diagnostics_json(diagnostics)
}

/// `prop_changed(prop)` — an inspector field wrote `document.edits.<prop>`. Push the
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

/// `refname_changed()` — the inspector's ref field wrote `document.edits.refname`.
/// Assign it on the selected node — `sync_refs` turns it into a `name: &name`
/// block and the preserving writer emits `<<: *name` (or a new ref block).
fn refname_changed(view: &mut ForgeView, shared: &SharedState, cx: &mut Context<ForgeView>) {
    let state = view.state();
    let Some(path) = get_path(state, &parse_path("document.selected").unwrap())
        .and_then(|j| j.as_str())
        .map(str::to_owned)
    else {
        return;
    };
    let value = get_path(state, &parse_path("document.edits.refname").unwrap())
        .and_then(|j| j.as_str())
        .unwrap_or_default()
        .trim()
        .to_owned();
    record_undo(view, shared, cx);
    let preview = view.preview("canvas", cx);
    preview.update(cx, |preview, cx| {
        preview.set_node_refname(&path, (!value.is_empty()).then_some(value), cx);
    });
    if let Some(doc) = preview.read(cx).document().cloned() {
        let src = yaml::to_string_preserving(&doc, &yaml::WriteOptions::default());
        view.update_state(&parse_path("editor.source").unwrap(), Json::String(src), cx);
        view.reset_inputs("source");
    }
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
/// the path in `document.selected`. This just surfaces it in the status bar.
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
