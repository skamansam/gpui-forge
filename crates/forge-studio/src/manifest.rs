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

/// A `[section] key = ["a", "b"]` list as owned strings — inline or spread
/// across lines. The scan quotes each item, so a `,` inside a string does
/// not split. A missing key gives an empty vec.
pub fn manifest_list_section(src: &str, section: &str, key: &str) -> Vec<String> {
    // Section bodies end at the next line-start `[`, never a `[` inside a
    // value — `documents = ["a"]` holds its own bracket.
    let body = src
        .split(&format!("\n[{section}]"))
        .nth(1)
        .or_else(|| src.strip_prefix(&format!("[{section}]")))
        .unwrap_or_default();
    let body = body.split("\n[").next().unwrap_or_default();
    let mut lines = body.lines().map(str::trim);
    let mut text = String::new();
    for l in lines.by_ref() {
        let Some(v) = l
            .strip_prefix(key)
            .and_then(|r| r.trim_start().strip_prefix('='))
        else {
            continue;
        };
        text.push_str(v);
        if v.contains(']') {
            break;
        }
        // Multi-line array — pull lines until the closing bracket.
        for l in lines.by_ref() {
            text.push_str(l);
            if l.contains(']') {
                break;
            }
        }
        break;
    }
    let mut items = Vec::new();
    let mut rest = text.as_str();
    while let Some(open) = rest.find('"') {
        rest = &rest[open + 1..];
        let Some(close) = rest.find('"') else { break };
        items.push(rest[..close].to_owned());
        rest = &rest[close + 1..];
    }
    items
}

/// Every `[[apps]]` block as `(name, document)` — `name` falls back to the
/// document's file stem. `forge.document` is the primary application. These
/// are the extra entry points a project declares.
pub fn manifest_apps(src: &str) -> Vec<(String, String)> {
    src.split("[[apps]]")
        .skip(1)
        .filter_map(|block| {
            // The block body ends at the next `[` — another `[[apps]]` or a
            // `[section]` header.
            let body = block.split('[').next()?;
            let value = |key: &str| {
                body.lines().map(str::trim).find_map(|l| {
                    let v = l.strip_prefix(key)?.trim_start().strip_prefix('=')?.trim();
                    v.strip_prefix('"')
                        .and_then(|s| s.strip_suffix('"'))
                        .map(str::to_owned)
                })
            };
            let document = value("document")?;
            let name = value("name").unwrap_or_else(|| {
                std::path::Path::new(&document)
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .trim_end_matches(".forge")
                    .to_owned()
            });
            Some((name, document))
        })
        .collect()
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

    /// `[[apps]]` blocks yield one (name, document) pair each. `name` falls
    /// back to the document stem, and the parser drops a block with no
    /// `document`. A `documents` list must not read as `document`.
    #[test]
    fn manifest_apps_lists_each_block() {
        let src = "[forge]\ndocument = \"main.forge.yaml\"\ndocuments = [\"x.forge.yaml\"]\n\n[[apps]]\nname = \"console\"\ndocument = \"docs/console.forge.yaml\"\n\n[[apps]]\ndocument = \"docs/kiosk.forge.yaml\"\n\n[assets]\ndir = \"assets\"\n";
        assert_eq!(
            manifest_apps(src),
            vec![
                ("console".to_owned(), "docs/console.forge.yaml".to_owned()),
                ("kiosk".to_owned(), "docs/kiosk.forge.yaml".to_owned()),
            ]
        );
        assert!(manifest_apps("[forge]\ndocument = \"a.forge.yaml\"\n").is_empty());
    }
}
