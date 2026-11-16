//! `cargo xtask components` — upstream-drift check for catalog `component:`
//! paths and pinned upstream versions.
//!
//! For every library a catalog references, this resolves the version Forge
//! actually builds against (Cargo.lock → base manifests → vendor manifests →
//! catalog `version:`), indexes the matching upstream source with `syn`, and
//! answers three questions:
//!
//!   versions — is the resolved version still crates.io's `max_stable_version`?
//!   removed  — does every catalog `component:` path still exist upstream?
//!   added    — which exported widget types no catalog references yet?
//!
//! `removed` and outdated versions fail `--check`; `added` is advisory.
//! Vendored (`vendor/<dir>`) and workspace (`local`) sources skip crates.io.
//! Policy lives in `drift.toml`.

use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

const USER_AGENT: &str = "gpui-forge xtask components (component drift check)";

// ---------------------------------------------------------------------------
// drift.toml

#[derive(Deserialize, Default)]
struct DriftConfig {
    #[serde(default)]
    crates: BTreeMap<String, CratePolicy>,
}

#[derive(Clone, Deserialize, Default)]
struct CratePolicy {
    /// crates.io package when it differs from `<lib>` (default: `_` → `-`).
    package: Option<String>,
    /// Source tree in the repo for crates not on crates.io.
    vendored: Option<String>,
    /// Workspace crate — component paths check against its own `src/`.
    local: Option<String>,
    /// Don't check this crate; the string is the reason shown in the report.
    skip: Option<String>,
    /// Intentional non-latest version — reported as pinned, not a failure.
    pin: Option<String>,
    reason: Option<String>,
    /// Restrict the not-cataloged report to this module subtree.
    surface: Option<String>,
    /// Export names to never list as "not cataloged".
    #[serde(default)]
    ignore: Vec<String>,
}

fn load_config() -> DriftConfig {
    let path = crate::root().join("drift.toml");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return DriftConfig::default();
    };
    toml::from_str(&text).unwrap_or_else(|e| {
        eprintln!("drift.toml: {e}");
        std::process::exit(2)
    })
}

// ---------------------------------------------------------------------------
// Catalog + manifest inputs

/// One `component:` reference, expanded out of `a::b::{C, D}` brace lists.
struct CompRef {
    catalog: String,
    element: String,
    path: String,
}

fn load_catalogs() -> Vec<(String, forge_spec::catalog::Catalog)> {
    let dir = crate::root().join("catalogs");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("catalogs/ directory")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "yaml"))
        .collect();
    files.sort();
    files
        .into_iter()
        .filter_map(|p| {
            let slug = p.file_name()?.to_string_lossy().into_owned();
            let text = std::fs::read_to_string(&p).ok()?;
            let cat = forge_spec::catalog::from_yaml(&text)
                .map_err(|d| eprintln!("{slug}: {d:?}"))
                .ok()?;
            Some((format!("catalogs/{slug}"), cat))
        })
        .collect()
}

/// Expands `a::b::{C, D}` → `a::b::C`, `a::b::D` (nested braces recurse).
fn expand_component(raw: &str) -> Vec<String> {
    let s = raw.trim();
    let Some(open) = s.find('{') else {
        return vec![s.to_string()];
    };
    let Some(close) = s[open..].find('}').map(|i| open + i) else {
        return vec![s.to_string()];
    };
    let prefix = s[..open].trim_end_matches("::");
    let suffix = &s[close + 1..];
    s[open + 1..close]
        .split(',')
        .flat_map(|part| {
            let part = part.trim();
            if part.is_empty() {
                Vec::new()
            } else {
                expand_component(&format!("{prefix}::{part}{suffix}"))
            }
        })
        .collect()
}

/// `Cargo.lock` `[[package]]` name → version, from each lockfile we have.
fn lock_versions() -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for lock in ["Cargo.lock", "hosts/adabraka/Cargo.lock"] {
        let Ok(text) = std::fs::read_to_string(crate::root().join(lock)) else {
            continue;
        };
        let Ok(doc) = text.parse::<toml::Table>() else {
            continue;
        };
        for pkg in doc
            .get("package")
            .and_then(|p| p.as_array())
            .into_iter()
            .flatten()
        {
            if let (Some(n), Some(v)) = (pkg["name"].as_str(), pkg["version"].as_str()) {
                out.entry(n.to_string()).or_insert_with(|| v.to_string());
            }
        }
    }
    out
}

/// `[package] version` from a manifest, resolving `version.workspace`.
fn manifest_version(dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(dir.join("Cargo.toml")).ok()?;
    let doc = text.parse::<toml::Table>().ok()?;
    let version = doc.get("package")?.get("version")?;
    if let Some(v) = version.as_str() {
        return Some(v.to_string());
    }
    if version.get("workspace").and_then(|w| w.as_bool()) == Some(true) {
        let text = std::fs::read_to_string(crate::root().join("Cargo.toml")).ok()?;
        let doc = text.parse::<toml::Table>().ok()?;
        return doc
            .get("workspace")?
            .get("package")?
            .get("version")?
            .as_str()
            .map(str::to_string);
    }
    None
}

