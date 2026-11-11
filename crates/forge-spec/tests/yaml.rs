use forge_spec::yaml::{self, IdStyle, WriteOptions};
use forge_spec::{BindingMode, Document, ElementRef, Path, PathSegment, Segment, Unit, Value};

const SPEC: &str = include_str!("../../../docs/format/forge-yaml.md");

fn yaml_blocks(md: &str) -> Vec<String> {
    md.replace("\r\n", "\n")
        .split("```yaml\n")
        .skip(1)
        .map(|block| block.split("```").next().unwrap().to_owned())
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
    for src in &blocks {
        let doc = read(src);
        let out = write(&doc);
        assert_eq!(read(&out), doc, "round trip changed the model:\n{out}");
        assert_eq!(write(&read(&out)), out, "writer is not idempotent");
    }
}

#[test]
fn reads_the_login_example() {
    let doc = read(&yaml_blocks(SPEC)[0]);
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
    background: '#1e293b80'
    color: $theme.colors.text
  children:
    - Text: "Welcome back, {$user.name} {{beta}"
    - Input#email:
      value: bind $form.email
      placeholder: Email
      label: true
    - Button: Sign in
      variant: primary
      disabled: $form.invalid
      on click: sign_in()
    - Select:
      options: [light, dark, system default, 42]
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
    assert!(e.contains("5:3: duplicated key in mapping"), "{e}");
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

fn prop<'a>(node: &'a forge_spec::Node, key: &str) -> Option<&'a Value> {
    node.props.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn text(value: Option<&Value>) -> Option<&str> {
    match value {
        Some(Value::String(s)) | Some(Value::Enum(s)) => Some(s),
        _ => None,
    }
}

#[test]
fn anchors_aliases_and_merge_keys() {
    // Fragments at the top level hold `&anchor` slots; `<<` splices them in.
    let doc = read(
        r#"
forge: "0.1"
shared: &shared
  icon: save
  on click: save()
root:
  VStack:
  children:
    - Button:
      <<: *shared
    - Label: hi
"#,
    );
    let button = &doc.root.children[0];
    assert_eq!(button.element.name, "Button");
    assert_eq!(text(prop(button, "icon")), Some("save"));
    assert!(button.events.iter().any(|e| e.event == "click"));
}

#[test]
fn aliases_inherit_whole_nodes_and_scalar_anchors() {
    let doc = read(
        r#"
forge: "0.1"
fragments:
  btn: &btn
    Button: Save
  t: &t "hi"
root:
  VStack:
  children:
    - *btn
    - Label: *t
"#,
    );
    assert_eq!(doc.root.children[0].element.name, "Button");
    assert_eq!(text(doc.root.children[0].content.as_ref()), Some("Save"));
    assert_eq!(text(doc.root.children[1].content.as_ref()), Some("hi"));
}

#[test]
fn merge_keys_explicit_wins_and_errors() {
    // Explicit keys override merged ones.
    let doc = read(
        r#"
forge: "0.1"
m: &m
  icon: x
  tooltip: t
root:
  Button:
  <<: *m
  icon: y
"#,
    );
    assert_eq!(text(prop(&doc.root, "icon")), Some("y"));
    assert_eq!(text(prop(&doc.root, "tooltip")), Some("t"));

    // Sequences of maps merge in order, earlier winning.
    let doc = read(
        r#"
forge: "0.1"
a: &a {icon: x, tooltip: a}
b: &b {tooltip: b}
root:
  Button:
  <<: [*a, *b]
"#,
    );
    assert_eq!(text(prop(&doc.root, "tooltip")), Some("a"));

    // Undefined aliases and scalar merges error.
    assert!(errors("forge: \"0.1\"\nroot:\n  Button:\n  <<: *missing\n").contains("anchor"));
    assert!(errors("forge: \"0.1\"\nroot:\n  Button:\n  <<: 'text'\n").contains("merge key `<<`"));
}

#[test]
fn rejects_multiple_documents() {
    assert!(
        errors("forge: \"0.1\"\nroot:\n  Text: a\n---\nforge: \"0.1\"\n").contains("exactly one")
    );
}

fn preserve(doc: &Document) -> String {
    yaml::to_string_preserving(doc, &WriteOptions::default())
}

#[test]
fn preserving_writer_returns_unmodified_source_verbatim() {
    let src = r#"# Forge Studio doc
forge: "0.1"   # version

shared: &shared {variant: ghost}   # shared fragment

Dialog#prefs:
  title: Prefs

root:
  VStack#app:
  style: {gap: 8px}   # inline style
  <<: *shared
  children:
    # leading comment
    - Button#ok: Sign in
    - Label: done   # trailing comment
# tail comment
"#;
    let doc = read(src);
    assert_eq!(preserve(&doc), src);
}

