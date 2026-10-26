use forge_spec::catalog::{self, Catalog};
use forge_spec::validate::validate;
use forge_spec::yaml;

const GPUI_KIT: &str = include_str!("../../../catalogs/gpui-kit.catalog.yaml");
const SPEC: &str = include_str!("../../../docs/spec/forge-yaml.md");
const ELY: &str = "catalog: \"0.1\"
library: ely-gpui-components
version: 0.4.0
bases:
  gpui-kit: \"^0.8\"
elements:
  Badge:
    content: label
    props:
      label: string
  Group:
    children: [Badge, gpui-kit.Text]
  Stepper:
    props:
      value: number
    events:
      change: number
";
const BEZEL_ONLY: &str = "catalog: \"0.1\"
library: bezel-chart
version: 0.1.0
bases:
  bezel: \"^0.1\"
elements:
  Chart:
";

fn catalogs() -> Vec<Catalog> {
    [GPUI_KIT, ELY, BEZEL_ONLY]
        .iter()
        .map(|src| catalog::from_yaml(src).unwrap_or_else(|e| panic!("{e}")))
        .collect()
}

fn check(src: &str) -> Vec<String> {
    let (doc, map) = yaml::from_str_with_source_map(src).unwrap_or_else(|e| panic!("{e}"));
    validate(&doc, "gpui-kit", &catalogs(), Some(&map))
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn doc(body: &str) -> String {
    format!("forge: \"0.1\"\nuse:\n  ely: ely-gpui-components\nroot:\n{body}")
}

/// Asserts exactly one error starting with `expected`, or none when `expected` is empty.
fn assert_one(body: &str, expected: &str) {
    let errors = check(&doc(body));
    if expected.is_empty() {
        assert!(errors.is_empty(), "{errors:#?}");
        return;
    }
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert!(errors[0].starts_with(expected), "{errors:#?}");
}

#[test]
fn spec_examples_are_valid() {
    for block in SPEC.replace("\r\n", "\n").split("```yaml\n").skip(1) {
        let src = block.split("```").next().unwrap();
        assert_eq!(check(src), Vec::<String>::new(), "\n{src}");
    }
}

#[test]
fn unknown_element() {
    assert_one(
        "  Buton: Save\n",
        "5:3: `Buton` is not an element of `gpui-kit`",
    );
    assert_one(
        "  ely.Card:\n",
        "5:3: `Card` is not an element of `ely-gpui-components`",
    );
}

#[test]
fn unknown_alias() {
    assert_one("  kit.Button: Save\n", "5:3: unknown library alias `kit`");
}

#[test]
fn unknown_prop_lists_known_props() {
    assert_one(
        "  Link: Docs\n  url: x\n",
        "6:3: `Link` has no prop `url`; known props: label, href, disabled",
    );
}

#[test]
fn prop_types() {
    assert_one(
        "  Button: Save\n  variant: huge\n",
        "6:3: `Button.variant` must be one of default,",
    );
    assert_one(
        "  Button: Save\n  disabled: \"yes\"\n",
        "6:3: `Button.disabled` must be bool",
    );
    assert_one("  Checkbox: Ok\n  checked: $form.ok\n", "");
}

#[test]
fn two_way_bindings_need_bindable_props() {
    assert_one("  Input:\n  value: bind $form.email\n", "");
    assert_one(
        "  Button:\n  label: bind $form.x\n",
        "6:3: `Button.label` does not accept two-way bindings",
    );
}

#[test]
fn content() {
    assert_one("  VStack: hi\n", "5:11: `VStack` takes no content");
    assert_one(
        "  Button: Save\n  label: Save\n",
        "5:11: `Button` sets `label` both as content and as a prop",
    );
    assert_one("  Icon:\n", "5:3: `Icon` requires `name`");
    assert_one("  Icon:\n  name: check\n", "");
}

#[test]
fn events() {
    assert_one(
        "  Button: Save\n  on hover: preview()\n",
        "6:3: `Button` has no event `hover`; known events: click",
    );
    assert_one(
        "  Text: hi\n  on click: go()\n",
        "6:3: `Text` has no event `click`; it has no events",
    );
}

#[test]
fn children() {
    assert_one(
        "  Button: Save\n  children:\n    - Text: hi\n",
        "7:7: `Button` cannot have children",
    );
    assert_one(
        "  Tabs:\n  children:\n    - Tab: One\n    - Button: Two\n",
        "8:7: `Tabs` cannot contain `Button`; allowed children: Tab",
    );
}

#[test]
fn duplicate_ids() {
    assert_one(
        "  VStack#main:\n  children:\n    - Text#main: hi\n",
        "7:7: duplicate id `main` (first used at 5:3)",
    );
    assert_one(
        "  VStack#a:\n  children:\n    - Text .b: hi\n    - Text#c: hi\n",
        "",
    );
}

#[test]
fn imports_are_checked_against_the_base() {
    let errors =
        check("forge: \"0.1\"\nuse:\n  chart: bezel-chart\n  foo: foo-lib ^1\nroot:\n  Text: hi\n");
    assert_eq!(errors.len(), 2, "{errors:#?}");
    assert!(
        errors[0].starts_with(
            "3:3: `bezel-chart` does not support the `gpui-kit` base; it supports: bezel"
        ),
        "{errors:#?}"
    );
    assert!(
        errors[1].starts_with("4:3: unknown library `foo-lib`"),
        "{errors:#?}"
    );
}

#[test]
fn imported_children_resolve_by_library() {
    assert_one(
        "  ely.Group:\n  children:\n    - ely.Badge: new\n    - Text: hi\n",
        "",
    );
    let errors = check(
        "forge: \"0.1\"\nuse:\n  ui: ely-gpui-components\nroot:\n  ui.Group:\n  children:\n    - ui.Badge: new\n    - Button: no\n",
    );
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert!(
        errors[0].starts_with("8:7: `ui.Group` cannot contain `Button`"),
        "{errors:#?}"
    );
}

#[test]
fn repeated_ids_point_at_the_first() {
    let errors = check(&doc(
        "  VStack#a:\n  children:\n    - Text#a: x\n    - Text#a: y\n",
    ));
    assert_eq!(errors.len(), 2, "{errors:#?}");
    assert!(
        errors.iter().all(|e| e.contains("(first used at 5:3)")),
        "{errors:#?}"
    );
}

#[test]
fn two_way_bindings_inside_lists() {
    assert_one(
        "  Select:\n  options: [bind $form.x]\n",
        "6:3: `Select.options` cannot contain two-way bindings inside a list",
    );
}

#[test]
fn document_base_must_match_project_base() {
    let errors = check("forge: \"0.1\"\nbase: bezel\nroot:\n  Text: hi\n");
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert!(
        errors[0]
            .starts_with("2:7: document declares base `bezel` but the project uses `gpui-kit`"),
        "{errors:#?}"
    );
}

#[test]
fn missing_base_catalog() {
    let doc = yaml::from_str("forge: \"0.1\"\nroot:\n  Text: hi\n").unwrap();
    let errors = validate(&doc, "adabraka", &catalogs(), None);
    assert_eq!(
        errors[0].to_string(),
        "1:1: no catalog installed for base `adabraka`"
    );
}