// ---------------------------------------------------------------------------
// crates.io

fn curl(url: &str, out: Option<&Path>) -> Result<String, String> {
    let mut cmd = Command::new("curl");
    cmd.args(["-fsSL", "--retry", "2", "-H"])
        .arg(format!("User-Agent: {USER_AGENT}"))
        .arg(url);
    if let Some(path) = out {
        cmd.arg("-o").arg(path);
    }
    let output = cmd.output().map_err(|e| format!("curl: {e}"))?;
    if !output.status.success() {
        return Err(format!("{url}: curl exited {}", output.status));
    }
    match out {
        Some(_) => Ok(String::new()),
        None => String::from_utf8(output.stdout).map_err(|e| e.to_string()),
    }
}

fn crates_io_latest(package: &str) -> Result<String, String> {
    let body = curl(&format!("https://crates.io/api/v1/crates/{package}"), None)?;
    let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    json["crate"]["max_stable_version"]
        .as_str()
        .or_else(|| json["crate"]["newest_version"].as_str())
        .map(str::to_string)
        .ok_or_else(|| "no version field in crates.io response".to_string())
}

/// Download + extract `{pkg}-{ver}.crate` into `target/drift/src/`, cached.
fn crate_source(package: &str, version: &str) -> Result<PathBuf, String> {
    let dir = crate::root().join(format!("target/drift/src/{package}-{version}"));
    if dir.join("Cargo.toml").is_file() {
        return Ok(dir);
    }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let ball = dir.with_file_name(format!("{package}-{version}.crate"));
    curl(
        &format!("https://static.crates.io/crates/{package}/{package}-{version}.crate"),
        Some(&ball),
    )?;
    let ok = Command::new("tar")
        .args(["-xzf"])
        .arg(&ball)
        .arg("-C")
        .arg(&dir)
        .arg("--strip-components=1")
        .status()
        .is_ok_and(|s| s.success());
    if !ok {
        return Err(format!("{package}-{version}: could not extract tarball"));
    }
    Ok(dir)
}

/// `version` from a dep table can be a req (`0.11`, `^1`) rather than a
/// concrete release — resolve it via crates.io's version list.
fn resolve_version(package: &str, req: &str) -> String {
    let req = req.trim_start_matches(['=', '^', '~', '>', ' ']);
    let is_concrete =
        req.split('.').count() == 3 && req.chars().next().is_some_and(|c| c.is_ascii_digit());
    if is_concrete {
        return req.to_string();
    }
    let Ok(text) = curl(&format!("https://crates.io/api/v1/crates/{package}"), None) else {
        return req.to_string();
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return req.to_string();
    };
    json["versions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|v| {
            v["yanked"].as_bool() != Some(true) && !v["num"].as_str().unwrap_or("").contains('-')
        })
        .filter_map(|v| v["num"].as_str())
        .filter(|num| *num == req || num.starts_with(&format!("{req}.")))
        .max()
        .unwrap_or(req)
        .to_string()
}

// ---------------------------------------------------------------------------
// Upstream source index (syn)

#[derive(Default)]
struct Module {
    /// `pub` items declared here (types, fns, consts).
    items: BTreeSet<String>,
    /// Leaf names re-exported by `pub use`.
    reexports: BTreeSet<String>,
    /// `pub use m::*` — candidate local module paths to expand (bare first
    /// segments get both the crate-root and sibling-module readings).
    globs: BTreeSet<String>,
    /// `pub use dep::*` — graft the dep's surface into THIS module.
    extern_globs: Vec<(String /*lib*/, String /*subpath*/)>,
    /// `pub use dep` / `pub use dep::sub` / `as alias` — graft the dep's
    /// `subpath` surface at `module::alias`.
    extern_names: Vec<(
        String, /*alias*/
        String, /*lib*/
        String, /*subpath*/
    )>,
    /// `mod` declarations → is it `pub`?
    mods: BTreeMap<String, bool>,
}

struct CrateIndex {
    mods: BTreeMap<String, Module>,
    /// Rust `use`-name → crates.io package (dep key, or `package =` override).
    deps: BTreeMap<String, String>,
    /// package → version req string from the manifest.
    dep_reqs: BTreeMap<String, String>,
}

