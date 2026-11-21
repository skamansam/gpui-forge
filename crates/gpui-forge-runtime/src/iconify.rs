//! Iconify (iconify.design) — browse the hosted catalog and vendor just the
//! icons a document uses into a project's `assets/icons/<set>/`.
//!
//! Icon data comes as whole-set `IconifyJSON` files from the upstream
//! `iconify/icon-sets` GitHub repo (`json/<set>.json`, via the jsDelivr gh
//! mirror) — one GET per set gets every name, body, alias and dimension plus
//! `lastModified`, `info`, `categories`, `prefixes`/`suffixes`, `hidden` and
//! `chars`, cached at `~/.cache/gpui-forge/iconify/<set>.json`. `svg()`
//! synthesizes `<svg>` bytes locally from that JSON (resolving aliases and
//! rotate/flip transforms), so `AssetSource::load("icons/<set>/<name>.svg")`
//! works with no per-icon network at all once a set is cached.
//!
//! `sets.json` (the `/collections` index) still provides the catalog listing
//! and license metadata for attribution.
//!
//! All fetches are blocking `ureq` on the background executor; `ensure_*`
//! calls `refresh_windows` when data lands so callers just re-render.

use gpui_kit::App;
use serde_json::Value as Json;
use std::collections::HashSet;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

const API: &str = "https://api.iconify.design";
/// The `iconify/icon-sets` repo's `json/` dir via the jsDelivr GitHub mirror
/// — CDN-cached, no API rate limits.
const SETS: &str = "https://cdn.jsdelivr.net/gh/iconify/icon-sets@master/json";
/// Mirror of `SETS` — the same `iconify/icon-sets` repo over GitHub's raw
/// host; a flaky TLS/CDN stream on one isn't fatal.
const SETS_FALLBACK: &str = "https://raw.githubusercontent.com/iconify/icon-sets/master/json";

/// A host-provided cache-dir override (the `icon_cache` pref). `None` falls
/// back to `dirs::cache_dir()/gpui-forge/iconify`.
static CACHE_DIR: std::sync::RwLock<Option<PathBuf>> = std::sync::RwLock::new(None);

/// Overrides the iconify cache root — call before any fetch, or memos are
/// cleared so subsequent reads see the new location.
pub fn set_cache_dir(dir: Option<PathBuf>) {
    *CACHE_DIR.write().unwrap_or_else(|e| e.into_inner()) = dir;
    json_memo().lock().unwrap().clear();
    requested().lock().unwrap().clear();
}

/// The cache root in effect — the override or the platform default.
pub fn cache_dir_path() -> PathBuf {
    if let Some(dir) = CACHE_DIR.read().unwrap_or_else(|e| e.into_inner()).clone() {
        return dir;
    }
    dirs::cache_dir().map_or_else(
        || PathBuf::from("/tmp/gpui-forge-iconify"),
        |d| d.join("gpui-forge/iconify"),
    )
}

fn cache_dir() -> PathBuf {
    cache_dir_path()
}

fn sets_file() -> PathBuf {
    cache_dir().join("sets.json")
}

fn set_file(set: &str) -> PathBuf {
    cache_dir().join(format!("{set}.json"))
}

fn read_json(file: &Path) -> Option<Json> {
    std::fs::read_to_string(file)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

// ------------------------------------------------------- cached set metadata

/// Every set id the catalog knows — empty until `ensure_sets` lands once.
pub fn sets() -> Vec<String> {
    read_json(&sets_file())
        .and_then(|j| j.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()))
        .unwrap_or_default()
}

