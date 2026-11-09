//! JSON Schema for the `.forge.yaml` surface syntax — generated from the
//! catalog manifests so schema-aware editors (yaml-language-server in
//! VS Code/Zed, JetBrains schema mappings) can flag unknown elements, props,
//! events, style keys and `use:` libraries without running `forge check`.
//!
//! `forge schema --yaml [--base <id>]` prints one; `cargo xtask schemas`
//! regenerates the set published under `site/static/schema/0.1/`.
//!
//! The emitted schema targets draft-07 — the dialect yaml-language-server and
//! JetBrains fully evaluate (`propertyNames`, `if`/`then`, `patternProperties`).

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value as Json, json};

use crate::SPEC_VERSION;
use crate::base::Base;
use crate::catalog::{Catalog, Children, ElementDef, PropType};
use crate::style::{self, StyleType};

/// Published base URL — the docs site serves `site/static/` verbatim.
pub const SCHEMA_URL: &str = "https://gpui-forge.rudeboy.dev/schema";

/// The published URL of a generated schema — `<base-id>.json` or `generic.json`.
pub fn schema_url(slug: &str) -> String {
    format!("{SCHEMA_URL}/{SPEC_VERSION}/{slug}.json")
}

const DRAFT: &str = "http://json-schema.org/draft-07/schema#";

/// Canonical pretty-printing — sorts object keys recursively so committed
/// schemas are byte-stable no matter which `serde_json` feature flags a
/// consumer's build resolves (`preserve_order` changes `Map`'s ordering).
pub fn to_json(schema: &Json) -> String {
    fn sorted(value: &Json) -> Json {
        match value {
            Json::Object(map) => {
                let mut entries: Vec<_> = map.iter().map(|(k, v)| (k.clone(), sorted(v))).collect();
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                Json::Object(entries.into_iter().collect())
            }
            Json::Array(items) => Json::Array(items.iter().map(sorted).collect()),
            other => other.clone(),
        }
    }
    let mut out = serde_json::to_string_pretty(&sorted(schema)).unwrap_or_default();
    out.push('\n');
    out
}

/// Optional `#id` / ` .id` suffix on an element key (`docs/format/forge-yaml.md`).
const ID_SUFFIX: &str = r"(#[\w-]+| \.[\w-]+)?";
/// An import alias — `is_namespace` in the reader.
const ALIAS: &str = r"[a-z][\w-]*";
/// A prop-style key — `is_prop_key` in the reader.
const PROP_KEY: &str = r"[a-z_][A-Za-z0-9_-]*";
/// `on <event>` keys.
const EVENT_KEY: &str = r"on [a-z_][A-Za-z0-9_-]*";
/// Keys a node may carry that never name an element.
const NON_ELEMENT_KEY: &str = r"([a-z_][A-Za-z0-9_-]*|on [a-z_][A-Za-z0-9_-]*|<<)";
/// A scalar holding a runtime value — `$path`, `{$path}` or `bind $path`.
const BOUND: &str = r"^(\$|\{)";
/// A scalar holding a runtime value or a two-way binding.
const BOUND_OR_BIND: &str = r"^(\$|\{|bind\s)";

/// `a|b|c` — longest first so alternation order never shadows a longer name.
fn alt<'a>(names: impl IntoIterator<Item = &'a str>) -> String {
    let mut names: Vec<&str> = names.into_iter().collect();
    names.sort_by(|a, b| b.len().cmp(&a.len()).then(a.cmp(b)));
    names.dedup();
    names.join("|")
}

