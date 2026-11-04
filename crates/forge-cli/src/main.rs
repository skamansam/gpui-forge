//! `forge` — the Forge toolchain.
//!
//! `forge check doc.forge.yaml`  — parse + validate a document
//! `forge build doc.forge.yaml`  — scaffold a native app crate and compile it

use std::path::{Path, PathBuf};
use std::process::Command;

/// `forge.toml` — a Forge project manifest.
#[allow(dead_code)]
#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct ForgeToml {
    package: Package,
    forge: ForgeSection,
    hooks: Hooks,
    assets: Assets,
    libraries: std::collections::HashMap<String, String>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct Package {
    name: String,
    version: String,
    /// Application/bundle id, e.g. `dev.example.my-app`.
    id: String,
    icon: String,
    title: String,
    /// Package an existing workspace binary instead of a generated app
    /// (e.g. `binary = "forge-studio"` builds `-p forge-studio`).
    binary: String,
}

#[allow(dead_code)]
#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct ForgeSection {
    base: String,
    /// Entry document, relative to the manifest.
    document: String,
}

#[allow(dead_code)]
#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct Hooks {
    /// Path to a Rust crate that registers hook handlers.
    krate: String,
}

#[allow(dead_code)]
#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct Assets {
    dir: String,
}

fn load_manifest(start: &Path) -> Option<(PathBuf, ForgeToml)> {
    let mut dir = Some(start);
    while let Some(d) = dir {
        let candidate = d.join("forge.toml");
        if candidate.exists() {
            match std::fs::read_to_string(&candidate)
                .ok()
                .map(|s| toml::from_str::<ForgeToml>(&s))
            {
                Some(Ok(m)) => return Some((d.to_path_buf(), m)),
                Some(Err(e)) => eprintln!("forge: {}: {e}", candidate.display()),
                None => {}
            }
            return None;
        }
        dir = d.parent();
    }
    None
}

const APP_MAIN: &str = r#"use gpui_forge_runtime::render::ForgeView;
use gpui_kit::prelude::*;
use gpui_kit::*;

mod themes;

const SRC: &str = include_str!("../app.forge.yaml");

fn main() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::AllAssets)
        .run(|cx| {
            gpui_kit::init(cx);
            for (_, json) in themes::THEMES {
                let _ = gpui_kit::component::theme::ThemeRegistry::global_mut(cx)
                    .load_themes_from_str(json);
            }
            let catalogs = gpui_forge_runtime::check::catalogs();
            let (doc, diagnostics) = gpui_forge_runtime::check::check(
                SRC, gpui_forge_runtime::base::BASE_ID, &catalogs);
            for d in &diagnostics {
                eprintln!("app.forge.yaml:{}:{}: {}", d.pos.line, d.pos.col, d.message);
            }
            let bounds = Bounds::centered(None, size(px(1280.0), px(800.0)), cx);
            let options = WindowOptions {
                app_id: Some("{APP_ID}".into()),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("{TITLE}".into()),
                    appears_transparent: true,
                    traffic_light_position: None,
                }),
                app_owns_titlebar_drag: true,
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, move |_, cx| {
                use gpui_kit::WindowAppearance;
                use gpui_kit::component::theme::{Theme, ThemeMode};
                Theme::change(
                    match cx.window_appearance() {
                        WindowAppearance::Dark | WindowAppearance::VibrantDark => ThemeMode::Dark,
                        _ => ThemeMode::Light,
                    },
                    None,
                    cx,
                );
                let catalogs = catalogs.clone();
                let doc = doc.clone();
                cx.new(move |cx| {
                    let mut view = ForgeView::new(catalogs);
                    if let Some(doc) = doc {
                        view.set_document(doc, cx);
                    }
                    view
                })
            })
            .expect("failed to open window");
        });
}
"#;

const APP_TOML: &str = r#"[package]
name = "{NAME}"
version = "0.1.0"
edition = "2024"

[dependencies]
{LIBS}
forge-spec = { path = "{ROOT}/crates/forge-spec" }
gpui-forge-runtime = { path = "{ROOT}/crates/gpui-forge-runtime", features = ["base-gpui-kit"] }
gpui-kit = "=0.8.1"
serde_json = "1"

