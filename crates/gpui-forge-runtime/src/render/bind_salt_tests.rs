//! `bind_salt` regression — sibling fields bound to `style.gap` vs
//! `style.gap-x` must salt their entity cache keys differently or a
//! reordered row inherits a subscription that writes the other field.

use super::gpui_kit::bind_salt;
use forge_spec::model::{BindingMode, ElementRef, Node, Path, PathSegment, Value};

fn node_with(path: &str) -> Node {
    Node {
        element: ElementRef {
            namespace: None,
            name: "Input".into(),
        },
        id: None,
        content: None,
        props: vec![(
            "value".to_owned(),
            Value::Binding {
                path: Path(
                    path.split('.')
                        .map(|s| PathSegment::Field(s.to_owned()))
                        .collect(),
                ),
                mode: BindingMode::TwoWay,
            },
        )],
        style: vec![],
        events: vec![],
        children: vec![],
        control: None,
        refname: None,
        merged: vec![],
    }
}

#[test]
fn bind_salt_distinguishes_sibling_style_fields() {
    let gap = node_with("document.edits.style.gap");
    let gap_x = node_with("document.edits.style.gap-x");
    assert_ne!(bind_salt(&gap), bind_salt(&gap_x));
    assert!(bind_salt(&gap).contains("value=document.edits.style.gap"));

    let mut unbound = gap;
    unbound.props = vec![];
    assert_eq!(bind_salt(&unbound), "");
}