/// `[lib] path` / package name / dep tables from an extracted crate manifest.
fn crate_manifest(
    dir: &Path,
) -> (
    String,
    PathBuf,
    BTreeMap<String, String>,
    BTreeMap<String, String>,
) {
    let Ok(text) = std::fs::read_to_string(dir.join("Cargo.toml")) else {
        return (
            String::new(),
            PathBuf::from("src/lib.rs"),
            BTreeMap::new(),
            BTreeMap::new(),
        );
    };
    let Ok(doc) = text.parse::<toml::Value>() else {
        return (
            String::new(),
            PathBuf::from("src/lib.rs"),
            BTreeMap::new(),
            BTreeMap::new(),
        );
    };
    let get = |path: &[&str]| -> Option<&toml::Value> {
        let mut v = &doc;
        for key in path {
            v = v.get(*key)?;
        }
        Some(v)
    };
    let name = get(&["package", "name"])
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let lib_path = get(&["lib", "path"])
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("src/lib.rs"));
    let mut deps = BTreeMap::new();
    let mut reqs = BTreeMap::new();
    let mut read_deps = |table: &toml::Value| {
        let Some(map) = table.as_table() else {
            return;
        };
        for (key, val) in map {
            let (package, req) = match val {
                toml::Value::String(req) => (key.clone(), req.clone()),
                toml::Value::Table(t) => (
                    t.get("package")
                        .and_then(|p| p.as_str())
                        .unwrap_or(key)
                        .to_string(),
                    t.get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("*")
                        .to_string(),
                ),
                _ => continue,
            };
            deps.insert(key.clone(), package.clone());
            reqs.entry(package).or_insert(req);
        }
    };
    if let Some(d) = get(&["dependencies"]) {
        read_deps(d);
    }
    if let Some(targets) = get(&["target"]).and_then(|t| t.as_table()) {
        for (_, t) in targets {
            if let Some(d) = t.get("dependencies") {
                read_deps(d);
            }
        }
    }
    (name, lib_path, deps, reqs)
}

/// `src/` relative file path → module path (`a/mod.rs` → `a`; the `[lib] path`
/// entry point — usually `src/lib.rs` — is the crate root).
fn file_mod_path(crate_dir: &Path, lib_path: &Path, file: &Path) -> Option<String> {
    if file == crate_dir.join(lib_path) {
        return Some(String::new());
    }
    let rel = file.strip_prefix(crate_dir.join("src")).ok()?;
    let mut parts: Vec<String> = rel
        .with_extension("")
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    if matches!(parts.last().map(String::as_str), Some("mod")) {
        parts.pop();
    }
    Some(parts.join("::"))
}

fn item_is_pub(item: &syn::Item) -> bool {
    let vis = match item {
        syn::Item::Struct(i) => &i.vis,
        syn::Item::Enum(i) => &i.vis,
        syn::Item::Type(i) => &i.vis,
        syn::Item::Trait(i) => &i.vis,
        syn::Item::Fn(i) => &i.vis,
        syn::Item::Union(i) => &i.vis,
        syn::Item::Use(i) => &i.vis,
        syn::Item::Mod(i) => &i.vis,
        _ => return false,
    };
    matches!(vis, syn::Visibility::Public(_))
}

fn collect_items(
    items: &[syn::Item],
    mods: &mut BTreeMap<String, Module>,
    path: &str,
    deps: &BTreeMap<String, String>,
) {
    let mut inline: Vec<(String, &Vec<syn::Item>)> = Vec::new();
    let m = mods.entry(path.to_string()).or_default();
    for item in items {
        match item {
            syn::Item::Struct(i) if item_is_pub(item) => {
                m.items.insert(i.ident.to_string());
            }
            syn::Item::Enum(i) if item_is_pub(item) => {
                m.items.insert(i.ident.to_string());
            }
            syn::Item::Type(i) if item_is_pub(item) => {
                m.items.insert(i.ident.to_string());
            }
            syn::Item::Trait(i) if item_is_pub(item) => {
                m.items.insert(i.ident.to_string());
            }
            syn::Item::Fn(i) if item_is_pub(item) => {
                m.items.insert(i.sig.ident.to_string());
            }
            syn::Item::Union(i) if item_is_pub(item) => {
                m.items.insert(i.ident.to_string());
            }
            syn::Item::Use(i) if item_is_pub(item) => {
                let mut segs = Vec::new();
                walk_use(&i.tree, &mut segs, path, m, deps);
            }
            syn::Item::Mod(i) => {
                m.mods.insert(i.ident.to_string(), item_is_pub(item));
                if let Some((_, content)) = &i.content {
                    let child = if path.is_empty() {
                        i.ident.to_string()
                    } else {
                        format!("{path}::{}", i.ident)
                    };
                    inline.push((child, content));
                }
            }
            _ => {}
        }
    }
    for (child, content) in inline {
        collect_items(content, mods, &child, deps);
    }
}

/// A `use` path's first segment names a dep when it matches the dep key —
/// `pub use ::gpui::*` in gpui-kit hits `gpui = { package = "gpui-pre" }`.
/// `use` renders hyphens as underscores (`gpui-component` → `gpui_component`),
/// so both spellings are checked. Lib-name mismatches (e.g. `use guise` via
/// dep key `guise-ui`) fall back to the local-module reading, which is also
/// what Rust resolves first in `use`.
fn dep_package<'a>(name: &str, deps: &'a BTreeMap<String, String>) -> Option<&'a String> {
    deps.get(name).or_else(|| deps.get(&name.replace('_', "-")))
}

fn first_is_dep(segs: &[String], deps: &BTreeMap<String, String>) -> bool {
    segs.first().is_some_and(|s| dep_package(s, deps).is_some())
}

