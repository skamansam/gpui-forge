//! Forge Studio: a Forge document rendered by the Forge runtime.
//!
//! The shell is `studio.forge.yaml` — toolbar, palette, source editor, preview and
//! inspector are all declared in YAML and interpreted by `ForgeView`. Events in that
//! document call the hooks registered below.

mod manifest;

use manifest::{manifest_section, manifest_value};

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use gpui_forge_runtime::base::BASE_ID;
use gpui_forge_runtime::check;
use gpui_forge_runtime::forge_spec::Document;
use gpui_forge_runtime::forge_spec::catalog::Catalog;
use gpui_forge_runtime::forge_spec::value::parse_path;
use gpui_forge_runtime::forge_spec::{
    Control, Diagnostic, ElementRef, Entries, Node, Unit, Value, path_steps, path_string, yaml,
};
use gpui_forge_runtime::render::{ForgeView, ResizableDialog, get_path, pane_state_path, set_path};
use gpui_kit::component::Placement;
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

/// Reads `rel` (crate-dir-relative — `CARGO_MANIFEST_DIR` bakes the source
/// dir at compile time) from disk, falling back to the `include_str!` copy.
/// Packaged builds won't find the path and get the embedded bytes.
fn file_src(rel: &str, embedded: &'static str) -> String {
    std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel))
        .unwrap_or_else(|_| embedded.to_owned())
}

/// A document's `meta.watch` opts its file into hot-reload — Studio then
/// polls it and swaps the parsed doc on change (see `watch_file` /
/// `watch_shell`).
fn doc_watches(doc: &Document) -> bool {
    doc.meta
        .iter()
        .any(|(k, v)| k == "watch" && manifest::truthy(v))
}

/// Same check on the loaded view — `view.meta()` mirrors the document's map.
fn view_watches(view: &ForgeView) -> bool {
    view.meta()
        .get("watch")
        .and_then(Json::as_str)
        .is_some_and(manifest::truthy)
}

/// `watch` declared in `meta:` or in the project's `[forge]` — either marks
/// the file for hot-reload.
fn file_watches(view: &ForgeView, shared: &SharedState) -> bool {
    view_watches(view) || shared.borrow().project_watch
}
const BLANK: &str = "$schema: https://gpui-forge.rudeboy.dev/schema/0.1/gpui-kit.json\nforge: \"0.1\"\nbase: gpui-kit\n\nroot:\n  VStack:\n  style:\n    padding: 16px\n  children:\n    - Text: Hello, Forge!\n";

// One demo per widget set, shared with `examples/` and the docs site (#129).
const EXAMPLE_ELY: &str = include_str!("../../../examples/demos/ely-demo.forge.yaml");
const EXAMPLE_GFLOW: &str = include_str!("../../../examples/demos/gflow-demo.forge.yaml");
const EXAMPLE_BASE: &str = include_str!("../../../examples/demos/base-demo.forge.yaml");
const EXAMPLE_VIDEO: &str = include_str!("../../../examples/demos/video-demo.forge.yaml");
const EXAMPLE_PDF: &str = include_str!("../../../examples/demos/pdf-demo.forge.yaml");
const EXAMPLE_WB: &str = include_str!("../../../examples/demos/whiteboard-demo.forge.yaml");
const EXAMPLE_MATH: &str = include_str!("../../../examples/demos/math-demo.forge.yaml");
const EXAMPLE_HIG: &str = include_str!("../../../examples/demos/hig-demo.forge.yaml");
// Foreign-base showcases — they render in the base's own preview host.
const EXAMPLE_GUISE: &str = include_str!("../../../examples/showcases/guise-showcase.forge.yaml");
const EXAMPLE_BEZEL: &str = include_str!("../../../examples/showcases/bezel-showcase.forge.yaml");
const EXAMPLE_RATATUI: &str =
    include_str!("../../../examples/showcases/ratatui-showcase.forge.yaml");
const EXAMPLE_ICED: &str = include_str!("../../../examples/showcases/iced-showcase.forge.yaml");
const EXAMPLE_ADABRAKA: &str =
    include_str!("../../../examples/showcases/adabraka-showcase.forge.yaml");

/// Bases the Theme tab's preview selector offers — `studio.preview_bases`
/// carries them to the `Select`, and `cycle_preview_base` walks the list.
const PREVIEW_BASES: &[&str] = &[
    "gpui-kit",
    "ely",
    "hig",
    "gflow",
    "math",
    "pdf",
    "gpui-base",
    "guise",
    "bezel",
    "adabraka",
    "iced",
    "ratatui",
];

/// Widget sets the Start tab offers in its showcase picker (#129): the
/// gpui-kit base, the `forge` intrinsics, every registered `forge-lib-*`.
/// Then the foreign-base showcases (they start their preview host).
const WIDGET_SETS: &[&str] = &[
    "gpui-kit", "forge", "ely", "gflow", "base", "video", "hig", "pdf", "wb", "math", "guise",
    "bezel", "ratatui", "iced", "adabraka",
];

struct Shared {
    /// The file `editor.source` was loaded from. `None` for embedded documents.
    file: Option<PathBuf>,
    /// The directory the current document lives in — asset paths resolve
    /// here and it's forwarded to the preview host as `base_dir`.
    dir: Option<PathBuf>,
    catalogs: Vec<Catalog>,
    /// The open project's `[icons] set` — forwarded to the canvas preview as
    /// its `project_iconset` for docs under `project_dir`. A document's own
    /// `meta.iconset` overrides it.
    project_iconset: Option<String>,
    /// The open project's root — `project_iconset` only applies to docs
    /// under it. Files opened elsewhere fall back to lucide.
    project_dir: Option<PathBuf>,
    /// The project's `[forge] watch` — when true, opened docs hot-reload
    /// even without their own `meta.watch`, and `watch_manifest` tracks the
    /// `forge.toml` itself.
    project_watch: bool,
    /// The `forge.toml` currently being watched — `watch_manifest` exits
    /// when this no longer matches its path (project switched).
    manifest_watching: Option<PathBuf>,
    /// The open project's `forge.toml` — the Project tab edits this file.
    /// `save_project` writes `studio.toml_source` back here.
    project_toml: Option<PathBuf>,
    /// Undo/redo — snapshots of `(document, editor source)` taken before each
    /// mutation, so a step back restores the editor text exactly.
    undo: Vec<(Document, String)>,
    redo: Vec<(Document, String)>,
}

/// #150 — a persisted window placement. `maximized` restores as
/// `WindowBounds::Maximized`. Fullscreen saves as windowed (restoring
/// fullscreen at startup is hostile).
#[derive(Clone, Copy, PartialEq)]
struct SavedWindow {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    maximized: bool,
}

impl SavedWindow {
    fn new(b: Bounds<Pixels>, maximized: bool) -> Self {
        Self {
            x: f64::from(b.origin.x),
            y: f64::from(b.origin.y),
            w: f64::from(b.size.width),
            h: f64::from(b.size.height),
            maximized,
        }
    }
}

/// Last geometry the poll task observed — `save_prefs` writes it out. A
/// static (like HOST/HOSTS) because `#[forge_hook]` call sites can't reach
/// `Shared`.
static LAST_WINDOW: std::sync::Mutex<Option<SavedWindow>> = std::sync::Mutex::new(None);

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
    /// #120 Phase 3 — a TUI frame as the host's real ANSI byte stream.
    /// the card feeds it through `forge_term`'s emulator into styled cells.
    Frame {
        seq: u64,
        cols: u16,
        rows: u16,
        data: String,
    },
    /// #120 Phase 2 — an rgba screenshot from a pixel-capable host (iced).
    FrameRgba {
        w: u32,
        h: u32,
        data: String,
    },
    /// #156 — stream-pane DnD: the deepest doc path under a `down` cell.
    /// The sender stashes it as the drag source for a node move.
    Zone(String),
    /// #156 — the doc path a `drop-hover` currently highlights, or "".
    DropTarget(String),
    /// #156 — the doc path a `drop` resolved to, or "" on a miss. Carries
    /// the commit — a pending pane payload pairs with it.
    Drop(String),
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

mod studio_assets;
use studio_assets::{StudioAssets, add_icon_root};