/// The JSON Schema for one `.forge.yaml` document.
///
/// `base: Some` scopes element/prop/event/`use` validation to that base's
/// catalog plus the libraries its manifest registers; `base: None` emits the
/// generic schema that accepts every catalog's elements, for editors that map
/// `*.forge.yaml` without knowing a document's base.
pub fn document_schema(base: Option<&Base>, bases: &[Base], catalogs: &[Catalog]) -> Json {
    let base_catalog = base.and_then(|b| {
        catalogs
            .iter()
            .find(|c| c.library == b.catalog || c.library == b.id)
    });
    // Catalogs importable via `use:` — those declaring support for the base;
    // the base's own catalog joins so `use: kit: gpui-kit` aliases work.
    let importable: Vec<&Catalog> = match base {
        Some(b) => catalogs
            .iter()
            .filter(|c| {
                c.supports_base(&b.id)
                    || Some(c.library.as_str()) == base_catalog.map(|c| c.library.as_str())
            })
            .collect(),
        None => catalogs.iter().collect(),
    };
    // Library ids `use:` values may name — catalogs plus adapters the base
    // manifest registers without shipping a catalog.
    let mut libraries: BTreeSet<&str> = importable.iter().map(|c| c.library.as_str()).collect();
    if let Some(b) = base {
        libraries.extend(b.libraries.iter().map(String::as_str));
    } else {
        libraries.extend(
            bases
                .iter()
                .flat_map(|b| b.libraries.iter().map(String::as_str)),
        );
    }

    // Element names usable unqualified (the base's own) and namespaced
    // (`alias.Element` — the alias is user-chosen, so only the name is known).
    let own: Vec<&str> = base_catalog
        .map(|c| c.elements.iter().map(|e| e.name.as_str()).collect())
        .unwrap_or_else(|| {
            importable
                .iter()
                .flat_map(|c| c.elements.iter().map(|e| e.name.as_str()))
                .collect()
        });
    let namespaced: BTreeSet<&str> = importable
        .iter()
        .flat_map(|c| c.elements.iter().map(|e| e.name.as_str()))
        .collect();

    let mut guards = Vec::new();
    // Unqualified elements: one guard per element when a base is selected;
    // otherwise merge same-named elements across catalogs (the generic
    // schema can't know which library an unqualified name means).
    if let Some(catalog) = base_catalog {
        for def in &catalog.elements {
            guards.push(element_guard(&def.name, std::slice::from_ref(&def)));
        }
    }
    let mut by_name: BTreeMap<&str, Vec<&ElementDef>> = BTreeMap::new();
    for def in importable.iter().flat_map(|c| &c.elements) {
        by_name.entry(def.name.as_str()).or_default().push(def);
    }
    for (name, defs) in &by_name {
        if base_catalog.is_none() {
            guards.push(element_guard(name, defs));
        }
        guards.push(element_guard(&format!("{ALIAS}\\.{name}"), defs));
    }
    // `Dialog#name:` declarations resolve to the base's `Dialog` element —
    // merged across catalogs for the generic schema.
    let dialog_defs: Vec<&ElementDef> = match base_catalog {
        Some(catalog) => catalog.element("Dialog").into_iter().collect(),
        None => by_name.get("Dialog").cloned().unwrap_or_default(),
    };

    let slug = base.map_or("generic", |b| b.id.as_str());
    let title = base.map_or_else(
        || "Forge document (any base)".to_string(),
        |b| format!("Forge document ({})", b.id),
    );
    let mut node_alternatives = vec![
        format!("({}){ID_SUFFIX}", alt(own.iter().copied())),
        EVENT_KEY.to_string(),
        PROP_KEY.to_string(),
        "<<".to_string(),
    ];
    if !namespaced.is_empty() {
        node_alternatives.insert(
            1,
            format!("{ALIAS}\\.({}){ID_SUFFIX}", alt(namespaced.iter().copied())),
        );
    }
    let node = json!({
        "type": "object",
        "minProperties": 1,
        "description": "One element key plus props, `style`, `on <event>` and `children`.",
        "propertyNames": { "pattern": format!("^({})$", node_alternatives.join("|")) },
        // At least one element key: fail when every key is prop/event-shaped.
        "not": { "propertyNames": { "pattern": format!("^{NON_ELEMENT_KEY}$") } },
        "properties": {
            "style": { "$ref": "#/definitions/style" },
            "children": { "type": "array", "items": { "$ref": "#/definitions/node" } },
            "tooltip": {
                "type": ["string", "number", "boolean", "null"],
                "description": "Universal prop — hover tooltip text.",
            },
        },
        "allOf": guards,
    });

    let base_prop = match base {
        Some(b) => json!({
            "const": b.id,
            "description": "Base component system — fixed for this schema.",
        }),
        None => json!({
            "enum": bases.iter().map(|b| b.id.as_str()).collect::<Vec<_>>(),
            "description": "Base component system; inherited from `forge.toml` when absent.",
        }),
    };
    json!({
        "$schema": DRAFT,
        "$id": schema_url(slug),
        "title": title,
        "type": "object",
        "required": ["forge", "root"],
        "properties": {
            "$schema": {
                "type": "string",
                "description": "JSON Schema URL for editors (`$schema`).",
            },
            "forge": {
                "const": SPEC_VERSION,
                "description": "Forge spec version — quote it: `forge: \"0.1\"`.",
            },
            "base": base_prop,
            "use": {
                "type": "object",
                "description": "Import alias → `<library-id> [<version-req>]`.",
                "propertyNames": { "pattern": format!("^{ALIAS}$") },
                "additionalProperties": {
                    "type": "string",
                    "pattern": format!("^({})(\\s+\\S+)?$", alt(libraries.iter().copied())),
                },
            },
            "meta": {
                "type": "object",
                "additionalProperties": { "type": ["string", "number", "boolean", "null"] },
            },
            "state": {
                "type": "object",
                "description": "Declared state vars — `path: type` (e.g. `slider: number`).",
                "additionalProperties": { "type": ["string", "number", "boolean", "null"] },
            },
            "root": { "$ref": "#/definitions/node" },
        },
        // `Dialog#name:` declarations; other unknown top-level keys are
        // fragment slots for `&anchor` declarations, so they stay allowed.
        "patternProperties": {
            // `Dialog#name:`, `Dialog.name:` and `Dialog .name:` are all
            // declarations — the reader accepts each spelling.
            "^Dialog((#| ?\\.)[\\w-]+)?$": dialog_body_schema(&dialog_defs),
        },
        "additionalProperties": true,
        "definitions": {
            "node": node,
            "style": style_schema(),
        },
    })
}

