use forge_spec::catalog::{self, Catalog, Children, PropType};
use forge_spec::{Value, yaml};

const GPUI_KIT: &str = include_str!("../../../catalogs/gpui-kit.catalog.yaml");
const SPEC: &str = include_str!("../../../docs/spec/forge-yaml.md");
const MANIFEST_SPEC: &str = include_str!("../../../docs/spec/catalog-manifest.md");

fn gpui_kit() -> Catalog {
    catalog::from_yaml(GPUI_KIT).unwrap_or_else(|e| panic!("{e}"))
}

fn errors(src: &str) -> String {
    catalog::from_yaml(src)
        .expect_err("expected errors")
        .to_string()
}

const HEADER: &str = "catalog: \"0.1\"\nlibrary: x\nversion: 1.0.0\nbases:\n  gpui-kit: \"^0.8\"\n";

#[test]
fn gpui_kit_catalog_loads() {
    let catalog = gpui_kit();
    assert_eq!(catalog.library, "gpui-kit");
    assert!(catalog.supports_base("gpui-kit"));
    assert!(!catalog.supports_base("bezel"));

    let button = catalog.element("Button").unwrap();
    assert_eq!(button.content.as_deref(), Some("label"));
    assert_eq!(
        button.prop("disabled").unwrap().default,
        Some(Value::Bool(false))
    );
    assert!(button.event("click").is_some());
    assert!(
        catalog
            .element("Input")
            .unwrap()
            .prop("value")
            .unwrap()
            .bindable
    );
    assert_eq!(
        catalog.element("Tabs").unwrap().children,
        Children::Only(vec!["Tab".into()])
    );
    assert_eq!(
        catalog
            .element("Select")
            .unwrap()
            .prop("options")
            .unwrap()
            .ty,
        PropType::List(Box::new(PropType::String))
    );
}

#[test]
fn spec_examples_use_cataloged_elements() {
    let catalog = gpui_kit();
    for block in SPEC.replace("\r\n", "\n").split("```yaml\n").skip(1) {
        let doc = yaml::from_str(block.split("```").next().unwrap()).unwrap();
        let mut stack = vec![&doc.root];
        while let Some(node) = stack.pop() {
            stack.extend(&node.children);
            if node.element.namespace.is_some() {
                continue;
            }
            let element = catalog
                .element(&node.element.name)
                .unwrap_or_else(|| panic!("`{}` is not in the gpui-kit catalog", node.element));
            for (name, value) in &node.props {
                let prop = element
                    .prop(name)
                    .unwrap_or_else(|| panic!("`{}` has no prop `{name}`", node.element));
                assert!(
                    prop.ty.accepts(value),
                    "{}.{name} = {value:?}",
                    node.element
                );
            }
        }
    }
}

#[test]
fn manifest_spec_example_loads() {
    let src = MANIFEST_SPEC.split("```yaml\n").nth(1).unwrap();
    catalog::from_yaml(src.split("```").next().unwrap()).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn prop_types_accept_values() {
    let value = |s| forge_spec::value::parse_value(s, forge_spec::value::Quoting::Plain).unwrap();
    let size = PropType::Enum(vec!["Small".into(), "Medium".into()]);
    assert!(size.accepts(&value("Small")));
    assert!(!size.accepts(&value("small")));
    assert!(!size.accepts(&value("XLarge")));
    assert!(PropType::Length.accepts(&value("12px")));
    assert!(!PropType::Number.accepts(&value("12px")));
    assert!(PropType::Bool.accepts(&value("$form.ok")));
    assert!(!PropType::Bool.accepts(&value("yes")));
    assert!(PropType::Color.accepts(&value("$theme.colors.primary")));
    assert!(PropType::String.accepts(&value("42")));
}

#[test]
fn reports_bad_manifests() {
    let e = errors(&format!(
        "{HEADER}elements:\n  Button:\n    content: text\n    props:\n      size: huge\n      variant:\n        type: enum\n      on: bool\n      disabled:\n        type: bool\n        default: nope\n"
    ));
    let lines: Vec<&str> = e.lines().collect();
    assert_eq!(lines.len(), 4, "{e}");
    assert!(lines[0].starts_with("8:14: content prop `text`"), "{e}");
    assert!(
        lines[1].starts_with("10:13: unknown prop type `huge`"),
        "{e}"
    );
    assert!(lines[2].starts_with("12:15: `enum` props need"), "{e}");
    assert!(
        lines[3].starts_with("16:18: default for `disabled` must be bool"),
        "{e}"
    );
}

#[test]
fn requires_header_keys() {
    let e = errors("elements:\n");
    assert!(e.contains("missing `catalog` key"), "{e}");
    assert!(e.contains("missing `library` key"), "{e}");
    assert!(e.contains("at least one supported base"), "{e}");
    assert!(errors(&HEADER.replace("\"0.1\"", "\"9\"")).contains("unsupported catalog format"));
}

#[test]
fn rejects_bad_names() {
    let e = errors(&format!(
        "{HEADER}elements:\n  button:\n  Tabs:\n    children: [tab]\n"
    ));
    assert!(e.contains("7:3: element names are PascalCase"), "{e}");
    assert!(e.contains("9:16: invalid element name `tab`"), "{e}");
}
