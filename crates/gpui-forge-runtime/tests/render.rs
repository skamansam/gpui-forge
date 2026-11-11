//! Integration test — a `ForgeView` driven inside a live App context (#21).
//!
//! Exercises the entity through the same public API hosts use: `set_document`,
//! `update_state`, `fire_call`, `preview`, `set_node_style`. Everything runs in
//! a single app launch because the platform loop owns the process — the
//! harness `#[gpui_kit::test]` needs `test-support`, which pulls the whole
//! gpui-fast engine graph into the workspace.

// Not on macOS: `MacPlatform::new` demands a `MainThreadMarker`, which only
// exists on the real main thread — cargo test runs cases on worker threads,
// so a live App context can't be created there.
#![cfg(not(target_os = "macos"))]

use gpui_forge_runtime::check;
use gpui_forge_runtime::forge_spec::yaml;
use gpui_forge_runtime::render::{ForgeLibrary, ForgeView};
use gpui_kit::{self, AppContext};
use std::cell::RefCell;
use std::rc::Rc;

const DOC: &str = r#"forge: "0.1"
base: gpui-kit

root:
  VStack:
  style:
    padding: 16px
  children:
    - Text: Hello, Forge!
    - Button: Go
      on click: go()
"#;

/// A no-op adapter — registered on the shell to prove nested previews share
/// the parent's `libraries` (alias.Name elements render instead of the
/// "not provided by this adapter" placeholder).
struct Stub;

impl ForgeLibrary for Stub {
    fn alias(&self) -> &'static str {
        "stub"
    }
    fn render(
        &self,
        _: &gpui_forge_runtime::forge_spec::Node,
        _: &str,
        _: &ForgeView,
        _: &mut gpui_kit::Window,
        _: &mut gpui_kit::Context<ForgeView>,
    ) -> Option<gpui_kit::AnyElement> {
        None
    }
}

#[test]
fn forge_view_integration() {
    let fired = Rc::new(RefCell::new(Vec::new()));
    let bubbled = Rc::new(RefCell::new(Vec::new()));
    let states = Rc::new(RefCell::new(Vec::new()));
    let (f, b, s) = (fired.clone(), bubbled.clone(), states.clone());
    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        let mut view = ForgeView::new(check::catalogs());
        let (doc, diagnostics) =
            check::check(DOC, gpui_forge_runtime::base::BASE_ID, &view.catalogs);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let doc = doc.expect("document parses");
        let (f2, b2, s2) = (f.clone(), b.clone(), s.clone());
        let view = cx.new(|cx| {
            view.set_sinks(
                Rc::new(move |call, _, _| b2.borrow_mut().push(call.to_owned())),
                Rc::new(move |state, _, _| s2.borrow_mut().push(state.clone())),
            );
            view.set_hook(
                "go",
                Rc::new(move |args, _, _| f2.borrow_mut().push(args.join(","))),
            );
            view.set_document(doc, cx);
            view
        });

        // The document is live: root VStack, two children, labels resolved.
        view.read_with(cx, |view, _| {
            let doc = view.document().expect("document set");
            assert_eq!(doc.root.children.len(), 2);
            assert_eq!(doc.root.children[0].element.name, "Text");
        });

        // Registered hooks fire; unknown names bubble to the host's sink.
        view.update(cx, |view, cx| {
            view.fire_call("go", vec!["'a'".into()], cx);
            view.fire_call("nope", vec!["42".into()], cx);
        });
        assert_eq!(f.borrow().as_slice(), ["'a'"]);
        assert_eq!(b.borrow().as_slice(), ["nope(42)"]);

        // Two-way state writes reach the state sink.
        view.update(cx, |view, cx| {
            view.update_state(
                &gpui_forge_runtime::forge_spec::value::parse_path("form.field").unwrap(),
                serde_json::Value::String("x".into()),
                cx,
            );
        });
        assert_eq!(
            s.borrow().last().and_then(|s| s["form"]["field"].as_str()),
            Some("x")
        );

        // A named preview hosts its own document — and the parent's adapters.
        view.update(cx, |view, _| {
            view.register_library(Stub);
        });
        let (child, _) = check::check(DOC, gpui_forge_runtime::base::BASE_ID, &check::catalogs());
        view.update(cx, |view, cx| {
            view.set_preview_document("canvas", child.unwrap(), cx);
            let preview = view.preview("canvas", cx);
            assert!(preview.read(cx).document().is_some());
            assert_eq!(preview.read(cx).libraries.len(), 1);
        });

        // Style mutations land in the document and survive the write-back.
        view.update(cx, |view, cx| {
            view.set_node_style("0", "gap", &serde_json::json!("8px"), cx);
        });
        view.read_with(cx, |view, _| {
            let node = view.document().unwrap().node_at("0").unwrap();
            assert!(node.style.iter().any(|(k, _)| k == "gap"));
            let src = yaml::to_string_preserving(
                view.document().unwrap(),
                &yaml::WriteOptions::default(),
            );
            assert!(src.contains("gap"), "{src}");
        });
        std::process::exit(0);
    });
    // Unreachable: the run callback exits the process after the checks.
    unreachable!()
}
