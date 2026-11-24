//! Ratatui-base `ForgeView` coverage (#156) — control expansion, `{$theme.*}`
//! styles, the namespaced-library render hook and mouse hit zones, verified
//! against `TestBackend` buffers.
//!
//! Run: `cargo test -p gpui-forge-runtime --no-default-features --features
//! base-ratatui --test ratatui`. Under the default base the file compiles
//! empty — the base features are mutually exclusive.
#![cfg(feature = "base-ratatui")]

use gpui_forge_runtime::check;
use gpui_forge_runtime::forge_spec::Node;
use gpui_forge_runtime::forge_spec::value::parse_path;
use gpui_forge_runtime::render::{ForgeLibrary, ForgeView};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Paragraph;
use serde_json::json;

fn view_of(src: &str) -> ForgeView {
    let mut view = ForgeView::new(check::catalogs());
    let (doc, diags) = check::check(src, gpui_forge_runtime::base::BASE_ID, &view.catalogs);
    assert!(doc.is_some(), "doc must parse: {diags:?}");
    view.set_document(doc.unwrap());
    view
}

fn buffer_of(view: &mut ForgeView, w: u16, h: u16) -> Buffer {
    let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
    term.draw(|f| view.render(f)).unwrap();
    term.backend().buffer().clone()
}

fn text_of(buf: &Buffer) -> String {
    buf.content().iter().map(|c| c.symbol()).collect()
}

#[test]
fn control_for_repeats_children() {
    let mut view = view_of(
        r#"forge: "0.1"
base: ratatui
root:
  VStack:
  children:
    - for: $items
      each: item
      children:
        - Text: "{$item}"
"#,
    );
    view.update_state(&parse_path("items").unwrap(), json!(["alpha", "beta"]));
    let buf = buffer_of(&mut view, 40, 10);
    let text = text_of(&buf);
    assert!(text.contains("alpha"), "first item rendered: {text:?}");
    assert!(text.contains("beta"), "second item rendered: {text:?}");
}

#[test]
fn control_if_and_else() {
    let mut view = view_of(
        r#"forge: "0.1"
base: ratatui
root:
  VStack:
  children:
    - Text: "shown"
      if: $flag
    - if: $flag
      else:
        - Text: "fallback"
"#,
    );
    view.update_state(&parse_path("flag").unwrap(), json!(false));
    let buf = buffer_of(&mut view, 40, 10);
    let text = text_of(&buf);
    assert!(!text.contains("shown"), "false if hides: {text:?}");
    assert!(text.contains("fallback"), "else branch rendered: {text:?}");

    view.update_state(&parse_path("flag").unwrap(), json!(true));
    let buf = buffer_of(&mut view, 40, 10);
    let text = text_of(&buf);
    assert!(text.contains("shown"), "true if renders: {text:?}");
    assert!(!text.contains("fallback"), "else skipped: {text:?}");
}

#[test]
fn control_match_and_with() {
    let mut view = view_of(
        r#"forge: "0.1"
base: ratatui
root:
  VStack:
  children:
    - match: $mode
      case a:
        - Text: "mode-a"
      else:
        - Text: "no-mode"
    - with: $user
      as: u
      children:
        - Text: "user {$u.name}"
"#,
    );
    view.update_state(&parse_path("mode").unwrap(), json!("a"));
    view.update_state(&parse_path("user").unwrap(), json!({"name": "sam"}));
    let buf = buffer_of(&mut view, 40, 10);
    let text = text_of(&buf);
    assert!(text.contains("mode-a"), "matched arm: {text:?}");
    assert!(text.contains("user sam"), "with scope resolved: {text:?}");
}

#[test]
fn theme_token_paints_text() {
    let mut view = view_of(
        r#"forge: "0.1"
base: ratatui
root:
  VStack:
  children:
    - Text: "accent"
      style:
        color: $theme.accent
"#,
    );
    let mut colors = serde_json::Map::new();
    colors.insert("accent".to_owned(), json!("#ff0000"));
    view.set_theme(true, &colors);
    let buf = buffer_of(&mut view, 40, 10);
    assert_eq!(
        buf[(0, 0)].fg,
        Color::Rgb(255, 0, 0),
        "$theme.accent must land on the cell"
    );
}

struct Stub;

impl ForgeLibrary for Stub {
    fn alias(&self) -> &'static str {
        "stub"
    }
    fn render(
        &self,
        _node: &Node,
        area: Rect,
        _view: &mut ForgeView,
        frame: &mut ratatui::Frame,
    ) -> bool {
        frame.render_widget(Paragraph::new("stub-rendered"), area);
        true
    }
}