/// An element's catalog contract, unioned across same-named defs: a prop or
/// event is allowed when any def declares it and required only when every
/// def does (defs lacking it can't produce the element anyway).
struct Contract<'a> {
    /// prop name → one value schema per declaring def.
    props: BTreeMap<&'a str, Vec<Json>>,
    prop_summaries: BTreeMap<&'a str, &'a str>,
    events: BTreeSet<&'a str>,
    event_summaries: BTreeMap<&'a str, &'a str>,
    /// Props required by every def and never fed by positional content.
    required: Vec<&'a str>,
    /// One content schema per def — `null` for defs taking none.
    content: Vec<Json>,
    children: Vec<&'a Children>,
}

fn contract<'a>(defs: &[&'a ElementDef]) -> Contract<'a> {
    let mut props: BTreeMap<&str, Vec<Json>> = BTreeMap::new();
    let mut prop_summaries: BTreeMap<&str, &str> = BTreeMap::new();
    let mut events: BTreeSet<&str> = BTreeSet::new();
    let mut event_summaries: BTreeMap<&str, &str> = BTreeMap::new();
    let mut required_counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut content = Vec::new();
    let mut content_props: BTreeSet<&str> = BTreeSet::new();
    let mut children = Vec::new();

    for def in defs {
        for prop in &def.props {
            props
                .entry(prop.name.as_str())
                .or_default()
                .push(prop_schema(&prop.ty, prop.bindable));
            if let Some(s) = &prop.summary {
                prop_summaries.insert(prop.name.as_str(), s);
            }
            // Content satisfies the prop it feeds — never require the key.
            if prop.required && Some(prop.name.as_str()) != def.content.as_deref() {
                *required_counts.entry(prop.name.as_str()).or_default() += 1;
            }
        }
        for event in &def.events {
            events.insert(event.name.as_str());
            if let Some(s) = &event.summary {
                event_summaries.insert(event.name.as_str(), s);
            }
        }
        if let Some(name) = &def.content {
            content_props.insert(name.as_str());
            match def.prop(name) {
                Some(prop) => content.push(prop_schema(&prop.ty, false)),
                None => content.push(json!({ "type": ["string", "number", "boolean", "null"] })),
            }
        } else {
            content.push(json!({ "type": "null" }));
        }
        children.push(&def.children);
    }

    Contract {
        props,
        prop_summaries,
        events,
        event_summaries,
        required: required_counts
            .iter()
            .filter(|(name, count)| **count == defs.len() && !content_props.contains(*name))
            .map(|(name, _)| *name)
            .collect(),
        content,
        children,
    }
}