#[test]
fn preserving_writer_splices_unchanged_regions() {
    let src = r#"forge: "0.1"

# a comment about the root
root:
  VStack#app:
  gap: 8
  children:
    - Button#ok: Sign in   # keep me
    - Label: done
"#;
    let mut doc = read(src);
    // Change one prop: only that line rewrites canonically.
    doc.set_prop(
        "0",
        "gap",
        Some(Value::Number {
            value: 12.0,
            unit: None,
        }),
    );
    let out = preserve(&doc);
    assert!(out.contains("# a comment about the root"));
    assert!(out.contains("- Button#ok: Sign in   # keep me"));
    assert!(out.contains("- Label: done"));
    assert!(out.contains("gap: 12"));
    assert!(!out.contains("gap: 8"));
    // And it still reads back to the same document.
    assert_eq!(read(&out), doc);
}

#[test]
fn preserving_writer_keeps_anchors_and_merges() {
    let src = r#"forge: "0.1"

frag: &frag
  icon: check

root:
  Button:
  <<: *frag
  variant: ghost
"#;
    let mut doc = read(src);
    doc.set_prop("0", "variant", Some(Value::Enum("solid".into())));
    let out = preserve(&doc);
    assert!(out.contains("frag: &frag"));
    assert!(out.contains("<<: *frag"));
    assert!(out.contains("variant: solid"));
    assert_eq!(read(&out), doc);
}

#[test]
fn preserving_writer_drops_removed_and_adds_new() {
    let src = r#"forge: "0.1"
root:
  VStack:
  children:
    - Button: A
    - Label: B
"#;
    let mut doc = read(src);
    doc.remove_node("0.1");
    let button = forge_spec::Node {
        element: forge_spec::ElementRef {
            namespace: None,
            name: "Input".into(),
        },
        ..Default::default()
    };
    doc.insert_node("0", button, Some(0));
    let out = preserve(&doc);
    assert!(out.contains("- Button: A"));
    assert!(!out.contains("Label: B"));
    assert!(out.contains("- Input"));
    assert_eq!(read(&out), doc);
}

use forge_spec::{Compare, Control};

fn binding(text: &str) -> Value {
    Value::Binding {
        path: path(&[text]),
        mode: BindingMode::OneWay,
    }
}

#[test]
fn control_blocks_read() {
    let doc = read(
        r#"forge: "0.1"
root:
  VStack:
  children:
    - if: $preview
      is: gpui
      children:
        - Text: GPUI
      else:
        - Text: none
    - for: $items
      each: item
      index: i
      children:
        - Text: "{$item}"
      else:
        - Text: empty
    - with: $tab
      as: t
      children:
        - Text: "{$t.name}"
    - match: $mode
      case gpui:
        - Text: G
      case 2:
        - Text: two
      else:
        - Text: fallback
"#,
    );
    let [if_node, for_node, with_node, match_node] = &doc.root.children[..] else {
        unreachable!()
    };
    assert!(if_node.is_block());
    assert_eq!(if_node.element.name, "If");
    let Some(Control::If {
        when,
        test,
        otherwise,
    }) = &if_node.control
    else {
        unreachable!()
    };
    assert_eq!(*when, binding("preview"));
    assert_eq!(*test, Some(Compare::Is(Value::Enum("gpui".into()))));
    assert_eq!(otherwise[0].element.name, "Text");

    let Some(Control::For {
        source,
        each,
        index,
        otherwise,
        ..
    }) = &for_node.control
    else {
        unreachable!()
    };
    assert_eq!(*source, binding("items"));
    assert_eq!(each, "item");
    assert_eq!(index.as_deref(), Some("i"));
    assert_eq!(otherwise.len(), 1);

    let Some(Control::With { source, name }) = &with_node.control else {
        unreachable!()
    };
    assert_eq!(*source, binding("tab"));
    assert_eq!(name, "t");

    let Some(Control::Match {
        source,
        cases,
        otherwise,
    }) = &match_node.control
    else {
        unreachable!()
    };
    assert_eq!(*source, binding("mode"));
    assert_eq!(cases[0].value, Value::Enum("gpui".into()));
    assert_eq!(otherwise.len(), 1);
}

#[test]
fn control_modifiers_on_elements() {
    let doc = read(
        "forge: \"0.1\"\nroot:\n  VStack:\n  children:\n    - Tab: \"{$tab.name}\"\n      for: $tabs\n      each: tab\n    - Text: Signed in\n      if: $user.signed_in\n",
    );
    let [tab, text] = &doc.root.children[..] else {
        unreachable!()
    };
    assert!(!tab.is_block());
    assert_eq!(tab.element.name, "Tab");
    assert!(matches!(tab.control, Some(Control::For { .. })));
    assert!(matches!(text.control, Some(Control::If { .. })));
    assert_eq!(text.element.name, "Text");
}

