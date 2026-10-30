use forge_web_canvas::check::{catalogs, check, to_json};
use forge_web_canvas::BASE;
use forge_web_canvas::render::set_path;

#[test]
fn valid_document_has_no_diagnostics() {
    let src = "forge: \"0.1\"\nroot:\n  VStack:\n  children:\n    - Button: Save\n      variant: Primary\n";
    let (doc, diagnostics) = check(src, BASE, &catalogs());
    assert!(doc.is_some());
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn invalid_document_still_renders() {
    let src = "forge: \"0.1\"\nroot:\n  Button: Save\n  variant: primary\n";
    let (doc, diagnostics) = check(src, BASE, &catalogs());
    assert!(
        doc.is_some(),
        "a wrong enum value must not blank the canvas"
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].pos.line, 4);
}

#[test]
fn syntax_error_reports_position() {
    let (doc, diagnostics) =  check("forge: \"0.1\"\nroot: [\n", BASE, &catalogs());
    assert!(doc.is_none());
    let json: serde_json::Value = serde_json::from_str(&to_json(&diagnostics)).unwrap();
    assert!(json[0]["line"].as_u64().unwrap() >= 1);
}

#[test]
fn two_way_bindings_write_nested_paths() {
    let path = |s| forge_spec::value::parse_path(s).unwrap();
    let mut state = serde_json::json!({ "form": { "email": "" } });
    set_path(&mut state, &path("form.email"), "a@b.c".into());
    set_path(&mut state, &path("todos[1].done"), true.into());
    assert_eq!(state["form"]["email"], "a@b.c");
    assert_eq!(state["todos"][1]["done"], true);
    assert!(state["todos"][0].is_null());
}
