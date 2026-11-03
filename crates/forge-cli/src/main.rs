//! `forge` — the Forge toolchain.
//!
//! `forge check doc.forge.yaml`  — parse + validate a document
//! `forge build doc.forge.yaml`  — scaffold a native app crate and compile it

use std::path::{Path, PathBuf};
use std::process::Command;

const APP_MAIN: &str = r#"use gpui_forge_runtime::base::BASE_ID;
use gpui_forge_runtime::check;
use gpui_forge_runtime::render::ForgeView;
use gpui_kit::prelude::*;
use gpui_kit::*;

const SRC: &str = include_str!("../app.forge.yaml");

fn main() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::AllAssets)
        .run(|cx| {
            gpui_kit::init(cx);
            let catalogs = check::catalogs();
            let (doc, diagnostics) = check::check(SRC, BASE_ID, &catalogs);
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
        "check" | "validate" => check_cmd(&rest),
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
    eprintln!("  forge check <doc.forge.yaml>            parse + validate");
    eprintln!("  forge build <doc.forge.yaml> [opts]     compile a native app");
    eprintln!("      -o <path>       output binary path (default: ./<name>)");
    eprintln!("      --release       release profile (default: dev)");
    eprintln!("      --dir <path>    scaffold dir (default: <doc>/.forge-build/<name>)");
    eprintln!("  forge fmt <doc.forge.yaml>              canonical reformat (stdout or -w)");
    eprintln!("  forge convert <file>                  between .forge.yaml and JSON");
    eprintln!("      --from yaml|json  input format (else detected by extension/content)");
    eprintln!("      --to yaml|json    output format (default: the other one)");
    eprintln!("      -o <path>         output file (default: stdout; `-` reads stdin)");
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
    let (doc, diagnostics) = gpui_forge_runtime::check::check(
        &src,
        gpui_forge_runtime::base::BASE_ID,
        &catalogs,
    );
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

/// `.forge.yaml` <-> JSON. Format is detected by extension, then content;
/// `--from`/`--to` override. `-` as the input reads stdin.
fn convert_cmd(rest: &[String]) -> i32 {
    let file = rest.iter().find(|a| !a.starts_with('-') && *a != "--from" && *a != "--to" && *a != "-o");
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
    let Ok((doc, _)) = gpui_forge_runtime::forge_spec::yaml::from_str_with_source_map(&src)
    else {
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

fn build_cmd(rest: &[String]) -> i32 {
    let Some(path) = doc_path(rest) else {
        usage();
        return 2;
    };
    let release = rest.iter().any(|a| a == "--release");
    let path = path.canonicalize().unwrap_or(path);
    let name = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .replace('.', "-");
    let dir = opt_value(rest, "--dir").map(PathBuf::from).unwrap_or_else(|| {
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
    let root = workspace_root().canonicalize().unwrap_or_else(|_| workspace_root());
    let title = name.replace('-', " ");
    write(
        &dir.join("Cargo.toml"),
        &APP_TOML.replace("{NAME}", &name).replace("{ROOT}", &root.display().to_string()),
    );
    write(
        &src_dir.join("main.rs"),
        &APP_MAIN.replace("{APP_ID}", &name).replace("{TITLE}", &title),
    );
    write(&dir.join("app.forge.yaml"), &std::fs::read_to_string(&path).unwrap_or_default());

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
        eprintln!("forge: cannot copy {} -> {}: {e}", binary.display(), out.display());
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