#[test]
fn control_flow_round_trip() {
    const SRC: &str = r#"forge: "0.1"
root:
  VStack:
  children:
    - if: $preview
      is: gpui
      children:
        - Text: GPUI
      else:
        - Text: none
    - for: $items
      each: item
      index: i
      children:
        - Text: "{$item}"
      else:
        - Text: empty
    - with: $tab
      as: t
      children:
        - Text: "{$t.name}"
    - match: $mode
      case gpui:
        - Text: G
      case 'a b':
        - Text: ab
      else:
        - Text: fallback
    - Tab: "{$tab.name}"
      for: $tabs
      each: tab
    - Text: Signed in
      if: $user.signed_in
"#;
    let doc = read(SRC);
    let out = write(&doc);
    assert_eq!(read(&out), doc, "round trip changed the model:\n{out}");
    assert_eq!(write(&read(&out)), out, "writer is not idempotent:\n{out}");
    assert_eq!(preserve(&doc), SRC);
}

#[test]
fn control_flow_errors() {
    // `else` without a control head.
    let e = errors("forge: \"0.1\"\nroot:\n  else:\n    - Text: a\n");
    assert!(e.contains("`else` needs `if`, `for` or `match`"), "{e}");
    // Two control keys.
    let e = errors("forge: \"0.1\"\nroot:\n  if: $a\n  for: $b\n  each: x\n");
    assert!(e.contains("two control keys"), "{e}");
    // `is` without `if`.
    let e = errors("forge: \"0.1\"\nroot:\n  Button: x\n  is: y\n");
    assert!(e.contains("`is` needs `if`"), "{e}");
    // `for` without `each`.
    let e = errors("forge: \"0.1\"\nroot:\n  for: $items\n");
    assert!(e.contains("`for` needs `each: <name>`"), "{e}");
    // `with` without `as`.
    let e = errors("forge: \"0.1\"\nroot:\n  with: $tab\n");
    assert!(e.contains("`with` needs `as: <name>`"), "{e}");
    // `match` with `children`.
    let e = errors("forge: \"0.1\"\nroot:\n  match: $x\n  children:\n    - Text: a\n");
    assert!(e.contains("`match` uses `case` arms"), "{e}");
    // `match` modifying an element.
    let e = errors("forge: \"0.1\"\nroot:\n  Button: x\n  match: $m\n  case a:\n    - Text: a\n");
    assert!(e.contains("`match` cannot modify an element"), "{e}");
    // Props on a bare control block.
    let e = errors("forge: \"0.1\"\nroot:\n  if: $a\n  tooltip: t\n");
    assert!(e.contains("prop `tooltip` needs an element"), "{e}");
    // `case` without `match`.
    let e = errors("forge: \"0.1\"\nroot:\n  Button: x\n  case a:\n    - Text: a\n");
    assert!(e.contains("`case a` needs `match`"), "{e}");
    // `else` on `with`.
    let e = errors("forge: \"0.1\"\nroot:\n  with: $x\n  as: y\n  else:\n    - Text: a\n");
    assert!(e.contains("`else` needs `if`, `for` or `match`"), "{e}");
    // `bind` (two-way) is not a valid control operand.
    let e = errors("forge: \"0.1\"\nroot:\n  for: bind $items\n  each: i\n");
    assert!(e.contains("only allowed on props"), "{e}");
    // A scalar that isn't a binding or list is not a `for` source.
    let e = errors("forge: \"0.1\"\nroot:\n  for: 42\n  each: i\n");
    assert!(e.contains("`for` needs a binding like `$items`"), "{e}");
}

#[test]
fn refname_captured_from_merge_alias() {
    let doc = yaml::from_str(
        r#"
forge: "0.1"
frag: &frag
  children:
    - Text: shared
root:
  VStack:
  children:
    - Tab: One
      <<: *frag
    - VStack:
      <<: *frag
"#,
    )
    .unwrap();
    assert_eq!(doc.node_at("0.0").unwrap().refname.as_deref(), Some("frag"));
    assert_eq!(doc.node_at("0.1").unwrap().refname.as_deref(), Some("frag"));
    assert_eq!(doc.node_at("0.0").unwrap().children.len(), 1);
    assert!(doc.node_at("0").unwrap().refname.is_none());
}

#[test]
fn duplicate_ref_names_are_rejected() {
    let err = yaml::from_str(
        r#"
forge: "0.1"
frag: &frag
  children:
    - Text: one
dup: &frag
  children:
    - Text: two
root:
  VStack:
  children:
    - Tab: One
      <<: *frag
"#,
    )
    .unwrap_err();
    assert!(
        err.0
            .iter()
            .any(|d| d.message.contains("duplicate ref `&frag`")),
        "{:?}",
        err.0
    );
}

#[test]
fn for_key_parses_and_round_trips() {
    let src = r#"forge: "0.1"

root:
  VStack:
  children:
    - for: $items
      each: item
      index: i
      key: item.id
      children:
        - Text: "{$item.name}"
"#;
    let doc = read(src);
    let node = &doc.root.children[0];
    assert!(matches!(
        &node.control,
        Some(Control::For { key: Some(_), .. })
    ));
    assert_eq!(yaml::to_string(&doc, &Default::default()), src);
}
