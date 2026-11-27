//! Live interaction test — a `base.Checkbox` clicked inside a real window.
//!
//! Asserts the whole Forge event path end to end: the element renders with
//! default chrome, a simulated click toggles the uncontrolled root, the
//! `on change:` hook fires with a `"true"`/`"false"` payload, and the
//! `bind $form.accepted` write-back reaches the state sink.

// See gpui-forge-runtime/tests/render.rs — no live App on macOS worker threads.
#![cfg(not(target_os = "macos"))]

use forge_lib_base::Base;
use gpui::{Modifiers, TestAppContext, VisualTestContext, px, size};
use gpui_forge_runtime::base::{BASE_ID, kit};
use gpui_forge_runtime::check;
use gpui_forge_runtime::render::ForgeView;
use std::cell::RefCell;
use std::rc::Rc;

const EMPTY: &str = r#"forge: "1.0"
base: gpui-kit
use:
  base: base-gpui

root:
  VStack:
  children: []
"#;

const TEXT: &str = r#"forge: "1.0"
base: gpui-kit
use:
  base: base-gpui

root:
  VStack:
  children:
    - Text: hi
"#;

const BUTTON_PLAIN: &str = r#"forge: "1.0"
base: gpui-kit
use:
  base: base-gpui

root:
  VStack:
  children:
    - base.Button: Go
"#;

const CHECKBOX_PLAIN: &str = r#"forge: "1.0"
base: gpui-kit
use:
  base: base-gpui

root:
  VStack:
  children:
    - base.Checkbox: Accept terms
"#;

const DOC: &str = r#"forge: "1.0"
base: gpui-kit
use:
  base: base-gpui

root:
  VStack:
  style:
    padding: 16px
  children:
    - base.Checkbox: Accept terms
      checked: bind $form.accepted
      on change: accepted()
"#;

fn open_doc(
    cx: &mut TestAppContext,
    doc_src: &'static str,
    fired: Rc<RefCell<Vec<String>>>,
    states: Rc<RefCell<Vec<serde_json::Value>>>,
) -> gpui::AnyWindowHandle {
    cx.open_window(size(px(400.), px(300.)), move |_, cx| {
        let mut view = ForgeView::new(check::catalogs());
        let (doc, diagnostics) = check::check(doc_src, BASE_ID, &view.catalogs);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        view.register_library(Base::new());
        let (f2, s2) = (fired.clone(), states.clone());
        view.set_sinks(
            Rc::new(|_, _, _| {}),
            Rc::new(move |state, _, _| s2.borrow_mut().push(state.clone())),
        );
        view.set_hook(
            "accepted",
            Rc::new(move |args, _, _| f2.borrow_mut().push(args.join(","))),
        );
        view.set_document(doc.expect("document parses"), cx);
        view
    })
    .into()
}

#[gpui::test]
fn empty_doc_renders(cx: &mut TestAppContext) {
    cx.update(kit::init);
    let window = open_doc(cx, EMPTY, Default::default(), Default::default());
    let cx = VisualTestContext::from_window(window, cx);
    cx.run_until_parked();
}

#[gpui::test]
fn text_child_renders(cx: &mut TestAppContext) {
    cx.update(kit::init);
    let window = open_doc(cx, TEXT, Default::default(), Default::default());
    let cx = VisualTestContext::from_window(window, cx);
    cx.run_until_parked();
}

#[gpui::test]
fn button_plain_renders(cx: &mut TestAppContext) {
    cx.update(kit::init);
    let window = open_doc(cx, BUTTON_PLAIN, Default::default(), Default::default());
    let cx = VisualTestContext::from_window(window, cx);
    cx.run_until_parked();
}

#[gpui::test]
fn checkbox_plain_renders(cx: &mut TestAppContext) {
    cx.update(kit::init);
    let window = open_doc(cx, CHECKBOX_PLAIN, Default::default(), Default::default());
    let cx = VisualTestContext::from_window(window, cx);
    cx.run_until_parked();
}

#[gpui::test]
fn checkbox_click_fires_change_and_writes_binding(cx: &mut TestAppContext) {
    cx.update(kit::init);
    let fired = Rc::new(RefCell::new(Vec::new()));
    let states = Rc::new(RefCell::new(Vec::new()));
    let window = open_doc(cx, DOC, fired.clone(), states.clone());
    let mut cx = VisualTestContext::from_window(window, cx);
    cx.run_until_parked();

    // The checkbox row records its node key (`root.0`) as a debug selector.
    // Click near the row's origin — the row center lands on the label, which
    // is a sibling of the toggle target, not part of `CheckboxRoot`.
    let bounds = cx.debug_bounds("root.0").expect("checkbox row rendered");
    let hit = bounds.origin + gpui::point(px(8.), px(8.));
    cx.simulate_click(hit, Modifiers::none());
    cx.run_until_parked();
    assert_eq!(fired.borrow().as_slice(), ["true"]);
    assert_eq!(
        states
            .borrow()
            .last()
            .and_then(|s| s["form"]["accepted"].as_bool()),
        Some(true)
    );

    // Second click toggles back off.
    cx.simulate_click(hit, Modifiers::none());
    cx.run_until_parked();
    assert_eq!(fired.borrow().as_slice(), ["true", "false"]);
    assert_eq!(
        states
            .borrow()
            .last()
            .and_then(|s| s["form"]["accepted"].as_bool()),
        Some(false)
    );
}
