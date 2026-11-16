//! `forge.Files` self-contained listing — `root:` scan, glob filters,
//! search narrowing.
#![cfg(feature = "base-gpui-kit")]

use gpui_forge_runtime::render::{filter_listing, glob_match, scan_files};
use serde_json::json;

fn forge_studio_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../forge-studio")
}

#[test]
fn scan_lists_dirs_and_matching_files() {
    let filters = vec!["*.forge.yaml".to_owned()];
    let mut budget = 400usize;
    let entries = scan_files(&forge_studio_dir(), &filters, 6, false, &mut budget);
    let names: Vec<&str> = entries.iter().filter_map(|e| e["name"].as_str()).collect();
    assert!(
        names.iter().any(|n| n.ends_with(".forge.yaml")),
        "expected forge docs in {names:?}"
    );
    assert!(names.contains(&"src"), "expected src dir in {names:?}");
    assert!(
        !names.contains(&"Cargo.toml"),
        "filter should drop Cargo.toml: {names:?}"
    );
}

#[test]
fn scan_unfiltered_lists_top_level() {
    let mut budget = 400usize;
    let entries = scan_files(&forge_studio_dir(), &[], 1, false, &mut budget);
    let names: Vec<&str> = entries.iter().filter_map(|e| e["name"].as_str()).collect();
    assert!(names.contains(&"Cargo.toml"), "{names:?}");
    assert!(names.contains(&"src"), "{names:?}");
    assert!(!names.contains(&"dist"), "SKIP dirs excluded: {names:?}");
}

#[test]
fn glob_match_cases() {
    assert!(glob_match("*.forge.yaml", "studio.forge.yaml"));
    assert!(glob_match("*", "anything"));
    assert!(glob_match("*.rs", "main.rs"));
    assert!(!glob_match("*.rs", "main.toml"));
    assert!(glob_match("a?c", "abc"));
    assert!(!glob_match("a?c", "ac"));
}

#[test]
fn filter_listing_keeps_matching_ancestors() {
    let listing = vec![
        json!({"name":"src","path":"/x/src","dir":true,"children":[
            {"name":"main.rs","path":"/x/src/main.rs","dir":false,"children":[]},
            {"name":"lib.rs","path":"/x/src/lib.rs","dir":false,"children":[]},
        ]}),
        json!({"name":"readme.md","path":"/x/readme.md","dir":false,"children":[]}),
    ];
    let hits = filter_listing(&listing, "main");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0]["name"], "src");
    assert_eq!(hits[0]["children"].as_array().unwrap().len(), 1);
}