fn main() {
    // The shell document alone needs ~10MB of layout stack — re-exec under
    // a raised rlimit before gpui starts.
    gpui_forge_runtime::ensure_main_stack();
    add_icon_root(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"));
    gpui_kit::application().with_assets(StudioAssets).run(|cx| {
        gpui_kit::init(cx);
        cx.bind_keys([
            gpui::KeyBinding::new("escape", gpui_forge_runtime::render::CancelPick, None),
            gpui::KeyBinding::new("ctrl-s", gpui_forge_runtime::render::SaveDocument, None),
            // Never fires on Linux/Windows. Keeps the binding for macOS.
            gpui::KeyBinding::new("cmd-s", gpui_forge_runtime::render::SaveDocument, None),
        ]);
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
        // The windowed shell IS the app — quit when its last window closes so
        // no headless process lingers.
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        // #150 — restore persisted geometry. The saved rect counts as
        // visible when its title-bar strip still overlaps a live
        // monitor (a monitor may have been unplugged). Size floors sit
        // at #149's eventual minimum. Anything else → centered.
        let prefs = load_prefs();
        let key = |k: &str| prefs.get(k).and_then(Json::as_f64);
        let centered = |cx: &App| Bounds::centered(None, size(px(1400.0), px(900.0)), cx);
        let bounds = match (
            key("window_x"),
            key("window_y"),
            key("window_w"),
            key("window_h"),
        ) {
            (Some(x), Some(y), Some(w), Some(h)) => {
                let w = w.max(640.0);
                let h = h.max(400.0);
                let visible = cx.displays().iter().any(|d| {
                    let db = d.bounds();
                    let (dx0, dy0) = (f32::from(db.origin.x), f32::from(db.origin.y));
                    let (dx1, dy1) = (
                        dx0 + f32::from(db.size.width),
                        dy0 + f32::from(db.size.height),
                    );
                    let (x, y, w) = (x as f32, y as f32, w as f32);
                    let iw = (x + w).min(dx1) - x.max(dx0);
                    let ih = (y + 30.0).min(dy1) - y.max(dy0);
                    iw >= 100.0 && ih >= 10.0
                });
                if visible {
                    Bounds::new(
                        point(px(x as f32), px(y as f32)),
                        size(px(w as f32), px(h as f32)),
                    )
                } else {
                    centered(cx)
                }
            }
            _ => centered(cx),
        };
        let maximized = prefs
            .get("window_maximized")
            .and_then(Json::as_bool)
            .unwrap_or(false);
        let native_deco = native_decorations(&prefs);
        let options = WindowOptions {
            app_id: Some("gpui-forge".into()),
            window_bounds: Some(if maximized {
                WindowBounds::Maximized(bounds)
            } else {
                WindowBounds::Windowed(bounds)
            }),
            titlebar: Some(TitlebarOptions {
                title: Some("Forge Studio".into()),
                appears_transparent: !native_deco,
                traffic_light_position: None,
            }),
            app_owns_titlebar_drag: !native_deco,
            window_decorations: Some(if native_deco {
                WindowDecorations::Server
            } else {
                WindowDecorations::Client
            }),
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
        project_iconset: None,
        project_dir: None,
        project_watch: false,
        manifest_watching: None,
        project_toml: None,
        undo: Vec::new(),
        redo: Vec::new(),
    }));

    let mut view = ForgeView::new(shared.borrow().catalogs.clone());
    let shell = file_src("studio.forge.yaml", SHELL);
    let (doc, diagnostics) = check::check(&shell, BASE_ID, &shared.borrow().catalogs);
    for d in &diagnostics {
        eprintln!(
            "studio.forge.yaml:{}:{}: {}",
            d.pos.line, d.pos.col, d.message
        );
    }
    // `meta.watch` on the shell marks it for hot-reload. A broken parse
    // spawns the watcher anyway — it's the recovery path for the fix.
    let shell_watched = doc.as_ref().is_none_or(doc_watches);
    if let Some(doc) = doc {
        view.set_document(doc, cx);
    }
    let shell_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("studio.forge.yaml");
    if shell_watched && shell_path.is_file() {
        watch_shell(shell_path, shared.clone(), cx);
    }
    refresh_recent_menu(&mut view, &load_recents(), cx);

    view.register_library(forge_lib_ely::Ely::new());
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
    // Global pref — a custom iconify cache root, applied before any load.
    if let Some(dir) = prefs.get("icon_cache").and_then(|j| j.as_str())
        && !dir.is_empty()
    {
        gpui_forge_runtime::iconify::set_cache_dir(Some(dir.into()));
    }
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
                        // #144 — the base selector's options live in state so the
        // cycle button walks the same list. `theme_preview` persists
                        // across sessions via the prefs file.
                        "preview_bases": PREVIEW_BASES,
                        "theme_preview": prefs
                            .get("theme_preview")
                            .and_then(|j| j.as_str())
                            .unwrap_or(BASE_ID),
                        // `forge.Files` options — Preferences → Files writes these.
                        // persisted across sessions via the prefs file.
                        "files_search": prefs
                            .get("files_search")
                            .and_then(|j| j.as_bool())
                            .unwrap_or(true),
                        "files_icons": prefs
                            .get("files_icons")
                            .and_then(|j| j.as_bool())
                            .unwrap_or(true),
                        "files_hidden": prefs
                            .get("files_hidden")
                            .and_then(|j| j.as_bool())
                            .unwrap_or(false),
                        "files_folders": prefs
                            .get("files_folders")
                            .and_then(|j| j.as_str())
                            .unwrap_or("tree"),
                        "files_highlights": prefs
                            .get("files_highlights")
                            .and_then(|j| j.as_str())
                            .unwrap_or(HL_DEFAULT),
                        // Preferences → Highlights — `hl_seed` builds
                        // `hl_list` rows from the shorthand. The picker's
                        // `colors:` reads this palette.
                        "hl_palette": HL_PALETTE,
                        "hl_list": [],
                        // Iconify cache root — Preferences → Icons. Empty means the
                        // platform default (`~/.cache/gpui-forge/iconify`).
                        "icon_cache": prefs
                            .get("icon_cache")
                            .and_then(|j| j.as_str())
                            .unwrap_or(""),
                        // Window chrome — Preferences → Window. An `if:` on this
                        // path hides in-app title rows when the WM draws them.
                        "native_decorations": native_decorations(&prefs),
                        "file": "welcome.forge.yaml",
                        // Assets tab — `refresh_assets` rewrites dir + listing when a
                        // document/project loads.
                        "assets_dir": "",
                        "asset_files": [],
                        "assets_zoom": 0,
                        // Files sidebar "project" toggle — the document's folder.
                        "doc_dir": "",
                        "diagnostics": "",
                        "widget_sets": WIDGET_SETS,
                        "spec": gpui_forge_runtime::forge_spec::SPEC_VERSION,
                        "base": BASE_ID,
                    },
                    "editor": { "source": file_src("welcome.forge.yaml", WELCOME) },
                    "preview": { "state": "" },
                    "status": { "message": "ready — every panel you see is a Forge document" },
                }),
        cx,
    );
    // #148 — the project sidebar roots at the startup cwd. Opening a document
    // re-roots it at the file's directory via `load_source`.
    if let Ok(cwd) = std::env::current_dir() {
        refresh_files(&mut view, &cwd, cx);
        refresh_assets(&mut view, &cwd, cx);
    }

    for name in [
        "new_document",
        "open_document",
        "open_project",
        "open_path",
        "open_recent",
        "files_scope",
        "grep_project",
        "icon_copy",
        "toggle_icon_theme",
        "retry_iconset",
        "create_project",
        "preview_in_host",
        "raise_host",
        "pane_key",
        "pane_mouse",
        "pane_drop_at",
        "save_document",
        "save_project",
        "format_document",
        "apply_theme",
        "apply_theme_name",
        "apply_theme_json",
        "reset_theme",
        "cycle_preview_base",
        "persist_preview_base",
        "persist_prefs",
        "files_pref_changed",
        "hl_validate",
        "hl_add",
        "hl_remove",
        "hl_reset",
        "hl_changed",
        "hl_commit",
        "close_prefs",
        "apply_icon_cache",
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
        "close_dialog",
        "files_new",
        "files_reveal",
        "files_rename",
        "files_add",
        "rename_file",
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

    let src = file_src("welcome.forge.yaml", WELCOME);
    load_source(
        &mut view,
        "welcome.forge.yaml",
        &src,
        Some(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))),
        &shared,
        cx,
    );
    // `forge-studio <doc.forge.yaml>` opens straight into the document.
    // `forge-studio <dir>` or `forge-studio forge.toml` opens the project —
    // same explicit-only resolution the CLI uses.
    if let Some(path) = std::env::args().nth(1).map(PathBuf::from) {
        let toml = if path.is_dir() {
            Some(path.join("forge.toml"))
        } else {
            (path.file_name() == Some(std::ffi::OsStr::new("forge.toml"))).then_some(path.clone())
        };
        match toml.filter(|t| t.is_file()) {
            Some(toml) => load_project(toml, &mut view, shared.clone(), cx),
            None if path.is_file() => load_file(path, &mut view, shared.clone(), cx),
            None => {}
        }
    }
    // FORGE_DEBUG_TAB=<n> forces the initial tab — headless harnesses have
    // no clicks to reach the Theme tab, whose `if: $studio.tab` guard gates
    // pane spawn.
    if let Some(n) = std::env::var("FORGE_DEBUG_TAB")
        .ok()
        .and_then(|t| t.parse::<i32>().ok())
    {
        view.update_state(&parse_path("studio.tab").unwrap(), Json::from(n), cx);
    }
    // Declared panes in the shell document (the Theme-tab theme preview)
    // spawn hosts from startup — no need to wait for a foreign-base open.
    pane_sync(shared.clone(), cx);
    // #150 — poll the window bounds and persist on change. Covers
    // move/resize and crash-quits without platform resize-event plumbing.
    // writes only on an actual geometry change.
    cx.spawn(async move |view, cx| {
        let mut last: Option<SavedWindow> = None;
        loop {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(500))
                .await;
            let cur = cx.update(|cx| {
                cx.windows().first().and_then(|h| {
                    h.update(cx, |_, w, _| match w.window_bounds() {
                        WindowBounds::Windowed(b) => Some(SavedWindow::new(b, false)),
                        WindowBounds::Maximized(b) => Some(SavedWindow::new(b, true)),
                        WindowBounds::Fullscreen(b) => Some(SavedWindow::new(b, false)),
                    })
                    .ok()
                    .flatten()
                })
            });
            let Some(g) = cur else { continue };
            if last == Some(g) {
                continue;
            }
            last = Some(g);
            *LAST_WINDOW.lock().unwrap() = Some(g);
            view.update(cx, |view, _| save_prefs(view)).ok();
        }
    })
    .detach();
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
    // `resizable: true` wraps the dialog in a `ResizableDialog`. The declared
    // width/height become its *initial* size and the root fills it (`full`) so
    // the corner grip can move the card's edges.
    let resizable = node
        .props
        .iter()
        .any(|(k, v)| k == "resizable" && matches!(v, Value::Bool(true)));
    // `sheet: true` opens the declaration as an edge sheet instead of a
    // centered dialog. `placement:` picks the edge (default `right`).
    let sheet = node
        .props
        .iter()
        .any(|(k, v)| k == "sheet" && matches!(v, Value::Bool(true)));
    let placement = node
        .props
        .iter()
        .find_map(|(k, v)| match (k.as_str(), v) {
            ("placement", Value::String(s) | Value::Enum(s)) => Some(match s.as_str() {
                "top" => Placement::Top,
                "bottom" => Placement::Bottom,
                "left" => Placement::Left,
                _ => Placement::Right,
            }),
            _ => None,
        })
        .unwrap_or(Placement::Right);
    // The dialog chrome fixes its own width (448px default) and ignores the
    // child's. A declared `width:` or a resizable wrapper must reach the panel
    // through `dialog.w` / `sheet.size`, or the card stays 448 wide.
    let declared_w = style_px(&node.style, "width");
    let declared_h = style_px(&node.style, "height");
    let mut style = node.style.clone();
    let initial = resizable.then(|| {
        let size = size(
            style_px(&style, "width").unwrap_or(px(560.)),
            style_px(&style, "height").unwrap_or(px(400.)),
        );
        for (key, value) in &mut style {
            if key == "width" || key == "height" {
                *value = Value::Enum("full".into());
            }
        }
        size
    });
    // The prefs dialog edits `studio.hl_list` rows — rebuild them from the
    // persisted shorthand before the dialog's state copy is taken.
    if name == "preferences" {
        hl_seed(view, cx);
    }
    let catalogs = view.catalogs.clone();
    let libraries = view.libraries.clone();
    let shell = cx.entity().downgrade();
    let shared = shared.clone();
    let studio_seed = view.state().get("studio").cloned().unwrap_or(Json::Null);
    let child = cx.new(|cx| {
        let mut v = ForgeView::new(catalogs);
        v.libraries = libraries;
        let weak = shell.clone();
        let shared = shared.clone();
        let prev_studio = Rc::new(RefCell::new(studio_seed.clone()));
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
                let text = state.to_string();
                // `studio.*` binds inside the dialog write the dialog's own
                // state — merge each changed top-level `studio` key back into
                // the shell so toggles, validation and `persist_prefs` see
                // them. `link` mirrors shell writes the other way. Only keys
                // changed since the last sink call go through. Writing every
                // key per write amplifies through `link`'s forward into a
                // defer storm when two keys disagree.
                let studio_now = state.get("studio").cloned().unwrap_or(Json::Null);
                let mut prev = prev_studio.borrow_mut();
                let changes: Vec<(String, Json)> = studio_now
                    .as_object()
                    .map(|obj| {
                        obj.iter()
                            .filter(|(k, v)| prev.get(*k) != Some(*v))
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect()
                    })
                    .unwrap_or_default();
                *prev = studio_now;
                drop(prev);
                cx.defer(move |cx| {
                    shell
                        .update(cx, |view, cx| {
                            for (key, value) in changes {
                                if let Ok(path) = parse_path(&format!("studio.{key}"))
                                    && get_path(view.state(), &path) != Some(&value)
                                {
                                    view.update_state(&path, value, cx);
                                }
                            }
                            view.update_state(
                                &parse_path("dialog.state").unwrap(),
                                Json::String(text.clone()),
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
                    style: style.clone(),
                    children: node.children.clone(),
                    ..Default::default()
                },
            },
            cx,
        );
        // Seed fields dialogs bind against — the base registry for pickers,
        // defaults for the New Project form. A second `open_dialog` arg
        // prefills `name` (the rename dialog passes the current file name).
        v.set_state(
            json!({
                "name": args.get(1).map(String::as_str).unwrap_or("app"),
                "base": BASE_ID,
                "bases": check::bases()
                    .iter()
                    .map(|b| b.id.clone())
                    .collect::<Vec<String>>(),
                // `studio.*` binds resolve against the dialog's own state —
                // seed a copy of the shell's subtree so prefs fields start
                // with their live values. The `on_state` merge writes edits
                // back. `link` mirrors shell writes into the dialog.
                "studio": studio_seed,
            }),
            cx,
        );
        v
    });
    // Shell `studio.*` writes (validation, sync handlers) mirror into the
    // open dialog — the reverse of the `on_state` merge above.
    view.link(child.downgrade());
    // The hook runs inside the main window's own update cycle. `App::windows()`
    // Handles can't be `update`d from in here because the window is taken out of
    // the slot map for the duration. A foreground task runs after the update
    // finishes, when the window is live again.
    //
    // `ResizableDialog` is created here, not in the builder — `open_dialog`'s
    // builder is `Fn` and can re-run. A fresh entity each call would lose the
    // in-progress drag size.
    let resizable = initial.map(|initial| {
        cx.new(|_| ResizableDialog::new(child.clone(), initial, size(px(480.), px(320.))))
    });
    let app = cx.to_async();
    let title = SharedString::from(dialog_title(&node).unwrap_or_else(|| name.clone()));
    cx.foreground_executor()
        .spawn(async move {
            let opened = app.update(|cx| {
                for handle in cx.windows() {
                    let title = title.clone();
                    let child = child.clone();
                    let resizable = resizable.clone();
                    if handle
                        .update(cx, move |_, window, cx| {
                            let content = {
                                let child = child.clone();
                                let resizable = resizable.clone();
                                move || -> AnyElement {
                                    match &resizable {
                                        Some(resizable) => resizable.clone().into_any_element(),
                                        None => child.clone().into_any_element(),
                                    }
                                }
                            };
                            if sheet {
                                window.open_sheet_at(placement, cx, {
                                    let title = title.clone();
                                    let resizable = resizable.clone();
                                    move |sheet, _, cx| {
                                        let mut sheet = sheet.title(title.clone()).child(content());
                                        // `size` is the cross axis — width for
                                        // side sheets, height for top/bottom.
                                        let extent = match &resizable {
                                            Some(r) => {
                                                let s = r.read(cx).size();
                                                Some(match placement {
                                                    Placement::Top | Placement::Bottom => s.height,
                                                    _ => s.width,
                                                })
                                            }
                                            None => match placement {
                                                Placement::Top | Placement::Bottom => declared_h,
                                                _ => declared_w,
                                            },
                                        };
                                        if let Some(extent) = extent {
                                            sheet = sheet.size(extent);
                                        }
                                        sheet
                                    }
                                });
                            } else {
                                window.open_dialog(cx, {
                                    let title = title.clone();
                                    let resizable = resizable.clone();
                                    move |dialog, _, cx| {
                                        let mut dialog =
                                            dialog.title(title.clone()).child(content());
                                        // The builder re-runs each frame, so a
                                        // resizable dialog reads the drag's
                                        // live size. A static one reads its declared
                                        // width. No prop means the 448 default.
                                        if let Some(w) = resizable
                                            .as_ref()
                                            .map(|r| r.read(cx).size().width)
                                            .or(declared_w)
                                        {
                                            dialog = dialog.w(w);
                                        }
                                        dialog
                                    }
                                });
                            }
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

/// The `title:` prop on a `Dialog#id` declaration, if the author set one.
fn dialog_title(node: &Node) -> Option<String> {
    node.props.iter().find_map(|(k, v)| match (k.as_str(), v) {
        ("title", Value::String(s) | Value::Enum(s)) => Some(s.clone()),
        _ => None,
    })
}

/// Reads a `width`/`height`-style entry as pixels — `px` and unitless numbers
/// directly, `rem` at 16px. Anything else (`full`, `%`, missing) is `None` and
/// the caller falls back to a default.
fn style_px(style: &Entries, key: &str) -> Option<Pixels> {
    style
        .iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| match v {
            Value::Number {
                value,
                unit: None | Some(Unit::Px),
            } => Some(px(*value as f32)),
            Value::Number {
                value,
                unit: Some(Unit::Rem),
            } => Some(px(*value as f32 * 16.)),
            _ => None,
        })
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
    // Show the theme's JSON in the Theme editor.
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

/// `reset_theme()` — the Theme tab's reset icon — restores the registry's
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
        "files_scope" => files_scope(view, cx),
        "grep_project" => {
            let query = args
                .first()
                .map(String::as_str)
                .unwrap_or_default()
                .to_owned();
            grep_project(view, &query, cx);
        }
        // Icons-tab: clicking a cell copies its SVG into the open project's
        // `assets/icons[<set>]/` so the built app ships it.
        "icon_copy" => {
            if let Some(name) = args.first() {
                icon_copy(view, name, shared, cx);
            }
        }
        // Icons-tab: the yaml-side theme checkboxes toggle one encoded
        // `s:`/`p:` entry in `studio.icon_theme`. `forge.Icons` re-derives
        // the filter + re-publishes `checked` flags from there.
        "toggle_icon_theme" => {
            if let Some(k) = args.first() {
                let p = parse_path("studio.icon_theme").expect("static path");
                let mut arr = get_path(view.state(), &p)
                    .and_then(|j| j.as_array())
                    .cloned()
                    .unwrap_or_default();
                match arr.iter().position(|v| v.as_str() == Some(k.as_str())) {
                    Some(ix) => {
                        arr.remove(ix);
                    }
                    None => arr.push(Json::String(k.clone())),
                }
                view.update_state(&p, Json::Array(arr), cx);
            }
        }
        // Icons-tab: Retry on a failed set download.
        "retry_iconset" => {
            if let Some(set) = args.first() {
                gpui_forge_runtime::iconify::retry(set, cx);
            }
        }
        "create_project" => create_project(view, shared.clone(), cx),
        "preview_in_host" => preview_in_host(view, shared.clone(), cx),
        "raise_host" => host_send(&json!({"cmd":"raise"})),
        // Stream cards (ratatui `mode: stream`) — the card fires `pane_key` /
        // `pane_mouse` for input over its tracked TermView. Forward to that
        // pane's host, never the primary host.
        "pane_key" => {
            if let [id, key] = args {
                pane_send(id, &json!({"cmd":"key","key":key}));
                // The demo docs have no bound widgets, so a forwarded key
                // produces no visible change — the status line is the
                // proof of the wire path.
                status(view, &format!("{id} <- key {key}"), cx);
            }
        }
        "pane_mouse" => {
            if let [id, kind, col, row] = args {
                pane_send(
                    id,
                    &json!({
                        "cmd": "mouse",
                        "kind": kind,
                        "col": col.parse::<u16>().unwrap_or(0),
                        "row": row.parse::<u16>().unwrap_or(0),
                    }),
                );
                status(view, &format!("{id} <- mouse {kind} @{col},{row}"), cx);
            }
        }
        // #156 — a ForgeDrag payload dropped on a stream card. Stash it,
        // then `drop` — the host resolves the cell to a doc path and its
        // `drop` event commits the pair via drop_component/move_node.
        "pane_drop" => {
            if let [id, kind, payload, col, row] = args {
                PANE_DND
                    .lock()
                    .unwrap()
                    .insert(id.clone(), (kind.clone(), payload.clone()));
                pane_send(
                    id,
                    &json!({
                        "cmd": "mouse",
                        "kind": "drop",
                        "col": col.parse::<u16>().unwrap_or(0),
                        "row": row.parse::<u16>().unwrap_or(0),
                    }),
                );
                status(view, &format!("{id} <- drop {payload}"), cx);
            }
        }
        // #156 — the local ratatui canvas resolved the drop cell itself, so
        // the doc path arrives in the args — no PANE_DND stash, no host
        // round-trip. Same commit as `pane_drop`'s `drop` event.
        "pane_drop_at" => {
            if let [id, kind, payload, path] = args {
                if path.is_empty() {
                    status(
                        view,
                        &format!("preview pane `{id}`: drop missed every node"),
                        cx,
                    );
                } else {
                    match kind.as_str() {
                        "component" => {
                            drop_component(view, &[payload.clone(), path.clone()], shared, cx)
                        }
                        _ => move_node(view, &[payload.clone(), path.clone()], shared, cx),
                    }
                }
            }
        }
        // A node dragged inside the card. The source path is the `zone`
        // the host reported on `down`. The release cell is the target.
        "pane_move" => {
            if let [id, col, row] = args {
                let src = get_path(view.state(), &pane_state_path(id, "zone"))
                    .and_then(|j| j.as_str())
                    .unwrap_or_default()
                    .to_owned();
                // The drop cmd clears the host's hover highlight even when
                // no source zone was stashed — send it unconditionally.
                if !src.is_empty() {
                    PANE_DND
                        .lock()
                        .unwrap()
                        .insert(id.clone(), ("node".to_string(), src.clone()));
                }
                pane_send(
                    id,
                    &json!({
                        "cmd": "mouse",
                        "kind": "drop",
                        "col": col.parse::<u16>().unwrap_or(0),
                        "row": row.parse::<u16>().unwrap_or(0),
                    }),
                );
                status(view, &format!("{id} <- move {src}"), cx);
            }
        }
        "open_recent" => {
            if let Some(path) = args.first().map(PathBuf::from) {
                load_file(path, view, shared.clone(), cx);
            }
        }
        // #148 — project tree clicks open into the Document tab (index 1),
        // unlike recent/menu opens which land on the Canvas (index 2). A
        // `.json` that parses as a `ThemeSet` opens in the Theme editor
        // (index 4) instead — loaded, not applied. Apply stays explicit.
        "open_path" => {
            if let Some(path) = args.first().map(PathBuf::from) {
                // The tree's `forge.toml` row opens the Project tab, not
                // the document editor — re-read so Source shows the file.
                if path.file_name().is_some_and(|n| n == "forge.toml") {
                    let src = std::fs::read_to_string(&path).unwrap_or_default();
                    view.update_state(
                        &parse_path("studio.toml_source").unwrap(),
                        Json::String(src),
                        cx,
                    );
                    view.update_state(&parse_path("studio.tab").unwrap(), Json::from(7), cx);
                    return;
                }
                let src = std::fs::read_to_string(&path).ok();
                let theme = src
                    .as_deref()
                    .filter(|_| path.extension().is_some_and(|e| e == "json"))
                    .and_then(|s| serde_json::from_str::<ThemeSet>(s).ok());
                match theme {
                    Some(_) => {
                        view.update_state(
                            &parse_path("studio.theme_json").unwrap(),
                            Json::String(src.unwrap_or_default()),
                            cx,
                        );
                        view.update_state(&parse_path("studio.tab").unwrap(), Json::from(4), cx);
                    }
                    None => {
                        load_file(path, view, shared.clone(), cx);
                        view.update_state(&parse_path("studio.tab").unwrap(), Json::from(1), cx);
                    }
                }
            }
        }
        // #148 — the project tree's right-click actions.
        "files_new" => files_new(view, args, shared.clone(), cx),
        "files_reveal" => files_reveal(view, args, cx),
        "files_rename" => files_rename(view, args, shared, cx),
        "files_add" => files_add(view, args, shared, cx),
        "rename_file" => rename_file(view, shared.clone(), cx),
        "close_dialog" => close_dialog(cx),
        "save_document" => save_document(view, shared, cx),
        "save_project" => save_project(view, shared, cx),
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
        "cycle_preview_base" => {
            let bases = get_path(view.state(), &parse_path("studio.preview_bases").unwrap())
                .and_then(|j| j.as_array().cloned())
                .unwrap_or_default();
            if !bases.is_empty() {
                let cur = get_path(view.state(), &parse_path("studio.theme_preview").unwrap())
                    .and_then(|j| j.as_str())
                    .unwrap_or_default()
                    .to_owned();
                let ix = bases
                    .iter()
                    .position(|b| b.as_str() == Some(cur.as_str()))
                    .map(|i| (i + 1) % bases.len())
                    .unwrap_or(0);
                view.update_state(
                    &parse_path("studio.theme_preview").unwrap(),
                    bases[ix].clone(),
                    cx,
                );
                save_prefs(view);
            }
        }
        "persist_preview_base" => save_prefs(view),
        // Preferences → Files — bound `studio.files_*` changes flush to
        // `~/.gpui-forge/settings.toml` on every control change.
        "persist_prefs" => {
            save_prefs(view);
            apply_decorations(view, cx);
        }
        // Preferences → Files — `files_hidden` re-emits the listing.
        "files_pref_changed" => files_pref_changed(view, cx),
        // Preferences → Highlights rows — scoped `bind` writes the row,
        // these check/sync/persist around it.
        "hl_validate" => hl_validate(view, args, cx),
        "hl_commit" => hl_commit(view, args, cx),
        "hl_changed" => hl_changed(view, cx),
        "hl_add" => hl_add(view, cx),
        "hl_remove" => hl_remove(view, args, cx),
        "hl_reset" => hl_reset(view, cx),
        "close_prefs" => {
            save_prefs(view);
            close_dialog(cx);
        }
        // Preferences → Icons — the iconify cache root takes effect
        // immediately (memos cleared), then persists like the other prefs.
        "apply_icon_cache" => {
            let dir = view
                .state()
                .get("studio")
                .and_then(|s| s.get("icon_cache"))
                .and_then(|j| j.as_str())
                .unwrap_or_default()
                .to_owned();
            gpui_forge_runtime::iconify::set_cache_dir((!dir.is_empty()).then(|| dir.into()));
            save_prefs(view);
        }
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
        "guise" | "guise-showcase.forge.yaml" => (
            "../../examples/showcases/guise-showcase.forge.yaml",
            EXAMPLE_GUISE,
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
/// — or the first `[[apps]]` document — resolves relative to it and becomes
/// the loaded document, while `studio.project` tracks the project
/// directory (#49).
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
    view.update_state(
        &parse_path("studio.apps").unwrap(),
        Json::Array(Vec::new()),
        cx,
    );
    view.update_state(
        &parse_path("studio.multiapp").unwrap(),
        Json::Bool(false),
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

/// #148 context menu — the row's path may be a dir or a file. `dir_of` picks
/// the folder the action applies to (files act on their parent).
fn context_dir(arg: Option<&String>, shared: &SharedState) -> Option<PathBuf> {
    let mut dir = arg
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .or_else(|| shared.borrow().dir.clone())?;
    if !dir.is_dir() {
        dir.pop();
    }
    Some(dir)
}

/// `files_new(path)` — the tree's "New document here". Creates the first free
/// `untitled{,-N}.forge.yaml` in the folder seeded with the open document's
/// base, then opens it in the Document tab.
fn files_new(
    view: &mut ForgeView,
    args: &[String],
    shared: SharedState,
    cx: &mut Context<ForgeView>,
) {
    let Some(dir) = context_dir(args.first(), &shared) else {
        status(view, "no project folder for the new document", cx);
        return;
    };
    let mut candidate = dir.join("untitled.forge.yaml");
    let mut n = 2;
    while candidate.exists() {
        candidate = dir.join(format!("untitled-{n}.forge.yaml"));
        n += 1;
    }
    let base = get_path(view.state(), &parse_path("studio.doc_base").unwrap())
        .and_then(Json::as_str)
        .unwrap_or(BASE_ID)
        .to_owned();
    let doc = format!(
        "$schema: {}\nforge: \"0.1\"\nbase: {base}\n\nroot:\n  VStack:\n  style:\n    padding: 16px\n    gap: 8px\n  children:\n    - Text: Hello, Forge!\n",
        gpui_forge_runtime::forge_spec::yaml_schema::schema_url(&base)
    );
    if let Err(e) = std::fs::write(&candidate, &doc) {
        status(
            view,
            &format!("cannot create {}: {e}", candidate.display()),
            cx,
        );
        return;
    }
    status(view, &format!("created {}", candidate.display()), cx);
    // `load_file` refreshes the tree via `load_source`. It lands on Canvas,
    // so re-point at the Document tab like `open_path` does.
    load_file(candidate, view, shared, cx);
    view.update_state(&parse_path("studio.tab").unwrap(), Json::from(1), cx);
}

/// `files_reveal(path)` — show the file or folder in the OS file manager.
fn files_reveal(view: &mut ForgeView, args: &[String], cx: &mut Context<ForgeView>) {
    let Some(path) = args.first().map(PathBuf::from) else {
        return;
    };
    let target = if path.is_dir() {
        path.clone()
    } else {
        path.parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| path.clone())
    };
    #[cfg(target_os = "linux")]
    let spawned = std::process::Command::new("xdg-open").arg(&target).spawn();
    #[cfg(target_os = "macos")]
    let spawned = std::process::Command::new("open").arg(&target).spawn();
    #[cfg(target_os = "windows")]
    let spawned = std::process::Command::new("explorer").arg(&target).spawn();
    match spawned {
        Ok(_) => status(view, &format!("revealed {}", target.display()), cx),
        Err(e) => status(
            view,
            &format!("cannot reveal {}: {e}", target.display()),
            cx,
        ),
    }
}

/// `files_rename(path)` — remember the target, then open the `rename` dialog
/// with the input pre-seeded to the current name (`open_dialog`'s second arg
/// lands in the dialog's `name` field).
fn files_rename(
    view: &mut ForgeView,
    args: &[String],
    shared: &SharedState,
    cx: &mut Context<ForgeView>,
) {
    let Some(path) = args.first().map(PathBuf::from) else {
        return;
    };
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    view.update_state(
        &parse_path("studio.rename_target").unwrap(),
        Json::String(path.display().to_string()),
        cx,
    );
    open_dialog(view, &["rename".to_owned(), name], shared, cx);
}

/// `rename_file()` — the rename dialog's accept action. Reads `studio.rename_target`
/// and the dialog's `name` field, then `fs::rename`s and refreshes the tree.
/// Renaming the open document re-points `shared.file` so a later save doesn't
/// resurrect the old name.
fn rename_file(view: &mut ForgeView, shared: SharedState, cx: &mut Context<ForgeView>) {
    let Some(from) = get_path(view.state(), &parse_path("studio.rename_target").unwrap())
        .and_then(|j| j.as_str().map(str::to_owned))
        .map(PathBuf::from)
    else {
        status(view, "nothing to rename", cx);
        return;
    };
    let fields = get_path(view.state(), &parse_path("dialog.state").unwrap())
        .and_then(|j| j.as_str().map(str::to_owned))
        .and_then(|s| serde_json::from_str::<Json>(&s).ok())
        .unwrap_or_default();
    let name = fields["name"]
        .as_str()
        .unwrap_or_default()
        .trim()
        .to_owned();
    if name.is_empty() || name.contains(['/', '\\']) {
        status(view, "name required (no slashes)", cx);
        return;
    }
    if from.is_file() && !name.ends_with(".forge.yaml") {
        status(view, "file names must end in .forge.yaml", cx);
        return;
    }
    let Some(parent) = from.parent().map(PathBuf::from) else {
        return;
    };
    let to = parent.join(&name);
    if to == from {
        close_dialog(cx);
        return;
    }
    if to.exists() {
        status(view, &format!("{} already exists", to.display()), cx);
        return;
    }
    if let Err(e) = std::fs::rename(&from, &to) {
        status(view, &format!("cannot rename {}: {e}", from.display()), cx);
        return;
    }
    if shared.borrow().file.as_deref() == Some(from.as_path()) {
        shared.borrow_mut().file = Some(to.clone());
        if file_watches(view, &shared) {
            watch_file(to.clone(), shared.clone(), cx);
        }
    }
    refresh_files(view, &parent, cx);
    refresh_assets(view, &parent, cx);
    refresh_themes(view, &parent, cx);
    status(view, &format!("renamed to {}", to.display()), cx);
    close_dialog(cx);
}

/// `window.close_dialog` can't run mid-update (windows are out of the slot
/// map) — the same defer `open_dialog` uses for opening, for closing.
fn close_dialog(cx: &mut Context<ForgeView>) {
    let app = cx.to_async();
    cx.foreground_executor()
        .spawn(async move {
            app.update(|cx| {
                for handle in cx.windows() {
                    let _ = handle.update(cx, |_, window, cx| window.close_dialog(cx));
                }
            });
        })
        .detach();
}

/// `preview_in_host()` — spawn the out-of-process preview host (#97) on the
/// current document. The host renders its own GPUI graph. Studio feeds it
/// the doc path over the JSON control channel and reads its events back.
/// `call`/`state` lines play the same role as the embedded preview's sinks.
fn preview_in_host(view: &mut ForgeView, shared: SharedState, cx: &mut Context<ForgeView>) {
    // `load` for file-backed docs (also seeds the host's base_dir). Unsaved
    // documents go over `source` — the editor buffer is the document.
    let file = shared.borrow().file.clone();
    // One host binary per base — `forge-preview-guise` for guise docs, and
    // the plain `forge-preview` stays the gpui-kit host (#96).
    let base = get_path(view.state(), &parse_path("studio.doc_base").unwrap())
        .and_then(Json::as_str)
        .unwrap_or(BASE_ID)
        .to_owned();
    // #156 — ratatui docs render in-process on the canvas card. No host —
    // it would only stream frames nothing paints.
    if base == "ratatui" {
        return;
    }
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
                        seq: v["seq"].as_u64().unwrap_or(0),
                        cols: v["cols"].as_u64().unwrap_or(80) as u16,
                        rows: v["rows"].as_u64().unwrap_or(24) as u16,
                        data: v["data"].as_str().unwrap_or_default().to_owned(),
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
            view.update_state(
                &parse_path("studio.problem_variant").unwrap(),
                Json::String(if items.is_empty() {
                    "Success".into()
                } else {
                    "Error".into()
                }),
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
        HostEvent::Frame {
            seq,
            cols,
            rows,
            data,
        } => {
            // #120 Phase 3 — the ANSI stream chunk lands in `preview.frame`.
            // the card's emulator consumes it by `seq`.
            view.update_state(
                &parse_path("preview.frame").unwrap(),
                serde_json::json!({"seq": seq, "cols": cols, "rows": rows, "data": data}),
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
        // Stream-pane DnD events — only pane hosts emit them. The pane
        // reader maps each onto `preview.panes.<id>.*` in `pane_sync`.
        HostEvent::Zone(_) | HostEvent::DropTarget(_) | HostEvent::Drop(_) => {}
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
    let dark = get_path(view.state(), &parse_path("studio.dark").unwrap())
        .and_then(|j| j.as_bool())
        .unwrap_or_default();
    let mut cmd = json!({"cmd":"theme","dark": dark,"colors": colors});
    if let Some(name) = get_path(view.state(), &parse_path("studio.theme").unwrap())
        .and_then(|j| j.as_str())
        .map(str::to_owned)
    {
        cmd["name"] = Json::from(name);
    }
    cmd
}

/// A pane's theme mode — `preview.panes.<id>.mode`: `follow` (default —
/// Studio's theme), `dark`, or `light`. Dark/light-only bases pin their own
/// appearance here instead of silently degrading (#144). A pinned pane gets
/// a bare dark/light command — a named theme or palette would paint the
/// *wrong* mode's colors.
fn pane_theme_cmd(theme: &Json, view: &ForgeView, id: &str) -> Json {
    match get_path(view.state(), &pane_state_path(id, "mode")).and_then(|j| j.as_str()) {
        Some("dark") => json!({"cmd":"theme","dark": true,"colors":{}}),
        Some("light") => json!({"cmd":"theme","dark": false,"colors":{}}),
        _ => theme.clone(),
    }
}

fn host_send_theme(view: &ForgeView, cx: &App) {
    host_send(&theme_cmd(view, cx.theme()));
}

/// Media globs the Assets tab lists — images, video, fonts. Keep the
/// `forge.Files` `filter:` in `studio.forge.yaml` in sync.
const ASSET_GLOBS: &[&str] = &[
    "*.png", "*.jpg", "*.jpeg", "*.gif", "*.webp", "*.svg", "*.ico", "*.bmp", "*.avif", "*.mp4",
    "*.webm", "*.mov", "*.mkv", "*.ttf", "*.otf", "*.woff", "*.woff2",
];

const IMAGE_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "svg", "ico", "bmp", "avif",
];

/// The directory the Assets tab browses for `root`: the project's
/// `[assets] dir` when the manifest names one, else `<root>/assets` when it
/// exists, else the root itself.
fn assets_root(root: &std::path::Path) -> PathBuf {
    let declared = std::fs::read_to_string(root.join("forge.toml"))
        .ok()
        .and_then(|src| manifest_section(&src, "assets", "dir"))
        .map(|d| root.join(d));
    declared.filter(|d| d.is_dir()).unwrap_or_else(|| {
        let guess = root.join("assets");
        if guess.is_dir() {
            guess
        } else {
            root.to_path_buf()
        }
    })
}

/// Seeds the Assets tab: `studio.assets_dir` names the browsed root and
/// `studio.asset_files` is the flattened media listing the thumbnail/grid
/// zooms iterate.
fn refresh_assets(view: &mut ForgeView, root: &std::path::Path, cx: &mut Context<ForgeView>) {
    let dir = assets_root(root);
    view.update_state(
        &parse_path("studio.assets_dir").unwrap(),
        Json::String(dir.display().to_string()),
        cx,
    );
    let mut budget = 800usize;
    let filters: Vec<String> = ASSET_GLOBS.iter().map(|s| s.to_string()).collect();
    let entries = gpui_forge_runtime::render::scan_files(&dir, &filters, 4, false, &mut budget);
    let mut files = Vec::new();
    flatten_assets(&entries, &dir, &mut files);
    view.update_state(
        &parse_path("studio.asset_files").unwrap(),
        Json::Array(files),
        cx,
    );
}

/// The project's theme dir: `[themes] dir` in `forge.toml` when declared,
/// else `<root>/themes`. Mirrors `emit_themes`' project half in forge-cli.
fn themes_root(root: &std::path::Path) -> PathBuf {
    std::fs::read_to_string(root.join("forge.toml"))
        .ok()
        .and_then(|src| manifest_section(&src, "themes", "dir"))
        .map(|d| root.join(d))
        .filter(|d| d.is_dir())
        .unwrap_or_else(|| root.join("themes"))
}

/// Loads the project's theme JSONs into the shared registry and re-seeds
/// the Theme tab's select lists so project themes appear alongside the
/// shipped catalog. Registry entries are keyed by theme name — a project
/// theme with the same name replaces the shipped one, matching `forge
/// build`'s project-wins merge.
fn refresh_themes(view: &mut ForgeView, root: &std::path::Path, cx: &mut Context<ForgeView>) {
    // A doc refresh names the doc's folder — when a project is open its
    // themes are the ones that matter, so prefer the project root.
    let project = view.state()["studio"]["project"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let root = if project.is_empty() {
        root.to_path_buf()
    } else {
        PathBuf::from(project)
    };
    let dir = themes_root(&root);
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            if e.path().extension().and_then(|s| s.to_str()) == Some("json")
                && let Ok(content) = std::fs::read_to_string(e.path())
            {
                let _ = ThemeRegistry::global_mut(cx).load_themes_from_str(&content);
            }
        }
    }
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
    let mut all_themes = Vec::with_capacity(light_themes.len() + dark_themes.len() + 2);
    all_themes.push("--- Light Themes ---".to_string());
    all_themes.extend(light_themes.iter().cloned());
    all_themes.push("--- Dark Themes ---".to_string());
    all_themes.extend(dark_themes.iter().cloned());
    for (key, list) in [
        ("studio.light_themes", light_themes),
        ("studio.dark_themes", dark_themes),
        ("studio.all_themes", all_themes),
    ] {
        view.update_state(
            &parse_path(key).unwrap(),
            Json::Array(list.into_iter().map(Json::String).collect()),
            cx,
        );
    }
}

/// `scan_files` returns nested `{name,path,dir,children}` entries — the
/// thumbnail/grid zooms want one flat row per media file.
fn flatten_assets(entries: &[Json], root: &std::path::Path, out: &mut Vec<Json>) {
    for entry in entries {
        if entry["dir"].as_bool().unwrap_or(false) {
            if let Some(children) = entry["children"].as_array() {
                flatten_assets(children, root, out);
            }
            continue;
        }
        let path = entry["path"].as_str().unwrap_or_default();
        let name = entry["name"].as_str().unwrap_or_default();
        let ext = std::path::Path::new(name)
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let rel = std::path::Path::new(path)
            .strip_prefix(root)
            .map(|r| r.display().to_string())
            .unwrap_or_else(|_| name.to_owned());
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or_default();
        out.push(serde_json::json!({
            "name": name,
            "rel": rel,
            "path": path,
            "ext": ext.clone(),
            "img": IMAGE_EXTS.contains(&ext.as_str()),
            "size": size,
        }));
    }
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
// A `forge.Preview` with `remote:` + `id:` declares its own preview host.
// `base:` picks the host binary (empty → the document's base). `doc:`
// picks the document (a doc-relative path, a named ref's head-less body, or
// the parent document when absent). `pane_sync` diffs declared panes against
// running hosts, spawns/kills deltas, streams each pane's document, and
// `frame_rect`s each host window over its tracked slot.

/// A remote `forge.Preview` pane declared in the open document.
struct PaneSpec {
    /// The pane's `#id` — also the `preview_bounds` key its slot tracks.
    id: String,
    base: String,
    /// `doc:` — doc-relative path, ref name, or none (the parent document).
    doc: Option<String>,
    /// `mode:` — `embedded` floats the host window over the slot. `stream`
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
static PANE_STDERR: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<String, StderrTail>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(Default::default()));

fn pane_send(id: &str, value: &Json) {
    use std::io::Write;
    let mut guard = HOSTS.lock().unwrap();
    if let Some((_, host)) = guard.get_mut(id) {
        let _ = writeln!(host.stdin, "{value}");
        let _ = host.stdin.flush();
    }
}

/// Does any top-level X11 window carry `_NET_WM_PID == pid`? `None` when the
/// check can't run (no X connection — e.g. wayland-only) — the caller treats
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
    // `_NET_WM_PID` sits on the client window. Under XWayland/XWM (WSLg) a
    // toplevel needn't be a direct child of root, so walk the whole tree
    // rather than `query_tree(root)`'s first level alone.
    let tree = conn.query_tree(root).ok()?.reply().ok()?;
    let mut stack: Vec<u32> = tree.children.to_vec();
    let mut visited = 0usize;
    while let Some(win) = stack.pop() {
        if visited >= 512 {
            break;
        }
        visited += 1;
        let ours = conn
            .get_property(false, win, pid_atom, AtomEnum::CARDINAL, 0, 1)
            .ok()
            .and_then(|c| c.reply().ok())
            .and_then(|r| r.value32().and_then(|mut v| v.next()))
            == Some(pid);
        if ours {
            return Some(true);
        }
        if let Some(children) = conn.query_tree(win).ok().and_then(|c| c.reply().ok()) {
            stack.extend_from_slice(&children.children);
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

/// #156 — a drop's Studio-side payload awaiting the host's resolved path:
/// `pane id → (kind, payload)` where `kind` is `component` or `node`.
/// `pane_drop`/`pane_move` stash it. The pane's `drop` event commits it.
static PANE_DND: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<String, (String, String)>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

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

/// A pane's last stderr lines — pane stderr is drained rather than
/// inherited, so a host panic would otherwise leave no trace. A short
/// ring is kept and the tail is reported when the host exits.
type StderrTail = std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<String>>>;

fn stderr_tail(map: &std::collections::HashMap<String, StderrTail>, id: &str) -> Option<String> {
    map.get(id).and_then(|t| {
        t.lock()
            .unwrap()
            .iter()
            .rev()
            .find(|l| !l.trim().is_empty())
            .cloned()
    })
}

fn spawn_pane(base: &str, id: &str) -> Option<(Host, StderrTail)> {
    let binary = find_preview_binary(base)?;
    let mut child = preview_command(&binary, base)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        // Drained, not inherited — a pane's panics/debug dumps would spam
        // Studio's own stderr (and crash-looping hosts spam it per respawn).
        .stderr(std::process::Stdio::piped())
        .spawn()
        .ok()?;
    let tail: StderrTail = Default::default();
    if let Some(stderr) = child.stderr.take() {
        let tail = tail.clone();
        std::thread::spawn(move || {
            use std::io::BufRead;
            for line in std::io::BufReader::new(stderr)
                .lines()
                .map_while(Result::ok)
            {
                let mut ring = tail.lock().unwrap();
                if ring.len() >= 30 {
                    ring.pop_front();
                }
                ring.push_back(line);
            }
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
                    Some("call") => {
                        HostEvent::Call(v["call"].as_str().unwrap_or_default().to_owned())
                    }
                    Some("zone") => {
                        HostEvent::Zone(v["path"].as_str().unwrap_or_default().to_owned())
                    }
                    Some("drop_target") => {
                        HostEvent::DropTarget(v["path"].as_str().unwrap_or_default().to_owned())
                    }
                    Some("drop") => {
                        HostEvent::Drop(v["path"].as_str().unwrap_or_default().to_owned())
                    }
                    Some("frame") => HostEvent::Frame {
                        seq: v["seq"].as_u64().unwrap_or(0),
                        cols: v["cols"].as_u64().unwrap_or(80) as u16,
                        rows: v["rows"].as_u64().unwrap_or(24) as u16,
                        data: v["data"].as_str().unwrap_or_default().to_owned(),
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
    Some((
        Host {
            child,
            stdin,
            generation: HOST_GEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1,
        },
        tail,
    ))
}

/// The `source` command for a pane — a named ref becomes a document clone
/// with the ref body as root. No `doc:` previews the parent buffer.
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

/// Starts the pane-sync task once. Diffs declared `remote:` panes against
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
                        HostEvent::Frame {
                            seq,
                            cols,
                            rows,
                            data,
                        } if streams.contains(pid.as_str()) =>
                        {
                            // Feed the emulator per event, in seq order.
                            // The state slot below holds only the last
                            // frame of a drain batch. Render-time feeding
                            // drops every burst except the final diff —
                            // the full first frame was always lost.
                            view.feed_term_frame(
                                &pid, seq, cols, rows, data.as_bytes(),
                            );
                            view.update_state(
                                &pane_state_path(&pid, "frame"),
                                serde_json::json!({"seq": seq, "cols": cols, "rows": rows, "data": data}),
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
                        HostEvent::Zone(path) => {
                            view.update_state(
                                &pane_state_path(&pid, "zone"),
                                Json::String(path),
                                cx,
                            );
                        }
                        HostEvent::DropTarget(path) => {
                            view.update_state(
                                &pane_state_path(&pid, "drop_target"),
                                Json::String(path),
                                cx,
                            );
                        }
                        HostEvent::Drop(path) => {
                            view.update_state(
                                &pane_state_path(&pid, "drop"),
                                Json::String(path.clone()),
                                cx,
                            );
                            // A stashed pane payload pairs with the resolved
                            // path — the same `drop_component`/`move_node`
                            // the gpui canvas fires.
                            if let Some((kind, payload)) =
                                PANE_DND.lock().unwrap().remove(&pid)
                            {
                                if path.is_empty() {
                                    status(
                                        view,
                                        &format!(
                                            "preview pane `{pid}`: drop missed every node"
                                        ),
                                        cx,
                                    );
                                } else {
                                    match kind.as_str() {
                                        "component" => drop_component(
                                            view,
                                            &[payload, path.clone()],
                                            &shared,
                                            cx,
                                        ),
                                        _ => move_node(
                                            view,
                                            &[payload, path.clone()],
                                            &shared,
                                            cx,
                                        ),
                                    }
                                }
                            }
                        }
                        HostEvent::Call(call) => {
                            // Hooks inside the pane's doc (`on <event>:`)
                            // ride the same dispatch as the primary host's.
                            let (name, args) = parse_call(&call);
                            dispatch(view, &name, &args, &shared, cx);
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
                        // Stream panes size their grid to the `TermView`'s
                        // own painted rect — the card's `preview_bounds`
                        // includes padding the grid never sees.
                        let cell = if p.mode == "stream" {
                            view.term_bounds(&p.id)
                                .or_else(|| view.preview_bounds(&p.id))
                        } else {
                            view.preview_bounds(&p.id)
                        };
                        cell.map(|cell| (p.id.clone(), *cell.borrow()))
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
                PANE_STDERR.lock().unwrap().remove(&id);
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
                let tail = stderr_tail(&PANE_STDERR.lock().unwrap(), &id)
                    .map(|l| format!(" — {l}"))
                    .unwrap_or_default();
                if *tries > 1 {
                    if gave_up.insert(id.clone()) {
                        let _ = view.update(cx, |view, cx| {
                            status(
                                view,
                                &format!(
                                    "preview host for `{id}` keeps exiting — giving up{tail}"
                                ),
                                cx,
                            )
                        });
                    }
                } else {
                    let _ = view.update(cx, |view, cx| {
                        status(
                            view,
                            &format!("preview host for `{id}` exited — restarting{tail}"),
                            cx,
                        )
                    });
                }
            }

            // A host that stays alive but never maps a window still claims the
            // card's "separate window" is open. Old-gpui keeps running even
            // when its window creation fails. Probe X11 once, three seconds
            // after spawn. A live child with no `_NET_WM_PID` window is killed
            // and reported instead of sitting as a zombie.
            let unmapped: Vec<String> = pending
                .iter()
                .filter(|(_, (t0, _))| t0.elapsed() > std::time::Duration::from_secs(6))
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
                PANE_STDERR.lock().unwrap().remove(&id);
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
            // Expired entries were probed (or couldn't be — Wayland) once.
            // drop them either way.
            pending.retain(|_, (t0, _)| t0.elapsed() <= std::time::Duration::from_secs(6));

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
                        Some((host, tail)) => {
                            if pane.mode != "stream" {
                                pending.insert(
                                    pane.id.clone(),
                                    (std::time::Instant::now(), host.child.id()),
                                );
                            }
                            HOSTS.lock().unwrap().insert(pane.id.clone(), (spec, host));
                            PANE_STDERR.lock().unwrap().insert(pane.id.clone(), tail);
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
                // same theme everywhere it can, except a pane pinned to
                // dark/light via `preview.panes.<id>.mode` (#144).
                let cmd = view
                    .update(cx, |view, _| pane_theme_cmd(&theme, view, &pane.id))
                    .unwrap_or_else(|_| theme.clone());
                let key = cmd.to_string();
                if themes.get(&pane.id) != Some(&key) {
                    pane_send(&pane.id, &cmd);
                    themes.insert(pane.id.clone(), key);
                }

                // Float the host window over the pane's tracked slot — or, for
                // a streaming host, size its headless grid to the slot.
                let slot = pane
                    .wants_frame_rect()
                    .then(|| slots.get(&pane.id).copied().flatten())
                    .flatten();
                if let (Some(wb), Some(b)) = (wb, slot) {
                    let mut rect = json!({
                        "cmd": "frame_rect",
                        "x": f64::from(wb.origin.x) + f64::from(b.origin.x),
                        "y": f64::from(wb.origin.y) + f64::from(b.origin.y),
                        "w": f64::from(b.size.width),
                        "h": f64::from(b.size.height),
                    });
                    // Stream panes ride the `TermView`'s measured cell
                    // metrics — without them the host guesses an 8x18px
                    // cell and the grids drift on resize (#156).
                    if pane.mode == "stream"
                        && let Ok(Some((cw, ch))) = view.update(cx, |view, _| {
                            view.term_metrics(&pane.id).and_then(|c| *c.borrow())
                        })
                    {
                        rect["cols"] =
                            json!((f64::from(b.size.width) / f64::from(cw)).floor().max(2.));
                        rect["rows"] =
                            json!((f64::from(b.size.height) / f64::from(ch)).floor().max(2.));
                    }
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

/// The project's applications in order: `forge.document` is the primary
/// entry — named by `[package] name` or its file stem — and every `[[apps]]`
/// document follows. Mirrors the window order `forge skeleton` emits.
fn project_apps(src: &str) -> Vec<(String, String)> {
    let mut apps = Vec::new();
    if let Some(doc) = manifest_value(src, "document") {
        let name = manifest_section(src, "package", "name").unwrap_or_else(|| {
            std::path::Path::new(&doc)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .trim_end_matches(".forge")
                .to_owned()
        });
        apps.push((name, doc));
    }
    apps.extend(manifest::manifest_apps(src));
    apps
}

/// Re-marks `studio.apps` rows against the open file so the sidebar
/// highlights the application the editor shows. A member document marks its
/// `meta.window` target's app — the file you edit belongs to that app.
fn mark_active_app(view: &mut ForgeView, file: &std::path::Path, cx: &mut Context<ForgeView>) {
    let Json::Array(apps) = view.state()["studio"]["apps"].clone() else {
        return;
    };
    if apps.is_empty() {
        return;
    }
    let canon = |p: &std::path::Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let open = canon(file);
    // `meta.window` on the open doc resolves relative to the doc's folder.
    let window = view
        .document()
        .and_then(|doc| {
            doc.meta
                .iter()
                .find(|(k, _)| k == "window")
                .map(|(_, v)| v.clone())
        })
        .map(|w| canon(&file.parent().unwrap_or(std::path::Path::new(".")).join(w)));
    let rows = apps
        .into_iter()
        .map(|mut row| {
            let path = row["path"].as_str().map(|p| canon(std::path::Path::new(p)));
            let active = path == Some(open.clone()) || path == window;
            row["active"] = Json::Bool(active);
            row
        })
        .collect();
    view.update_state(&parse_path("studio.apps").unwrap(), Json::Array(rows), cx);
}

/// One scanned `*.forge.yaml` — path plus the `meta:` fields the project
/// tree groups on. Docs that fail to parse still list, just with empty
/// `meta:`.
struct ScannedDoc {
    path: std::path::PathBuf,
    window: Option<String>,
    include: Vec<String>,
    assets: Option<String>,
    icon: Option<String>,
    base: Option<String>,
}

/// A `meta:` scalar off a doc's source without a full parse. This is the
/// fallback for docs whose body has errors — `from_str` fails wholesale
/// on one bad node, but a broken doc still belongs to its app.
fn meta_scalar(src: &str, key: &str) -> Option<String> {
    let mut in_meta = false;
    for line in src.lines() {
        let t = line.trim_start();
        if !in_meta {
            in_meta = line.starts_with("meta:") && t == "meta:";
            continue;
        }
        if !t.is_empty() && !line.starts_with(' ') && !line.starts_with('\t') {
            break;
        }
        if let Some(v) = t.strip_prefix(&format!("{key}:")) {
            let v = v.trim().trim_matches('"');
            if !v.is_empty() {
                return Some(v.to_owned());
            }
        }
    }
    None
}

/// Top-level `key: value` off a doc's source without a full parse — the
/// `meta_scalar` twin for keys that sit outside `meta:`.
fn top_scalar(src: &str, key: &str) -> Option<String> {
    for line in src.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            continue;
        }
        if let Some(v) = line.trim().strip_prefix(&format!("{key}:")) {
            let v = v.trim().trim_matches('"').trim_matches('\'');
            if !v.is_empty() {
                return Some(v.to_owned());
            }
        }
    }
    None
}

/// `meta:` off every `*.forge.yaml` under `dir` — one cheap parse each,
/// the line-scan fallback for docs that fail it.
fn scan_docs(dir: &std::path::Path) -> Vec<ScannedDoc> {
    let mut budget = 400usize;
    let filters = vec!["*.forge.yaml".to_owned()];
    let entries = gpui_forge_runtime::render::scan_files(dir, &filters, 8, false, &mut budget);
    let mut paths = Vec::new();
    let mut stack = entries;
    while let Some(e) = stack.pop() {
        if e["dir"].as_bool() == Some(true) {
            stack.extend(e["children"].as_array().cloned().unwrap_or_default());
        } else if let Some(p) = e["path"].as_str() {
            paths.push(std::path::PathBuf::from(p));
        }
    }
    let canon = |p: &std::path::Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    paths
        .into_iter()
        .map(|path| {
            let src = std::fs::read_to_string(&path).unwrap_or_default();
            let doc = yaml::from_str(&src).ok();
            let meta = |key: &str| {
                doc.as_ref()
                    .and_then(|d| {
                        d.meta
                            .iter()
                            .find(|(n, _)| n == key)
                            .map(|(_, v)| v.clone())
                    })
                    .filter(|v| !v.is_empty())
                    .or_else(|| meta_scalar(&src, key))
            };
            // `meta.include` splits on commas, newlines and whitespace —
            // the same grammar `forge skeleton` uses.
            let include = meta("include")
                .map(|v| {
                    v.split([',', '\n'])
                        .flat_map(str::split_whitespace)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default();
            ScannedDoc {
                path: canon(&path),
                window: meta("window"),
                include,
                assets: meta("assets"),
                icon: meta("icon"),
                base: doc
                    .as_ref()
                    .and_then(|d| d.base.clone())
                    .or_else(|| top_scalar(&src, "base")),
            }
        })
        .collect()
}

/// A project application — window doc, claimed member docs, asset folder.
/// `assets_linker` names what declares the folder: the window doc for
/// `meta.assets`, the manifest for `[assets] dir`. `icon` is the sidebar
/// glyph — a lucide name by default, a project-relative file path when a
/// spec icon exists.
struct ProjectApp {
    name: String,
    icon: String,
    docs: Vec<std::path::PathBuf>,
    assets_dir: Option<std::path::PathBuf>,
    assets_linker: Option<std::path::PathBuf>,
}

/// The sidebar's default app glyph per base — a spec icon
/// (`meta.icon`, `package.icon`) overrides it in `project_groups`.
fn base_icon(base: &str) -> &'static str {
    match base {
        "gpui-kit" => "app-window",
        "ratatui" => "terminal",
        "iced" => "snowflake",
        "guise" => "venetian-mask",
        "bezel" => "frame",
        "adabraka" => "monitor-play",
        _ => "package",
    }
}

/// Claims each `*.forge.yaml` under `dir` for an application: `meta.window`
/// points at the app, `meta.include` follows the includer, `forge.documents`
/// lands on the primary app. The rest come back unassigned. Mirrors
/// `forge skeleton`'s grouping — an unlisted `meta.window` target becomes
/// an application too. Also returns each shown doc's linker — the file
/// whose declaration pulls it in — for the view's `linked by` tooltips.
fn project_groups(
    dir: &std::path::Path,
    src: &str,
) -> (
    Vec<ProjectApp>,
    Vec<std::path::PathBuf>,
    std::collections::HashMap<std::path::PathBuf, std::path::PathBuf>,
) {
    let canon = |p: &std::path::Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let manifest = canon(&dir.join("forge.toml"));
    let docs = scan_docs(dir);
    let mut windows: Vec<(String, std::path::PathBuf)> = project_apps(src)
        .into_iter()
        .map(|(n, d)| (n, canon(&dir.join(d))))
        .collect();
    let declared_len = windows.len();
    // `meta.window` targets are windows even when `[[apps]]` never names
    // them — implicit applications, same as `group_apps`. `implicit_src`
    // keeps the first doc that names each target — its linker.
    let mut member_of: std::collections::HashMap<std::path::PathBuf, std::path::PathBuf> =
        std::collections::HashMap::new();
    let mut implicit_src: std::collections::HashMap<std::path::PathBuf, std::path::PathBuf> =
        std::collections::HashMap::new();
    for d in &docs {
        if let Some(w) = &d.window {
            let target = canon(&d.path.parent().unwrap_or(std::path::Path::new(".")).join(w));
            if target != d.path {
                member_of.insert(d.path.clone(), target.clone());
                implicit_src.entry(target).or_insert_with(|| d.path.clone());
            }
        }
    }
    for target in member_of.values() {
        if !windows.iter().any(|(_, p)| p == target) {
            let name = target
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .trim_end_matches(".forge")
                .to_owned();
            windows.push((name, target.clone()));
        }
    }
    let mut links: std::collections::HashMap<std::path::PathBuf, std::path::PathBuf> =
        std::collections::HashMap::new();
    for (ix, (_, w)) in windows.iter().enumerate() {
        let linker = if ix < declared_len {
            manifest.clone()
        } else {
            implicit_src
                .get(w)
                .cloned()
                .unwrap_or_else(|| manifest.clone())
        };
        links.insert(w.clone(), linker);
    }
    // Include edges — the includer's app claims the included doc. The
    // first includer wins, matching the chase order `collect_docs` uses.
    let mut included_by: std::collections::HashMap<std::path::PathBuf, std::path::PathBuf> =
        std::collections::HashMap::new();
    for d in &docs {
        let base = d.path.parent().unwrap_or(std::path::Path::new("."));
        for inc in &d.include {
            let p = canon(&base.join(inc));
            if p != d.path {
                included_by.entry(p).or_insert_with(|| d.path.clone());
            }
        }
    }
    let declared: std::collections::HashSet<std::path::PathBuf> =
        manifest::manifest_list_section(src, "forge", "documents")
            .into_iter()
            .map(|d| canon(&dir.join(d)))
            .collect();
    let claim = |path: &std::path::Path,
                 seen: &mut std::collections::HashSet<std::path::PathBuf>|
     -> Option<usize> {
        let mut cur = path.to_path_buf();
        loop {
            if let Some(i) = windows.iter().position(|(_, w)| *w == cur) {
                return Some(i);
            }
            if !seen.insert(cur.clone()) {
                return None;
            }
            if let Some(next) = member_of.get(&cur).or_else(|| included_by.get(&cur)) {
                cur = next.clone();
                continue;
            }
            return declared.contains(&cur).then_some(0);
        }
    };
    let manifest_assets = manifest_section(src, "assets", "dir");
    let manifest_base = manifest_section(src, "forge", "base");
    let package_icon = manifest_section(src, "package", "icon");
    let mut groups: Vec<ProjectApp> = windows
        .iter()
        .enumerate()
        .map(|(ix, (name, window))| {
            // `meta.assets` on the window doc wins. `[assets] dir` fills
            // the gap — the same precedence the runtime applies.
            let meta_assets = docs
                .iter()
                .find(|d| d.path == *window)
                .and_then(|d| d.assets.clone());
            let (assets_dir, assets_linker) = if let Some(a) = meta_assets {
                (
                    Some(window.parent().unwrap_or(std::path::Path::new(".")).join(a)),
                    window.clone(),
                )
            } else {
                (
                    manifest_assets.clone().map(|a| dir.join(a)),
                    manifest.clone(),
                )
            };
            let assets_dir = assets_dir.filter(|p| p.is_dir());
            // Base: the window doc's own `base:`, then `[forge] base`,
            // then the runtime default — `forge package`'s precedence.
            let base = docs
                .iter()
                .find(|d| d.path == *window)
                .and_then(|d| d.base.clone())
                .or_else(|| manifest_base.clone())
                .unwrap_or_else(|| BASE_ID.to_owned());
            // Spec icon: `meta.icon` on the window doc, then `package.icon`
            // for the primary app — `forge package`'s precedence. Emits a
            // project-relative path only when the file sits under the
            // project root. The tree falls back to the base's glyph.
            let spec = docs
                .iter()
                .find(|d| d.path == *window)
                .and_then(|d| d.icon.clone())
                .map(|i| window.parent().unwrap_or(std::path::Path::new(".")).join(i))
                .or_else(|| {
                    (ix == 0)
                        .then(|| package_icon.clone().map(|i| dir.join(i)))
                        .flatten()
                });
            let icon = spec
                .map(|p| canon(&p))
                .filter(|p| p.is_file())
                .and_then(|p| {
                    manifest
                        .parent()
                        .and_then(|root| p.strip_prefix(root).ok())
                        .map(|r| r.display().to_string().replace('\\', "/"))
                })
                .unwrap_or_else(|| base_icon(&base).to_owned());
            ProjectApp {
                name: name.clone(),
                icon,
                docs: vec![window.clone()],
                assets_dir: assets_dir.clone(),
                assets_linker: assets_dir.map(|_| assets_linker),
            }
        })
        .collect();
    let mut unassigned = Vec::new();
    for d in &docs {
        if windows.iter().any(|(_, w)| *w == d.path) {
            continue;
        }
        let mut seen = std::collections::HashSet::new();
        match claim(&d.path, &mut seen) {
            Some(i) => {
                // Same precedence `claim` walks — `meta.window`, then
                // `meta.include`, then `forge.documents` on the manifest.
                let linker = member_of
                    .get(&d.path)
                    .or_else(|| included_by.get(&d.path))
                    .cloned()
                    .unwrap_or_else(|| manifest.clone());
                links.insert(d.path.clone(), linker);
                groups[i].docs.push(d.path.clone());
            }
            None => unassigned.push(d.path.clone()),
        }
    }
    (groups, unassigned, links)
}

/// A `scan_files`-shaped file row. `tip` is the row's tooltip — the
/// project view fills it with `path - linked by declarer`.
fn file_item(path: &std::path::Path, tip: &str) -> Json {
    json!({
        "name": path.file_name().unwrap_or_default().to_string_lossy(),
        "path": path.display().to_string(),
        "dir": false,
        "children": [],
        "tip": tip,
    })
}

/// A `scan_files`-shaped folder row. `path` doubles as the tree's row id,
/// so virtual folders name a stable synthetic spot under the project.
fn dir_item(name: &str, path: std::path::PathBuf, children: Vec<Json>, tip: &str) -> Json {
    json!({
        "name": name,
        "path": path.display().to_string(),
        "dir": true,
        "children": children,
        "tip": tip,
    })
}

/// The project-relative form of `p` — falls back to the full path
/// outside `root`.
fn rel_of(root: &std::path::Path, p: &std::path::Path) -> String {
    p.strip_prefix(root).unwrap_or(p).display().to_string()
}

/// `<path> - linked by <declarer>` — the project-view tooltip shape.
fn link_tip(root: &std::path::Path, name: &str, linker: &std::path::Path) -> String {
    format!("{name} - linked by {}", rel_of(root, linker))
}

/// File rows nested by their folders under `root` — an app's docs keep
/// their directory shape instead of listing flat. Tooltips name each
/// doc's declarer from `links`. Folder rows point at the app's window doc.
fn doc_tree(
    root: &std::path::Path,
    paths: &[std::path::PathBuf],
    links: &std::collections::HashMap<std::path::PathBuf, std::path::PathBuf>,
    window: &std::path::Path,
) -> Vec<Json> {
    #[derive(Default)]
    struct Tier {
        files: Vec<Json>,
        dirs: std::collections::BTreeMap<String, Tier>,
    }
    fn emit(
        prefix: &std::path::Path,
        tier: Tier,
        root: &std::path::Path,
        window: &std::path::Path,
    ) -> Vec<Json> {
        let mut out: Vec<Json> = tier
            .dirs
            .into_iter()
            .map(|(name, sub)| {
                let path = prefix.join(&name);
                dir_item(
                    &name,
                    path.clone(),
                    emit(&path, sub, root, window),
                    &link_tip(root, &rel_of(root, &path), window),
                )
            })
            .collect();
        out.extend(tier.files);
        out
    }
    let mut root_tier = Tier::default();
    for p in paths {
        let rel = p.strip_prefix(root).unwrap_or(p.as_path());
        let mut tier = &mut root_tier;
        if let Some(parent) = rel.parent() {
            for part in parent.components() {
                tier = tier
                    .dirs
                    .entry(part.as_os_str().to_string_lossy().into_owned())
                    .or_default();
            }
        }
        let linker = links.get(p).map(|p| p.as_path()).unwrap_or(window);
        tier.files
            .push(file_item(p, &link_tip(root, &rel_of(root, p), linker)));
    }
    emit(root, root_tier, root, window)
}

/// Stamps `<path> - linked by <linker>` tips onto a `scan_files` listing —
/// the project's asset trees get the same tooltip treatment as docs.
fn tip_scan(root: &std::path::Path, linker: &std::path::Path, items: &mut [Json]) {
    for e in items.iter_mut() {
        let path = std::path::PathBuf::from(e["path"].as_str().unwrap_or_default());
        e["tip"] = Json::String(link_tip(root, &rel_of(root, &path), linker));
        if let Some(kids) = e["children"].as_array_mut() {
            tip_scan(root, linker, kids);
        }
    }
}

/// The sidebar's project view — `forge.toml` first, then each application
/// with its claimed docs and asset folder. Only declared membership lists:
/// a file the manifest and `meta:` never name is not the project's, and
/// stays out of this view. Emitted as `scan_files`-shaped entries so
/// `forge.Files of:` renders it like a scanned tree.
fn project_items(dir: &std::path::Path, src: &str, hidden: bool) -> Vec<Json> {
    let (mut apps, _, links) = project_groups(dir, src);
    let base = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    let synth = |parts: &[&str]| {
        let mut p = dir.join(".forge-studio");
        p.extend(parts);
        p
    };
    let mut out = vec![file_item(
        &dir.join("forge.toml"),
        "forge.toml - the project manifest",
    )];
    for app in &mut apps {
        let window = app.docs[0].clone();
        app.docs[1..].sort();
        let app_linker = links
            .get(&window)
            .cloned()
            .unwrap_or_else(|| base.join("forge.toml"));
        let mut kids = vec![dir_item(
            "files",
            synth(&[&app.name, "files"]),
            doc_tree(&base, &app.docs, &links, &window),
            &link_tip(&base, "files", &window),
        )];
        if let (Some(ad), Some(linker)) = (&app.assets_dir, &app.assets_linker) {
            let mut budget = 200usize;
            let mut scan = gpui_forge_runtime::render::scan_files(ad, &[], 6, hidden, &mut budget);
            tip_scan(&base, linker, &mut scan);
            if !scan.is_empty() {
                kids.push(dir_item(
                    "assets",
                    synth(&[&app.name, "assets"]),
                    scan,
                    &link_tip(&base, "assets", linker),
                ));
            }
        }
        let mut app_row = dir_item(
            &app.name,
            synth(&[&app.name]),
            kids,
            &link_tip(&base, &app.name, &app_linker),
        );
        app_row["icon"] = Json::String(app.icon.clone());
        out.push(app_row);
    }
    out
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
    // The project's applications: `forge.document` is the primary entry,
    // every `[[apps]]` document follows — same order `forge skeleton` uses.
    let apps = project_apps(&src);
    let Some((_, rel)) = apps.first() else {
        status(view, "forge.toml names no entry document", cx);
        return;
    };
    let rel = rel.clone();
    // `[icons] set` — a non-lucide iconset served from the project's
    // `<assets>/icons/<set>/` directory. `meta.iconset` on a doc wins. It
    // scopes to docs under `project_dir` — a file opened elsewhere falls
    // back to lucide.
    shared.borrow_mut().project_iconset = manifest_section(&src, "icons", "set");
    // `[forge] watch = true` — opened docs hot-reload and the manifest
    // itself is polled (one watcher per forge.toml. `watch_manifest`
    // re-enters here on change without re-spawning).
    let watching = manifest_section(&src, "forge", "watch").is_some_and(|v| manifest::truthy(&v));
    shared.borrow_mut().project_watch = watching;
    if !watching {
        shared.borrow_mut().manifest_watching = None;
    }
    let dir = toml.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    shared.borrow_mut().project_dir = Some(dir.clone());
    shared.borrow_mut().project_toml = Some(toml.clone());
    // The assets root serves `icons/<set>/` iconsets. The project root
    // itself joins the roots so a spec icon (`meta.icon`, `package.icon`)
    // resolves wherever in the project the file lives.
    add_icon_root(assets_root(&dir));
    add_icon_root(dir.clone());
    let doc_path = dir.join(&rel);
    // `studio.apps` feeds the sidebar's application list — each row's `path`
    // is absolute so `open_path` can load it directly.
    let canon = |p: &std::path::Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let open = canon(&doc_path);
    let rows: Vec<Json> = apps
        .iter()
        .map(|(name, doc)| {
            let path = dir.join(doc);
            json!({
                "name": name,
                "doc": doc,
                "path": path.display().to_string(),
                "active": canon(&path) == open,
            })
        })
        .collect();
    view.update_state(&parse_path("studio.apps").unwrap(), Json::Array(rows), cx);
    view.update_state(
        &parse_path("studio.multiapp").unwrap(),
        Json::Bool(apps.len() > 1),
        cx,
    );
    view.update_state(
        &parse_path("studio.project").unwrap(),
        Json::String(dir.display().to_string()),
        cx,
    );
    view.update_state(
        &parse_path("studio.files_mode").unwrap(),
        Json::String("project".to_owned()),
        cx,
    );
    view.update_state(
        &parse_path("studio.toml_source").unwrap(),
        Json::String(src.clone()),
        cx,
    );
    // The Editor sub-tab's form fields — each reads its manifest key, an
    // empty field means "absent". `documents` lists one path per line and
    // `[[apps]]` comes back as `name = doc` lines (name optional).
    let mut set = |key: &str, v: Json| {
        view.update_state(&parse_path(&format!("studio.{key}")).unwrap(), v, cx);
    };
    for key in ["name", "version", "id", "title", "icon", "binary"] {
        set(
            &format!("proj_{key}"),
            Json::String(manifest_section(&src, "package", key).unwrap_or_default()),
        );
    }
    set(
        "proj_base",
        Json::String(manifest_section(&src, "forge", "base").unwrap_or_default()),
    );
    set(
        "proj_document",
        Json::String(manifest_value(&src, "document").unwrap_or_default()),
    );
    set(
        "proj_watch",
        Json::Bool(manifest_section(&src, "forge", "watch").is_some_and(|v| manifest::truthy(&v))),
    );
    set(
        "proj_documents",
        Json::String(manifest::manifest_list_section(&src, "forge", "documents").join("\n")),
    );
    set(
        "proj_apps",
        Json::String(
            manifest::manifest_apps(&src)
                .into_iter()
                .map(|(n, d)| format!("{n} = {d}"))
                .collect::<Vec<_>>()
                .join("\n"),
        ),
    );
    set(
        "proj_assets_dir",
        Json::String(manifest_section(&src, "assets", "dir").unwrap_or_default()),
    );
    set(
        "proj_themes_dir",
        Json::String(manifest_section(&src, "themes", "dir").unwrap_or_default()),
    );
    set(
        "proj_iconset",
        Json::String(manifest_section(&src, "icons", "set").unwrap_or_default()),
    );
    // Root the workspace panels at the project — the Files tree, Assets
    // tab and theme list all follow `studio.project`.
    refresh_files(view, &dir, cx);
    refresh_assets(view, &dir, cx);
    refresh_themes(view, &dir, cx);
    if watching && shared.borrow().manifest_watching.as_deref() != Some(toml.as_path()) {
        shared.borrow_mut().manifest_watching = Some(toml.clone());
        watch_manifest(toml.clone(), shared.clone(), cx);
    }
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
            mark_active_app(view, &path, cx);
            let recents = record_recent(&path);
            refresh_recent_menu(view, &recents, cx);
            // `meta.watch` or project `[forge] watch` — hot-reload on save.
            if file_watches(view, &shared) {
                watch_file(path.clone(), shared.clone(), cx);
            }
            // The project tree highlights `studio.file_selected` — keep it on
            // the open file whichever way it was opened (tree, recents, menu).
            view.update_state(
                &parse_path("studio.file_selected").unwrap(),
                Json::String(path.display().to_string()),
                cx,
            );
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

/// The `native_decorations` preference — true means the WM draws the
/// window frame and title bar. Defaults preserve the platform's existing
/// look: server frames on Linux and Windows, the transparent unified
/// titlebar on macOS. Docs read it as `studio.native_decorations`, so an
/// `if:` can hide an in-app title row or move the menus.
fn native_decorations(prefs: &Json) -> bool {
    prefs
        .get("native_decorations")
        .and_then(Json::as_bool)
        .unwrap_or(!cfg!(target_os = "macos"))
}

fn load_prefs() -> Json {
    let s = prefs_file()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .unwrap_or_default();
    // tiny TOML-ish parse: `key = value` lines. Strings quoted, bools and
    // numbers bare.
    let mut m = serde_json::Map::new();
    for line in s.lines() {
        let line = line.trim();
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let (k, v) = (k.trim(), v.trim());
        if v == "true" || v == "false" {
            m.insert(k.to_owned(), Json::Bool(v == "true"));
        } else if let Ok(n) = v.parse::<f64>() {
            m.insert(k.to_owned(), Json::from(n));
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
    let preview_base = view
        .state()
        .get("studio")
        .and_then(|s| s.get("theme_preview"))
        .and_then(|j| j.as_str())
        .unwrap_or_default()
        .to_owned();
    // `forge.Files` options — the Preferences → Files page binds these.
    let files = |key: &str| {
        view.state()
            .get("studio")
            .and_then(|s| s.get(key))
            .cloned()
            .unwrap_or(Json::Null)
    };
    if let Some(f) = prefs_file() {
        let _ = std::fs::create_dir_all(f.parent().unwrap());
        let mut body =
            format!("theme = \"{theme}\"\ndark = {dark}\ntheme_preview = \"{preview_base}\"\n");
        let bool_pref = |v: &Json| {
            if v.as_bool().unwrap_or(false) {
                "true"
            } else {
                "false"
            }
        };
        body += &format!(
            "files_search = {}\nfiles_icons = {}\nfiles_hidden = {}\n",
            bool_pref(&files("files_search")),
            bool_pref(&files("files_icons")),
            bool_pref(&files("files_hidden")),
        );
        let folders = files("files_folders");
        let folders = folders.as_str().unwrap_or("tree");
        let highlights = files("files_highlights");
        let highlights = highlights.as_str().unwrap_or_default();
        body += &format!("files_folders = \"{folders}\"\nfiles_highlights = \"{highlights}\"\n");
        let icon_cache = files("icon_cache");
        let icon_cache = icon_cache.as_str().unwrap_or_default();
        body += &format!("icon_cache = \"{icon_cache}\"\n");
        body += &format!(
            "native_decorations = {}\n",
            bool_pref(&files("native_decorations"))
        );
        // #150 — last observed geometry, when the poll task has seen any.
        if let Some(w) = *LAST_WINDOW.lock().unwrap() {
            body += &format!(
                "window_x = {}\nwindow_y = {}\nwindow_w = {}\nwindow_h = {}\nwindow_maximized = {}\n",
                w.x, w.y, w.w, w.h, w.maximized
            );
        }
        let _ = std::fs::write(f, body);
    }
}

/// The fixed palette a new highlight row picks from — hex strings both the
/// `highlights:` spec parser and `ColorPicker`'s `colors:` prop read.
/// Factory `files_highlights` — common file kinds keyed to palette colors.
/// The dotfile rule goes last: matching takes the first glob that hits.
const HL_DEFAULT: &str = "*.forge.yaml = #e03131 bold; *.toml = #f59f00 bold; \
    *.md = #1c7ed6; *.rs = #f76707; *.json = #37b24d; *.yaml = #ae3ec9; \
    *.yml = #ae3ec9; .* = #868e96";

/// Twelve-color pool for the `colors:` featured palette — `hl_add` hands
/// out the first one no row uses yet.
const HL_PALETTE: &[&str] = &[
    "#e03131", "#f76707", "#f59f00", "#37b24d", "#0ca678", "#1098ad", "#1c7ed6", "#3b5bdb",
    "#7048e8", "#ae3ec9", "#d6336c", "#868e96",
];

/// Highlight-spec words that are styles, not colors — weights, `italic`,
/// `underline`. `hl_seed` keeps the first non-style word as the row color.
const HL_STYLE_WORDS: &[&str] = &[
    "italic",
    "underline",
    "thin",
    "light",
    "medium",
    "normal",
    "semibold",
    "bold",
    "black",
];

/// `studio.hl_list` — the editable rows `{id, glob, color, bold, error}` behind
/// the `files_highlights` shorthand. `bind $h.glob` writes rows directly
/// (scope origins), `hl_validate`/`hl_add`/`hl_remove` mutate the list.
fn hl_rows(view: &ForgeView) -> Vec<Json> {
    view.state()["studio"]["hl_list"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

/// Rebuilds `studio.files_highlights` — `glob = color` joined by `;` —
/// from the row list. Empty globs drop out. The rest round-trips to the
/// prefs file verbatim.
fn hl_sync(view: &mut ForgeView, rows: &[Json], cx: &mut Context<ForgeView>) {
    let spec = rows
        .iter()
        .filter_map(|row| {
            let glob = row["glob"].as_str()?.trim();
            (!glob.is_empty()).then(|| {
                let color = row["color"].as_str().unwrap_or_default();
                let bold = if row["bold"].as_bool().unwrap_or(false) {
                    " bold"
                } else {
                    ""
                };
                format!("{glob} = {color}{bold}")
            })
        })
        .collect::<Vec<_>>()
        .join("; ");
    view.update_state(
        &parse_path("studio.files_highlights").unwrap(),
        Json::String(spec),
        cx,
    );
}

/// Parses `files_highlights` shorthand (`glob = spec` pairs) into `hl_list`
/// rows. A spec's color is its last non-style word. A style-only spec
/// falls back to a palette color. Runs when prefs load and before the
/// dialog's `studio` copy is taken.
fn hl_seed(view: &mut ForgeView, cx: &mut Context<ForgeView>) {
    let spec = view.state()["studio"]["files_highlights"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let mut rows = Vec::new();
    for (i, part) in spec.split([';', '\n']).enumerate() {
        let Some((glob, spec)) = part.split_once('=') else {
            continue;
        };
        let glob = glob.trim();
        if glob.is_empty() {
            continue;
        }
        let color = spec
            .split(|c: char| c.is_whitespace() || c == ',')
            .rfind(|w| !w.is_empty() && !HL_STYLE_WORDS.contains(w))
            .unwrap_or(HL_PALETTE[i % HL_PALETTE.len()])
            .to_owned();
        let bold = spec
            .split(|c: char| c.is_whitespace() || c == ',')
            .any(|w| matches!(w, "bold" | "semibold" | "black"));
        rows.push(json!({
            "id": i.to_string(),
            "glob": glob,
            "color": color,
            "bold": bold,
            "error": "",
        }));
    }
    view.update_state(
        &parse_path("studio.hl_list").unwrap(),
        Json::Array(rows),
        cx,
    );
}

/// A glob the highlighter can match — `*`, `?` and literal text. Other
/// wildcard syntax fails quietly, so it gets a small error instead.
fn valid_glob(glob: &str) -> Result<(), String> {
    if glob.trim().is_empty() {
        return Err("glob is empty".to_owned());
    }
    if glob.chars().all(|c| c == '*' || c == '?') {
        return Err("matches every file".to_owned());
    }
    if glob.contains('/') {
        return Err("globs match file names, not paths".to_owned());
    }
    if let Some(c) = glob.chars().find(|c| "[]{}!\\".contains(*c)) {
        return Err(format!("`{c}` isn't a wildcard — only `*` and `?` match"));
    }
    Ok(())
}

/// `hl_validate(id)` — a row's glob input wrote its value already. Check
/// it, flag `error`, resync the shorthand. Persistence waits for submit,
/// add/remove, or close — not every keystroke.
fn hl_validate(view: &mut ForgeView, args: &[String], cx: &mut Context<ForgeView>) {
    let Some(id) = args.first() else {
        return;
    };
    let mut rows = hl_rows(view);
    if let Some(row) = rows
        .iter_mut()
        .find(|r| r["id"].as_str() == Some(id.as_str()))
    {
        let glob = row["glob"].as_str().unwrap_or_default().to_owned();
        row["error"] = Json::String(valid_glob(&glob).err().unwrap_or_default());
    }
    view.update_state(
        &parse_path("studio.hl_list").unwrap(),
        Json::Array(rows.clone()),
        cx,
    );
    hl_sync(view, &rows, cx);
}

/// `hl_add` — a new row with a palette color the list isn't using yet,
/// wrapping when all twelve are taken.
fn hl_add(view: &mut ForgeView, cx: &mut Context<ForgeView>) {
    let mut rows = hl_rows(view);
    let used: Vec<&str> = rows.iter().filter_map(|r| r["color"].as_str()).collect();
    let color = HL_PALETTE
        .iter()
        .find(|c| !used.contains(c))
        .copied()
        .unwrap_or_else(|| HL_PALETTE[rows.len() % HL_PALETTE.len()]);
    let id = rows
        .iter()
        .filter_map(|r| r["id"].as_str()?.parse::<usize>().ok())
        .max()
        .map(|m| m + 1)
        .unwrap_or(0);
    rows.push(json!({
        "id": id.to_string(),
        "glob": "*.forge.yaml",
        "color": color,
        "bold": false,
        "error": "",
    }));
    view.update_state(
        &parse_path("studio.hl_list").unwrap(),
        Json::Array(rows.clone()),
        cx,
    );
    hl_sync(view, &rows, cx);
    save_prefs(view);
}

/// `hl_reset` — restores the factory highlight list and persists it.
fn hl_reset(view: &mut ForgeView, cx: &mut Context<ForgeView>) {
    view.update_state(
        &parse_path("studio.files_highlights").unwrap(),
        Json::String(HL_DEFAULT.to_owned()),
        cx,
    );
    hl_seed(view, cx);
    save_prefs(view);
}

/// `hl_remove(id)` — drops the row, resyncs, persists.
fn hl_remove(view: &mut ForgeView, args: &[String], cx: &mut Context<ForgeView>) {
    let Some(id) = args.first() else {
        return;
    };
    let rows: Vec<Json> = hl_rows(view)
        .into_iter()
        .filter(|r| r["id"].as_str() != Some(id.as_str()))
        .collect();
    view.update_state(
        &parse_path("studio.hl_list").unwrap(),
        Json::Array(rows.clone()),
        cx,
    );
    hl_sync(view, &rows, cx);
    save_prefs(view);
}

/// `hl_changed` — a color pick wrote its row already. Resync and persist.
fn hl_changed(view: &mut ForgeView, cx: &mut Context<ForgeView>) {
    let rows = hl_rows(view);
    hl_sync(view, &rows, cx);
    save_prefs(view);
}

/// `hl_commit(id)` — Enter on a glob field — check, resync, persist.
fn hl_commit(view: &mut ForgeView, args: &[String], cx: &mut Context<ForgeView>) {
    hl_validate(view, args, cx);
    save_prefs(view);
}

/// `files_pref_changed` — `files_hidden` feeds the emitted listing rather
/// than a bound prop, so the tree re-emits before prefs flush to disk.
fn files_pref_changed(view: &mut ForgeView, cx: &mut Context<ForgeView>) {
    let dir = view.state()["studio"]["project_dir"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    if !dir.is_empty() {
        emit_project_items(view, std::path::Path::new(&dir), cx);
    }
    save_prefs(view);
}

/// Live window-chrome switch — `request_decorations` applies on Linux.
/// macOS ignores it, so the transparent-titlebar half of the preference
/// there needs a restart.
fn apply_decorations(view: &ForgeView, cx: &mut Context<ForgeView>) {
    let native = get_path(
        view.state(),
        &parse_path("studio.native_decorations").unwrap(),
    )
    .and_then(|j| j.as_bool())
    .unwrap_or(false);
    if let Some(h) = cx.windows().first() {
        let _ = h.update(cx, |_, w, _| {
            w.request_decorations(if native {
                WindowDecorations::Server
            } else {
                WindowDecorations::Client
            });
        });
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
            let keep = view
                .update(cx, move |view, cx| {
                    if source(view) == src {
                        return true;
                    }
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.display().to_string());
                    let dir = path.parent().map(|p| p.to_path_buf());
                    load_source(view, &name, &src, dir.as_deref(), &shared, cx);
                    status(view, "reloaded — file changed on disk", cx);
                    // A reload that dropped `meta.watch` ends the watch.
                    file_watches(view, &shared)
                })
                .unwrap_or(false);
            if !keep {
                return;
            }
        }
    })
    .detach();
}

/// `meta.watch` on `studio.forge.yaml` — the shell hot-reloads itself.
/// Valid reloads swap in via `set_document` (view state survives). A parse
/// failure keeps the last-good document and reports the first diagnostic
/// in the status bar instead of blanking the window. Removing `watch` from
/// a valid reload ends the watch.
fn watch_shell(path: PathBuf, shared: SharedState, cx: &mut Context<ForgeView>) {
    cx.spawn(async move |view, cx| {
        let mut mtime = std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok());
        loop {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(1))
                .await;
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
            let shared = shared.clone();
            let keep = view
                .update(cx, move |view, cx| {
                    let catalogs = shared.borrow().catalogs.clone();
                    let (doc, diagnostics) = check::check(&src, BASE_ID, &catalogs);
                    match doc {
                        Some(doc) => {
                            let keep = doc_watches(&doc);
                            view.set_document(doc, cx);
                            // Clear any reload diagnostics a broken edit left.
                            view.update_state(
                                &parse_path("studio.diagnostics").unwrap(),
                                Json::String(problems(&diagnostics)),
                                cx,
                            );
                            view.update_state(
                                &parse_path("studio.problem_variant").unwrap(),
                                Json::String(problem_variant(&diagnostics)),
                                cx,
                            );
                            view.update_state(
                                &parse_path("studio.markers").unwrap(),
                                Json::Array(Vec::new()),
                                cx,
                            );
                            status(view, "shell reloaded — studio.forge.yaml changed", cx);
                            keep
                        }
                        None => {
                            // Surface on every channel `load_source` uses:
                            // status bar + diagnostics panel + editor markers.
                            view.update_state(
                                &parse_path("studio.diagnostics").unwrap(),
                                Json::String(problems(&diagnostics)),
                                cx,
                            );
                            view.update_state(
                                &parse_path("studio.problem_variant").unwrap(),
                                Json::String(problem_variant(&diagnostics)),
                                cx,
                            );
                            view.update_state(
                                &parse_path("studio.markers").unwrap(),
                                markers(&diagnostics),
                                cx,
                            );
                            let msg = format!(
                                "shell reload failed — {}",
                                diagnostics
                                    .first()
                                    .map(|d| {
                                        format!(
                                            "studio.forge.yaml:{}:{}: {}",
                                            d.pos.line, d.pos.col, d.message
                                        )
                                    })
                                    .unwrap_or_else(|| "invalid yaml".into())
                            );
                            status(view, &msg, cx);
                            // Keep polling — the next edit may fix it.
                            true
                        }
                    }
                })
                .unwrap_or(false);
            if !keep {
                return;
            }
        }
    })
    .detach();
}

/// `[forge] watch` — polls the project manifest and re-runs `load_project`
/// on change so `document`/`icons`/`assets`/`themes` edits apply live.
/// Exits when `manifest_watching` moves to a different `forge.toml`.
fn watch_manifest(toml: PathBuf, shared: SharedState, cx: &mut Context<ForgeView>) {
    cx.spawn(async move |view, cx| {
        let mut mtime = std::fs::metadata(&toml)
            .ok()
            .and_then(|m| m.modified().ok());
        loop {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(1))
                .await;
            if shared.borrow().manifest_watching.as_deref() != Some(toml.as_path()) {
                return;
            }
            let new = std::fs::metadata(&toml)
                .ok()
                .and_then(|m| m.modified().ok());
            if new.is_none() || new == mtime {
                continue;
            }
            mtime = new;
            let toml = toml.clone();
            let shared = shared.clone();
            view.update(cx, move |view, cx| {
                load_project(toml, view, shared, cx);
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
    let (doc, diagnostics) = check::check(src, &doc_base(src, BASE_ID), &catalogs);
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
        &parse_path("studio.problem_variant").unwrap(),
        Json::String(problem_variant(&diagnostics)),
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
        // The Files sidebar's "project" toggle returns here when it leaves
        // the project root.
        view.update_state(
            &parse_path("studio.doc_dir").unwrap(),
            Json::String(dir.display().to_string()),
            cx,
        );
    }
    if let Some(doc) = doc {
        // Vendor-on-use — download just the icons this doc references into
        // `<assets>/icons/<set>/`, then re-render so they resolve locally.
        vendor_doc_icons(dir, &doc, shared, cx);
        view.set_preview_document("canvas", doc, cx);
        // `[icons] set` scopes to docs under the project root — a file opened
        // elsewhere, or with no project loaded, resolves icons as lucide.
        let iconset = {
            let shared = shared.borrow();
            dir.is_some_and(|d| {
                shared
                    .project_dir
                    .as_ref()
                    .is_some_and(|root| d.starts_with(root))
            })
            .then(|| shared.project_iconset.clone())
            .flatten()
        };
        let root = dir.map(assets_root);
        view.preview("canvas", cx).update(cx, |p, _| {
            p.set_project_iconset(iconset);
            if let Some(dir) = dir {
                p.set_base_dir(dir.to_path_buf());
            }
        });
        if let Some(dir) = dir {
            add_icon_root(dir.join("assets"));
        }
        if let Some(root) = root {
            add_icon_root(root);
        }
    }
    host_send_source(view, shared);
    // #148 — document context roots the sidebar at the document's folder,
    // falling back to the last-opened folder when the source has no file
    // behind it. An open project keeps its scope — the Files tree only
    // leaves the project root when `files_scope` toggles it.
    if view.state()["studio"]["project"]
        .as_str()
        .unwrap_or_default()
        .is_empty()
    {
        let root = dir
            .map(|d| d.to_path_buf())
            .or_else(|| shared.borrow().dir.clone());
        if let Some(root) = root {
            refresh_files(view, &root, cx);
            refresh_assets(view, &root, cx);
            refresh_themes(view, &root, cx);
        }
    }
}

/// Icons-tab copy — writes the clicked icon's SVG into the open document's
/// `assets/icons[<set>]/` so the built app ships it, plus the set's
/// `LICENSE.txt` on first copy. Iconify sets synthesize bytes locally from
/// the cached `IconifyJSON`. Bundled/vendored icons load straight from the
/// asset source. Lands under a registered icon root, so the browser's
/// "on disk" ring shows on the next render.
fn icon_copy(view: &mut ForgeView, name: &str, shared: &SharedState, cx: &mut Context<ForgeView>) {
    let set = get_path(view.state(), &parse_path("studio.iconset").unwrap())
        .and_then(|j| j.as_str())
        .unwrap_or_default()
        .to_owned();
    let bundled = matches!(set.as_str(), "" | "lucide" | "heroicons");
    let rel = if bundled {
        format!("icons/{name}.svg")
    } else {
        format!("icons/{set}/{name}.svg")
    };
    let Some(bytes) = cx.asset_source().load(&rel).ok().flatten() else {
        status(
            view,
            &format!("icon `{name}` isn't available locally yet"),
            cx,
        );
        return;
    };
    let Some(dir) = shared.borrow().dir.clone() else {
        status(view, "open a document first — no assets/ to copy into", cx);
        return;
    };
    let dest_dir = if bundled {
        assets_root(&dir).join("icons")
    } else {
        assets_root(&dir).join("icons").join(&set)
    };
    let dest = dest_dir.join(format!("{name}.svg"));
    match std::fs::create_dir_all(&dest_dir).and_then(|_| std::fs::write(&dest, &bytes)) {
        Ok(()) => {
            // Ship attribution with the first copied icon of a licensed set.
            if !bundled
                && let Some((title, license, _spdx)) = gpui_forge_runtime::iconify::set_info(&set)
            {
                let lic = dest_dir.join("LICENSE.txt");
                if !lic.exists() {
                    let _ = std::fs::write(
                        &lic,
                        format!("{title} icon set\nLicense: {license}\nhttps://iconify.design/\n"),
                    );
                }
            }
            status(
                view,
                &format!("copied {set}:{name} → {}", dest.display()),
                cx,
            );
        }
        Err(e) => status(view, &format!("copy failed: {e}"), cx),
    }
    cx.refresh_windows();
}

/// Vendor-on-use — walks a freshly-loaded doc for `icon:` props and `Icon`
/// element names under a non-lucide iconset, then downloads just those SVGs
/// into `<assets>/icons/<set>/`. The files land under the registered icon
/// root. The next render resolves them through the normal asset path —
/// a project accumulates exactly the icons it uses.
fn vendor_doc_icons(
    dir: Option<&std::path::Path>,
    doc: &Document,
    shared: &SharedState,
    cx: &mut Context<ForgeView>,
) {
    let Some(dir) = dir else { return };
    // Same precedence as the preview's `iconset()` — `meta.iconset`, then a
    // project-scoped `[icons] set`, then lucide (nothing to vendor).
    let iconset = doc
        .meta
        .iter()
        .find(|(k, _)| k == "iconset")
        .map(|(_, v)| v.clone())
        .or_else(|| {
            let shared = shared.borrow();
            dir.starts_with(shared.project_dir.as_ref()?)
                .then(|| shared.project_iconset.clone())
                .flatten()
        })
        .unwrap_or_default();
    if matches!(iconset.as_str(), "" | "lucide") {
        return;
    }
    let mut names = Vec::new();
    collect_icon_names(&doc.root, &mut names);
    if names.is_empty() {
        return;
    }
    let dest = assets_root(dir).join("icons").join(&iconset);
    let set_name = iconset.clone();
    cx.spawn(async move |_, cx| {
        // Blocking ureq calls go on the background executor. A
        // non-iconify set (e.g. a custom local one) errors out rather than
        // 404ing per icon, since `vendor_blocking` gates on the catalog.
        let result = cx
            .background_executor()
            .spawn(async move {
                gpui_forge_runtime::iconify::vendor_blocking(&iconset, &names, &dest)
                    .map(|(n, missing)| (n, missing, dest))
            })
            .await;
        match result {
            Ok((0, _, _)) => {}
            Ok((n, missing, dest)) => {
                eprintln!("[icons] vendored {n} icons → {}", dest.display());
                if !missing.is_empty() {
                    eprintln!("[icons] not found in `{set_name}`: {}", missing.join(", "));
                }
            }
            Err(e) => eprintln!("[icons] vendor failed: {e}"),
        }
        // Fresh files under the assets root resolve on the next render.
        cx.update(|app| app.refresh_windows());
    })
    .detach();
}

/// Every icon name a node tree references — `Icon` element content/`name`
/// and any element's `icon:` prop — recursing `children` and control-flow
/// branch lists (`else:`, `match` cases).
fn collect_icon_names(node: &gpui_forge_runtime::forge_spec::model::Node, out: &mut Vec<String>) {
    use gpui_forge_runtime::forge_spec::model::{Control, Value};
    let as_name = |v: &Value| match v {
        Value::String(s) | Value::Enum(s) => Some(s.clone()),
        _ => None,
    };
    if node.element.name == "Icon"
        && let Some(name) = node
            .props
            .iter()
            .find(|(k, _)| k == "name")
            .and_then(|(_, v)| as_name(v))
            .or_else(|| node.content.as_ref().and_then(as_name))
    {
        out.push(name);
    }
    if let Some(name) = node
        .props
        .iter()
        .find(|(k, _)| k == "icon")
        .and_then(|(_, v)| as_name(v))
    {
        out.push(name);
    }
    fn walk(nodes: &[gpui_forge_runtime::forge_spec::model::Node], out: &mut Vec<String>) {
        for child in nodes {
            collect_icon_names(child, out);
        }
    }
    walk(&node.children, out);
    match &node.control {
        Some(Control::If { otherwise, .. })
        | Some(Control::For { otherwise, .. })
        | Some(Control::Match { otherwise, .. }) => walk(otherwise, out),
        _ => {}
    }
    if let Some(Control::Match { cases, .. }) = &node.control {
        for case in cases {
            walk(&case.children, out);
        }
    }
}

/// The sidebar's "project" button — toggles `studio.project_dir` between the
/// project root (`studio.project`, set by `open_project`) and the current
/// document's folder (`studio.doc_dir`).
fn files_scope(view: &mut ForgeView, cx: &mut Context<ForgeView>) {
    let (project, current) = {
        let state = view.state();
        (
            state["studio"]["project"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
            state["studio"]["files_mode"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
        )
    };
    if project.is_empty() {
        status(view, "no project — open a forge.toml first", cx);
        return;
    }
    // `project` view shows declared membership, `files` browses the whole
    // folder — same root, two listings.
    let mode = if current == "files" {
        "project"
    } else {
        "files"
    };
    view.update_state(
        &parse_path("studio.files_mode").unwrap(),
        Json::String(mode.to_owned()),
        cx,
    );
    emit_project_items(view, std::path::Path::new(&project), cx);
}

/// Appends `rel` to `[forge] documents` in the manifest text — creates
/// the array when absent. Members land on the primary app, matching how
/// `forge skeleton` resolves the list.
fn add_document(src: &str, rel: &str) -> String {
    let mut doc = src.parse::<toml_edit::DocumentMut>().unwrap_or_default();
    let mut arr = doc
        .get("forge")
        .and_then(|f| f.get("documents"))
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();
    arr.push(rel);
    doc["forge"]["documents"] = toml_edit::value(arr);
    doc.to_string()
}

/// `files_add` — the tree's "Add to project" context action. Adds the doc
/// to `[forge] documents` and reloads the project so the hierarchy shows
/// the new member.
fn files_add(
    view: &mut ForgeView,
    args: &[String],
    shared: &SharedState,
    cx: &mut Context<ForgeView>,
) {
    let Some(path) = args.first().map(PathBuf::from) else {
        return;
    };
    let Some(toml) = shared.borrow().project_toml.clone() else {
        status(view, "no project open", cx);
        return;
    };
    if !path.to_string_lossy().ends_with(".forge.yaml") {
        status(view, "only Forge documents join a project", cx);
        return;
    }
    let dir = toml.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let canon = |p: &std::path::Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let doc_path = canon(&path);
    let Ok(rel) = doc_path.strip_prefix(canon(&dir)) else {
        status(view, "file is outside the project", cx);
        return;
    };
    // Already claimed? `claim` resolution says so — nothing to write.
    let src = std::fs::read_to_string(&toml).unwrap_or_default();
    let (groups, _, _) = project_groups(&dir, &src);
    if groups
        .iter()
        .flat_map(|g| g.docs.iter())
        .any(|d| *d == doc_path)
    {
        status(view, "already in the project", cx);
        return;
    }
    let rel = rel.to_string_lossy().replace('\\', "/");
    match std::fs::write(&toml, add_document(&src, &rel)) {
        Ok(()) => {
            status(view, &format!("added {rel} to the project"), cx);
            load_project(toml, view, shared.clone(), cx);
        }
        Err(e) => status(view, &format!("cannot write {}: {e}", toml.display()), cx),
    }
}

/// `on change: grep_project($studio.project_grep)` — finds the query text
/// inside every readable file under the Files root. It rewrites
/// `studio.project_items` into flat `file:line` match rows, so the same
/// `forge.Files` tree lists them. An empty query emits the normal listing
/// back.
fn grep_project(view: &mut ForgeView, query: &str, cx: &mut Context<ForgeView>) {
    let query = query.trim();
    if query.is_empty() {
        let dir = view.state()["studio"]["project_dir"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        if !dir.is_empty() {
            emit_project_items(view, std::path::Path::new(&dir), cx);
        }
        return;
    }
    let root = view.state()["studio"]["project_dir"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    if root.is_empty() {
        return;
    }
    let needle = query.to_lowercase();
    let mut budget = 600usize;
    let entries = gpui_forge_runtime::render::scan_files(
        std::path::Path::new(&root),
        &[],
        8,
        false,
        &mut budget,
    );
    let mut rows = Vec::new();
    let mut stack = entries;
    while let Some(e) = stack.pop() {
        if rows.len() >= 200 {
            break;
        }
        if e["dir"].as_bool() == Some(true) {
            stack.extend(e["children"].as_array().cloned().unwrap_or_default());
            continue;
        }
        let Some(path) = e["path"].as_str() else {
            continue;
        };
        // Text files only — `read_to_string` fails on binary content, and
        // big files cap at 256 KiB.
        let meta = std::fs::metadata(path).ok();
        if meta.is_none_or(|m| m.len() > 256 * 1024) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let rel = std::path::Path::new(path)
            .strip_prefix(&root)
            .unwrap_or(std::path::Path::new(path))
            .display()
            .to_string();
        for (ix, line) in text.lines().enumerate() {
            if rows.len() >= 200 {
                break;
            }
            if line.to_lowercase().contains(&needle) {
                let snippet: String = line.trim().chars().take(60).collect();
                rows.push(json!({
                    "name": format!("{rel}:{} · {snippet}", ix + 1),
                    "path": path,
                    "dir": false,
                    "children": [],
                    "tip": format!("{rel}:{}", ix + 1),
                }));
            }
        }
    }
    view.update_state(
        &parse_path("studio.project_items").unwrap(),
        Json::Array(rows),
        cx,
    );
}

/// #148. Point the sidebar's `forge.Files` at `dir` and bust its scan cache.
/// Re-roots the tree and bumps `studio.files_nonce`, then emits
/// `studio.project_items` — the project hierarchy at the root, a plain
/// scan elsewhere.
fn refresh_files(view: &mut ForgeView, dir: &std::path::Path, cx: &mut Context<ForgeView>) {
    view.update_state(
        &parse_path("studio.project_dir").unwrap(),
        Json::String(dir.display().to_string()),
        cx,
    );
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| dir.display().to_string());
    view.update_state(
        &parse_path("studio.project_name").unwrap(),
        Json::String(name),
        cx,
    );
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default();
    view.update_state(
        &parse_path("studio.files_nonce").unwrap(),
        Json::from(nonce),
        cx,
    );
    emit_project_items(view, dir, cx);
}

/// `studio.project_items` drives the `of:` listing — the project
/// hierarchy at the root in `project` mode, a plain filesystem scan
/// everywhere else (`files` mode browses the whole folder).
fn emit_project_items(view: &mut ForgeView, dir: &std::path::Path, cx: &mut Context<ForgeView>) {
    let state = view.state();
    let hidden = state["studio"]["files_hidden"]
        .as_bool()
        .unwrap_or_default();
    let project = state["studio"]["project"].as_str().unwrap_or_default();
    let browse = state["studio"]["files_mode"].as_str().unwrap_or_default() == "files";
    let canon = |p: &std::path::Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let items =
        if !browse && !project.is_empty() && canon(dir) == canon(std::path::Path::new(project)) {
            let src = std::fs::read_to_string(dir.join("forge.toml")).unwrap_or_default();
            project_items(dir, &src, hidden)
        } else {
            let mut budget = 400usize;
            gpui_forge_runtime::render::scan_files(dir, &[], 6, hidden, &mut budget)
        };
    view.update_state(
        &parse_path("studio.project_items").unwrap(),
        Json::Array(items),
        cx,
    );
}

/// `on change: source_changed()` — re-check the source on every edit and keep the
/// preview showing the last valid document.
fn reload(view: &mut ForgeView, shared: &SharedState, cx: &mut Context<ForgeView>) {
    view.update_state(&parse_path("studio.dirty").unwrap(), Json::Bool(true), cx);
    let src = source(view);
    let catalogs = shared.borrow().catalogs.clone();
    let current_base = get_path(view.state(), &parse_path("studio.doc_base").unwrap())
        .and_then(Json::as_str)
        .unwrap_or(BASE_ID)
        .to_owned();
    let (doc, diagnostics) = check::check(&src, &doc_base(&src, &current_base), &catalogs);
    view.update_state(
        &parse_path("studio.diagnostics").unwrap(),
        Json::String(problems(&diagnostics)),
        cx,
    );
    view.update_state(
        &parse_path("studio.problem_variant").unwrap(),
        Json::String(problem_variant(&diagnostics)),
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

/// `save_project` — the Project tab's Apply button: write `studio.toml_source`
/// back to `forge.toml` and re-run `load_project` so document/icons/assets/
/// themes changes apply immediately (the manifest watcher re-detects it too).
/// The Editor tab's form written back into the manifest text — `toml_edit`
/// patches the named keys only, so comments and untouched sections survive.
/// An empty field removes its key. `documents`/`apps` parse one entry per
/// line (`apps` takes `name = doc`, the name optional).
fn patch_manifest(src: &str, studio: &Json) -> String {
    use toml_edit::{Array, ArrayOfTables, DocumentMut, Item, Table, value};
    let mut doc = src.parse::<DocumentMut>().unwrap_or_default();
    let get = |key: &str| studio[key].as_str().unwrap_or_default().trim().to_owned();
    fn put(doc: &mut DocumentMut, sec: &str, key: &str, v: String) {
        if v.is_empty() {
            if let Some(t) = doc.get_mut(sec).and_then(Item::as_table_mut) {
                t.remove(key);
            }
        } else {
            doc[sec][key] = toml_edit::value(v);
        }
    }
    for key in ["name", "version", "id", "title", "icon", "binary"] {
        put(&mut doc, "package", key, get(&format!("proj_{key}")));
    }
    for key in ["base", "document"] {
        put(&mut doc, "forge", key, get(&format!("proj_{key}")));
    }
    // `watch` is a bool — the Switch's value is authoritative.
    let watch = studio["proj_watch"].as_bool().unwrap_or_default();
    doc["forge"]["watch"] = value(watch);
    // `[forge] documents` — one relative path per line.
    let docs_text = get("proj_documents");
    let docs: Vec<&str> = docs_text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if docs.is_empty() {
        if let Some(t) = doc.get_mut("forge").and_then(Item::as_table_mut) {
            t.remove("documents");
        }
    } else {
        let mut a = Array::new();
        for d in docs {
            a.push(d);
        }
        doc["forge"]["documents"] = value(a);
    }
    // `[[apps]]` — one `name = doc` (or bare `doc`) per line.
    let apps: Vec<(String, String)> = get("proj_apps")
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| {
            l.split_once('=')
                .map(|(n, d)| (n.trim().to_owned(), d.trim().to_owned()))
                .unwrap_or_else(|| (String::new(), l.to_owned()))
        })
        .collect();
    if apps.is_empty() {
        doc.remove("apps");
    } else {
        let mut aot = ArrayOfTables::new();
        for (name, doc_path) in apps {
            let mut t = Table::new();
            if !name.is_empty() {
                t["name"] = value(name);
            }
            t["document"] = value(doc_path);
            aot.push(t);
        }
        doc["apps"] = Item::ArrayOfTables(aot);
    }
    put(&mut doc, "assets", "dir", get("proj_assets_dir"));
    put(&mut doc, "themes", "dir", get("proj_themes_dir"));
    put(&mut doc, "icons", "set", get("proj_iconset"));
    doc.to_string()
}

fn save_project(view: &mut ForgeView, shared: &SharedState, cx: &mut Context<ForgeView>) {
    let Some(toml) = shared.borrow().project_toml.clone() else {
        status(view, "no project open", cx);
        return;
    };
    // The Source sub-tab writes its buffer. The Editor sub-tab patches the
    // file on disk from its form fields.
    let mode = get_path(view.state(), &parse_path("studio.project_mode").unwrap())
        .and_then(Json::as_u64)
        .unwrap_or_default();
    let src = if mode == 1 {
        get_path(view.state(), &parse_path("studio.toml_source").unwrap())
            .and_then(|j| j.as_str())
            .unwrap_or_default()
            .to_owned()
    } else {
        let base = std::fs::read_to_string(&toml).unwrap_or_default();
        patch_manifest(&base, &view.state()["studio"])
    };
    match std::fs::write(&toml, &src) {
        Ok(()) => {
            status(view, &format!("saved {}", toml.display()), cx);
            load_project(toml, view, shared.clone(), cx);
        }
        Err(e) => status(view, &format!("cannot save {}: {e}", toml.display()), cx),
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
                    if file_watches(view, &shared) {
                        watch_file(path.clone(), shared.clone(), cx);
                    }
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
    let src = source(view);
    let (doc, _) = check::check(&src, &doc_base(&src, BASE_ID), &shared.borrow().catalogs);
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
        echo_selected(view, cx);
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
        echo_selected(view, cx);
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
        echo_selected(view, cx);
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

/// Removes the selected node (`document.selected`) — undoable, root-excluded.
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
    echo_selected(view, cx);
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

/// The check context for an open document: its own `base:` when declared, the
/// compiled base otherwise. This is the rule `forge check` uses — a
/// foreign-base doc checks against its own catalog, not a mismatch. A source
/// that fails to parse keeps `current`, so a mid-edit keystroke does not
/// flicker the base back to the default.
fn doc_base(src: &str, current: &str) -> String {
    match yaml::from_str(src) {
        Ok(doc) => doc.base.unwrap_or_else(|| BASE_ID.to_owned()),
        Err(_) => current.to_owned(),
    }
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

/// `studio.problem_variant` — the Diagnostics panel renders an `Alert` bound
/// to this: `Success` (green) clean, `Error` (red) with any diagnostic.
fn problem_variant(diagnostics: &[Diagnostic]) -> String {
    if diagnostics.is_empty() {
        "Success".into()
    } else {
        "Error".into()
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
    echo_selected(view, cx);
}

/// Shell-side `document.selected` writes must reach the preview's own state —
/// the canvas paints its accent frame off the preview's copy. A structure
/// click otherwise selects in the panel but never highlights on canvas.
fn echo_selected(view: &mut ForgeView, cx: &mut Context<ForgeView>) {
    let path = parse_path("document.selected").unwrap();
    let value = get_path(view.state(), &path).cloned().unwrap_or(Json::Null);
    let preview = view.preview("canvas", cx);
    preview.update(cx, |p, cx| {
        if get_path(p.state(), &path) != Some(&value) {
            p.update_state(&path, value.clone(), cx);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{add_document, base_icon, doc_base, patch_manifest, project_apps, project_groups};
    use gpui_forge_runtime::base::BASE_ID;
    use serde_json::json;

    /// A foreign-base document must check against its own catalog — the
    /// ratatui showcase otherwise reports a bogus base mismatch and loses
    /// real diagnostics.
    #[test]
    fn doc_base_follows_the_document() {
        let ratatui = "forge: \"0.1\"\nbase: ratatui\nroot:\n  Text:\n  text: \"hi\"\n";
        assert_eq!(doc_base(ratatui, BASE_ID), "ratatui");
        let default = "forge: \"0.1\"\nroot:\n  Text:\n  text: \"hi\"\n";
        assert_eq!(doc_base(default, BASE_ID), BASE_ID);
        // Mid-edit garbage keeps the last-known base instead of flickering.
        assert_eq!(doc_base("root:\n  Text: [", "ratatui"), "ratatui");
    }

    /// `forge.document` leads, `[[apps]]` entries follow — the sidebar order
    /// matches `forge skeleton`'s window order. `package.name` names the
    /// primary app. `name` or the file stem names the rest.
    #[test]
    fn project_apps_lists_document_then_apps() {
        let src = "[package]\nname = \"studio\"\n\n[forge]\ndocument = \"app.forge.yaml\"\n\n[[apps]]\nname = \"console\"\ndocument = \"console.forge.yaml\"\n\n[[apps]]\ndocument = \"kiosk.forge.yaml\"\n";
        assert_eq!(
            project_apps(src),
            vec![
                ("studio".to_owned(), "app.forge.yaml".to_owned()),
                ("console".to_owned(), "console.forge.yaml".to_owned()),
                ("kiosk".to_owned(), "kiosk.forge.yaml".to_owned()),
            ]
        );
        // No `forge.document` — the first `[[apps]]` block is the entry.
        let only_apps = "[[apps]]\ndocument = \"a.forge.yaml\"\n";
        assert_eq!(
            project_apps(only_apps),
            vec![("a".to_owned(), "a.forge.yaml".to_owned())]
        );
        assert!(project_apps("[forge]\nbase = \"gpui-kit\"\n").is_empty());
    }

    #[test]
    fn patch_manifest_writes_fields_and_keeps_comments() {
        let src = "# header comment\n\n[package]\nname = \"old\"\n\n[forge]\ndocument = \"a.forge.yaml\"\n\n[[apps]]\ndocument = \"b.forge.yaml\"\n";
        let studio = json!({
            "proj_name": "new-name",
            "proj_version": "1.2.3",
            "proj_id": "", "proj_title": "", "proj_icon": "", "proj_binary": "",
            "proj_base": "ratatui",
            "proj_document": "main.forge.yaml",
            "proj_watch": true,
            "proj_documents": "docs/a.forge.yaml\ndocs/b.forge.yaml",
            "proj_apps": "console = c.forge.yaml\nk.forge.yaml",
            "proj_assets_dir": "res",
            "proj_themes_dir": "",
            "proj_iconset": "",
        });
        let out = patch_manifest(src, &studio);
        assert!(out.contains("# header comment"));
        assert!(out.contains("name = \"new-name\""));
        assert!(out.contains("version = \"1.2.3\""));
        assert!(out.contains("base = \"ratatui\""));
        assert!(out.contains("document = \"main.forge.yaml\""));
        assert!(out.contains("watch = true"));
        assert!(out.contains("docs/a.forge.yaml") && out.contains("docs/b.forge.yaml"));
        assert!(out.contains("name = \"console\"") && out.contains("k.forge.yaml"));
        assert!(out.contains("dir = \"res\""));
        // The patch replaces the old `[[apps]]` block — not appends to it.
        assert!(!out.contains("document = \"b.forge.yaml\""));
    }

    #[test]
    fn add_document_appends_and_creates_the_list() {
        // Existing list — the new path appends, old members stay.
        let src = "[forge]\ndocuments = [\"a.forge.yaml\"]\n";
        let out = add_document(src, "docs/b.forge.yaml");
        assert!(out.contains("\"a.forge.yaml\"") && out.contains("\"docs/b.forge.yaml\""));
        // No list at all — the key is created under `[forge]`.
        let out = add_document("[forge]\nbase = \"gpui-kit\"\n", "c.forge.yaml");
        assert!(out.contains("documents") && out.contains("\"c.forge.yaml\""));
    }

    /// App glyphs: the base's default unless a spec icon exists —
    /// `meta.icon` on the window doc, `package.icon` for the primary app.
    /// A spec path only wins when the file sits under the project root.
    #[test]
    fn project_groups_resolves_app_icons() {
        let dir = std::env::temp_dir().join(format!("forge-icons-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("main.forge.yaml"),
            "forge: \"0.1\"\nbase: ratatui\nmeta:\n  icon: app.svg\nroot:\n  Text:\n  text: hi\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("console.forge.yaml"),
            "forge: \"0.1\"\nroot:\n  Text:\n  text: c\n",
        )
        .unwrap();
        std::fs::write(dir.join("app.svg"), "<svg/>").unwrap();
        // `meta.icon` (doc-relative) beats the ratatui default.
        let src = "[package]\nname = \"demo\"\n\n[forge]\ndocument = \"main.forge.yaml\"\n\n[[apps]]\ndocument = \"console.forge.yaml\"\n";
        let (apps, _, _) = project_groups(&dir, src);
        assert_eq!(apps[0].icon, "app.svg");
        // No spec icon — the manifest's `[forge] base` picks the glyph.
        assert_eq!(apps[1].icon, base_icon("gpui-kit"));
        // `package.icon` fills the primary app's gap when `meta.icon` is absent.
        let src = "[package]\nname = \"demo\"\nicon = \"pkg.png\"\n\n[forge]\ndocument = \"console.forge.yaml\"\n";
        std::fs::write(dir.join("pkg.png"), "png").unwrap();
        let (apps, _, _) = project_groups(&dir, src);
        assert_eq!(apps[0].icon, "pkg.png");
        // A spec icon outside the project can't resolve — base glyph wins.
        let src = "[package]\nname = \"demo\"\nicon = \"../outside.png\"\n\n[forge]\ndocument = \"console.forge.yaml\"\n";
        let (apps, _, _) = project_groups(&dir, src);
        assert_eq!(apps[0].icon, base_icon("gpui-kit"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn project_groups_claims_members_and_leaves_strays() {
        let dir = std::env::temp_dir().join(format!("forge-proj-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("docs")).unwrap();
        std::fs::write(
            dir.join("main.forge.yaml"),
            "forge: \"0.1\"\nmeta:\n  title: Main\nroot:\n  Text:\n  text: hi\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("docs/dialog.forge.yaml"),
            "forge: \"0.1\"\nmeta:\n  window: ../main.forge.yaml\nroot:\n  Text:\n  text: d\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("stray.forge.yaml"),
            "forge: \"0.1\"\nroot:\n  Text:\n  text: s\n",
        )
        .unwrap();
        let src = "[package]\nname = \"demo\"\n\n[forge]\ndocument = \"main.forge.yaml\"\n";
        let (apps, unassigned, links) = project_groups(&dir, src);
        let base = |v: &[std::path::PathBuf]| -> Vec<String> {
            v.iter()
                .map(|p| {
                    p.strip_prefix(&dir)
                        .unwrap_or(p)
                        .display()
                        .to_string()
                        .replace('\\', "/")
                })
                .collect()
        };
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "demo");
        assert_eq!(
            base(&apps[0].docs),
            vec!["main.forge.yaml", "docs/dialog.forge.yaml"]
        );
        assert_eq!(base(&unassigned), vec!["stray.forge.yaml"]);
        // `links` names the declarer behind each shown doc — the manifest
        // for the app's entry, the window target for `meta.window` members.
        let rel = |p: &std::path::Path| {
            p.strip_prefix(&dir)
                .unwrap_or(p)
                .display()
                .to_string()
                .replace('\\', "/")
        };
        assert_eq!(
            rel(links
                .get(&dir.join("main.forge.yaml").canonicalize().unwrap())
                .unwrap()),
            "forge.toml"
        );
        assert_eq!(
            rel(links
                .get(&dir.join("docs/dialog.forge.yaml").canonicalize().unwrap())
                .unwrap()),
            "main.forge.yaml"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
