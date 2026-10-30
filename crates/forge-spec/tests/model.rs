use forge_spec::{Document, ElementRef, Node, Value};
use forge_spec::yaml;

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
    assert!(doc.set_prop("0.1", "gap", Some(Value::Number { value: 4.0, unit: None })));
    let row = doc.node_at("0.1").unwrap();
    assert_eq!(row.props[0].0, "gap");
    // replace again
    assert!(doc.set_prop("0.1", "gap", Some(Value::Number { value: 9.0, unit: None })));
    assert!(matches!(doc.node_at("0.1").unwrap().props[0].1, Value::Number { value: 9.0, .. }));
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
        element: ElementRef { namespace: None, name: "Label".into() },
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
