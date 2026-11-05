use forge_lib_template::Template;
use gpui_forge_runtime::render::ForgeLibrary;

#[test]
fn catalog_loads_and_lists_elements() {
    let cat = Template.catalog().expect("catalog");
    assert_eq!(cat.library, "template-lib");
    assert!(cat.element("Pill").is_some());
}

#[test]
fn alias_matches_catalog_namespace() {
    assert_eq!(Template.alias(), "mylib");
}
