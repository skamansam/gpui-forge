//! The generated `.forge.yaml` JSON Schema against real documents: it must
//! accept valid docs and reject unknown elements, props, events and `use:`
//! libraries the way `forge check` does.

use forge_spec::base::Base;
use forge_spec::catalog::{self, Catalog};
use forge_spec::yaml_schema;
use saphyr::{LoadableYamlNode, Yaml};
use serde_json::{Value as Json, json};

// Must mirror the bundled set in `gpui-forge-runtime`'s `check::catalogs()` —
// the published schemas below are only "fresh" when generated from this list.
fn catalogs() -> Vec<Catalog> {
    [
        "gpui-kit",
        "forge",
        "ely",
        "gc",
        "gpui02",
        "adabraka",
        "bezel",
        "ratatui",
        "iced",
        "video",
        "pdf",
        "whiteboard",
        "math",
        "bezel-chart",
        "bezel-editor",
        "bezel-canvas",
        "bezel-markdown",
        "bezel-terminal",
        "bezel-icons",
        "bezel-motion",
        "ferrum",
        "rhythm",
        "symbols",
        "hig",
        "base",
        "gflow",
    ]
    .iter()
    .map(|id| {
        let src = match *id {
            "gpui-kit" => include_str!("../../../catalogs/gpui-kit.catalog.yaml"),
            "forge" => include_str!("../../../catalogs/forge.catalog.yaml"),
            "ely" => include_str!("../../../catalogs/ely.catalog.yaml"),
            "gc" => include_str!("../../../catalogs/gc.catalog.yaml"),
            "gpui02" => include_str!("../../../catalogs/gpui02.catalog.yaml"),
            "adabraka" => include_str!("../../../catalogs/adabraka.catalog.yaml"),
            "bezel" => include_str!("../../../catalogs/bezel.catalog.yaml"),
            "ratatui" => include_str!("../../../catalogs/ratatui.catalog.yaml"),
            "iced" => include_str!("../../../catalogs/iced.catalog.yaml"),
            "video" => include_str!("../../../catalogs/video.catalog.yaml"),
            "pdf" => include_str!("../../../catalogs/pdf.catalog.yaml"),
            "whiteboard" => include_str!("../../../catalogs/whiteboard.catalog.yaml"),
            "math" => include_str!("../../../catalogs/math.catalog.yaml"),
            "bezel-chart" => include_str!("../../../catalogs/bezel-chart.catalog.yaml"),
            "bezel-editor" => include_str!("../../../catalogs/bezel-editor.catalog.yaml"),
            "bezel-canvas" => include_str!("../../../catalogs/bezel-canvas.catalog.yaml"),
            "bezel-markdown" => {
                include_str!("../../../catalogs/bezel-markdown.catalog.yaml")
            }
            "bezel-terminal" => {
                include_str!("../../../catalogs/bezel-terminal.catalog.yaml")
            }
            "bezel-icons" => include_str!("../../../catalogs/bezel-icons.catalog.yaml"),
            "bezel-motion" => include_str!("../../../catalogs/bezel-motion.catalog.yaml"),
            "ferrum" => include_str!("../../../catalogs/ferrum.catalog.yaml"),
            "rhythm" => include_str!("../../../catalogs/rhythm.catalog.yaml"),
            "symbols" => include_str!("../../../catalogs/symbols.catalog.yaml"),
            "hig" => include_str!("../../../catalogs/hig.catalog.yaml"),
            "base" => include_str!("../../../catalogs/base.catalog.yaml"),
            _ => include_str!("../../../catalogs/gflow.catalog.yaml"),
        };
        catalog::from_yaml(src).unwrap_or_else(|d| panic!("catalog {id}: {d}"))
    })
    .collect()
}

fn bases() -> Vec<Base> {
    [
        include_str!("../../../bases/gpui-kit.base.yaml"),
        include_str!("../../../bases/gpui02.base.yaml"),
        include_str!("../../../bases/adabraka.base.yaml"),
        include_str!("../../../bases/bezel.base.yaml"),
        include_str!("../../../bases/ratatui.base.yaml"),
        include_str!("../../../bases/iced.base.yaml"),
    ]
    .iter()
    .map(|src| Base::from_yaml(src).expect("base manifest parses"))
    .collect()
}

fn schema(base: &str) -> Json {
    let bases = bases();
    yaml_schema::document_schema(bases.iter().find(|b| b.id == base), &bases, &catalogs())
}