/// Collect `pub use` leaf names, glob sources, and extern (facade) grafts.
fn walk_use(
    tree: &syn::UseTree,
    segs: &mut Vec<String>,
    module: &str,
    m: &mut Module,
    deps: &BTreeMap<String, String>,
) {
    match tree {
        syn::UseTree::Path(p) => {
            segs.push(p.ident.to_string());
            walk_use(&p.tree, segs, module, m, deps);
            segs.pop();
        }
        syn::UseTree::Name(n) => {
            let leaf = n.ident.to_string();
            // Full path = prefix segs + leaf; `pub use ext` has empty segs and
            // the leaf IS the first segment.
            let mut full = segs.clone();
            full.push(leaf.clone());
            if first_is_dep(&full, deps) {
                let sub = full
                    .get(1..full.len().saturating_sub(1))
                    .unwrap_or(&[])
                    .join("::");
                m.extern_names.push((leaf.clone(), full[0].clone(), sub));
            }
            m.reexports.insert(leaf);
        }
        syn::UseTree::Rename(r) => {
            let alias = r.rename.to_string();
            let mut full = segs.clone();
            full.push(r.ident.to_string());
            if first_is_dep(&full, deps) {
                let sub = full
                    .get(1..full.len().saturating_sub(1))
                    .unwrap_or(&[])
                    .join("::");
                m.extern_names.push((alias.clone(), full[0].clone(), sub));
            }
            m.reexports.insert(alias);
        }
        syn::UseTree::Glob(_) => {
            if first_is_dep(segs, deps) {
                m.extern_globs.push((segs[0].clone(), segs[1..].join("::")));
            } else if !segs.is_empty() {
                let normalized = normalize_use_path(segs, module);
                m.globs.insert(normalized.clone());
                // A bare first segment also reads as a sibling of this module —
                // `pub use guise::*` inside `render` is `render::guise`, not
                // the extern `guise` crate (local items shadow the extern
                // prelude in `use`).
                if segs[0] != "self" && segs[0] != "super" && segs[0] != "crate" {
                    let sibling = if module.is_empty() {
                        normalized
                    } else {
                        format!("{module}::{normalized}")
                    };
                    m.globs.insert(sibling);
                }
            }
        }
        syn::UseTree::Group(g) => {
            for t in &g.items {
                walk_use(t, segs, module, m, deps);
            }
        }
    }
}

/// `self`/`super`/`crate` anchor relative `use` paths; anything else is
/// crate-absolute (root item or extern crate name).
fn normalize_use_path(segs: &[String], module: &str) -> String {
    let mut parts: Vec<&str> = module.split("::").filter(|s| !s.is_empty()).collect();
    if segs.first().is_some_and(|s| s != "self" && s != "super") {
        parts.clear();
    }
    let mut i = 0;
    while i < segs.len() {
        match segs[i].as_str() {
            "self" | "crate" => i += 1,
            "super" => {
                parts.pop();
                i += 1;
            }
            s => {
                parts.push(s);
                i += 1;
            }
        }
    }
    parts.join("::")
}

/// Splice `src_index`'s module `sub` (and its submodules) into `mods` at
/// `alias` — how `pub use ext` and `pub use ext::sub::*` become resolvable.
fn graft(mods: &mut BTreeMap<String, Module>, alias: &str, src_index: &CrateIndex, sub: &str) {
    for (spath, smod) in &src_index.mods {
        // sub="" grafts the dep's whole tree (root + every submodule).
        if !(sub.is_empty() || spath == sub || spath.starts_with(&format!("{sub}::"))) {
            continue;
        }
        let suffix = spath
            .strip_prefix(sub)
            .unwrap_or("")
            .trim_start_matches("::");
        let dest = if alias.is_empty() && suffix.is_empty() {
            String::new()
        } else if suffix.is_empty() {
            alias.to_string()
        } else if alias.is_empty() {
            suffix.to_string()
        } else {
            format!("{alias}::{suffix}")
        };
        let d = mods.entry(dest.clone()).or_default();
        d.items.extend(smod.items.iter().cloned());
        d.items.extend(smod.reexports.iter().cloned());
        for (name, public) in &smod.mods {
            d.mods.entry(name.clone()).or_insert(*public);
        }
        // Register the grafted path in its parent's mod table so
        // `module_is_public` traverses it.
        if let Some((parent, leaf)) = dest.rsplit_once("::") {
            mods.entry(parent.to_string())
                .or_default()
                .mods
                .entry(leaf.to_string())
                .or_insert(true);
        }
    }
    if !alias.is_empty() {
        if let Some((parent, leaf)) = alias.rsplit_once("::") {
            mods.entry(parent.to_string())
                .or_default()
                .mods
                .entry(leaf.to_string())
                .or_insert(true);
        } else {
            mods.entry(String::new())
                .or_default()
                .mods
                .entry(alias.to_string())
                .or_insert(true);
        }
    }
}