# A generated app sits inside the gpui-forge checkout but is its own workspace.
[workspace]
"#;

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(cmd) = args.next() else {
        usage();
        std::process::exit(2);
    };
    let rest: Vec<String> = args.collect();
    let code = match cmd.as_str() {
        "new" => new_cmd(&rest),
        "package" => package_cmd(&rest),
        "hooks" => hooks_cmd(&rest),
        "check" | "validate" => check_cmd(&rest),
        "schema" => {
            print!("{}", gpui_forge_runtime::forge_spec::json::schema());
            0
        }
        "convert" => convert_cmd(&rest),
        "fmt" => fmt_cmd(&rest),
        "build" => build_cmd(&rest),
        "preview" => preview_cmd(&rest),
        _ => {
            usage();
            2
        }
    };
    std::process::exit(code);
}

fn usage() {
    eprintln!("usage:");
    eprintln!("  forge new <name>                      scaffold a project (forge.toml + doc)");
    eprintln!("  forge check <doc.forge.yaml>            parse + validate");
    eprintln!("  forge build <doc.forge.yaml> [opts]     compile a native app");
    eprintln!("      -o <path>       output binary path (default: ./<name>)");
    eprintln!("      --release       release profile (default: dev)");
    eprintln!("      --codegen       compile the document to Rust, not embedded YAML");
    eprintln!("      --dir <path>    scaffold dir (default: <doc>/.forge-build/<name>)");
    eprintln!("  forge fmt <doc.forge.yaml>              canonical reformat (stdout or -w)");
    eprintln!("  forge convert <file>                  between .forge.yaml and JSON");
    eprintln!("      --from yaml|json  input format (else detected by extension/content)");
    eprintln!("      --to yaml|json    output format (default: the other one)");
    eprintln!("      -o <path>         output file (default: stdout; `-` reads stdin)");
    eprintln!("  forge schema                          JSON Schema for the document model");
    eprintln!("  forge hooks <doc>                     hook manifest (JSON) the doc requires");
    eprintln!("  forge package                         build --release + bundle for this OS");
    eprintln!("  forge preview <doc.forge.yaml>        build + launch the app");
}

/// The gpui-forge workspace root — this crate sits at `<root>/crates/forge-cli`.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

fn doc_path(rest: &[String]) -> Option<PathBuf> {
    let file = rest.iter().find(|a| !a.starts_with('-'))?;
    let path = PathBuf::from(file);
    path.exists().then_some(path)
}

fn opt_value<'a>(rest: &'a [String], flag: &str) -> Option<&'a str> {
    rest.iter()
        .position(|a| a == flag)
        .and_then(|i| rest.get(i + 1))
        .map(String::as_str)
}

/// Scaffold a project: forge.toml + app.forge.yaml in ./<name>.
fn new_cmd(rest: &[String]) -> i32 {
    let Some(name) = rest.iter().find(|a| !a.starts_with('-')).cloned() else {
        usage();
        return 2;
    };
    let dir = PathBuf::from(&name);
    if dir.exists() {
        eprintln!("forge: {} already exists", dir.display());
        return 1;
    }
    let _ = std::fs::create_dir_all(&dir);
    write(
        &dir.join("forge.toml"),
        &format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
id = "dev.forge.{name}"

[forge]
base = "gpui-kit"
document = "app.forge.yaml"

# [assets]
# dir = "assets"

# [libraries]
# ely-gpui-components = "^0.4"
"#
        ),
    );
    write(
        &dir.join("app.forge.yaml"),
        r#"forge: "0.1"
base: gpui-kit

root:
  VStack:
  style:
    padding: 16px
    gap: 8px
  children:
    - Text: Hello, Forge!
"#,
    );
    println!(
        "forge: created {} — `forge build` inside it compiles a native app",
        dir.display()
    );
    0
}