fn errors(schema: &Json, doc: &Json) -> Vec<String> {
    let validator = jsonschema::validator_for(schema).expect("schema compiles");
    validator.iter_errors(doc).map(|e| e.to_string()).collect()
}

fn valid(schema: &Json, doc: Json) {
    let errs = errors(schema, &doc);
    assert!(errs.is_empty(), "expected valid, got: {errs:?}");
}

fn invalid(schema: &Json, doc: Json) {
    let errs = errors(schema, &doc);
    assert!(!errs.is_empty(), "expected errors, got none");
}

/// YAML → JSON for validation — mirrors what an editor's YAML parser yields.
fn to_json(y: &Yaml) -> Json {
    if let Some(map) = y.as_mapping() {
        Json::Object(
            map.iter()
                .filter_map(|(k, v)| k.as_str().map(|k| (k.to_string(), to_json(v))))
                .collect(),
        )
    } else if let Some(seq) = y.as_vec() {
        Json::Array(seq.iter().map(to_json).collect())
    } else if let Some(b) = y.as_bool() {
        json!(b)
    } else if let Some(i) = y.as_integer() {
        json!(i)
    } else if let Some(f) = y.as_floating_point() {
        json!(f)
    } else {
        y.as_str().map_or(Json::Null, |s| json!(s))
    }
}

fn yaml_doc(src: &str) -> Json {
    let docs = Yaml::load_from_str(src).expect("yaml parses");
    to_json(&docs[0])
}

#[test]
fn document_level() {
    let s = schema("gpui-kit");
    valid(&s, json!({"forge": "0.1", "root": {"Text": "hi"}}));
    invalid(&s, json!({"forge": "0.2", "root": {"Text": "hi"}}));
    invalid(&s, json!({"forge": "0.1"}));
    invalid(
        &s,
        json!({"forge": "0.1", "base": "ratatui", "root": {"Text": "hi"}}),
    );
}

#[test]
fn unknown_element_and_props() {
    let s = schema("gpui-kit");
    valid(
        &s,
        json!({"forge": "0.1", "root": {"Button": "OK", "variant": "Primary", "on click": "go()"}}),
    );
    invalid(&s, json!({"forge": "0.1", "root": {"Butto": "OK"}}));
    invalid(
        &s,
        json!({"forge": "0.1", "root": {"Button": "OK", "variat": "Primary"}}),
    );
    invalid(
        &s,
        json!({"forge": "0.1", "root": {"Button": "OK", "variant": "Nope"}}),
    );
    invalid(
        &s,
        json!({"forge": "0.1", "root": {"Button": "OK", "on hover": "x()"}}),
    );
    invalid(
        &s,
        json!({"forge": "0.1", "root": {"Button": "OK", "children": [{"Text": "x"}]}}),
    );
}

#[test]
fn use_imports_and_namespaced_elements() {
    let s = schema("gpui-kit");
    valid(
        &s,
        json!({
            "forge": "0.1",
            "use": {"ely": "ely-core ^0.4"},
            "root": {"ely.Button": "Hi", "variant": "Primary"},
        }),
    );
    invalid(
        &s,
        json!({"forge": "0.1", "use": {"ely": "no-such-lib"}, "root": {"Text": "x"}}),
    );
    invalid(
        &s,
        json!({"forge": "0.1", "use": {"Ely": "ely-core"}, "root": {"Text": "x"}}),
    );
    invalid(&s, json!({"forge": "0.1", "root": {"ely.NotAWidget": "x"}}));
    invalid(
        &s,
        json!({"forge": "0.1", "root": {"ely.Button": "x", "variant": "Nope"}}),
    );
}

#[test]
fn style_map() {
    let s = schema("gpui-kit");
    valid(
        &s,
        json!({"forge": "0.1", "root": {"VStack": null, "style": {"gap": "12px", "align": "center"}}}),
    );
    invalid(
        &s,
        json!({"forge": "0.1", "root": {"VStack": null, "style": {"gpa": "12px"}}}),
    );
}

#[test]
fn ids_and_content() {
    let s = schema("gpui-kit");
    valid(
        &s,
        json!({"forge": "0.1", "root": {"Input#email": null, "value": "bind $form.email"}}),
    );
    valid(&s, json!({"forge": "0.1", "root": {"Input .email": null}}));
}

