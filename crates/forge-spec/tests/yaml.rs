use forge_spec::yaml::{self, IdStyle, WriteOptions};
use forge_spec::{BindingMode, Document, ElementRef, Path, PathSegment, Segment, Unit, Value};

const SPEC: &str = include_str!("../../../docs/spec/forge-yaml.md");

fn yaml_blocks(md: &str) -> Vec<&str> {
    md.split("```yaml\n")
        .skip(1)
        .map(|block| block.split("```").next().unwrap())
        .collect()
}

fn read(src: &str) -> Document {
    yaml::from_str(src).unwrap_or_else(|e| panic!("{e}\n---\n{src}"))
}

fn write(doc: &Document) -> String {
    yaml::to_string(doc, &WriteOptions::default())
}

fn errors(src: &str) -> String {
    yaml::from_str(src)
        .expect_err("expected errors")
        .to_string()
}

fn path(fields: &[&str]) -> Path {
    Path(
        fields
            .iter()
            .map(|f| PathSegment::Field((*f).into()))
            .collect(),
    )
}

#[test]
fn spec_examples_read_and_round_trip() {
    let blocks = yaml_blocks(SPEC);
    assert!(blocks.len() >= 3);
    for src in blocks {
        let doc = read(src);
        let out = write(&doc);
        assert_eq!(read(&out), doc, "round trip changed the model:\n{out}");
        assert_eq!(write(&read(&out)), out, "writer is not idempotent");
    }
}

#[test]
fn reads_the_login_example() {
    let doc = read(yaml_blocks(SPEC)[0]);
    assert_eq!(doc.version, "0.1");
    assert_eq!(doc.base.as_deref(), Some("gpui-kit"));
    assert_eq!(doc.imports[0].alias, "ely");
    assert_eq!(doc.imports[0].library, "ely-gpui-components");
    assert_eq!(doc.imports[0].version.as_deref(), Some("^0.4"));
    assert_eq!(doc.meta, vec![("title".into(), "Login".into())]);

    let root = &doc.root;
    assert_eq!(root.element.name, "VStack");
    assert_eq!(root.id.as_deref(), Some("root"));
    assert_eq!(
        root.style[0],
        (
            "gap".into(),
            Value::Number {
                value: 12.0,
                unit: Some(Unit::Px)
            }
        )
    );
    assert_eq!(root.children.len(), 4);

    let [text, input, button, badge] = &root.children[..] else {
        unreachable!()
    };
    assert_eq!(
        text.content,
        Some(Value::Interpolated(vec![
            Segment::Text("Welcome back, ".into()),
            Segment::Binding(path(&["user", "name"])),
        ]))
    );
    assert_eq!(input.id.as_deref(), Some("email"));
    assert_eq!(
        input.props[0],
        (
            "value".into(),
            Value::Binding {
                path: path(&["form", "email"]),
                mode: BindingMode::TwoWay
            }
        )
    );
    assert_eq!(button.content, Some(Value::String("Sign in".into())));
    assert_eq!(button.events[0].event, "click");
    assert_eq!(button.events[0].handler.name, "sign_in");
    assert_eq!(
        badge.element,
        ElementRef {
            namespace: Some("ely".into()),
            name: "Badge".into()
        }
    );
    assert_eq!(badge.content, Some(Value::Enum("beta".into())));
}

#[test]
fn canonical_output_is_stable() {
    const CANONICAL: &str = r##"$schema: https://gpui-forge.rudeboy.dev/schema/0.1/gpui-kit.json
forge: "0.1"
base: gpui-kit
use:
  ely: ely-gpui-components ^0.4
meta:
  title: 'Login: step 1'

root:
  VStack#root:
  style:
    gap: 12px
    background: "#1e293b80"
    color: $theme.colors.text
  children:
    - Text: "Welcome back, {$user.name} {{beta}"
    - Input#email:
      value: bind $form.email
      placeholder: Email
      label: 'true'
    - Button: Sign in
      variant: primary
      disabled: $form.invalid
      on click: sign_in()
    - Select:
      options: [light, dark, system default, '42']
      on change: set_theme($settings.id, 2, dark mode)
    - List:
      items:
        - $rows[0]
        - $rows[1]
    - ely.Badge: beta
"##;
    assert_eq!(write(&read(CANONICAL)), CANONICAL);
}

#[test]
fn both_id_styles() {
    let doc = read("forge: \"0.1\"\nroot:\n  Input .email:\n");
    assert_eq!(doc.root.id.as_deref(), Some("email"));
    assert_eq!(doc, read("forge: \"0.1\"\nroot:\n  Input#email:\n"));
    let dot = yaml::to_string(
        &doc,
        &WriteOptions {
            id_style: IdStyle::Dot,
        },
    );
    assert!(dot.contains("  Input .email:\n"), "{dot}");
    assert_eq!(read(&dot), doc);
}

#[test]
fn two_element_keys() {
    let e = errors("forge: \"0.1\"\nroot:\n  Text: hi\n  Button: ok\n");
    assert!(
        e.contains("4:3: node has two element keys (`Text`, `Button`)"),
        "{e}"
    );
}

#[test]
fn unquoted_color_gets_a_hint() {
    let e = errors("forge: \"0.1\"\nroot:\n  Box:\n  style:\n    color: #fff\n");
    assert!(e.contains("5:5: `color` has no value; quote colors"), "{e}");
}

#[test]
fn duplicate_keys() {
    let e = errors("forge: \"0.1\"\nroot:\n  Text: a\n  size: lg\n  size: sm\n");
    assert!(
        e.contains("5:3: duplicate key `size` (first defined at 4:3)"),
        "{e}"
    );
}

#[test]
fn missing_element_key() {
    let e = errors("forge: \"0.1\"\nroot:\n  variant: primary\n");
    assert!(e.contains("3:3: node has no element key"), "{e}");
}

#[test]
fn props_nested_under_the_element_key() {
    let e = errors("forge: \"0.1\"\nroot:\n  Input#email:\n    value: x\n");
    assert!(e.contains("an element key holds the node's content"), "{e}");
}

#[test]
fn unsupported_version() {
    let e = errors("forge: 0.2\nroot:\n  Text: a\n");
    assert!(e.contains("1:8: unsupported spec version `0.2`"), "{e}");
}

#[test]
fn two_way_binding_outside_props() {
    let e = errors("forge: \"0.1\"\nroot:\n  Box:\n  style:\n    width: bind $w\n");
    assert!(e.contains("only allowed on props"), "{e}");
}

#[test]
fn bad_interpolation() {
    let e = errors("forge: \"0.1\"\nroot:\n  Text: \"Hi {user}\"\n");
    assert!(e.contains("3:9: `{user}` is not a binding"), "{e}");
}

#[test]
fn yaml_syntax_errors_have_positions() {
    let e = errors("forge: \"0.1\"\nroot:\n\tText: a\n");
    assert!(e.starts_with("3:"), "{e}");
}

#[test]
fn collects_every_error() {
    let e = errors(
        "forge: \"0.1\"\nwhat: 1\nroot:\n  Box:\n  children:\n    - variant: x\n    - Text: \"{oops}\"\n",
    );
    assert_eq!(e.lines().count(), 3, "{e}");
    assert!(e.contains("2:1: unknown top-level key `what`"), "{e}");
}

#[test]
fn rejects_anchors_and_multiple_documents() {
    assert!(errors("forge: \"0.1\"\nroot: &r\n  Text: a\n").contains("anchors"));
    assert!(
        errors("forge: \"0.1\"\nroot:\n  Text: a\n---\nforge: \"0.1\"\n").contains("exactly one")
    );
}