fn check_cmd(rest: &[String]) -> i32 {
    let Some(path) = doc_path(rest) else {
        usage();
        return 2;
    };
    let src = match std::fs::read_to_string(&path) {
        Ok(src) => src,
        Err(e) => {
            eprintln!("forge: cannot read {}: {e}", path.display());
            return 1;
        }
    };
    let catalogs = gpui_forge_runtime::check::catalogs();
    let (doc, diagnostics) =
        gpui_forge_runtime::check::check(&src, gpui_forge_runtime::base::BASE_ID, &catalogs);
    let name = path.display();
    for d in &diagnostics {
        eprintln!("{name}:{}:{}: {}", d.pos.line, d.pos.col, d.message);
    }
    if doc.is_none() {
        eprintln!("forge: {name}: not a valid Forge document");
        return 1;
    }
    if diagnostics.is_empty() {
        println!("{name}: ok");
        0
    } else {
        1
    }
}

/// `forge hooks <doc>` — the hook manifest: every `on <event>` binding in the
/// document, grouped by handler name with its events and argument shapes.
/// Emitted as JSON so a hooks crate knows what to implement.
fn hooks_cmd(rest: &[String]) -> i32 {
    let Some(path) = doc_path(rest) else {
        usage();
        return 2;
    };
    let src = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("forge: cannot read {}: {e}", path.display());
            return 1;
        }
    };
    let catalogs = gpui_forge_runtime::check::catalogs();
    let (doc, _) =
        gpui_forge_runtime::check::check(&src, gpui_forge_runtime::base::BASE_ID, &catalogs);
    let Some(doc) = doc else {
        eprintln!("forge: {} does not parse", path.display());
        return 1;
    };
    use gpui_forge_runtime::forge_spec::Node;
    // handler -> sorted (event -> count); keep signature args of last use
    let mut map: std::collections::BTreeMap<String, (Vec<String>, Vec<String>)> =
        std::collections::BTreeMap::new();
    fn walk<'a>(
        n: &'a Node,
        map: &mut std::collections::BTreeMap<String, (Vec<String>, Vec<String>)>,
    ) {
        for e in &n.events {
            let entry = map.entry(e.handler.name.clone()).or_default();
            if !entry.0.contains(&e.event) {
                entry.0.push(e.event.clone());
            }
            entry.1 = e.handler.args.iter().map(|v| format!("{v:?}")).collect();
        }
        for c in &n.children {
            walk(c, map);
        }
    }
    for d in &doc.dialogs {
        walk(d, &mut map);
    }
    walk(&doc.root, &mut map);
    let hooks: Vec<serde_json::Value> = map
        .into_iter()
        .map(|(name, (mut events, args))| {
            events.sort();
            serde_json::json!({ "name": name, "events": events, "args": args })
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "document": path.display().to_string(),
            "hooks": hooks,
        }))
        .unwrap_or_default()
    );
    0
}

/// `.forge.yaml` <-> JSON. Format is detected by extension, then content;
/// `--from`/`--to` override. `-` as the input reads stdin.
fn convert_cmd(rest: &[String]) -> i32 {
    let file = rest
        .iter()
        .find(|a| !a.starts_with('-') && *a != "--from" && *a != "--to" && *a != "-o");
    let src = if file.map(|f| f.as_str()) == Some("-") {
        use std::io::Read;
        let mut buf = String::new();
        if std::io::stdin().read_to_string(&mut buf).is_err() {
            eprintln!("forge: cannot read stdin");
            return 1;
        }
        buf
    } else {
        let Some(file) = file else {
            usage();
            return 2;
        };
        match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("forge: cannot read {file}: {e}");
                return 1;
            }
        }
    };
    let ext_json = file
        .and_then(|f| Path::new(f).extension())
        .is_some_and(|e| e == "json");
    let mut from = opt_value(rest, "--from").unwrap_or_else(|| {
        if ext_json || src.trim_start().starts_with('{') {
            "json"
        } else {
            "yaml"
        }
    });
    if from == "forge" {
        from = "yaml";
    }
    let to = opt_value(rest, "--to").unwrap_or(if from == "json" { "yaml" } else { "json" });
    let out = match (from, to) {
        ("yaml", "yaml") | ("json", "json") => src.clone(),
        ("yaml", "json") => {
            match gpui_forge_runtime::forge_spec::yaml::from_str_with_source_map(&src) {
                Ok((doc, _)) => gpui_forge_runtime::forge_spec::json::to_string(&doc),
                Err(e) => {
                    eprintln!("forge: YAML parse failed: {e:?}");
                    return 1;
                }
            }
        }
        ("json", "yaml") => match gpui_forge_runtime::forge_spec::json::from_str(&src) {
            Ok(doc) => gpui_forge_runtime::forge_spec::yaml::to_string(
                &doc,
                &gpui_forge_runtime::forge_spec::yaml::WriteOptions::default(),
            ),
            Err(e) => {
                eprintln!("forge: JSON parse failed: {e}");
                return 1;
            }
        },
        (f, t) => {
            eprintln!("forge: unknown format conversion {f} -> {t} (supported: yaml, json)");
            return 2;
        }
    };
    if let Some(path) = opt_value(rest, "-o") {
        if let Err(e) = std::fs::write(path, &out) {
            eprintln!("forge: cannot write {path}: {e}");
            return 1;
        }
        println!("forge: wrote {path}");
    } else {
        print!("{out}");
    }
    0
}