#[test]
fn shipped_docs_validate() {
    for (name, src, base) in [
        (
            "blank",
            include_str!("../../forge-studio/blank.forge.yaml"),
            "gpui-kit",
        ),
        (
            "welcome",
            include_str!("../../forge-studio/welcome.forge.yaml"),
            "gpui-kit",
        ),
        (
            "studio",
            include_str!("../../forge-studio/studio.forge.yaml"),
            "gpui-kit",
        ),
        (
            "gallery",
            include_str!("../../forge-studio/gallery.forge.yaml"),
            "gpui-kit",
        ),
        (
            "gallery-forge",
            include_str!("../../forge-studio/gallery.forge.forge.yaml"),
            "gpui-kit",
        ),
        (
            "ratatui-showcase",
            include_str!("../../../examples/ratatui-showcase.forge.yaml"),
            "ratatui",
        ),
        (
            "ratatui-dashboard",
            include_str!("../../../examples/ratatui-dashboard.forge.yaml"),
            "ratatui",
        ),
        (
            "ratatui-forms",
            include_str!("../../../examples/ratatui-forms.forge.yaml"),
            "ratatui",
        ),
        (
            "base-demo",
            include_str!("../../../examples/base-demo.forge.yaml"),
            "gpui-kit",
        ),
        (
            "gflow-demo",
            include_str!("../../../examples/gflow-demo.forge.yaml"),
            "gpui-kit",
        ),
        (
            "hig-demo",
            include_str!("../../../examples/hig-demo.forge.yaml"),
            "gpui-kit",
        ),
        (
            "math-demo",
            include_str!("../../../examples/math-demo.forge.yaml"),
            "gpui-kit",
        ),
        (
            "pdf-demo",
            include_str!("../../../examples/pdf-demo.forge.yaml"),
            "gpui-kit",
        ),
        (
            "video-demo",
            include_str!("../../../examples/video-demo.forge.yaml"),
            "gpui-kit",
        ),
        (
            "whiteboard-demo",
            include_str!("../../../examples/whiteboard-demo.forge.yaml"),
            "gpui-kit",
        ),
        (
            "bezel-showcase",
            include_str!("../../../examples/bezel-showcase.forge.yaml"),
            "bezel",
        ),
        (
            "adabraka-showcase",
            include_str!("../../../examples/adabraka-showcase.forge.yaml"),
            "adabraka",
        ),
        (
            "ely-demo",
            include_str!("../../../examples/ely-demo.forge.yaml"),
            "gpui-kit",
        ),
        (
            "gc-showcase",
            include_str!("../../../examples/gc-showcase.forge.yaml"),
            "gpui-kit",
        ),
        (
            "gpui-kit-showcase",
            include_str!("../../../examples/gpui-kit-showcase.forge.yaml"),
            "gpui-kit",
        ),
        (
            "gpui02-showcase",
            include_str!("../../../examples/gpui02-showcase.forge.yaml"),
            "gpui02",
        ),
        (
            "iced-showcase",
            include_str!("../../../examples/iced-showcase.forge.yaml"),
            "iced",
        ),
    ] {
        let errs = errors(&schema(base), &yaml_doc(src));
        assert!(errs.is_empty(), "{name} ({base}): {errs:#?}");
    }
}

/// The committed schemas editors load must match generation output — run
/// `cargo xtask schemas` after touching catalogs, bases or `yaml_schema`.
#[test]
fn published_schemas_are_fresh() {
    let bases = bases();
    let catalogs = catalogs();
    for (base, file) in [
        (
            "adabraka",
            include_str!("../../../site/static/schema/0.1/adabraka.json"),
        ),
        (
            "bezel",
            include_str!("../../../site/static/schema/0.1/bezel.json"),
        ),
        (
            "gpui-kit",
            include_str!("../../../site/static/schema/0.1/gpui-kit.json"),
        ),
        (
            "gpui02",
            include_str!("../../../site/static/schema/0.1/gpui02.json"),
        ),
        (
            "iced",
            include_str!("../../../site/static/schema/0.1/iced.json"),
        ),
        (
            "ratatui",
            include_str!("../../../site/static/schema/0.1/ratatui.json"),
        ),
    ] {
        let expected = yaml_schema::to_json(&yaml_schema::document_schema(
            bases.iter().find(|b| b.id == base),
            &bases,
            &catalogs,
        ));
        assert_eq!(
            file, expected,
            "{base}.json is stale — run `cargo xtask schemas`"
        );
    }
    let expected = yaml_schema::to_json(&yaml_schema::document_schema(None, &bases, &catalogs));
    assert_eq!(
        include_str!("../../../site/static/schema/0.1/generic.json"),
        expected,
        "generic.json is stale — run `cargo xtask schemas`"
    );
}
