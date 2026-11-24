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

/// Tier-2/3 extension widgets — the crate-backed arms all paint through
/// the same TestBackend path. `Image` decodes a real PNG written to a
/// temp dir so `src`/`base_dir` resolution runs end to end.
#[test]
fn tier2_widgets_render() {
    // A 2x2 opaque PNG (red top row, blue bottom row).
    let png: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x08, 0x06, 0x00, 0x00, 0x00, 0x72,
        0xb6, 0x0d, 0x24, 0x00, 0x00, 0x00, 0x12, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8,
        0xcf, 0xc0, 0xf0, 0x1f, 0x84, 0x19, 0xa0, 0xf4, 0x7f, 0x00, 0x43, 0xce, 0x07, 0xf9, 0xfa,
        0x81, 0x4a, 0xfd, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];
    let dir = std::env::temp_dir().join(format!("forge-img-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("pixel.png"), png).unwrap();
    let src = r##"forge: "0.1"
base: ratatui
root:
  VStack:
  children:
    - Toast:
      text: saved file
      type: success
      style:
        height: 3
    - Popup:
      title: help
      width: 20
      height: 5
      style:
        height: 7
      children:
        - Text: inside-pop
    - Slider:
      value: 40
      label: vol
      style:
        height: 1
    - WidgetList:
      style:
        height: 4
      children:
        - Text: row-alpha
        - Text: row-beta
    - PieChart:
      data: [30, 40, 30]
      labels: "cpu\nmem\ndisk"
      style:
        height: 8
    - BigText:
      text: OK
      size: quadrant
      style:
        height: 4
    - Nodes:
      edges: "0.0>1.0"
      style:
        height: 6
      children:
        - GraphNode: agent
        - GraphNode: tool
    - Shimmer:
      text: shimmering
      phase: 0.5
      style:
        height: 1
    - Dialog:
      open: true
      title: sure
      text: proceed now
      style:
        height: 9
    - Fx:
      kind: coalesce
      progress: 1.0
      style:
        height: 1
      children:
        - Text: fx-under
    - QrCode:
      data: hello-forge
      style:
        height: 4
    - Globe:
      zoom: 1.4
      style:
        height: 6
    - Image:
      src: pixel.png
      style:
        height: 1
"##;
    let mut view = view_of(src);
    view.set_base_dir(&dir);
    let buf = buffer_of(&mut view, 60, 80);
    let text = text_of(&buf);
    for needle in [
        "saved file",
        "inside-pop",
        "vol",
        "row-alpha",
        "agent",
        "shimmering",
        "sure",
        "proceed now",
        "fx-under",
    ] {
        assert!(text.contains(needle), "paints {needle}: {text:?}");
    }
    // Graphical widgets paint non-blank cells — QR blocks, big glyphs,
    // the globe mesh and the decoded PNG all leave marks.
    let filled = buf.content().iter().filter(|c| c.symbol() != " ").count();
    assert!(filled > 400, "graphical widgets paint cells: {filled}");
    let decoded = buf.content().iter().any(|c| {
        c.symbol() == "\u{2580}" && c.fg == Color::Rgb(255, 0, 0) && c.bg == Color::Rgb(0, 0, 255)
    });
    assert!(decoded, "png decoded to halfblocks");
    std::fs::remove_dir_all(&dir).ok();
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

/// A bound Slider takes arrows and a track click. A bound Dialog commits
/// its highlighted button to `choice` on Enter.
#[test]
fn slider_and_dialog_interact() {
    let mut view = view_of(
        r#"forge: "0.1"
base: ratatui
root:
  VStack:
  children:
    - Slider:
      value: bind $vol
      min: 0
      max: 10
      step: 2
      style:
        height: 1
    - Dialog:
      open: true
      title: confirm
      text: ok
      choice: bind $pick
      style:
        height: 8
"#,
    );
    view.update_state(&parse_path("vol").unwrap(), json!(4));
    view.update_state(&parse_path("pick").unwrap(), json!(""));
    buffer_of(&mut view, 40, 12);
    // The slider is focused first — step right by `step` → 6.
    view.on_key("Right");
    buffer_of(&mut view, 40, 12);
    assert_eq!(
        view.state().get("vol"),
        Some(&json!(6.0)),
        "arrow steps the bound value"
    );
    // Click the right half of the track → writes near `max`.
    view.on_mouse("down", 39, 0);
    let vol = view
        .state()
        .get("vol")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    assert!(vol >= 8.0, "track click maps column to value: {vol}");
    // Focus the dialog, pick No, commit.
    view.on_key("Tab");
    view.on_key("Right");
    view.on_key("Enter");
    assert_eq!(
        view.state().get("pick"),
        Some(&json!("no")),
        "dialog commits the picked button"
    );
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