#[test]
fn namespaced_element_dispatches_to_library() {
    let mut view = view_of(
        r#"forge: "0.1"
base: ratatui
use:
  stub: stub-lib
root:
  VStack:
  children:
    - stub.Widget:
"#,
    );
    view.register_library(Stub);
    let buf = buffer_of(&mut view, 40, 10);
    let text = text_of(&buf);
    assert!(text.contains("stub-rendered"), "library drew: {text:?}");
    assert!(
        !text.contains("no ratatui library"),
        "placeholder skipped: {text:?}"
    );
}

#[test]
fn mouse_click_focuses_and_writes() {
    let mut view = view_of(
        r#"forge: "0.1"
base: ratatui
root:
  VStack:
  children:
    - Checkbox:
      checked: bind $on
"#,
    );
    view.update_state(&parse_path("on").unwrap(), json!(false));
    buffer_of(&mut view, 40, 10); // draw once so zones exist
    view.on_mouse("down", 0, 0);
    // The click wrote `on` — re-render and read the glyph back.
    let buf = buffer_of(&mut view, 40, 10);
    let text = text_of(&buf);
    assert!(text.contains("[x]"), "click toggled the checkbox: {text:?}");
}

#[test]
fn extension_widgets_render() {
    let src = r##"forge: "0.1"
base: ratatui
root:
  VStack:
  children:
    - Tree:
      title: Files
      items: "src\n  main.rs\n  lib.rs\ndocs"
      style:
        height: 5
    - Spinner:
      kind: braille-six
      label: Loading
      step: 2
      style:
        height: 1
    - Markdown:
      text: "# Head\n\n- a\n- b"
      style:
        height: 6
    - Terminal:
      text: "prompt$ run\nok done"
      title: term
      style:
        height: 5
    - ScrollView:
      width: 20
      height: 30
      scrollbars: never
      style:
        height: 4
      children:
        - Text: "row-one"
        - Text: "row-two"
    - CodeEditor:
      language: rust
      text: "fn main() {{}}"
      style:
        height: 4
"##;
    let mut view = view_of(src);
    let buf = buffer_of(&mut view, 40, 30);
    let text = text_of(&buf);
    assert!(text.contains("main.rs"), "tree renders items: {text:?}");
    assert!(text.contains("Loading"), "spinner label paints: {text:?}");
    assert!(text.contains("Head"), "markdown heading paints: {text:?}");
    assert!(text.contains("prompt"), "terminal paints text: {text:?}");
    assert!(
        text.contains("row-one"),
        "scrollview child paints: {text:?}"
    );
    assert!(text.contains("fn"), "code editor paints text: {text:?}");
}

#[test]
fn ansi_escapes_become_styles() {
    let src = "forge: \"0.1\"\nbase: ratatui\nroot:\n  VStack:\n  children:\n    - AnsiText: \"\u{1b}[31mRED\u{1b}[0m\"\n";
    let mut view = view_of(src);
    let buf = buffer_of(&mut view, 20, 3);
    let text = text_of(&buf);
    assert!(text.contains("RED"), "ansi text paints: {text:?}");
    let cell = buf.cell((0, 0)).expect("cell exists");
    assert_eq!(cell.fg, Color::Red, "escape parsed to fg red");
}

/// The shipped showcase must keep parsing and painting — it exercises every
/// catalog widget, so a broken arm fails here before a user sees it.
#[test]
fn showcase_doc_renders() {
    let src = include_str!("../../../examples/showcases/ratatui-showcase.forge.yaml");
    let mut view = view_of(src);
    let buf = buffer_of(&mut view, 120, 80);
    let text = text_of(&buf);
    for needle in [
        "main.rs",
        "building",
        "cargo build",
        "scrollable",
        "green",
        "fn main",
    ] {
        assert!(text.contains(needle), "showcase paints {needle}: {text:?}");
    }
}

/// `document.selected` paints the accent frame over the node's zone —
/// the canvas's selection outline — but only when `dnd` flagged the view.
#[test]
fn selection_frames_the_node() {
    let mut view = view_of(
        r#"forge: "0.1"
base: ratatui
root:
  VStack:
  children:
    - Block:
      title: card
      children:
        - Text: inside
"#,
    );
    view.update_state(&parse_path("document.selected").unwrap(), json!("0.0"));
    let plain = buffer_of(&mut view, 20, 5);
    assert!(
        !plain
            .content()
            .iter()
            .any(|c| c.fg == Color::Cyan && (c.symbol() == "│" || c.symbol() == "─")),
        "no frame without dnd"
    );
    view.set_dnd(true);
    let buf = buffer_of(&mut view, 20, 5);
    assert!(
        buf.content()
            .iter()
            .any(|c| c.fg == Color::Cyan && (c.symbol() == "│" || c.symbol() == "─")),
        "cyan frame cells paint under dnd"
    );
}