/// Full index for one crate source dir: files → dep facades (recursively) →
/// glob expansion. `lib_names` maps package → its `[lib] name` once fetched.
fn index_crate(
    crate_dir: &Path,
    locks: &BTreeMap<String, String>,
    depth: usize,
    cache: &mut BTreeMap<String, CrateIndex>,
) -> CrateIndex {
    let (_, lib_path, deps, dep_reqs) = crate_manifest(crate_dir);
    let mut index = CrateIndex {
        mods: BTreeMap::new(),
        deps: deps.clone(),
        dep_reqs,
    };
    let lib_file = crate_dir.join(&lib_path);

    // Walk every .rs under src/ (plus the [lib] path if it lives elsewhere).
    let mut stack = vec![crate_dir.join("src")];
    if !lib_file.starts_with(crate_dir.join("src")) && lib_file.is_file() {
        stack.push(lib_file.clone());
    }
    while let Some(dir) = stack.pop() {
        let entries: Vec<PathBuf> = if dir.is_file() {
            vec![dir]
        } else {
            match std::fs::read_dir(&dir) {
                Ok(rd) => rd.flatten().map(|e| e.path()).collect(),
                Err(_) => continue,
            }
        };
        for path in entries {
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if !path.extension().is_some_and(|x| x == "rs") {
                continue;
            }
            let Some(mod_path) = file_mod_path(crate_dir, &lib_path, &path) else {
                continue;
            };
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            // Upstream files may use syntax newer than our syn — parse
            // failures and literal-handling panics are both recoverable.
            let file =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| syn::parse_file(&text)))
                    .ok()
                    .and_then(|r| r.ok());
            let Some(file) = file else {
                eprintln!("warning: could not parse {}", path.display());
                continue;
            };
            collect_items(&file.items, &mut index.mods, &mod_path, &index.deps);
        }
    }

    // Resolve extern grafts (facades) recursively, depth-capped.
    if depth < 3 {
        let grafts: Vec<(String, String, String)> = index
            .mods
            .iter()
            .flat_map(|(mpath, m)| {
                m.extern_globs
                    .iter()
                    .map(move |(lib, sub)| (mpath.clone(), lib.clone(), sub.clone()))
                    .chain(m.extern_names.iter().map(move |(alias, lib, sub)| {
                        (
                            if mpath.is_empty() {
                                alias.clone()
                            } else {
                                format!("{mpath}::{alias}")
                            },
                            lib.clone(),
                            sub.clone(),
                        )
                    }))
            })
            .collect();
        for (target, lib, sub) in grafts {
            let Some(package) = dep_package(&lib, &index.deps).cloned() else {
                continue;
            };
            let version = locks
                .get(&package)
                .cloned()
                .or_else(|| index.dep_reqs.get(&package).cloned());
            let Some(version) = version else {
                continue;
            };
            let version = resolve_version(&package, &version);
            let key = format!("{package}-{version}");
            if !cache.contains_key(&key) {
                match crate_source(&package, &version) {
                    Ok(d) => {
                        let idx = index_crate(&d, locks, depth + 1, cache);
                        cache.insert(key.clone(), idx);
                    }
                    Err(e) => {
                        eprintln!("warning: facade dep {package}: {e}");
                        continue;
                    }
                }
            }
            if let Some(dep_idx) = cache.get(&key) {
                graft(&mut index.mods, &target, dep_idx, &sub);
            }
        }
    }

    // Glob re-exports: `pub use a::*` pulls a's public names into this module.
    for _ in 0..20 {
        let mut changed = false;
        let snapshot: Vec<(String, BTreeSet<String>)> = index
            .mods
            .iter()
            .map(|(p, m)| (p.clone(), m.globs.clone()))
            .collect();
        for (path, globs) in snapshot {
            let mut pulled = BTreeSet::new();
            for g in globs {
                if let Some(src) = index.mods.get(&g) {
                    pulled.extend(src.items.iter().cloned());
                    pulled.extend(src.reexports.iter().cloned());
                }
            }
            let m = index.mods.get_mut(&path).unwrap();
            changed |= m.reexports.len() != {
                m.reexports.extend(pulled);
                m.reexports.len()
            };
        }
        if !changed {
            break;
        }
    }
    index
}

/// Is module `path` reachable through `pub mod` chains from the crate root?
fn module_is_public(mods: &BTreeMap<String, Module>, path: &str) -> bool {
    let mut parent = String::new();
    for seg in path.split("::") {
        match mods.get(&parent).and_then(|m| m.mods.get(seg)) {
            Some(true) => {
                if !parent.is_empty() {
                    parent.push_str("::");
                }
                parent.push_str(seg);
            }
            _ => return false,
        }
    }
    true
}

/// Does `component: path` resolve to a public item in the indexed source?
/// Existence-based: private `mod` files count too — catalogs document the
/// implementing type, which may live in a private module behind `pub use`.
fn path_resolves(mods: &BTreeMap<String, Module>, path: &str) -> bool {
    let segs: Vec<&str> = path.split("::").filter(|s| !s.is_empty()).collect();
    if segs.len() < 2 {
        return false;
    }
    let (leaf, mod_segs) = (segs[segs.len() - 1], &segs[1..segs.len() - 1]);
    let module = mod_segs.join("::");
    if let Some(m) = mods.get(&module) {
        if m.items.contains(leaf) || m.reexports.contains(leaf) || m.mods.contains_key(leaf) {
            return true;
        }
    }
    // The leaf may itself be a module (`component: foo::bar`).
    let leaf_mod = if module.is_empty() {
        leaf.to_string()
    } else {
        format!("{module}::{leaf}")
    };
    mods.contains_key(&leaf_mod)
}

// ---------------------------------------------------------------------------
// Report

