use forge_spec::yaml;
use forge_spec::{Document, ElementRef, Node, Value};

fn doc() -> Document {
    yaml::from_str(
        r#"
forge: "0.1"
root:
  VStack#app:
  style: {gap: 8px}
  children:
    - Label: "hi"
    - HStack#row:
      children:
        - Button: A
        - Button#b: B
"#,
    )
    .unwrap()
}

#[test]
fn node_at_resolves_index_paths() {
    let doc = doc();
    assert_eq!(doc.node_at("0").unwrap().element.name, "VStack");
    assert_eq!(doc.node_at("0.0").unwrap().element.name, "Label");
    assert_eq!(doc.node_at("0.1").unwrap().element.name, "HStack");
    assert_eq!(doc.node_at("0.1.1").unwrap().id.as_deref(), Some("b"));
    assert!(doc.node_at("1").is_none());
    assert!(doc.node_at("0.9").is_none());
    assert!(doc.node_at("0.1.x").is_none());
    assert!(doc.node_at("").is_none());
}

#[test]
fn set_prop_updates_inserts_and_removes() {
    let mut doc = doc();
    // insert: HStack has no gap yet
    assert!(doc.set_prop(
        "0.1",
        "gap",
        Some(Value::Number {
            value: 4.0,
            unit: None
        })
    ));
    let row = doc.node_at("0.1").unwrap();
    assert_eq!(row.props[0].0, "gap");
    // replace again
    assert!(doc.set_prop(
        "0.1",
        "gap",
        Some(Value::Number {
            value: 9.0,
            unit: None
        })
    ));
    assert!(matches!(
        doc.node_at("0.1").unwrap().props[0].1,
        Value::Number { value: 9.0, .. }
    ));
    // remove
    assert!(doc.set_prop("0.1", "gap", None));
    assert!(doc.node_at("0.1").unwrap().props.is_empty());
    // bad path → no change, returns false
    assert!(!doc.set_prop("0.9", "x", Some(Value::Bool(true))));
}

#[test]
fn insert_and_remove_nodes() {
    let mut doc = doc();
    let node = Node {
        element: ElementRef {
            namespace: None,
            name: "Label".into(),
        },
        content: Some(Value::String("new".into())),
        ..Default::default()
    };
    assert!(doc.insert_node("0.1", node, Some(0)));
    let row = doc.node_at("0.1").unwrap();
    assert_eq!(row.children.len(), 3);
    assert_eq!(row.children[0].element.name, "Label");
    // remove the inserted node
    let removed = doc.remove_node("0.1.0").unwrap();
    assert_eq!(removed.element.name, "Label");
    assert_eq!(doc.node_at("0.1").unwrap().children.len(), 2);
    // root cannot be removed
    assert!(doc.remove_node("0").is_none());
    assert!(doc.remove_node("0.9").is_none());
}

#[test]
fn reparent_moves_and_guards() {
    let mut doc = doc();
    // move Button#b (0.1.1) under the root
    assert!(doc.reparent("0.1.1", "0", None));
    assert_eq!(doc.node_at("0.1").unwrap().children.len(), 1);
    assert_eq!(doc.node_at("0.2").unwrap().id.as_deref(), Some("b"));
    // cannot move root, into itself, or into a descendant
    assert!(!doc.reparent("0", "0.1", None));
    assert!(!doc.reparent("0.1", "0.1", None));
    assert!(!doc.reparent("0.1", "0.1.0", None));
}

#[test]
fn insert_after_and_move_after_place_siblings() {
    let mut doc = doc();
    // insert a Label right after the first Button (0.1.0)
    let node = Node {
        element: ElementRef {
            namespace: None,
            name: "Label".into(),
        },
        content: Some(Value::String("x".into())),
        ..Default::default()
    };
    assert_eq!(doc.insert_after("0.1.0", node).as_deref(), Some("0.1.1"));
    assert_eq!(doc.node_at("0.1.1").unwrap().element.name, "Label");
    // move it after the root-level HStack sibling's first child — same parent reorder
    assert_eq!(doc.move_after("0.1.0", "0.1.2").as_deref(), Some("0.1.2"));
    // moving after itself or into a descendant fails
    assert!(doc.move_after("0.1", "0.1").is_none());
    assert!(doc.move_after("0.1", "0.1.0").is_none());
}

fn control_doc() -> Document {
    yaml::from_str(
        r#"
forge: "0.1"
root:
  VStack:
  children:
    - if: $cond
      children:
        - Text: then
      else:
        - Text: e0
        - Text: e1
    - match: $mode
      case a:
        - Text: ca0
      case b:
        - Text: cb0
"#,
    )
    .unwrap()
}

#[test]
fn node_at_resolves_branch_paths() {
    let doc = control_doc();
    // else children — `e` steps
    assert_eq!(doc.node_at("0.0.e0").unwrap().element.name, "Text");
    assert_eq!(doc.node_at("0.0.e1").unwrap().element.name, "Text");
    assert!(doc.node_at("0.0.e2").is_none());
    // case children — `c` steps
    assert_eq!(doc.node_at("0.1.c0.0").unwrap().element.name, "Text");
    assert_eq!(doc.node_at("0.1.c1.0").unwrap().element.name, "Text");
    assert!(doc.node_at("0.1.c2.0").is_none());
    // bare `e`/`c` list paths never resolve to a node
    assert!(doc.node_at("0.0.e").is_none());
    assert!(doc.node_at("0.1.c0").is_none());
}

#[test]
fn edit_ops_reach_branch_children() {
    let mut doc = control_doc();
    // remove an else child — e1 shifts into the e0 slot
    assert!(doc.remove_node("0.0.e0").is_some());
    assert_eq!(
        doc.node_at("0.0.e0").unwrap().content,
        Some(Value::Enum("e1".into()))
    );
    assert!(doc.node_at("0.0.e1").is_none());
    // insert into the `else:` list itself via the `e` container path
    let node = Node {
        element: ElementRef {
            namespace: None,
            name: "Label".into(),
        },
        ..Default::default()
    };
    assert!(doc.insert_node("0.0.e", node, Some(0)));
    assert_eq!(doc.node_at("0.0.e0").unwrap().element.name, "Label");
    // insert into a case arm's list via `c{n}`
    let node = Node {
        element: ElementRef {
            namespace: None,
            name: "Badge".into(),
        },
        ..Default::default()
    };
    assert!(doc.insert_node("0.1.c1", node, None));
    assert_eq!(doc.node_at("0.1.c1.1").unwrap().element.name, "Badge");
    // set_prop on a branch child
    assert!(doc.set_prop("0.1.c0.0", "size", Some(Value::Enum("lg".into()))));
    assert_eq!(
        doc.node_at("0.1.c0.0").unwrap().props[0].1,
        Value::Enum("lg".into())
    );
}