impl Contract<'_> {
    /// `properties` for the props, `tooltip`, `style`, `children` and literal
    /// `on <event>` keys this contract allows.
    fn properties(&self) -> Map<String, Json> {
        let mut properties = Map::new();
        for (name, schemas) in &self.props {
            let mut schema = if schemas.len() == 1 {
                schemas[0].clone()
            } else {
                json!({ "anyOf": schemas })
            };
            if let Some(summary) = self.prop_summaries.get(name) {
                schema["description"] = json!(summary);
            }
            properties.insert((*name).to_string(), schema);
        }
        properties.insert(
            "tooltip".into(),
            json!({
                "type": ["string", "number", "boolean", "null"],
                "description": "Universal prop — hover tooltip text.",
            }),
        );
        properties.insert("style".into(), json!({ "$ref": "#/definitions/style" }));
        properties.insert("children".into(), children_schema(&self.children));
        for event in &self.events {
            let mut schema = json!({ "type": ["string", "null"] });
            if let Some(summary) = self.event_summaries.get(event) {
                schema["description"] = json!(summary);
            }
            properties.insert(format!("on {event}"), schema);
        }
        properties
    }

    /// Every key name this contract allows besides the element key itself.
    fn allowed_keys(&self) -> BTreeSet<String> {
        let mut keys: BTreeSet<String> = [
            "tooltip".to_string(),
            "style".to_string(),
            "children".to_string(),
            "<<".to_string(),
        ]
        .into();
        keys.extend(self.props.keys().map(|k| (*k).to_string()));
        if !self.events.is_empty() {
            keys.insert(format!("on ({})", alt(self.events.iter().copied())));
        }
        keys
    }
}

/// One `allOf` member scoping a node's keys to an element's catalog contract.
///
/// `key_pat` matches the element key — `Button` for base elements,
/// `[a-z][\w-]*\.Button` for imported ones.
fn element_guard(key_pat: &str, defs: &[&ElementDef]) -> Json {
    let contract = contract(defs);
    let mut alternatives = contract.allowed_keys();
    alternatives.insert(format!("{key_pat}{ID_SUFFIX}"));

    let mut then = Map::new();
    then.insert(
        "propertyNames".into(),
        json!({ "pattern": format!("^({})$", alternatives.into_iter().collect::<Vec<_>>().join("|")) }),
    );
    then.insert("properties".into(), contract.properties().into());
    then.insert(
        "patternProperties".into(),
        json!({
            format!("^{key_pat}{ID_SUFFIX}$"): {
                "anyOf": contract.content,
                "description": "Positional content — mapped to the element's primary prop.",
            },
        }),
    );
    if !contract.required.is_empty() {
        then.insert("required".into(), json!(contract.required));
    }

    // JSON Schema can't say "has a key matching this pattern", so flip it:
    // `if` holds when not every key avoids the element pattern — i.e. some
    // key is `Button`/`Button#id`/`alias.Button`.
    json!({
        "if": {
            "not": {
                "propertyNames": { "pattern": format!("^(?!{key_pat}{ID_SUFFIX}$).*$") },
            },
        },
        "then": then,
    })
}

/// The value of a top-level `Dialog#name:` — the reader synthesizes the
/// `Dialog` element key, so the body carries only props, `style`, `on`
/// handlers and `children` of the `Dialog` element (no element key of its
/// own — a second one is a parse error).
fn dialog_body_schema(defs: &[&ElementDef]) -> Json {
    if defs.is_empty() {
        return json!({
            "type": "object",
            "propertyNames": { "pattern": format!("^{NON_ELEMENT_KEY}$") },
        });
    }
    let contract = contract(defs);
    let alternatives = contract.allowed_keys();
    let mut body = Map::new();
    body.insert("type".into(), json!("object"));
    body.insert(
        "propertyNames".into(),
        json!({ "pattern": format!("^({})$", alternatives.into_iter().collect::<Vec<_>>().join("|")) }),
    );
    body.insert("properties".into(), contract.properties().into());
    if !contract.required.is_empty() {
        body.insert("required".into(), json!(contract.required));
    }
    body.into()
}

