//! `forge.toml` field access — the manifest format is flat enough that a
//! line scan beats pulling in a TOML dependency.

/// Reads `key = "value"` lines out of a manifest's `[forge]` section.
pub fn manifest_value(src: &str, key: &str) -> Option<String> {
    manifest_section(src, "forge", key)
}

/// `key = "…"` inside a `[section]` of a `forge.toml` — section-aware so
/// `[assets] dir` doesn't read a `[forge] dir` by accident.
pub fn manifest_section(src: &str, section: &str, key: &str) -> Option<String> {
    let body = src
        .split(&format!("[{section}]"))
        .nth(1)?
        .split('[')
        .next()?;
    body.lines().map(str::trim).find_map(|l| {
        let v = l.strip_prefix(key)?.trim_start().strip_prefix('=')?.trim();
        match v.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
            Some(s) => Some(s.to_owned()),
            // Bare values too — `watch = true` (a TOML bool) reads as "true".
            None if !v.is_empty() => Some(v.to_owned()),
            None => None,
        }
    })
}

/// `meta.watch` / `[forge] watch` are declared as strings or bare bools —
/// both normalise through here.
pub fn truthy(v: &str) -> bool {
    matches!(v, "true" | "1" | "yes" | "on")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `key = "value"` with spaces around `=` must parse — the workspace
    /// `forge.toml` is written that way, and a miss surfaced as a bogus
    /// "forge.toml has no forge.document".
    #[test]
    fn manifest_section_parses_spaced_equals() {
        let src = "[package]\nname = \"x\"\n\n[forge]\nbase = \"gpui-kit\"\ndocument = \"app.forge.yaml\"\n\n[icons]\nset = \"heroicons\"\n";
        assert_eq!(
            manifest_value(src, "document").as_deref(),
            Some("app.forge.yaml")
        );
        assert_eq!(
            manifest_section(src, "icons", "set").as_deref(),
            Some("heroicons")
        );
        assert_eq!(manifest_section(src, "forge", "missing"), None);
        // `documents` must not satisfy a `document` lookup.
        let docs = "[forge]\ndocuments = [\"a.forge.yaml\"]\n";
        assert_eq!(manifest_value(docs, "document"), None);
        // Bare bools parse too — `[forge] watch = true`.
        assert_eq!(
            manifest_value("[forge]\nwatch = true\n", "watch").as_deref(),
            Some("true")
        );
        assert!(truthy("true") && truthy("yes") && !truthy("false") && !truthy(""));
        // The real workspace manifest parses its entry document.
        let repo = include_str!("../../../forge.toml");
        assert_eq!(
            manifest_value(repo, "document").as_deref(),
            Some("crates/forge-studio/studio.forge.yaml")
        );
    }
}