/// License metadata for a set from the catalog — `(title, spdx)`. Tolerates
/// the old flat `{prefix: "title"}` cache format.
fn license_pair(set: &str) -> Option<(String, String)> {
    let v = read_json(&sets_file())?.get(set)?.clone();
    if let Some(title) = v.as_str() {
        return Some((title.to_owned(), String::new()));
    }
    Some((
        v["license"]["title"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        v["license"]["spdx"].as_str().unwrap_or_default().to_owned(),
    ))
}

/// A set's license title, for vendored-attribution files.
pub fn license_of(set: &str) -> Option<String> {
    license_pair(set).map(|(t, _)| t)
}

/// Display metadata for a set — its display name, license title and SPDX id.
/// `(name, license_title, spdx)`.
pub fn set_info(set: &str) -> Option<(String, String, String)> {
    let v = read_json(&sets_file())?.get(set)?.clone();
    let (title, spdx) = license_pair(set)?;
    Some((v["name"].as_str().unwrap_or(set).to_owned(), title, spdx))
}

/// SPDX ids that carry no notice-retention requirement — everything else
/// (MIT, BSD, Apache, CC-BY, OFL, GPL…) legally needs the license text and
/// copyright notices shipped with the app.
const NO_ATTRIBUTION: &[&str] = &[
    "CC0-1.0",
    "Unlicense",
    "WTFPL",
    "MIT-0",
    "0BSD",
    "BSD-1-Clause",
    "CC-PDDC",
    "PDM-1.0",
];

/// Whether an SPDX license id requires attribution/notice retention in a
/// built app. An unknown id errs on the side of warning; an empty id means
/// "no catalog data", which does not.
pub fn spdx_needs_attribution(spdx: &str) -> bool {
    !spdx.is_empty() && !NO_ATTRIBUTION.contains(&spdx)
}

/// Whether a set's license requires attribution/notice retention in a built
/// app.
pub fn attribution_required(set: &str) -> bool {
    license_pair(set)
        .map(|(_, spdx)| spdx_needs_attribution(&spdx))
        .unwrap_or(false)
}

/// Whether `set` is an iconify-catalog id (once the catalog is fetched).
pub fn known_set(set: &str) -> bool {
    read_json(&sets_file())
        .and_then(|j| j.as_object().map(|o| o.contains_key(set)))
        .unwrap_or(false)
}

/// The raw catalog entry for a set — the `collections` object (`name`,
/// `total`, `author{name,url}`, `license{title,spdx,url}`, `url`, `samples`,
/// `height`, `displayHeight`, `category`, `tags`, `palette`, `hidden`),
/// falling back to the set's own `info.json` once downloaded.
pub fn catalog_entry(set: &str) -> Option<Json> {
    if let Some(entry) = read_json(&sets_file()).and_then(|j| j.get(set).cloned()) {
        return Some(entry);
    }
    set_json(set).and_then(|j| j.get("info").cloned())
}

/// The set file's `lastModified` epoch seconds — the version stamp the
/// freshness check and the browser's "updated" line share.
pub fn last_modified(set: &str) -> Option<i64> {
    set_json(set).and_then(|j| j["lastModified"].as_i64())
}

/// Epoch seconds → `YYYY-MM-DD` (Howard Hinnant's civil-from-days).
pub fn epoch_ymd(secs: i64) -> String {
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

// ----------------------------------------------------- collection metadata

/// Lucide categories vendored from `lucide-icons/lucide` repo metadata —
/// iconify's `lucide.json` ships none. Same `categories` shape; regenerate
/// with `scripts/gen-lucide-categories.py`.
const LUCIDE_CATEGORIES: &str = include_str!("lucide-categories.json");

fn parse_categories_map(v: &Json) -> Vec<(String, Vec<String>)> {
    v.as_object()
        .map(|o| {
            o.iter()
                .map(|(k, v)| {
                    // Two shapes in the wild: a plain names array, or the
                    // older `{icons: [...], hidden: [...]}` object.
                    let names = v
                        .as_array()
                        .or_else(|| v["icons"].as_array())
                        .map(|a| {
                            a.iter()
                                .filter_map(|n| n.as_str().map(str::to_owned))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    (k.clone(), names)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// `(category, names)` pairs from the set's `metadata.json`, sorted by name —
/// icons appear under every category they belong to. Names in no category
/// get an `Uncategorized` bucket (computed locally — the npm `metadata.json`
/// doesn't always carry the field the API's `collection` response does).
pub fn categories(set: &str) -> Vec<(String, Vec<String>)> {
    let j = set_json(set);
    let mut out: Vec<(String, Vec<String>)> = j
        .as_ref()
        .map(|j| parse_categories_map(&j["categories"]))
        .unwrap_or_default();
    if out.is_empty()
        && set == "lucide"
        && let Ok(v) = serde_json::from_str::<Json>(LUCIDE_CATEGORIES)
    {
        out = parse_categories_map(&v["categories"]);
    }
    let Some(j) = j else {
        out.sort_by(|a, b| a.0.cmp(&b.0));
        return out;
    };
    let mut uncategorized: Vec<String> = j["uncategorized"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|n| n.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    if uncategorized.is_empty() && !out.is_empty() {
        let categorized: std::collections::HashSet<&str> = out
            .iter()
            .flat_map(|(_, list)| list.iter().map(|s| s.as_str()))
            .collect();
        uncategorized = names(set)
            .into_iter()
            .filter(|n| !categorized.contains(n.as_str()))
            .collect();
    }
    if !uncategorized.is_empty() {
        out.push(("Uncategorized".to_owned(), uncategorized));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Whether a theme is a name prefix (`baseline-home`) or suffix
/// (`home-filled`) match — the `prefixes`/`suffixes` maps from the
/// collection metadata.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeKind {
    Prefix,
    Suffix,
}

/// `(kind, key, title)` theme entries from `prefixes` + `suffixes`, sorted
/// default-first then by title. An empty `key` marks the default theme —
/// icons matching no other key of that kind belong to it; when the map
/// omits `""` one is synthesized so a "no suffix" choice always exists.
pub fn themes(set: &str) -> Vec<(ThemeKind, String, String)> {
    let Some(j) = set_json(set) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (field, kind) in [
        ("prefixes", ThemeKind::Prefix),
        ("suffixes", ThemeKind::Suffix),
    ] {
        if let Some(o) = j[field].as_object() {
            let mut saw_default = false;
            for (k, v) in o {
                if let Some(title) = v.as_str() {
                    saw_default |= k.is_empty();
                    out.push((kind, k.clone(), title.to_owned()));
                }
            }
            if !saw_default {
                let t = if kind == ThemeKind::Prefix {
                    "no prefix"
                } else {
                    "no suffix"
                };
                out.push((kind, String::new(), t.to_owned()));
            }
        }
    }
    out.sort_by(|a, b| b.1.is_empty().cmp(&a.1.is_empty()).then(a.2.cmp(&b.2)));
    out
}

fn theme_key_matches(kind: ThemeKind, key: &str, name: &str) -> bool {
    match kind {
        ThemeKind::Prefix => name.starts_with(&format!("{key}-")),
        ThemeKind::Suffix => name.ends_with(&format!("-{key}")),
    }
}

/// Whether `name` belongs to theme `(kind, key)` — an empty `key` is the
/// default theme: it matches names no other key of that kind claims.
pub fn theme_matches(
    themes: &[(ThemeKind, String, String)],
    kind: ThemeKind,
    key: &str,
    name: &str,
) -> bool {
    if key.is_empty() {
        !themes.iter().any(|(k, key2, _)| {
            *k == kind && !key2.is_empty() && theme_key_matches(kind, key2, name)
        })
    } else {
        theme_key_matches(kind, key, name)
    }
}

/// Icon names the set marks `hidden` — deprecated/unlisted entries that
/// still render. Exposed for completeness; the browser doesn't filter them.
pub fn hidden(set: &str) -> Vec<String> {
    set_json(set)
        .and_then(|j| j["hidden"].as_array().cloned())
        .map(|a| {
            a.iter()
                .filter_map(|n| n.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// `(hex-char, icon-name)` pairs from the file's `chars` map — present on
/// sets imported from icon fonts.
pub fn chars(set: &str) -> Vec<(String, String)> {
    set_json(set)
        .and_then(|j| j["chars"].as_object().cloned())
        .map(|o| {
            let mut v: Vec<_> = o
                .iter()
                .filter_map(|(k, n)| n.as_str().map(|s| (k.clone(), s.to_owned())))
                .collect();
            v.sort();
            v
        })
        .unwrap_or_default()
}

/// Case-insensitive natural ordering — digit runs compare numerically so
/// `icon-2` sorts before `icon-10`. Keeps the browser grid stable across
/// re-renders regardless of `HashMap` iteration order upstream.
pub fn nat_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i].is_ascii_digit() && b[j].is_ascii_digit() {
            let (s, e) = (i, j);
            while i < a.len() && a[i].is_ascii_digit() {
                i += 1;
            }
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            let (mut na, mut nb) = (&a[s..i], &b[e..j]);
            let za = na.iter().take_while(|&&c| c == b'0').count();
            let zb = nb.iter().take_while(|&&c| c == b'0').count();
            na = &na[za..];
            nb = &nb[zb..];
            // All-zero runs compare equal — the natural ordering treats
            // "007" as 7.
            let ord = na
                .len()
                .cmp(&nb.len())
                .then_with(|| na.cmp(nb))
                .then_with(|| za.cmp(&zb));
            if ord != Ordering::Equal {
                return ord;
            }
        } else {
            let ord = a[i].to_ascii_lowercase().cmp(&b[j].to_ascii_lowercase());
            if ord != Ordering::Equal {
                return ord;
            }
            i += 1;
            j += 1;
        }
    }
    a.len().cmp(&b.len())
}

// ----------------------------------------------------------- cached set JSON

fn json_memo() -> &'static Mutex<std::collections::HashMap<String, Arc<Json>>> {
    static MEMO: OnceLock<Mutex<std::collections::HashMap<String, Arc<Json>>>> = OnceLock::new();
    MEMO.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

/// A set's full `IconifyJSON`, parsed once then memoized.
fn set_json(set: &str) -> Option<Arc<Json>> {
    if let Some(hit) = json_memo().lock().unwrap().get(set) {
        return Some(hit.clone());
    }
    let json = Arc::new(read_json(&set_file(set))?);
    json_memo()
        .lock()
        .unwrap()
        .insert(set.to_string(), json.clone());
    Some(json)
}

/// Whether the set's `IconifyJSON` is on disk — the "is this set local" test.
pub fn set_cached(set: &str) -> bool {
    set_file(set).is_file()
}

/// Icon names within a set — `icons` + `aliases` keys (aliases resolve to a
/// parent body, so they render like real icons). Empty until `ensure_set`
/// lands.
pub fn names(set: &str) -> Vec<String> {
    let Some(json) = set_json(set) else {
        return Vec::new();
    };
    let mut names: Vec<String> = Vec::new();
    if let Some(o) = json["icons"].as_object() {
        names.extend(o.keys().cloned());
    }
    if let Some(o) = json["aliases"].as_object() {
        names.extend(o.keys().cloned());
    }
    names.sort_by(|a, b| nat_cmp(a, b));
    names
}

/// Whether `name` resolves to an icon in the cached set — the "can this cell
/// render" test; `svg()` synthesizes bytes from the same data.
pub fn icon_cached(set: &str, name: &str) -> bool {
    set_json(set)
        .is_some_and(|j| j["icons"].get(name).is_some() || j["aliases"].get(name).is_some())
}

// ------------------------------------------------------------- svg synthesis

fn json_i64(j: &Json, key: &str) -> Option<i64> {
    j[key].as_i64()
}

/// `mergeIconData` semantics — the child entry applies over the parent:
/// `rotate` adds (mod 4 in `icon_to_svg`), `hFlip`/`vFlip` XOR, everything
/// else overrides.
fn merge_icon(base: &mut serde_json::Map<String, Json>, over: &serde_json::Map<String, Json>) {
    for (k, v) in over {
        match k.as_str() {
            "rotate" => {
                let sum =
                    base.get(k).and_then(|v| v.as_i64()).unwrap_or(0) + v.as_i64().unwrap_or(0);
                base.insert(k.clone(), sum.into());
            }
            "hFlip" | "vFlip" => {
                let x = base.get(k).and_then(|v| v.as_bool()).unwrap_or(false)
                    ^ v.as_bool().unwrap_or(false);
                base.insert(k.clone(), x.into());
            }
            _ => {
                base.insert(k.clone(), v.clone());
            }
        }
    }
}

/// Resolves `name` to merged icon data — walks the alias chain (outermost
/// alias applies last, matching `getIconData`; the counter stops pathological
/// cycles).
fn resolve_icon(set: &Json, name: &str) -> Option<Json> {
    let mut chain = Vec::new();
    let mut current = name.to_string();
    for _ in 0..8 {
        if let Some(icon) = set["icons"].get(&current) {
            let mut merged = icon.as_object()?.clone();
            for alias in chain.iter().rev() {
                merge_icon(&mut merged, alias);
            }
            return Some(Json::Object(merged));
        }
        let alias = set["aliases"].get(&current)?.as_object()?;
        let mut props = alias.clone();
        props.remove("parent");
        chain.push(props);
        current = alias.get("parent")?.as_str()?.to_string();
    }
    None
}

/// Builds `<svg>` bytes for an icon — `body` inside a `viewBox` from
/// `left`/`top`/`width`/`height` (icon overrides set defaults, then 16/0).
/// `rotate`/`hFlip`/`vFlip` become a `<g transform>` — the same math
/// `@iconify/utils`' `iconToSVG` applies (double-flip folds into rotate-180;
/// odd rotations swap the output viewBox axes).
fn icon_to_svg(set: &Json, icon: &Json) -> Option<String> {
    let body = icon["body"].as_str()?;
    let dim = |k: &str| json_i64(icon, k).or_else(|| json_i64(set, k));
    let (mut left, mut top) = (dim("left").unwrap_or(0), dim("top").unwrap_or(0));
    let (mut w, mut h) = (dim("width").unwrap_or(16), dim("height").unwrap_or(16));
    let mut rotate = json_i64(icon, "rotate").unwrap_or(0);
    let hflip = icon["hFlip"].as_bool().unwrap_or(false);
    let vflip = icon["vFlip"].as_bool().unwrap_or(false);

    let mut transforms = Vec::new();
    if hflip {
        if vflip {
            // Both flips ≡ a 180° rotation.
            rotate += 2;
        } else {
            transforms.push(format!("translate({} {}) scale(-1 1)", w + left, -top));
            left = 0;
            top = 0;
        }
    } else if vflip {
        transforms.push(format!("translate({} {}) scale(1 -1)", -left, h + top));
        left = 0;
        top = 0;
    }
    let r = rotate.rem_euclid(4);
    match r {
        1 => {
            let c = h as f64 / 2.0 + top as f64;
            transforms.insert(0, format!("rotate(90 {c} {c})"));
        }
        2 => transforms.insert(
            0,
            format!(
                "rotate(180 {} {})",
                w as f64 / 2.0 + left as f64,
                h as f64 / 2.0 + top as f64
            ),
        ),
        3 => {
            let c = w as f64 / 2.0 + left as f64;
            transforms.insert(0, format!("rotate(-90 {c} {c})"));
        }
        _ => {}
    }
    if r % 2 == 1 {
        std::mem::swap(&mut w, &mut h);
        std::mem::swap(&mut left, &mut top);
    }
    let inner = if transforms.is_empty() {
        body.to_string()
    } else {
        format!("<g transform=\"{}\">{body}</g>", transforms.join(" "))
    };
    Some(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{left} {top} {w} {h}\">{inner}</svg>"
    ))
}

/// SVG bytes for one icon — synthesized from the cached `IconifyJSON`; `None`
/// when the set isn't cached or the name doesn't resolve.
pub fn svg(set: &str, name: &str) -> Option<Vec<u8>> {
    let json = set_json(set)?;
    let icon = resolve_icon(&json, name)?;
    icon_to_svg(&json, &icon).map(String::into_bytes)
}

// ------------------------------------------------------------------- fetching

fn requested() -> &'static Mutex<HashSet<String>> {
    static REQUESTED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    REQUESTED.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Fire a key's fetch once per session — re-renders don't re-queue work.
fn request_once(key: String) -> bool {
    requested().lock().unwrap().insert(key)
}

// ---------------------------------------------------------- progress + errors

/// In-flight download progress: `set -> (bytes_done, bytes_total)`; `total`
/// 0 when the response carried no Content-Length.
fn progress() -> &'static Mutex<std::collections::HashMap<String, (u64, u64)>> {
    static PROGRESS: OnceLock<Mutex<std::collections::HashMap<String, (u64, u64)>>> =
        OnceLock::new();
    PROGRESS.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

/// `(bytes_done, bytes_total)` for an in-flight download, `None` otherwise.
pub fn download_progress(set: &str) -> Option<(u64, u64)> {
    progress().lock().unwrap().get(set).copied()
}

/// Last fetch error per key (`set` names; `"catalog"` for the index) —
/// cleared on success or `retry`.
fn errors() -> &'static Mutex<std::collections::HashMap<String, String>> {
    static ERRORS: OnceLock<Mutex<std::collections::HashMap<String, String>>> = OnceLock::new();
    ERRORS.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

/// The last fetch error for a key, if any.
pub fn fetch_error(key: &str) -> Option<String> {
    errors().lock().unwrap().get(key).cloned()
}

fn set_error(key: &str, msg: String) {
    errors().lock().unwrap().insert(key.to_string(), msg);
}

/// Clear a set's error and consumed request keys, then re-queue its fetch —
/// the browser's retry path.
pub fn retry(set: &str, cx: &mut App) {
    errors().lock().unwrap().remove(set);
    let mut req = requested().lock().unwrap();
    for tag in ["set:", "migrate:", "fresh:", "meta:"] {
        req.remove(&format!("{tag}{set}"));
    }
    drop(req);
    ensure_set(set, cx);
}

/// One blocking GET → response bytes; errors on non-2xx.
#[cfg(not(target_arch = "wasm32"))]
fn get_body(uri: &str) -> io::Result<Vec<u8>> {
    let mut response = ureq::get(uri).call().map_err(io::Error::other)?;
    if !response.status().is_success() {
        return Err(io::Error::other(format!(
            "{uri}: HTTP {}",
            response.status()
        )));
    }
    let mut body = Vec::new();
    response
        .body_mut()
        .as_reader()
        .read_to_end(&mut body)
        .map_err(io::Error::other)?;
    Ok(body)
}

/// `get_body` with byte tracking — publishes `(done, total)` under `set` in
/// the progress map so the browser can render a live bar. Callers own the
/// entry's lifecycle (inserted lazily here, removed after the await).
#[cfg(not(target_arch = "wasm32"))]
fn get_body_tracked(uri: &str, set: &str) -> io::Result<Vec<u8>> {
    let mut response = ureq::get(uri).call().map_err(io::Error::other)?;
    if !response.status().is_success() {
        return Err(io::Error::other(format!(
            "{uri}: HTTP {}",
            response.status()
        )));
    }
    let total = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    progress()
        .lock()
        .unwrap()
        .insert(set.to_string(), (0, total));
    let mut body = Vec::new();
    let mut buf = [0u8; 64 * 1024];
    let mut reader = response.body_mut().as_reader();
    loop {
        let n = reader.read(&mut buf).map_err(io::Error::other)?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&buf[..n]);
        progress()
            .lock()
            .unwrap()
            .insert(set.to_string(), (body.len() as u64, total));
    }
    Ok(body)
}

/// Blocking GETs don't exist on wasm — the canvas only renders icons already
/// in the asset layer (bundled sets), so a fetch request errors cleanly.
#[cfg(target_arch = "wasm32")]
fn get_body(uri: &str) -> io::Result<Vec<u8>> {
    Err(io::Error::other(format!(
        "{uri}: icon fetches are unavailable on wasm"
    )))
}

/// See `get_body` — the tracked variant never runs under wasm.
#[cfg(target_arch = "wasm32")]
fn get_body_tracked(uri: &str, set: &str) -> io::Result<Vec<u8>> {
    let _ = set;
    get_body(uri)
}

/// Fetch the catalog index — `api.iconify.design/collections` → `sets.json`,
/// keeping each set's full entry (`name`, `total`, `author`, `license`,
/// `height`, `palette`, `category`, `tags`, `samples`, `url`, `hidden`) for
/// the browser's metadata display and attribution check.
fn fetch_sets_blocking() -> io::Result<()> {
    let body = get_body(&format!("{API}/collections"))?;
    let json: Json = serde_json::from_slice(&body).map_err(io::Error::other)?;
    std::fs::create_dir_all(cache_dir())?;
    std::fs::write(sets_file(), serde_json::to_vec(&json)?)
}

/// Fetch one set's `lastModified` timestamp from the API — the version check
/// that decides whether a cached `IconifyJSON` needs re-downloading.
fn fetch_last_modified(set: &str) -> io::Result<Option<i64>> {
    let body = get_body(&format!("{API}/last-modified?prefixes={set}"))?;
    let json: Json = serde_json::from_slice(&body).map_err(io::Error::other)?;
    Ok(json["lastModified"][set].as_i64())
}

/// Fetch one set's full `IconifyJSON` from the `iconify/icon-sets` repo →
/// `<set>.json`. The repo files carry `info`, `categories`,
/// `prefixes`/`suffixes`, `hidden` and `chars` inline — one fetch gets
/// everything.
fn fetch_set_blocking(set: &str) -> io::Result<()> {
    // Try the jsDelivr mirror, then the repo's raw host — mid-stream TLS
    // failures on one mirror (they happen on the multi-MB files) shouldn't
    // be the end of the request.
    let mut last_err = None;
    let mut body = None;
    for base in [SETS, SETS_FALLBACK] {
        match get_body_tracked(&format!("{base}/{set}.json"), set) {
            Ok(b) => {
                body = Some(b);
                break;
            }
            Err(e) => last_err = Some(e),
        }
    }
    let body =
        body.ok_or_else(|| last_err.unwrap_or_else(|| io::Error::other("no mirror attempted")))?;
    // Validate before caching — a CDN error page would poison the memo.
    let _: Json = serde_json::from_slice(&body).map_err(io::Error::other)?;
    std::fs::create_dir_all(cache_dir())?;
    std::fs::write(set_file(set), body)?;
    json_memo().lock().unwrap().remove(set);
    errors().lock().unwrap().remove(set);
    Ok(())
}

/// Queue the catalog fetch once; `refresh_windows` re-renders when it lands.
pub fn ensure_sets(cx: &mut App) {
    if !request_once("sets".to_string()) {
        return;
    }
    cx.spawn(async move |cx| {
        if let Err(e) = cx
            .background_executor()
            .spawn(async move { fetch_sets_blocking() })
            .await
        {
            set_error("catalog", e.to_string());
            eprintln!("[icons] catalog fetch failed: {e}");
        }
        cx.update(|app| app.refresh_windows());
    })
    .detach();
}

/// Spawn the tracked set download plus a watcher that refreshes the UI as
/// progress ticks — the bar animates while the file streams in.
fn queue_fetch(set: &str, cx: &mut App) {
    // Seed the progress entry so the watcher outlives scheduling latency.
    progress().lock().unwrap().insert(set.to_string(), (0, 0));
    let watched = set.to_string();
    cx.spawn(async move |cx| {
        loop {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(200))
                .await;
            if download_progress(&watched).is_none() {
                break;
            }
            cx.update(|app| app.refresh_windows());
        }
    })
    .detach();
    let set = set.to_string();
    cx.spawn(async move |cx| {
        let set2 = set.clone();
        let set3 = set.clone();
        if let Err(e) = cx
            .background_executor()
            .spawn(async move { fetch_set_blocking(&set2) })
            .await
        {
            set_error(&set3, e.to_string());
            eprintln!("[icons] set fetch failed for {set3}: {e}");
        }
        progress().lock().unwrap().remove(&set3);
        cx.update(|app| app.refresh_windows());
    })
    .detach();
}

/// Queue one set's `IconifyJSON` fetch once — a browser/doc shows
/// placeholders until it lands, then every icon renders locally. A cached
/// set still queues a `lastModified` check (once per session) and only
/// re-downloads when upstream published a newer version.
pub fn ensure_set(set: &str, cx: &mut App) {
    if set_cached(set) {
        // Files cached from the old npm mirror lack `info`/categories —
        // refetch once to migrate them to the GitHub-sourced format.
        let legacy = set_json(set).is_some_and(|j| j.get("info").is_none());
        if legacy && request_once(format!("migrate:{set}")) {
            queue_fetch(set, cx);
        }
        // Version check once per session.
        if !request_once(format!("fresh:{set}")) {
            return;
        }
        let set = set.to_string();
        cx.spawn(async move |cx| {
            let set2 = set.clone();
            let stale = cx
                .background_executor()
                .spawn(async move { check_stale(&set2) })
                .await
                .unwrap_or(false);
            if stale {
                let _ = cx
                    .background_executor()
                    .spawn(async move { fetch_set_blocking(&set) })
                    .await;
                cx.update(|app| app.refresh_windows());
            }
        })
        .detach();
        return;
    }
    if !request_once(format!("set:{set}")) {
        return;
    }
    queue_fetch(set, cx);
}

/// Whether upstream reports a newer `lastModified` than the cached file —
/// `false` on any uncertainty so a flaky check never wipes a good cache.
fn check_stale(set: &str) -> io::Result<bool> {
    let Some(remote) = fetch_last_modified(set)? else {
        return Ok(false);
    };
    let cached = set_json(set)
        .and_then(|j| j["lastModified"].as_i64())
        .unwrap_or(0);
    Ok(remote > cached)
}

// ------------------------------------------------------------------ vendoring

/// Vendors `names` of `set` into `dest` — blocking; call from a background
/// thread. Fetches the set's `IconifyJSON` if uncached, synthesizes each
/// icon's SVG locally, skips files already on disk, and writes `LICENSE.txt`.
/// Returns `(vendored count, not-found names)`.
pub fn vendor_blocking(
    set: &str,
    names: &[String],
    dest: &Path,
) -> io::Result<(usize, Vec<String>)> {
    if sets().is_empty() {
        fetch_sets_blocking()?;
    }
    if !known_set(set) {
        return Err(io::Error::other(format!(
            "`{set}` is not an iconify catalog set"
        )));
    }
    if !set_cached(set) {
        fetch_set_blocking(set)?;
    }
    std::fs::create_dir_all(dest)?;
    let mut got = 0;
    let mut missing = Vec::new();
    for name in names {
        let file = dest.join(format!("{name}.svg"));
        if file.is_file() {
            continue;
        }
        match svg(set, name) {
            Some(bytes) => {
                std::fs::write(&file, bytes)?;
                got += 1;
            }
            None => missing.push(name.clone()),
        }
    }
    if let Some(license) = license_of(set) {
        let notice = format!(
            "Icons from the `{set}` set ({license}) via iconify.design — vendored on use.\n"
        );
        let _ = std::fs::write(dest.join("LICENSE.txt"), notice);
    }
    Ok((got, missing))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set() -> Json {
        serde_json::json!({
            "prefix": "test",
            "width": 24,
            "height": 24,
            "icons": {
                "plain": { "body": "<path d=\"M0 0h24v24H0z\"/>" },
                "tall": { "body": "<rect/>", "width": 12, "height": 20 },
                "rotated": { "body": "<line/>", "rotate": 1 },
            },
            "aliases": {
                "copy": { "parent": "plain" },
                "flipped": { "parent": "plain", "hFlip": true },
                "spin": { "parent": "rotated", "rotate": 3 },
                "both": { "parent": "plain", "hFlip": true, "vFlip": true },
            }
        })
    }

    #[test]
    fn plain_svg() {
        let s = set();
        let icon = resolve_icon(&s, "plain").unwrap();
        let svg = icon_to_svg(&s, &icon).unwrap();
        assert_eq!(
            svg,
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><path d=\"M0 0h24v24H0z\"/></svg>"
        );
    }

    #[test]
    fn icon_dims_override_set() {
        let s = set();
        let svg = icon_to_svg(&s, &resolve_icon(&s, "tall").unwrap()).unwrap();
        assert!(svg.contains("viewBox=\"0 0 12 20\""));
    }

    #[test]
    fn alias_resolves_to_parent_body() {
        let s = set();
        let svg = icon_to_svg(&s, &resolve_icon(&s, "copy").unwrap()).unwrap();
        assert!(svg.contains("<path d=\"M0 0h24v24H0z\"/>"));
        assert!(!svg.contains("<g transform"));
    }

    #[test]
    fn alias_hflip_wraps_transform() {
        let s = set();
        let svg = icon_to_svg(&s, &resolve_icon(&s, "flipped").unwrap()).unwrap();
        assert!(svg.contains("translate(24 0) scale(-1 1)"));
    }

    #[test]
    fn alias_rotate_adds_over_parent() {
        let s = set();
        // parent rotate 1 + alias rotate 3 = 4 ≡ 0 — no transform.
        let svg = icon_to_svg(&s, &resolve_icon(&s, "spin").unwrap()).unwrap();
        assert!(!svg.contains("<g transform"));
    }

    #[test]
    fn double_flip_is_rotate_180() {
        let s = set();
        let svg = icon_to_svg(&s, &resolve_icon(&s, "both").unwrap()).unwrap();
        assert!(svg.contains("rotate(180 12 12)"));
        assert!(!svg.contains("scale("));
    }

    #[test]
    fn attribution_classification() {
        assert!(spdx_needs_attribution("MIT"));
        assert!(spdx_needs_attribution("Apache-2.0"));
        assert!(spdx_needs_attribution("CC-BY-4.0"));
        assert!(spdx_needs_attribution("UnknownLicense")); // unknown errs safe
        assert!(!spdx_needs_attribution("CC0-1.0"));
        assert!(!spdx_needs_attribution("Unlicense"));
        assert!(!spdx_needs_attribution("")); // no catalog data → quiet
    }

    #[test]
    fn natural_sort() {
        use std::cmp::Ordering;
        assert_eq!(nat_cmp("icon-2", "icon-10"), Ordering::Less);
        assert_eq!(nat_cmp("icon-10", "icon-2"), Ordering::Greater);
        assert_eq!(nat_cmp("Arrow-Up", "arrow-down"), Ordering::Greater);
        assert_eq!(nat_cmp("a", "a"), Ordering::Equal);
        // More leading zeros sorts later on tie-break ("icon-7" < "icon-007").
        assert_eq!(nat_cmp("icon-007", "icon-7"), Ordering::Greater);
    }

    #[test]
    fn theme_matching() {
        let themes = vec![
            (ThemeKind::Suffix, "filled".to_owned(), "Filled".to_owned()),
            (
                ThemeKind::Suffix,
                "outlined".to_owned(),
                "Outlined".to_owned(),
            ),
            (
                ThemeKind::Prefix,
                "baseline".to_owned(),
                "Baseline".to_owned(),
            ),
            (ThemeKind::Suffix, String::new(), "Default".to_owned()),
        ];
        assert!(theme_matches(
            &themes,
            ThemeKind::Suffix,
            "filled",
            "home-filled"
        ));
        // The dash is part of the match — a bare `filledhome` isn't "Filled".
        assert!(!theme_matches(
            &themes,
            ThemeKind::Suffix,
            "filled",
            "filled-home"
        ));
        assert!(theme_matches(
            &themes,
            ThemeKind::Prefix,
            "baseline",
            "baseline-home"
        ));
        assert!(!theme_matches(
            &themes,
            ThemeKind::Prefix,
            "baseline",
            "baselinehome"
        ));
        // The empty-key default claims whatever no real key does.
        assert!(theme_matches(&themes, ThemeKind::Suffix, "", "home"));
        assert!(!theme_matches(
            &themes,
            ThemeKind::Suffix,
            "",
            "home-filled"
        ));
    }

    #[test]
    fn epoch_date() {
        assert_eq!(epoch_ymd(0), "1970-01-01");
        assert_eq!(epoch_ymd(1_737_398_331), "2025-01-20");
    }

    #[test]
    fn lucide_vendored_categories() {
        // iconify's lucide.json ships no `categories`; the vendored file
        // (scripts/gen-lucide-categories.py) supplies them regardless of
        // whether a set file is cached.
        let cats = categories("lucide");
        assert!(cats.len() > 40);
        let arrows = cats
            .iter()
            .find(|(k, _)| k == "Arrows")
            .map(|(_, v)| v)
            .expect("Arrows category");
        assert!(arrows.iter().any(|n| n == "arrow-up"));
    }
}

#[cfg(test)]
mod usvg_check {
    /// Every icon in every cached set must parse through the same `usvg`
    /// `Icon`→`svg()` uses — a parse failure renders a blank cell forever.
    #[test]
    fn all_cached_sets_parse() {
        for set in super::sets() {
            if !super::set_cached(&set) {
                continue;
            }
            let names = super::names(&set);
            let mut bad = Vec::new();
            for name in &names {
                let Some(bytes) = super::svg(&set, name) else {
                    bad.push(format!("{name}: synth failed"));
                    continue;
                };
                if let Err(e) = usvg::Tree::from_data(&bytes, &usvg::Options::default()) {
                    bad.push(format!("{name}: {e}"));
                }
            }
            eprintln!(
                "{set}: {}/{} ok, {} bad",
                names.len() - bad.len(),
                names.len(),
                bad.len()
            );
            for b in bad.iter().take(5) {
                eprintln!("  {b}");
            }
            assert!(bad.is_empty());
        }
    }
}
