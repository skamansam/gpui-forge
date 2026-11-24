//! Interprets a Forge document as gpui-kit elements.
//!
//! The gpui-kit adapter (#75). `ForgeView` renders a `Document`, keeps the JSON view
//! state that `bind` writes to, and dispatches `on <event>` bindings to registered
//! hooks or the hook sink.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::render::{get_path, set_path, style_edit_value};
use forge_spec::catalog::{Catalog, Children, PropType};
use forge_spec::value::parse_path;
use forge_spec::yaml;
use forge_spec::{
    BindingMode, Color, Compare, Control, Document, ElementRef, Entries, EventBinding, HandlerRef,
    Node, Path, PathSegment, Segment, Unit, Value,
};
use gpui_kit::component::Colorize;
use gpui_kit::component::TitleBar;
use gpui_kit::component::WindowExt;
use gpui_kit::component::accordion::Accordion;
use gpui_kit::component::alert::{Alert, AlertVariant};
use gpui_kit::component::attachment::{
    Attachment, AttachmentContent, AttachmentDescription, AttachmentTitle,
};
use gpui_kit::component::avatar::{Avatar, AvatarGroup};
use gpui_kit::component::badge::Badge;
use gpui_kit::component::breadcrumb::{Breadcrumb, BreadcrumbItem};
use gpui_kit::component::bubble::{Bubble, BubbleVariant};
use gpui_kit::component::button::{Button, ButtonVariant, ButtonVariants};
use gpui_kit::component::calendar::{Calendar, CalendarState};
use gpui_kit::component::carousel::{
    Carousel, CarouselContent, CarouselItem, CarouselNext, CarouselPagination, CarouselPrevious,
    CarouselState,
};
use gpui_kit::component::chart::{BarChart, LineChart, PieChart, RadarChart, SankeyChart};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::clipboard::Clipboard;
use gpui_kit::component::collapsible::Collapsible;
use gpui_kit::component::color_picker::{ColorPicker, ColorPickerEvent, ColorPickerState};
use gpui_kit::component::combobox::{Combobox, ComboboxEvent, ComboboxState};
use gpui_kit::component::command::{Command, CommandGroup, CommandItem, CommandState};
use gpui_kit::component::date_picker::{DatePicker, DatePickerState};
use gpui_kit::component::description_list::{DescriptionItem, DescriptionList};
use gpui_kit::component::dialog::AlertDialog;
use gpui_kit::component::diff::{Diff, DiffFile, DiffState};
use gpui_kit::component::empty::{
    Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyMediaVariant, EmptyTitle,
};
use gpui_kit::component::form::{Field, Form};
use gpui_kit::component::group_box::{GroupBox, GroupBoxVariant, GroupBoxVariants};
use gpui_kit::component::hover_card::HoverCard;
use gpui_kit::component::input::{
    Editor, EditorState, Input, InputEvent, InputState, NumberInput, OtpEvent, OtpInput, OtpState,
    Textarea, TextareaState,
};
use gpui_kit::component::input::{
    InputGroup, InputGroupAddon, InputGroupAddonAlignment, InputGroupButton, InputGroupText,
};
use gpui_kit::component::kbd::Kbd;
use gpui_kit::component::label::Label;
use gpui_kit::component::link::Link;
use gpui_kit::component::list::{List, ListDelegate, ListEvent, ListItem, ListState};
use gpui_kit::component::marker::{Marker, MarkerAlignment, MarkerContent, MarkerVariant};
use gpui_kit::component::menu::{ContextMenuExt as _, DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::message::{
    Message, MessageAlignment, MessageAvatar, MessageContent, MessageFooter, MessageHeader,
};
use gpui_kit::component::notification::{Notification, NotificationType};
use gpui_kit::component::pagination::Pagination;
use gpui_kit::component::plot::shape::SankeyLink;
use gpui_kit::component::popover::Popover;
use gpui_kit::component::progress::{Progress, ProgressCircle};
use gpui_kit::component::questionnaire::{
    Questionnaire, QuestionnaireChoiceDefinition, QuestionnaireItemDefinition, QuestionnaireState,
};
use gpui_kit::component::radio::{Radio, RadioGroup};
use gpui_kit::component::rating::Rating;
use gpui_kit::component::resizable::{
    ResizablePanel, ResizableState, h_resizable, resizable_panel, v_resizable,
};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::separator::Separator;
use gpui_kit::component::setting::{SettingGroup, SettingItem, SettingPage, Settings};
use gpui_kit::component::shimmer::ShimmerText;
use gpui_kit::component::sidebar::{Sidebar, SidebarCollapsible, SidebarMenu, SidebarMenuItem};
use gpui_kit::component::skeleton::Skeleton;
use gpui_kit::component::slider::{Slider, SliderEvent, SliderState, SliderValue};
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::status_bar::StatusBar;
use gpui_kit::component::stepper::{Stepper, StepperItem};
use gpui_kit::component::switch::Switch;
use gpui_kit::component::tab::{Tab, TabBar, TabVariant};
use gpui_kit::component::table::{
    Table, TableBody, TableCaption, TableCell, TableFooter, TableHead, TableHeader,
};
use gpui_kit::component::tag::{Tag, TagVariant};
use gpui_kit::component::text::TextView;
use gpui_kit::component::time_field::{TimeField, TimeFieldState};
use gpui_kit::component::toolbar::Toolbar;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::tree::{Tree, TreeItem, TreeState};
use gpui_kit::component::{
    ActiveTheme, Disableable, FocusTrapElement, Icon, IndexPath, Selectable, Sizable, Size, v_flex,
};
use gpui_kit::{prelude::*, *};
use serde_json::Value as Json;

/// Called with `hook(args)` when an event fires and no hook is registered for its name.
pub type HookSink = Rc<dyn Fn(&str, &mut ForgeView, &mut Context<ForgeView>)>;

gpui::actions!(gpui_forge_runtime, [CancelPick, SaveDocument]);

/// A drag payload: `Component` comes from palette items (`insert_component`
/// handlers), `Node` from existing document nodes (reparent drags).
pub enum ForgeDrag {
    Component(String),
    Node(String),
}

/// The floating chip rendered while dragging.
struct DragGhost(SharedString);

impl Render for DragGhost {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .px_2()
            .py_1()
            .rounded_md()
            .text_sm()
            .bg(theme.accent)
            .text_color(theme.accent_foreground)
            .child(self.0.clone())
            .into_any_element()
    }
}

/// Panics caught inside `update` calls where the view itself is unreachable — a
/// double-leased entity can't be updated to report its own error. Drained by
/// `take_errors` on the next paint.
static PENDING_ERRORS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// `weak.update` wrapped in `catch_unwind` — a reentrant entity update panics
/// inside gpui, and this converts it into a reported error instead of a crash.
fn try_update<F, R>(weak: &WeakEntity<ForgeView>, cx: &mut App, what: &str, f: F) -> Result<R, ()>
where
    F: FnOnce(&mut ForgeView, &mut Context<ForgeView>) -> R,
{
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        weak.update(cx, f).map_err(|_| ())
    }));
    match result {
        Ok(result) => result,
        Err(payload) => {
            let detail = payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown panic".to_owned());
            // The entity is leased (that's why the update panicked) — defer the
            // report so it lands after the lease releases and forces a repaint.
            let weak = weak.clone();
            let what = what.to_owned();
            cx.defer(move |cx| {
                weak.update(cx, |view, cx| {
                    view.report(format!("{what} failed: {detail}"));
                    cx.notify();
                })
                .ok();
            });
            Err(())
        }
    }
}
/// GPUI keystroke → the logical name a ratatui preview host's `on_key`
/// expects. `key_char` carries Shift for characters. Ctrl/cmd chords have no
/// wire name yet — dropped rather than sent as a bare letter.
fn pane_key_name(ks: &Keystroke) -> Option<String> {
    if ks.modifiers.control || ks.modifiers.platform {
        return None;
    }
    if ks.key.chars().count() == 1 {
        return Some(ks.key_char.clone().unwrap_or_else(|| ks.key.clone()));
    }
    let name = match ks.key.as_str() {
        "tab" => {
            if ks.modifiers.shift {
                "BackTab"
            } else {
                "Tab"
            }
        }
        "enter" => "Enter",
        "backspace" => "Backspace",
        "space" => " ",
        "up" => "Up",
        "down" => "Down",
        "left" => "Left",
        "right" => "Right",
        _ => return None,
    };
    Some(name.to_string())
}

/// Maps a pointer position inside a stream/local card to its `(col, row)`
/// cell. The cell comes from the grid's tracked rect and the same monospace
/// metrics it paints with, so clicks land on the drawn cell. `term_hover`
/// remembers the result — `on_drop` carries no position.
fn term_cell(
    view: &mut ForgeView,
    id: &str,
    pos: Point<Pixels>,
    window: &Window,
    family: &SharedString,
) -> Option<(u16, u16)> {
    let origin = view
        .term_bounds
        .get(id)
        .and_then(|cell| cell.borrow().map(|b| b.origin))?;
    // The published metrics decide the cell pitch — recomputing here would
    // drift from the painted grid (TermView floors cell_h at 1.2em).
    let (cell_w, cell_h) = match view.term_metrics.get(id).and_then(|cell| *cell.borrow()) {
        Some(m) => m,
        None => {
            let ts = window.text_system();
            let font_id = ts.resolve_font(&Font {
                family: family.clone(),
                ..Default::default()
            });
            let w = ts
                .advance(font_id, px(12.), '0')
                .map(|s| s.width)
                .unwrap_or(px(12.) * 0.6);
            let h = (ts.ascent(font_id, px(12.)) + ts.descent(font_id, px(12.))).max(px(12.) * 1.2);
            (w, h)
        }
    };
    let dx = pos.x - origin.x;
    let dy = pos.y - origin.y;
    if dx < px(0.) || dy < px(0.) {
        return None;
    }
    let cell = ((dx / cell_w) as u16, (dy / cell_h) as u16);
    view.term_hover.insert(id.to_string(), cell);
    Some(cell)
}

/// A pointer event on a stream card → `pane_mouse(id, kind, col, row)`.
fn send_pane_mouse(
    view: &mut ForgeView,
    id: &str,
    kind: &str,
    pos: Point<Pixels>,
    window: &Window,
    cx: &mut Context<ForgeView>,
) {
    let family = cx.theme().mono_font_family.clone();
    let Some((col, row)) = term_cell(view, id, pos, window, &family) else {
        return;
    };
    view.fire_call(
        "pane_mouse",
        vec![
            id.to_string(),
            kind.to_string(),
            col.to_string(),
            row.to_string(),
        ],
        cx,
    );
}

/// #156 — what a local ratatui pane's sinks queue. The pane's
/// `ratatui::ForgeView` fires hooks and state writes through sinks that get
/// `&mut ForgeView` — the TUI contract has no `Context`. The closures push
/// here, and `drain_tui_outbox` dispatches them on the event path.
#[cfg(feature = "ratatui-embed")]
enum TuiOut {
    /// A doc `on <event>:` call, formatted `name(a, b)` — same wire shape
    /// the preview host emits.
    Call(String),
    /// The pane's whole view state after a binding write.
    State(Json),
}

/// Splits a `name(a, b)` wire call — the pane-twin of Studio's `parse_call`.
#[cfg(feature = "ratatui-embed")]
fn parse_tui_call(call: &str) -> (String, Vec<String>) {
    match call.split_once('(') {
        Some((name, rest)) => {
            let args = rest.trim_end_matches(')');
            let args = if args.trim().is_empty() {
                Vec::new()
            } else {
                args.split(", ").map(|a| a.trim().to_owned()).collect()
            };
            (name.trim().to_owned(), args)
        }
        None => (call.trim().to_owned(), Vec::new()),
    }
}

/// Drains queued pane hooks/state onto the shell — calls dispatch through
/// `fire_call` (Studio's `pane_drop_at`/`select_node` arms), state writes
/// mirror `document.selected`/`studio.drop_target` back like the preview
/// entity's `on_state` sink does.
#[cfg(feature = "ratatui-embed")]
fn drain_tui_outbox(view: &mut ForgeView, cx: &mut Context<ForgeView>) {
    let out = std::mem::take(&mut *view.tui_outbox.borrow_mut());
    for item in out {
        match item {
            TuiOut::Call(call) => {
                let (name, args) = parse_tui_call(&call);
                view.fire_call(&name, args, cx);
            }
            TuiOut::State(state) => {
                for key in ["document.selected", "studio.drop_target"] {
                    if let Ok(path) = parse_path(key) {
                        let value = get_path(&state, &path).cloned().unwrap_or(Json::Null);
                        if get_path(view.state(), &path) != Some(&value) {
                            view.update_state(&path, value, cx);
                        }
                    }
                }
            }
        }
    }
}

/// A ratatui `Buffer` → the `GridView`'s snapshot rows. `Reset` maps to the
/// style's default fg/bg. Named ANSI colors keep their palette index.
#[cfg(feature = "ratatui-embed")]
fn buffer_rows(buf: &::ratatui::buffer::Buffer) -> Vec<Vec<forge_term::CellSnapshot>> {
    use ::ratatui::buffer::CellDiffOption;
    use ::ratatui::style::{Color, Modifier};
    use forge_term::{CellColor, CellSnapshot};
    let map_color = |c: Color, default: CellColor| match c {
        Color::Reset => default,
        Color::Black => CellColor::Indexed(0),
        Color::Red => CellColor::Indexed(1),
        Color::Green => CellColor::Indexed(2),
        Color::Yellow => CellColor::Indexed(3),
        Color::Blue => CellColor::Indexed(4),
        Color::Magenta => CellColor::Indexed(5),
        Color::Cyan => CellColor::Indexed(6),
        Color::Gray => CellColor::Indexed(7),
        Color::DarkGray => CellColor::Indexed(8),
        Color::LightRed => CellColor::Indexed(9),
        Color::LightGreen => CellColor::Indexed(10),
        Color::LightYellow => CellColor::Indexed(11),
        Color::LightBlue => CellColor::Indexed(12),
        Color::LightMagenta => CellColor::Indexed(13),
        Color::LightCyan => CellColor::Indexed(14),
        Color::White => CellColor::Indexed(15),
        Color::Indexed(i) => CellColor::Indexed(i),
        Color::Rgb(r, g, b) => CellColor::Rgb(r, g, b),
    };
    let area = buf.area;
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| {
                    let Some(cell) = buf.cell((x, y)) else {
                        return CellSnapshot {
                            ch: ' ',
                            fg: CellColor::Foreground,
                            bg: CellColor::Background,
                            bold: false,
                            dim: false,
                            italic: false,
                            underline: false,
                            inverse: false,
                            hidden: false,
                            wide: false,
                            wide_spacer: false,
                        };
                    };
                    let m = cell.modifier;
                    CellSnapshot {
                        ch: cell.symbol().chars().next().unwrap_or(' '),
                        fg: map_color(cell.fg, CellColor::Foreground),
                        bg: map_color(cell.bg, CellColor::Background),
                        bold: m.contains(Modifier::BOLD),
                        dim: m.contains(Modifier::DIM),
                        italic: m.contains(Modifier::ITALIC),
                        underline: m.contains(Modifier::UNDERLINED),
                        inverse: m.contains(Modifier::REVERSED),
                        hidden: m.contains(Modifier::HIDDEN),
                        wide: buf
                            .cell((x + 1, y))
                            .is_some_and(|next| next.diff_option == CellDiffOption::Skip),
                        wide_spacer: cell.diff_option == CellDiffOption::Skip,
                    }
                })
                .collect()
        })
        .collect()
}

/// The card's frame size in cells — the painted rect over the real cell
/// metrics, same formula Studio sizes remote pane frames with. `None` until
/// the card has painted once.
#[cfg(feature = "ratatui-embed")]
fn tui_frame_size(view: &ForgeView, id: &str) -> Option<(u16, u16)> {
    let bounds = (*view.term_bounds.get(id)?.borrow())?;
    let (cw, ch) = view.term_metrics.get(id).and_then(|cell| *cell.borrow())?;
    let cols = (f64::from(bounds.size.width) / f64::from(cw))
        .floor()
        .max(4.) as u16;
    let rows = (f64::from(bounds.size.height) / f64::from(ch))
        .floor()
        .max(2.) as u16;
    Some((cols, rows))
}

/// Called with the whole view state after a two-way binding writes to it.
pub type StateSink = Rc<dyn Fn(&Json, &mut ForgeView, &mut Context<ForgeView>)>;
/// Rust logic behind `on <event>: name(args)` — the hook registry (#52).
pub type HookHandler = Rc<dyn Fn(&[String], &mut ForgeView, &mut Context<ForgeView>)>;

type Options = Vec<SharedString>;

/// Shared cell-metrics slot — structurally `forge_term::CellMetrics`, kept
/// local so this file's wasm build never names `forge_term`.
type CellMetrics = std::rc::Rc<std::cell::RefCell<Option<(Pixels, Pixels)>>>;

/// An external element library plugged into the runtime — the adapter contract
/// from the multi-base RFC: alias + catalog + a render shim for `alias.Name`
/// elements.
pub trait ForgeLibrary {
    /// The `use:` alias this library handles (e.g. "ely" for `ely.VideoPlayer`).
    fn alias(&self) -> &'static str;
    /// The library's catalog manifest, merged into the view's catalogs on
    /// registration so validation/props resolve.
    fn catalog(&self) -> Option<&Catalog> {
        None
    }
    /// Renders `alias.<element>`. `None` falls through to the unknown-element
    /// placeholder. The view is read-only here — a shim builds native elements
    /// from the node + its own state and window/cx.
    fn render(
        &self,
        node: &Node,
        key: &str,
        view: &ForgeView,
        window: &mut Window,
        cx: &mut Context<ForgeView>,
    ) -> Option<AnyElement>;
}

pub struct ForgeView {
    doc: Option<Document>,
    pub catalogs: Vec<Catalog>,
    /// The document's `meta:` map — `{$meta.k}` bindings resolve against it.
    meta: Json,
    state: Json,
    inputs: HashMap<String, Entity<InputState>>,
    textareas: HashMap<String, Entity<TextareaState>>,
    editors: HashMap<String, Entity<EditorState>>,
    sliders: HashMap<String, Entity<SliderState>>,
    color_pickers: HashMap<String, Entity<ColorPickerState>>,
    otps: HashMap<String, Entity<OtpState>>,
    date_pickers: HashMap<String, Entity<DatePickerState>>,
    calendars: HashMap<String, Entity<CalendarState>>,
    time_fields: HashMap<String, Entity<TimeFieldState>>,
    lists: HashMap<String, Entity<ListState<ForgeListDelegate>>>,
    trees: HashMap<String, Entity<TreeState>>,
    /// `forge.Files` `root:` scans — keyed by `key|root|filter|depth|hidden|nonce`
    /// so a `nonce:` bump (or the refresh button) is what rescans.
    files_cache: HashMap<String, Vec<Json>>,
    /// `forge.Files` search text per element key — the `query:` prop can also
    /// bind it into document state for hosts to read.
    files_queries: HashMap<String, String>,
    /// `forge.Files` `folders: navigate` — current directory per element key.
    /// Always clamped back under `root:` at render time, so a project switch
    /// or a removed folder self-heals without explicit resets.
    files_nav: HashMap<String, String>,
    /// `forge.Icons` search text per element key — `query:` can bind it out.
    icons_queries: HashMap<String, String>,
    /// `forge.Icons` grid scroll handles per element key — the handle's
    /// bounds are how the cell-column count tracks the rendered width.
    icons_scrolls: HashMap<String, UniformListScrollHandle>,
    /// The host's iconset (`[icons] set` in forge.toml) — a document's
    /// `meta.iconset` overrides it, and `lucide` is the bundled default.
    project_iconset: Option<String>,
    selects: HashMap<String, Entity<SelectState<Options>>>,
    /// Persistent `ResizableState`s keyed by element key — `use_keyed_state`
    /// orphans panel state when the element-id stack shifts. A stateful
    /// sibling or ancestor id toggling is enough. A fresh state lays panels
    /// out at their minimums. Owning the entity keeps user-dragged sizes alive.
    resizables: HashMap<String, Entity<ResizableState>>,
    /// The shared `ThemeSwitcher` select — one Light/Dark state per view.
    theme_select: Option<Entity<SelectState<Options>>>,
    comboboxes: HashMap<String, Entity<ComboboxState<Vec<SharedString>>>>,
    carousels: HashMap<String, Entity<CarouselState>>,
    commands: HashMap<String, Entity<CommandState>>,
    questionnaires: HashMap<String, Entity<QuestionnaireState>>,
    /// `Diff` state per element key, with the patch text it was built from —
    /// `set_files` is only re-parsed when the `patch:` input changes.
    diffs: HashMap<String, (String, Entity<DiffState>)>,
    tabs: HashMap<String, usize>,
    previews: HashMap<String, Entity<ForgeView>>,
    /// `forge.Preview` id → the `doc:` src it has loaded — skips re-parsing
    /// unchanged preview documents each render.
    preview_srcs: HashMap<String, String>,
    /// `forge.Preview` bounds cells — `id` → the card's window-space rect,
    /// populated by `BoundsTracker` each paint (Studio's `frame_rect`).
    preview_bounds: HashMap<String, std::rc::Rc<std::cell::RefCell<Option<Bounds<Pixels>>>>>,
    /// `forge.Preview` remote panes with `mode: stream` — the pane's
    /// terminal emulator (fed ANSI bytes from `preview.panes.<id>.frame`
    /// chunks, deduped by `seq`) and the seq it last consumed (#120).
    /// Native-only: `forge_term`/`alacritty_terminal` don't build on wasm.
    #[cfg(not(target_arch = "wasm32"))]
    term_panes: HashMap<String, (u64, std::rc::Rc<std::cell::RefCell<forge_term::Emulator>>)>,
    /// `forge.Preview` stream cards — the `TermView`'s painted rect, tracked
    /// each paint so pointer events map to (col, row) cells.
    term_bounds: HashMap<String, std::rc::Rc<std::cell::RefCell<Option<Bounds<Pixels>>>>>,
    /// One stable `FocusHandle` per stream card. A fresh handle each render
    /// drops focus between frames, and `on_key_down` never fires.
    term_focus: HashMap<String, FocusHandle>,
    /// `forge.Preview` stream cards — the `TermView`'s measured
    /// (`cell_w`, `cell_h`), published each prepaint so the sender sizes
    /// the host grid by real metrics, not a px-per-cell guess.
    /// Native-only populated — `forge_term` doesn't build on wasm.
    term_metrics: HashMap<String, CellMetrics>,
    /// `forge.Preview` stream cards — the last cell a pointer event mapped
    /// to. `on_drop` carries no position, so a drop reuses the last hover.
    term_hover: HashMap<String, (u16, u16)>,
    /// `forge.Preview` stream cards — `id` → the pointer is dragging inside
    /// the card. Set on the first dragging move, read on `up` for the
    /// in-card node move (#156).
    term_drag: HashMap<String, bool>,
    /// #156 — `forge.Preview` on a ratatui-base doc draws in-process: the
    /// pane's `ratatui::ForgeView`, doc-mirrored from the `previews` entity
    /// each render. Replaces the remote stream for the edited doc.
    #[cfg(feature = "ratatui-embed")]
    tui_panes: HashMap<String, crate::render::ratatui::ForgeView>,
    /// The last-rendered cell rows each `GridView` paints.
    #[cfg(feature = "ratatui-embed")]
    tui_grids: HashMap<String, std::rc::Rc<std::cell::RefCell<Vec<Vec<forge_term::CellSnapshot>>>>>,
    /// Pane sinks queue here — `drain_tui_outbox` dispatches on the event
    /// path, where a `Context` exists.
    #[cfg(feature = "ratatui-embed")]
    tui_outbox: std::rc::Rc<std::cell::RefCell<Vec<TuiOut>>>,
    /// Persistent `ScrollHandle`s keyed by scrollable element id — element-state
    /// GC drops scroll offsets on every `cx.notify` re-render. An explicit
    /// `track_scroll` handle keeps them alive.
    scroll_handles: HashMap<String, ScrollHandle>,
    pub libraries: Vec<Rc<dyn ForgeLibrary>>,
    hooks: HashMap<String, HookHandler>,
    on_hook: Option<HookSink>,
    on_state: Option<StateSink>,
    subscriptions: Vec<Subscription>,
    /// Render-time problems surfaced for the host — `take_errors` drains them.
    errors: Vec<String>,
    /// The directory of the document this view renders — `src` paths on
    /// `Svg`/image elements resolve relative to it.
    base_dir: Option<std::path::PathBuf>,
    /// Element-picker mode — rendered nodes get a click overlay that writes
    /// `document.selected` (the host's structure/inspector selection).
    pick: bool,
    /// Click-to-place mode — the pick click writes `studio.drop_target` and
    /// fires `place_component` instead of selecting.
    placing: bool,
    /// Pick hit zones — `(bounds, path)` per rendered node, written by each
    /// node's zone `canvas` during prepaint while `pick` is on. The single
    /// top-level pick overlay hit-tests these to find the deepest node under
    /// the pointer. Nested per-node overlays can't, because the root's
    /// overlay shadows every descendant in dispatch order.
    pick_zones: PickZones,
    /// The pick zone under the pointer — each zone canvas paints its accent
    /// frame while its path matches, so only the deepest node highlights.
    pick_hover: Rc<std::cell::RefCell<Option<String>>>,
    /// Drag & drop — rendered nodes are drag sources (`ForgeDrag::Node`) and
    /// drop targets (writes `studio.drop_target`, fires `drop_component` /
    /// `move_node`). Host enables on the preview.
    dnd: bool,
    /// Shown while there is no document (e.g. inside `forge.Preview`).
    empty_text: Option<String>,
    /// `for`/`with` scope vars — `(name, value)` frames pushed while control
    /// blocks render. `$item` lookups shadow state from the innermost frame.
    scopes: Vec<(String, Json)>,
}

type PickZones = Rc<std::cell::RefCell<Vec<(Bounds<Pixels>, String)>>>;

/// A `ListDelegate` that renders `List` children on demand. `render_item` is called
/// several times per row per paint (measure then paint). Rows are rebuilt per call
/// through the owning view rather than handed out once.
struct ForgeListDelegate {
    view: WeakEntity<ForgeView>,
    key: String,
    children: Vec<Node>,
    selected: Rc<std::cell::Cell<Option<IndexPath>>>,
}

impl ForgeListDelegate {
    /// A single `for:` child is the row template — one virtualized row per
    /// resolved item instead of one row rendering the whole loop eagerly.
    /// Item rows re-resolve each call, so bound lists stay live. An empty
    /// source renders `else:` as the only row.
    fn for_control(&self) -> Option<&Control> {
        match self.children.as_slice() {
            [child] => match &child.control {
                Some(c @ Control::For { .. }) => Some(c),
                _ => None,
            },
            _ => None,
        }
    }
}

impl ListDelegate for ForgeListDelegate {
    type Item = ListItem;

    fn items_count(&self, _: usize, app: &gpui_kit::App) -> usize {
        if let Some(Control::For {
            source, otherwise, ..
        }) = self.for_control()
        {
            let n = self
                .view
                .upgrade()
                .map(|view| view.read(app).for_items(source).len())
                .unwrap_or(0);
            return usize::from(n == 0 && !otherwise.is_empty()).max(n);
        }
        self.children.len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let key = format!("{}/{}", self.key, ix.row);
        if self.for_control().is_some() {
            let node = self.children[0].clone();
            let key2 = key.clone();
            let els = try_update(&self.view, cx, "list row", |view, cx| {
                let Some(Control::For {
                    source,
                    each,
                    index,
                    key: key_path,
                    otherwise,
                }) = node.control.clone()
                else {
                    return Vec::new();
                };
                match view.for_items(&source).get(ix.row).cloned() {
                    Some(item) => {
                        view.scopes.push((each, item));
                        if let Some(index) = &index {
                            view.scopes.push((index.clone(), Json::from(ix.row)));
                        }
                        // `key: item.id` — resolve the member path against
                        // the scoped item for stable identity (default:
                        // position), same as `expand`'s for branch.
                        let item_key = key_path
                            .as_ref()
                            .and_then(|path| {
                                let resolved = view.resolve(&Value::Binding {
                                    path: path.clone(),
                                    mode: BindingMode::OneWay,
                                });
                                resolved
                                    .as_str()
                                    .map(str::to_owned)
                                    .or_else(|| (!resolved.is_null()).then(|| resolved.to_string()))
                            })
                            .unwrap_or_else(|| ix.row.to_string());
                        let out =
                            view.render_target(&node, &format!("{key2}/{item_key}"), window, cx);
                        view.scopes
                            .truncate(view.scopes.len() - 1 - usize::from(index.is_some()));
                        out
                    }
                    // Row 0 on an empty source renders `else:`.
                    None => view.render_kids(&otherwise, &key, window, cx),
                }
            });
            return match els {
                Ok(els) => Some(ListItem::new(ix).children(els)),
                Err(()) => None,
            };
        }
        let node = self.children.get(ix.row)?.clone();
        try_update(&self.view, cx, "list render", |view, cx| {
            ListItem::new(ix).children(view.expand(&node, &key, window, cx))
        })
        .ok()
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) {
        self.selected.set(ix);
    }
}

impl ForgeView {
    pub fn new(catalogs: Vec<Catalog>) -> Self {
        Self {
            doc: None,
            catalogs,
            meta: Json::Object(Default::default()),
            state: Json::Object(Default::default()),
            inputs: HashMap::new(),
            textareas: HashMap::new(),
            editors: HashMap::new(),
            sliders: HashMap::new(),
            color_pickers: HashMap::new(),
            otps: HashMap::new(),
            date_pickers: HashMap::new(),
            calendars: HashMap::new(),
            time_fields: HashMap::new(),
            lists: HashMap::new(),
            trees: HashMap::new(),
            files_cache: HashMap::new(),
            files_queries: HashMap::new(),
            files_nav: HashMap::new(),
            icons_queries: HashMap::new(),
            icons_scrolls: HashMap::new(),
            project_iconset: None,
            selects: HashMap::new(),
            resizables: HashMap::new(),
            theme_select: None,
            comboboxes: HashMap::new(),
            carousels: HashMap::new(),
            commands: HashMap::new(),
            questionnaires: HashMap::new(),
            diffs: HashMap::new(),
            tabs: HashMap::new(),
            preview_bounds: HashMap::new(),
            #[cfg(not(target_arch = "wasm32"))]
            term_panes: HashMap::new(),
            term_bounds: HashMap::new(),
            term_focus: HashMap::new(),
            term_metrics: HashMap::new(),
            term_hover: HashMap::new(),
            term_drag: HashMap::new(),
            #[cfg(feature = "ratatui-embed")]
            tui_panes: HashMap::new(),
            #[cfg(feature = "ratatui-embed")]
            tui_grids: HashMap::new(),
            #[cfg(feature = "ratatui-embed")]
            tui_outbox: std::rc::Rc::new(std::cell::RefCell::new(Vec::new())),
            preview_srcs: HashMap::new(),
            scroll_handles: HashMap::new(),
            previews: HashMap::new(),
            libraries: Vec::new(),
            hooks: HashMap::new(),
            on_hook: None,
            on_state: None,
            subscriptions: Vec::new(),
            errors: Vec::new(),
            base_dir: None,
            pick: false,
            placing: false,
            pick_zones: Rc::new(std::cell::RefCell::new(Vec::new())),
            pick_hover: Rc::new(std::cell::RefCell::new(None)),
            dnd: false,
            empty_text: None,
            scopes: Vec::new(),
        }
    }

    pub fn set_document(&mut self, doc: Document, cx: &mut Context<Self>) {
        self.meta = crate::render::doc_meta(&doc);
        // Seed `state:` declarations — a declared-but-unwritten path gets its
        // type's zero value so bound widgets and `forge.Vars` see it, the
        // same boilerplate `forge build --codegen` emits for built apps.
        // Only absent/null paths seed: live values survive a reload.
        for (path, ty) in &doc.state {
            let Ok(p) = parse_path(path) else { continue };
            let missing = get_path(&self.state, &p).is_none_or(|v| v.is_null());
            if missing {
                set_path(&mut self.state, &p, state_default(ty));
            }
        }
        self.doc = Some(doc);
        cx.notify();
    }

    pub fn set_state(&mut self, state: Json, cx: &mut Context<Self>) {
        self.state = state;
        // Inputs and selects copy their value once. Rebuild them from the new state.
        self.inputs.clear();
        self.textareas.clear();
        self.editors.clear();
        self.sliders.clear();
        self.color_pickers.clear();
        self.otps.clear();
        self.date_pickers.clear();
        self.calendars.clear();
        self.time_fields.clear();
        self.lists.clear();
        self.trees.clear();
        self.selects.clear();
        self.comboboxes.clear();
        self.carousels.clear();
        self.commands.clear();
        self.questionnaires.clear();
        self.diffs.clear();
        self.subscriptions.clear();
        cx.notify();
    }

    /// The view state that bindings read from and write to.
    pub fn state(&self) -> &Json {
        &self.state
    }

    /// The document's `meta:` map — `meta.watch` opts a file into
    /// hot-reload.
    pub fn meta(&self) -> &Json {
        &self.meta
    }

    /// Writes `value` at `path` in the view state without rebuilding inputs.
    /// The parsed document this view renders, if any.
    pub fn document(&self) -> Option<&Document> {
        self.doc.as_ref()
    }

    /// Drops a cached input/editor state so the next render rebuilds it from bound
    /// state — pushes generated text into the source pane after a model edit.
    pub fn reset_inputs(&mut self, key: &str) {
        self.inputs.remove(key);
        self.textareas.remove(key);
        self.editors.remove(key);
    }

    pub fn update_state(&mut self, path: &Path, value: Json, cx: &mut Context<Self>) {
        self.write(path, value, cx);
    }

    /// Text displayed while the view has no document (used by `forge.Preview`).
    pub fn set_empty_text(&mut self, text: Option<String>) {
        self.empty_text = text;
    }

    /// Registers `handler` for `on <event>: name(args)` bindings. Handlers may edit this
    /// view (state, previews) freely. Unregistered names are reported to the hook sink.
    pub fn set_hook(&mut self, name: impl Into<String>, handler: HookHandler) {
        self.hooks.insert(name.into(), handler);
    }

    /// The `forge.Preview` card's window-space bounds for `id` — populated
    /// by `BoundsTracker` on paint. `None` before the first frame.
    pub fn preview_bounds(
        &self,
        id: &str,
    ) -> Option<std::rc::Rc<std::cell::RefCell<Option<Bounds<Pixels>>>>> {
        self.preview_bounds.get(id).cloned()
    }

    /// The stream card's painted `TermView` rect — `None` before the first
    /// paint. Tighter than `preview_bounds`: it excludes the card padding.
    pub fn term_bounds(
        &self,
        id: &str,
    ) -> Option<std::rc::Rc<std::cell::RefCell<Option<Bounds<Pixels>>>>> {
        self.term_bounds.get(id).cloned()
    }

    /// The stream card's measured terminal cell size — `None` before the
    /// first paint. The sender turns a px slot into `frame_rect` cols/rows
    /// with it so the host grid matches the painted grid exactly.
    pub fn term_metrics(&self, id: &str) -> Option<CellMetrics> {
        self.term_metrics.get(id).cloned()
    }

    /// Feed one streaming `frame` event into the pane's terminal
    /// emulator — resize to the frame grid, then append the bytes, once
    /// per seq. This must run in the event path. The state slot
    /// `preview.panes.<id>.frame` keeps only the LAST event of each drain
    /// batch. Render-time feeding drops every burst frame but the final
    /// diff, so the full first frame is always lost.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn feed_term_frame(&mut self, id: &str, seq: u64, cols: u16, rows: u16, data: &[u8]) {
        let entry = self.term_panes.entry(id.to_owned()).or_insert_with(|| {
            (
                0,
                std::rc::Rc::new(std::cell::RefCell::new(forge_term::Emulator::new(
                    cols, rows,
                ))),
            )
        });
        // A respawned host restarts seq at 1. Its stream is a fresh
        // terminal — replace the emulator instead of layering the new
        // stream onto the dead host's scrollback.
        if seq < entry.0 {
            *entry = (
                0,
                std::rc::Rc::new(std::cell::RefCell::new(forge_term::Emulator::new(
                    cols, rows,
                ))),
            );
        }
        if seq != entry.0 {
            {
                let mut emu = entry.1.borrow_mut();
                if emu.cols() != cols as usize || emu.rows() != rows as usize {
                    // Grid change: start a fresh emulator rather than
                    // resize-preserving. Ratatui emits a sparse diff —
                    // only cells that differ from EMPTY get written.
                    // Preserved reflow content survives under new-empty
                    // cells and ghosts the old layout over the new.
                    *emu = forge_term::Emulator::new(cols, rows);
                }
                emu.feed(data);
            }
            entry.0 = seq;
        }
    }

    /// #156 — `forge.Preview` on a ratatui-base doc renders in-process.
    /// The pane's `ratatui::ForgeView` draws into a headless `TestBackend`
    /// buffer — the same loop `forge-preview ratatui` runs, minus the wire.
    /// The cells paint through `GridView`. Input, zone resolution and drop
    /// commit all stay local. `None` keeps the caller on the compat
    /// path (a non-ratatui doc).
    #[cfg(feature = "ratatui-embed")]
    fn tui_card(
        &mut self,
        node: &Node,
        id: &str,
        preview: &Entity<ForgeView>,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let preview_view = preview.read(cx);
        let doc = preview_view.doc.clone()?;
        if doc.base.as_deref() != Some("ratatui") {
            return None;
        }
        let base_dir = preview_view.base_dir.clone();
        // Pull every theme value out eagerly — `cx.theme()` borrows `cx`,
        // which the handlers' `focus_handle`/`try_update` calls need mutably.
        let theme = cx.theme();
        let (bg, fg, mono, dark) = (
            theme.background,
            theme.foreground,
            theme.mono_font_family.clone(),
            theme.is_dark(),
        );
        let colors = serde_json::to_value(theme.colors)
            .ok()
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();
        // DnD edits the document the pane mirrors — only a `doc:`-less pane
        // previews studio.doc (#156).
        let editable = self.prop_text(node, "doc").is_none_or(|d| d.is_empty());
        // `document.selected` mirrors into the pane — hoisted before the
        // `tui_panes` borrow since it reads the shell's state.
        let selected = parse_path("document.selected").ok().map(|path| {
            let value = get_path(self.state(), &path).cloned().unwrap_or(Json::Null);
            (path, value)
        });
        // Frame size follows the painted rect — also before the pane borrow.
        let (cols, rows) = tui_frame_size(self, id).unwrap_or((80, 24));
        let catalogs = self.catalogs.clone();
        let outbox = self.tui_outbox.clone();
        let tui = self.tui_panes.entry(id.to_owned()).or_insert_with(|| {
            let mut view = crate::render::ratatui::ForgeView::new(catalogs);
            view.set_dnd(editable);
            if let Some(dir) = base_dir {
                view.set_base_dir(dir);
            }
            let calls = outbox.clone();
            let states = outbox.clone();
            view.set_sinks(
                std::rc::Rc::new(move |call, _| {
                    calls.borrow_mut().push(TuiOut::Call(call.to_owned()));
                }),
                std::rc::Rc::new(move |state, _| {
                    states.borrow_mut().push(TuiOut::State(state.clone()));
                }),
            );
            view
        });
        // The preview entity stays the doc store — every studio write path
        // flows through it untouched. Insert, remove and prop edits land
        // there, and the pane mirrors whatever it holds each frame.
        if tui.document() != Some(&doc) {
            tui.set_document(doc);
        }
        // Outline/inspector selection arrives as `document.selected` —
        // echoed so the pane paints the frame it owns.
        if let Some((path, value)) = selected
            && get_path(tui.state(), &path) != Some(&value)
        {
            tui.update_state(&path, value);
        }
        // `{$theme.*}` bindings resolve off the shell's resolved palette.
        tui.set_theme(dark, &colors);

        // Draw — 80×24 until the card has painted once.
        let mut term = ::ratatui::Terminal::new(::ratatui::backend::TestBackend::new(cols, rows))
            .expect("test backend");
        term.draw(|frame| tui.render(frame)).ok();
        let grid = self
            .tui_grids
            .entry(id.to_owned())
            .or_insert_with(|| std::rc::Rc::new(std::cell::RefCell::new(Vec::new())))
            .clone();
        *grid.borrow_mut() = buffer_rows(term.backend().buffer());

        // Same chrome/handlers as the stream card — bounds and metrics feed
        // `term_cell`, `term_drag`/`term_hover` the DnD paths.
        let focus = self
            .term_focus
            .entry(id.to_owned())
            .or_insert_with(|| cx.focus_handle())
            .clone();
        let tcell = self
            .term_bounds
            .entry(id.to_owned())
            .or_insert_with(|| std::rc::Rc::new(std::cell::RefCell::new(None)))
            .clone();
        let mcell = self
            .term_metrics
            .entry(id.to_owned())
            .or_insert_with(|| std::rc::Rc::new(std::cell::RefCell::new(None)))
            .clone();
        let family = mono.clone();
        let weak = cx.entity().downgrade();
        let (weak_k, weak_d, weak_u, weak_s, weak_mv, weak_dm, weak_dp) = (
            weak.clone(),
            weak.clone(),
            weak.clone(),
            weak.clone(),
            weak.clone(),
            weak.clone(),
            weak.clone(),
        );
        let focus_d = focus.clone();
        let (id_k, id_d, id_u, id_s, id_mv, id_dm, id_dp) = (
            id.to_owned(),
            id.to_owned(),
            id.to_owned(),
            id.to_owned(),
            id.to_owned(),
            id.to_owned(),
            id.to_owned(),
        );
        let (fam_d, fam_u, fam_s, fam_mv, fam_dm) = (
            family.clone(),
            family.clone(),
            family.clone(),
            family.clone(),
            family.clone(),
        );
        Some(
            div()
                .id(format!("preview-tui-{id}"))
                .size_full()
                .track_focus(&focus)
                // Local twin of the stream card's input: keys and pointer
                // cells go straight to the pane's view. `pane_key`/
                // `pane_mouse` hooks still fire so Studio's status line
                // reports input identically on both paths.
                .on_key_down(move |ev: &KeyDownEvent, _window, cx| {
                    let Some(key) = pane_key_name(&ev.keystroke) else {
                        return;
                    };
                    try_update(&weak_k, cx, "event", |this, cx| {
                        if let Some(tui) = this.tui_panes.get_mut(&id_k) {
                            tui.on_key(&key);
                        }
                        this.fire_call("pane_key", vec![id_k.clone(), key], cx);
                        drain_tui_outbox(this, cx);
                    })
                    .ok();
                })
                .on_mouse_down(MouseButton::Left, move |ev, window, cx| {
                    focus_d.focus(window, cx);
                    let (id, fam) = (id_d.clone(), fam_d.clone());
                    try_update(&weak_d, cx, "event", |this, cx| {
                        this.term_drag.insert(id.clone(), false);
                        let Some((col, row)) = term_cell(this, &id, ev.position, window, &fam)
                        else {
                            return;
                        };
                        // Stash the drag source and select the zone — the
                        // in-process twin of the host's `zone` event plus
                        // the compat canvas's click-select.
                        let mut zone = None;
                        if let Some(tui) = this.tui_panes.get_mut(&id) {
                            zone = tui.zone_path_at(col, row);
                            tui.on_mouse("down", col, row);
                        }
                        this.update_state(
                            &pane_state_path(&id, "zone"),
                            Json::String(zone.clone().unwrap_or_default()),
                            cx,
                        );
                        if editable && let Some(path) = zone {
                            this.update_state(
                                &parse_path("document.selected").unwrap(),
                                Json::String(path),
                                cx,
                            );
                            this.fire_call("select_node", vec![], cx);
                        }
                        drain_tui_outbox(this, cx);
                    })
                    .ok();
                })
                .on_mouse_up(MouseButton::Left, move |ev, window, cx| {
                    let (id, fam) = (id_u.clone(), fam_u.clone());
                    try_update(&weak_u, cx, "event", |this, cx| {
                        let Some((col, row)) = term_cell(this, &id, ev.position, window, &fam)
                        else {
                            return;
                        };
                        if editable && this.term_drag.get(&id).copied().unwrap_or(false) {
                            // Drag inside the card — a node move. The source
                            // is the `zone` stashed on `down`.
                            this.term_drag.insert(id.clone(), false);
                            let mut target = None;
                            if let Some(tui) = this.tui_panes.get_mut(&id) {
                                target = tui.drop_at(col, row);
                            }
                            let src = get_path(this.state(), &pane_state_path(&id, "zone"))
                                .and_then(|j| j.as_str())
                                .unwrap_or_default()
                                .to_owned();
                            this.fire_call(
                                "pane_drop_at",
                                vec![
                                    id.clone(),
                                    "node".to_string(),
                                    src,
                                    target.unwrap_or_default(),
                                ],
                                cx,
                            );
                        } else if let Some(tui) = this.tui_panes.get_mut(&id) {
                            tui.on_mouse("up", col, row);
                        }
                        drain_tui_outbox(this, cx);
                    })
                    .ok();
                })
                .on_mouse_move(move |ev: &MouseMoveEvent, window, cx| {
                    if !editable || !ev.dragging() {
                        return;
                    }
                    let (id, fam) = (id_mv.clone(), fam_mv.clone());
                    try_update(&weak_mv, cx, "event", |this, cx| {
                        this.term_drag.insert(id.clone(), true);
                        let Some((col, row)) = term_cell(this, &id, ev.position, window, &fam)
                        else {
                            return;
                        };
                        let mut target = None;
                        if let Some(tui) = this.tui_panes.get_mut(&id) {
                            target = tui.drop_hover(col, row);
                        }
                        this.update_state(
                            &pane_state_path(&id, "drop_target"),
                            Json::String(target.unwrap_or_default()),
                            cx,
                        );
                        drain_tui_outbox(this, cx);
                    })
                    .ok();
                })
                // Palette/doc drags over the card — the pane resolves cells
                // to doc paths itself. `pane_drop_at` commits without the
                // host round-trip `pane_drop` needs (#156).
                .on_drag_move::<ForgeDrag>(move |ev, window, cx| {
                    if !editable {
                        return;
                    }
                    let (id, fam) = (id_dm.clone(), fam_dm.clone());
                    try_update(&weak_dm, cx, "event", |this, cx| {
                        let Some((col, row)) =
                            term_cell(this, &id, ev.event.position, window, &fam)
                        else {
                            return;
                        };
                        let mut target = None;
                        if let Some(tui) = this.tui_panes.get_mut(&id) {
                            target = tui.drop_hover(col, row);
                        }
                        this.update_state(
                            &pane_state_path(&id, "drop_target"),
                            Json::String(target.unwrap_or_default()),
                            cx,
                        );
                    })
                    .ok();
                })
                .on_drop::<ForgeDrag>(move |drag, _window, cx| {
                    if !editable {
                        return;
                    }
                    let id = id_dp.clone();
                    try_update(&weak_dp, cx, "drop", |this, cx| {
                        let (kind, payload) = match drag {
                            ForgeDrag::Component(t) => ("component", t.clone()),
                            ForgeDrag::Node(p) => ("node", p.clone()),
                        };
                        let mut target = None;
                        if let Some(tui) = this.tui_panes.get_mut(&id) {
                            let (col, row) = this.term_hover.get(&id).copied().unwrap_or_default();
                            target = tui.drop_at(col, row);
                        }
                        this.update_state(&pane_state_path(&id, "drop_target"), Json::Null, cx);
                        this.fire_call(
                            "pane_drop_at",
                            vec![
                                id.clone(),
                                kind.to_string(),
                                payload,
                                target.unwrap_or_default(),
                            ],
                            cx,
                        );
                        drain_tui_outbox(this, cx);
                    })
                    .ok();
                })
                .on_scroll(move |ev: &ScrollEvent, window, cx| {
                    let kind = if f32::from(ev.delta.pixel_delta(px(20.)).y) < 0. {
                        "scroll-down"
                    } else {
                        "scroll-up"
                    };
                    let (id, fam) = (id_s.clone(), fam_s.clone());
                    try_update(&weak_s, cx, "event", |this, cx| {
                        if let Some((col, row)) = term_cell(this, &id, ev.position, window, &fam)
                            && let Some(tui) = this.tui_panes.get_mut(&id)
                        {
                            tui.on_mouse(kind, col, row);
                        }
                        drain_tui_outbox(this, cx);
                    })
                    .ok();
                })
                .child(
                    div()
                        .size_full()
                        .p_2()
                        .overflow_hidden()
                        .bg(bg)
                        .child(BoundsTracker::new(
                            forge_term::GridView::new(
                                grid,
                                forge_term::TermStyle {
                                    font_family: mono,
                                    font_size: px(12.),
                                    default_fg: fg,
                                    default_bg: bg,
                                    cursor: fg,
                                },
                            )
                            .metrics_sink(mcell)
                            .into_any_element(),
                            tcell,
                        )),
                )
                .into_any_element(),
        )
    }

    /// The `forge.Preview` host for `id`, creating the nested view on first use.
    /// The child shares the parent's registered libraries so `alias.Name`
    /// elements render inside previews too.
    pub fn preview(&mut self, id: &str, cx: &mut Context<Self>) -> Entity<ForgeView> {
        let catalogs = self.catalogs.clone();
        let libraries = self.libraries.clone();
        self.previews
            .entry(id.to_owned())
            .or_insert_with(|| {
                cx.new(|_| {
                    let mut view = ForgeView::new(catalogs);
                    view.libraries = libraries;
                    view
                })
            })
            .clone()
    }

    /// Sets the document a `forge.Preview` renders.
    pub fn set_preview_document(&mut self, id: &str, doc: Document, cx: &mut Context<Self>) {
        self.preview(id, cx)
            .update(cx, |view, cx| view.set_document(doc, cx));
    }

    pub fn set_sinks(&mut self, on_hook: HookSink, on_state: StateSink) {
        self.on_hook = Some(on_hook);
        self.on_state = Some(on_state);
    }

    /// `forge.Vars` — a live state inspector for the *edited document's*
    /// state: `of` names the `forge.Preview` id whose nested ForgeView
    /// supplies the vars. Scalars edit via subscribed `Input`s, objects/
    /// arrays via a `Textarea` + `set`, and an "add" row creates new vars.
    /// All writes go to the *preview* view's state, so `{$path}` bindings in
    /// the preview tab react live.
    fn vars(
        &mut self,
        node: &Node,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let of = self
            .prop_text(node, "of")
            .unwrap_or_else(|| "canvas".to_owned());
        let preview = self.preview(&of, cx);
        let state = preview.read(cx).state().clone();
        // `state:` declarations on the doc — `path -> type` — pick the editor
        // widget per type. Undeclared vars fall back to the Json type.
        let doc_state: Vec<(String, String)> = preview
            .read(cx)
            .document()
            .map(|d| d.state.clone())
            .unwrap_or_default();
        let var_type = |path: &str| {
            doc_state
                .iter()
                .find(|(p, _)| p == path)
                .map(|(_, t)| t.as_str())
        };

        fn flatten(prefix: &str, json: &Json, out: &mut Vec<(String, Json)>) {
            match json {
                Json::Object(map) => {
                    for (k, v) in map {
                        flatten(&format!("{prefix}{k}."), v, out);
                    }
                }
                _ => out.push((prefix.trim_end_matches('.').to_owned(), json.clone())),
            }
        }
        let mut rows: Vec<(String, Json)> = Vec::new();
        flatten("", &state, &mut rows);
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        // `count:` — the doc's "N variables" line reads this instead of
        // re-flattening state itself.
        if let Some(path) = self.two_way(node, "count") {
            let value = Json::from(rows.len());
            if get_path(self.state(), &path) != Some(&value) {
                self.update_state(&path, value, cx);
            }
        }

        let mut list = div().flex().flex_col().child(
            div()
                .flex()
                .flex_row()
                .gap_3()
                .pb_1()
                .child(
                    div()
                        .w(px(200.))
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("variable"),
                )
                .child(
                    div()
                        .flex_grow(1.0)
                        .min_w_0()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("value"),
                ),
        );
        for (path, value) in rows {
            let state_key = format!("vars-{key}-{path}");
            let display = match &value {
                Json::String(s) => s.clone(),
                _ => serde_json::to_string_pretty(&value).unwrap_or_default(),
            };
            // `state:` types dispatch the editor: bool → Checkbox, number →
            // Slider, enum → Select, color → Input (hex), slider → Slider —
            // undeclared falls back to the Json type below.
            let typed_edit: Option<AnyElement> = match var_type(&path) {
                Some("bool") => {
                    // Click writes the new value into the preview's state —
                    // `checked` alone made a display-only box that ignored
                    // clicks entirely.
                    let cb_path = parse_path(&path).unwrap();
                    let pv = preview.clone();
                    Some(
                        Checkbox::new(SharedString::from(format!("vars-bool-{key}-{path}")))
                            .checked(value.as_bool().unwrap_or(false))
                            .on_click(move |v: &bool, _w, cx| {
                                pv.update(cx, |view, cx| {
                                    view.update_state(&cb_path, Json::Bool(*v), cx)
                                });
                            })
                            .into_any_element(),
                    )
                }
                // Numbers get a NumberInput — a slider suggests a bounded
                // range a bare `number` decl doesn't carry. `slider` keeps
                // the slider for docs that ask for one.
                Some("number") => {
                    let s = match self.inputs.get(&state_key) {
                        Some(s) => s.clone(),
                        None => {
                            let s = cx.new(|cx| {
                                InputState::new(window, cx).default_value(display.clone())
                            });
                            let path = parse_path(&path).unwrap();
                            let preview = preview.clone();
                            let sub = cx.subscribe(&s, move |_this, s, event, cx| {
                                if matches!(event, InputEvent::Change) {
                                    let v = s.read(cx).value().to_string();
                                    // Only write parseable numbers — a
                                    // half-typed `-`/`8.` shouldn't clobber
                                    // the state the row is editing.
                                    if let Ok(n) = v.parse::<f64>() {
                                        preview.update(cx, |v, cx| {
                                            v.update_state(&path, Json::from(n), cx)
                                        });
                                    }
                                }
                            });
                            self.subscriptions.push(sub);
                            self.inputs.insert(state_key.clone(), s.clone());
                            s
                        }
                    };
                    Some(NumberInput::new(&s).w_full().into_any_element())
                }
                Some("slider") => {
                    let s = match self.sliders.get(&state_key) {
                        Some(s) => s.clone(),
                        None => {
                            let s = cx.new(|_cx| {
                                SliderState::new()
                                    .default_value(value.as_f64().unwrap_or_default() as f32)
                            });
                            let path = parse_path(&path).unwrap();
                            let preview = preview.clone();
                            let sub = cx.subscribe(&s, move |_this, _s, event, cx| {
                                let SliderEvent::Change(value) = event else {
                                    return;
                                };
                                if let SliderValue::Single(v) = value {
                                    preview.update(cx, |view, cx| {
                                        view.update_state(&path, Json::from(*v), cx)
                                    });
                                }
                            });
                            self.subscriptions.push(sub);
                            self.sliders.insert(state_key.clone(), s.clone());
                            s
                        }
                    };
                    Some(Slider::new(&s).w_full().into_any_element())
                }
                Some(t) if t.starts_with("enum") || t == "select" => {
                    // `enum(a | b | c)` — variants parsed from the type string.
                    let options: Vec<SharedString> = t
                        .strip_prefix("enum")
                        .and_then(|s| s.strip_prefix('('))
                        .and_then(|s| s.strip_suffix(')'))
                        .map(|s| {
                            s.split('|')
                                .map(|v| SharedString::from(v.trim().to_owned()))
                                .collect()
                        })
                        .unwrap_or_else(|| {
                            value
                                .as_array()
                                .map(|a| {
                                    a.iter()
                                        .filter_map(|v| {
                                            v.as_str().map(|s| SharedString::from(s.to_owned()))
                                        })
                                        .collect()
                                })
                                .unwrap_or_default()
                        });
                    let selected = value
                        .as_str()
                        .and_then(|s| options.iter().position(|o| o == s))
                        .map(IndexPath::new);
                    let s = match self.selects.get(&state_key) {
                        Some(s) => s.clone(),
                        None => {
                            let s = cx.new(|cx| SelectState::new(options, selected, window, cx));
                            let path = parse_path(&path).unwrap();
                            let preview = preview.clone();
                            let sub = cx.subscribe(
                                &s,
                                move |_this, _s, event: &SelectEvent<Options>, cx| {
                                    if let SelectEvent::Confirm(Some(v)) = event {
                                        preview.update(cx, |view, cx| {
                                            view.update_state(
                                                &path,
                                                Json::String(v.to_string()),
                                                cx,
                                            )
                                        });
                                    }
                                },
                            );
                            self.subscriptions.push(sub);
                            self.selects.insert(state_key.clone(), s.clone());
                            s
                        }
                    };
                    Some(Select::new(&s).w_full().into_any_element())
                }
                Some("color") => {
                    let s = self.inputs.get(&state_key).cloned().unwrap_or_else(|| {
                        let s =
                            cx.new(|cx| InputState::new(window, cx).default_value(display.clone()));
                        self.inputs.insert(state_key.clone(), s.clone());
                        s
                    });
                    Some(Input::new(&s).w_full().into_any_element())
                }
                _ => None,
            };
            let row_edit = if let Some(t) = typed_edit {
                div().flex().flex_row().flex_grow(1.0).child(t)
            } else {
                match &value {
                    Json::String(_) | Json::Number(_) | Json::Bool(_) => {
                        let state = match self.inputs.get(&state_key) {
                            Some(s) => s.clone(),
                            None => {
                                let s = cx.new(|cx| {
                                    InputState::new(window, cx).default_value(display.clone())
                                });
                                let path = parse_path(&path).unwrap();
                                let preview = preview.clone();
                                let sub = cx.subscribe(&s, move |_this, s, event, cx| {
                                    if matches!(event, InputEvent::Change) {
                                        let v = s.read(cx).value().to_string();
                                        let parsed =
                                            serde_json::from_str(&v).unwrap_or(Json::String(v));
                                        preview
                                            .update(cx, |v, cx| v.update_state(&path, parsed, cx));
                                    }
                                });
                                self.subscriptions.push(sub);
                                self.inputs.insert(state_key.clone(), s.clone());
                                s
                            }
                        };
                        div()
                            .flex()
                            .flex_row()
                            .flex_grow(1.0)
                            .gap_1()
                            .child(Input::new(&state).with_size(size("small")).w_full())
                    }
                    _ => {
                        let edit_path_str = format!("vars_edit.{}", path.replace('.', "_"));
                        let state = match self.textareas.get(&state_key) {
                            Some(s) => s.clone(),
                            None => {
                                let s = cx.new(|cx| {
                                    TextareaState::new(window, cx)
                                        .default_value(display.clone())
                                        .rows(3)
                                });
                                let edit_path = parse_path(&edit_path_str).unwrap();
                                let preview = preview.clone();
                                let sub = cx.subscribe(&s, move |_this, s, event, cx| {
                                    if matches!(event, InputEvent::Change) {
                                        let v = s.read(cx).value().to_string();
                                        preview.update(cx, |view, cx| {
                                            view.update_state(&edit_path, Json::String(v), cx)
                                        });
                                    }
                                });
                                self.subscriptions.push(sub);
                                self.textareas.insert(state_key.clone(), s.clone());
                                s
                            }
                        };
                        let preview = preview.clone();
                        let path_for_set = parse_path(&path).unwrap();
                        let edit_path = parse_path(&edit_path_str).unwrap();
                        div()
                            .flex()
                            .flex_col()
                            .flex_grow(1.0)
                            .gap_1()
                            .child(Textarea::new(&state).w_full())
                            .child(
                                Button::new(SharedString::from(format!("vars-set-{key}-{path}")))
                                    .label("set")
                                    .with_variant(ButtonVariant::Primary)
                                    .with_size(size("small"))
                                    .on_click(move |_, _, cx| {
                                        let preview = preview.clone();
                                        let path_for_set = path_for_set.clone();
                                        preview.update(cx, |view, cx| {
                                            let text = get_path(view.state(), &edit_path)
                                                .and_then(|j| j.as_str())
                                                .unwrap_or_default()
                                                .to_owned();
                                            if let Ok(parsed) = serde_json::from_str(&text) {
                                                view.update_state(&path_for_set, parsed, cx);
                                            }
                                        });
                                    }),
                            )
                    }
                }
            };
            // Table row — a fixed-width label column keeps every editor's
            // left edge on the same gridline. Long paths ellipsize with the
            // full name on hover. `pt` optically aligns the label with the
            // control's text. `items_start` keeps tall rows (textareas)
            // anchored to the first line.
            list = list.child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_start()
                            .gap_3()
                            .child(
                                div()
                                    .id(format!("vars-tip-{key}-{path}"))
                                    .w(px(200.))
                                    .flex_shrink_0()
                                    .pt(px(7.))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .whitespace_nowrap()
                                            .overflow_hidden()
                                            .text_ellipsis()
                                            .child(format!("${path}")),
                                    )
                                    .tooltip({
                                        let tip = format!("${path}");
                                        move |_, cx| cx.new(|_| Tooltip::new(tip.clone())).into()
                                    }),
                            )
                            .child(div().flex_grow(1.0).min_w_0().child(row_edit)),
                    )
                    .child(Separator::horizontal().mt_1()),
            );
        }

        // Add-new-var: Inputs write `vars_add.{path,value}` in the preview's
        // state.  "+" commits the new path.
        let add_path_key = format!("vars-add-path-{key}");
        let add_val_key = format!("vars-add-val-{key}");
        let preview_clone = preview.clone();
        let add_path_state = self.inputs.get(&add_path_key).cloned().unwrap_or_else(|| {
            let s = cx.new(|cx| InputState::new(window, cx).placeholder("path (e.g. studio.new)"));
            let preview = preview_clone.clone();
            let sub = cx.subscribe(&s, move |_this, s, event, cx| {
                if matches!(event, InputEvent::Change) {
                    let v = s.read(cx).value().to_string();
                    preview.update(cx, |view, cx| {
                        view.update_state(
                            &parse_path("vars_add.path").unwrap(),
                            Json::String(v),
                            cx,
                        )
                    });
                }
            });
            self.subscriptions.push(sub);
            self.inputs.insert(add_path_key.clone(), s.clone());
            s
        });
        let preview_clone = preview.clone();
        let add_val_state = self.inputs.get(&add_val_key).cloned().unwrap_or_else(|| {
            let s = cx.new(|cx| InputState::new(window, cx).placeholder("value"));
            let preview = preview_clone.clone();
            let sub = cx.subscribe(&s, move |_this, s, event, cx| {
                if matches!(event, InputEvent::Change) {
                    let v = s.read(cx).value().to_string();
                    preview.update(cx, |view, cx| {
                        view.update_state(
                            &parse_path("vars_add.value").unwrap(),
                            Json::String(v),
                            cx,
                        )
                    });
                }
            });
            self.subscriptions.push(sub);
            self.inputs.insert(add_val_key.clone(), s.clone());
            s
        });
        let add_btn = Button::new(SharedString::from(format!("vars-add-{key}")))
            .label("+")
            .with_variant(ButtonVariant::Primary)
            .with_size(size("small"))
            .on_click(move |_, _, cx| {
                preview.update(cx, |view, cx| {
                    let path = get_path(view.state(), &parse_path("vars_add.path").unwrap())
                        .and_then(|j| j.as_str())
                        .unwrap_or_default()
                        .to_owned();
                    let val = get_path(view.state(), &parse_path("vars_add.value").unwrap())
                        .and_then(|j| j.as_str())
                        .unwrap_or_default()
                        .to_owned();
                    if !path.is_empty() {
                        let parsed = serde_json::from_str(&val).unwrap_or(Json::String(val));
                        view.update_state(&parse_path(&path).unwrap(), parsed, cx);
                    }
                });
            });
        list = list.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .w(px(200.))
                        .flex_shrink_0()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("new var"),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .flex_grow(1.0)
                        .min_w_0()
                        .gap_1()
                        .child(Input::new(&add_path_state).with_size(size("small")))
                        .child(Input::new(&add_val_state).with_size(size("small")))
                        .child(add_btn),
                ),
        );

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(list)
            .into_any_element()
    }

    /// `forge.Palette` — a catalog-driven element palette: a dropdown selects
    /// the library (base or any registered `ForgeLibrary`), and a grid of
    /// buttons inserts its elements through the `insert_component` hook.
    fn palette(
        &mut self,
        _node: &Node,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        // (label, library id, tag prefix) — skip the `forge` intrinsics catalog
        // and any catalog the edited document's base doesn't support (#98).
        let doc_base = get_path(self.state(), &parse_path("studio.doc_base").unwrap())
            .and_then(|j| j.as_str())
            .unwrap_or(crate::base::BASE_ID)
            .to_owned();
        let mut libs: Vec<(String, String, String)> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for cat in &self.catalogs {
            // A library's catalog can land in `self.catalogs` twice — the
            // bundled `check::catalogs()` set plus the `register_library`
            // copy — so dedupe by library id or the dropdown lists it twice.
            if cat.library == "forge" || !cat.supports_base(&doc_base) {
                continue;
            }
            if !seen.insert(cat.library.clone()) {
                continue;
            }
            let alias = self
                .libraries
                .iter()
                .find(|l| l.catalog().is_some_and(|c| c.library == cat.library))
                .map(|l| l.alias().to_string())
                .unwrap_or_default();
            let label = cat.name.clone().unwrap_or_else(|| cat.library.clone());
            libs.push((label, cat.library.clone(), alias));
        }
        let selected = get_path(self.state(), &parse_path("studio.palette_lib").unwrap())
            .and_then(|j| j.as_str())
            .map(str::to_owned)
            .unwrap_or_else(|| {
                libs.first()
                    .map(|(_, id, _)| id.clone())
                    .unwrap_or_default()
            });
        let current_label = libs
            .iter()
            .find(|(_, id, _)| id == &selected)
            .map(|(l, _, _)| l.clone())
            .unwrap_or_else(|| selected.clone());

        let weak_menu = cx.entity().downgrade();
        let weak = cx.entity().downgrade();
        let selector = Button::new(SharedString::from(format!("{key}-lib")))
            .label(SharedString::from(current_label))
            .with_variant(ButtonVariant::Secondary)
            .with_size(size("small"))
            .dropdown_menu({
                let libs = libs.clone();
                move |menu, _, _| {
                    let mut menu = menu;
                    for (label, id, _) in &libs {
                        let weak = weak_menu.clone();
                        let id = id.clone();
                        menu = menu.item(PopupMenuItem::new(label.clone()).on_click(
                            move |_, _, cx| {
                                try_update(&weak, cx, "palette", |this, cx| {
                                    this.update_state(
                                        &parse_path("studio.palette_lib").unwrap(),
                                        Json::String(id.clone()),
                                        cx,
                                    )
                                })
                                .ok();
                            },
                        ));
                    }
                    menu
                }
            });

        // `studio.palette_all` widens the listing to every catalog. Each
        // element still carries its own library's tag prefix, and groups are
        // prefixed with the library name so `gpui-kit` and `ely` items don't
        // collide. `studio.palette_query` filters element names.
        let all = get_path(self.state(), &parse_path("studio.palette_all").unwrap())
            .and_then(|j| j.as_bool())
            .unwrap_or(false);
        let query = get_path(self.state(), &parse_path("studio.palette_query").unwrap())
            .and_then(|j| j.as_str())
            .unwrap_or_default()
            .to_lowercase();
        // Accordion groups of icon buttons — `icon:`/`group:` are palette
        // metadata declared per element in the catalog manifest.
        type PaletteItem = (String, String, String); // name, insert tag, icon
        let mut groups: Vec<(String, Vec<PaletteItem>)> = Vec::new();
        for (label, lib, prefix) in &libs {
            if !all && *lib != selected {
                continue;
            }
            let Some(cat) = self.catalogs.iter().find(|c| c.library == *lib) else {
                continue;
            };
            for def in &cat.elements {
                let name = def.name.clone();
                if !query.is_empty() && !name.to_lowercase().contains(&query) {
                    continue;
                }
                let base_group = def.group.clone().unwrap_or_else(|| "Elements".into());
                let group = if all {
                    format!("{label} · {base_group}")
                } else {
                    base_group
                };
                let tag = if prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{prefix}.{name}")
                };
                let icon_name = def.icon.clone().unwrap_or_else(|| "component".into());
                match groups.iter_mut().find(|(g, _)| g == &group) {
                    Some((_, items)) => items.push((name, tag, icon_name)),
                    None => groups.push((group, vec![(name, tag, icon_name)])),
                }
            }
        }
        // Open state lives in studio.palette_open (indices) — groups start open
        // (untracked == open) and header clicks toggle via the accordion callback.
        let open_path = parse_path("studio.palette_open").unwrap();
        // `None` (never toggled) = every group open. An explicit `[]` =
        // collapse-all — don't conflate them or the last collapse snaps
        // everything back open on the next render.
        let open_set: Option<Vec<usize>> = get_path(self.state(), &open_path)
            .and_then(|j| j.as_array().cloned())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_u64().map(|u| u as usize))
                    .collect()
            });
        let weak_toggle = cx.entity().downgrade();
        let open_path_c = open_path.clone();
        let mut grid = Accordion::new(SharedString::from(format!("{key}-groups")))
            .multiple(true)
            .bordered(false)
            .on_toggle_click(move |indices: &[usize], _, cx| {
                let open_path = open_path_c.clone();
                try_update(&weak_toggle, cx, "palette", |this, cx| {
                    this.update_state(&open_path, Json::from(indices.to_vec()), cx)
                })
                .ok();
            });
        for (i, (group, items)) in groups.iter().enumerate() {
            let mut row = div().flex().flex_row().flex_wrap().gap_1();
            for (name, tag, icon_name) in items.iter().cloned() {
                // Synthesised node carrying `on click: insert_component('<tag>')` —
                // clicks run through the same fire path as real elements.
                let synth = Node {
                    element: ElementRef {
                        name: "Button".into(),
                        namespace: None,
                    },
                    events: vec![EventBinding {
                        event: "click".into(),
                        handler: HandlerRef {
                            name: "insert_component".into(),
                            args: vec![Value::String(tag.clone())],
                        },
                    }],
                    ..Default::default()
                };
                let weak = weak.clone();
                let ghost: SharedString = tag.clone().into();
                row = row.child(
                    div()
                        .id(format!("pal-{key}-{name}"))
                        .child(
                            Button::new(SharedString::from(format!("{key}-{name}")))
                                .icon(icon(&icon_name))
                                .with_variant(ButtonVariant::Ghost)
                                .with_size(size("small"))
                                .tooltip(name)
                                .on_click(move |_, _, cx| {
                                    try_update(&weak, cx, "event", |this, cx| {
                                        this.fire(&synth, "click", cx)
                                    })
                                    .ok();
                                }),
                        )
                        .on_drag(ForgeDrag::Component(tag.clone()), move |_, _, _, cx| {
                            cx.new(|_| DragGhost(ghost.clone()))
                        }),
                );
            }
            // Untracked (no open_set) means "everything open" — the default.
            let open = open_set.as_ref().is_none_or(|set| set.contains(&i));
            let item = gpui_kit::component::accordion::AccordionItem::new()
                .title(SharedString::from(group.clone()))
                .open(open)
                .child(row);
            grid = grid.item(move |_| item);
        }
        // Search input + "all sets" scope toggle — the query filters element
        // names in the selected library by default. Checking the box widens
        // the listing to every catalog (groups get a `lib ·` prefix).
        let search_state = match self.inputs.get(&format!("{key}-pal-search")) {
            Some(state) => state.clone(),
            None => {
                let state = cx.new(|cx| InputState::new(window, cx).placeholder("search widgets…"));
                let subscription = cx.subscribe(&state, move |this, state, event, cx| {
                    if let InputEvent::Change = event {
                        let value = state.read(cx).value().to_string();
                        this.update_state(
                            &parse_path("studio.palette_query").unwrap(),
                            Json::String(value),
                            cx,
                        );
                    }
                });
                self.subscriptions.push(subscription);
                self.inputs
                    .insert(format!("{key}-pal-search"), state.clone());
                state
            }
        };
        let weak_all = cx.entity().downgrade();
        let all_cb = Checkbox::new(SharedString::from(format!("{key}-all")))
            .label("all sets")
            .checked(all)
            .on_click(move |v: &bool, _w: &mut Window, cx: &mut App| {
                let v = *v;
                try_update(&weak_all, cx, "palette all", move |this, cx| {
                    this.update_state(
                        &parse_path("studio.palette_all").unwrap(),
                        Json::Bool(v),
                        cx,
                    );
                })
                .ok();
            });
        let search_row = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_1()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(Input::new(&search_state).cleanable(true)),
            )
            .child(all_cb);

        // The palette fills its docked slot (`size_full`) — a content-height
        // column let collapse-all shrink the whole window chain (status bar
        // floated). The library selector pins at top. The accordion groups
        // scroll inside the leftover space.
        let handle = self
            .scroll_handles
            .entry(format!("{key}-palette-scroll"))
            .or_default()
            .clone();
        div()
            .flex()
            .flex_col()
            .gap_2()
            .size_full()
            .min_h_0()
            .child(selector)
            .child(search_row)
            .child(
                div()
                    .id(SharedString::from(format!("{key}-groups")))
                    .track_scroll(&handle)
                    .overflow_y_scrollbar()
                    .id(SharedString::from(format!("{key}-groups-scrollbar")))
                    .flex_1()
                    .min_h_0()
                    .child(grid),
            )
            .into_any_element()
    }

    /// `forge.Icons` — a searchable grid of the app's icon catalog, read
    /// from `AssetSource::list("icons/")`. `icons/<name>.svg` is `lucide`
    /// (the bundled set). `icons/<set>/<name>.svg` marks a named iconset
    /// (`icons/heroicons/…`), which the host's asset source layers in.
    /// `set:` picks/binds the browsed set, `sets:` publishes the discovered
    /// sets, `search:` adds a filter box (`query:` exposes its text), and a
    /// click writes `selected` + fires `select` with the icon name.
    /// `category:`/`theme:` bind IconifyJSON metadata filters — categories
    /// (an icon displays under every category it belongs to) and theme
    /// prefixes/suffixes (`baseline-`, `-filled`). A bound prop gets a
    /// picker, a static one pins the filter. Names sort naturally so the
    /// grid order never shifts under state changes.
    fn icons(
        &mut self,
        node: &Node,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let paths = cx.asset_source().list("icons/").unwrap_or_default();
        let mut by_set: HashMap<String, Vec<String>> = HashMap::new();
        for path in &paths {
            let Some(stem) = path
                .strip_prefix("icons/")
                .and_then(|rest| rest.strip_suffix(".svg"))
            else {
                continue;
            };
            match stem.split_once('/') {
                Some((set, name)) => by_set
                    .entry(set.to_owned())
                    .or_default()
                    .push(name.to_owned()),
                None => by_set
                    .entry("lucide".into())
                    .or_default()
                    .push(stem.to_owned()),
            }
        }
        // Iconify-catalog sets list even with no local files — picking one
        // browses it over `https` and lazily gets its name list.
        for set in crate::iconify::sets() {
            by_set.entry(set).or_default();
        }
        for names in by_set.values_mut() {
            names.sort();
        }
        let mut sets: Vec<String> = by_set.keys().cloned().collect();
        sets.sort();
        // Catalog load — fire-once per session. `refresh_windows` re-renders
        // on land. Always asked for so a stale `sets.json` refreshes too.
        crate::iconify::ensure_sets(cx);
        // `set:` — bound or static. Absent means the view's own iconset. An
        // empty value (bound or static) falls back too, then gets seeded so
        // the picker and grid agree on first render.
        let set_path = self.two_way(node, "set");
        let set = set_path
            .as_ref()
            .and_then(|p| self.lookup(p))
            .and_then(|j| j.as_str())
            .map(str::to_owned)
            .or_else(|| self.prop_text(node, "set"))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| self.iconset().to_owned());
        if let Some(p) = &set_path {
            let empty = self
                .lookup(p)
                .and_then(|j| j.as_str())
                .unwrap_or_default()
                .is_empty();
            if empty {
                self.update_state(p, Json::String(set.clone()), cx);
            }
        }
        if let Some(p) = self.two_way(node, "sets") {
            let value = Json::Array(sets.iter().map(|s| Json::String(s.clone())).collect());
            if get_path(self.state(), &p) != Some(&value) {
                self.update_state(&p, value, cx);
            }
        }
        // `query:` — bound state like `forge.Files`', else a per-element slot.
        let query_path = self.two_way(node, "query");
        let query = match &query_path {
            Some(p) => self
                .lookup(p)
                .and_then(|j| j.as_str())
                .unwrap_or_default()
                .to_owned(),
            None => self.icons_queries.get(key).cloned().unwrap_or_default(),
        };
        // Iconify-catalog sets route through `ensure_set` — uncached sets get
        // their one IconifyJSON load, cached sets a `lastModified` check,
        // all fire-once. Only mergeable sets load: heroicons' iconify names
        // and suffix themes join the bundled listing. Lucide's flat
        // `icons/` namespace can't merge and its categories are vendored,
        // so its IconifyJSON buys nothing.
        if iconify_mergeable(&set) && crate::iconify::known_set(&set) {
            crate::iconify::ensure_set(&set, cx);
        }
        let filter = query.to_lowercase();
        // Names = locally-materialized files ∪ the iconify name list — a
        // partially-vendored set still browses its whole catalog.
        let materialized: std::collections::HashSet<String> = by_set
            .get(&set)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();
        let mut names: Vec<String> = materialized.iter().cloned().collect();
        if iconify_mergeable(&set) {
            for name in crate::iconify::names(&set) {
                if !materialized.contains(&name) {
                    names.push(name);
                }
            }
        }
        names.retain(|n| filter.is_empty() || n.to_lowercase().contains(&filter));
        // Natural sort keeps the grid stable across re-renders —
        // `materialized` came from a `HashSet`, so any state change used to
        // reshuffle every cell.
        names.sort_by(|a, b| crate::iconify::nat_cmp(a, b));

        // `category:` — IconifyJSON `categories` buckets (an icon displays under
        // every category it belongs to). `theme:` — `prefixes`/`suffixes`
        // variations. Either binds (a picker renders) or pins statically.
        let mut cats = crate::iconify::categories(&set);
        let themes = crate::iconify::themes(&set);
        let cat_path = self.two_way(node, "category");
        let theme_path = self.two_way(node, "theme");
        let resolve = |path: &Option<Path>, name: &str| {
            path.as_ref()
                .and_then(|p| self.lookup(p))
                .and_then(|j| j.as_str())
                .map(str::to_owned)
                .or_else(|| self.prop_text(node, name))
                .unwrap_or_default()
        };
        let cat = resolve(&cat_path, "category");
        // `""`/`"[all]"` = display every group. A bound value pointing at a
        // category the set doesn't have resets to the sentinel.
        if !cat.is_empty()
            && cat != "[all]"
            && !cats.iter().any(|(k, _)| *k == cat)
            && let Some(p) = &cat_path
        {
            self.update_state(p, Json::String("[all]".into()), cx);
        }
        // `theme:` — multi-select. Bound state holds a `["s:key"/"p:key"]`
        // array. A bare string is back-compat (the old Select stored
        // titles). Unset defaults to the no-suffix `""` themes so variant
        // sets open on their base icons. All-checked or empty = no filter.
        let theme_raw: Option<Json> = theme_path
            .as_ref()
            .and_then(|p| self.lookup(p).cloned())
            .or_else(|| self.prop_text(node, "theme").map(Json::String));
        let theme_checked = theme_selection(&themes, theme_raw.as_ref());
        if let Some(p) = &theme_path {
            let canonical = Json::Array(
                theme_checked
                    .iter()
                    .map(|(k, t)| Json::String(theme_sel_encode(*k, t)))
                    .collect(),
            );
            if self.lookup(p) != Some(&canonical) {
                self.update_state(p, canonical, cx);
            }
        }
        if !theme_checked.is_empty() && theme_checked.len() < themes.len() {
            names.retain(|n| {
                theme_checked
                    .iter()
                    .any(|(k, t)| crate::iconify::theme_matches(&themes, *k, t, n))
            });
        }
        let weak = cx.entity().downgrade();
        let selected_path = self.two_way(node, "selected");
        let selected = selected_path
            .as_ref()
            .and_then(|p| self.lookup(p))
            .and_then(|j| j.as_str())
            .map(str::to_owned);
        let select_handler = node
            .events
            .iter()
            .find(|e| e.event == "select")
            .map(|e| e.handler.name.clone());

        // `bare:`. The doc lays out its own chrome around the grid from the
        // published outputs (`categories`/`themes`/`meta`/`status`/
        // `attribution`), so none of the synthesized header/search/footer
        // UI. Or its entities — gets built. The grid stays here either way:
        // measured-width virtualization + per-cell materialization aren't
        // doc-expressible. Same contract as `forge.Inspector`'s data mode.
        let bare = self.prop_bool(node, "bare");
        // Filter row — labeled pickers: `Iconset`, `Category`, `Suffix`.
        // Each is a synthesized `Select` bound to its prop's path (or an
        // internal slot). A pinned static prop hides its picker since it IS
        // the filter. `search:` renders on its own row below.
        let chrome = !bare;
        let label = |text: &str, cx: &mut Context<Self>| {
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(text.to_owned())
        };
        let mut header = div().flex().flex_row().flex_wrap().items_center().gap_2();
        let static_set = node
            .props
            .iter()
            .any(|(k, v)| k == "set" && !matches!(v, Value::Binding { .. }));
        if chrome && sets.len() > 1 && !static_set {
            let bind = set_path
                .clone()
                .unwrap_or_else(|| parse_path("studio.icons_set").expect("static path"));
            let select = picker(sets.clone(), bind);
            header = header.child(label("Iconset", cx)).child(
                div().w(px(160.)).child(self.render_node(
                    &select,
                    &format!("{key}-set"),
                    window,
                    cx,
                )),
            );
        }
        let search_row = if chrome && self.prop_bool(node, "search") {
            let search_key = format!("{key}-search");
            let state = match self.inputs.get(&search_key) {
                Some(state) => state.clone(),
                None => {
                    let state = cx.new(|cx| {
                        InputState::new(window, cx)
                            .placeholder("filter icons…")
                            .default_value(query.clone())
                    });
                    let path = query_path.clone();
                    let state_key = key.to_owned();
                    let subscription = cx.subscribe(&state, move |this, state, event, cx| {
                        if let InputEvent::Change = event {
                            let value = state.read(cx).value().to_string();
                            this.icons_queries.insert(state_key.clone(), value.clone());
                            if let Some(path) = &path {
                                this.write(path, Json::String(value), cx);
                            }
                            cx.notify();
                        }
                    });
                    self.subscriptions.push(subscription);
                    self.inputs.insert(search_key, state.clone());
                    state
                }
            };
            if query_path.is_some() && state.read(cx).value() != query {
                state.update(cx, |s, cx| s.set_value(query.clone(), window, cx));
            }
            Some(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(label("Search", cx))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(Input::new(&state).cleanable(true)),
                    ),
            )
        } else {
            None
        };
        // Category + suffix/theme pickers join the set picker on the filter
        // row — same bound-or-static rules. A bound slot holding no valid
        // option seeds the "[all]" sentinel.
        let static_cat = node
            .props
            .iter()
            .any(|(k, v)| k == "category" && !matches!(v, Value::Binding { .. }));
        let static_theme = node
            .props
            .iter()
            .any(|(k, v)| k == "theme" && !matches!(v, Value::Binding { .. }));
        if chrome && cats.len() > 1 && !static_cat {
            let mut options = vec!["[all]".to_owned()];
            options.extend(cats.iter().map(|(k, _)| k.clone()));
            let bind = cat_path
                .clone()
                .unwrap_or_else(|| parse_path("studio.icons_cat").expect("static path"));
            let cur = self
                .lookup(&bind)
                .and_then(|j| j.as_str())
                .unwrap_or_default();
            if !options.iter().any(|o| o == cur) {
                self.update_state(&bind, Json::String("[all]".into()), cx);
            }
            let select = picker(options, bind);
            header = header.child(label("Category", cx)).child(
                div().w(px(160.)).child(self.render_node(
                    &select,
                    &format!("{key}-cat"),
                    window,
                    cx,
                )),
            );
        }
        // Suffix/prefix themes are checkboxes (multi-select, OR semantics)
        // labeled by the dominant kind. Checked state lives in the bound
        // array. Each click toggles one entry in place.
        if chrome && themes.len() > 1 && !static_theme {
            use crate::iconify::ThemeKind;
            let bind = theme_path
                .clone()
                .unwrap_or_else(|| parse_path("studio.icons_theme").expect("static path"));
            let theme_label = if themes.iter().all(|(k, _, _)| *k == ThemeKind::Prefix) {
                "Prefix"
            } else if themes.iter().all(|(k, _, _)| *k == ThemeKind::Suffix) {
                "Suffix"
            } else {
                "Theme"
            };
            header = header.child(label(theme_label, cx));
            for (kind, tkey, title) in themes.iter() {
                let kind = *kind;
                let checked = theme_checked.iter().any(|(k, t)| *k == kind && t == tkey);
                let tkey = tkey.clone();
                let bind2 = bind.clone();
                let themes2 = themes.clone();
                let weak2 = weak.clone();
                let cb = Checkbox::new(SharedString::from(format!(
                    "{key}-th-{}",
                    theme_sel_encode(kind, &tkey)
                )))
                .label(title.clone())
                .checked(checked)
                .on_click(move |v: &bool, _w: &mut Window, cx: &mut App| {
                    let (v, tkey, bind2, themes2) =
                        (*v, tkey.clone(), bind2.clone(), themes2.clone());
                    try_update(&weak2, cx, "icon theme", move |this, cx| {
                        let cur = this.lookup(&bind2).cloned();
                        let mut sel = theme_selection(&themes2, cur.as_ref());
                        let want = (kind, tkey);
                        if v {
                            if !sel.contains(&want) {
                                sel.push(want);
                            }
                        } else {
                            sel.retain(|x| *x != want);
                        }
                        // Stable order = checkbox order, not click order.
                        sel.sort_by_key(|(k, t)| {
                            themes2
                                .iter()
                                .position(|(k2, t2, _)| k2 == k && t2 == t)
                                .unwrap_or(usize::MAX)
                        });
                        this.write(
                            &bind2,
                            Json::Array(
                                sel.iter()
                                    .map(|(k, t)| Json::String(theme_sel_encode(*k, t)))
                                    .collect(),
                            ),
                            cx,
                        );
                    })
                    .ok();
                });
                header = header.child(cb);
            }
        }
        // Size dropdown — `size:` drives icon size + cell stride. Sits
        // beside the other filters per #bugs.
        if chrome {
            let size_bind = self
                .two_way(node, "size")
                .unwrap_or_else(|| parse_path("studio.icon_size").expect("static path"));
            let size_sel = picker(
                vec![
                    "XSmall".into(),
                    "Small".into(),
                    "Medium".into(),
                    "Large".into(),
                ],
                size_bind,
            );
            header =
                header
                    .child(label("Size", cx))
                    .child(div().w(px(120.)).child(self.render_node(
                        &size_sel,
                        &format!("{key}-size"),
                        window,
                        cx,
                    )));
        }
        // License line + attribution alert for the browsed set. Bundled sets
        // have static licenses. Iconify sets carry catalog metadata (`name`,
        // `license.title/spdx`). Any license not in the no-attribution list
        // legally requires shipping the notice with the built app. Computed
        // before `set` moves into the list closure.
        let (set_title, license_title, spdx) = match set.as_str() {
            "lucide" => ("Lucide".to_owned(), "ISC".to_owned(), "ISC".to_owned()),
            "heroicons" => ("Heroicons".to_owned(), "MIT".to_owned(), "MIT".to_owned()),
            _ => crate::iconify::set_info(&set)
                .unwrap_or_else(|| (set.clone(), String::new(), String::new())),
        };
        let mut footer = v_flex().gap_1();
        if !license_title.is_empty() {
            footer = footer.child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!("{set_title} · {license_title}")),
            );
        }
        // Second line — size/author/version from the catalog entry and the
        // set file's `lastModified`.
        if let Some(entry) = crate::iconify::catalog_entry(&set) {
            let mut bits: Vec<String> = Vec::new();
            if let Some(total) = entry["total"].as_u64() {
                bits.push(format!("{total} icons"));
            }
            if let Some(author) = entry["author"]["name"].as_str() {
                bits.push(format!("by {author}"));
            }
            if let Some(version) = entry["version"].as_str() {
                bits.push(format!("v{version}"));
            }
            if let Some(palette) = entry["palette"].as_str() {
                bits.push(palette.to_owned());
            }
            if let Some(secs) = crate::iconify::last_modified(&set) {
                bits.push(format!("updated {}", crate::iconify::epoch_ymd(secs)));
            }
            if !bits.is_empty() {
                footer = footer.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(bits.join(" · ")),
                );
            }
        }
        // Download state — a live bar while a set loads, an error + retry
        // when it failed (the request key stays consumed until `retry`).
        let dl = crate::iconify::download_progress(&set);
        let dl_error = crate::iconify::fetch_error(&set);
        // Live download state in the footer while the grid still has content
        // — a reload (stale check / migrate) shouldn't blank the browser.
        if let Some((done, total)) = dl {
            let pct = if total > 0 {
                (done as f32 / total as f32) * 100.
            } else {
                0.
            };
            footer = footer.child(
                div()
                    .h(px(4.))
                    .w_full()
                    .child(Progress::new(format!("{key}-dl-bar")).value(pct)),
            );
        }
        if let Some(err) = &dl_error {
            footer = footer.child(
                div()
                    .text_xs()
                    .text_color(cx.theme().danger)
                    .child(format!("{set}: {err}")),
            );
        }
        if crate::iconify::spdx_needs_attribution(&spdx) {
            footer = footer.child(
                Alert::new(
                    format!("{key}-attr"),
                    format!(
                        "`{set_title}` is licensed {license_title} — the built app must ship its license text and copyright notices."
                    ),
                )
                .with_variant(AlertVariant::Error)
                .title("Attribution required")
                .banner(),
            );
        }
        // Cells — icon over its name. `selected` highlights, click writes the
        // bound path and fires `select` with the name baked as its arg. A
        // `uniform_list` of fixed-height rows virtualizes the ~1800-icon lucide
        // catalog — a flat flex-wrap grid of that many SVGs hangs layout.
        // Column count follows the list's measured width (the tracked scroll
        // handle's bounds, populated after first layout. The window viewport
        // minus sidebar chrome estimates the first frame). Resizes re-render
        // and re-wrap.
        // `size:` (XSmall/Small/Medium/Large) sizes both the icon glyphs and
        // the cell stride. The legacy `studio.icon_size` slot is the fallback
        // for docs that never bound the prop. Unset = Medium, the original
        // 88px.
        let icon_size = self
            .prop_text(node, "size")
            .filter(|s| !s.is_empty())
            .or_else(|| {
                get_path(self.state(), &parse_path("studio.icon_size").unwrap())
                    .and_then(|j| j.as_str())
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| "Medium".to_owned());
        let (cell, row_h) = match icon_size.as_str() {
            "XSmall" => (56., 44.),
            "Small" => (72., 60.),
            "Large" => (120., 108.),
            _ => (88., 84.),
        };
        let scroll = self
            .icons_scrolls
            .entry(key.to_owned())
            .or_default()
            .clone();
        let measured = scroll.0.borrow().base_handle.bounds().size.width;
        let w: f32 = if measured > px(0.) {
            measured.into()
        } else {
            (window.viewport_size().width - px(360.)).into()
        };
        let cols = ((w / cell).floor() as usize).max(1);
        // One fixed content width for EVERY cell — `w/cols` minus the
        // inter-cell gap. `flex_1`/caps gave full and partial rows different
        // widths. A pinned width puts every column on the same grid.
        let gap_px = 8.0_f32;
        let cell_w = ((w - (cols.saturating_sub(1) as f32) * gap_px) / cols as f32).max(24.);
        // Category grouping — `[all]`/unset renders every non-empty category
        // as a section (an icon in multiple categories appears under each).
        // a selection narrows to that one group. Sets without categories
        // render flat with no headers. Empty groups emit no rows at all.
        // Recompute Uncategorized against the merged, filtered name list.
        // `categories()` derives it from iconify names only, which misses
        // bundled-set extras and materialized copies a set file never saw.
        if !cats.is_empty() {
            let categorized: std::collections::HashSet<&str> = cats
                .iter()
                .filter(|(k, _)| k != "Uncategorized")
                .flat_map(|(_, l)| l.iter().map(String::as_str))
                .collect();
            let mut unc: Vec<String> = names
                .iter()
                .filter(|n| !categorized.contains(n.as_str()))
                .cloned()
                .collect();
            cats.retain(|(k, _)| k != "Uncategorized");
            if !unc.is_empty() {
                unc.sort_by(|a, b| crate::iconify::nat_cmp(a, b));
                cats.push(("Uncategorized".to_owned(), unc));
                cats.sort_by(|a, b| a.0.cmp(&b.0));
            }
        }

        enum IconRow {
            Header(String),
            Icons(Vec<String>),
        }
        let grouped = !cats.is_empty();
        let groups: Vec<(String, Vec<String>)> = if grouped {
            let valid: std::collections::HashSet<&String> = names.iter().collect();
            cats.iter()
                .filter(|(k, _)| cat.is_empty() || cat == "[all]" || *k == cat)
                .map(|(k, list)| {
                    let mut members: Vec<String> =
                        list.iter().filter(|n| valid.contains(n)).cloned().collect();
                    members.sort_by(|a, b| crate::iconify::nat_cmp(a, b));
                    (k.clone(), members)
                })
                .filter(|(_, members)| !members.is_empty())
                .collect()
        } else {
            vec![(String::new(), names.clone())]
        };
        let mut rows: Vec<IconRow> = Vec::new();
        for (title, members) in groups {
            if grouped {
                rows.push(IconRow::Header(format!("{title} · {}", members.len())));
            }
            rows.extend(members.chunks(cols).map(|c| IconRow::Icons(c.to_vec())));
        }
        let row_count = rows.len();
        let fmt_bytes = |n: u64| {
            if n >= 1 << 20 {
                format!("{:.1} MB", n as f64 / (1 << 20) as f64)
            } else if n >= 1 << 10 {
                format!("{:.0} KB", n as f64 / (1 << 10) as f64)
            } else {
                format!("{n} B")
            }
        };
        // Data-mode outputs — bound out-props publish everything the built-in
        // chrome renders so a doc can lay out its own (`bare:` suppresses the
        // chrome. The grid draws either way). `categories` includes the
        // computed Uncategorized bucket so a yaml picker can select it.
        // `themes` carries each option's encoded key + checked flag so a
        // doc-side checkbox row can toggle `theme:` through a host hook.
        // `meta`/`status`/`attribution` cover license info, download state
        // and the attribution warning.
        {
            let outs = [
                ("categories", self.two_way(node, "categories")),
                ("themes", self.two_way(node, "themes")),
                ("meta", self.two_way(node, "meta")),
                ("status", self.two_way(node, "status")),
                ("attribution", self.two_way(node, "attribution")),
            ];
            if outs.iter().any(|(_, o)| o.is_some()) {
                let entry = crate::iconify::catalog_entry(&set);
                let line1 = if license_title.is_empty() {
                    set_title.clone()
                } else {
                    format!("{set_title} · {license_title}")
                };
                let mut bits: Vec<String> = Vec::new();
                if let Some(total) = entry.as_ref().and_then(|e| e["total"].as_u64()) {
                    bits.push(format!("{total} icons"));
                }
                if let Some(author) = entry.as_ref().and_then(|e| e["author"]["name"].as_str()) {
                    bits.push(format!("by {author}"));
                }
                if let Some(version) = entry.as_ref().and_then(|e| e["version"].as_str()) {
                    bits.push(format!("v{version}"));
                }
                if let Some(palette) = entry.as_ref().and_then(|e| e["palette"].as_str()) {
                    bits.push(palette.to_owned());
                }
                if let Some(secs) = crate::iconify::last_modified(&set) {
                    bits.push(format!("updated {}", crate::iconify::epoch_ymd(secs)));
                }
                let (dl_label, dl_pct) = match dl {
                    Some((done, total)) => (
                        if total > 0 {
                            format!(
                                "Downloading {set}… {} / {}",
                                fmt_bytes(done),
                                fmt_bytes(total)
                            )
                        } else {
                            format!("Downloading {set}… {}", fmt_bytes(done))
                        },
                        if total > 0 {
                            done as f64 / total as f64 * 100.
                        } else {
                            0.
                        },
                    ),
                    None => (String::new(), 0.),
                };
                for (name, out) in outs {
                    let Some(path) = out else { continue };
                    let value = match name {
                        // Empty for category-less sets — a doc-side `if:`
                        // hides its picker rather than displaying a lone
                        // `[all]`. The built-in picker hides the same way.
                        "categories" => Json::Array(if cats.is_empty() {
                            Vec::new()
                        } else {
                            std::iter::once(Json::String("[all]".into()))
                                .chain(cats.iter().map(|(k, _)| Json::String(k.clone())))
                                .collect()
                        }),
                        "themes" => Json::Array(
                            themes
                                .iter()
                                .map(|(k, t, title)| {
                                    serde_json::json!({
                                        "key": theme_sel_encode(*k, t),
                                        "title": title,
                                        "checked": theme_checked
                                            .iter()
                                            .any(|(k2, t2)| k2 == k && t2 == t),
                                    })
                                })
                                .collect(),
                        ),
                        "meta" => serde_json::json!({
                            "set": set,
                            "title": set_title,
                            "license": license_title,
                            "spdx": spdx,
                            "line1": line1,
                            "line2": bits.join(" · "),
                        }),
                        "status" => serde_json::json!({
                            "downloading": dl.is_some(),
                            "label": dl_label,
                            "percent": dl_pct,
                            "error": dl_error.clone().unwrap_or_default(),
                        }),
                        "attribution" => Json::Bool(crate::iconify::spdx_needs_attribution(&spdx)),
                        _ => continue,
                    };
                    if get_path(self.state(), &path) != Some(&value) {
                        self.update_state(&path, value, cx);
                    }
                }
            }
        }
        let body = if row_count == 0 {
            if let Some((done, total)) = dl {
                let pct = if total > 0 {
                    (done as f32 / total as f32) * 100.
                } else {
                    0.
                };
                let label = if total > 0 {
                    format!(
                        "Downloading {set}… {} / {}",
                        fmt_bytes(done),
                        fmt_bytes(total)
                    )
                } else {
                    format!("Downloading {set}… {}", fmt_bytes(done))
                };
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(label),
                    )
                    .child(
                        div()
                            .w(px(320.))
                            .child(Progress::new(format!("{key}-dl")).value(pct)),
                    )
                    .into_any_element()
            } else if let Some(err) = &dl_error {
                let set2 = set.clone();
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .child(
                        Alert::new(format!("{key}-err"), format!("{set}: {err}"))
                            .with_variant(AlertVariant::Error)
                            .title("Icon set download failed")
                            .banner(),
                    )
                    .child(
                        Button::new(SharedString::from(format!("{key}-retry")))
                            .label("Retry")
                            .with_variant(ButtonVariant::Secondary)
                            .on_click(move |_, _, cx| crate::iconify::retry(&set2, cx)),
                    )
                    .into_any_element()
            } else {
                placeholder("no icons in this set".to_owned(), cx)
            }
        } else {
            let key = key.to_owned();
            let icon_size = icon_size.clone();
            // Scoped children — `children:` is the cell template, rendered
            // once per icon with `$item` =
            // `{name, set, selected, downloaded}` pushed (the same scope
            // mechanism `for:` uses). No children → the built-in
            // glyph+name cell renders.
            let cell_tpl = node.children.clone();
            uniform_list(
                format!("{key}-grid"),
                row_count,
                move |range, window, cx| {
                    range
                        .map(|ix| match &rows[ix] {
                            // Section header — a label row the same height
                            // as icon rows so `uniform_list`'s fixed stride
                            // stays valid. Bottom-aligned so the title sits
                            // just above its grid.
                            IconRow::Header(title) => div()
                                .flex()
                                .flex_row()
                                .items_end()
                                .h(px(row_h))
                                .pb_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(cx.theme().foreground)
                                        .child(title.clone()),
                                )
                                .into_any_element(),
                            IconRow::Icons(names) => {
                                let mut row = div().flex().flex_row().gap(px(gap_px)).h(px(row_h));
                                for name in names {
                                    let active = selected.as_deref() == Some(name.as_str());
                                    // A materialized name means the icon is
                                    // already on disk under `assets/icons/` —
                                    // ring it so the browser reads "have it"
                                    // at a glance. Bundled sets are all
                                    // embedded, so the mark adds no signal.
                                    let on_disk = materialized.contains(name)
                                        && !BUNDLED_SETS.contains(&set.as_str());
                                    let select_path = selected_path.clone();
                                    let handler = select_handler.clone();
                                    let cell_name = name.clone();
                                    let weak = weak.clone();
                                    let mut cell_div = div()
                                        .id(format!("{key}-{name}"))
                                        .flex()
                                        .flex_col()
                                        .items_center()
                                        .justify_center()
                                        .gap_1()
                                        // Pinned to the shared column
                                        // width — every row's cells are
                                        // the same box, so columns align
                                        // and partial rows left-pack.
                                        .w(px(cell_w))
                                        .flex_shrink_0()
                                        .p_2()
                                        .rounded_md()
                                        .cursor_pointer()
                                        .when(active, |d| d.bg(cx.theme().accent))
                                        .when(on_disk, |d| {
                                            d.border_1()
                                                .border_color(cx.theme().success.opacity(0.6))
                                        })
                                        .hover(|d| d.bg(cx.theme().accent.opacity(0.4)));
                                    if cell_tpl.is_empty() {
                                        // Never point `path()` at bytes that
                                        // don't exist — the image cache
                                        // retains a failed load forever. An
                                        // iconify name resolvable in the
                                        // cached set JSON renders through
                                        // the `svg()` layer. Otherwise the
                                        // whole set's one load is queued
                                        // and a placeholder displays until
                                        // `refresh_windows` flips
                                        // everything local.
                                        let cell_icon = if materialized.contains(name)
                                            || crate::iconify::icon_cached(&set, name)
                                            || !crate::iconify::known_set(&set)
                                        {
                                            icon_set(&set, name)
                                        } else {
                                            crate::iconify::ensure_set(&set, cx);
                                            icon("image")
                                        }
                                        .with_size(size(&icon_size));
                                        cell_div = cell_div.child(cell_icon).child(
                                            div()
                                                .w_full()
                                                .h(px(16.))
                                                .text_xs()
                                                .text_center()
                                                .text_color(if on_disk {
                                                    cx.theme().success
                                                } else {
                                                    cx.theme().muted_foreground
                                                })
                                                .whitespace_nowrap()
                                                .overflow_hidden()
                                                .text_ellipsis()
                                                .child(name.clone()),
                                        );
                                    } else {
                                        // The doc's template — the runtime's
                                        // own probe stays out (the doc's
                                        // `Icon` resolves lazily itself).
                                        let item = serde_json::json!({
                                            "name": name,
                                            "set": set,
                                            "selected": active,
                                            "downloaded": on_disk,
                                        });
                                        let tpl_key = format!("{key}/{name}");
                                        let weak2 = weak.clone();
                                        cell_div = cell_div.children(
                                            try_update(&weak2, cx, "icon cell", |view, cx| {
                                                view.scopes.push(("item".to_owned(), item));
                                                let mut out = Vec::new();
                                                for (ci, child) in cell_tpl.iter().enumerate() {
                                                    out.extend(view.expand(
                                                        child,
                                                        &format!("{tpl_key}/{ci}"),
                                                        &mut *window,
                                                        cx,
                                                    ));
                                                }
                                                view.scopes.pop();
                                                out
                                            })
                                            .unwrap_or_default(),
                                        );
                                    }
                                    row = row.child(
                                        cell_div
                                            .tooltip({
                                                let tip = name.clone();
                                                move |_, cx| {
                                                    cx.new(|_| Tooltip::new(tip.clone())).into()
                                                }
                                            })
                                            .on_click(move |_, _, cx| {
                                                let name = cell_name.clone();
                                                try_update(&weak, cx, "icon select", |this, cx| {
                                                    if let Some(path) = &select_path {
                                                        this.write(
                                                            path,
                                                            Json::String(name.clone()),
                                                            cx,
                                                        );
                                                    }
                                                    if let Some(handler) = &handler {
                                                        let synth = Node {
                                                            events: vec![EventBinding {
                                                                event: "select".into(),
                                                                handler: HandlerRef {
                                                                    name: handler.clone(),
                                                                    args: vec![Value::String(
                                                                        name.clone(),
                                                                    )],
                                                                },
                                                            }],
                                                            ..Default::default()
                                                        };
                                                        this.fire(&synth, "select", cx);
                                                    }
                                                })
                                                .ok();
                                            }),
                                    );
                                }
                                row.into_any_element()
                            }
                        })
                        .collect()
                },
            )
            .track_scroll(&scroll)
            .flex_1()
            .min_h_0()
            .into_any_element()
        };
        // `size_full` passes a bounded parent's height through to the inner
        // scroll slot (`flex_1/min_h_0`), same convention as `forge.Files`.
        let mut root = v_flex().size_full().gap_2();
        if chrome {
            root = root.child(header);
            if let Some(search_row) = search_row {
                root = root.child(search_row);
            }
            // License/attribution panel sits above the grid — it carries the
            // set's legal info and shouldn't hide below the fold.
            root = root.child(footer);
        }
        root.child(body).into_any_element()
    }

    /// `forge.Files` (#148). A file tree. Two listing modes: `of:` names a
    /// state path holding a JSON listing (`[{name, path, dir, children}]`),
    /// `root:` names a directory the element scans itself. `Filter:` globs
    /// limit file names, `depth:`/`hidden:` bound the scan, and a `nonce:`
    /// prop or the refresh button busts the cache after out-of-band changes.
    /// `search:` adds a filter box whose text `query:` exposes. `icons:` and
    /// `icon-map:` theme the rows. File rows write `bind selected` and fire
    /// `select`, folders just expand. Directory ids carry a trailing `/` so
    /// the click handler can skip them.
    fn files(
        &mut self,
        node: &Node,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let sel = self.two_way(node, "selected");
        // The bound value marks the "active" row (the open file in Studio).
        // External writes highlight its row even when the tree's own click
        // selection sits elsewhere.
        let sel_value = sel
            .as_ref()
            .and_then(|p| self.lookup(p))
            .and_then(|j| j.as_str())
            .map(str::to_owned);
        let show_icons = self.prop(node, "icons").is_none_or(|v| self.truthy(v));
        let icon_map = self
            .prop(node, "icon-map")
            .map(|v| self.icon_map(v))
            .unwrap_or_default();
        let empty_text = self
            .prop_text(node, "placeholder")
            .unwrap_or_else(|| "no files".into());
        let root = self.prop_text(node, "root").filter(|r| !r.is_empty());
        // `folders: navigate` — dir rows enter the folder in place instead of
        // expanding (`tree`, the default, expands in place). Only meaningful
        // in `root:` mode. The current dir is clamped under `root:` so project
        // switches and removed folders reset to the root.
        let navigate =
            root.is_some() && self.prop_text(node, "folders").as_deref() == Some("navigate");
        let current = if navigate {
            let root_str = root.clone().unwrap_or_default();
            Some(
                self.files_nav
                    .get(key)
                    .filter(|cur| std::path::Path::new(cur.as_str()).starts_with(root_str.as_str()))
                    .cloned()
                    .unwrap_or(root_str),
            )
        } else {
            None
        };
        // `of:` is an explicit state listing — when set it always wins over
        // `root:`. Absent, `root:` scans the filesystem itself.
        let (data, cache_key) = if root.is_some() && self.prop(node, "of").is_none() {
            let filters = self
                .prop(node, "filter")
                .map(|v| match v {
                    Value::List(_) | Value::Binding { .. } => self.list(v),
                    _ => self
                        .text(v)
                        .split(',')
                        .map(|s| s.trim().to_owned())
                        .filter(|s| !s.is_empty())
                        .collect(),
                })
                .unwrap_or_default();
            let depth = self
                .prop(node, "depth")
                .and_then(|v| self.number(v))
                .unwrap_or(6.0) as usize;
            let hidden = self.prop_bool(node, "hidden");
            let nonce = self
                .prop(node, "nonce")
                .map(|v| self.text(v))
                .unwrap_or_default();
            let scan_root = current
                .clone()
                .unwrap_or_else(|| root.clone().unwrap_or_default());
            // Navigate mode lists one folder flat — `depth:` still applies to
            // the scan's child collection, but only the top level is displayed.
            let depth = if navigate { 1 } else { depth };
            let cache_key = format!("{key}|{scan_root}|{filters:?}|{depth}|{hidden}|{nonce}");
            let entries = match self.files_cache.get(&cache_key) {
                Some(entries) => entries.clone(),
                None => {
                    let mut budget = 400usize;
                    let entries = scan_files(
                        std::path::Path::new(&scan_root),
                        &filters,
                        depth,
                        hidden,
                        &mut budget,
                    );
                    self.files_cache.insert(cache_key.clone(), entries.clone());
                    entries
                }
            };
            (entries, Some(cache_key))
        } else {
            let of = self
                .prop_text(node, "of")
                .unwrap_or_else(|| "studio.files".into());
            let data = parse_path(&of)
                .ok()
                .and_then(|p| get_path(self.state(), &p).cloned())
                .unwrap_or(Json::Null);
            (data.as_array().cloned().unwrap_or_default(), None)
        };

        // `query:` exposes the search text to the host. Without it the
        // component keeps the query in `files_queries` keyed by element.
        let query_path = self.two_way(node, "query");
        let query = match &query_path {
            Some(path) => self
                .lookup(path)
                .and_then(|j| j.as_str())
                .unwrap_or_default()
                .to_owned(),
            None => self.files_queries.get(key).cloned().unwrap_or_default(),
        };
        let mut entries = if query.is_empty() {
            data
        } else {
            filter_listing(&data, &query)
        };
        // Navigate mode — a `..` row climbs back toward `root:`. Injected
        // after search-filtering so it survives queries.
        if let (Some(root), Some(current)) = (&root, &current)
            && current != root
        {
            let parent = std::path::Path::new(current)
                .parent()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| root.clone());
            entries.insert(
                0,
                serde_json::json!({"name": "..", "path": parent, "dir": true, "children": []}),
            );
        }
        let highlights = self
            .prop(node, "highlights")
            .map(|v| self.highlights(v, cx))
            .unwrap_or_default();
        let mut items = Vec::new();
        Self::files_data(&entries, &mut items);

        // Search field + refresh button — the field is a real `InputState`
        // like the `Input` arm's, so it's keyboard-focusable and cleanable.
        let mut header = Vec::new();
        if self.prop_bool(node, "search") {
            let search_key = format!("{key}-search");
            let state = match self.inputs.get(&search_key) {
                Some(state) => state.clone(),
                None => {
                    let state = cx.new(|cx| {
                        InputState::new(window, cx)
                            .placeholder("filter files…")
                            .default_value(query.clone())
                    });
                    let path = query_path.clone();
                    let state_key = key.to_owned();
                    let subscription = cx.subscribe(&state, move |this, state, event, cx| {
                        if let InputEvent::Change = event {
                            let value = state.read(cx).value().to_string();
                            this.files_queries.insert(state_key.clone(), value.clone());
                            if let Some(path) = &path {
                                this.write(path, Json::String(value), cx);
                            }
                            cx.notify();
                        }
                    });
                    self.subscriptions.push(subscription);
                    self.inputs.insert(search_key, state.clone());
                    state
                }
            };
            // `query: bind` is controlled — external writes (clears, preseeds)
            // push into the cached state, same sync as the `Input` arm.
            if query_path.is_some() && state.read(cx).value() != query {
                state.update(cx, |s, cx| s.set_value(query.clone(), window, cx));
            }
            header.push(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(Input::new(&state).cleanable(true))
                    .into_any_element(),
            );
        }
        if let Some(cache_key) = cache_key {
            let weak = cx.entity().downgrade();
            header.push(
                Button::new(SharedString::from(format!("{key}-refresh")))
                    .icon(icon("rotate-ccw"))
                    .with_variant(ButtonVariant::Ghost)
                    .with_size(size("small"))
                    .tooltip("refresh")
                    .on_click(move |_, _, cx| {
                        try_update(&weak, cx, "files refresh", |this, cx| {
                            this.files_cache.remove(&cache_key);
                            cx.notify();
                        })
                        .ok();
                    })
                    .into_any_element(),
            );
        }

        let skey = format!("{key}|{}", bind_salt(node));
        // Collapse state survives the items rebuild below — the tree only
        // flattens `expanded` flags on the items it was last given, so they
        // carry over by id.
        let previous = self
            .trees
            .get(&skey)
            .map(|state| tree_expansion(state.read(cx)))
            .unwrap_or_default();
        carry_expansion(&mut items, &previous);
        let tree = if items.is_empty() {
            placeholder(
                if query.is_empty() {
                    empty_text
                } else {
                    format!("nothing matching `{query}`")
                },
                cx,
            )
        } else {
            self.files_tree(
                node, key, sel, sel_value, show_icons, icon_map, highlights, navigate, items, cx,
            )
        };
        // The `Tree` virtualizes rows off a uniform_list — it needs a
        // bounded slot or its scroll viewport is empty and nothing paints.
        // `size_full` on the column + `flex_1/min_h_0` on the tree slot lets
        // a bounded parent pass its height through. Unbounded parents fall
        // back to content height and the list just renders every row.
        v_flex()
            .size_full()
            .gap_1()
            .when(!header.is_empty(), |v| {
                v.child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .children(header),
                )
            })
            .child(div().flex_1().min_h_0().child(tree))
            .into_any_element()
    }

    /// The `Tree` half of `forge.Files` — rows carry the click/select/context
    /// wiring. `files` wraps it with the search header when needed. `sel_value`
    /// is the `selected:` bind's current value — the row whose id matches is
    /// painted active regardless of where the tree's own click selection sits.
    #[allow(clippy::too_many_arguments)]
    fn files_tree(
        &mut self,
        node: &Node,
        key: &str,
        sel: Option<Path>,
        sel_value: Option<String>,
        show_icons: bool,
        icon_map: Vec<(String, String)>,
        highlights: Vec<(String, FileHighlight)>,
        navigate: bool,
        items: Vec<TreeItem>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let skey = format!("{key}|{}", bind_salt(node));
        let state = match self.trees.get(&skey) {
            Some(state) => {
                state.update(cx, |state, cx| {
                    *state = std::mem::replace(state, TreeState::new(cx)).items(items.clone());
                });
                state.clone()
            }
            None => {
                let state = cx.new(|cx| TreeState::new(cx).items(items));
                self.trees.insert(skey.clone(), state.clone());
                state
            }
        };
        let weak = cx.entity().downgrade();
        let files_node = node.clone();
        let nav_key = key.to_owned();
        Tree::new(&state, move |ix, entry, selected, _, _cx| {
            let mut item = ListItem::new(ix);
            let path = entry.item().id.to_string();
            let is_dir = path.ends_with('/');
            let active = sel_value.as_deref() == Some(path.as_str());
            let sel = sel.clone();
            let files_node = files_node.clone();
            let weak = weak.clone();
            let nav_weak = weak.clone();
            let menu_weak = weak.clone();
            let nav_key = nav_key.clone();
            let nav_path = path.clone();
            let highlight = highlights
                .iter()
                .find(|(pattern, _)| glob_match(pattern, entry.item().label.as_str()))
                .map(|(_, h)| *h);
            let row = div()
                .id(format!("files-row-{}", entry.item().id))
                .flex()
                .flex_row()
                .items_center()
                .gap_1p5()
                .pl(px(entry.depth() as f32 * 14.))
                .when_some(highlight, |d, hl| {
                    let d = match hl.weight {
                        Some(w) => d.font_weight(w),
                        None => d,
                    };
                    let d = if hl.italic { d.italic() } else { d };
                    let d = if hl.underline { d.underline() } else { d };
                    match hl.color {
                        Some(c) => d.text_color(c),
                        None => d,
                    }
                })
                .when(show_icons, |d| {
                    d.child(
                        icon(&file_icon(
                            entry.item().label.as_str(),
                            is_dir,
                            entry.is_expanded(),
                            &icon_map,
                        ))
                        .with_size(size("small")),
                    )
                })
                .when(!is_dir, |d| {
                    d.on_mouse_down(MouseButton::Left, move |e, _, cx| {
                        let clicks = e.click_count;
                        try_update(&weak, cx, "event", |this, cx| {
                            if let Some(p) = &sel {
                                this.write(p, Json::String(path.clone()), cx);
                            }
                            // Single click selects, double fires `open` —
                            // hosts that don't distinguish can wire both to
                            // the same hook.
                            if clicks >= 2 {
                                this.fire(&files_node, "open", cx);
                            } else {
                                this.fire(&files_node, "select", cx);
                            }
                        })
                        .ok();
                    })
                })
                .when(is_dir && navigate, |d| {
                    d.on_mouse_down(MouseButton::Left, move |_, _, cx| {
                        let dir = nav_path.trim_end_matches('/').to_owned();
                        let key = nav_key.clone();
                        try_update(&nav_weak, cx, "files-nav", move |this, cx| {
                            this.files_nav.insert(key, dir);
                            cx.notify();
                        })
                        .ok();
                    })
                })
                // #148 nice-to-haves — right-click file ops. The items call
                // shell hooks (`files_new`/`files_reveal`/`files_rename`)
                // with the row's path. A host without those hooks sees no
                // action (the sink no-ops unknown calls).
                .context_menu({
                    let weak = menu_weak.clone();
                    let raw = entry.item().id.to_string();
                    move |menu, _, _| {
                        let path = raw.trim_end_matches('/').to_owned();
                        let hook_item = |label: &'static str, hook: &'static str| {
                            let weak = weak.clone();
                            let path = path.clone();
                            PopupMenuItem::new(label).on_click(move |_, _, cx| {
                                let weak = weak.clone();
                                let path = path.clone();
                                try_update(&weak, cx, "files-context", move |this, cx| {
                                    this.fire_call(hook, vec![path], cx);
                                })
                                .ok();
                            })
                        };
                        menu.item(hook_item("New document here", "files_new"))
                            .item(hook_item("Reveal in file manager", "files_reveal"))
                            .separator()
                            .item(hook_item("Rename…", "files_rename"))
                    }
                })
                .child(entry.item().label.to_string());
            item = item.child(row);
            item.selected(selected || active)
        })
        .into_any_element()
    }

    /// JSON `[{name, path, dir, children}]` → `TreeItem`s — the listing Studio
    /// writes to `studio.files`. Directories expand by default and get the `/`
    /// marker the row handler checks.
    fn files_data(entries: &[Json], items: &mut Vec<TreeItem>) {
        for e in entries {
            let name = e["name"].as_str().unwrap_or_default();
            let mut path = e["path"].as_str().unwrap_or_default().to_owned();
            let is_dir = e["dir"].as_bool().unwrap_or(false);
            if is_dir {
                path.push('/');
            }
            let mut data = TreeItem::new(&path, name);
            let mut subs = Vec::new();
            if let Some(children) = e["children"].as_array() {
                Self::files_data(children, &mut subs);
            }
            if !subs.is_empty() {
                data = data.children(subs).expanded(true);
            }
            items.push(data);
        }
    }

    /// `forge.Showcase` (#125) — the theme editor's live widget gallery. Same
    /// library dropdown as `forge.Palette` (state `studio.showcase_lib`), but
    /// each element renders as a real widget under its catalog `group:` in an
    /// accordion. The current theme's effect is visible on every widget.
    fn showcase(
        &mut self,
        _node: &Node,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let doc_base = get_path(self.state(), &parse_path("studio.doc_base").unwrap())
            .and_then(|j| j.as_str())
            .unwrap_or(crate::base::BASE_ID)
            .to_owned();
        let mut libs: Vec<(String, String, String)> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        for cat in &self.catalogs {
            if cat.library == "forge" || !cat.supports_base(&doc_base) {
                continue;
            }
            if !seen.insert(cat.library.clone()) {
                continue;
            }
            let alias = self
                .libraries
                .iter()
                .find(|l| l.catalog().is_some_and(|c| c.library == cat.library))
                .map(|l| l.alias().to_string())
                .unwrap_or_default();
            let label = cat.name.clone().unwrap_or_else(|| cat.library.clone());
            libs.push((label, cat.library.clone(), alias));
        }
        let selected = get_path(self.state(), &parse_path("studio.showcase_lib").unwrap())
            .and_then(|j| j.as_str())
            .map(str::to_owned)
            .unwrap_or_else(|| {
                libs.first()
                    .map(|(_, id, _)| id.clone())
                    .unwrap_or_default()
            });
        let current_label = libs
            .iter()
            .find(|(_, id, _)| id == &selected)
            .map(|(l, _, _)| l.clone())
            .unwrap_or_else(|| selected.clone());

        let weak_menu = cx.entity().downgrade();
        let selector = Button::new(SharedString::from(format!("{key}-lib")))
            .label(SharedString::from(current_label))
            .with_variant(ButtonVariant::Secondary)
            .with_size(size("small"))
            .dropdown_menu({
                let libs = libs.clone();
                move |menu, _, _| {
                    let mut menu = menu;
                    for (label, id, _) in &libs {
                        let weak = weak_menu.clone();
                        let id = id.clone();
                        menu = menu.item(PopupMenuItem::new(label.clone()).on_click(
                            move |_, _, cx| {
                                try_update(&weak, cx, "showcase", |this, cx| {
                                    this.update_state(
                                        &parse_path("studio.showcase_lib").unwrap(),
                                        Json::String(id.clone()),
                                        cx,
                                    )
                                })
                                .ok();
                            },
                        ));
                    }
                    menu
                }
            });

        let prefix = libs
            .iter()
            .find(|(_, id, _)| id == &selected)
            .map(|(_, _, p)| p.clone())
            .unwrap_or_default();
        let mut groups: Vec<(String, Vec<forge_spec::catalog::ElementDef>)> = Vec::new();
        if let Some(cat) = self.catalogs.iter().find(|c| c.library == selected) {
            for def in &cat.elements {
                let group = def.group.clone().unwrap_or_else(|| "Elements".into());
                match groups.iter_mut().find(|(g, _)| g == &group) {
                    Some((_, items)) => items.push(def.clone()),
                    None => groups.push((group, vec![def.clone()])),
                }
            }
        }
        let open_path = parse_path("studio.showcase_open").unwrap();
        let open_set: Option<Vec<usize>> = get_path(self.state(), &open_path)
            .and_then(|j| j.as_array().cloned())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_u64().map(|u| u as usize))
                    .collect()
            });
        let weak_toggle = cx.entity().downgrade();
        let open_path_c = open_path.clone();
        let mut grid = Accordion::new(SharedString::from(format!("{key}-groups")))
            .multiple(true)
            .bordered(false)
            .on_toggle_click(move |indices: &[usize], _, cx| {
                let open_path = open_path_c.clone();
                try_update(&weak_toggle, cx, "showcase", |this, cx| {
                    this.update_state(&open_path, Json::from(indices.to_vec()), cx)
                })
                .ok();
            });
        for (i, (group, defs)) in groups.iter().enumerate() {
            let mut col = div().flex().flex_col().gap_3().w_full().pb_2();
            for def in defs {
                let name = def.name.clone();
                // Synthesized node: element name as positional content and
                // sample values for props, so the widget displays a themed state.
                let mut props: Entries = Vec::new();
                for p in &def.props {
                    if let Some(v) = sample_prop_value(p) {
                        props.push((p.name.clone(), v));
                    }
                }
                let synth = Node {
                    element: ElementRef {
                        namespace: if prefix.is_empty() {
                            None
                        } else {
                            Some(prefix.clone())
                        },
                        name: name.clone(),
                    },
                    content: def.content.as_ref().map(|_| Value::String(name.clone())),
                    props,
                    ..Default::default()
                };
                let widget = self.render_node(&synth, &format!("{key}-{i}-{name}"), window, cx);
                col = col.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .w_full()
                        .child(div().text_xs().text_color(muted).child(name.clone()))
                        .child(widget),
                );
            }
            let open = open_set.as_ref().is_none_or(|set| set.contains(&i));
            let item = gpui_kit::component::accordion::AccordionItem::new()
                .title(SharedString::from(group.clone()))
                .open(open)
                .child(col);
            grid = grid.item(move |_| item);
        }
        div()
            .flex()
            .flex_col()
            .gap_2()
            .w_full()
            .child(selector)
            .child(grid)
            .into_any_element()
    }

    fn fire(&mut self, node: &Node, event: &str, cx: &mut Context<Self>) {
        let Some(binding) = node.events.iter().find(|e| e.event == event) else {
            return;
        };
        let args: Vec<String> = binding.handler.args.iter().map(|a| self.text(a)).collect();
        let name = binding.handler.name.clone();
        // Hook handlers may `entity.update` — possibly reentrantly — so a panic is
        // caught and reported to the host instead of killing the window.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if let Some(handler) = self.hooks.get(&name).cloned() {
                handler(&args, self, cx);
            } else if let Some(sink) = self.on_hook.clone() {
                let call = format!("{}({})", name, args.join(", "));
                sink(&call, self, cx);
            }
        }));
        if let Err(payload) = result {
            let detail = payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown panic".to_owned());
            self.report(format!("hook `{name}` failed: {detail}"));
            cx.notify();
        }
    }

    fn write(&mut self, path: &Path, value: Json, cx: &mut Context<Self>) {
        set_path(&mut self.state, path, value);
        if let Some(sink) = self.on_state.clone() {
            let state = self.state.clone();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                sink(&state, self, cx);
            }));
            if let Err(payload) = result {
                let detail = payload
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_else(|| "unknown panic".to_owned());
                self.report(format!("state sink failed: {detail}"));
            }
        }
        cx.notify();
    }

    /// Walks `json` down a spec `Path` — the read side of `set_path`.
    fn lookup(&self, path: &Path) -> Option<&Json> {
        let mut segments = path.0.iter();
        // `meta.*` reads the document's `meta:` map — a constant namespace.
        if let Some(PathSegment::Field(name)) = path.0.first()
            && name == "meta"
        {
            segments.next();
            return segments.try_fold(&self.meta, |json, segment| match segment {
                PathSegment::Field(name) => json.get(name),
                PathSegment::Index(i) => json.get(i),
            });
        }
        let mut json = &self.state;
        // `for`/`with` scope vars shadow state — innermost frame wins.
        if let Some(PathSegment::Field(name)) = path.0.first()
            && let Some((_, value)) = self.scopes.iter().rev().find(|(n, _)| n == name)
        {
            json = value;
            segments.next();
        }
        segments.try_fold(json, |json, segment| match segment {
            PathSegment::Field(name) => json.get(name),
            PathSegment::Index(i) => json.get(i),
        })
    }

    fn text(&self, value: &Value) -> String {
        match value {
            Value::String(s) | Value::Enum(s) => s.clone(),
            Value::Interpolated(segments) => segments
                .iter()
                .map(|s| match s {
                    Segment::Text(t) => t.clone(),
                    Segment::Binding(p) => self.json_text(p),
                })
                .collect(),
            Value::Number { value, unit } => {
                format!("{value}{}", unit.map(Unit::as_str).unwrap_or_default())
            }
            Value::Bool(b) => b.to_string(),
            Value::Color(c) => c.to_string(),
            Value::Token(p) => format!("$theme.{p}"),
            Value::List(items) => items
                .iter()
                .map(|i| self.text(i))
                .collect::<Vec<_>>()
                .join(", "),
            Value::Map(entries) => {
                let items = entries
                    .iter()
                    .map(|(k, v)| format!("{k}: {}", self.text(v)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{items}}}")
            }
            Value::Binding { path, .. } => self.json_text(path),
        }
    }

    fn json_text(&self, path: &Path) -> String {
        match self.lookup(path) {
            Some(Json::String(s)) => s.clone(),
            Some(Json::Null) | None => String::new(),
            Some(other) => other.to_string(),
        }
    }

    fn truthy(&self, value: &Value) -> bool {
        match value {
            Value::Bool(b) => *b,
            Value::Binding { path, .. } => match self.lookup(path) {
                Some(Json::Bool(b)) => *b,
                Some(Json::Null) | None => false,
                Some(Json::String(s)) => !s.is_empty(),
                Some(Json::Number(n)) => n.as_f64() != Some(0.0),
                Some(_) => true,
            },
            _ => false,
        }
    }

    fn number(&self, value: &Value) -> Option<f64> {
        match value {
            Value::Number { value, .. } => Some(*value),
            Value::Binding { path, .. } => self.lookup(path).and_then(Json::as_f64),
            _ => None,
        }
    }

    fn list(&self, value: &Value) -> Vec<String> {
        match value {
            Value::List(items) => items.iter().map(|i| self.text(i)).collect(),
            Value::Binding { path, .. } => match self.lookup(path) {
                Some(Json::Array(items)) => items
                    .iter()
                    .map(|i| i.as_str().map_or_else(|| i.to_string(), str::to_owned))
                    .collect(),
                _ => Vec::new(),
            },
            other => vec![self.text(other)],
        }
    }

    /// A `Value` as state `Json` — bindings look up, literals convert.
    /// `pub` so hosts (Studio's pane sync) evaluate control flow the same way.
    pub fn resolve(&self, value: &Value) -> Json {
        match value {
            Value::Binding { path, .. } => self.lookup(path).cloned().unwrap_or(Json::Null),
            Value::Bool(b) => Json::Bool(*b),
            Value::Number { value, unit: None } => Json::from(*value),
            Value::List(items) => Json::Array(items.iter().map(|i| self.resolve(i)).collect()),
            Value::Map(entries) => Json::Object(
                entries
                    .iter()
                    .map(|(k, v)| (k.clone(), self.resolve(v)))
                    .collect(),
            ),
            other => Json::String(self.text(other)),
        }
    }

    /// An `if:` condition — truthiness alone, or equality against `is:`/`not:`.
    /// `pub` so hosts (Studio's pane sync) evaluate control flow the same way.
    pub fn condition(&self, when: &Value, test: &Option<Compare>) -> bool {
        let value = self.resolve(when);
        match test {
            None => json_truthy(&value),
            Some(Compare::Is(operand)) => self.value_eq(&value, &self.resolve(operand)),
            Some(Compare::Not(operand)) => !self.value_eq(&value, &self.resolve(operand)),
        }
    }

    /// `is:`/`not:`/`case` equality — numbers compare by value so a `u64`
    /// index write (`Json::from(usize)`) and a `4` literal (`f64`) agree.
    /// serde_json `Number` equality is representation-strict. `4 != 4.0`.
    pub fn value_eq(&self, a: &Json, b: &Json) -> bool {
        match (a, b) {
            (Json::Number(x), Json::Number(y)) => x.as_f64() == y.as_f64(),
            _ => a == b,
        }
    }

    /// `for:`'s source as items — a single (non-list) value iterates once,
    /// which is what makes `for` double as a per-object template.
    /// `pub` so hosts (Studio's pane sync) evaluate control flow the same way.
    pub fn for_items(&self, source: &Value) -> Vec<Json> {
        match self.resolve(source) {
            Json::Array(items) => items,
            Json::Null => Vec::new(),
            value => vec![value],
        }
    }

    /// Renders `nodes` as siblings — control blocks and control-modified
    /// elements each expand to zero or more elements. A `for` loop, not
    /// `flat_map`: iterator adaptor frames cost kilobytes of stack each, and
    /// this recurses once per document depth. Deep docs overflow the
    /// default 8MB stack otherwise.
    fn render_kids(
        &mut self,
        nodes: &[Node],
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let mut out = Vec::with_capacity(nodes.len());
        for (i, node) in nodes.iter().enumerate() {
            out.extend(self.expand(node, &format!("{key}/{i}"), window, cx));
        }
        out
    }

    /// One node's expansion — `if` gates, `for` repeats, `with` scopes and
    /// `match` picks an arm. Plain elements render once.
    fn expand(
        &mut self,
        node: &Node,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let Some(control) = &node.control else {
            return vec![self.render_node(node, key, window, cx)];
        };
        match control {
            Control::If {
                when,
                test,
                otherwise,
            } => {
                if self.condition(when, test) {
                    self.render_target(node, key, window, cx)
                } else {
                    self.render_kids(otherwise, key, window, cx)
                }
            }
            Control::For {
                source,
                each,
                index,
                key: key_path,
                otherwise,
            } => {
                let items = self.for_items(source);
                if items.is_empty() {
                    return self.render_kids(otherwise, key, window, cx);
                }
                let mut out = Vec::new();
                for (i, item) in items.into_iter().enumerate() {
                    self.scopes.push((each.clone(), item));
                    if let Some(index) = index {
                        self.scopes.push((index.clone(), Json::from(i)));
                    }
                    // `key: item.id` — resolve the member path against the
                    // scoped item for stable identity (default: position).
                    let item_key = key_path
                        .as_ref()
                        .and_then(|path| {
                            let resolved = self.resolve(&Value::Binding {
                                path: path.clone(),
                                mode: BindingMode::OneWay,
                            });
                            resolved
                                .as_str()
                                .map(str::to_owned)
                                .or_else(|| (!resolved.is_null()).then(|| resolved.to_string()))
                        })
                        .unwrap_or_else(|| i.to_string());
                    out.extend(self.render_target(node, &format!("{key}/{item_key}"), window, cx));
                    let pop = 1 + usize::from(index.is_some());
                    self.scopes.truncate(self.scopes.len() - pop);
                }
                out
            }
            Control::With { source, name } => {
                let value = self.resolve(source);
                self.scopes.push((name.clone(), value));
                let out = self.render_target(node, key, window, cx);
                self.scopes.pop();
                out
            }
            Control::Match {
                source,
                cases,
                otherwise,
            } => {
                let value = self.resolve(source);
                match cases
                    .iter()
                    .find(|case| self.value_eq(&self.resolve(&case.value), &value))
                {
                    Some(case) => self.render_kids(&case.children, key, window, cx),
                    None => self.render_kids(otherwise, key, window, cx),
                }
            }
        }
    }

    /// The control node's payload. A bare block expands its `children`. A
    /// modified element renders once with the control consumed and scope vars
    /// baked into event args (events fire after the scope unwinds).
    fn render_target(
        &mut self,
        node: &Node,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        if node.is_block() {
            self.render_kids(&node.children, key, window, cx)
        } else {
            let mut node = node.clone();
            node.control = None;
            self.bake_scope(&mut node);
            vec![self.render_node(&node, key, window, cx)]
        }
    }

    /// Rewrites event-arg bindings rooted in a `for`/`with` scope var to the
    /// resolved literal, recursively — event handlers fire after the scope
    /// that named `$item` has unwound.
    fn bake_scope(&self, node: &mut Node) {
        if self.scopes.is_empty() {
            return;
        }
        for event in &mut node.events {
            for arg in &mut event.handler.args {
                self.bake_value(arg);
            }
        }
        for child in &mut node.children {
            self.bake_scope(child);
        }
        match &mut node.control {
            Some(Control::Match {
                cases, otherwise, ..
            }) => {
                for case in cases {
                    for child in &mut case.children {
                        self.bake_scope(child);
                    }
                }
                for child in otherwise {
                    self.bake_scope(child);
                }
            }
            Some(Control::If { otherwise, .. }) | Some(Control::For { otherwise, .. }) => {
                for child in otherwise {
                    self.bake_scope(child);
                }
            }
            _ => {}
        }
    }

    fn bake_value(&self, value: &mut Value) {
        match value {
            Value::Binding { path, .. } if self.scoped(path) => {
                *value = literal(self.lookup(path).cloned().unwrap_or(Json::Null));
            }
            Value::Interpolated(segments)
                if segments
                    .iter()
                    .any(|s| matches!(s, Segment::Binding(p) if self.scoped(p))) =>
            {
                *value = Value::String(self.text(&value.clone()));
            }
            Value::List(items) => items.iter_mut().for_each(|i| self.bake_value(i)),
            Value::Map(entries) => entries.iter_mut().for_each(|(_, v)| self.bake_value(v)),
            _ => {}
        }
    }

    /// `path` is rooted in a `for`/`with` scope var.
    fn scoped(&self, path: &Path) -> bool {
        matches!(
            path.0.first(),
            Some(PathSegment::Field(name))
                if self.scopes.iter().rev().any(|(n, _)| n == name)
        )
    }

    /// An `Editor`'s `diagnostics:` prop — squiggle markers as `{line, col,
    /// end_col?, severity?, message, hint?}` maps (1-based line/col), from a
    /// bound list (`$studio.markers`, fed by `forge check`) or an inline list.
    /// No `end_col` underlines to end-of-line (#128).
    fn markers(&self, value: &Value) -> Vec<gpui_base::input::Diagnostic> {
        use gpui_base::input::{Diagnostic, DiagnosticSeverity, Position};
        let items: Vec<Json> = match value {
            Value::Binding { path, .. } => match self.lookup(path) {
                Some(Json::Array(items)) => items.clone(),
                _ => Vec::new(),
            },
            Value::List(items) => items
                .iter()
                .filter_map(|item| match item {
                    Value::Map(entries) => Some(Json::Object(
                        entries
                            .iter()
                            .map(|(k, v)| {
                                let json = match v {
                                    Value::Bool(b) => Json::Bool(*b),
                                    Value::Number { value, .. } => Json::from(*value),
                                    other => Json::String(self.text(other)),
                                };
                                (k.clone(), json)
                            })
                            .collect(),
                    )),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        items
            .iter()
            .filter_map(|item| {
                let line = item.get("line")?.as_u64()? as u32;
                let col = item.get("col").and_then(Json::as_u64).unwrap_or(1) as u32;
                let start = Position::new(line.saturating_sub(1), col.saturating_sub(1));
                let end = item
                    .get("end_col")
                    .and_then(Json::as_u64)
                    .map(|e| (e as u32).saturating_sub(1).max(start.character + 1))
                    .unwrap_or(u32::MAX);
                let mut message = item
                    .get("message")
                    .and_then(Json::as_str)
                    .unwrap_or_default()
                    .to_owned();
                if let Some(hint) = item.get("hint").and_then(Json::as_str) {
                    message = format!("{message} — {hint}");
                }
                let severity = match item.get("severity").and_then(Json::as_str) {
                    Some("warning") => DiagnosticSeverity::Warning,
                    Some("info") => DiagnosticSeverity::Info,
                    Some("hint") => DiagnosticSeverity::Hint,
                    _ => DiagnosticSeverity::Error,
                };
                Some(
                    Diagnostic::new(start..Position::new(start.line, end), message)
                        .with_severity(severity)
                        .with_source("forge check"),
                )
            })
            .collect()
    }

    /// The catalog a node's element defs come from — its namespace when set,
    /// else the document's base catalog, else the first (gpui-kit). Foreign-
    /// base nodes inside previews resolve props against their own library.
    fn node_catalog<'a>(&'a self, node: &'a Node) -> Option<&'a Catalog> {
        node.element
            .namespace
            .as_deref()
            .or_else(|| self.doc.as_ref().and_then(|d| d.base.as_deref()))
            .and_then(|lib| self.catalogs.iter().find(|c| c.library == lib))
            .or_else(|| self.catalogs.first())
    }

    /// A prop's value: the node's content when the catalog maps it to this prop, then the
    /// written prop, then the catalog default.
    fn prop<'a>(&'a self, node: &'a Node, name: &str) -> Option<&'a Value> {
        let def = self
            .node_catalog(node)
            .and_then(|c| c.element(&node.element.name));
        if let Some(content) = &node.content
            && def.and_then(|d| d.content.as_deref()) == Some(name)
        {
            return Some(content);
        }
        node.props
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v)
            .or_else(|| def?.prop(name)?.default.as_ref())
    }

    /// Builds an `Avatar` from a node — used directly and per child of `AvatarGroup`.
    fn avatar(&self, node: &Node, cx: &mut App) -> Avatar {
        let mut avatar = Avatar::new();
        if let Some(name) = self.prop_text(node, "name") {
            avatar = avatar.name(name);
        }
        if let Some(src) = self.prop_text(node, "src") {
            avatar = avatar.src(src);
        }
        if let Some(name) = self.prop_text(node, "icon") {
            avatar = avatar.placeholder(self.doc_icon(&name, cx));
        }
        avatar
    }

    /// Renders a slot element's children — `MessageHeader`, `TableCaption`, etc.
    fn slot_children(
        &mut self,
        node: &Node,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let mut out = Vec::with_capacity(node.children.len());
        for (i, child) in node.children.iter().enumerate() {
            out.extend(self.expand(child, &format!("{key}/{i}"), window, cx));
        }
        out
    }

    /// A `SettingPage` node — `title`/`text` prop or positional content names it.
    /// Children are `SettingGroup`s.
    fn setting_page(
        &mut self,
        node: &Node,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> SettingPage {
        let title = self
            .prop_text(node, "title")
            .or_else(|| self.prop_text(node, "text"))
            .unwrap_or_default();
        let groups = node
            .children
            .iter()
            .enumerate()
            .map(|(i, c)| self.setting_group(c, &format!("{key}/{i}"), window, cx))
            .collect::<Vec<_>>();
        SettingPage::new(title).groups(groups)
    }

    /// A `SettingGroup` node — children are `SettingItem`s.
    fn setting_group(
        &mut self,
        node: &Node,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> SettingGroup {
        let title = self
            .prop_text(node, "title")
            .or_else(|| self.prop_text(node, "text"))
            .unwrap_or_default();
        let items = node
            .children
            .iter()
            .enumerate()
            .map(|(i, c)| self.setting_item(c, &format!("{key}/{i}"), window, cx))
            .collect::<Vec<_>>();
        SettingGroup::new().title(title).items(items)
    }

    /// A `SettingItem` node — `title`/`text` names the row. Children render as
    /// its field element lazily inside the dialog's render.
    fn setting_item(
        &mut self,
        node: &Node,
        key: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> SettingItem {
        let title = self
            .prop_text(node, "title")
            .or_else(|| self.prop_text(node, "text"));
        let weak = cx.entity().downgrade();
        let node = node.clone();
        let key = key.to_owned();
        SettingItem::render(move |_, window, cx| {
            let kids = try_update(&weak, cx, "setting-item", |v, cx| {
                v.slot_children(&node, &format!("{key}/0"), window, cx)
            })
            .unwrap_or_default();
            let field = div().children(kids);
            match &title {
                Some(title) => div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(Label::new(title.clone()))
                    .child(field)
                    .into_any_element(),
                None => field.into_any_element(),
            }
        })
    }

    /// Builds a `TableHead` — shared by `Table` header cells and standalone nodes.
    fn table_head(&self, node: &Node) -> TableHead {
        let mut head = TableHead::new().child(self.prop_text(node, "label").unwrap_or_default());
        if let Some(n) = self.prop(node, "col-span").and_then(|v| self.number(v)) {
            head = head.col_span(n as usize);
        }
        match self.prop_text(node, "text-align").as_deref() {
            Some("center") => head = head.text_center(),
            Some("right") => head = head.text_right(),
            _ => {}
        }
        head
    }

    /// Builds a `TableCell` — shared by `Table` body/footer cells and standalone nodes.
    fn table_cell(&self, node: &Node) -> TableCell {
        let mut cell = TableCell::new().child(self.prop_text(node, "value").unwrap_or_default());
        if let Some(n) = self.prop(node, "col-span").and_then(|v| self.number(v)) {
            cell = cell.col_span(n as usize);
        }
        match self.prop_text(node, "text-align").as_deref() {
            Some("center") => cell = cell.text_center(),
            Some("right") => cell = cell.text_right(),
            _ => {}
        }
        cell
    }

    /// Writes a prop on the node at `path` in this view's document — inspector edits.
    /// `json` arrives from the bound field's state. The catalog prop type maps it to `Value`.
    /// `Json::Null` removes the prop.
    pub fn set_node_prop(&mut self, path: &str, name: &str, json: &Json, cx: &mut Context<Self>) {
        let Some(node) = self.doc.as_ref().and_then(|doc| doc.node_at(path)) else {
            return;
        };
        let (element, namespaced) = (node.element.name.clone(), node.element.namespace.is_some());
        let ty = if namespaced {
            None
        } else {
            self.catalogs
                .first()
                .and_then(|catalog| catalog.element(&element))
                .and_then(|def| def.prop(name))
                .map(|prop| prop.ty.clone())
        };
        let value = if json.is_null() {
            None
        } else {
            ty.as_ref()
                .and_then(|ty| self.json_value(ty, json))
                .or_else(|| json.as_str().map(|s| Value::String(s.to_owned())))
        };
        if let Some(doc) = self.doc.as_mut() {
            doc.set_prop(path, name, value);
            cx.notify();
        }
    }

    /// `refname_changed` write-through — `doc.set_refname`. The preserving
    /// writer emits `<<: *name` (or a new `name: &name` block).
    pub fn set_node_refname(
        &mut self,
        path: &str,
        refname: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if let Some(doc) = self.doc.as_mut() {
            doc.set_refname(path, refname);
            cx.notify();
        }
    }

    /// `Style` tab write-through — `style_changed` → `doc.set_style`.
    pub fn set_node_style(&mut self, path: &str, name: &str, json: &Json, cx: &mut Context<Self>) {
        let value = style_edit_value(json);
        if let Some(doc) = self.doc.as_mut() {
            doc.set_style(path, name, value);
            cx.notify();
        }
    }

    /// Records a render-time problem — surfaced to the host as a toast next paint.
    pub fn report(&mut self, message: impl Into<String>) {
        self.errors.push(message.into());
    }

    /// Sets the directory the current document lives in — relative `src` paths
    /// on `Svg`/image elements resolve against it.
    pub fn set_base_dir(&mut self, dir: impl Into<std::path::PathBuf>) {
        self.base_dir = Some(dir.into());
    }

    /// Element-picker mode — while on, every rendered node is clickable and
    /// selects itself via `document.selected` before the mode clears.
    pub fn set_pick(&mut self, pick: bool, cx: &mut Context<Self>) {
        self.pick = pick;
        self.placing = false;
        self.pick_zones.borrow_mut().clear();
        *self.pick_hover.borrow_mut() = None;
        cx.notify();
    }

    /// Click-to-place mode — the same overlay as `pick`, but the click reports
    /// `studio.drop_target` and fires `place_component` for the shell.
    pub fn set_placing(&mut self, placing: bool, cx: &mut Context<Self>) {
        self.pick = placing;
        self.placing = placing;
        self.pick_zones.borrow_mut().clear();
        *self.pick_hover.borrow_mut() = None;
        cx.notify();
    }

    /// Enables drag & drop on rendered nodes (the preview canvas).
    pub fn set_dnd(&mut self, dnd: bool, cx: &mut Context<Self>) {
        self.dnd = dnd;
        cx.notify();
    }

    /// Fires a registered hook by name, bubbling to the host's sink when this
    /// view doesn't handle it — the shared path for events and drops. Public
    /// so hosts can trigger doc events programmatically (tests, IPC).
    pub fn fire_call(&mut self, name: &str, args: Vec<String>, cx: &mut Context<Self>) {
        if let Some(handler) = self.hooks.get(name).cloned() {
            handler(&args, self, cx);
        } else if let Some(sink) = self.on_hook.clone() {
            sink(&format!("{}({})", name, args.join(", ")), self, cx);
        }
    }

    /// Registers a namespaced-element library: its catalog is merged for
    /// validation/props and `alias.<name>` elements render through it.
    pub fn register_library(&mut self, lib: impl ForgeLibrary + 'static) {
        if let Some(cat) = lib.catalog().cloned() {
            self.catalogs.push(cat);
        }
        self.libraries.push(Rc::new(lib));
    }

    /// The host's iconset — `[icons] set` in forge.toml. `meta.iconset` on
    /// the document itself still wins. `lucide` is the bundled default.
    pub fn set_project_iconset(&mut self, set: Option<String>) {
        self.project_iconset = set;
    }

    /// The iconset document icons resolve against — `meta.iconset`, then the
    /// project setting, then `lucide`.
    fn iconset(&self) -> &str {
        self.meta
            .get("iconset")
            .and_then(|j| j.as_str())
            .or(self.project_iconset.as_deref())
            .unwrap_or("lucide")
    }

    /// A document-facing icon (`icon:` props, `Icon` elements) — follows
    /// `iconset()`. Chrome glyphs (chevrons, file rows, toolbar) stay on the
    /// bundled lucide set via `icon()`. The asset-source probe resolves every
    /// local layer (embedded, fs roots, iconify cache). Only when bytes are
    /// missing *and* the set is iconify-hosted does it queue a lazy load and
    /// display a placeholder. The image cache retains a failed load, so the
    /// real path must not render until the SVG exists.
    fn doc_icon(&self, name: &str, cx: &mut App) -> Icon {
        doc_icon_for(self.iconset(), name, cx)
    }

    /// Resolves `src` against `base_dir` when relative — `std::fs` calls use this
    /// so `src` is document-relative, not cwd-relative. Public for adapters
    /// (forge-lib-*) that load files from `src`-style props.
    pub fn resolve_path(&self, src: &str) -> std::path::PathBuf {
        let path = std::path::Path::new(src);
        if path.is_relative()
            && let Some(base) = &self.base_dir
        {
            return base.join(path);
        }
        path.to_path_buf()
    }

    /// Drains accumulated errors — the host captures them to display or handle.
    pub fn take_errors(&mut self) -> Vec<String> {
        let mut errors = std::mem::take(&mut self.errors);
        errors.append(&mut PENDING_ERRORS.lock().unwrap_or_else(|e| e.into_inner()));
        errors
    }

    /// Maps a state `Json` to a spec `Value` via the catalog prop type.
    fn json_value(&self, ty: &PropType, json: &Json) -> Option<Value> {
        match json {
            Json::Bool(b) => Some(Value::Bool(*b)),
            Json::Number(n) => n.as_f64().map(|value| Value::Number { value, unit: None }),
            Json::String(s) => match ty {
                PropType::Enum(_) => Some(Value::Enum(s.clone())),
                PropType::Number => s
                    .parse()
                    .ok()
                    .map(|value| Value::Number { value, unit: None }),
                PropType::Bool => s.parse().ok().map(Value::Bool),
                PropType::Color => Color::parse_hex(s).map(Value::Color),
                _ => Some(Value::String(s.clone())),
            },
            _ => None,
        }
    }

    /// Maps a prop `Value` back to state `Json` for seeding bound fields.
    fn value_json(&self, value: &Value) -> Json {
        match value {
            Value::Bool(b) => Json::Bool(*b),
            Value::Number { value, .. } => Json::from(*value),
            Value::String(s) | Value::Enum(s) => Json::String(s.clone()),
            other => Json::String(self.text(other)),
        }
    }

    /// A set style value's inspector seed — `value_json`, except Factor /
    /// Fraction bools become `1`/`0`: those rows render a `NumberInput` that
    /// can't display `true`, and `1`/`0` round-trip through
    /// `style_edit_value` identically (`grow: true` ≡ `grow: 1`).
    fn style_seed_json(&self, name: &str, value: &Value) -> Json {
        use forge_spec::style::{StyleType, key_type};
        match (key_type(name), value) {
            (Some(StyleType::Factor | StyleType::Fraction), Value::Bool(b)) => {
                Json::from(if *b { 1.0 } else { 0.0 })
            }
            _ => self.value_json(value),
        }
    }

    /// Renders `forge.Inspector` — a catalog-driven `Form` of the selected node's props.
    /// Each field binds `document.edits.<prop>`. `prop_changed` writes through to the doc.
    fn inspector(
        &mut self,
        node: &Node,
        target: &Node,
        path: &str,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        // Re-seed `document.edits.*` with the target's prop values whenever the selected
        // node changes — the guard keeps this to one extra paint per selection.
        let seed_path = parse_path("document.edits.__path").expect("static path");
        let seeded = get_path(self.state(), &seed_path)
            .and_then(|j| j.as_str())
            .unwrap_or_default()
            .to_owned();
        // The target lives in the previewed doc — its base comes from
        // `studio.doc_base` (the shell doc is always gpui-kit). Namespace
        // wins when the element names a library explicitly (#156).
        let doc_base = get_path(self.state(), &parse_path("studio.doc_base").unwrap())
            .and_then(|j| j.as_str().map(str::to_owned));
        let props = target
            .element
            .namespace
            .as_deref()
            .map(str::to_owned)
            .or(doc_base)
            .and_then(|lib| self.catalogs.iter().find(|c| c.library == lib))
            .or_else(|| self.catalogs.first())
            .and_then(|c| c.element(&target.element.name))
            .map(|def| def.props.clone())
            .unwrap_or_default();
        if seeded != *path {
            self.update_state(&seed_path, Json::String(path.to_owned()), cx);
            for prop in &props {
                let json = target
                    .props
                    .iter()
                    .find(|(k, _)| k == &prop.name)
                    .map(|(_, v)| self.value_json(v))
                    .unwrap_or(Json::Null);
                if let Ok(edit_path) = parse_path(&format!("document.edits.{}", prop.name)) {
                    self.update_state(&edit_path, json, cx);
                }
            }
            for (name, value) in style_rows(&target.style) {
                let json = value
                    .as_ref()
                    .map(|v| self.style_seed_json(&name, v))
                    .unwrap_or_else(|| unset_style_seed(&name));
                match parse_path(&format!("document.edits.style.{name}")) {
                    Ok(edit_path) => self.update_state(&edit_path, json, cx),
                    Err(e) => self.report(format!("style `{name}` skipped: {e}")),
                }
            }
            if let Ok(p) = parse_path("document.edits.refname") {
                self.update_state(
                    &p,
                    Json::String(target.refname.clone().unwrap_or_default()),
                    cx,
                );
            }
        }
        // First row — `Element - path`, with ` #id`/` &ref` tailing it when
        // the node has them.
        let mut header_text = format!("{} - {}", target.element, path);
        if let Some(id) = &target.id {
            header_text = format!("{header_text} #{id}");
        }
        // Data mode — binding `items`/`styles`/`header` switches the element
        // to a data provider: the doc lays out the fields itself with
        // `for:` + `forge.InspectorField`. Nothing renders here.
        let items_out = self.two_way(node, "items");
        let styles_out = self.two_way(node, "styles");
        let header_out = self.two_way(node, "header");
        if items_out.is_some() || styles_out.is_some() || header_out.is_some() {
            let mut items = vec![serde_json::json!({
                "kind": "ref", "name": "refname", "label": "refname",
                "type": "string",
                "description": "ref  —  `<<: *name` ",
                "value": target.refname.clone().unwrap_or_default()
            })];
            for prop in &props {
                let value = target
                    .props
                    .iter()
                    .find(|(k, _)| k == &prop.name)
                    .map(|(_, v)| self.value_json(v))
                    .unwrap_or(Json::Null);
                items.push(serde_json::json!({
                    "kind": "prop", "name": prop.name, "label": prop.name,
                    "type": prop_type_name(&prop.ty),
                    "options": match &prop.ty { PropType::Enum(v) => v.clone(), _ => vec![] },
                    "description": prop.summary.clone().unwrap_or_default(),
                    "required": prop.required,
                    "value": value,
                }));
            }
            let styles: Vec<Json> = style_rows(&target.style)
                .iter()
                .map(|(name, v)| {
                    let value = v
                        .as_ref()
                        .map(|v| self.style_seed_json(name, v))
                        .unwrap_or(Json::Null);
                    let (ty, options) = style_item_type(name, &value);
                    serde_json::json!({
                        "kind": "style", "name": name, "label": name,
                        "type": ty, "options": options, "value": value,
                    })
                })
                .collect();
            for (out, value) in [
                (items_out, Json::Array(items)),
                (styles_out, Json::Array(styles)),
                (header_out, Json::String(header_text)),
            ] {
                if let Some(p) = out
                    && get_path(self.state(), &p) != Some(&value)
                {
                    self.update_state(&p, value, cx);
                }
            }
            return div().into_any_element();
        }
        let header = div()
            .text_sm()
            .font_weight(FontWeight::MEDIUM)
            .child(header_text);
        // One `FormField` per catalog prop — the control nodes come from the
        // same `inspector_control` helper `forge.InspectorField` dispatches on.
        let mut fields: Vec<Node> = props
            .iter()
            .filter_map(|prop| {
                let control = inspector_control(
                    "prop",
                    &prop.name,
                    prop_type_name(&prop.ty),
                    match &prop.ty {
                        PropType::Enum(v) => v.clone(),
                        _ => vec![],
                    },
                )?;
                Some(Node {
                    element: ElementRef {
                        namespace: None,
                        name: "FormField".into(),
                    },
                    props: vec![("label".to_owned(), Value::String(prop.name.clone()))],
                    children: vec![control],
                    ..Default::default()
                })
            })
            .collect();
        // `ref` block name — editable: `refname_changed` writes `<<: *name`
        // (or a new `name: &name` block) through the preserving writer.
        if let Some(control) = inspector_control("ref", "refname", "string", vec![]) {
            fields.insert(
                0,
                Node {
                    element: ElementRef {
                        namespace: None,
                        name: "FormField".into(),
                    },
                    props: vec![("label".to_owned(), Value::String("refname".into()))],
                    children: vec![control],
                    ..Default::default()
                },
            );
        }
        let prop_children = if fields.is_empty() {
            vec![Node {
                element: ElementRef {
                    namespace: None,
                    name: "Text".into(),
                },
                content: Some(Value::String("no catalog props".into())),
                ..Default::default()
            }]
        } else {
            // `Form` ignores `style:` — the scroll styles go on a wrapping
            // `VStack`, which renders through `container()` → scrollbar.
            vec![Node {
                element: ElementRef {
                    namespace: None,
                    name: "VStack".into(),
                },
                style: inspector_scroll_style(),
                children: vec![Node {
                    element: ElementRef {
                        namespace: None,
                        name: "Form".into(),
                    },
                    props: vec![("label-layout".to_owned(), Value::Enum("Vertical".into()))],
                    children: fields,
                    ..Default::default()
                }],
                ..Default::default()
            }]
        };
        // Style tab — one typed control per `KEYS` entry (set first, then
        // unset), bound `document.edits.style.<name>`. Keyword styles render
        // Selects led by `[none]` (clears the entry but keeps the row).
        let style_children = vec![Node {
            element: ElementRef {
                namespace: None,
                name: "VStack".into(),
            },
            style: inspector_scroll_style(),
            children: vec![Node {
                element: ElementRef {
                    namespace: None,
                    name: "Form".into(),
                },
                props: vec![("label-layout".to_owned(), Value::Enum("Vertical".into()))],
                children: style_rows(&target.style)
                    .iter()
                    .filter_map(|(name, v)| {
                        let value = v.as_ref().map(|v| self.value_json(v)).unwrap_or(Json::Null);
                        let (ty, options) = style_item_type(name, &value);
                        let control = inspector_control("style", name, ty, options)?;
                        Some(Node {
                            element: ElementRef {
                                namespace: None,
                                name: "FormField".into(),
                            },
                            props: vec![("label".to_owned(), Value::String(name.clone()))],
                            children: vec![control],
                            ..Default::default()
                        })
                    })
                    .collect(),
                ..Default::default()
            }],
            ..Default::default()
        }];
        let tabs = Node {
            element: ElementRef {
                namespace: None,
                name: "Tabs".into(),
            },
            children: vec![
                Node {
                    element: ElementRef {
                        namespace: None,
                        name: "Tab".into(),
                    },
                    props: vec![("label".to_owned(), Value::String("Properties".into()))],
                    children: prop_children,
                    ..Default::default()
                },
                Node {
                    element: ElementRef {
                        namespace: None,
                        name: "Tab".into(),
                    },
                    props: vec![("label".to_owned(), Value::String("Style".into()))],
                    children: style_children,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let tabs_key = format!("{key}/{path}");
        let tabs = self.render_node(&tabs, &tabs_key, window, cx);
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(header)
            .child(tabs)
            .into_any_element()
    }

    /// Renders `forge.InspectorField` — the prop editor for one inspector item.
    /// `item:` takes the `{$item}` descriptor from a `for:` over `items:`/`styles:`.
    /// `kind`/`name`/`type`/`options` props override or supply the same fields.
    /// The control binds `document.edits.*` and fires `*_changed` just like the
    /// built-in layout does — wrap it in `FormField` for a label.
    fn inspector_field(
        &mut self,
        node: &Node,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let item = self.prop(node, "item").map(|v| self.resolve(v));
        let item_str = |name: &str| -> Option<String> {
            item.as_ref()
                .and_then(|j| j.get(name))
                .and_then(|j| j.as_str())
                .map(str::to_owned)
        };
        let kind = self
            .prop_text(node, "kind")
            .or_else(|| item_str("kind"))
            .unwrap_or_else(|| "prop".into());
        let ty = self
            .prop_text(node, "type")
            .or_else(|| item_str("type"))
            .unwrap_or_else(|| "string".into());
        let options: Vec<String> = self
            .prop(node, "options")
            .and_then(|v| self.resolve(v).as_array().cloned())
            .or_else(|| {
                item.as_ref()
                    .and_then(|j| j.get("options"))
                    .and_then(|j| j.as_array())
                    .cloned()
            })
            .unwrap_or_default()
            .into_iter()
            .filter_map(|j| j.as_str().map(str::to_owned))
            .collect();
        let Some(name) = self.prop_text(node, "name").or_else(|| item_str("name")) else {
            return placeholder("InspectorField: needs `item` or `name`".to_owned(), cx);
        };
        match inspector_control(&kind, &name, &ty, options) {
            Some(control) => self.render_node(&control, key, window, cx),
            None => placeholder(format!("InspectorField: bad name `{name}`"), cx),
        }
    }

    /// Renders `forge.Outline` — a `Tree` of the document's nodes. Rows carry a click
    /// handler that writes `bind selected` and fires `select` with the node index path.
    fn outline(
        &mut self,
        node: &Node,
        key: &str,
        doc: &Document,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let sel = self.two_way(node, "selected");
        let sel_value = sel
            .as_ref()
            .and_then(|p| self.lookup(p))
            .and_then(|j| j.as_str())
            .map(str::to_owned);
        let skey = format!("{key}|{}", bind_salt(node));
        // Collapse state survives the items rebuild — the tree only flattens
        // `expanded` flags on the items it was last given, so they carry over
        // by id. Without this every state write snapped all folders back open.
        let previous = self
            .trees
            .get(&skey)
            .map(|state| tree_expansion(state.read(cx)))
            .unwrap_or_default();
        let mut items = Vec::new();
        self.outline_data(&doc.root, "0".to_owned(), &mut items);
        carry_expansion(&mut items, &previous);
        let state = match self.trees.get(&skey) {
            Some(state) => {
                state.update(cx, |state, cx| {
                    *state = std::mem::replace(state, TreeState::new(cx)).items(items.clone());
                });
                state.clone()
            }
            None => {
                let state = cx.new(|cx| TreeState::new(cx).items(items));
                self.trees.insert(skey.clone(), state.clone());
                state
            }
        };
        // `render_item` runs several times per row per paint (measure then paint), so the
        // row is rebuilt inside the closure — nothing is handed out once and consumed.
        let weak = cx.entity().downgrade();
        let drop_weak = weak.clone();
        let outline = node.clone();
        Tree::new(&state, move |ix, entry, selected, _, _cx| {
            let mut item = ListItem::new(ix);
            let id = entry.item().id.to_string();
            let sel = sel.clone();
            let outline = outline.clone();
            let weak = weak.clone();
            let drag_id = id.clone();
            let drop_id = id.clone();
            let drop_weak = drop_weak.clone();
            let ghost: SharedString = entry.item().label.to_string().into();
            let active = sel_value.as_deref() == Some(entry.item().id.as_str());
            let row = div()
                .id(format!("outline-row-{}", entry.item().id))
                .flex()
                .flex_row()
                .items_center()
                .gap_1p5()
                .pl(px(entry.depth() as f32 * 14.))
                .child(if entry.is_folder() {
                    icon(if entry.is_expanded() {
                        "chevron-down"
                    } else {
                        "chevron-right"
                    })
                    .with_size(size("XSmall"))
                    .into_any_element()
                } else {
                    // Leaf spacer aligns the label with folder labels.
                    div().w(px(12.)).into_any_element()
                })
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    try_update(&weak, cx, "event", |this, cx| {
                        if let Some(path) = &sel {
                            this.write(path, Json::String(id.clone()), cx);
                        }
                        this.fire(&outline, "select", cx);
                    })
                    .ok();
                })
                .on_drag(ForgeDrag::Node(drag_id), move |_, _, _, cx| {
                    cx.new(|_| DragGhost(ghost.clone()))
                })
                .on_drop::<ForgeDrag>(move |drag, _, cx| {
                    try_update(&drop_weak, cx, "drop", |p, cx| {
                        p.write(
                            &parse_path("studio.drop_target").unwrap(),
                            Json::String(drop_id.clone()),
                            cx,
                        );
                        match drag {
                            ForgeDrag::Component(tag) => p.fire_call(
                                "drop_component",
                                vec![tag.clone(), drop_id.clone()],
                                cx,
                            ),
                            ForgeDrag::Node(src) => {
                                p.fire_call("move_node", vec![src.clone(), drop_id.clone()], cx)
                            }
                        }
                        cx.notify();
                    })
                    .ok();
                })
                .child(entry.item().label.to_string());
            item = item.child(row);
            item.selected(selected || active)
        })
        .into_any_element()
    }

    /// One `TreeItem` per document node, recursing through `node.children`. The row is
    /// rebuilt per paint from `entry.item()` — id is the node's index path.
    fn outline_data(&mut self, node: &Node, path: String, items: &mut Vec<TreeItem>) {
        let mut label = node.element.name.clone();
        if let Some(id) = &node.id {
            label = format!("{label} #{id}");
        }
        if let Some(content) = &node.content {
            let text = self.text(content);
            if !text.is_empty() {
                label = format!("{label} · {text}");
            }
        }
        let mut data = TreeItem::new(&path, &label).expanded(true);
        let mut subs = Vec::new();
        for (i, child) in node.children.iter().enumerate() {
            self.outline_data(child, format!("{path}.{i}"), &mut subs);
        }
        // `else:`/`case:` branch children — `e`/`c` path steps (`node_at`).
        match &node.control {
            Some(Control::Match {
                cases, otherwise, ..
            }) => {
                for (c, case) in cases.iter().enumerate() {
                    for (i, child) in case.children.iter().enumerate() {
                        self.outline_data(child, format!("{path}.c{c}.{i}"), &mut subs);
                    }
                }
                for (i, child) in otherwise.iter().enumerate() {
                    self.outline_data(child, format!("{path}.e{i}"), &mut subs);
                }
            }
            Some(control) if !control.otherwise().is_empty() => {
                for (i, child) in control.otherwise().iter().enumerate() {
                    self.outline_data(child, format!("{path}.e{i}"), &mut subs);
                }
            }
            _ => {}
        }
        if !subs.is_empty() {
            data = data.children(subs);
        }
        items.push(data);
    }

    /// Builds `TreeItem` data recursively. Each node's row content is pre-rendered into
    /// `map` under its id for the `render_item` closure to hand out per paint.
    fn tree_data(&mut self, node: &Node, key: &str, nodes: &mut HashMap<String, Node>) -> TreeItem {
        let id = node.id.clone().unwrap_or_else(|| key.to_owned());
        let label = self.prop_text(node, "label").unwrap_or_default();
        let mut data = TreeItem::new(&id, &label);
        if self.prop_bool(node, "expanded") {
            data = data.expanded(true);
        }
        nodes.insert(id.clone(), node.clone());
        let mut subs = Vec::new();
        for (i, child) in node.children.iter().enumerate() {
            if child.element.name.as_str() == "TreeItem" {
                subs.push(self.tree_data(child, &format!("{key}/{i}"), nodes));
            }
        }
        if !subs.is_empty() {
            data = data.children(subs);
        }
        data
    }

    /// Builds a `SidebarMenuItem` — recursively nests `SidebarMenuItem` children.
    #[allow(clippy::only_used_in_recursion)]
    fn sidebar_item(
        &mut self,
        node: &Node,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> SidebarMenuItem {
        let kids: Vec<SidebarMenuItem> = node
            .children
            .iter()
            .enumerate()
            .filter(|(_, c)| c.element.name.as_str() == "SidebarMenuItem")
            .map(|(i, c)| self.sidebar_item(c, &format!("{key}/{i}"), window, cx))
            .collect();
        let mut item = SidebarMenuItem::new(self.prop_text(node, "label").unwrap_or_default())
            .active(self.prop_bool(node, "active"))
            .disable(self.prop_bool(node, "disabled"))
            .default_open(self.prop_bool(node, "open"));
        if let Some(name) = self.prop_text(node, "icon") {
            item = item.icon(self.doc_icon(&name, cx));
        }
        if !kids.is_empty() {
            item = item.children(kids);
        }
        let weak = cx.entity().downgrade();
        let clicked = node.clone();
        item.on_click(move |_, _, cx| {
            try_update(&weak, cx, "event", |this, cx| {
                this.fire(&clicked, "click", cx)
            })
            .ok();
        })
    }

    /// Builds a `Field` from a `FormField` node — used by `Form` and standalone fields.
    fn form_field(
        &mut self,
        node: &Node,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Field {
        let mut kids: Vec<AnyElement> = Vec::with_capacity(node.children.len());
        for (i, child) in node.children.iter().enumerate() {
            kids.extend(self.expand(child, &format!("{key}/{i}"), window, cx));
        }
        let mut field = Field::new().children(kids);
        if let Some(label) = self.prop_text(node, "label") {
            field = field.label(label);
        }
        if let Some(description) = self.prop_text(node, "description") {
            field = field.description(description);
        }
        field = field
            .required(self.prop_bool(node, "required"))
            .visible(!matches!(
                self.prop(node, "visible"),
                Some(Value::Bool(false))
            ))
            // `Field`'s own default is `true` — only an explicit prop turns the
            // label area off. `prop_bool` returns `false` for absent props,
            // which used to suppress every label.
            .label_indent(
                self.prop(node, "label-indent")
                    .is_none_or(|v| self.truthy(v)),
            );
        match self.prop_text(node, "align").as_deref() {
            Some("center") => field = field.items_center(),
            Some("end") => field = field.items_end(),
            _ => {}
        }
        if let Some(n) = self.prop(node, "col-span").and_then(|v| self.number(v)) {
            field = field.col_span(n as u16);
        }
        if let Some(n) = self.prop(node, "col-start").and_then(|v| self.number(v)) {
            field = field.col_start(n as i16);
        }
        field
    }

    fn prop_text(&self, node: &Node, name: &str) -> Option<String> {
        self.prop(node, name).map(|v| self.text(v))
    }

    fn prop_bool(&self, node: &Node, name: &str) -> bool {
        self.prop(node, name).is_some_and(|v| self.truthy(v))
    }

    fn two_way(&self, node: &Node, name: &str) -> Option<Path> {
        match self.prop(node, name)? {
            Value::Binding {
                path,
                mode: BindingMode::TwoWay,
            } => Some(path.clone()),
            _ => None,
        }
    }

    /// Shared `Input`/`InputGroupInput` backing state — placeholder+masked key
    /// the cache, `value:` is the initial default, `value: bind` subscribes for
    /// change/submit events and the two-way write-back.
    fn input_state(
        &mut self,
        node: &Node,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        let placeholder = self.prop_text(node, "placeholder").unwrap_or_default();
        let masked = self.prop_bool(node, "masked");
        let path = self.two_way(node, "value");
        // The bound path rides the cache key. A reordered row (inspector
        // set-first ordering, `for:` shifts) reusing this slot must not
        // inherit an entity whose subscription still writes the old path.
        let state_key = format!(
            "{key}|{placeholder}|{masked}|{}",
            path.as_ref().map(ToString::to_string).unwrap_or_default()
        );
        let value = self.prop_text(node, "value").unwrap_or_default();
        if let Some(state) = self.inputs.get(&state_key) {
            return state.clone();
        }
        let state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(placeholder)
                .masked(masked)
                .default_value(value.clone())
        });
        let changed = node.clone();
        let subscription = cx.subscribe(&state, move |this, state, event, cx| match event {
            InputEvent::Change => {
                if let Some(path) = &path {
                    let value = state.read(cx).value().to_string();
                    this.write(path, Json::String(value), cx);
                }
                this.fire(&changed, "change", cx);
            }
            InputEvent::PressEnter { .. } => this.fire(&changed, "submit", cx),
            _ => {}
        });
        self.subscriptions.push(subscription);
        self.inputs.insert(state_key, state.clone());
        state
    }

    /// Same deal as `input_state`, for `Textarea`/`InputGroupTextarea`.
    fn textarea_state(
        &mut self,
        node: &Node,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<TextareaState> {
        let placeholder = self.prop_text(node, "placeholder").unwrap_or_default();
        let path = self.two_way(node, "value");
        let state_key = format!(
            "{key}|{placeholder}|{}",
            path.as_ref().map(ToString::to_string).unwrap_or_default()
        );
        let value = self.prop_text(node, "value").unwrap_or_default();
        if let Some(state) = self.textareas.get(&state_key) {
            return state.clone();
        }
        let rows = self.prop(node, "rows").and_then(|v| self.number(v));
        let state = cx.new(|cx| {
            let state = TextareaState::new(window, cx)
                .placeholder(placeholder)
                .default_value(value.clone());
            match rows {
                Some(rows) => state.rows(rows as usize),
                None => state,
            }
        });
        let changed = node.clone();
        let subscription = cx.subscribe(&state, move |this, state, event, cx| {
            if let InputEvent::Change = event {
                if let Some(path) = &path {
                    let value = state.read(cx).value().to_string();
                    this.write(path, Json::String(value), cx);
                }
                this.fire(&changed, "change", cx);
            }
        });
        self.subscriptions.push(subscription);
        self.textareas.insert(state_key, state.clone());
        state
    }

    /// `{key: icon-name}` override table — `icon-map:` accepts a style map
    /// inline or a `{$path}` to a JSON object.
    fn icon_map(&self, value: &Value) -> Vec<(String, String)> {
        match value {
            Value::Map(entries) => entries
                .iter()
                .map(|(k, v)| (k.clone(), self.text(v)))
                .collect(),
            Value::Binding { path, .. } => self
                .lookup(path)
                .and_then(Json::as_object)
                .map(|m| {
                    m.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_owned())))
                        .collect()
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    /// Render one document node — public so `ForgeLibrary` adapters can defer
    /// child content into their own proxy entities (re-entering through
    /// `Entity::update` inside this render pass would double-borrow).
    pub fn render_node(
        &mut self,
        node: &Node,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        // Control blocks expand to zero or more elements — child lists route
        // through `expand` so this only guards stray single-node calls.
        if node.control.is_some() {
            let kids = self.expand(node, key, window, cx);
            return div().children(kids).into_any_element();
        }
        if let Some(namespace) = &node.element.namespace {
            let library = self
                .doc
                .as_ref()
                .and_then(|doc| doc.imports.iter().find(|i| &i.alias == namespace))
                .map(|import| import.library.as_str());
            for lib in &self.libraries {
                if lib.alias() == namespace
                    && let Some(el) = lib.render(node, key, self, window, cx)
                {
                    // Adapter events — `on click:` on library elements wraps the
                    // widget in an interactive div that fires through the hook
                    // path. Adapters get events without the render signature
                    // carrying mutable view state.
                    if node.events.iter().any(|e| e.event == "click") {
                        let weak = cx.entity().downgrade();
                        let node = node.clone();
                        return div()
                            .id(format!("lib-{key}"))
                            .child(el)
                            .on_click(move |_, _, cx| {
                                try_update(&weak, cx, "event", |this, cx| {
                                    this.fire(&node, "click", cx)
                                })
                                .ok();
                            })
                            .into_any_element();
                    }
                    return el;
                }
            }
            let element: AnyElement = match (library, node.element.name.as_str()) {
                (Some("forge"), "Preview") => {
                    let id = node.id.clone().unwrap_or_else(|| key.to_owned());
                    let remote = self.prop_text(node, "remote").filter(|t| !t.is_empty());
                    // `compat: true` (#145) — the pane renders the document
                    // through the gpui-kit analogue map even while a remote
                    // host is live: mapped widgets give structure, selection
                    // and DnD. The host keeps running separately for fidelity.
                    let compat = self.prop_text(node, "compat").is_some_and(|v| v == "true");
                    // `remote:` non-empty — the document previews in an
                    // out-of-process host (#97). The element is a framed card
                    // instead of a nested view, and `on click:` fires through
                    // the hook path (Studio uses it to raise the host window).
                    if let Some(remote) = remote.clone().filter(|_| !compat) {
                        let weak = cx.entity().downgrade();
                        let node = node.clone();
                        let theme = cx.theme();
                        let embedded = self
                            .prop_text(&node, "mode")
                            .is_some_and(|m| m == "embedded");
                        let placeholder =
                            self.prop_text(&node, "placeholder").unwrap_or_else(|| {
                                "rendered by the preview host — click to focus".into()
                            });
                        // Track the card's bounds under the pane `id` — Studio
                        // polls `preview_bounds(id)` to `frame_rect` each pane's
                        // host window over its slot.
                        let cell = self
                            .preview_bounds
                            .entry(id.clone())
                            .or_insert_with(|| std::rc::Rc::new(std::cell::RefCell::new(None)))
                            .clone();
                        // A frame-streaming host's ANSI bytes land in state as
                        // `preview.panes.<id>.frame` ({seq, cols, rows, data}).
                        // The card feeds new chunks through the pane's terminal
                        // emulator and paints the styled grid — native-only.
                        // alacritty can't build on wasm, where the web canvas
                        // renders ratatui through its own path.
                        #[cfg(not(target_arch = "wasm32"))]
                        let term_card = get_path(self.state(), &pane_state_path(&id, "frame"))
                            .and_then(|j| {
                                Some((
                                    j["seq"].as_u64()?,
                                    j["data"].as_str()?.to_owned(),
                                    j["cols"].as_u64().unwrap_or(80) as u16,
                                    j["rows"].as_u64().unwrap_or(24) as u16,
                                ))
                            })
                            .map(|(seq, _data, cols, rows)| {
                                // Feeding lives in `feed_term_frame` on the
                                // event path — see its comment. Render only
                                // attaches the already-fed emulator.
                                let entry =
                                    self.term_panes.entry(id.clone()).or_insert_with(|| {
                                        (
                                            seq,
                                            std::rc::Rc::new(std::cell::RefCell::new(
                                                forge_term::Emulator::new(cols, rows),
                                            )),
                                        )
                                    });
                                // Track the grid's painted rect so pointer
                                // events map to (col, row) cells.
                                let tcell = self
                                    .term_bounds
                                    .entry(id.clone())
                                    .or_insert_with(|| {
                                        std::rc::Rc::new(std::cell::RefCell::new(None))
                                    })
                                    .clone();
                                let mcell = self
                                    .term_metrics
                                    .entry(id.clone())
                                    .or_insert_with(|| {
                                        std::rc::Rc::new(std::cell::RefCell::new(None))
                                    })
                                    .clone();
                                (entry.1.clone(), tcell, mcell)
                            })
                            .map(|(term, tcell, mcell)| {
                                div()
                                    .size_full()
                                    .p_2()
                                    .overflow_hidden()
                                    .bg(theme.background)
                                    .child(BoundsTracker::new(
                                        forge_term::TermView::new(
                                            term,
                                            forge_term::TermStyle {
                                                font_family: theme.mono_font_family.clone(),
                                                font_size: px(12.),
                                                default_fg: theme.foreground,
                                                default_bg: theme.background,
                                                cursor: theme.foreground,
                                            },
                                        )
                                        .metrics_sink(mcell)
                                        .into_any_element(),
                                        tcell,
                                    ))
                                    .into_any_element()
                            });
                        #[cfg(target_arch = "wasm32")]
                        let term_card: Option<AnyElement> = None;
                        let streaming = term_card.is_some();
                        let focus = self
                            .term_focus
                            .entry(id.clone())
                            .or_insert_with(|| cx.focus_handle())
                            .clone();
                        // DnD edits the document the pane mirrors — only a
                        // `doc:`-less stream pane previews studio.doc (#156).
                        let editable = self.prop_text(&node, "doc").is_none_or(|d| d.is_empty());
                        let weak_k = weak.clone();
                        let weak_d = weak.clone();
                        let weak_u = weak.clone();
                        let weak_s = weak.clone();
                        let weak_mv = weak.clone();
                        let weak_dm = weak.clone();
                        let weak_dp = weak.clone();
                        let focus_d = focus.clone();
                        let id_k = id.clone();
                        let id_d = id.clone();
                        let id_u = id.clone();
                        let id_s = id.clone();
                        let id_mv = id.clone();
                        let id_dm = id.clone();
                        let id_dp = id.clone();
                        BoundsTracker::new(
                            div()
                                .id(format!("preview-remote-{key}"))
                                .size_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .track_focus(&focus)
                                // Stream cards forward input to their ratatui
                                // host: `pane_key`/`pane_mouse` hooks carry it
                                // over `pane_send`. The doc's `on click:` still
                                // fires (Studio raises the host window).
                                .on_key_down(move |ev: &KeyDownEvent, _window, cx| {
                                    if !streaming {
                                        return;
                                    }
                                    let Some(key) = pane_key_name(&ev.keystroke) else {
                                        return;
                                    };
                                    try_update(&weak_k, cx, "event", |this, cx| {
                                        this.fire_call("pane_key", vec![id_k.clone(), key], cx)
                                    })
                                    .ok();
                                })
                                .on_mouse_down(MouseButton::Left, move |ev, window, cx| {
                                    // Focus first so the card's keys reach
                                    // `on_key_down` from this click on.
                                    focus_d.focus(window, cx);
                                    if !streaming {
                                        return;
                                    }
                                    try_update(&weak_d, cx, "event", |this, cx| {
                                        this.term_drag.insert(id_d.clone(), false);
                                        send_pane_mouse(
                                            this,
                                            &id_d,
                                            "down",
                                            ev.position,
                                            window,
                                            cx,
                                        )
                                    })
                                    .ok();
                                })
                                .on_mouse_up(MouseButton::Left, move |ev, window, cx| {
                                    if !streaming {
                                        return;
                                    }
                                    try_update(&weak_u, cx, "event", |this, cx| {
                                        if editable
                                            && this.term_drag.get(&id_u).copied().unwrap_or(false)
                                        {
                                            // Drag inside the card — a node
                                            // move. `pane_move` resolves the
                                            // source (`preview.panes.*.zone`)
                                            // and the drop cell.
                                            this.term_drag.insert(id_u.clone(), false);
                                            let (col, row) = this
                                                .term_hover
                                                .get(&id_u)
                                                .copied()
                                                .unwrap_or_default();
                                            this.fire_call(
                                                "pane_move",
                                                vec![
                                                    id_u.clone(),
                                                    col.to_string(),
                                                    row.to_string(),
                                                ],
                                                cx,
                                            );
                                        } else {
                                            send_pane_mouse(
                                                this,
                                                &id_u,
                                                "up",
                                                ev.position,
                                                window,
                                                cx,
                                            )
                                        }
                                    })
                                    .ok();
                                })
                                .on_mouse_move(move |ev: &MouseMoveEvent, window, cx| {
                                    if !streaming || !editable || !ev.dragging() {
                                        return;
                                    }
                                    // Dragging inside the card — mark the
                                    // move-drag and hover the drop zone.
                                    try_update(&weak_mv, cx, "event", |this, cx| {
                                        this.term_drag.insert(id_mv.clone(), true);
                                        send_pane_mouse(
                                            this,
                                            &id_mv,
                                            "drop-hover",
                                            ev.position,
                                            window,
                                            cx,
                                        )
                                    })
                                    .ok();
                                })
                                // Palette/doc drags over the card — the host
                                // resolves cells to doc paths (`zone`,
                                // `drop_target`, `drop` events). The payload
                                // stays here. `pane_drop` pairs it with the
                                // resolved path at commit time (#156).
                                .on_drag_move::<ForgeDrag>(move |ev, window, cx| {
                                    if !streaming || !editable {
                                        return;
                                    }
                                    try_update(&weak_dm, cx, "event", |this, cx| {
                                        send_pane_mouse(
                                            this,
                                            &id_dm,
                                            "drop-hover",
                                            ev.event.position,
                                            window,
                                            cx,
                                        )
                                    })
                                    .ok();
                                })
                                .on_drop::<ForgeDrag>(move |drag, _, cx| {
                                    if !streaming || !editable {
                                        return;
                                    }
                                    try_update(&weak_dp, cx, "drop", |this, cx| {
                                        let (kind, payload) = match drag {
                                            ForgeDrag::Component(t) => ("component", t.clone()),
                                            ForgeDrag::Node(p) => ("node", p.clone()),
                                        };
                                        let (col, row) = this
                                            .term_hover
                                            .get(&id_dp)
                                            .copied()
                                            .unwrap_or_default();
                                        this.fire_call(
                                            "pane_drop",
                                            vec![
                                                id_dp.clone(),
                                                kind.to_string(),
                                                payload,
                                                col.to_string(),
                                                row.to_string(),
                                            ],
                                            cx,
                                        );
                                    })
                                    .ok();
                                })
                                .on_scroll(move |ev: &ScrollEvent, window, cx| {
                                    if !streaming {
                                        return;
                                    }
                                    let kind = if f32::from(ev.delta.pixel_delta(px(20.)).y) < 0. {
                                        "scroll-down"
                                    } else {
                                        "scroll-up"
                                    };
                                    try_update(&weak_s, cx, "event", |this, cx| {
                                        send_pane_mouse(this, &id_s, kind, ev.position, window, cx)
                                    })
                                    .ok();
                                })
                                .on_click(move |_, _, cx| {
                                    try_update(&weak, cx, "event", |this, cx| {
                                        this.fire(&node, "click", cx)
                                    })
                                    .ok();
                                })
                                .child(if embedded {
                                    // #120 — `mode: embedded` — the host window
                                    // floats over this slot via `frame_rect`. The
                                    // content area is transparent so the foreign
                                    // pixels bleed through, chrome stays for context.
                                    div()
                                        .size_full()
                                        .border_1()
                                        .border_color(theme.border)
                                        .into_any_element()
                                } else if let Some(card) = term_card {
                                    // Streaming host (ratatui) — the card IS
                                    // the terminal: alacritty-emulated cells,
                                    // mono font, real colors.
                                    card
                                } else {
                                    div()
                                        .flex()
                                        .flex_col()
                                        .items_center()
                                        .gap_2()
                                        .child(icon("app-window"))
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(theme.foreground)
                                                .child(remote),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(theme.muted_foreground)
                                                .child(placeholder),
                                        )
                                        .into_any_element()
                                })
                                .into_any_element(),
                            cell,
                        )
                        .into_any_element()
                    } else {
                        let empty_text = self.prop_text(node, "placeholder");
                        let doc_spec = self.prop_text(node, "doc").filter(|t| !t.is_empty());
                        let catalogs = self.catalogs.clone();
                        let libraries = self.libraries.clone();
                        let preview = self
                            .previews
                            .entry(id.clone())
                            .or_insert_with(|| {
                                cx.new(|_| {
                                    let mut view = ForgeView::new(catalogs);
                                    view.libraries = libraries;
                                    view.empty_text = empty_text;
                                    view
                                })
                            })
                            .clone();
                        // `doc:` — a doc-relative `.forge.yaml` path, or a named
                        // ref (`name: &name`) whose head-less body becomes the
                        // preview's root. Loaded once per src string.
                        if let Some(spec) =
                            doc_spec.filter(|s| self.preview_srcs.get(&id) != Some(s))
                        {
                            let loaded = if let Some(body) = self.doc.as_ref().and_then(|d| {
                                d.refs
                                    .iter()
                                    .find(|(n, _)| n == &spec)
                                    .map(|(_, b)| b.clone())
                            }) {
                                let mut doc = self.doc.clone().unwrap();
                                doc.root = body;
                                Some(doc)
                            } else {
                                let path = self.resolve_path(&spec);
                                std::fs::read_to_string(&path)
                                    .ok()
                                    .and_then(|src| yaml::from_str(&src).ok())
                            };
                            if let Some(doc) = loaded {
                                preview.update(cx, |view, cx| view.set_document(doc, cx));
                                self.preview_srcs.insert(id.clone(), spec);
                            }
                        }
                        // A view element sizes to content in a flex column.
                        // The `size_full` wrapper gives the nested view a
                        // definite slot. The doc root's `grow` has space to
                        // claim.
                        let element = BoundsLogger::new(
                            div()
                                .id("preview-fill")
                                .size_full()
                                .child(
                                    preview
                                        .clone()
                                        .cached(
                                            gpui::StyleRefinement::default()
                                                .flex_1()
                                                .min_h_0()
                                                .w_full(),
                                        )
                                        .into_any_element(),
                                )
                                .into_any_element(),
                            "preview-fill",
                        )
                        .into_any_element();
                        // #156 — a ratatui-base doc draws in-process instead
                        // of through the compat map: same renderer as the
                        // preview host, no wire, direct hit-testing.
                        #[cfg(feature = "ratatui-embed")]
                        let element = match self.tui_card(node, &id, &preview, cx) {
                            Some(card) => card,
                            None => element,
                        };
                        if remote.is_some() {
                            // Compat canvas — pane bounds stay tracked so an
                            // `embedded` host still anchors over the slot.
                            let cell = self
                                .preview_bounds
                                .entry(id)
                                .or_insert_with(|| std::rc::Rc::new(std::cell::RefCell::new(None)))
                                .clone();
                            BoundsTracker::new(element, cell).into_any_element()
                        } else {
                            element
                        }
                    }
                }
                (Some("forge"), "Outline") => {
                    let of = self.prop_text(node, "of").unwrap_or_default();
                    let empty_text = self
                        .prop_text(node, "placeholder")
                        .unwrap_or_else(|| "no document".into());
                    let doc = self
                        .previews
                        .get(&of)
                        .and_then(|preview| preview.read(cx).doc.clone());
                    match doc {
                        Some(doc) => self.outline(node, key, &doc, cx),
                        None => placeholder(empty_text, cx),
                    }
                }
                (Some("forge"), "Inspector") => {
                    let of = self.prop_text(node, "of").unwrap_or_default();
                    let path = self.prop_text(node, "selected").unwrap_or_default();
                    let empty_text = self
                        .prop_text(node, "placeholder")
                        .unwrap_or_else(|| "nothing selected".into());
                    let target = self
                        .previews
                        .get(&of)
                        .and_then(|preview| preview.read(cx).doc.clone())
                        .and_then(|doc| doc.node_at(&path).cloned());
                    match target {
                        Some(target) => self.inspector(node, &target, &path, key, window, cx),
                        None => {
                            // Data mode still publishes (emptied) outputs so
                            // `for:` loops over them collapse cleanly.
                            for (out, value) in [
                                (self.two_way(node, "items"), Json::Array(vec![])),
                                (self.two_way(node, "styles"), Json::Array(vec![])),
                                (self.two_way(node, "header"), Json::String(String::new())),
                            ] {
                                if let Some(p) = out
                                    && get_path(self.state(), &p) != Some(&value)
                                {
                                    self.update_state(&p, value, cx);
                                }
                            }
                            placeholder(empty_text, cx)
                        }
                    }
                }
                (Some("forge"), "InspectorField") => self.inspector_field(node, key, window, cx),
                (Some("forge"), "Palette") => self.palette(node, key, window, cx),
                (Some("forge"), "Icons") => self.icons(node, key, window, cx),
                (Some("forge"), "Files") => self.files(node, key, window, cx),
                (Some("forge"), "Showcase") => self.showcase(node, key, window, cx),
                (Some("forge"), "Vars") => self.vars(node, key, window, cx),
                _ => {
                    // #145 — compat canvas: a namespaced element no adapter or
                    // intrinsic provides renders through the gpui-kit analogue
                    // its catalog declares via `canvas:`. The mapped node keeps
                    // its path/id, so selection, outline and DnD edits land on
                    // the real document node. Unmapped names get the box.
                    let analogue = library
                        .or(Some(namespace.as_str()))
                        .and_then(|lib| self.catalogs.iter().find(|c| c.library == lib))
                        .and_then(|cat| cat.element(&node.element.name))
                        .and_then(|def| def.canvas.clone());
                    match analogue {
                        Some(map) => {
                            let mut mapped = node.clone();
                            mapped.element.name = map.element;
                            mapped.element.namespace = None;
                            for (k, _) in mapped.props.iter_mut() {
                                if let Some(to) = map.props.get(k.as_str()) {
                                    *k = to.clone();
                                }
                            }
                            return self.render_node(&mapped, key, window, cx);
                        }
                        None => placeholder(
                            format!(
                                "`{namespace}.{}` is not provided by this adapter yet",
                                node.element.name
                            ),
                            cx,
                        ),
                    }
                }
            };
            return wrap(
                element,
                &node.style,
                &format!("forge-{key}"),
                self.prop_text(node, "tooltip"),
                &mut self.scroll_handles,
                cx,
            );
        }
        let id = SharedString::from(format!("forge-{key}"));
        let weak = cx.entity().downgrade();
        let children = |this: &mut Self, window: &mut Window, cx: &mut Context<Self>| {
            let mut out = Vec::with_capacity(node.children.len());
            for (i, child) in node.children.iter().enumerate() {
                out.extend(this.expand(child, &format!("{key}.{i}"), window, cx));
            }
            out
        };
        // #145 — compat canvas: a same-named foreign element (a `Badge` in a
        // guise doc) reaches the kit dispatch before the fallback lookup, so
        // the document base's `canvas:` map applies here — retargets and prop
        // renames included. Identity self-maps are skipped. Chained maps
        // resolve for a few hops, and the mapped clone keeps path/id/children.
        let mut mapped: Option<Node> = None;
        if let Some(cat) = self
            .doc
            .as_ref()
            .and_then(|d| d.base.as_deref())
            .and_then(|base| self.catalogs.iter().find(|c| c.library == base))
        {
            for _ in 0..4 {
                let name = mapped
                    .as_ref()
                    .map_or(node.element.name.as_str(), |m| m.element.name.as_str());
                let Some(map) = cat.element(name).and_then(|d| d.canvas.as_ref()) else {
                    break;
                };
                if map.element == name && map.props.is_empty() {
                    break;
                }
                let m = mapped.get_or_insert_with(|| node.clone());
                for (k, _) in m.props.iter_mut() {
                    if let Some(to) = map.props.get(k.as_str()) {
                        *k = to.clone();
                    }
                }
                if map.element == m.element.name {
                    break;
                }
                m.element.name = map.element.clone();
            }
        }
        let node = mapped.as_ref().unwrap_or(node);
        let mut styled = false;
        let element: AnyElement = match node.element.name.as_str() {
            "Box" => {
                let kids = children(self, window, cx);
                styled = true;
                container(div(), &node.style, id, kids, &mut self.scroll_handles, cx)
            }
            "VStack" => {
                let kids = children(self, window, cx);
                styled = true;
                container(
                    v_flex(),
                    &node.style,
                    id,
                    kids,
                    &mut self.scroll_handles,
                    cx,
                )
            }
            // `h_flex()` centers children on the cross axis. A Forge `HStack` is a plain
            // flex row, so children stretch (use `align: center` to center them).
            "HStack" => {
                let kids = children(self, window, cx);
                styled = true;
                container(
                    div().flex().flex_row(),
                    &node.style,
                    id,
                    kids,
                    &mut self.scroll_handles,
                    cx,
                )
            }
            "Text" => {
                let text = self.prop_text(node, "text").unwrap_or_default();
                styled = true;
                style(div(), &node.style, cx).child(text).into_any_element()
            }
            "Label" => Label::new(self.prop_text(node, "text").unwrap_or_default())
                .when_some(self.prop_text(node, "secondary"), |l, s| l.secondary(s))
                .masked(self.prop_bool(node, "masked"))
                .into_any_element(),
            "Button" => {
                let clicked = node.clone();
                Button::new(id)
                    .label(self.prop_text(node, "label").unwrap_or_default())
                    .with_variant(button_variant(
                        &self.prop_text(node, "variant").unwrap_or_default(),
                    ))
                    .with_size(size(&self.prop_text(node, "size").unwrap_or_default()))
                    .when_some(self.prop_text(node, "icon"), |b, name| {
                        b.icon(self.doc_icon(&name, cx))
                    })
                    .when_some(self.prop_text(node, "tooltip"), |b, t| b.tooltip(t))
                    .disabled(self.prop_bool(node, "disabled"))
                    .loading(self.prop_bool(node, "loading"))
                    .when(self.prop_bool(node, "outline"), |b| b.outline())
                    .when(self.prop_bool(node, "compact"), |b| b.compact())
                    .on_click(move |_, _, cx| {
                        try_update(&weak, cx, "event", |this, cx| {
                            this.fire(&clicked, "click", cx)
                        })
                        .ok();
                    })
                    .into_any_element()
            }
            "DropdownMenu" => {
                // The popover builder runs inside a `Context<PopupMenu>` at open time,
                // so `MenuItem` props are baked here and clicks go back through `weak`.
                let weak2 = weak.clone();
                let mut button = Button::new(SharedString::from(format!("{id}-menu")))
                    .label(self.prop_text(node, "label").unwrap_or_default())
                    .with_variant(button_variant(
                        &self.prop_text(node, "variant").unwrap_or_default(),
                    ))
                    .with_size(size(&self.prop_text(node, "size").unwrap_or_default()))
                    .disabled(self.prop_bool(node, "disabled"));
                if let Some(name) = self.prop_text(node, "icon") {
                    button = button.icon(self.doc_icon(&name, cx));
                }
                let items: Vec<(String, Option<String>, bool, bool, Node)> = node
                    .children
                    .iter()
                    .map(|c| {
                        let label = self
                            .prop_text(c, "label")
                            .or_else(|| c.content.as_ref().map(|v| self.text(v)))
                            .unwrap_or_default();
                        (
                            label,
                            self.prop_text(c, "icon"),
                            self.prop_bool(c, "disabled"),
                            self.prop_bool(c, "checked"),
                            c.clone(),
                        )
                    })
                    .collect();
                let iconset = self.iconset().to_owned();
                button
                    .dropdown_menu(move |menu, _, cx| {
                        let mut menu = menu;
                        for (label, icon_name, disabled, checked, child) in &items {
                            if child.element.name.as_str() == "MenuSeparator" {
                                menu = menu.separator();
                                continue;
                            }
                            let mut item = PopupMenuItem::new(label.clone());
                            if let Some(name) = icon_name {
                                item = item.icon(doc_icon_for(&iconset, name, cx));
                            }
                            item = item.disabled(*disabled).checked(*checked);
                            let weak = weak2.clone();
                            let child = child.clone();
                            item = item.on_click(move |_, _, cx| {
                                try_update(&weak, cx, "event", |this, cx| {
                                    this.fire(&child, "click", cx)
                                })
                                .ok();
                            });
                            menu = menu.item(item);
                        }
                        menu
                    })
                    .into_any_element()
            }
            "Input" => {
                let value = self.prop_text(node, "value").unwrap_or_default();
                let state = self.input_state(node, key, window, cx);
                // `value: bind` is controlled — external writes (theme pickers,
                // doc loads) push into the cached state. `set_value` emits no
                // events. A static `value:` stays a one-time default.
                if matches!(self.prop(node, "value"), Some(Value::Binding { .. }))
                    && state.read(cx).value() != value
                {
                    state.update(cx, |s, cx| s.set_value(value, window, cx));
                }
                Input::new(&state)
                    .cleanable(self.prop_bool(node, "cleanable"))
                    .disabled(self.prop_bool(node, "disabled"))
                    .into_any_element()
            }
            "Textarea" => {
                let value = self.prop_text(node, "value").unwrap_or_default();
                let state = self.textarea_state(node, key, window, cx);
                // Same controlled-bind sync as `Input` — see that arm.
                if matches!(self.prop(node, "value"), Some(Value::Binding { .. }))
                    && state.read(cx).value() != value
                {
                    state.update(cx, |s, cx| s.set_value(value, window, cx));
                }
                let textarea = Textarea::new(&state)
                    .disabled(self.prop_bool(node, "disabled"))
                    .readonly(self.prop_bool(node, "readonly"));
                match self.prop(node, "height").and_then(length) {
                    Some(height) => textarea.h(height).into_any_element(),
                    None => textarea.into_any_element(),
                }
            }
            "NumberInput" => {
                let placeholder = self.prop_text(node, "placeholder").unwrap_or_default();
                let value = self.prop_text(node, "value").unwrap_or_default();
                let path = self.two_way(node, "value");
                let state_key = format!(
                    "{key}|{}",
                    path.as_ref().map(ToString::to_string).unwrap_or_default()
                );
                let state = match self.inputs.get(&state_key) {
                    Some(state) => state.clone(),
                    None => {
                        let state = cx.new(|cx| {
                            InputState::new(window, cx)
                                .placeholder(placeholder)
                                .default_value(value.clone())
                        });
                        let changed = node.clone();
                        let subscription = cx.subscribe(&state, move |this, state, event, cx| {
                            if let InputEvent::Change = event {
                                if let Some(path) = &path {
                                    let value = state.read(cx).value().to_string();
                                    this.write(path, Json::String(value), cx);
                                }
                                this.fire(&changed, "change", cx);
                            }
                        });
                        self.subscriptions.push(subscription);
                        self.inputs.insert(state_key.clone(), state.clone());
                        state
                    }
                };
                // Same controlled-bind sync as `Input` — see that arm.
                if matches!(self.prop(node, "value"), Some(Value::Binding { .. }))
                    && state.read(cx).value() != value
                {
                    state.update(cx, |s, cx| s.set_value(value, window, cx));
                }
                NumberInput::new(&state)
                    .with_size(size(&self.prop_text(node, "size").unwrap_or_default()))
                    .appearance(!matches!(
                        self.prop(node, "appearance"),
                        Some(Value::Bool(false))
                    ))
                    .disabled(self.prop_bool(node, "disabled"))
                    .into_any_element()
            }
            "Editor" => {
                let placeholder = self.prop_text(node, "placeholder").unwrap_or_default();
                // `highlighting.default` overrides the flat `language` prop.
                let mut language = self.prop_text(node, "language").unwrap_or_default();
                if let Some(Value::Map(entries)) = self.prop(node, "highlighting")
                    && let Some((_, value)) = entries.iter().find(|(k, _)| k == "default")
                {
                    match value {
                        Value::String(s) | Value::Enum(s) => language = s.clone(),
                        _ => {}
                    }
                }
                let path = self.two_way(node, "value");
                let state_key = format!(
                    "{key}|{language}|{}",
                    path.as_ref().map(ToString::to_string).unwrap_or_default()
                );
                let value = self.prop_text(node, "value").unwrap_or_default();
                let state = match self.editors.get(&state_key) {
                    Some(state) => state.clone(),
                    None => {
                        let state = cx.new(|cx| {
                            let state = EditorState::new(window, cx)
                                .placeholder(placeholder)
                                .default_value(value.clone());
                            if language.is_empty() {
                                state
                            } else {
                                state.language(language)
                            }
                        });
                        let changed = node.clone();
                        let subscription = cx.subscribe(&state, move |this, state, event, cx| {
                            if let InputEvent::Change = event {
                                if let Some(path) = &path {
                                    let value = state.read(cx).value().to_string();
                                    this.write(path, Json::String(value), cx);
                                }
                                this.fire(&changed, "change", cx);
                            }
                        });
                        self.subscriptions.push(subscription);
                        self.editors.insert(state_key, state.clone());
                        state
                    }
                };
                // Same controlled-bind sync as `Input` — see that arm.
                if matches!(self.prop(node, "value"), Some(Value::Binding { .. }))
                    && state.read(cx).value() != value
                {
                    state.update(cx, |s, cx| s.set_value(value, window, cx));
                }
                // `diagnostics:` — squiggle markers, usually bound to state fed
                // by `forge check` (#128). Rebuilt each render. The component
                // underlines the ranges and displays a hover popover.
                if let Some(value) = self.prop(node, "diagnostics") {
                    let markers = self.markers(value);
                    state.update(cx, |s, _| {
                        if let Some(set) = s.diagnostics_mut() {
                            set.clear();
                            set.extend(markers);
                        }
                    });
                }
                let editor = Editor::new(&state)
                    .disabled(self.prop_bool(node, "disabled"))
                    .readonly(self.prop_bool(node, "readonly"));
                let element = match self.prop(node, "height").and_then(length) {
                    Some(height) => editor.h(height).into_any_element(),
                    None => editor.h_full().into_any_element(),
                };
                if std::env::var_os("FORGE_BOUNDS").is_some() {
                    BoundsLogger::new(element, format!("editor-{key}")).into_any_element()
                } else {
                    element
                }
            }
            "Checkbox" | "Switch" | "Toggle" | "SwitchButton" => {
                let label = self.prop_text(node, "label");
                let checked = self.prop_bool(node, "checked");
                let disabled = self.prop_bool(node, "disabled");
                let path = self.two_way(node, "checked");
                let changed = node.clone();
                let on_change = move |value: &bool, _: &mut Window, cx: &mut App| {
                    try_update(&weak, cx, "event", |this, cx| {
                        if let Some(path) = &path {
                            this.write(path, Json::Bool(*value), cx);
                        }
                        this.fire(&changed, "change", cx);
                    })
                    .ok();
                };
                if node.element.name == "Checkbox" {
                    Checkbox::new(id)
                        .when_some(label, |c, l| c.label(l))
                        .checked(checked)
                        .disabled(disabled)
                        .on_click(on_change)
                        .into_any_element()
                } else {
                    Switch::new(id)
                        .when_some(label, |s, l| s.label(l))
                        .checked(checked)
                        .disabled(disabled)
                        .on_click(on_change)
                        .into_any_element()
                }
            }
            "RadioGroup" => {
                let selected = self.prop(node, "selected").and_then(|v| self.number(v));
                let path = self.two_way(node, "selected");
                let changed = node.clone();
                let horizontal = self.prop_text(node, "layout").as_deref() == Some("Horizontal");
                let radios: Vec<Radio> = node
                    .children
                    .iter()
                    .enumerate()
                    .map(|(i, child)| {
                        Radio::new(SharedString::from(format!("forge-{key}.{i}")))
                            .label(self.prop_text(child, "label").unwrap_or_default())
                            .disabled(self.prop_bool(child, "disabled"))
                    })
                    .collect();
                RadioGroup::new(id)
                    .layout(if horizontal {
                        Axis::Horizontal
                    } else {
                        Axis::Vertical
                    })
                    .selected_index(selected.map(|n| n as usize))
                    .disabled(self.prop_bool(node, "disabled"))
                    .children(radios)
                    .on_click(move |index, _, cx| {
                        try_update(&weak, cx, "event", |this, cx| {
                            if let Some(path) = &path {
                                this.write(path, Json::from(*index), cx);
                            }
                            this.fire(&changed, "change", cx);
                        })
                        .ok();
                    })
                    .into_any_element()
            }
            "Select" => {
                let options: Options = self
                    .prop(node, "options")
                    .map(|v| self.list(v))
                    .unwrap_or_default()
                    .into_iter()
                    .map(SharedString::from)
                    .collect();
                let value = self.prop_text(node, "value").unwrap_or_default();
                let path = self.two_way(node, "value");
                let state_key = format!(
                    "{key}|{}|{}",
                    options.join("\u{1f}"),
                    path.as_ref().map(ToString::to_string).unwrap_or_default()
                );
                let state = match self.selects.get(&state_key) {
                    Some(state) => state.clone(),
                    None => {
                        let selected = options.iter().position(|o| *o == value).map(IndexPath::new);
                        let ix = options.iter().position(|o| *o == value).map(IndexPath::new);
                        let state =
                            cx.new(|cx| SelectState::new(options.clone(), selected, window, cx));
                        // Sync the bound `value` into the state's selected index —
                        // external writes (e.g. theme reset) land here too.
                        state.update(cx, |s, cx| s.set_selected_index(ix, window, cx));
                        let changed = node.clone();
                        let subscription = cx.subscribe(
                            &state,
                            move |this, _, event: &SelectEvent<Options>, cx| {
                                let SelectEvent::Confirm(value) = event;
                                if let Some(path) = &path {
                                    let value = value.as_ref().map(|v| v.to_string());
                                    this.write(path, Json::from(value), cx);
                                }
                                this.fire(&changed, "change", cx);
                            },
                        );
                        self.subscriptions.push(subscription);
                        self.selects.insert(state_key, state.clone());
                        state
                    }
                };
                // `value: bind` is controlled — external writes (theme reset,
                // doc loads) re-select the matching option when it differs.
                if matches!(self.prop(node, "value"), Some(Value::Binding { .. }))
                    && let Some(ix) = options.iter().position(|o| *o == value).map(IndexPath::new)
                    && state.read(cx).selected_index(cx).map(|p| p.row) != Some(ix.row)
                {
                    state.update(cx, |s, cx| s.set_selected_index(Some(ix), window, cx));
                }
                Select::new(&state)
                    .when_some(self.prop_text(node, "placeholder"), |s, p| s.placeholder(p))
                    .disabled(self.prop_bool(node, "disabled"))
                    .into_any_element()
            }
            "Tabs" => {
                // `selected` only controls the bar when the node sets it —
                // the catalog ships `default: 0`. `Prop()` resolves that
                // for unset nodes, which would pin every Tabs to index 0 and
                // swallow clicks. Read the node's own props.
                let bound = node
                    .props
                    .iter()
                    .find(|(k, _)| k == "selected")
                    .and_then(|(_, v)| self.number(v));
                let selected = bound
                    .map(|n| n as usize)
                    .or_else(|| self.tabs.get(key).copied())
                    .unwrap_or(0)
                    .min(node.children.len().saturating_sub(1));
                let path = self.two_way(node, "selected");
                let changed = node.clone();
                let tab_key = key.to_owned();
                let tabs: Vec<Tab> = node
                    .children
                    .iter()
                    .map(|child| {
                        Tab::new()
                            .label(self.prop_text(child, "label").unwrap_or_default())
                            .disabled(self.prop_bool(child, "disabled"))
                    })
                    .collect();
                let content = node.children.get(selected).map(|tab| {
                    let mut out = Vec::with_capacity(tab.children.len());
                    for (i, child) in tab.children.iter().enumerate() {
                        out.extend(self.expand(
                            child,
                            &format!("{key}.{selected}.{i}"),
                            window,
                            cx,
                        ));
                    }
                    out
                });
                let bar = TabBar::new(id.clone())
                    .with_variant(tab_variant(
                        &self.prop_text(node, "variant").unwrap_or_default(),
                    ))
                    .selected_index(selected)
                    .children(tabs)
                    .on_click(move |index: &usize, _, cx| {
                        let index = *index;
                        let _ = try_update(&weak, cx, "tab click", |this, cx| {
                            this.tabs.insert(tab_key.clone(), index);
                            if let Some(path) = &path {
                                this.write(path, Json::from(index), cx);
                            }
                            this.fire(&changed, "change", cx);
                            cx.notify();
                        });
                    });
                // `styled` — the `style:` entries belong on this column itself.
                // Plain `div()` is `display: block`, so letting `wrap` carry
                // `grow`/`basis`/`min-height` would leave this column at
                // content height and the tab content couldn't fill the panel.
                styled = true;
                container(
                    v_flex().gap_3(),
                    &node.style,
                    id,
                    vec![
                        bar.into_any_element(),
                        v_flex()
                            .flex_1()
                            .min_h_0()
                            .children(content.unwrap_or_default())
                            .into_any_element(),
                    ],
                    &mut self.scroll_handles,
                    cx,
                )
            }
            "Separator" => {
                let separator = match self.prop_text(node, "orientation").as_deref() {
                    Some("Vertical") => Separator::vertical(),
                    _ => Separator::horizontal(),
                };
                separator
                    .when(self.prop_bool(node, "dashed"), |s| s.dashed())
                    .when_some(self.prop_text(node, "label"), |s, l| s.label(l))
                    .into_any_element()
            }
            "GroupBox" => {
                let kids = children(self, window, cx);
                styled = true;
                style(
                    GroupBox::new()
                        .id(id)
                        .with_variant(group_box_variant(
                            &self.prop_text(node, "variant").unwrap_or_default(),
                        ))
                        .when_some(self.prop_text(node, "title"), |g, t| g.title(t))
                        .when_some(self.prop_text(node, "footer"), |g, f| g.footer(f))
                        .children(kids),
                    &node.style,
                    cx,
                )
                .into_any_element()
            }
            "Empty" => {
                let kids = children(self, window, cx);
                let mut header = EmptyHeader::new();
                if let Some(name) = self.prop_text(node, "icon") {
                    header = header.media(
                        EmptyMedia::new()
                            .with_variant(EmptyMediaVariant::Icon)
                            .child(self.doc_icon(&name, cx)),
                    );
                }
                if let Some(title) = self.prop_text(node, "title") {
                    header = header.title(EmptyTitle::new().child(title));
                }
                if let Some(description) = self.prop_text(node, "description") {
                    header = header.description(EmptyDescription::new().child(description));
                }
                let mut empty = Empty::new().header(header);
                if !kids.is_empty() {
                    empty = empty.content(EmptyContent::new().children(kids));
                }
                empty.into_any_element()
            }
            "Link" => {
                let clicked = node.clone();
                Link::new(id)
                    .when_some(self.prop_text(node, "href"), |l, h| l.href(h))
                    .disabled(self.prop_bool(node, "disabled"))
                    .child(self.prop_text(node, "label").unwrap_or_default())
                    .on_click(move |_, _, cx| {
                        try_update(&weak, cx, "event", |this, cx| {
                            this.fire(&clicked, "click", cx)
                        })
                        .ok();
                    })
                    .into_any_element()
            }
            "Badge" => {
                let count = self.prop(node, "count").and_then(|v| self.number(v));
                let max = self.prop(node, "max").and_then(|v| self.number(v));
                let color = self.prop(node, "color").and_then(|v| color(v, cx));
                let kids = children(self, window, cx);
                Badge::new()
                    .when_some(count, |b, n| b.count(n as usize))
                    .when_some(max, |b, n| b.max(n as usize))
                    .when(self.prop_bool(node, "dot"), |b| b.dot())
                    .when_some(color, |b, c| b.color(c))
                    .children(kids)
                    .into_any_element()
            }
            "Tag" => Tag::new()
                .with_variant(tag_variant(
                    &self.prop_text(node, "variant").unwrap_or_default(),
                ))
                .when(self.prop_bool(node, "outline"), |t| t.outline())
                .child(self.prop_text(node, "label").unwrap_or_default())
                .into_any_element(),
            "Icon" => {
                let color = self.prop(node, "color").and_then(|v| color(v, cx));
                let name = self.prop_text(node, "name").unwrap_or_default();
                // `set:` (or a `set:name` prefix) picks the icon set — default
                // is the doc's `meta.iconset`/project iconset. A name missing
                // locally queues the iconify load where resolvable and displays
                // a placeholder until the SVG lands.
                let (set, name) = match name.split_once(':') {
                    Some((set, name)) => (set.to_string(), name.to_string()),
                    None => (
                        self.prop_text(node, "set")
                            .unwrap_or_else(|| self.iconset().to_owned()),
                        name,
                    ),
                };
                let element = svg().path(doc_icon_path(&set, &name, cx));
                // `svg()` has no intrinsic size — `size:` takes a px length
                // or the catalog enum (gpui-kit's steps: 12/14/16/24px).
                // default 16px like a Medium icon.
                let element = match self.prop(node, "size").and_then(length) {
                    Some(size) => element.w(size).h(size),
                    None => {
                        let s = match self.prop_text(node, "size").as_deref() {
                            Some("XSmall") => 12.,
                            Some("Small") => 14.,
                            Some("Large") => 24.,
                            _ => 16.,
                        };
                        element.w(px(s)).h(px(s))
                    }
                };
                // `svg()` skips `paint_svg` when `style.text.color` is None —
                // the ambient text style carries no color inside containers
                // like `uniform_list` cells, so a colorless `Icon` paints
                // nothing. Default to the theme foreground like the `Icon`
                // component does.
                element
                    .text_color(color.unwrap_or_else(|| cx.theme().foreground))
                    .into_any_element()
            }
            "Image" => {
                let src = self.prop_text(node, "src").unwrap_or_default();
                // `img()` keeps a multicolor image's colors — `svg()` is a
                // single-color mask tinted with `text_color` for theme icons.
                // A `Path` source reads the file directly. A URL or a bare name
                // falls through as `ImageSource` (uri / embedded asset key).
                let path = self.resolve_path(&src);
                let element = if path.is_file() {
                    img(path)
                } else {
                    img(src.as_str())
                };
                let fit = match self.prop_text(node, "object-fit").as_deref() {
                    Some("Cover") => ObjectFit::Cover,
                    Some("Fill") => ObjectFit::Fill,
                    Some("ScaleDown") => ObjectFit::ScaleDown,
                    _ => ObjectFit::Contain,
                };
                let element = element.object_fit(fit);
                match self.prop(node, "size").and_then(length) {
                    Some(size) => element.w(size).h(size),
                    None => element,
                }
                .into_any_element()
            }
            "Spinner" => {
                let color = self.prop(node, "color").and_then(|v| color(v, cx));
                Spinner::new()
                    .when_some(self.prop_text(node, "icon"), |s, name| {
                        s.icon(self.doc_icon(&name, cx))
                    })
                    .when_some(color, |s, c| s.color(c))
                    .into_any_element()
            }
            "Alert" => {
                let color = self.prop(node, "color").and_then(|v| color(v, cx));
                let message = self.prop_text(node, "message").unwrap_or_default();
                let mut alert = Alert::new(id, message).with_variant(alert_variant(
                    &self.prop_text(node, "variant").unwrap_or_default(),
                ));
                if let Some(title) = self.prop_text(node, "title") {
                    alert = alert.title(title);
                }
                alert
                    .when(self.prop_bool(node, "banner"), |a| a.banner())
                    .visible(!matches!(
                        self.prop(node, "visible"),
                        Some(Value::Bool(false))
                    ))
                    .when_some(self.prop_text(node, "icon"), |a, name| {
                        a.icon(self.doc_icon(&name, cx))
                    })
                    .when_some(color, |a, c| a.icon(icon("info").text_color(c)))
                    .into_any_element()
            }
            "Kbd" => match Keystroke::parse(Box::leak(
                self.prop_text(node, "keys")
                    .unwrap_or_default()
                    .into_boxed_str(),
            )) {
                Ok(stroke) => Kbd::new(stroke)
                    .appearance(!matches!(
                        self.prop(node, "appearance"),
                        Some(Value::Bool(false))
                    ))
                    .when(self.prop_bool(node, "outline"), |k| k.outline())
                    .into_any_element(),
                Err(_) => placeholder("invalid keystroke".to_owned(), cx),
            },
            "Settings" => {
                let id = format!("settings-{key}");
                let pages = node
                    .children
                    .iter()
                    .enumerate()
                    .map(|(i, c)| self.setting_page(c, &format!("{key}/{i}"), window, cx))
                    .collect::<Vec<_>>();
                Settings::new(id).pages(pages).into_any_element()
            }
            "SettingPage" | "SettingsGroup" | "SettingItem" => placeholder(
                format!("`{}` only renders inside `Settings`", node.element.name),
                cx,
            ),
            // A `Dialog` used inline renders its contents in a bordered box.
            // Top-level `Dialog#id:` declarations open via `open_dialog`.
            "Dialog" => {
                let kids = self.slot_children(node, &format!("{key}/0"), window, cx);
                let title = self.prop_text(node, "title");
                let el = div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded_lg()
                    .p_4()
                    .when_some(title, |d, t| d.child(Label::new(t)))
                    .children(kids);
                styled = true;
                style(el, &node.style, cx).into_any_element()
            }
            "Skeleton" => Skeleton::new()
                .when(self.prop_bool(node, "secondary"), |s| s.secondary())
                .into_any_element(),
            "Breadcrumb" => {
                let items: Vec<BreadcrumbItem> = node
                    .children
                    .iter()
                    .map(|child| {
                        BreadcrumbItem::new(self.prop_text(child, "label").unwrap_or_default())
                            .disabled(self.prop_bool(child, "disabled"))
                    })
                    .collect();
                Breadcrumb::new().children(items).into_any_element()
            }
            "Pagination" => {
                let changed = node.clone();
                let path = self.two_way(node, "page");
                Pagination::new(id)
                    .current_page(
                        self.prop(node, "page")
                            .and_then(|v| self.number(v))
                            .unwrap_or_default() as usize,
                    )
                    .total_pages(
                        self.prop(node, "total")
                            .and_then(|v| self.number(v))
                            .unwrap_or_default() as usize,
                    )
                    .when(self.prop_bool(node, "compact"), |p| p.compact())
                    .when_some(
                        self.prop(node, "visible").and_then(|v| self.number(v)),
                        |p, n| p.visible_pages(n as usize),
                    )
                    .on_click(move |page, _, cx| {
                        try_update(&weak, cx, "event", |this, cx| {
                            if let Some(path) = &path {
                                this.write(path, Json::from(*page), cx);
                            }
                            this.fire(&changed, "change", cx);
                        })
                        .ok();
                    })
                    .into_any_element()
            }
            "Rating" => {
                let changed = node.clone();
                let path = self.two_way(node, "value");
                Rating::new(id)
                    .with_size(size(&self.prop_text(node, "size").unwrap_or_default()))
                    .disabled(self.prop_bool(node, "disabled"))
                    .value(
                        self.prop(node, "value")
                            .and_then(|v| self.number(v))
                            .unwrap_or_default() as usize,
                    )
                    .max(
                        self.prop(node, "max")
                            .and_then(|v| self.number(v))
                            .unwrap_or(5.0) as usize,
                    )
                    .on_click(move |value, _, cx| {
                        try_update(&weak, cx, "event", |this, cx| {
                            if let Some(path) = &path {
                                this.write(path, Json::from(*value), cx);
                            }
                            this.fire(&changed, "change", cx);
                        })
                        .ok();
                    })
                    .into_any_element()
            }
            "Toolbar" => {
                let kids = children(self, window, cx);
                Toolbar::new(id)
                    .disabled(self.prop_bool(node, "disabled"))
                    .contents(kids)
                    .into_any_element()
            }
            "Progress" => {
                let color = self.prop(node, "color").and_then(|v| color(v, cx));
                let mut bar = Progress::new(id)
                    .loading(self.prop_bool(node, "loading"))
                    .value(
                        self.prop(node, "value")
                            .and_then(|v| self.number(v))
                            .unwrap_or_default() as f32,
                    );
                if let Some(c) = color {
                    bar = bar.color(c);
                }
                bar.into_any_element()
            }
            "ProgressCircle" => {
                let color = self.prop(node, "color").and_then(|v| color(v, cx));
                let mut circle = ProgressCircle::new(id)
                    .loading(self.prop_bool(node, "loading"))
                    .value(
                        self.prop(node, "value")
                            .and_then(|v| self.number(v))
                            .unwrap_or_default() as f32,
                    );
                if let Some(c) = color {
                    circle = circle.color(c);
                }
                circle.into_any_element()
            }
            "DescriptionList" => {
                let items: Vec<DescriptionItem> = node
                    .children
                    .iter()
                    .map(|child| {
                        let mut item = DescriptionItem::new(
                            self.prop_text(child, "label").unwrap_or_default(),
                        )
                        .value(self.prop_text(child, "value").unwrap_or_default());
                        if let Some(span) = self.prop(child, "span").and_then(|v| self.number(v)) {
                            item = item.span(span as usize);
                        }
                        item
                    })
                    .collect();
                let horizontal = self
                    .prop_text(node, "layout")
                    .is_some_and(|l| l.eq_ignore_ascii_case("horizontal"));
                let list = if horizontal {
                    DescriptionList::horizontal()
                } else {
                    DescriptionList::vertical()
                };
                list.children(items)
                    .bordered(self.prop_bool(node, "bordered"))
                    .when_some(
                        self.prop(node, "columns").and_then(|v| self.number(v)),
                        |l, n| l.columns(n as usize),
                    )
                    .when_some(self.prop(node, "label-width").and_then(length), |l, w| {
                        l.label_width(w)
                    })
                    .into_any_element()
            }
            "StatusBar" => {
                let mut kids = children(self, window, cx).into_iter();
                let mut bar = StatusBar::new();
                if let Some(left) = kids.next() {
                    bar = bar.left(left);
                }
                if let Some(right) = kids.next() {
                    bar = bar.right(right);
                }
                bar.into_any_element()
            }
            "Avatar" => self.avatar(node, cx).into_any_element(),
            "AvatarGroup" => {
                let avatars: Vec<Avatar> = node
                    .children
                    .iter()
                    .map(|child| self.avatar(child, cx))
                    .collect();
                AvatarGroup::new()
                    .children(avatars)
                    .when_some(
                        self.prop(node, "limit").and_then(|v| self.number(v)),
                        |g, n| g.limit(n as usize),
                    )
                    .when(self.prop_bool(node, "ellipsis"), |g| g.ellipsis())
                    .into_any_element()
            }
            "Stepper" => {
                let items: Vec<StepperItem> = node
                    .children
                    .iter()
                    .map(|child| {
                        let mut item =
                            StepperItem::new().disabled(self.prop_bool(child, "disabled"));
                        if let Some(name) = self.prop_text(child, "icon") {
                            item = item.icon(self.doc_icon(&name, cx));
                        }
                        if let Some(label) = self.prop_text(child, "label") {
                            item = item.child(label);
                        }
                        item
                    })
                    .collect();
                let changed = node.clone();
                let path = self.two_way(node, "selected");
                Stepper::new(id)
                    .items(items)
                    .selected_index(
                        self.prop(node, "selected")
                            .and_then(|v| self.number(v))
                            .unwrap_or_default() as usize,
                    )
                    .text_center(self.prop_bool(node, "text-center"))
                    .disabled(self.prop_bool(node, "disabled"))
                    .when(
                        self.prop_text(node, "layout")
                            .is_some_and(|l| l.eq_ignore_ascii_case("vertical")),
                        |s| s.vertical(),
                    )
                    .on_click(move |index, _, cx| {
                        try_update(&weak, cx, "event", |this, cx| {
                            if let Some(path) = &path {
                                this.write(path, Json::from(*index), cx);
                            }
                            this.fire(&changed, "change", cx);
                        })
                        .ok();
                    })
                    .into_any_element()
            }
            "Slider" => {
                let path = self.two_way(node, "value");
                let state_key = format!(
                    "{key}|{}",
                    path.as_ref().map(ToString::to_string).unwrap_or_default()
                );
                let state = match self.sliders.get(&state_key) {
                    Some(state) => state.clone(),
                    None => {
                        let value = self
                            .prop(node, "value")
                            .and_then(|v| self.number(v))
                            .unwrap_or_default();
                        let mut state = SliderState::new()
                            .min(
                                self.prop(node, "min")
                                    .and_then(|v| self.number(v))
                                    .unwrap_or_default() as f32,
                            )
                            .max(
                                self.prop(node, "max")
                                    .and_then(|v| self.number(v))
                                    .unwrap_or(100.0) as f32,
                            );
                        if let Some(step) = self.prop(node, "step").and_then(|v| self.number(v)) {
                            state = state.step(step as f32);
                        }
                        state = state.default_value(SliderValue::Single(value as f32));
                        let state = cx.new(|_| state);
                        let changed = node.clone();
                        let subscription = cx.subscribe(&state, move |this, _state, event, cx| {
                            let SliderEvent::Change(value) = event else {
                                return;
                            };
                            if let SliderValue::Single(value) = value {
                                if let Some(path) = &path {
                                    this.write(path, Json::from(*value), cx);
                                }
                                this.fire(&changed, "change", cx);
                            }
                        });
                        self.subscriptions.push(subscription);
                        self.sliders.insert(state_key.clone(), state.clone());
                        state
                    }
                };
                let slider = Slider::new(&state)
                    .disabled(self.prop_bool(node, "disabled"))
                    .when(self.prop_bool(node, "vertical"), |s| s.vertical())
                    .when(self.prop_bool(node, "reverse"), |s| s.reverse());
                slider.into_any_element()
            }
            "Accordion" => {
                let changed = node.clone();
                // The component Accordion is controlled: the trigger only reports the
                // new open set via `on_toggle_click`. Persist it in view state so each
                // re-render rebuilds items with the right `open` flags.
                let open_path = Path(vec![
                    PathSegment::Field("accordion".into()),
                    PathSegment::Field(key.to_owned()),
                    PathSegment::Field("open".into()),
                ]);
                let open_set: HashSet<usize> = match get_path(&self.state, &open_path) {
                    Some(Json::Array(list)) => list
                        .iter()
                        .filter_map(|v| v.as_u64())
                        .map(|v| v as usize)
                        .collect(),
                    _ => {
                        let seeded: Vec<usize> = node
                            .children
                            .iter()
                            .enumerate()
                            .filter(|(_, c)| self.prop_bool(c, "open"))
                            .map(|(i, _)| i)
                            .collect();
                        set_path(&mut self.state, &open_path, Json::from(seeded.clone()));
                        seeded.into_iter().collect()
                    }
                };
                let open_path_c = open_path.clone();
                let mut accordion = Accordion::new(id)
                    .multiple(self.prop_bool(node, "multiple"))
                    .bordered(self.prop_bool(node, "bordered"))
                    .disabled(self.prop_bool(node, "disabled"))
                    .on_toggle_click(move |indices: &[usize], _, cx| {
                        let open_path = open_path_c.clone();
                        let changed = changed.clone();
                        try_update(&weak, cx, "event", |this, cx| {
                            this.update_state(&open_path, Json::from(indices.to_vec()), cx);
                            this.fire(&changed, "change", cx);
                        })
                        .ok();
                    });
                for (ci, child) in node.children.iter().enumerate() {
                    let title = self.prop_text(child, "title").unwrap_or_default();
                    let mut kids: Vec<AnyElement> = Vec::with_capacity(child.children.len());
                    for (i, kid) in child.children.iter().enumerate() {
                        kids.extend(self.expand(kid, &format!("{key}.{ci}.{i}"), window, cx));
                    }
                    let mut item = gpui_kit::component::accordion::AccordionItem::new()
                        .title(title)
                        .open(open_set.contains(&ci))
                        .disabled(self.prop_bool(child, "disabled"))
                        .child(v_flex().children(kids));
                    if let Some(name) = self.prop_text(child, "icon") {
                        item = item.icon(self.doc_icon(&name, cx));
                    }
                    accordion = accordion.item(move |_| item);
                }
                accordion.into_any_element()
            }
            "Collapsible" => {
                let kids = children(self, window, cx);
                Collapsible::new()
                    .open(self.prop_bool(node, "open"))
                    .motion_id(id)
                    .content(v_flex().children(kids))
                    .into_any_element()
            }
            "Form" => {
                let fields: Vec<Field> = node
                    .children
                    .iter()
                    .enumerate()
                    .map(|(i, child)| self.form_field(child, &format!("{key}/{i}"), window, cx))
                    .collect();
                let form = if self
                    .prop_text(node, "layout")
                    .is_some_and(|l| l.eq_ignore_ascii_case("horizontal"))
                {
                    Form::horizontal()
                } else {
                    Form::vertical()
                };
                form.children(fields)
                    .when_some(
                        self.prop(node, "columns").and_then(|v| self.number(v)),
                        |f, n| f.columns(n as usize),
                    )
                    .when_some(
                        self.prop(node, "label-width").and_then(|v| self.number(v)),
                        |f, n| f.label_width(px(n as f32)),
                    )
                    .when(
                        self.prop_text(node, "label-layout")
                            .is_some_and(|l| l.eq_ignore_ascii_case("vertical")),
                        |f| f.label_layout(Axis::Vertical),
                    )
                    .into_any_element()
            }
            "FormField" => self.form_field(node, key, window, cx).into_any_element(),
            "ColorPicker" => {
                let skey = format!("{key}|{}", bind_salt(node));
                let state = match self.color_pickers.get(&skey) {
                    Some(state) => state.clone(),
                    None => {
                        let state = cx.new(|cx| ColorPickerState::new(window, cx));
                        let path = self.two_way(node, "value");
                        let changed = node.clone();
                        let subscription = cx.subscribe(&state, move |this, _state, event, cx| {
                            let ColorPickerEvent::Change(value) = event;
                            if let Some(path) = &path {
                                let hex = value.map(|c| c.to_hex()).unwrap_or_default();
                                this.write(path, Json::String(hex), cx);
                            }
                            this.fire(&changed, "change", cx);
                        });
                        self.subscriptions.push(subscription);
                        self.color_pickers.insert(skey.clone(), state.clone());
                        state
                    }
                };
                ColorPicker::new(&state).into_any_element()
            }
            "OtpInput" => {
                let skey = format!("{key}|{}", bind_salt(node));
                let state = match self.otps.get(&skey) {
                    Some(state) => state.clone(),
                    None => {
                        let length = self
                            .prop(node, "length")
                            .and_then(|v| self.number(v))
                            .unwrap_or(6.0) as usize;
                        let state = cx.new(|cx| OtpState::new(length, window, cx));
                        let path = self.two_way(node, "value");
                        let changed = node.clone();
                        let subscription =
                            cx.subscribe(&state, move |this, state, event, cx| match event {
                                OtpEvent::Change => {
                                    if let Some(path) = &path {
                                        let value = state.read(cx).value().to_string();
                                        this.write(path, Json::String(value), cx);
                                    }
                                    this.fire(&changed, "change", cx);
                                }
                                OtpEvent::Complete => {
                                    this.fire(&changed, "complete", cx);
                                }
                                _ => {}
                            });
                        self.subscriptions.push(subscription);
                        self.otps.insert(skey.clone(), state.clone());
                        state
                    }
                };
                OtpInput::new(&state).into_any_element()
            }
            "DatePicker" => {
                let skey = format!("{key}|{}", bind_salt(node));
                let state = match self.date_pickers.get(&skey) {
                    Some(state) => state.clone(),
                    None => {
                        let state = cx.new(|cx| DatePickerState::new(window, cx));
                        let path = self.two_way(node, "value");
                        let changed = node.clone();
                        let subscription = cx.subscribe(&state, move |this, _state, event, cx| {
                            let gpui_kit::component::date_picker::DatePickerEvent::Change(date) =
                                event;
                            if let Some(path) = &path {
                                this.write(path, Json::String(format!("{date:?}")), cx);
                            }
                            this.fire(&changed, "change", cx);
                        });
                        self.subscriptions.push(subscription);
                        self.date_pickers.insert(skey.clone(), state.clone());
                        state
                    }
                };
                DatePicker::new(&state)
                    .disabled(self.prop_bool(node, "disabled"))
                    .into_any_element()
            }
            "Calendar" => {
                let skey = format!("{key}|{}", bind_salt(node));
                let state = match self.calendars.get(&skey) {
                    Some(state) => state.clone(),
                    None => {
                        let state = cx.new(|cx| CalendarState::new(window, cx));
                        self.calendars.insert(skey.clone(), state.clone());
                        state
                    }
                };
                Calendar::new(&state).into_any_element()
            }
            "TimeField" => {
                let skey = format!("{key}|{}", bind_salt(node));
                let state = match self.time_fields.get(&skey) {
                    Some(state) => state.clone(),
                    None => {
                        let state = cx.new(|cx| TimeFieldState::new(window, cx));
                        let changed = node.clone();
                        let subscription = cx.subscribe(&state, move |this, _state, _event, cx| {
                            this.fire(&changed, "change", cx);
                        });
                        self.subscriptions.push(subscription);
                        self.time_fields.insert(skey.clone(), state.clone());
                        state
                    }
                };
                TimeField::new(&state)
                    .invalid(self.prop_bool(node, "invalid"))
                    .into_any_element()
            }
            "TitleBar" => TitleBar::new()
                .children(children(self, window, cx))
                .into_any_element(),
            // Light/Dark `Select` wired straight to `Theme::change` — library
            // adapters (ely) resync their own theme off the host's each render.
            "ThemeSwitcher" => {
                let state = match &self.theme_select {
                    Some(state) => state.clone(),
                    None => {
                        let dark = gpui_kit::component::theme::Theme::global(cx).is_dark();
                        let state = cx.new(|cx| {
                            SelectState::new(
                                vec!["Light".into(), "Dark".into()],
                                Some(IndexPath::new(if dark { 1 } else { 0 })),
                                window,
                                cx,
                            )
                        });
                        self.subscriptions.push(cx.subscribe(
                            &state,
                            |_this, _state, event: &SelectEvent<Options>, cx| {
                                use gpui_kit::component::theme::{Theme, ThemeMode};
                                if let SelectEvent::Confirm(Some(value)) = event {
                                    Theme::change(
                                        if value.as_ref() == "Dark" {
                                            ThemeMode::Dark
                                        } else {
                                            ThemeMode::Light
                                        },
                                        None,
                                        cx,
                                    );
                                }
                            },
                        ));
                        self.theme_select = Some(state.clone());
                        state
                    }
                };
                Select::new(&state).into_any_element()
            }
            "Message" => {
                let mut message = Message::new();
                if self
                    .prop_text(node, "alignment")
                    .is_some_and(|a| a.eq_ignore_ascii_case("end"))
                {
                    message =
                        message.alignment(gpui_kit::component::message::MessageAlignment::End);
                }
                if let Some(name) = self.prop_text(node, "avatar") {
                    message = message.avatar(self.doc_icon(&name, cx));
                }
                for (i, child) in node.children.iter().enumerate() {
                    let kids = self.slot_children(child, &format!("{key}/{i}"), window, cx);
                    match child.element.name.as_str() {
                        "MessageAvatar" => {
                            message = message.avatar_slot(MessageAvatar::new().children(kids));
                        }
                        "MessageHeader" => {
                            message = message.header(MessageHeader::new().children(kids));
                        }
                        "MessageContent" => {
                            message = message.content(MessageContent::new().children(kids));
                        }
                        "MessageFooter" => {
                            message = message.footer(MessageFooter::new().children(kids));
                        }
                        _ => {}
                    }
                }
                message.into_any_element()
            }
            "Table" | "DataTable" => {
                let mut table = Table::new();
                for (i, child) in node.children.iter().enumerate() {
                    let key = format!("{key}/{i}");
                    match child.element.name.as_str() {
                        "TableHeader" => {
                            let heads: Vec<TableHead> =
                                child.children.iter().map(|h| self.table_head(h)).collect();
                            table = table.child(TableHeader::new().children(heads));
                        }
                        "TableBody" | "TableFooter" => {
                            let rows: Vec<gpui_kit::component::table::TableRow> = child
                                .children
                                .iter()
                                .map(|r| {
                                    let cells: Vec<TableCell> =
                                        r.children.iter().map(|c| self.table_cell(c)).collect();
                                    gpui_kit::component::table::TableRow::new().children(cells)
                                })
                                .collect();
                            table = if child.element.name.as_str() == "TableFooter" {
                                table.child(TableFooter::new().children(rows))
                            } else {
                                table.child(TableBody::new().children(rows))
                            };
                            let _ = key;
                        }
                        "TableCaption" => {
                            let kids = self.slot_children(child, &key, window, cx);
                            table = table.child(TableCaption::new().children(kids));
                        }
                        _ => {}
                    }
                }
                table.into_any_element()
            }
            "TableHead" => self.table_head(node).into_any_element(),
            "TableCell" => self.table_cell(node).into_any_element(),
            "Resizable" => {
                let panels: Vec<ResizablePanel> = node
                    .children
                    .iter()
                    .enumerate()
                    .map(|(i, child)| {
                        let kids = self.slot_children(child, &format!("{key}/{i}"), window, cx);
                        let mut panel = resizable_panel().children(kids);
                        if let Some(n) = self.prop(child, "size").and_then(|v| self.number(v)) {
                            panel = panel.size(px(n as f32));
                        }
                        let min = self.prop(child, "min-size").and_then(|v| self.number(v));
                        let max = self.prop(child, "max-size").and_then(|v| self.number(v));
                        if min.is_some() || max.is_some() {
                            panel = panel.size_range(
                                px(min.unwrap_or(0.0) as f32)
                                    ..px(max.unwrap_or(f32::MAX as f64) as f32),
                            );
                        }
                        panel
                            .when(self.prop_bool(child, "fixed"), |p| p.fixed())
                            .visible(!matches!(
                                self.prop(child, "visible"),
                                Some(Value::Bool(false))
                            ))
                    })
                    .collect();
                let skey = format!("{key}|{}", bind_salt(node));
                if !self.resizables.contains_key(&skey) {
                    let state = cx.new(|_| ResizableState::default());
                    self.subscriptions
                        .push(cx.observe(&state, |_, _, cx| cx.notify()));
                    self.resizables.insert(skey.clone(), state);
                }
                let state = self.resizables[&skey].clone();
                let group = if self
                    .prop_text(node, "layout")
                    .is_some_and(|l| l.eq_ignore_ascii_case("vertical"))
                {
                    v_resizable(id)
                } else {
                    h_resizable(id)
                };
                group.with_state(&state).children(panels).into_any_element()
            }
            "Sidebar" => {
                let mut sidebar =
                    Sidebar::<SidebarMenu>::new(id).collapsed(self.prop_bool(node, "collapsed"));
                if self
                    .prop_text(node, "side")
                    .is_some_and(|s| s.eq_ignore_ascii_case("right"))
                {
                    sidebar = sidebar.side(gpui_kit::base::Side::Right);
                }
                match self.prop_text(node, "collapsible").as_deref() {
                    Some("none") => {
                        sidebar = sidebar.collapsible(SidebarCollapsible::None);
                    }
                    Some("offcanvas") => {
                        sidebar = sidebar.collapsible(SidebarCollapsible::Offcanvas);
                    }
                    _ => {}
                }
                for (i, child) in node.children.iter().enumerate() {
                    let key = format!("{key}/{i}");
                    match child.element.name.as_str() {
                        "SidebarHeader" => {
                            let kids = self.slot_children(child, &key, window, cx);
                            sidebar = sidebar.header(v_flex().children(kids));
                        }
                        "SidebarFooter" => {
                            let kids = self.slot_children(child, &key, window, cx);
                            sidebar = sidebar.footer(v_flex().children(kids));
                        }
                        "SidebarMenu" => {
                            let items: Vec<SidebarMenuItem> = child
                                .children
                                .iter()
                                .enumerate()
                                .map(|(j, item)| {
                                    self.sidebar_item(item, &format!("{key}/{j}"), window, cx)
                                })
                                .collect();
                            sidebar = sidebar.child(SidebarMenu::new().children(items));
                        }
                        "SidebarMenuItem" => {
                            let item = self.sidebar_item(child, &key, window, cx);
                            sidebar = sidebar.child(SidebarMenu::new().child(item));
                        }
                        _ => {}
                    }
                }
                sidebar.into_any_element()
            }
            "List" | "VirtualList" => {
                let skey = format!("{key}|{}", bind_salt(node));
                let state = match self.lists.get(&skey) {
                    Some(state) => state.clone(),
                    None => {
                        // The weak view must come from the OUTER context — `cx` inside
                        // `cx.new` is the new ListState's context, not the ForgeView's.
                        let weak = cx.entity().downgrade();
                        let state: Entity<ListState<ForgeListDelegate>> = cx.new(|cx| {
                            ListState::new(
                                ForgeListDelegate {
                                    view: weak,
                                    key: key.to_owned(),
                                    children: node.children.clone(),
                                    selected: Rc::default(),
                                },
                                window,
                                cx,
                            )
                        });
                        let path = self.two_way(node, "selected");
                        let changed = node.clone();
                        let subscription =
                            cx.subscribe(&state, move |this, _state, event, cx| match event {
                                ListEvent::Select(ix) => {
                                    if let Some(path) = &path {
                                        this.write(path, Json::from(ix.row), cx);
                                    }
                                    this.fire(&changed, "select", cx);
                                }
                                ListEvent::Confirm(ix) => {
                                    let _ = ix;
                                    this.fire(&changed, "confirm", cx);
                                }
                                _ => {}
                            });
                        self.subscriptions.push(subscription);
                        self.lists.insert(skey.clone(), state.clone());
                        state
                    }
                };
                List::new(&state).into_any_element()
            }
            "Tree" => {
                // Nodes live in `nodes` by their `TreeItem` id so the paint-time
                // `render_item` can rebuild rows — it runs several times per row.
                let mut nodes: HashMap<String, Node> = HashMap::new();
                let items: Vec<TreeItem> = node
                    .children
                    .iter()
                    .enumerate()
                    .map(|(i, child)| self.tree_data(child, &format!("{key}/{i}"), &mut nodes))
                    .collect();
                let skey = format!("{key}|{}", bind_salt(node));
                let state = match self.trees.get(&skey) {
                    Some(state) => {
                        state.update(cx, |state, cx| {
                            *state =
                                std::mem::replace(state, TreeState::new(cx)).items(items.clone());
                        });
                        state.clone()
                    }
                    None => {
                        let state = cx.new(|cx| TreeState::new(cx).items(items.clone()));
                        self.trees.insert(skey.clone(), state.clone());
                        state
                    }
                };
                // Sync the bound `selected` value into the tree's internal
                // `selected_ix` — the element-picker writes `document.selected`
                // (a doc path like "0.0.1"). The item id uses the same
                // `{key}/{i}` shape, so a `/`-normalized path matches.
                if let Some(selected) = self.prop_text(node, "selected") {
                    // doc path "0.4.1" → item id "root/4/1"
                    let id = selected.replacen('0', "root", 1).replace('.', "/");
                    state.update(cx, |state, cx| {
                        if let Some(ix) = state.index_of(&id.into()) {
                            state.set_selected_index(Some(ix), cx);
                        }
                    });
                }
                let weak = cx.entity().downgrade();
                let nodes = Rc::new(nodes);
                let key = key.to_owned();
                Tree::new(&state, move |ix, entry, selected, window, cx| {
                    let mut item = ListItem::new(ix);
                    let id = entry.item().id.to_string();
                    if let Some(node) = nodes.get(&id).cloned() {
                        let key = format!("{key}/{id}");
                        if let Ok(row) = try_update(&weak, cx, "tree row", |view, cx| {
                            let mut row = div().flex().flex_row().gap_2();
                            if let Some(name) = view.prop_text(&node, "icon") {
                                row = row.child(view.doc_icon(&name, cx));
                            }
                            if let Some(label) = view.prop_text(&node, "label") {
                                row = row.child(label);
                            }
                            for (i, child) in node.children.iter().enumerate() {
                                if child.element.name.as_str() != "TreeItem" {
                                    row = row.children(view.expand(
                                        child,
                                        &format!("{key}/{i}"),
                                        window,
                                        cx,
                                    ));
                                }
                            }
                            row
                        }) {
                            item = item.child(row);
                        }
                    }
                    item.selected(selected)
                })
                .into_any_element()
            }
            "Grid" | "GridLayout" => {
                // CSS-grid display. `columns:` sets the track count.
                let kids = children(self, window, cx);
                styled = true;
                let mut el = div().grid();
                if let Some(cols) = self.prop(node, "columns").and_then(|v| self.number(v)) {
                    el = el.grid_cols(cols as u16);
                }
                container(el, &node.style, id, kids, &mut self.scroll_handles, cx)
            }
            "Scrollable" => {
                // A styled column that always scrolls — `overflow` style entries
                // still win if the document sets them.
                let kids = children(self, window, cx);
                styled = true;
                let mut entries = node.style.clone();
                if !entries.iter().any(|(k, _)| k.starts_with("overflow")) {
                    entries.push(("overflow".to_owned(), Value::Enum("scroll".to_owned())));
                }
                container(v_flex(), &entries, id, kids, &mut self.scroll_handles, cx)
            }
            "TextView" => {
                let text = self
                    .prop_text(node, "text")
                    .or_else(|| node.content.as_ref().map(|v| self.text(v)))
                    .unwrap_or_default();
                match self.prop_text(node, "format").as_deref() {
                    Some("html") | Some("Html") => TextView::html(id, text).into_any_element(),
                    _ => TextView::markdown(id, text).into_any_element(),
                }
            }
            "Shimmer" => {
                let text = self
                    .prop_text(node, "text")
                    .or_else(|| node.content.as_ref().map(|v| self.text(v)))
                    .unwrap_or_default();
                ShimmerText::new(text).into_any_element()
            }
            "Marker" => {
                let variant = match self.prop_text(node, "variant").as_deref() {
                    Some("separator") | Some("Separator") => MarkerVariant::Separator,
                    Some("border") | Some("Border") => MarkerVariant::Border,
                    _ => MarkerVariant::Plain,
                };
                let mut marker = Marker::new()
                    .id(id)
                    .with_variant(variant)
                    .loading(self.prop_bool(node, "loading"));
                match self.prop_text(node, "alignment").as_deref() {
                    Some("center") | Some("Center") => {
                        marker = marker.alignment(MarkerAlignment::Center)
                    }
                    Some("end") | Some("End") => marker = marker.alignment(MarkerAlignment::End),
                    _ => {}
                }
                if let Some(text) = self
                    .prop_text(node, "text")
                    .or_else(|| node.content.as_ref().map(|v| self.text(v)))
                {
                    marker = marker.content(MarkerContent::new().text(text));
                }
                marker.into_any_element()
            }
            "Bubble" => {
                let kids = children(self, window, cx);
                let variant = match self.prop_text(node, "variant").as_deref() {
                    Some("secondary") | Some("Secondary") => BubbleVariant::Secondary,
                    Some("muted") | Some("Muted") => BubbleVariant::Muted,
                    Some("tinted") | Some("Tinted") => BubbleVariant::Tinted,
                    Some("outline") | Some("Outline") => BubbleVariant::Outline,
                    Some("ghost") | Some("Ghost") => BubbleVariant::Ghost,
                    Some("destructive") | Some("Destructive") => BubbleVariant::Destructive,
                    _ => BubbleVariant::Filled,
                };
                let mut bubble = Bubble::new().with_variant(variant);
                if matches!(
                    self.prop_text(node, "align").as_deref(),
                    Some("end") | Some("right") | Some("End")
                ) {
                    bubble = bubble.alignment(MessageAlignment::End);
                }
                bubble.children(kids).into_any_element()
            }
            "Notification" => {
                let message = self.prop_text(node, "message").unwrap_or_default();
                let mut note = match self.prop_text(node, "type").as_deref() {
                    Some("success") | Some("Success") => Notification::success(message.clone()),
                    Some("warning") | Some("Warning") => Notification::warning(message.clone()),
                    Some("error") | Some("Error") => Notification::error(message.clone()),
                    _ => Notification::info(message.clone()),
                };
                if let Some(title) = self.prop_text(node, "title") {
                    note = note.title(title);
                }
                if let Some(name) = self.prop_text(node, "icon") {
                    note = note.icon(self.doc_icon(&name, cx));
                }
                // `Notification` is a `Render` view, not an element — wrap it
                // in an entity so it joins the tree.
                cx.new(|_| note).into_any_element()
            }
            "Clipboard" => {
                let value = self
                    .prop_text(node, "value")
                    .or_else(|| node.content.as_ref().map(|v| self.text(v)))
                    .unwrap_or_default();
                Clipboard::new(id).value(value).into_any_element()
            }
            "Attachment" => {
                let title = self
                    .prop_text(node, "title")
                    .unwrap_or_else(|| "file".into());
                let mut content = AttachmentContent::new().title(AttachmentTitle::new(title));
                if let Some(description) = self.prop_text(node, "description") {
                    content = content.description(AttachmentDescription::new(description));
                }
                Attachment::new().content(content).into_any_element()
            }
            "Popover" => {
                // Deferred render: the content builder runs inside
                // `Context<PopoverState>` at open time. The body nodes go
                // through `try_update` to re-enter ForgeView and render live —
                // bindings inside the popover stay reactive.
                let trigger_kid = node.children.first().cloned();
                let body_nodes: Vec<Node> = node.children.iter().skip(1).cloned().collect();
                let weak_pop = weak.clone();
                let body_key = format!("{key}/popover");
                let mut popover = Popover::new(id);
                if let Some(anchor_name) = self.prop_text(node, "anchor") {
                    popover = popover.anchor(anchor(&anchor_name));
                }
                if let Some(trigger) = trigger_kid {
                    let els = self.expand(&trigger, &format!("{key}/trigger"), window, cx);
                    popover = popover.trigger(SelectableAny::new(
                        div().flex().children(els).into_any_element(),
                    ));
                }
                popover
                    .content(move |_, window, cx| {
                        try_update(&weak_pop, cx, "popover", |view, cx| {
                            let els: Vec<AnyElement> = body_nodes
                                .iter()
                                .enumerate()
                                .flat_map(|(i, n)| {
                                    view.expand(n, &format!("{body_key}/{i}"), window, cx)
                                })
                                .collect();
                            v_flex().gap_2().children(els)
                        })
                        .ok()
                        .unwrap_or_else(v_flex)
                    })
                    .into_any_element()
            }
            "HoverCard" => {
                let trigger_kid = node.children.first().cloned();
                let body_nodes: Vec<Node> = node.children.iter().skip(1).cloned().collect();
                let weak_card = weak.clone();
                let body_key = format!("{key}/hover");
                let mut card = HoverCard::new(id);
                if let Some(anchor_name) = self.prop_text(node, "anchor") {
                    card = card.anchor(anchor(&anchor_name));
                }
                if let Some(trigger) = trigger_kid {
                    let els = self.expand(&trigger, &format!("{key}/trigger"), window, cx);
                    card = card.trigger(div().flex().children(els));
                }
                card.content(move |_, window, cx| {
                    try_update(&weak_card, cx, "hover-card", |view, cx| {
                        let els: Vec<AnyElement> = body_nodes
                            .iter()
                            .enumerate()
                            .flat_map(|(i, n)| {
                                view.expand(n, &format!("{body_key}/{i}"), window, cx)
                            })
                            .collect();
                        v_flex().gap_2().children(els)
                    })
                    .ok()
                    .unwrap_or_else(v_flex)
                })
                .into_any_element()
            }
            "AlertDialog" => {
                let mut dialog = AlertDialog::new(cx);
                if self.prop_bool(node, "confirm") {
                    dialog = dialog.confirm();
                }
                if let Some(trigger) = node.children.first().cloned() {
                    let els = self.expand(&trigger, &format!("{key}/trigger"), window, cx);
                    dialog = dialog.trigger(div().flex().children(els));
                }
                if let Some(title) = self.prop_text(node, "title") {
                    dialog = dialog.title(title);
                }
                if let Some(ok) = self.prop_text(node, "ok-text") {
                    dialog = dialog.ok_text(ok);
                }
                if let Some(cancel) = self.prop_text(node, "cancel-text") {
                    dialog = dialog.cancel_text(cancel);
                }
                if let Some(description) = self
                    .prop_text(node, "description")
                    .or_else(|| node.content.as_ref().map(|v| self.text(v)))
                {
                    dialog = dialog.description(description);
                }
                let ok_node = node.clone();
                let weak_ok = weak.clone();
                dialog = dialog.on_ok(move |_, _, cx| {
                    try_update(&weak_ok, cx, "event", |view, cx| {
                        view.fire(&ok_node, "ok", cx)
                    })
                    .is_ok()
                });
                let cancel_node = node.clone();
                let weak_cancel = weak.clone();
                dialog = dialog.on_cancel(move |_, _, cx| {
                    try_update(&weak_cancel, cx, "event", |view, cx| {
                        view.fire(&cancel_node, "cancel", cx)
                    })
                    .is_ok()
                });
                dialog.into_any_element()
            }
            "DropdownButton" => {
                // Same shape as DropdownMenu — a button that opens a PopupMenu
                // built from `MenuItem`/`MenuSeparator` children.
                let mut mapped = node.clone();
                mapped.element.name = "DropdownMenu".into();
                return self.render_node(&mapped, key, window, cx);
            }
            "ContextMenu" => {
                let kids = children(self, window, cx);
                let items: Vec<(String, Option<String>, bool, bool, Node)> = node
                    .children
                    .iter()
                    .filter(|c| {
                        matches!(
                            c.element.name.as_str(),
                            "MenuItem" | "MenuSeparator" | "ContextMenuItem"
                        )
                    })
                    .map(|c| {
                        let label = self
                            .prop_text(c, "label")
                            .or_else(|| c.content.as_ref().map(|v| self.text(v)))
                            .unwrap_or_default();
                        (
                            label,
                            self.prop_text(c, "icon"),
                            self.prop_bool(c, "disabled"),
                            self.prop_bool(c, "checked"),
                            c.clone(),
                        )
                    })
                    .collect();
                let weak_menu = weak.clone();
                let iconset = self.iconset().to_owned();
                div()
                    .flex()
                    .id(id)
                    .children(kids)
                    .context_menu(move |menu, _, cx| {
                        let mut menu = menu;
                        for (label, icon_name, disabled, checked, child) in &items {
                            if child.element.name.as_str() == "MenuSeparator" {
                                menu = menu.separator();
                                continue;
                            }
                            let mut item = PopupMenuItem::new(label.clone());
                            if let Some(name) = icon_name {
                                item = item.icon(doc_icon_for(&iconset, name, cx));
                            }
                            item = item.disabled(*disabled).checked(*checked);
                            let weak = weak_menu.clone();
                            let child = child.clone();
                            item = item.on_click(move |_, _, cx| {
                                try_update(&weak, cx, "event", |this, cx| {
                                    this.fire(&child, "click", cx)
                                })
                                .ok();
                            });
                            menu = menu.item(item);
                        }
                        menu
                    })
                    .into_any_element()
            }
            "Combobox" => {
                let options: Vec<SharedString> = self
                    .prop(node, "options")
                    .map(|v| self.value_json(v))
                    .and_then(|v| v.as_array().cloned())
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|v| v.as_str().map(SharedString::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let placeholder = self.prop_text(node, "placeholder");
                let skey = format!("{key}|{}", bind_salt(node));
                let state = match self.comboboxes.get(&skey) {
                    Some(state) => state.clone(),
                    None => {
                        let state = cx
                            .new(|cx| ComboboxState::new(options.clone(), Vec::new(), window, cx));
                        let path = self.two_way(node, "value");
                        let changed = node.clone();
                        let subscription = cx.subscribe(&state, move |this, _state, event, cx| {
                            let values = match event {
                                ComboboxEvent::Change(values) | ComboboxEvent::Confirm(values) => {
                                    values
                                }
                            };
                            if let Some(path) = &path {
                                this.write(
                                    path,
                                    Json::Array(
                                        values
                                            .iter()
                                            .map(|v| Json::String(v.to_string()))
                                            .collect(),
                                    ),
                                    cx,
                                );
                            }
                            this.fire(&changed, "change", cx);
                        });
                        self.subscriptions.push(subscription);
                        self.comboboxes.insert(skey.clone(), state.clone());
                        state
                    }
                };
                Combobox::new(&state)
                    .when_some(placeholder, |c, p| c.placeholder(p))
                    .into_any_element()
            }
            "Carousel" => {
                let count = node.children.len();
                let vertical = self.prop_bool(node, "vertical");
                let item_width = self
                    .prop(node, "item-width")
                    .and_then(|v| self.number(v))
                    .map(|n| px(n as f32));
                let skey = format!("{key}|{}", bind_salt(node));
                let state = match self.carousels.get(&skey) {
                    Some(state) => state.clone(),
                    None => {
                        let state = cx.new(|_| {
                            CarouselState::new(count).with_axis(if vertical {
                                Axis::Vertical
                            } else {
                                Axis::Horizontal
                            })
                        });
                        self.carousels.insert(skey.clone(), state.clone());
                        state
                    }
                };
                state.update(cx, |state, cx| state.set_item_count(count, cx));
                let mut items = Vec::new();
                for (i, child) in node.children.iter().enumerate() {
                    for el in self.expand(child, &format!("{key}/{i}"), window, cx) {
                        let item =
                            CarouselItem::new(SharedString::from(format!("{id}.{i}")), i, &state)
                                .child(el);
                        items.push(match item_width {
                            Some(width) => item.w(width),
                            None => item,
                        });
                    }
                }
                let mut carousel =
                    Carousel::new(id, &state).child(CarouselContent::new(&state).children(items));
                if self.prop_bool(node, "controls") {
                    carousel = carousel
                        .child(CarouselPrevious::new(&state))
                        .child(CarouselNext::new(&state));
                }
                if self.prop_bool(node, "pagination") {
                    carousel = carousel.child(CarouselPagination::new());
                }
                carousel.into_any_element()
            }
            "Diff" => {
                let patch = self
                    .prop_text(node, "patch")
                    .or_else(|| node.content.as_ref().map(|v| self.text(v)))
                    .unwrap_or_default();
                let entry = self.diffs.entry(key.to_owned()).or_insert_with(|| {
                    (String::new(), cx.new(|cx| DiffState::new(Vec::new(), cx)))
                });
                if entry.0 != patch {
                    entry.1.update(cx, |state, cx| {
                        state.set_files(DiffFile::parse(&patch).unwrap_or_default(), cx)
                    });
                    entry.0 = patch.clone();
                }
                let state = entry.1.clone();
                Diff::new(&state)
                    .line_number(!matches!(
                        self.prop(node, "line-numbers"),
                        Some(Value::Bool(false))
                    ))
                    .into_any_element()
            }
            "Command" => {
                let skey = format!("{key}|{}", bind_salt(node));
                let state = match self.commands.get(&skey) {
                    Some(state) => state.clone(),
                    None => {
                        let state = cx.new(|cx| CommandState::new(window, cx));
                        self.commands.insert(skey.clone(), state.clone());
                        state
                    }
                };
                let mut command_item = |this: &Self, child: &Node| {
                    let label = this
                        .prop_text(child, "label")
                        .or_else(|| child.content.as_ref().map(|v| this.text(v)))
                        .unwrap_or_default();
                    let mut item = CommandItem::new().label(label);
                    if let Some(name) = this.prop_text(child, "icon") {
                        item = item.icon(this.doc_icon(&name, cx));
                    }
                    item
                };
                let mut command = Command::new(&state);
                for child in &node.children {
                    match child.element.name.as_str() {
                        "CommandItem" | "MenuItem" => {
                            command = command.item(command_item(self, child));
                        }
                        "CommandGroup" => {
                            let mut group = CommandGroup::new();
                            if let Some(t) = self.prop_text(child, "title") {
                                group = group.label(t);
                            }
                            group = group.items(
                                child
                                    .children
                                    .iter()
                                    .filter(|c| c.element.name == "CommandItem")
                                    .map(|c| command_item(self, c)),
                            );
                            command = command.group(group);
                        }
                        "CommandSeparator" | "MenuSeparator" => {
                            command = command.separator();
                        }
                        _ => {}
                    }
                }
                if let Some(placeholder) = self.prop_text(node, "placeholder") {
                    command = command.placeholder(placeholder);
                }
                let select_node = node.clone();
                let weak_sel = weak.clone();
                command
                    .on_select(move |_, _, cx| {
                        try_update(&weak_sel, cx, "event", |this, cx| {
                            this.fire(&select_node, "select", cx)
                        })
                        .ok();
                    })
                    .into_any_element()
            }
            "Questionnaire" => {
                let items: Vec<QuestionnaireItemDefinition> = node
                    .children
                    .iter()
                    .filter(|c| c.element.name == "QuestionnaireItem")
                    .enumerate()
                    .map(|(i, child)| {
                        let name = self
                            .prop_text(child, "name")
                            .unwrap_or_else(|| format!("item-{i}"));
                        let label = self
                            .prop_text(child, "title")
                            .or_else(|| child.content.as_ref().map(|v| self.text(v)))
                            .unwrap_or_else(|| name.clone());
                        let mut def = QuestionnaireItemDefinition::new(name, label)
                            .with_required(self.prop_bool(child, "required"))
                            .with_multiple(self.prop_bool(child, "multiple"));
                        if let Some(d) = self.prop_text(child, "description") {
                            def = def.with_description(d);
                        }
                        for option in &child.children {
                            if option.element.name != "QuestionnaireChoice" {
                                continue;
                            }
                            let value = self
                                .prop_text(option, "value")
                                .or_else(|| self.prop_text(option, "label"))
                                .or_else(|| option.content.as_ref().map(|v| self.text(v)))
                                .unwrap_or_default();
                            let choice_label = self
                                .prop_text(option, "label")
                                .unwrap_or_else(|| value.clone());
                            let mut choice =
                                QuestionnaireChoiceDefinition::new(value, choice_label)
                                    .with_default_selected(self.prop_bool(option, "selected"));
                            if let Some(d) = self.prop_text(option, "description") {
                                choice = choice.with_description(d);
                            }
                            def = def.with_choice(choice);
                        }
                        def
                    })
                    .collect();
                let skey = format!("{key}|{}", bind_salt(node));
                let state = match self.questionnaires.get(&skey) {
                    Some(state) => state.clone(),
                    None => {
                        let state = cx.new(|cx| {
                            QuestionnaireState::new(items.clone(), cx).unwrap_or_else(|_| {
                                QuestionnaireState::new(
                                    vec![QuestionnaireItemDefinition::new("_", "_")],
                                    cx,
                                )
                                .unwrap_or_else(|_| unreachable!())
                            })
                        });
                        self.questionnaires.insert(skey.clone(), state.clone());
                        state
                    }
                };
                Questionnaire::new(&state).into_any_element()
            }
            "LineChart" | "BarChart" | "PieChart" => {
                // `data:` is a plain `[number]` series — Line/Bar get
                // (index, value) pairs, Pie consumes the value directly.
                let data: Vec<f64> = self
                    .prop(node, "data")
                    .map(|v| self.value_json(v))
                    .and_then(|v| v.as_array().cloned())
                    .map(|items| items.iter().filter_map(|v| v.as_f64()).collect())
                    .unwrap_or_default();
                let pairs: Vec<(SharedString, f64)> = data
                    .iter()
                    .enumerate()
                    .map(|(i, v)| (SharedString::from(i.to_string()), *v))
                    .collect();
                match node.element.name.as_str() {
                    "BarChart" => BarChart::new(pairs)
                        .band(|p| p.0.clone())
                        .value(|p| p.1)
                        .into_any_element(),
                    "PieChart" => PieChart::new(data).value(|v| *v as f32).into_any_element(),
                    _ => LineChart::new(pairs)
                        .x(|p| p.0.clone())
                        .y(|p| p.1)
                        .into_any_element(),
                }
            }
            "RadarChart" => {
                let data: Vec<f64> = self
                    .prop(node, "data")
                    .map(|v| self.value_json(v))
                    .and_then(|v| v.as_array().cloned())
                    .map(|items| items.iter().filter_map(|v| v.as_f64()).collect())
                    .unwrap_or_default();
                let pairs: Vec<(SharedString, f64)> = data
                    .iter()
                    .enumerate()
                    .map(|(i, v)| (SharedString::from(i.to_string()), *v))
                    .collect();
                RadarChart::new(pairs)
                    .label(|p| p.0.clone())
                    .value(|p| p.1)
                    .into_any_element()
            }
            "SankeyChart" => {
                // `nodes:` labels, `links:` "source,target,value" entries.
                let nodes: Vec<SharedString> = self
                    .prop(node, "nodes")
                    .map(|v| self.value_json(v))
                    .and_then(|v| v.as_array().cloned())
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|v| v.as_str().map(SharedString::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let links: Vec<SankeyLink> = self
                    .prop(node, "links")
                    .map(|v| self.value_json(v))
                    .and_then(|v| v.as_array().cloned())
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|v| v.as_str())
                            .filter_map(|s| {
                                let mut p = s.split(',');
                                Some(SankeyLink::new(
                                    p.next()?.parse().ok()?,
                                    p.next()?.parse().ok()?,
                                    p.next()?.parse().ok()?,
                                ))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                SankeyChart::new(nodes, links)
                    .node_label(|n| n.clone())
                    .value_label(|_n, v| format!("{v}").into())
                    .into_any_element()
            }
            "WindowBorder" => {
                let mut b = gpui_kit::component::WindowBorder::new();
                if let Some(s) = self.prop(node, "shadow").and_then(|v| self.number(v)) {
                    b = b.shadow_size(px(s as f32));
                }
                if let Some(r) = self.prop(node, "resize-hit").and_then(|v| self.number(v)) {
                    b = b.resize_hit_size(px(r as f32));
                }
                b.extend(
                    node.children
                        .iter()
                        .enumerate()
                        .map(|(i, c)| self.render_node(c, &format!("{key}/{i}"), window, cx)),
                );
                b.into_any_element()
            }
            "InputGroup" => {
                // Slot-based: input children feed `.input()`, addons `.addon()`.
                let mut group = InputGroup::new(id.clone())
                    .disabled(self.prop_bool(node, "disabled"))
                    .invalid(self.prop_bool(node, "invalid"));
                for (i, child) in node.children.iter().enumerate() {
                    let ckey = format!("{key}/{i}");
                    match child.element.name.as_str() {
                        "Input" | "InputGroupInput" => {
                            let state = self.input_state(child, &ckey, window, cx);
                            group = group.input(Input::new(&state));
                        }
                        "Textarea" | "InputGroupTextarea" => {
                            let state = self.textarea_state(child, &ckey, window, cx);
                            group = group.input(Textarea::new(&state));
                        }
                        "InputGroupAddon" => {
                            let alignment = match self.prop_text(child, "align").as_deref() {
                                Some("end") | Some("InlineEnd") => {
                                    InputGroupAddonAlignment::InlineEnd
                                }
                                Some("block-start") | Some("BlockStart") => {
                                    InputGroupAddonAlignment::BlockStart
                                }
                                Some("block-end") | Some("BlockEnd") => {
                                    InputGroupAddonAlignment::BlockEnd
                                }
                                _ => InputGroupAddonAlignment::InlineStart,
                            };
                            let kids = self.expand(child, &ckey, window, cx);
                            group = group.addon(
                                InputGroupAddon::new(SharedString::from(format!("{id}.{i}")))
                                    .align(alignment)
                                    .children(kids),
                            );
                        }
                        _ => {}
                    }
                }
                group.into_any_element()
            }
            "InputGroupAddon" => {
                let alignment = match self.prop_text(node, "align").as_deref() {
                    Some("end") | Some("InlineEnd") => InputGroupAddonAlignment::InlineEnd,
                    Some("block-start") | Some("BlockStart") => {
                        InputGroupAddonAlignment::BlockStart
                    }
                    Some("block-end") | Some("BlockEnd") => InputGroupAddonAlignment::BlockEnd,
                    _ => InputGroupAddonAlignment::InlineStart,
                };
                InputGroupAddon::new(id)
                    .align(alignment)
                    .children(children(self, window, cx))
                    .into_any_element()
            }
            "InputGroupText" => {
                let mut group = InputGroupText::new();
                if let Some(text) = self
                    .prop_text(node, "text")
                    .or_else(|| node.content.as_ref().map(|v| self.text(v)))
                {
                    group = group.child(text);
                }
                group
                    .children(children(self, window, cx))
                    .into_any_element()
            }
            "InputGroupButton" => {
                let mut button = InputGroupButton::new(id);
                if let Some(name) = self.prop_text(node, "icon") {
                    button = button.icon(self.doc_icon(&name, cx));
                }
                if let Some(label) = self.prop_text(node, "label") {
                    button = button.label(label);
                }
                let clicked = node.clone();
                let weak_btn = weak.clone();
                button
                    .on_click(move |_, _, cx| {
                        try_update(&weak_btn, cx, "event", |this, cx| {
                            this.fire(&clicked, "click", cx)
                        })
                        .ok();
                    })
                    .into_any_element()
            }
            "InputGroupInput" => {
                let mut mapped = node.clone();
                mapped.element.name = "Input".into();
                return self.render_node(&mapped, key, window, cx);
            }
            "InputGroupTextarea" => {
                let mut mapped = node.clone();
                mapped.element.name = "Textarea".into();
                return self.render_node(&mapped, key, window, cx);
            }
            "FocusTrap" => {
                let kids = children(self, window, cx);
                let handle = cx.focus_handle();
                v_flex()
                    .id(id.clone())
                    .children(kids)
                    .focus_trap(SharedString::from(format!("{id}-trap")), &handle)
                    .into_any_element()
            }
            other => {
                // #145 — compat canvas: an element name the kit catalog doesn't
                // know maps through the `canvas:` analogue the document's base
                // catalog declares. The mapped node keeps path/id, so
                // selection, outline and DnD edits hit the real node.
                let analogue = self
                    .doc
                    .as_ref()
                    .and_then(|d| d.base.as_deref())
                    .and_then(|base| self.catalogs.iter().find(|c| c.library == base))
                    .and_then(|cat| cat.element(other))
                    .and_then(|def| def.canvas.clone());
                match analogue {
                    Some(map) => {
                        let mut mapped = node.clone();
                        mapped.element.name = map.element;
                        for (k, _) in mapped.props.iter_mut() {
                            if let Some(to) = map.props.get(k.as_str()) {
                                *k = to.clone();
                            }
                        }
                        return self.render_node(&mapped, key, window, cx);
                    }
                    None => placeholder(format!("`{other}` is not in the gpui-kit catalog"), cx),
                }
            }
        };
        let element = if styled {
            element
        } else {
            wrap(
                element,
                &node.style,
                &format!("forge-{key}"),
                self.prop_text(node, "tooltip"),
                &mut self.scroll_handles,
                cx,
            )
        };
        let element = if std::env::var_os("FORGE_BOUNDS").is_some() {
            BoundsLogger::new(element, format!("el-{key}")).into_any_element()
        } else {
            element
        };
        let path = key.replacen("root", "0", 1).replace('/', ".");
        let element = if self.pick {
            // Element-picker. Each node records its rect into `pick_zones`.
            // The single top-level pick overlay can then resolve the deepest
            // node under the pointer. Per-node click overlays cannot — the
            // root's shadows every descendant in dispatch order. The zone
            // canvas also paints the accent frame while it is `pick_hover`.
            let zones = self.pick_zones.clone();
            let hover = self.pick_hover.clone();
            let accent = cx.theme().accent;
            let zone_path = path.clone();
            let frame_path = path.clone();
            // The pick wrapper takes the node's slot in the flex parent, so
            // it needs the node's flex geometry or `grow` nodes collapse.
            flex_item(
                div().relative().child(element).child(
                    canvas(
                        move |bounds, _window, _cx| {
                            let mut zones = zones.borrow_mut();
                            match zones.iter_mut().find(|(_, p)| *p == zone_path) {
                                Some(entry) => entry.0 = bounds,
                                None => zones.push((bounds, zone_path.clone())),
                            }
                        },
                        move |bounds, (), window, _cx| {
                            if hover.borrow().as_deref() == Some(frame_path.as_str()) {
                                window.paint_quad(fill(bounds, accent.alpha(0.15)));
                                window.paint_quad(outline(bounds, accent, BorderStyle::Solid));
                            }
                        },
                    )
                    .absolute()
                    .inset_0(),
                ),
                &node.style,
                cx,
            )
            .into_any_element()
        } else {
            // Selection highlight — the outline/inspector selection and the
            // DnD drop target get a non-interactive accent frame. Only the
            // canvas view decorates nodes. In the Studio shell the nested
            // document's paths collide with outer nodes — `0.1.0` is the
            // outer Resizable — and a frame there resizes the chrome.
            let selected = self.dnd
                && get_path(self.state(), &parse_path("document.selected").unwrap())
                    .and_then(|j| j.as_str())
                    .map(str::to_owned)
                    == Some(path.clone());
            let drop_target = self.dnd
                && get_path(self.state(), &parse_path("studio.drop_target").unwrap())
                    .and_then(|j| j.as_str())
                    .map(str::to_owned)
                    == Some(path.clone());
            if selected || drop_target {
                let accent = cx.theme().accent;
                // A leaf's drop lands as the next sibling — a bottom bar.
                // A container's nests inside — a full frame.
                let into = self.catalogs.iter().any(|c| {
                    c.element(&node.element.name)
                        .is_some_and(|d| !matches!(d.children, Children::None))
                });
                let overlay = if drop_target && !into {
                    div()
                        .absolute()
                        .inset_0()
                        .border_b_2()
                        .border_color(accent)
                        .into_any_element()
                } else {
                    div()
                        .absolute()
                        .inset_0()
                        .border_2()
                        .border_color(accent)
                        .into_any_element()
                };
                flex_item(
                    div().relative().child(element).child(overlay),
                    &node.style,
                    cx,
                )
                .into_any_element()
            } else {
                element
            }
        };
        // Drag & drop — palette items (`insert_component` handlers) drag a
        // component tag. The preview also drags nodes (reparent) and accepts
        // drops at `studio.drop_target`.
        let tag = component_tag(node);
        if tag.is_none() && !self.dnd {
            return element;
        }
        let weak = cx.entity().downgrade();
        let mut dnd = div().id(format!("dnd-{key}")).child(element);
        if let Some(tag) = tag {
            let ghost: SharedString = tag.clone().into();
            dnd = dnd.on_drag(ForgeDrag::Component(tag), move |_, _, _, cx| {
                cx.new(|_| DragGhost(ghost.clone()))
            });
        } else {
            let ghost: SharedString = node.element.name.clone().into();
            dnd = dnd.on_drag(ForgeDrag::Node(path.clone()), move |_, _, _, cx| {
                cx.new(|_| DragGhost(ghost.clone()))
            });
        }
        if self.dnd {
            let move_path = path.clone();
            let move_weak = weak.clone();
            dnd = dnd
                .on_drag_move::<ForgeDrag>(move |_, _, cx| {
                    try_update(&move_weak, cx, "drag-move", |p, cx| {
                        p.write(
                            &parse_path("studio.drop_target").unwrap(),
                            Json::String(move_path.clone()),
                            cx,
                        );
                    })
                    .ok();
                })
                .on_drop::<ForgeDrag>(move |drag, _, cx| {
                    try_update(&weak, cx, "drop", |p, cx| {
                        p.write(
                            &parse_path("studio.drop_target").unwrap(),
                            Json::String(path.clone()),
                            cx,
                        );
                        match drag {
                            ForgeDrag::Component(tag) => {
                                p.fire_call("drop_component", vec![tag.clone(), path.clone()], cx)
                            }
                            ForgeDrag::Node(src) => {
                                p.fire_call("move_node", vec![src.clone(), path.clone()], cx)
                            }
                        }
                        cx.notify();
                    })
                    .ok();
                });
        }
        dnd.into_any_element()
    }
}

/// `id → expanded` for every item a `TreeState` holds — roots and descendants
/// alike (collapsed subtrees live in their parent's `children`, not entries).
fn tree_expansion(state: &TreeState) -> HashMap<SharedString, bool> {
    fn walk(item: &TreeItem, map: &mut HashMap<SharedString, bool>) {
        map.insert(item.id.clone(), item.is_expanded());
        for child in &item.children {
            walk(child, map);
        }
    }
    let mut map = HashMap::new();
    for ix in 0.. {
        match state.entry(ix) {
            Some(entry) => walk(entry.item(), &mut map),
            None => break,
        }
    }
    map
}

/// Applies a prior `tree_expansion` map to freshly built items by id, so a
/// rebuild doesn't snap every folder back open. `TreeItem::expanded` consumes,
/// but the flag lives in a shared `Rc` cell — a clone writes it in place.
fn carry_expansion(items: &mut [TreeItem], previous: &HashMap<SharedString, bool>) {
    for item in items {
        if let Some(&expanded) = previous.get(&item.id) {
            item.clone().expanded(expanded);
        }
        carry_expansion(&mut item.children, previous);
    }
}

/// `PropType` → the `type` string inspector items carry — the same token
/// `forge.InspectorField` dispatches on.
fn prop_type_name(ty: &PropType) -> &'static str {
    match ty {
        PropType::Bool => "bool",
        PropType::Enum(_) => "enum",
        PropType::Number => "number",
        PropType::Length => "length",
        PropType::Color => "color",
        PropType::Icon => "icon",
        PropType::List(_) => "list",
        PropType::Map => "map",
        PropType::String => "string",
    }
}

/// The scrolling fill column inspector tab bodies use — `min-height: 0`
/// lets it shrink below content height so `overflow: scroll` engages.
fn inspector_scroll_style() -> Entries {
    vec![
        ("grow".into(), Value::Bool(true)),
        (
            "min-height".into(),
            Value::Number {
                value: 0.0,
                unit: None,
            },
        ),
        ("overflow".into(), Value::Enum("scroll".into())),
    ]
}

/// Every style key the inspector lists. Set entries first in `style:`
/// order, then the rest of `KEYS`, so clearing a value (`[none]`, an empty
/// input) removes the entry without its row disappearing.
/// Rows in stable registry order — set-first ordering made a row teleport
/// mid-typing when its key became set (and churned every positional entity
/// key). Unregistered style keys keep document order at the tail.
fn style_rows(set: &Entries) -> Vec<(String, Option<Value>)> {
    let mut rows: Vec<(String, Option<Value>)> = forge_spec::style::KEYS
        .iter()
        .map(|(key, _)| {
            (
                (*key).to_owned(),
                set.iter()
                    .find(|(name, _)| name == key)
                    .map(|(_, value)| value.clone()),
            )
        })
        .collect();
    for (name, value) in set {
        if forge_spec::style::key_type(name).is_none() {
            rows.push((name.clone(), Some(value.clone())));
        }
    }
    rows
}

/// The zero value a `state:` declaration seeds — `string`→`""`, `bool`→
/// `false`, `number`/`slider`→`0`, `list`→`[]`, `object`/`map`→`{}`,
/// `color`→`"#000000"`, `enum(a | b)`→first variant, anything else `null`.
fn state_default(ty: &str) -> Json {
    match ty {
        "string" => Json::String(String::new()),
        "bool" => Json::Bool(false),
        "number" | "slider" => Json::from(0),
        "list" => Json::Array(Vec::new()),
        "object" | "map" => Json::Object(Default::default()),
        "color" => Json::String("#000000".into()),
        t if t.starts_with("enum") => t
            .strip_prefix("enum")
            .and_then(|s| s.strip_prefix('('))
            .and_then(|s| s.strip_suffix(')'))
            .and_then(|s| s.split('|').next())
            .map(|v| Json::String(v.trim().to_owned()))
            .unwrap_or(Json::Null),
        _ => Json::Null,
    }
}

/// What an unset style key seeds `document.edits.style.<key>` with —
/// `[none]` for keyword styles (the Select's clear option), else `null`.
fn unset_style_seed(key: &str) -> Json {
    use forge_spec::style::{StyleType, key_type};
    match key_type(key) {
        Some(StyleType::Keyword(_)) | Some(StyleType::LengthOrKeyword(_)) => {
            Json::String("[none]".into())
        }
        _ => Json::Null,
    }
}

/// A style key's `type`/`options` for `forge.Inspector` items and the
/// built-in layout — keyword styles become Selects led by `[none]` (the
/// clear sentinel `style_edit_value` maps to removal). A current value
/// outside the keyword list is kept selectable as the second option.
fn style_item_type(name: &str, value: &Json) -> (&'static str, Vec<String>) {
    use forge_spec::style::{StyleType, key_type};
    let words = match key_type(name) {
        Some(StyleType::Keyword(words)) | Some(StyleType::LengthOrKeyword(words)) => words,
        Some(StyleType::Bool) => return ("bool", vec![]),
        Some(StyleType::Factor) | Some(StyleType::Fraction) => return ("number", vec![]),
        _ => return ("string", vec![]),
    };
    let mut options: Vec<String> = vec!["[none]".to_owned()];
    options.extend(words.iter().map(|w| (*w).to_owned()));
    if let Some(current) = value.as_str()
        && !current.is_empty()
        && current != "[none]"
        && !words.contains(&current)
    {
        options.insert(1, current.to_owned());
    }
    ("enum", options)
}

/// The control node one inspector row renders — `kind` picks the
/// `document.edits.*` namespace and `*_changed` event, `type` the widget.
/// Shared by `forge.Inspector`'s built-in layout and `forge.InspectorField`.
fn inspector_control(kind: &str, name: &str, ty: &str, options: Vec<String>) -> Option<Node> {
    let (edit_path, event, args) = match kind {
        "ref" => (
            "document.edits.refname".to_owned(),
            "refname_changed".to_owned(),
            vec![],
        ),
        "style" => (
            format!("document.edits.style.{name}"),
            "style_changed".to_owned(),
            vec![Value::String(name.to_owned())],
        ),
        _ => (
            format!("document.edits.{name}"),
            "prop_changed".to_owned(),
            vec![Value::String(name.to_owned())],
        ),
    };
    let path = parse_path(&edit_path).ok()?;
    // `kind` picks the edit namespace/event (above). `ty` alone picks the
    // widget so `style` rows get the same typed controls as props.
    let (element, bind_prop, extra) = match ty {
        "bool" => ("Switch", "checked", vec![]),
        "enum" => (
            "Select",
            "value",
            vec![(
                "options".to_owned(),
                Value::List(options.into_iter().map(Value::Enum).collect()),
            )],
        ),
        "number" => ("NumberInput", "value", vec![]),
        "color" => ("ColorPicker", "value", vec![]),
        _ => ("Input", "value", vec![]),
    };
    let mut props = vec![(
        bind_prop.to_owned(),
        Value::Binding {
            path,
            mode: BindingMode::TwoWay,
        },
    )];
    props.extend(extra);
    if kind == "ref" {
        props.push((
            "placeholder".to_owned(),
            Value::String("ref block name".into()),
        ));
    }
    Some(Node {
        element: ElementRef {
            namespace: None,
            name: element.into(),
        },
        props,
        events: vec![EventBinding {
            event: "change".into(),
            handler: HandlerRef { name: event, args },
        }],
        ..Default::default()
    })
}

/// Truthiness of a resolved state `Json` — `if:` with no `is:`/`not:` test.
/// State path for a remote pane's streamed output — a pane `id` can carry
/// hyphens (`preview-iced`) that binding paths don't parse, so the state
/// segment sanitizes to `preview.panes.<id>.<key>`.
pub fn pane_state_path(id: &str, key: &str) -> Path {
    let safe: String = id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    parse_path(&format!("preview.panes.{safe}.{key}")).unwrap_or_else(|_| Path(vec![]))
}

fn json_truthy(value: &Json) -> bool {
    match value {
        Json::Bool(b) => *b,
        Json::Null => false,
        Json::String(s) => !s.is_empty(),
        Json::Number(n) => n.as_f64().is_some_and(|v| v != 0.0),
        _ => true,
    }
}

/// A state `Json` as a spec `Value` literal — bakes scoped vars into event args.
fn literal(json: Json) -> Value {
    match json {
        Json::Bool(b) => Value::Bool(b),
        Json::Number(n) => Value::Number {
            value: n.as_f64().unwrap_or_default(),
            unit: None,
        },
        Json::String(s) => Value::String(s),
        Json::Null => Value::String(String::new()),
        Json::Array(items) => Value::List(items.into_iter().map(literal).collect()),
        Json::Object(map) => Value::Map(map.into_iter().map(|(k, v)| (k, literal(v))).collect()),
    }
}

/// The tag a palette item inserts — the `insert_component('Tag')` arg on the
/// node's click handler doubles as its drag payload.
fn component_tag(node: &Node) -> Option<String> {
    node.events
        .iter()
        .find(|e| e.handler.name == "insert_component")
        .and_then(|e| e.handler.args.first())
        .and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        })
}

impl Render for ForgeView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let (background, foreground) = (theme.background, theme.foreground);
        let content = self
            .doc
            .clone()
            .map(|doc| {
                if doc.root.control.is_some() {
                    div()
                        .size_full()
                        .children(self.expand(&doc.root, "root", window, cx))
                        .into_any_element()
                } else {
                    // A doc rendered at root fills its surface like an app
                    // window. `height: full` resolves against the element's
                    // own slot, so a root with no size key still collapses to
                    // content height inside `forge.Preview` cards — the
                    // "detached" shrink users saw. `grow` claims the column's
                    // free space instead. `#forge-canvas` is `flex_col` so the
                    // claim has space to take.
                    let mut root = doc.root.clone();
                    if !root.style.iter().any(|(k, _)| {
                        matches!(
                            k.as_str(),
                            "grow" | "basis" | "height" | "min-height" | "max-height" | "size"
                        )
                    }) {
                        root.style.push(("grow".into(), Value::Bool(true)));
                    }
                    self.render_node(&root, "root", window, cx)
                }
            })
            .or_else(|| self.empty_text.clone().map(|text| empty(text, cx)));
        for message in self.take_errors() {
            window.push_notification((NotificationType::Error, message), cx);
        }
        let weak = cx.entity().downgrade();
        let pick_weak = weak.clone();
        let save_weak = weak.clone();
        let root = div()
            .id("forge-canvas")
            .size_full()
            .flex()
            .flex_col()
            .bg(background)
            .text_color(foreground)
            .on_action::<CancelPick>(move |_, _, cx| {
                try_update(&weak, cx, "cancel-place", |p, cx| {
                    if p.pick {
                        p.pick = false;
                        p.placing = false;
                        p.fire_call("cancel_place", vec![], cx);
                    }
                    cx.notify();
                })
                .ok();
            })
            .on_action::<SaveDocument>(move |_, _, cx| {
                try_update(&save_weak, cx, "save", |p, cx| {
                    p.fire_call("save_document", vec![], cx);
                })
                .ok();
            })
            .children(content);
        if !self.pick {
            return root;
        }
        // One overlay over the whole canvas — topmost, so its swallowed
        // mousedown keeps every element below inert (clicks, row selects,
        // toggles, drag starts). Selection resolves the deepest node by
        // hit-testing `pick_zones` rather than relying on listener order.
        self.pick_zones.borrow_mut().clear();
        let zones_move = self.pick_zones.clone();
        let zones_click = self.pick_zones.clone();
        let hover = self.pick_hover.clone();
        let click_weak = pick_weak.clone();
        let cancel_weak = pick_weak;
        root.relative().child(
            div()
                .id("pick-overlay")
                .absolute()
                .inset_0()
                .cursor_pointer()
                .occlude()
                .on_mouse_move(move |event, window, _cx| {
                    // Zones are pushed deepest-first (each node's canvas
                    // prepaints after its descendants), so the first
                    // containing zone is the deepest node under the pointer.
                    let hit = zones_move
                        .borrow()
                        .iter()
                        .find(|(bounds, _)| bounds.contains(&event.position))
                        .map(|(_, path)| path.clone());
                    let mut hover = hover.borrow_mut();
                    if *hover != hit {
                        *hover = hit;
                        window.refresh();
                    }
                })
                .on_click(move |event, _, cx| {
                    let ClickEvent::Mouse(click) = event else {
                        return;
                    };
                    let Some(path) = zones_click
                        .borrow()
                        .iter()
                        .find(|(bounds, _)| bounds.contains(&click.up.position))
                        .map(|(_, path)| path.clone())
                    else {
                        return;
                    };
                    try_update(&click_weak, cx, "pick", |p, cx| {
                        p.pick = false;
                        if p.placing {
                            p.placing = false;
                            p.write(
                                &parse_path("studio.drop_target").unwrap(),
                                Json::String(path.clone()),
                                cx,
                            );
                            p.fire_call("place_component", vec![path.clone()], cx);
                        } else {
                            p.write(
                                &parse_path("document.selected").unwrap(),
                                Json::String(path.clone()),
                                cx,
                            );
                        }
                        cx.notify();
                    })
                    .ok();
                })
                .on_mouse_down(MouseButton::Left, |_, _, cx| {
                    cx.stop_propagation();
                })
                .on_mouse_down(MouseButton::Right, move |_, _, cx| {
                    try_update(&cancel_weak, cx, "cancel-place", |p, cx| {
                        p.pick = false;
                        p.placing = false;
                        p.fire_call("cancel_place", vec![], cx);
                        cx.notify();
                    })
                    .ok();
                }),
        )
    }
}

/// A sized wrapper `open_dialog` puts around a `Dialog#id { resizable: true }`
/// view: the declaration's `width`/`height` become the initial extent and a
/// bottom-right wedge grip drags the card larger or smaller.
///
/// Drag tracking registers `window.on_mouse_event` from a `canvas` paint
/// callback — `on_mouse_event` asserts the Paint draw phase, and `canvas`
/// paint is the only paint-phase hook an element tree gets. Window-level
/// listeners are needed because the pointer leaves the 16px grip within a
/// pixel or two of a drag. `resizable_panel` and scrollbars listen
/// window-wide for the same reason.
/// `Popover::trigger` requires `Selectable`. This wrapper satisfies the bound
/// for arbitrary rendered content. The flag is inert: the popover only uses it
/// for open-state styling, which Forge triggers leave to the child.
struct SelectableAny {
    element: AnyElement,
    selected: bool,
}

impl SelectableAny {
    fn new(element: AnyElement) -> Self {
        Self {
            element,
            selected: false,
        }
    }
}

impl Selectable for SelectableAny {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl IntoElement for SelectableAny {
    type Element = AnyElement;

    fn into_element(self) -> Self::Element {
        self.element
    }
}

/// A dialog-sized container with a bottom-right corner grip that drags the
/// overlay wider/taller. `open_dialog` wraps `Dialog#id` declarations that opt
/// in with `resizable: true`. The declared `width`/`height` become the initial
/// size and the inner root style is rewritten to `full` so content tracks the
/// drag.
pub struct ResizableDialog {
    child: Entity<ForgeView>,
    size: gpui::Size<Pixels>,
    min: gpui::Size<Pixels>,
    drag: Option<ResizeDrag>,
}

#[derive(Clone, Copy)]
struct ResizeDrag {
    pointer: Point<Pixels>,
    size: gpui::Size<Pixels>,
}

impl ResizableDialog {
    /// `size` is the dialog's initial extent. `min` is the smallest the grip
    /// will shrink it to.
    pub fn new(
        child: Entity<ForgeView>,
        size: gpui::Size<Pixels>,
        min: gpui::Size<Pixels>,
    ) -> Self {
        Self {
            child,
            size,
            min,
            drag: None,
        }
    }
}

impl Render for ResizableDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let wedge_color = cx.theme().muted_foreground;
        let weak = cx.entity().downgrade();
        div()
            .relative()
            .w(self.size.width)
            .h(self.size.height)
            .overflow_hidden()
            .child(self.child.clone())
            .child(
                div()
                    .id("forge-dialog-resize")
                    .absolute()
                    .right_0()
                    .bottom_0()
                    .w(px(16.))
                    .h(px(16.))
                    .cursor(CursorStyle::ResizeUpLeftDownRight)
                    .on_mouse_down(MouseButton::Left, {
                        let weak = weak.clone();
                        move |event: &MouseDownEvent, _window, cx| {
                            weak.update(cx, |this, cx| {
                                this.drag = Some(ResizeDrag {
                                    pointer: event.position,
                                    size: this.size,
                                });
                                cx.notify();
                            })
                            .ok();
                        }
                    }),
            )
            .child(
                canvas(
                    |_bounds, _window, _cx| {},
                    move |bounds, (), window, _cx| {
                        window.on_mouse_event({
                            let weak = weak.clone();
                            move |event: &MouseMoveEvent, phase, window, cx| {
                                if !phase.bubble() || !event.dragging() {
                                    return;
                                }
                                weak.update(cx, |this, cx| {
                                    let Some(drag) = this.drag else {
                                        return;
                                    };
                                    let viewport = window.viewport_size();
                                    let w = f32::from(
                                        drag.size.width + event.position.x - drag.pointer.x,
                                    );
                                    let h = f32::from(
                                        drag.size.height + event.position.y - drag.pointer.y,
                                    );
                                    this.size = gpui::size(
                                        px(w.clamp(
                                            f32::from(this.min.width),
                                            f32::from(viewport.width) - 24.,
                                        )),
                                        px(h.clamp(
                                            f32::from(this.min.height),
                                            f32::from(viewport.height) - 24.,
                                        )),
                                    );
                                    cx.notify();
                                })
                                .ok();
                            }
                        });
                        window.on_mouse_event({
                            let weak = weak.clone();
                            move |_event: &MouseUpEvent, phase, _window, cx| {
                                if phase.bubble() {
                                    weak.update(cx, |this, cx| {
                                        if this.drag.take().is_some() {
                                            cx.notify();
                                        }
                                    })
                                    .ok();
                                }
                            }
                        });
                        // Corner wedge affordance.
                        let right = bounds.right();
                        let bottom = bounds.bottom();
                        let mut wedge = gpui::Path::new(point(right - px(11.), bottom));
                        wedge.line_to(point(right, bottom - px(11.)));
                        wedge.line_to(point(right, bottom));
                        window.paint_path(wedge, wedge_color);
                    },
                )
                .absolute()
                .inset_0(),
            )
    }
}

/// Applies `style` to a container. `overflow: scroll` needs an element id, so a scrolling
/// container is stateful.
fn container(
    element: Div,
    entries: &Entries,
    id: SharedString,
    kids: Vec<AnyElement>,
    handles: &mut HashMap<String, ScrollHandle>,
    cx: &App,
) -> AnyElement {
    if scrollable(entries) {
        let handle = handles.entry(id.to_string()).or_default().clone();
        // `.id()` — `Scrollable` defaults `id` to `caller_id()`, so every
        // `overflow: scroll` element would share one `element_state` slot and
        // the offsets would clobber each other. A per-element id keeps them
        // distinct.
        let scrollbar_id = SharedString::from(format!("{id}-scrollbar"));
        style(element.id(id).track_scroll(&handle), entries, cx)
            .overflow_y_scrollbar()
            .id(scrollbar_id)
            .children(kids)
            .into_any_element()
    } else {
        let out = style(element, entries, cx)
            .children(kids)
            .into_any_element();
        if std::env::var_os("FORGE_BOUNDS").is_some() {
            BoundsLogger::new(out, format!("cont-{id}")).into_any_element()
        } else {
            out
        }
    }
}

/// Applies `style` to a leaf element: it goes on a wrapper `div` (scrollable when
/// `overflow: scroll`, which likewise needs an element id).
fn wrap(
    element: AnyElement,
    entries: &Entries,
    id: &str,
    tooltip: Option<String>,
    handles: &mut HashMap<String, ScrollHandle>,
    cx: &App,
) -> AnyElement {
    // Always wrap leaves in a block `div` — gpui-kit roots are
    // `size_full()` (Select, DatePicker, …). Bare in a flex column that
    // `height: 100%` steals the column's free space from siblings. Inside a
    // block wrapper it resolves against the leaf's own slot instead.
    if scrollable(entries) {
        let handle = handles.entry(format!("{id}-scroll")).or_default().clone();
        let mut wrapper = style(
            div()
                .id(SharedString::from(format!("{id}-scroll")))
                .track_scroll(&handle),
            entries,
            cx,
        );
        if let Some(tip) = tooltip {
            wrapper = wrapper.tooltip(move |_, cx| cx.new(|_| Tooltip::new(tip.clone())).into());
        }
        wrapper
            .overflow_y_scrollbar()
            .id(SharedString::from(format!("{id}-scrollbar")))
            .child(element)
            .into_any_element()
    } else if let Some(tip) = tooltip {
        // Tooltips need interactivity — the wrapper must be stateful.
        style(
            div()
                .id(SharedString::from(format!("{id}-tip")))
                .tooltip(move |_, cx| cx.new(|_| Tooltip::new(tip.clone())).into()),
            entries,
            cx,
        )
        .child(element)
        .into_any_element()
    } else {
        let wrapped = style(div(), entries, cx).child(element).into_any_element();
        if std::env::var_os("FORGE_BOUNDS").is_some() {
            BoundsLogger::new(wrapped, format!("wrap-{id}")).into_any_element()
        } else {
            wrapped
        }
    }
}

/// Whether a `style:` map asks for a scrolling element (`overflow: scroll`).
fn scrollable(entries: &Entries) -> bool {
    entries.iter().any(|(key, value)| {
        key == "overflow"
            && matches!(value, Value::Enum(word) | Value::String(word) if word == "scroll")
    })
}

/// Shown by a `forge.Preview` with no document.
fn empty(text: String, cx: &App) -> AnyElement {
    let theme = cx.theme();
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text)
        .into_any_element()
}

fn placeholder(message: String, cx: &App) -> AnyElement {
    let theme = cx.theme();
    div()
        .px_2()
        .py_1()
        .border_1()
        .border_dashed()
        .border_color(theme.border)
        .rounded_md()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(message)
        .into_any_element()
}

/// `anchor:` prop names → the popover/hover-card anchor corner.
fn anchor(name: &str) -> Anchor {
    match name {
        "top-left" | "TopLeft" => Anchor::TopLeft,
        "top" | "top-center" | "TopCenter" => Anchor::TopCenter,
        "top-right" | "TopRight" => Anchor::TopRight,
        "bottom" | "bottom-center" | "BottomCenter" => Anchor::BottomCenter,
        "bottom-right" | "BottomRight" => Anchor::BottomRight,
        "left" | "left-center" | "LeftCenter" => Anchor::LeftCenter,
        "right" | "right-center" | "RightCenter" => Anchor::RightCenter,
        _ => Anchor::TopLeft,
    }
}

/// `icons/<set>/<name>.svg` — `lucide` (the bundled set) keeps the flat
/// `icons/` prefix. Any other set looks under its own directory, which the
/// host's `AssetSource` layers over the embedded catalog.
fn icon_path(set: &str, name: &str) -> String {
    match set {
        "" | "lucide" => format!("icons/{name}.svg"),
        set => format!("icons/{set}/{name}.svg"),
    }
}

/// Sets canonical to the bundled catalog — iconify hosts its own versions
/// under these names. Loading them would mix versions in the same `icons/`
/// namespace, so they never go through `iconify::ensure_set`.
const BUNDLED_SETS: &[&str] = &["lucide", "heroicons"];

/// Iconify names can merge into a set's listing only where `icon_path` maps
/// them into the `icons/<set>/` namespace the synth layer serves — embedded
/// files still win on collision. Flat-namespaced bundled sets (`lucide`:
/// `icons/<name>.svg`) can't merge. Those names would never resolve.
fn iconify_mergeable(set: &str) -> bool {
    !matches!(set, "" | "lucide")
}

/// A synthesized `Select` node two-way-bound to `bind` — the icon browser's
/// set/category/theme pickers.
fn picker(options: Vec<String>, bind: Path) -> Node {
    Node {
        element: ElementRef {
            namespace: None,
            name: "Select".into(),
        },
        props: vec![
            (
                "options".to_owned(),
                Value::List(options.into_iter().map(Value::Enum).collect()),
            ),
            (
                "value".to_owned(),
                Value::Binding {
                    path: bind,
                    mode: BindingMode::TwoWay,
                },
            ),
        ],
        ..Default::default()
    }
}

/// Theme keys encode `"s:<key>"`/`"p:<key>"` in bound state so prefix and
/// suffix entries never collide. `""` Keys (default themes) survive as
/// bare `"s:"`/`"p:"`.
fn theme_sel_encode(kind: crate::iconify::ThemeKind, key: &str) -> String {
    let k = if kind == crate::iconify::ThemeKind::Prefix {
        "p"
    } else {
        "s"
    };
    format!("{k}:{key}")
}

fn theme_sel_decode(s: &str) -> Option<(crate::iconify::ThemeKind, String)> {
    if let Some(k) = s.strip_prefix("p:") {
        Some((crate::iconify::ThemeKind::Prefix, k.to_owned()))
    } else {
        s.strip_prefix("s:")
            .map(|k| (crate::iconify::ThemeKind::Suffix, k.to_owned()))
    }
}

/// Checked `(kind, key)` pairs for the icon browser's theme checkboxes.
/// `raw` is the bound `theme` value: a `["s:key"/"p:key"]` array, or a bare
/// string for back-compat (`"[all]"`, or a theme title the old Select
/// wrote). Empty/unset state defaults to the `""` no-suffix entries so
/// variant sets open on their base icons. "display all" is reached by
/// checking every box, not by emptying the list.
fn theme_selection(
    themes: &[(crate::iconify::ThemeKind, String, String)],
    raw: Option<&Json>,
) -> Vec<(crate::iconify::ThemeKind, String)> {
    let default = || -> Vec<(crate::iconify::ThemeKind, String)> {
        let d: Vec<_> = themes
            .iter()
            .filter(|(_, k, _)| k.is_empty())
            .map(|(k, t, _)| (*k, t.clone()))
            .collect();
        if d.is_empty() {
            themes.iter().map(|(k, t, _)| (*k, t.clone())).collect()
        } else {
            d
        }
    };
    match raw {
        Some(Json::Array(a)) => {
            let sel: Vec<_> = a
                .iter()
                .filter_map(|v| v.as_str())
                .filter_map(theme_sel_decode)
                .filter(|(k, t)| themes.iter().any(|(k2, t2, _)| k2 == k && t2 == t))
                .collect();
            if sel.is_empty() { default() } else { sel }
        }
        Some(Json::String(s)) if s == "[all]" => {
            themes.iter().map(|(k, t, _)| (*k, t.clone())).collect()
        }
        Some(Json::String(s)) => themes
            .iter()
            .find(|(_, _, t)| t == s)
            .map(|(k, t, _)| vec![(*k, t.clone())])
            .unwrap_or_else(default),
        _ => default(),
    }
}

/// `name` in `set` — see `icon_path`.
fn icon_set(set: &str, name: &str) -> Icon {
    Icon::empty().path(icon_path(set, name))
}

/// `doc_icon` for closures that captured `iconset` rather than `self`. The
/// `AssetSource::load` probe covers every local layer. Embedded lucide,
/// bundled sets, fs roots, vendored files, the iconify cache. So only a
/// genuinely-remote name queues a load and displays a placeholder. The image
/// cache retains a failed load, so the real path cannot render early.
/// Probe-then-load icon path resolution. Never hands `path()` bytes that
/// don't exist (the image cache retains a failed load forever). Missing
/// locally, the iconify load is queued only where the name resolves in the
/// `icons/<set>/` namespace the synth layer serves. Bundled lucide's flat
/// paths can never synthesize. Bundled files stay canonical on collision.
fn doc_icon_path(set: &str, name: &str, cx: &mut App) -> String {
    let path = icon_path(set, name);
    if cx.asset_source().load(&path).ok().flatten().is_some() {
        return path;
    }
    if iconify_mergeable(set) && crate::iconify::known_set(set) {
        crate::iconify::ensure_set(set, cx);
    }
    icon_path("lucide", "image")
}

fn doc_icon_for(set: &str, name: &str, cx: &mut App) -> Icon {
    Icon::empty().path(doc_icon_path(set, name, cx))
}

fn icon(name: &str) -> Icon {
    Icon::empty().path(icon_path("lucide", name))
}

fn button_variant(name: &str) -> ButtonVariant {
    match name {
        "Primary" => ButtonVariant::Primary,
        "Secondary" => ButtonVariant::Secondary,
        "Danger" => ButtonVariant::Danger,
        "Info" => ButtonVariant::Info,
        "Success" => ButtonVariant::Success,
        "Warning" => ButtonVariant::Warning,
        "Ghost" => ButtonVariant::Ghost,
        "Link" => ButtonVariant::Link,
        "Text" => ButtonVariant::Text,
        _ => ButtonVariant::Default,
    }
}

fn alert_variant(name: &str) -> AlertVariant {
    match name.to_lowercase().as_str() {
        "success" => AlertVariant::Success,
        "warning" => AlertVariant::Warning,
        "error" | "danger" => AlertVariant::Error,
        _ => AlertVariant::Info,
    }
}

/// Text styling a `forge.Files` `highlights:` glob applies to matching rows —
/// font (weight/italic/underline) and text color, not row background.
#[derive(Clone, Copy, Default)]
struct FileHighlight {
    weight: Option<FontWeight>,
    italic: bool,
    underline: bool,
    color: Option<Hsla>,
}

impl ForgeView {
    /// `highlights:` — `{glob: style}` pairs. A style is a shorthand string
    /// (`"bold green"`, `"italic #ffcc00"`, `"underline $theme.success"`), a
    /// map (`{weight: bold, italic: true, underline: true, color: ...}`), or —
    /// for bindings — a JSON object/string in the same shapes. A bare string
    /// prop also reads as `"glob = style. Glob = style"` lines.
    fn highlights(&self, value: &Value, cx: &App) -> Vec<(String, FileHighlight)> {
        match value {
            Value::Map(entries) => entries
                .iter()
                .filter_map(|(k, v)| self.highlight_spec(v, cx).map(|h| (k.clone(), h)))
                .collect(),
            Value::Binding { path, .. } => match self.lookup(path) {
                Some(Json::Object(m)) => m
                    .iter()
                    .filter_map(|(k, v)| self.highlight_json(v, cx).map(|h| (k.clone(), h)))
                    .collect(),
                Some(Json::String(s)) => self.highlight_lines(s, cx),
                _ => Vec::new(),
            },
            Value::List(items) => items
                .iter()
                .flat_map(|i| self.highlight_lines(&self.text(i), cx))
                .collect(),
            other => self.highlight_lines(&self.text(other), cx),
        }
    }

    /// `"glob = style. Glob = style"` / newline-separated shorthand lines.
    fn highlight_lines(&self, text: &str, cx: &App) -> Vec<(String, FileHighlight)> {
        text.split([';', '\n'])
            .filter_map(|line| {
                let (glob, spec) = line.split_once('=')?;
                let glob = glob.trim();
                (!glob.is_empty()).then(|| (glob.to_owned(), self.highlight_tokens(spec, cx)))
            })
            .collect()
    }

    /// A `Value::String` shorthand or `Value::Map` `{weight, italic, underline, color}`.
    fn highlight_spec(&self, value: &Value, cx: &App) -> Option<FileHighlight> {
        match value {
            Value::String(s) | Value::Enum(s) => Some(self.highlight_tokens(s, cx)),
            Value::Map(entries) => {
                let mut hl = FileHighlight::default();
                for (k, v) in entries {
                    match k.as_str() {
                        "weight" => hl.weight = Some(font_weight(&self.text(v))),
                        "italic" => hl.italic = self.truthy(v),
                        "underline" => hl.underline = self.truthy(v),
                        "color" => hl.color = self.highlight_color(&self.text(v), cx),
                        _ => {}
                    }
                }
                Some(hl)
            }
            _ => None,
        }
    }

    /// JSON spec from a bound object — string shorthand or `{weight, color, ...}`.
    fn highlight_json(&self, value: &Json, cx: &App) -> Option<FileHighlight> {
        match value {
            Json::String(s) => Some(self.highlight_tokens(s, cx)),
            Json::Object(m) => {
                let mut hl = FileHighlight::default();
                for (k, v) in m {
                    match k.as_str() {
                        "weight" => hl.weight = v.as_str().map(font_weight),
                        "italic" => hl.italic = v.as_bool().unwrap_or(false),
                        "underline" => hl.underline = v.as_bool().unwrap_or(false),
                        "color" => hl.color = v.as_str().and_then(|c| self.highlight_color(c, cx)),
                        _ => {}
                    }
                }
                Some(hl)
            }
            _ => None,
        }
    }

    /// Space/comma separated words: font weights, `italic`, `underline`, named
    /// colors, `#hex`, and `$theme.*`/bare theme tokens.
    fn highlight_tokens(&self, spec: &str, cx: &App) -> FileHighlight {
        let mut hl = FileHighlight::default();
        for word in spec.split(|c: char| c.is_whitespace() || c == ',') {
            let word = word.trim();
            if word.is_empty() {
                continue;
            }
            match word {
                "italic" => hl.italic = true,
                "underline" => hl.underline = true,
                "thin" | "light" | "medium" | "normal" | "semibold" | "bold" | "black" => {
                    hl.weight = Some(font_weight(word))
                }
                _ => hl.color = hl.color.or(self.highlight_color(word, cx)),
            }
        }
        hl
    }

    /// A color word → `Hsla`: named hues, `#hex`, `$theme.x` or a bare theme
    /// token (`success`, `muted_foreground`, …).
    fn highlight_color(&self, word: &str, cx: &App) -> Option<Hsla> {
        if let Some(hex) = word.strip_prefix('#') {
            return parse_hex(hex).map(Hsla::from);
        }
        let word = word.strip_prefix('$').unwrap_or(word);
        let word = word.strip_prefix("theme.").unwrap_or(word);
        let named = match word {
            "red" => Some(hsla(0.0, 0.72, 0.55, 1.0)),
            "orange" => Some(hsla(0.07, 0.9, 0.55, 1.0)),
            "yellow" => Some(hsla(0.13, 0.85, 0.55, 1.0)),
            "green" => Some(hsla(0.38, 0.55, 0.48, 1.0)),
            "teal" | "cyan" => Some(hsla(0.5, 0.6, 0.45, 1.0)),
            "blue" => Some(hsla(0.6, 0.7, 0.58, 1.0)),
            "indigo" => Some(hsla(0.66, 0.55, 0.62, 1.0)),
            "purple" | "violet" => Some(hsla(0.75, 0.5, 0.62, 1.0)),
            "pink" => Some(hsla(0.92, 0.65, 0.62, 1.0)),
            "gray" | "grey" => Some(hsla(0.0, 0.0, 0.5, 1.0)),
            _ => None,
        };
        named.or_else(|| {
            parse_path(word)
                .ok()
                .and_then(|p| color(&Value::Token(p), cx))
        })
    }
}

/// `font_weight` word → gpui `FontWeight` — mirrors `style()`'s `weight:` table.
fn font_weight(word: &str) -> FontWeight {
    match word {
        "thin" => FontWeight::THIN,
        "light" => FontWeight::LIGHT,
        "medium" => FontWeight::MEDIUM,
        "semibold" => FontWeight::SEMIBOLD,
        "bold" => FontWeight::BOLD,
        "black" => FontWeight::BLACK,
        _ => FontWeight::NORMAL,
    }
}

/// `#rgb` / `#rrggbb` / `#rrggbbaa` → `Rgba`. No alpha channel means opaque.
fn parse_hex(hex: &str) -> Option<Rgba> {
    let hex = match hex.len() {
        3 | 4 => hex.chars().map(|c| format!("{c}{c}")).collect::<String>(),
        _ => hex.to_owned(),
    };
    if hex.len() != 6 && hex.len() != 8 {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some(Rgba {
        r: byte(0)? as f32 / 255.0,
        g: byte(2)? as f32 / 255.0,
        b: byte(4)? as f32 / 255.0,
        a: if hex.len() == 8 {
            byte(6)? as f32 / 255.0
        } else {
            1.0
        },
    })
}

/// Lucide icon for a `forge.Files` row: folders swap open/closed, files get a
/// glyph by extension, unknowns fall back to a plain `file`. `icon_map`
/// (`{ext-or-key: icon}`) overrides the built-in table. `dir`, `dir-open`
/// and `file` rename the structural glyphs.
fn file_icon(name: &str, is_dir: bool, expanded: bool, icon_map: &[(String, String)]) -> String {
    let custom = |key: &str| {
        icon_map
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, i)| i.clone())
    };
    if is_dir {
        let key = if expanded { "dir-open" } else { "dir" };
        return custom(key)
            .unwrap_or_else(|| if expanded { "folder-open" } else { "folder" }.to_owned());
    }
    if let Some(i) = name
        .rsplit('.')
        .next()
        .map(str::to_lowercase)
        .and_then(|ext| custom(&ext))
    {
        return i;
    }
    if name.ends_with(".forge.yaml") {
        return "file-braces".to_owned();
    }
    let ext = name.rsplit('.').next().unwrap_or_default();
    match ext {
        "rs" | "py" | "js" | "jsx" | "ts" | "tsx" | "go" | "c" | "cc" | "cpp" | "h" | "hpp"
        | "java" | "kt" | "rb" | "swift" | "cs" | "fs" | "ex" | "exs" | "erl" | "hrl" | "hs"
        | "ml" | "mli" | "clj" | "cljs" | "scala" | "dart" | "r" | "m" | "mm" | "pl" | "pm"
        | "php" | "vim" | "el" | "lisp" | "scm" | "rkt" | "nim" | "cr" | "d" | "pas" | "jl"
        | "groovy" | "lua" | "zig" | "v" | "sv" | "proto" | "graphql" | "gql" | "css" | "scss"
        | "less" | "html" | "htm" | "vue" | "svelte" | "astro" | "sql" | "wasm" => "file-code",
        "yaml" | "yml" | "json" | "toml" | "xml" | "kdl" | "ini" | "cfg" | "conf" => "file-braces",
        "md" | "markdown" | "txt" | "rst" | "adoc" | "org" | "tex" | "pdf" | "rtf" | "doc"
        | "docx" | "odt" | "epub" => "file-text",
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "bmp" | "ico" | "tif" | "tiff"
        | "avif" | "heic" | "heif" => "file-image",
        "mp3" | "wav" | "ogg" | "flac" | "m4a" | "aac" | "opus" | "mid" => "file-music",
        "mp4" | "mov" | "mkv" | "webm" | "avi" | "wmv" | "m4v" | "mpg" | "mpeg" => "file-play",
        "zip" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "7z" | "rar" | "zst" | "lz4" | "jar"
        | "war" | "apk" | "dmg" | "iso" => "file-archive",
        "csv" | "tsv" | "xls" | "xlsx" | "ods" => "file-spreadsheet",
        "sh" | "bash" | "zsh" | "fish" | "ps1" | "bat" | "cmd" | "exe" | "msi" | "com"
        | "appimage" | "bin" | "run" => "file-terminal",
        "env" | "key" | "pem" | "crt" | "cer" | "p12" | "pub" | "asc" | "gpg" | "secret"
        | "lock" => "file-lock",
        _ => match name {
            "Makefile" | "Dockerfile" | "Containerfile" | "Justfile" | "Taskfile" | "Procfile"
            | "Vagrantfile" | "Gemfile" | "Rakefile" => "file-terminal",
            _ => "file",
        },
    }
    .to_owned()
}

/// `forge.Files` `root:` scan — folders plus `filter:`-globbed files, dirs
/// first, VCS/build noise skipped, depth- and entry-capped so a huge tree
/// can't stall the render.
pub fn scan_files(
    dir: &std::path::Path,
    filters: &[String],
    depth: usize,
    hidden: bool,
    budget: &mut usize,
) -> Vec<Json> {
    const SKIP: &[&str] = &["target", "node_modules", "dist", "build", "out"];
    if depth == 0 || *budget == 0 {
        return Vec::new();
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut entries: Vec<(String, std::path::PathBuf, bool)> = rd
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let path = e.path();
            let is_dir = path.is_dir();
            if (!hidden && name.starts_with('.')) || SKIP.contains(&name.as_str()) {
                return None;
            }
            if !is_dir && !filters.is_empty() && !filters.iter().any(|f| glob_match(f, &name)) {
                return None;
            }
            Some((name, path, is_dir))
        })
        .collect();
    entries.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
    entries
        .into_iter()
        .filter_map(|(name, path, is_dir)| {
            if *budget == 0 {
                return None;
            }
            *budget -= 1;
            Some(serde_json::json!({
                "name": name,
                "path": path.display().to_string(),
                "dir": is_dir,
                "children": if is_dir {
                    scan_files(&path, filters, depth - 1, hidden, budget)
                } else {
                    Vec::<Json>::new()
                },
            }))
        })
        .collect()
}

/// `forge.Files` search. Name-substring filter over the listing JSON. A file
/// keeps itself on a hit. A dir keeps itself and its whole subtree on a hit,
/// or survives with filtered children so matches stay under their folders.
pub fn filter_listing(entries: &[Json], query: &str) -> Vec<Json> {
    let q = query.to_lowercase();
    entries
        .iter()
        .filter_map(|e| {
            let name = e["name"].as_str().unwrap_or_default();
            if name.to_lowercase().contains(&q) {
                return Some(e.clone());
            }
            if e["dir"].as_bool().unwrap_or(false) {
                let children = e["children"]
                    .as_array()
                    .map(|c| filter_listing(c, query))
                    .unwrap_or_default();
                if !children.is_empty() {
                    let mut e = e.clone();
                    e["children"] = Json::from(children);
                    return Some(e);
                }
            }
            None
        })
        .collect()
}

/// Tiny glob matcher for `forge.Files` `filter:` — `*` spans any run of
/// characters, `?` matches one. Comparison is ASCII case-insensitive.
pub fn glob_match(pattern: &str, name: &str) -> bool {
    fn inner(p: &[u8], n: &[u8]) -> bool {
        match p.first() {
            None => n.is_empty(),
            Some(b'*') => (0..=n.len()).any(|i| inner(&p[1..], &n[i..])),
            Some(b'?') => !n.is_empty() && inner(&p[1..], &n[1..]),
            Some(&c) => !n.is_empty() && c.eq_ignore_ascii_case(&n[0]) && inner(&p[1..], &n[1..]),
        }
    }
    inner(pattern.as_bytes(), name.as_bytes())
}

/// `forge.Showcase` sample props — a declared `default` wins. Otherwise a
/// plausible value per type so every widget displays a themed state. It uses
/// active for checked/pressed/selected/open, the first enum variant, and sample strings.
fn sample_prop_value(prop: &forge_spec::catalog::PropDef) -> Option<Value> {
    if let Some(default) = &prop.default {
        return Some(default.clone());
    }
    match &prop.ty {
        PropType::String => Some(Value::String(match prop.name.as_str() {
            "placeholder" => "sample…".into(),
            "src" | "url" => String::new(),
            _ => "Sample".into(),
        })),
        PropType::Number => Some(Value::Number {
            value: 50.,
            unit: None,
        }),
        PropType::Bool
            if matches!(
                prop.name.as_str(),
                "checked" | "pressed" | "selected" | "open"
            ) =>
        {
            Some(Value::Bool(true))
        }
        PropType::Enum(values) => values.first().map(|v| Value::Enum(v.clone())),
        _ => None,
    }
}

fn tag_variant(name: &str) -> TagVariant {
    match name {
        "Primary" => TagVariant::Primary,
        "Danger" => TagVariant::Danger,
        "Success" => TagVariant::Success,
        "Warning" => TagVariant::Warning,
        "Info" => TagVariant::Info,
        _ => TagVariant::Secondary,
    }
}

fn group_box_variant(name: &str) -> GroupBoxVariant {
    match name {
        "Fill" => GroupBoxVariant::Fill,
        "Outline" => GroupBoxVariant::Outline,
        _ => GroupBoxVariant::Normal,
    }
}

fn tab_variant(name: &str) -> TabVariant {
    match name {
        "Outline" => TabVariant::Outline,
        "Pill" => TabVariant::Pill,
        "Segmented" => TabVariant::Segmented,
        "Underline" => TabVariant::Underline,
        "Folder" => TabVariant::Folder,
        _ => TabVariant::Tab,
    }
}

fn size(name: &str) -> Size {
    match name {
        "XSmall" => Size::XSmall,
        "Small" => Size::Small,
        "Large" => Size::Large,
        _ => Size::Medium,
    }
}

fn length(value: &Value) -> Option<DefiniteLength> {
    match value {
        Value::Number { value, unit } => Some(match unit {
            None | Some(Unit::Px) => px(*value as f32).into(),
            Some(Unit::Rem) => rems(*value as f32).into(),
            Some(Unit::Percent) => relative(*value as f32 / 100.0),
            Some(Unit::Fr) => return None,
        }),
        _ => None,
    }
}

fn color(value: &Value, cx: &App) -> Option<Hsla> {
    match value {
        Value::Color(c) => Some(
            Rgba {
                r: f32::from(c.r) / 255.0,
                g: f32::from(c.g) / 255.0,
                b: f32::from(c.b) / 255.0,
                a: f32::from(c.a) / 255.0,
            }
            .into(),
        ),
        Value::Token(path) => {
            let theme = cx.theme();
            let name = match path.0.last()? {
                PathSegment::Field(name) => name.as_str(),
                PathSegment::Index(_) => return None,
            };
            Some(match name {
                "primary" => theme.primary,
                "secondary" => theme.secondary,
                "background" => theme.background,
                "foreground" => theme.foreground,
                "muted" => theme.muted,
                "muted_foreground" | "muted-foreground" => theme.muted_foreground,
                "accent" => theme.accent,
                "border" => theme.border,
                "danger" => theme.danger,
                "success" => theme.success,
                "warning" => theme.warning,
                "info" => theme.info,
                _ => return None,
            })
        }
        _ => None,
    }
}

/// Applies a node's `style:` map (`docs/format/style-layout.md`).
fn style<E: Styled>(element: E, entries: &Entries, cx: &App) -> E {
    entries.iter().fold(element, |e, (key, value)| {
        let word = match value {
            Value::Enum(s) | Value::String(s) => s.as_str(),
            _ => "",
        };
        let flag = matches!(value, Value::Bool(true));
        match (key.as_str(), length(value), color(value, cx)) {
            ("gap", Some(l), _) => e.gap(l),
            ("gap-x", Some(l), _) => e.gap_x(l),
            ("gap-y", Some(l), _) => e.gap_y(l),
            ("width", Some(l), _) => e.w(l),
            ("height", Some(l), _) => e.h(l),
            ("min-width", Some(l), _) => e.min_w(l),
            ("max-width", Some(l), _) => e.max_w(l),
            ("min-height", Some(l), _) => e.min_h(l),
            ("max-height", Some(l), _) => e.max_h(l),
            ("basis", Some(l), _) => e.flex_basis(l),
            ("line-height", Some(l), _) => e.line_height(l),
            ("width", _, _) if word == "full" => e.w_full(),
            ("height", _, _) if word == "full" => e.h_full(),
            ("padding", _, _) => match edges(value) {
                Some([t, r, b, l]) => e.pt(t).pr(r).pb(b).pl(l),
                None => e,
            },
            ("margin", _, _) => match edges(value) {
                Some([t, r, b, l]) => e.mt(t).mr(r).mb(b).ml(l),
                None => e,
            },
            ("background", _, Some(c)) => e.bg(c),
            ("color", _, Some(c)) => e.text_color(c),
            ("border-color", _, Some(c)) => e.border_color(c),
            ("border", Some(_), _) => e.border_1(),
            ("border-style", _, _) if word == "dashed" => e.border_dashed(),
            ("radius", Some(_), _) | ("radius", _, _) => match (word, value) {
                ("sm", _) => e.rounded_sm(),
                ("md", _) => e.rounded_md(),
                ("lg", _) => e.rounded_lg(),
                ("full", _) => e.rounded_full(),
                (
                    _,
                    Value::Number {
                        value,
                        unit: None | Some(Unit::Px),
                    },
                ) => e.rounded(px(*value as f32)),
                (
                    _,
                    Value::Number {
                        value,
                        unit: Some(Unit::Rem),
                    },
                ) => e.rounded(rems(*value as f32)),
                _ => e,
            },
            ("direction", _, _) => match word {
                "row" => e.flex_row(),
                "column" => e.flex_col(),
                "row-reverse" => e.flex_row_reverse(),
                "column-reverse" => e.flex_col_reverse(),
                _ => e,
            },
            ("align", _, _) => match word {
                "start" => e.items_start(),
                "center" => e.items_center(),
                "end" => e.items_end(),
                "stretch" => e.items_stretch(),
                "baseline" => e.items_baseline(),
                _ => e,
            },
            ("align-self", _, _) => match word {
                "start" => e.self_start(),
                "center" => e.self_center(),
                "end" => e.self_end(),
                "stretch" => e.self_stretch(),
                "baseline" => e.self_baseline(),
                _ => e,
            },
            ("justify", _, _) => match word {
                "start" => e.justify_start(),
                "center" => e.justify_center(),
                "end" => e.justify_end(),
                "between" => e.justify_between(),
                "around" => e.justify_around(),
                "evenly" => e.justify_evenly(),
                _ => e,
            },
            ("wrap", _, _) => {
                if flag {
                    e.flex_wrap()
                } else {
                    e.flex_nowrap()
                }
            }
            ("grow", _, _) => match value {
                Value::Number { value, .. } => e.flex_grow(*value as f32),
                Value::Bool(b) => e.flex_grow(if *b { 1.0 } else { 0.0 }),
                _ => e,
            },
            ("shrink", _, _) => match value {
                Value::Number { value, .. } => e.flex_shrink(*value as f32),
                Value::Bool(b) => e.flex_shrink(if *b { 1.0 } else { 0.0 }),
                _ => e,
            },
            // `scroll` needs an element id. `container`/`wrap` apply it instead.
            ("overflow", _, _) if !matches!(word, "visible" | "scroll") => e.overflow_hidden(),
            ("opacity", _, _) => match value {
                Value::Number { value, .. } => e.opacity(*value as f32),
                _ => e,
            },
            ("shadow", _, _) => match word {
                "sm" => e.shadow_sm(),
                "md" => e.shadow_md(),
                "lg" => e.shadow_lg(),
                "xl" => e.shadow_xl(),
                _ => e,
            },
            ("size", _, _) => match word {
                "xs" => e.text_xs(),
                "sm" => e.text_sm(),
                "base" | "md" => e.text_base(),
                "lg" => e.text_lg(),
                "xl" => e.text_xl(),
                "2xl" => e.text_2xl(),
                "3xl" => e.text_3xl(),
                _ => match value {
                    Value::Number { value, .. } => e.text_size(px(*value as f32)),
                    _ => e,
                },
            },
            ("weight", _, _) => e.font_weight(match word {
                "thin" => FontWeight::THIN,
                "light" => FontWeight::LIGHT,
                "medium" => FontWeight::MEDIUM,
                "semibold" => FontWeight::SEMIBOLD,
                "bold" => FontWeight::BOLD,
                "black" => FontWeight::BLACK,
                _ => FontWeight::NORMAL,
            }),
            ("text-align", _, _) => match word {
                "left" => e.text_left(),
                "center" => e.text_center(),
                "right" => e.text_right(),
                _ => e,
            },
            ("italic", _, _) if flag => e.italic(),
            ("underline", _, _) if flag => e.underline(),
            ("truncate", _, _) if flag => e.truncate(),
            _ => e,
        }
    })
}

/// Apply the flex geometry of `entries` to an overlay wrapper. Selection,
/// pick and drop-target frames wrap a node in `div().relative()`. A plain
/// wrapper drops the node's `grow`/`basis`, so the wrapper collapses to
/// content size inside a flex parent and the layout shrinks.
fn flex_item<E: Styled>(element: E, entries: &Entries, cx: &App) -> E {
    let flex: Entries = entries
        .iter()
        .filter(|(key, _)| {
            matches!(
                key.as_str(),
                "grow"
                    | "shrink"
                    | "basis"
                    | "width"
                    | "height"
                    | "min-width"
                    | "min-height"
                    | "max-width"
                    | "max-height"
            )
        })
        .cloned()
        .collect();
    style(element, &flex, cx)
}

/// `padding`/`margin` as `[top, right, bottom, left]`.
fn edges(value: &Value) -> Option<[DefiniteLength; 4]> {
    match value {
        Value::List(items) => {
            let l: Vec<DefiniteLength> = items.iter().filter_map(length).collect();
            match l.as_slice() {
                [a] => Some([*a; 4]),
                [y, x] => Some([*y, *x, *y, *x]),
                [t, r, b, l] => Some([*t, *r, *b, *l]),
                _ => None,
            }
        }
        other => length(other).map(|l| [l; 4]),
    }
}

/// #120 Phase 1 — wraps a child and records its window-space bounds into a
/// shared cell each paint. Studio reads the cell to send `frame_rect` to the
/// remote preview host (fake-embed position tracking).
pub struct BoundsTracker {
    child: AnyElement,
    cell: std::rc::Rc<std::cell::RefCell<Option<Bounds<Pixels>>>>,
}

/// Debug instrument — like `BoundsTracker` but logs the laid-out bounds to
/// stderr each frame. Wrap an element to measure what layout actually gave it.
pub struct BoundsLogger {
    child: AnyElement,
    label: String,
}

impl BoundsLogger {
    pub fn new(child: AnyElement, label: impl Into<String>) -> Self {
        Self {
            child,
            label: label.into(),
        }
    }
}

impl Element for BoundsLogger {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        (self.child.request_layout(window, cx), ())
    }
    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        eprintln!("[bounds] {} -> {:?}", self.label, bounds);
        self.child.prepaint(window, cx);
    }
    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx)
    }
}

impl IntoElement for BoundsLogger {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl BoundsTracker {
    pub fn new(
        child: AnyElement,
        cell: std::rc::Rc<std::cell::RefCell<Option<Bounds<Pixels>>>>,
    ) -> Self {
        Self { child, cell }
    }
}

impl Element for BoundsTracker {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        (self.child.request_layout(window, cx), ())
    }
    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        *self.cell.borrow_mut() = Some(bounds);
        self.child.prepaint(window, cx);
    }
    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx)
    }
}

impl IntoElement for BoundsTracker {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

/// Positional `key`s collide when generated rows reorder (`for:` shifts,
/// conditional branches). Salting a state cache key with the node's bound
/// paths stops a row inheriting an entity that still displays or writes
/// another binding's value.
pub(crate) fn bind_salt(node: &Node) -> String {
    let mut bound = node
        .props
        .iter()
        .filter_map(|(name, value)| match value {
            Value::Binding { path, .. } => Some(format!("{name}={path}")),
            _ => None,
        })
        .collect::<Vec<_>>();
    if let Some(Value::Binding { path, .. }) = &node.content {
        bound.push(format!("*={path}"));
    }
    bound.sort();
    bound.join("&")
}

#[cfg(test)]
mod tests {
    use super::pane_key_name;
    use gpui_kit::{Keystroke, Modifiers};

    fn ks(key: &str, key_char: Option<&str>, modifiers: Modifiers) -> Keystroke {
        Keystroke {
            modifiers,
            key: key.to_string(),
            key_char: key_char.map(str::to_string),
        }
    }

    #[test]
    fn pane_key_maps_named_keys() {
        let plain = Modifiers::default();
        assert_eq!(
            pane_key_name(&ks("tab", None, plain)).as_deref(),
            Some("Tab")
        );
        assert_eq!(
            pane_key_name(&ks(
                "tab",
                None,
                Modifiers {
                    shift: true,
                    ..Default::default()
                }
            ))
            .as_deref(),
            Some("BackTab")
        );
        assert_eq!(
            pane_key_name(&ks("enter", None, plain)).as_deref(),
            Some("Enter")
        );
        assert_eq!(
            pane_key_name(&ks("backspace", None, plain)).as_deref(),
            Some("Backspace")
        );
        assert_eq!(
            pane_key_name(&ks("space", Some(" "), plain)).as_deref(),
            Some(" ")
        );
        assert_eq!(pane_key_name(&ks("up", None, plain)).as_deref(), Some("Up"));
        assert_eq!(pane_key_name(&ks("f5", None, plain)), None);
    }

    #[test]
    fn pane_key_uses_key_char_and_drops_ctrl() {
        let plain = Modifiers::default();
        assert_eq!(
            pane_key_name(&ks("a", Some("a"), plain)).as_deref(),
            Some("a")
        );
        // Shift+a — `key_char` carries the typed character.
        assert_eq!(
            pane_key_name(&ks(
                "a",
                Some("A"),
                Modifiers {
                    shift: true,
                    ..Default::default()
                }
            ))
            .as_deref(),
            Some("A")
        );
        // Ctrl/cmd chords have no wire name — a bare letter would insert it.
        assert_eq!(
            pane_key_name(&ks(
                "c",
                None,
                Modifiers {
                    control: true,
                    ..Default::default()
                }
            )),
            None
        );
    }
}