#[derive(Clone, PartialEq)]
enum VerStatus {
    Ok,
    Pinned,
    Behind,
    Ahead,
    NotFound,
    Vendored,
    Local,
    Skipped,
    Unresolved,
}

struct CrateReport {
    lib: String,
    package: String,
    used: Option<String>,
    latest: Option<String>,
    status: VerStatus,
    removed: Vec<(String, String, String)>, // (catalog, element, path)
    added: Vec<String>,
    note: Option<String>,
    fetch_error: Option<String>,
}

fn semver_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    fn core(v: &str) -> (Vec<u64>, &str) {
        let v = v.split('+').next().unwrap_or(v);
        let (num, pre) = match v.split_once('-') {
            Some((n, p)) => (n, p),
            None => (v, ""),
        };
        (
            num.split('.').map(|s| s.parse().unwrap_or(0)).collect(),
            pre,
        )
    }
    let (an, ap) = core(a);
    let (bn, bp) = core(b);
    an.cmp(&bn)
        .then_with(|| match (ap.is_empty(), bp.is_empty()) {
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            _ => ap.cmp(bp),
        })
}

fn looks_like_widget(name: &str, ignore: &BTreeSet<&str>) -> bool {
    const SUFFIXES: &[&str] = &[
        "Event",
        "EventType",
        "Kind",
        "Error",
        "Config",
        "Options",
        "Builder",
        "State",
        "Mode",
        "Direction",
        "Orientation",
        "Side",
        "Placement",
        "Variant",
        "Size",
        "Layout",
        "Role",
        "Status",
        "Fit",
        "Outcome",
        "Dir",
        "Manager",
        "Stage",
        "Entry",
        "Item",
        "Node",
        "Column",
        "Record",
        "Ref",
        "Name",
        "Names",
        "Choice",
        "Source",
        "Version",
        "Token",
        "Handle",
        "Id",
        "Key",
        "Path",
        "Pos",
        "Rect",
        "Range",
        "Theme",
        "Palette",
        "Binding",
        "Signal",
        "Validator",
        "Rule",
        "Form",
        "FormState",
        "FormValues",
        "Highlighter",
        "Language",
        "Severity",
        "Model",
        "Diagnostic",
        "Metrics",
        "Stats",
        "Args",
        "Params",
        "Values",
        "Filter",
        "Sort",
        "Span",
        "Scale",
        "Shades",
        "Step",
        "Data",
        "Info",
        "Result",
        "Update",
        "Inset",
        "Type",
    ];
    const EXACT: &[&str] = &[
        "Align",
        "Justify",
        "Breakpoint",
        "Weekday",
        "Loop",
        "Curve",
        "Easing",
        "At",
        "Prop",
        "Track",
        "Glyph",
        "Color",
        "ColorName",
        "ColorScheme",
        "AnimValue",
        "Keyframe",
        "StaggerAxis",
        "StaggerFrom",
        "SelectionMode",
        "SortDir",
        "ObjectFit",
        "SplitDirection",
        "LoaderVariant",
        "TextAreaSubmit",
        "KeyOutcome",
        "IconName",
        "Context",
        "Surface",
    ];
    name.chars().next().is_some_and(char::is_uppercase)
        && name.chars().any(char::is_lowercase)
        && !ignore.contains(name)
        && !EXACT.contains(&name)
        && !SUFFIXES.iter().any(|s| name.ends_with(s))
}

