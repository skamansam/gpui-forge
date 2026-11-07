//! `cargo xtask` — the project's task runner (the matklad convention).
//! Wraps the base-matrix feature flags and per-base host builds so nobody
//! memorizes `--no-default-features --features base-<id>` incantations.

use std::path::Path;
use std::process::Command;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn root() -> std::path::PathBuf {
    Path::new(ROOT).join("../..").canonicalize().unwrap()
}

/// `base:` ids shipped in `bases/*.base.yaml` — drives `host`/`check-bases`.
fn bases() -> Vec<String> {
    let dir = root().join("bases");
    let mut ids: Vec<String> = std::fs::read_dir(dir)
        .expect("bases/ directory")
        .flatten()
        .filter_map(|e| {
            e.file_name()
                .to_string_lossy()
                .strip_suffix(".base.yaml")
                .map(str::to_owned)
        })
        .collect();
    ids.sort();
    ids
}

/// Feature flags selecting one base: gpui-kit is the default build, every
/// other base is `--no-default-features --features base-<id>`.
fn base_flags(base: &str) -> Vec<String> {
    if base == "gpui-kit" {
        Vec::new()
    } else {
        vec![
            "--no-default-features".into(),
            "--features".into(),
            format!("base-{base}"),
        ]
    }
}

fn run(args: &[&str]) -> bool {
    let status = Command::new("cargo")
        .args(args)
        .current_dir(root())
        .status()
        .expect("spawn cargo");
    status.success()
}

fn usage() -> ! {
    eprintln!("cargo xtask — project tasks");
    eprintln!("  xtask build              workspace build (gpui-kit)");
    eprintln!("  xtask test               workspace tests");
    eprintln!("  xtask check              fmt + clippy -D warnings (CI parity)");
    eprintln!("  xtask ci                 check + test");
    eprintln!("  xtask studio             run Forge Studio");
    eprintln!("  xtask cli <args..>       run the `forge` binary");
    eprintln!("  xtask bases              list registered bases");
    eprintln!("  xtask host [base..]      build forge-preview[-<base>] (default: all)");
    eprintln!("  xtask example <doc>      run the base's render example");
    eprintln!("      [--base <id>]        (default gpui-kit)");
    eprintln!("  xtask check-base <base>  clippy the runtime under a base feature");
    eprintln!("  xtask check-bases        check-base for every registered base");
    std::process::exit(2)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("build") => {
            std::process::exit(!run(&["build", "--workspace"]) as i32);
        }
        Some("test") => {
            std::process::exit(!run(&["test", "--workspace"]) as i32);
        }
        Some("check") => {
            let ok = run(&["fmt", "--", "--check"])
                && run(&[
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "--",
                    "-D",
                    "warnings",
                ]);
            std::process::exit(!ok as i32);
        }
        Some("ci") => {
            let ok = run(&["fmt", "--", "--check"])
                && run(&[
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "--",
                    "-D",
                    "warnings",
                ])
                && run(&["test", "--workspace"]);
            std::process::exit(!ok as i32);
        }
        Some("studio") => {
            std::process::exit(!run(&["run", "-p", "forge-studio"]) as i32);
        }
        Some("cli") => {
            let mut cargo = vec!["run", "-p", "forge-cli", "--"];
            cargo.extend(args[1..].iter().map(String::as_str));
            std::process::exit(!run(&cargo) as i32);
        }
        Some("bases") => {
            for b in bases() {
                println!("{b}");
            }
        }
        Some("host") => {
            let requested: Vec<&str> = args[1..].iter().map(String::as_str).collect();
            let targets: Vec<String> = if requested.is_empty() {
                bases()
            } else {
                requested.iter().map(|s| s.to_string()).collect()
            };
            let mut ok = true;
            for base in &targets {
                let mut cargo = vec!["build", "-p", "forge-preview"];
                let flags = base_flags(base);
                cargo.extend(flags.iter().map(String::as_str));
                println!("==> forge-preview ({base})");
                ok &= run(&cargo);
            }
            std::process::exit(!ok as i32);
        }
        Some("example") => {
            let mut base = "gpui-kit".to_string();
            let mut doc = None;
            let mut i = 1;
            while i < args.len() {
                if args[i] == "--base" {
                    base = args.get(i + 1).cloned().unwrap_or_default();
                    i += 2;
                } else {
                    doc = Some(args[i].clone());
                    i += 1;
                }
            }
            let Some(doc) = doc else { usage() };
            let example = match base.as_str() {
                "gpui02" => "render_gpui02",
                _ => "render",
            };
            let mut cargo = vec!["run", "-p", "gpui-forge-runtime"];
            let flags = base_flags(&base);
            cargo.extend(flags.iter().map(String::as_str));
            cargo.extend(["--example", example, "--", &doc]);
            if base != "gpui-kit" && std::env::var_os("WSL_DISTRO_NAME").is_some() {
                eprintln!(
                    "hint: upstream gpui may reject WSLg's Wayland — try\n  env -u WAYLAND_DISPLAY LIBGL_ALWAYS_SOFTWARE=1 cargo xtask example {doc} --base {base}"
                );
            }
            std::process::exit(!run(&cargo) as i32);
        }
        Some("check-base") => {
            let Some(base) = args.get(1) else { usage() };
            let mut cargo = vec!["clippy", "-p", "gpui-forge-runtime"];
            let flags = base_flags(base);
            cargo.extend(flags.iter().map(String::as_str));
            cargo.extend(["--", "-D", "warnings"]);
            std::process::exit(!run(&cargo) as i32);
        }
        Some("check-bases") => {
            let mut ok = true;
            for base in bases() {
                println!("==> {base}");
                let mut cargo = vec!["clippy", "-p", "gpui-forge-runtime"];
                let flags = base_flags(&base);
                cargo.extend(flags.iter().map(String::as_str));
                cargo.extend(["--", "-D", "warnings"]);
                ok &= run(&cargo);
            }
            std::process::exit(!ok as i32);
        }
        _ => usage(),
    }
}