/// `forge package` — release-build, then wrap for the host OS: an
/// AppDir (+AppImage when appimagetool is installed, else a tarball) on
/// Linux; .app/.nsi scaffolding is emitted for other platforms.
fn package_cmd(rest: &[String]) -> i32 {
    let root = workspace_root()
        .canonicalize()
        .unwrap_or_else(|_| workspace_root());
    // Binary mode: the manifest names a workspace crate (e.g. forge-studio) —
    // release-build it directly instead of scaffolding a generated app.
    let dir = doc_path(rest)
        .map(|p| {
            if p.is_dir() {
                p
            } else {
                p.parent().unwrap_or(Path::new(".")).to_path_buf()
            }
        })
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    if let Some((m_dir, manifest)) =
        load_manifest(&dir).filter(|(_, m)| !m.package.binary.is_empty())
    {
        let name = manifest.package.name.clone();
        eprintln!("forge: building {} (release)", manifest.package.binary);
        let ok = Command::new("cargo")
            .args([
                "build",
                "--release",
                "--locked",
                "-p",
                &manifest.package.binary,
            ])
            .current_dir(&root)
            .status()
            .is_ok_and(|s| s.success());
        if !ok {
            return 1;
        }
        let binary = root.join("target/release").join(&manifest.package.binary);
        return bundle(&m_dir, &manifest, &name, &binary);
    }
    let mut build_args: Vec<String> = rest.to_vec();
    build_args.push("--release".into());
    if build_cmd(&build_args) != 0 {
        return 1;
    }
    let Some((m_dir, manifest, _doc, name)) = project(rest) else {
        return 2;
    };
    let binary = root.join("target/forge-apps/release").join(&name);
    if !binary.exists() {
        eprintln!("forge: {} not found after build", binary.display());
        return 1;
    }
    bundle(&m_dir, &manifest, &name, &binary)
}

