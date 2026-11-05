//! Base manifest (`bases/<id>.base.yaml`) — which GPUI stack a document
//! targets. See docs/design/multi-base.md and docs/guide/adapters.md.

use saphyr::{LoadableYamlNode, Yaml};

/// A registered base (component system) — the `base:` field a document sets.
#[derive(Clone, Debug, PartialEq)]
pub struct Base {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    /// The GPUI crate + pin this base links (e.g. gpui-pre 0.3.10).
    pub gpui_crate: String,
    pub gpui_version: String,
    /// The component library crate + pin (e.g. gpui-kit 0.8).
    pub library_crate: String,
    pub library_version: String,
    pub toolchain: String,
    pub platforms: Vec<String>,
    pub wasm: bool,
    /// Theme system + base catalog ids.
    pub theme: String,
    pub catalog: String,
    pub license: Option<String>,
    /// Library ids known compatible with this base (`use: alias: lib`).
    pub libraries: Vec<String>,
}

impl Base {
    fn text(node: &Yaml, key: &str) -> String {
        node[key].as_str().unwrap_or_default().to_string()
    }

    pub fn from_yaml(src: &str) -> Result<Base, String> {
        let docs = Yaml::load_from_str(src).map_err(|e| e.to_string())?;
        let root = docs.into_iter().next().ok_or("empty base manifest")?;
        let gpui = &root["gpui"];
        let library = &root["library"];
        let libs: Vec<String> = root["libraries"]
            .as_mapping()
            .map(|m| {
                m.keys()
                    .filter_map(|k| k.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        let platforms: Vec<String> = root["platforms"]
            .as_vec()
            .map(|s| {
                s.iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        if root["id"].as_str().is_none() {
            return Err("base manifest is missing `id`".into());
        }
        if root["catalog"].as_str().is_none() {
            return Err("base manifest is missing `catalog`".into());
        }
        Ok(Base {
            id: Self::text(&root, "id"),
            name: Self::text(&root, "name"),
            description: Self::text(&root, "description").into(),
            gpui_crate: Self::text(&gpui, "crate"),
            gpui_version: Self::text(&gpui, "version"),
            library_crate: Self::text(&library, "crate"),
            library_version: Self::text(&library, "version"),
            toolchain: Self::text(&root, "toolchain"),
            platforms,
            wasm: root["wasm"].as_bool().unwrap_or(false),
            theme: Self::text(&root, "theme"),
            catalog: Self::text(&root, "catalog"),
            license: Self::text(&root, "license").into(),
            libraries: libs,
        })
    }
}
