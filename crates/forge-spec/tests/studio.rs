//! The Forge Studio shell (`crates/forge-studio/*.forge.yaml`) is a Forge document:
//! these tests keep it valid, so Studio stays renderable by the runtime it ships with.

use forge_spec::catalog::{self, Catalog};
use forge_spec::{Diagnostic, yaml};

fn catalogs() -> Vec<Catalog> {
    vec![
        catalog::from_yaml(include_str!("../../../catalogs/gpui-kit.catalog.yaml")).unwrap(),
        catalog::from_yaml(include_str!("../../../catalogs/forge.catalog.yaml")).unwrap(),
    ]
}

fn check(src: &str, base: &str, catalogs: &[Catalog]) -> (bool, Vec<Diagnostic>) {
    match yaml::from_str_with_source_map(src) {
        Ok((doc, map)) => (
            true,
            forge_spec::validate::validate(&doc, base, catalogs, Some(&map)),
        ),
        Err(errors) => (false, errors.0),
    }
}

#[test]
fn studio_documents_validate_clean() {
    let catalogs = catalogs();
    for (name, src) in [
        (
            "studio.forge.yaml",
            include_str!("../../forge-studio/studio.forge.yaml"),
        ),
        (
            "welcome.forge.yaml",
            include_str!("../../forge-studio/welcome.forge.yaml"),
        ),
        (
            "gallery.forge.yaml",
            include_str!("../../forge-studio/gallery.forge.yaml"),
        ),
    ] {
        let (parsed, diagnostics) = check(src, "gpui-kit", &catalogs);
        assert!(parsed, "{name} must parse: {diagnostics:?}");
        assert!(diagnostics.is_empty(), "{name}: {diagnostics:?}");
    }
}

#[test]
fn studio_documents_write_back_verbatim() {
    // Unmodified documents keep their source byte-for-byte — comments,
    // anchors, merge keys and formatting survive a save.
    for src in [
        include_str!("../../forge-studio/studio.forge.yaml"),
        include_str!("../../forge-studio/welcome.forge.yaml"),
        include_str!("../../forge-studio/gallery.forge.yaml"),
    ] {
        let doc = yaml::from_str(src).unwrap();
        assert_eq!(yaml::to_string_preserving(&doc, &Default::default()), src);
    }
}

#[test]
fn nested_merge_blocks_stay_verbatim() {
    let src = r#"forge: "0.1"
app: &app
  children:
    - MenuItem: X
frag: &frag
  children:
    - DropdownMenu:
      <<: *app
root:
  VStack:
  children:
    - TitleBar:
      <<: *frag
"#;
    let doc = yaml::from_str(src).unwrap();
    assert_eq!(yaml::to_string_preserving(&doc, &Default::default()), src);
}