/// Wraps a built binary for the host OS into `<manifest-dir>/dist/`.
fn bundle(m_dir: &Path, manifest: &ForgeToml, name: &str, binary: &Path) -> i32 {
    let title = if manifest.package.title.is_empty() {
        name.replace('-', " ")
    } else {
        manifest.package.title.clone()
    };
    let out_dir = m_dir.to_path_buf().join("dist");
    let _ = std::fs::remove_dir_all(&out_dir);
    let _ = std::fs::create_dir_all(&out_dir);

    if std::env::consts::OS == "linux" {
        let appdir = out_dir.join(format!("{name}.AppDir"));
        let _ = std::fs::create_dir_all(appdir.join("usr/bin"));
        let _ = std::fs::copy(binary, appdir.join("usr/bin").join(name));
        write(
            &appdir.join("AppRun"),
            &format!(
                r#"#!/bin/sh
exec "$(dirname "$0")/usr/bin/{name}" "$@"
"#
            ),
        );
        let _ = Command::new("chmod")
            .args(["+x", &appdir.join("AppRun").to_string_lossy()])
            .status();
        let mut desktop = format!(
            "[Desktop Entry]\nType=Application\nName={title}\nExec={name}\nCategories=Development;\nTerminal=false\n"
        );
        if !manifest.package.icon.is_empty() {
            let icon = m_dir.join(&manifest.package.icon);
            if let Some(base) = icon.file_name().map(|n| n.to_string_lossy().into_owned()) {
                if icon.exists() {
                    let _ = std::fs::copy(&icon, appdir.join(&base));
                    // Icon= wants the file's name without its extension.
                    let name = base
                        .rsplit_once('.')
                        .map(|(s, _)| s.to_string())
                        .unwrap_or(base);
                    desktop.push_str(&format!("Icon={name}\n"));
                }
            }
        }
        write(&appdir.join(format!("{name}.desktop")), &desktop);
        if Command::new("appimagetool")
            .arg(&appdir)
            .arg(out_dir.join(format!("{name}-{}.AppImage", std::env::consts::ARCH)))
            .status()
            .is_ok()
        {
            println!("forge: packaged {}", out_dir.display());
            return 0;
        }
        let tar = out_dir.join(format!("{name}.tar.gz"));
        let ok = Command::new("tar")
            .arg("-czf")
            .arg(&tar)
            .arg("-C")
            .arg(&out_dir)
            .arg(appdir.file_name().unwrap())
            .status()
            .is_ok_and(|s| s.success());
        println!(
            "forge: appimagetool not found — packed AppDir to {}",
            tar.display()
        );
        return if ok { 0 } else { 1 };
    }
    if std::env::consts::OS == "macos" {
        let app = out_dir.join(format!("{name}.app/Contents"));
        let _ = std::fs::create_dir_all(app.join("MacOS"));
        let _ = std::fs::copy(&binary, app.join("MacOS").join(&name));
        let id = if manifest.package.id.is_empty() {
            format!("dev.forge.{name}")
        } else {
            manifest.package.id.clone()
        };
        write(
            &app.join("Info.plist"),
            &format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
  <key>CFBundleExecutable</key><string>{name}</string>
  <key>CFBundleIdentifier</key><string>{id}</string>
  <key>CFBundleName</key><string>{title}</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>{}</string>
</dict></plist>"#,
                manifest.package.version
            ),
        );
        println!("forge: packaged {}", app.parent().unwrap().display());
        return 0;
    }
    if std::env::consts::OS == "windows" {
        let zip = out_dir.join(format!("{name}.zip"));
        write(
            &out_dir.join(format!("{name}.nsi")),
            &format!(
                r#"Name "{title}"
OutFile "{name}-setup.exe"
InstallDir "$PROGRAMFILES\{title}"
Section
  SetOutPath $INSTDIR
  File "{}"
  CreateShortcut "$DESKTOP\{title}.lnk" "$INSTDIR\{name}.exe"
SectionEnd
"#,
                binary.display()
            ),
        );
        let _ = Command::new("zip")
            .arg("-j")
            .arg(&zip)
            .arg(&binary)
            .status();
        println!("forge: binary zip + {name}.nsi in {}", out_dir.display());
        return 0;
    }
    eprintln!("forge: no packager for {}", std::env::consts::OS);
    1
}

/// Canonical reformat — the explicit formatter (not the preserving writer,
/// which is for faithful writes after edits).
fn fmt_cmd(rest: &[String]) -> i32 {
    let Some(path) = doc_path(rest) else {
        usage();
        return 2;
    };
    let src = match std::fs::read_to_string(&path) {
        Ok(src) => src,
        Err(e) => {
            eprintln!("forge: cannot read {}: {e}", path.display());
            return 1;
        }
    };
    let Ok((doc, _)) = gpui_forge_runtime::forge_spec::yaml::from_str_with_source_map(&src) else {
        eprintln!("forge: {}: not a valid Forge document", path.display());
        return 1;
    };
    let out = gpui_forge_runtime::forge_spec::yaml::to_string(
        &doc,
        &gpui_forge_runtime::forge_spec::yaml::WriteOptions::default(),
    );
    if rest.iter().any(|a| a == "-w" || a == "--write") {
        if let Err(e) = std::fs::write(&path, &out) {
            eprintln!("forge: cannot write {}: {e}", path.display());
            return 1;
        }
        println!("forge: formatted {}", path.display());
    } else {
        print!("{out}");
    }
    0
}