pub fn run(args: &[String]) -> ! {
    let mut check = false;
    let mut github = false;
    let mut report_path: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--check" => check = true,
            "--format" => {
                github = args.get(i + 1).is_some_and(|a| a == "github");
                i += 1;
            }
            "--github" => github = true,
            "--report" => {
                report_path = args.get(i + 1).map(PathBuf::from);
                i += 1;
            }
            other => {
                eprintln!("unknown flag: {other}");
                std::process::exit(2);
            }
        }
        i += 1;
    }

    let root = crate::root();
    let cfg = load_config();
    let catalogs = load_catalogs();
    let locks = lock_versions();

    // Base manifests pin the toolchain crates (gpui + library per base).
    let mut declared: BTreeMap<String, String> = BTreeMap::new();
    if let Ok(rd) = std::fs::read_dir(root.join("bases")) {
        for entry in rd.flatten() {
            let Ok(text) = std::fs::read_to_string(entry.path()) else {
                continue;
            };
            if let Ok(b) = forge_spec::base::Base::from_yaml(&text) {
                declared.entry(b.gpui_crate).or_insert(b.gpui_version);
                declared.entry(b.library_crate).or_insert(b.library_version);
            }
        }
    }

    // Gather component refs grouped by crate (first path segment).
    let mut by_crate: BTreeMap<String, Vec<CompRef>> = BTreeMap::new();
    for (file, cat) in &catalogs {
        for el in &cat.elements {
            let Some(comp) = &el.component else {
                continue;
            };
            for path in expand_component(comp) {
                let lib = path.split("::").next().unwrap_or_default().to_string();
                by_crate.entry(lib).or_default().push(CompRef {
                    catalog: file.clone(),
                    element: el.name.clone(),
                    path,
                });
            }
        }
    }

    // The version-check universe: every referenced package plus the base
    // manifests' gpui/library pins (both keyed by package name).
    let mut packages: BTreeSet<String> = BTreeSet::new();
    for lib in by_crate.keys() {
        let pol = cfg.crates.get(lib);
        if pol.is_some_and(|p| p.skip.is_some() || p.local.is_some()) {
            continue;
        }
        packages.insert(
            pol.and_then(|p| p.package.clone())
                .unwrap_or_else(|| lib.replace('_', "-")),
        );
    }
    packages.extend(declared.keys().cloned());

    // ------------------------------------------------------------------
    // Version check, one report row per package.
    let mut reports: Vec<CrateReport> = Vec::new();
    for pkg in &packages {
        let used = locks
            .get(pkg)
            .cloned()
            .or_else(|| declared.get(pkg).cloned());
        let lib = pkg.replace('-', "_");
        let pol = cfg.crates.get(&lib);
        let (latest, status, note) = if pol.is_some_and(|p| p.vendored.is_some()) {
            (
                None,
                VerStatus::Vendored,
                pol.and_then(|p| p.vendored.clone()),
            )
        } else {
            match crates_io_latest(pkg) {
                Ok(latest) => {
                    let status = match &used {
                        None => VerStatus::Unresolved,
                        Some(u) => match semver_cmp(u, &latest) {
                            std::cmp::Ordering::Equal => VerStatus::Ok,
                            std::cmp::Ordering::Greater => VerStatus::Ahead,
                            std::cmp::Ordering::Less => {
                                if pol.is_some_and(|p| p.pin.as_deref() == Some(u.as_str())) {
                                    VerStatus::Pinned
                                } else {
                                    VerStatus::Behind
                                }
                            }
                        },
                    };
                    (Some(latest), status, None)
                }
                Err(e) => (None, VerStatus::NotFound, Some(e)),
            }
        };
        reports.push(CrateReport {
            lib,
            package: pkg.clone(),
            used,
            latest,
            status,
            removed: Vec::new(),
            added: Vec::new(),
            note,
            fetch_error: None,
        });
    }

    // ------------------------------------------------------------------
    // Component checks per referenced crate.
    let mut index_cache: BTreeMap<String, CrateIndex> = BTreeMap::new();
    for (lib, refs) in by_crate {
        let pol = cfg.crates.get(&lib).cloned().unwrap_or_default();
        if let Some(reason) = pol.skip {
            reports.push(CrateReport {
                lib: lib.clone(),
                package: lib.clone(),
                used: None,
                latest: None,
                status: VerStatus::Skipped,
                removed: Vec::new(),
                added: Vec::new(),
                note: Some(reason),
                fetch_error: None,
            });
            continue;
        }

        let mut fetch_error = None;
        let (src_dir, used) = if let Some(v) = &pol.vendored {
            let dir = root.join(v);
            (Some(dir.clone()), manifest_version(&dir))
        } else if let Some(l) = &pol.local {
            let dir = root.join(l);
            (Some(dir.clone()), manifest_version(&dir))
        } else {
            let package = pol.package.clone().unwrap_or_else(|| lib.replace('_', "-"));
            let used = locks
                .get(&package)
                .cloned()
                .or_else(|| declared.get(&package).cloned())
                .or_else(|| {
                    catalogs
                        .iter()
                        .find(|(_, c)| c.library.replace('-', "_") == lib)
                        .map(|(_, c)| c.version.clone())
                });
            match &used {
                Some(v) => match crate_source(&package, v) {
                    Ok(d) => (Some(d), used),
                    Err(e) => {
                        fetch_error = Some(e);
                        (None, used)
                    }
                },
                None => (None, used),
            }
        };

        let Some(src_dir) = src_dir else {
            reports.push(CrateReport {
                lib: lib.clone(),
                package: lib.clone(),
                used,
                latest: None,
                status: VerStatus::Unresolved,
                removed: Vec::new(),
                added: Vec::new(),
                note: Some(if fetch_error.is_some() {
                    "fetch failed".into()
                } else {
                    "no version source".into()
                }),
                fetch_error,
            });
            continue;
        };

        let index = index_crate(&src_dir, &locks, 0, &mut index_cache);
        let mods = &index.mods;
        let mut removed = Vec::new();
        let mut referenced = BTreeSet::new();
        for r in &refs {
            if let Some(leaf) = r.path.rsplit("::").next() {
                referenced.insert(leaf.to_string());
            }
            if !path_resolves(mods, &r.path) {
                removed.push((r.catalog.clone(), r.element.clone(), r.path.clone()));
            }
        }

        // Upstream surface not referenced by any catalog.
        let ignore: BTreeSet<&str> = pol.ignore.iter().map(String::as_str).collect();
        let surface = pol.surface.unwrap_or_default();
        let mut added = BTreeSet::new();
        for (path, m) in mods {
            let under_surface =
                surface.is_empty() || path == &surface || path.starts_with(&format!("{surface}::"));
            if !under_surface || !module_is_public(mods, path) {
                continue;
            }
            for name in m.items.iter().chain(&m.reexports) {
                if !referenced.contains(name) && looks_like_widget(name, &ignore) {
                    added.insert(name.clone());
                }
            }
        }

        reports.push(CrateReport {
            lib: lib.clone(),
            package: pol.package.unwrap_or_else(|| lib.replace('_', "-")),
            used,
            latest: None,
            status: if pol.local.is_some() {
                VerStatus::Local
            } else if pol.vendored.is_some() {
                VerStatus::Vendored
            } else {
                VerStatus::Ok
            },
            removed,
            added: added.into_iter().collect(),
            note: None,
            fetch_error,
        });
    }

    // ------------------------------------------------------------------
    // Merge component rows into version rows by package.
    let mut merged: BTreeMap<String, CrateReport> = BTreeMap::new();
    for r in reports {
        merged
            .entry(r.package.clone())
            .and_modify(|e| {
                e.removed.extend(r.removed.clone());
                e.added.extend(r.added.clone());
                e.added.sort();
                e.added.dedup();
                if e.status == VerStatus::Ok {
                    e.status = r.status.clone();
                }
                if e.note.is_none() {
                    e.note = r.note.clone();
                }
                if e.fetch_error.is_none() {
                    e.fetch_error = r.fetch_error.clone();
                }
            })
            .or_insert(r);
    }

    // ------------------------------------------------------------------
    // Emit.
    let mut md = String::from("# Upstream drift report\n\n## Versions\n\n");
    md.push_str("| package | used | latest | status |\n|---|---|---|---|\n");
    let mut removed_rows: Vec<(String, String, String, String)> = Vec::new();
    let mut added_rows: Vec<(String, Vec<String>)> = Vec::new();
    let mut failures = 0usize;

    for r in merged.values() {
        let status = match r.status {
            VerStatus::Ok => "ok".to_string(),
            VerStatus::Pinned => {
                let reason = cfg
                    .crates
                    .get(&r.lib)
                    .and_then(|p| p.reason.clone())
                    .unwrap_or_default();
                format!("pinned ({reason})")
            }
            VerStatus::Behind => "**outdated**".into(),
            VerStatus::Ahead => "ahead of crates.io".into(),
            VerStatus::NotFound => "**not on crates.io**".into(),
            VerStatus::Vendored => "vendored".into(),
            VerStatus::Local => "workspace".into(),
            VerStatus::Skipped => "skipped".into(),
            VerStatus::Unresolved => "unresolved".into(),
        };
        md.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            r.package,
            r.used.as_deref().unwrap_or("—"),
            r.latest.as_deref().unwrap_or("—"),
            status
        ));
        for (cat, el, path) in &r.removed {
            removed_rows.push((r.package.clone(), cat.clone(), el.clone(), path.clone()));
        }
        if !r.added.is_empty() {
            added_rows.push((r.package.clone(), r.added.clone()));
        }

        failures += r.removed.len();
        if matches!(r.status, VerStatus::Behind | VerStatus::NotFound) || r.fetch_error.is_some() {
            failures += 1;
        }

        if github {
            if let Some(e) = &r.fetch_error {
                println!("::error::{}: {e}", r.package);
            }
            match r.status {
                VerStatus::Behind => println!(
                    "::error::{} is behind crates.io: used {}, latest {}",
                    r.package,
                    r.used.as_deref().unwrap_or("?"),
                    r.latest.as_deref().unwrap_or("?")
                ),
                VerStatus::Pinned => println!(
                    "::notice::{} pinned at {} (latest {})",
                    r.package,
                    r.used.as_deref().unwrap_or("?"),
                    r.latest.as_deref().unwrap_or("?")
                ),
                VerStatus::NotFound => println!(
                    "::error::{} not found on crates.io ({})",
                    r.package,
                    r.note.as_deref().unwrap_or("")
                ),
                _ => {}
            }
        }
    }

    md.push_str("\n## Removed upstream components\n\n");
    if removed_rows.is_empty() {
        md.push_str("None — every catalog `component:` path resolves.\n");
    } else {
        md.push_str("| package | catalog | element | path |\n|---|---|---|---|\n");
        for (pkg, cat, el, path) in &removed_rows {
            md.push_str(&format!("| {pkg} | {cat} | {el} | `{path}` |\n"));
            if github {
                println!(
                    "::error file={cat},title=removed upstream component::{path} ({el}) not found"
                );
            }
        }
    }

    md.push_str("\n## Upstream components not cataloged\n\n");
    if added_rows.is_empty() {
        md.push_str("None.\n");
    } else {
        for (pkg, names) in &added_rows {
            md.push_str(&format!(
                "<details><summary>{pkg} ({})</summary>\n\n{}\n\n</details>\n\n",
                names.len(),
                names.join(", ")
            ));
            if github {
                println!(
                    "::notice::{pkg}: {} upstream components not cataloged — see step summary",
                    names.len()
                );
            }
        }
    }

    if let Some(path) = report_path {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&path, &md).expect("write drift report");
    }
    print!("{md}");
    if check && failures > 0 {
        eprintln!("component drift check failed: {failures} finding(s)");
        std::process::exit(1);
    }
    std::process::exit(0)
}