/// `children` for a set of same-named defs — unioned so any `any` wins and
/// `Only` lists merge.
fn children_schema(defs: &[&Children]) -> Json {
    if defs.iter().any(|c| matches!(c, Children::Any)) {
        return json!({
            "type": "array",
            "items": { "$ref": "#/definitions/node" },
        });
    }
    if defs.iter().all(|c| matches!(c, Children::None)) {
        return json!({ "type": "array", "maxItems": 0 });
    }
    let mut allowed = Vec::new();
    for entry in defs.iter().flat_map(|c| match c {
        Children::Only(names) => names.iter().map(String::as_str).collect::<Vec<_>>(),
        _ => Vec::new(),
    }) {
        // `lib.Element` entries constrain the library, but the alias is the
        // document's choice — accept any alias prefix.
        allowed.push(match entry.rsplit_once('.') {
            Some((_, name)) => format!("{ALIAS}\\.{name}"),
            None => format!("({ALIAS}\\.)?{entry}"),
        });
    }
    json!({
        "type": "array",
        "items": {
            "allOf": [
                { "$ref": "#/definitions/node" },
                {
                    "propertyNames": {
                        "pattern": format!(
                            "^(({}){ID_SUFFIX}|{NON_ELEMENT_KEY})$",
                            allowed.join("|"),
                        ),
                    },
                },
            ],
        },
    })
}

/// A prop value schema from its catalog `PropType`. Bindings are strings at
/// YAML level (`$path`, `bind $path`), so most types accept a string spelling
/// a binding; `bindable` controls whether two-way `bind` is allowed.
fn prop_schema(ty: &PropType, bindable: bool) -> Json {
    let bound = if bindable { BOUND_OR_BIND } else { BOUND };
    let dynamic = json!({ "type": "string", "pattern": bound });
    match ty {
        PropType::String => json!({ "type": ["string", "number", "boolean", "null"] }),
        PropType::Number => json!({ "anyOf": [{ "type": "number" }, dynamic, { "type": "null" }] }),
        PropType::Length => json!({ "type": ["number", "string", "null"] }),
        PropType::Bool => {
            json!({ "anyOf": [{ "type": "boolean" }, { "type": "string" }, { "type": "null" }] })
        }
        PropType::Color | PropType::Icon => json!({ "type": ["string", "null"] }),
        PropType::Enum(values) => json!({
            "anyOf": [
                { "enum": values },
                dynamic,
                { "type": "null" },
            ],
        }),
        // Two-way bindings can't sit inside a list — items never `bind`.
        PropType::List(item) => json!({
            "anyOf": [
                { "type": "array", "items": prop_schema(item, false) },
                dynamic,
                { "type": "null" },
            ],
        }),
        PropType::Map => json!({ "anyOf": [{ "type": "object" }, dynamic, { "type": "null" }] }),
    }
}

/// The `style:` map schema — every key `style::KEYS` knows, typed per StyleType.
fn style_schema() -> Json {
    let mut properties = Map::new();
    for (key, ty) in style::KEYS {
        properties.insert((*key).to_string(), style_value_schema(*ty));
    }
    json!({
        "type": "object",
        "properties": properties,
        "additionalProperties": false,
    })
}

fn style_value_schema(ty: StyleType) -> Json {
    match ty {
        StyleType::Length => json!({ "type": ["number", "string", "null"] }),
        StyleType::Size => json!({
            "anyOf": [
                { "enum": ["auto", "full"] },
                { "type": ["number", "string", "null"] },
            ],
        }),
        StyleType::Edges => json!({
            "anyOf": [
                {
                    "type": "array",
                    "items": { "type": ["number", "string"] },
                    "minItems": 1,
                    "maxItems": 4,
                },
                { "type": ["number", "string", "null"] },
            ],
        }),
        StyleType::Color => json!({
            "type": ["string", "null"],
            "description": "A quoted color (`\"#3b82f6\"`) or `$theme.<field>` — unquoted `#` starts a comment.",
        }),
        StyleType::Bool => json!({ "type": ["boolean", "string", "null"] }),
        StyleType::Factor => json!({ "type": ["number", "boolean", "string", "null"] }),
        StyleType::Fraction => json!({
            "anyOf": [
                { "type": "number", "minimum": 0, "maximum": 1 },
                { "type": ["string", "null"] },
            ],
        }),
        StyleType::Keyword(words) => json!({
            "anyOf": [
                { "enum": words },
                { "type": ["string", "null"] },
            ],
        }),
        StyleType::LengthOrKeyword(words) => json!({
            "anyOf": [
                { "enum": words },
                { "type": ["number", "string", "null"] },
            ],
        }),
    }
}