/// Build then launch — a compiled preview of the document.
fn preview_cmd(rest: &[String]) -> i32 {
    let code = build_cmd(rest);
    if code != 0 {
        return code;
    }
    let Some(path) = doc_path(rest) else {
        return 2;
    };
    let path = path.canonicalize().unwrap_or(path);
    let name = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .replace('.', "-");
    let out = opt_value(rest, "-o")
        .map(PathBuf::from)
        .unwrap_or_else(|| path.parent().unwrap_or(Path::new(".")).join(&name));
    match Command::new(&out).spawn() {
        Ok(_) => {
            println!("forge: launched {}", out.display());
            0
        }
        Err(e) => {
            eprintln!("forge: cannot launch {}: {e}", out.display());
            1
        }
    }
}

/// Emits `src/doc.rs`: the document as compiled Rust instead of a runtime
/// YAML parse — the "codegen" alternative to embed mode.
fn codegen_doc(doc: &gpui_forge_runtime::forge_spec::Document) -> String {
    use gpui_forge_runtime::forge_spec::*;
    fn path(p: &Path) -> String {
        let segs =
            p.0.iter()
                .map(|s| match s {
                    PathSegment::Field(f) => format!("PathSegment::Field({f:?}.into())"),
                    PathSegment::Index(i) => format!("PathSegment::Index({i})"),
                })
                .collect::<Vec<_>>()
                .join(", ");
        format!("forge_spec::Path(vec![{segs}])")
    }
    fn value(v: &Value) -> String {
        match v {
            Value::String(t) => format!("Value::String({t:?}.into())"),
            Value::Number { value: n, unit } => {
                let u = unit
                    .map(|u| format!("Some(Unit::{u:?})"))
                    .unwrap_or("None".into());
                format!("Value::Number {{ value: {n:?}, unit: {u} }}")
            }
            Value::Bool(b) => format!("Value::Bool({b})"),
            Value::Enum(t) => format!("Value::Enum({t:?}.into())"),
            Value::Color(c) => format!(
                "Value::Color(Color {{ r: {}, g: {}, b: {}, a: {} }})",
                c.r, c.g, c.b, c.a
            ),
            Value::Token(p) => format!("Value::Token({})", path(p)),
            Value::List(items) => {
                format!(
                    "Value::List(vec![{}])",
                    items.iter().map(value).collect::<Vec<_>>().join(", ")
                )
            }
            Value::Map(entries) => {
                format!(
                    "Value::Map(vec![{}])",
                    entries
                        .iter()
                        .map(|(k, v)| format!("({k:?}.into(), {})", value(v)))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            Value::Binding { path: p, mode } => {
                format!(
                    "Value::Binding {{ path: {}, mode: BindingMode::{mode:?} }}",
                    path(p)
                )
            }
            Value::Interpolated(segs) => {
                format!(
                    "Value::Interpolated(vec![{}])",
                    segs.iter()
                        .map(|s| match s {
                            Segment::Text(t) => format!("Segment::Text({t:?}.into())"),
                            Segment::Binding(p) => format!("Segment::Binding({})", path(p)),
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
    }
    fn entries(es: &Entries) -> String {
        format!(
            "vec![{}]",
            es.iter()
                .map(|(k, v)| format!("({k:?}.into(), {})", value(v)))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
    fn node(n: &Node) -> String {
        let ns = n
            .element
            .namespace
            .as_ref()
            .map(|s| format!("Some({s:?}.into())"))
            .unwrap_or("None".into());
        let id =
            n.id.as_ref()
                .map(|s| format!("Some({s:?}.into())"))
                .unwrap_or("None".into());
        let content = n
            .content
            .as_ref()
            .map(|v| format!("Some({})", value(v)))
            .unwrap_or("None".into());
        let events = n.events.iter().map(|e| format!(
            "EventBinding {{ event: {:?}.into(), handler: HandlerRef {{ name: {:?}.into(), args: vec![{}] }} }}",
            e.event, e.handler.name, e.handler.args.iter().map(value).collect::<Vec<_>>().join(", ")
        )).collect::<Vec<_>>().join(", ");
        let kids = n.children.iter().map(node).collect::<Vec<_>>().join(", ");
        format!(
            "Node {{ element: ElementRef {{ namespace: {ns}, name: {:?}.into() }}, id: {id}, content: {content}, props: {}, style: {}, events: vec![{events}], children: vec![{kids}] }}",
            n.element.name,
            entries(&n.props),
            entries(&n.style)
        )
    }
    let dialogs = doc.dialogs.iter().map(node).collect::<Vec<_>>().join(", ");
    let schema = doc
        .schema
        .as_ref()
        .map(|s| format!("Some({s:?}.into())"))
        .unwrap_or("None".into());
    let base = doc
        .base
        .as_ref()
        .map(|s| format!("Some({s:?}.into())"))
        .unwrap_or("None".into());
    let imports = doc
        .imports
        .iter()
        .map(|i| {
            format!(
                "Import {{ alias: {:?}.into(), library: {:?}.into(), version: {:?} }}",
                i.alias, i.library, i.version
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let meta = doc
        .meta
        .iter()
        .map(|(k, v)| format!("({k:?}.into(), {v:?}.into())"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "use forge_spec::*;

pub fn document() -> Document {{
    Document {{
        schema: {schema},
        version: {:?}.into(),
        base: {base},
        imports: vec![{imports}],
        meta: vec![{meta}],
        dialogs: vec![{dialogs}],
        root: {},
        raw: None,
    }}
}}
",
        doc.version,
        node(&doc.root)
    )
}

/// Resolves the project: a forge.toml on the arg path or up from cwd, plus
/// the entry document and package name.
fn project(rest: &[String]) -> Option<(PathBuf, ForgeToml, PathBuf, String)> {
    let manifest_path = doc_path(rest)
        .filter(|p| p.is_dir() || p.file_name() == Some(std::ffi::OsStr::new("forge.toml")));
    let manifest = manifest_path
        .as_deref()
        .map(|p| {
            if p.is_dir() {
                p.to_path_buf()
            } else {
                p.parent().unwrap_or(Path::new(".")).to_path_buf()
            }
        })
        .map(|d| load_manifest(&d))
        .unwrap_or_else(|| load_manifest(&std::env::current_dir().unwrap_or_default()));
    let (manifest_dir, manifest) =
        manifest.unwrap_or_else(|| (PathBuf::new(), ForgeToml::default()));
    let path = doc_path(rest)
        .filter(|p| p.file_name() != Some(std::ffi::OsStr::new("forge.toml")))
        .or_else(|| manifest_dir.join(&manifest.forge.document).into());
    let path = path.filter(|p| p.exists())?;
    let path = path.canonicalize().unwrap_or(path);
    let name = if manifest.package.name.is_empty() {
        path.file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .replace('.', "-")
    } else {
        manifest.package.name.clone()
    };
    Some((manifest_dir, manifest, path, name))
}

fn build_cmd(rest: &[String]) -> i32 {
    let release = rest.iter().any(|a| a == "--release");
    let Some((_m_dir, manifest, path, name)) = project(rest) else {
        usage();
        return 2;
    };
    let dir = opt_value(rest, "--dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            path.parent()
                .unwrap_or(Path::new("."))
                .join(".forge-build")
                .join(&name)
        });
    let out = opt_value(rest, "-o")
        .map(PathBuf::from)
        .unwrap_or_else(|| path.parent().unwrap_or(Path::new(".")).join(&name));

    // Scaffold a minimal app crate: Cargo.toml with path deps back into this
    // workspace, a generated main.rs and the document itself (embedded).
    let src_dir = dir.join("src");
    if let Err(e) = std::fs::create_dir_all(&src_dir) {
        eprintln!("forge: cannot create {}: {e}", dir.display());
        return 1;
    }
    let root = workspace_root()
        .canonicalize()
        .unwrap_or_else(|_| workspace_root());
    let title = if manifest.package.title.is_empty() {
        name.replace('-', " ")
    } else {
        manifest.package.title.clone()
    };
    let app_id = if manifest.package.id.is_empty() {
        name.clone()
    } else {
        manifest.package.id.clone()
    };
    let libs = manifest
        .libraries
        .iter()
        .map(|(n, v)| format!("{n} = \"{v}\""))
        .collect::<Vec<_>>()
        .join("\n");
    write(
        &dir.join("Cargo.toml"),
        &APP_TOML
            .replace("{NAME}", &name)
            .replace("{ROOT}", &root.display().to_string())
            .replace("{LIBS}", &libs),
    );
    write(
        &src_dir.join("main.rs"),
        &APP_MAIN
            .replace("{APP_ID}", &app_id)
            .replace("{TITLE}", &title),
    );
    write(
        &dir.join("app.forge.yaml"),
        &std::fs::read_to_string(&path).unwrap_or_default(),
    );
    // Bundle every shipped theme — assets live in the binary, not on disk.
    let themes_in = root.join("themes");
    let themes_out = dir.join("themes");
    let _ = std::fs::create_dir_all(&themes_out);
    let mut theme_list = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&themes_in) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.ends_with(".json") && std::fs::copy(e.path(), themes_out.join(&name)).is_ok() {
                theme_list.push(name);
            }
        }
    }
    let themes_rs = format!(
        "pub const THEMES: &[(&str, &str)] = &[\n{}\n];\n",
        theme_list
            .iter()
            .map(|n| format!(
                "    ({:?}, include_str!(\"../themes/{}\")),",
                n.trim_end_matches(".json"),
                n
            ))
            .collect::<Vec<_>>()
            .join("\n")
    );
    write(&src_dir.join("themes.rs"), &themes_rs);
    if rest.iter().any(|a| a == "--codegen") {
        let src = std::fs::read_to_string(&path).unwrap_or_default();
        if let Ok((doc, _)) = gpui_forge_runtime::forge_spec::yaml::from_str_with_source_map(&src) {
            write(&src_dir.join("doc.rs"), &codegen_doc(&doc));
            let main = std::fs::read_to_string(src_dir.join("main.rs"))
                .unwrap_or_default()
                .replace(
                    r#"const SRC: &str = include_str!("../app.forge.yaml");"#,
                    "mod doc;",
                )
                .replace(
                    r#"let (doc, diagnostics) = gpui_forge_runtime::check::check(
                SRC, gpui_forge_runtime::base::BASE_ID, &catalogs);"#,
                    r#"let (doc, diagnostics): (
                Option<gpui_forge_runtime::forge_spec::Document>,
                Vec<gpui_forge_runtime::forge_spec::Diagnostic>,
            ) = (Some(doc::document()), Vec::new());"#,
                );
            write(&src_dir.join("main.rs"), &main);
        }
    }

    let profile = if release { "release" } else { "debug" };
    eprintln!("forge: building {name} ({profile}) in {}", dir.display());
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("--manifest-path")
        .arg(dir.join("Cargo.toml"))
        // Reuse the workspace's already-compiled dependency graph.
        .env("CARGO_TARGET_DIR", root.join("target/forge-apps"));
    if release {
        cmd.arg("--release");
    }
    let status = match cmd.status() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("forge: cannot run cargo: {e}");
            return 1;
        }
    };
    if !status.success() {
        eprintln!("forge: cargo build failed");
        return 1;
    }
    let binary = root.join("target/forge-apps").join(profile).join(&name);
    if let Err(e) = std::fs::copy(&binary, &out) {
        eprintln!(
            "forge: cannot copy {} -> {}: {e}",
            binary.display(),
            out.display()
        );
        return 1;
    }
    println!("forge: built {}", out.display());
    0
}

fn write(path: &Path, contents: &str) {
    if let Err(e) = std::fs::write(path, contents) {
        eprintln!("forge: cannot write {}: {e}", path.display());
        std::process::exit(1);
    }
}
