//! The studio's composed `AssetSource`. Icons resolve through layers, in
//! order: the bundled example set, project filesystem roots, gpui-kit's
//! embedded lucide catalog (`icons/<name>.svg`), then the iconify disk cache
//! (`~/.cache/…/iconify/<set>/<name>.svg`, filled lazily by `ensure_*`).
//! A non-lucide iconset lives under `icons/<set>/<name>.svg` — a project
//! supplies one by dropping SVGs into `<assets_dir>/icons/<set>/` and naming
//! it in `[icons] set` (or a document's `meta.iconset`).

use gpui::{AssetSource, Result, SharedString};
use gpui_kit as gpui;
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Directories searched before the embedded catalog — the shipped
/// `assets/` dir plus each opened project's assets root.
static ICON_ROOTS: RwLock<Vec<PathBuf>> = RwLock::new(Vec::new());

/// Registers `dir` as an icon root (idempotent). Missing dirs are kept —
/// the dir may appear later (assets are created on demand).
pub fn add_icon_root(dir: PathBuf) {
    let mut roots = ICON_ROOTS.write().unwrap_or_else(|e| e.into_inner());
    if !roots.contains(&dir) {
        roots.push(dir);
    }
}

/// A second iconset bundled with Studio so non-lucide rendering can be
/// exercised without a project — Heroicons (MIT), `icons/heroicons/…`.
const HEROICONS: &[(&str, &[u8])] = &[
    (
        "icons/heroicons/bell.svg",
        include_bytes!("../assets/icons/heroicons/bell.svg"),
    ),
    (
        "icons/heroicons/bolt.svg",
        include_bytes!("../assets/icons/heroicons/bolt.svg"),
    ),
    (
        "icons/heroicons/calendar.svg",
        include_bytes!("../assets/icons/heroicons/calendar.svg"),
    ),
    (
        "icons/heroicons/camera.svg",
        include_bytes!("../assets/icons/heroicons/camera.svg"),
    ),
    (
        "icons/heroicons/cloud.svg",
        include_bytes!("../assets/icons/heroicons/cloud.svg"),
    ),
    (
        "icons/heroicons/cog.svg",
        include_bytes!("../assets/icons/heroicons/cog.svg"),
    ),
    (
        "icons/heroicons/document.svg",
        include_bytes!("../assets/icons/heroicons/document.svg"),
    ),
    (
        "icons/heroicons/folder.svg",
        include_bytes!("../assets/icons/heroicons/folder.svg"),
    ),
    (
        "icons/heroicons/heart.svg",
        include_bytes!("../assets/icons/heroicons/heart.svg"),
    ),
    (
        "icons/heroicons/home.svg",
        include_bytes!("../assets/icons/heroicons/home.svg"),
    ),
    (
        "icons/heroicons/pencil.svg",
        include_bytes!("../assets/icons/heroicons/pencil.svg"),
    ),
    (
        "icons/heroicons/plus.svg",
        include_bytes!("../assets/icons/heroicons/plus.svg"),
    ),
    (
        "icons/heroicons/star.svg",
        include_bytes!("../assets/icons/heroicons/star.svg"),
    ),
    (
        "icons/heroicons/trash.svg",
        include_bytes!("../assets/icons/heroicons/trash.svg"),
    ),
    (
        "icons/heroicons/user.svg",
        include_bytes!("../assets/icons/heroicons/user.svg"),
    ),
];

pub struct StudioAssets;

/// Files under `root` matching `path`, returned as asset paths — recursive
/// so `list("icons/")` discovers `icons/<set>/<name>.svg`.
fn list_dir(root: &Path, path: &str, out: &mut Vec<SharedString>) {
    let dir = root.join(path);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    for entry in entries.flatten() {
        let file = entry.path();
        let Some(rel) = file.strip_prefix(root).ok().map(|p| p.to_path_buf()) else {
            continue;
        };
        let rel = rel.to_string_lossy().replace('\\', "/");
        if file.is_dir() {
            list_dir(root, &rel, out);
        } else {
            out.push(rel.into());
        }
    }
}

impl AssetSource for StudioAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some((_, bytes)) = HEROICONS.iter().find(|(p, _)| *p == path) {
            return Ok(Some(Cow::Borrowed(*bytes)));
        }
        if let Ok(roots) = ICON_ROOTS.read() {
            for root in roots.iter() {
                let file = root.join(path);
                if file.is_file()
                    && let Ok(bytes) = std::fs::read(&file)
                {
                    return Ok(Some(Cow::Owned(bytes)));
                }
            }
        }
        // `AllAssets` returns `Err` for paths it doesn't embed — a miss here
        // must fall through to the iconify layer, not abort the whole load.
        if let Ok(Some(bytes)) = gpui_kit::assets::AllAssets.load(path) {
            return Ok(Some(bytes));
        }
        // Iconify — `icons/<set>/<name>.svg` synthesizes from the cached
        // `IconifyJSON` (`ensure_set` fills it); renders gated on
        // `icon_cached` reach here only for resolvable names.
        if let Some(rest) = path.strip_prefix("icons/")
            && let Some((set, name)) = rest.split_once('/')
            && !name.contains('/')
            && let Some(bytes) =
                gpui_forge_runtime::iconify::svg(set, name.trim_end_matches(".svg"))
        {
            return Ok(Some(Cow::Owned(bytes)));
        }
        Ok(None)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut out = gpui_kit::assets::AllAssets.list(path)?;
        out.extend(
            HEROICONS
                .iter()
                .filter(|(p, _)| p.starts_with(path))
                .map(|(p, _)| (*p).into()),
        );
        if let Ok(roots) = ICON_ROOTS.read() {
            for root in roots.iter() {
                list_dir(root, path, &mut out);
            }
        }
        out.sort();
        out.dedup();
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_icons() {
        let files = StudioAssets.list("icons/").unwrap();
        eprintln!(
            "total: {}, first: {:?}",
            files.len(),
            &files[..5.min(files.len())]
        );
        let hero = files
            .iter()
            .filter(|f| f.starts_with("icons/heroicons/"))
            .count();
        eprintln!("heroicons: {hero}");
        assert!(!files.is_empty());
    }

    #[test]
    fn loads_lucide() {
        let bytes = StudioAssets.load("icons/bell.svg").unwrap();
        assert!(bytes.is_some());
    }
}
